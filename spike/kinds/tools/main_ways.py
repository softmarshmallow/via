"""The skeleton step: a settlement's main ways, stated by where they go, found over the land.

Town library knowledge, not the compiler's. The author writes a `main_way` with named ends (and, if it
must pass somewhere, `via = [...]`), and no line (`starts_at`, `ends_at`, `path = []`). This step finds each one's course over the ground
beneath it: the cheapest way that keeps its gradient low, goes round hills instead of over them,
crosses water only at a named `crossing`, and joins a main way already found instead of running beside
it. The course is then eased, so it is a road and not a walk from cell to cell.

A `crossing` owns its bridge. It is a point on a watercourse; this step gives it one straight deck,
square to the water there, from a bridgehead on one bank to a bridgehead on the other, level and
clear of the water, with the ground at each bridgehead raised to meet it. A main way that names the
crossing is led to one bridgehead in line with the deck and away from the other; a main way that
starts or ends at a crossing starts or ends at the bridgehead on its own side.

It hands back, for each main way: `street` features marked `arterial` (one stretch per bank) and an
`axis` along the whole of it for fields to measure from; for each crossing a `bridge` named
`<crossing>_deck`; the ground with the bridgeheads raised; and each watercourse it was told of as a
region marked `water` (unless the layer beneath already gives one), so the street and lot tools keep off it.
"""
import math

import numpy as np
from scipy.ndimage import binary_dilation, gaussian_filter1d, map_coordinates
from shapely.geometry import Polygon
from scipy.sparse import coo_matrix
from scipy.sparse.csgraph import dijkstra
from shapely.geometry import LineString, Point
from shapely.ops import substring, unary_union

from viasdk import step

s = step()
p, fr = s.params, s.frame
feats = s.build.features
g = float(p.get("cell", 8.0))                      # the grid the search walks, m
easy = float(p.get("easy_grade", 0.04))            # a gradient that costs as much again as level ground
sea = float(s.build.doc.get("params", {}).get("sea", 0.0))
ny, nx = int((fr.y1 - fr.y0) // g), int((fr.x1 - fr.x0) // g)
gy, gx = np.mgrid[0:ny, 0:nx].astype(float)
at_posts = [(gy + 0.5) * g / fr.resolution, (gx + 0.5) * g / fr.resolution]
H = s.build.field("height").astype(float).copy()        # the ground; bridgeheads are raised in it below
W = s.build.field("water").astype(float)


def cell(x, y):
    return min(max(int((y - fr.y0) // g), 0), ny - 1), min(max(int((x - fr.x0) // g), 0), nx - 1)


def place(name):
    f = feats.get(name)
    if f is None:
        return None
    return tuple(f["at"]) if "at" in f else tuple(f["points"][0])


def smooth01(a, b, x):
    k = np.clip((np.asarray(x, float) - a) / (b - a), 0.0, 1.0)
    return k * k * (3 - 2 * k)


# ---- watercourses, as regions for the tools above
water_regions, courses = [], []
for name in p.get("watercourses", []):
    f = feats.get(name)
    if f is None:
        s.error("unknown_watercourse", s.id, f"`watercourses` names '{name}', which is not a feature beneath this step")
        continue
    given = feats.get(name + "_water")
    if given is not None and given.get("shape") == "region":      # the layer beneath says how wide the water is
        bank = Polygon(given["points"]).buffer(0)
    else:
        bank = LineString(f["points"]).buffer(float(p.get("water_half", 7.0)))
        s.add("feature", {"id": name + "_water", "kind": "guide", "water": 1,
                          "region": [[round(x, 1), round(y, 1)] for x, y in list(bank.simplify(1.0).exterior.coords)[:-1]]})
    water_regions.append(bank)
    courses.append((name, LineString(f["points"]), bank))
river = unary_union(water_regions) if water_regions else None

# ---- crossings: each owns one straight, level deck, square to the water, between two raised bridgeheads
decks = {}                                 # crossing id -> its two bridgeheads and the points that lead to them
py_, px_ = np.mgrid[0:fr.ny, 0:fr.nx]
PX, PY = fr.x0 + px_ * fr.resolution, fr.y0 + py_ * fr.resolution
for cid, f in feats.items():
    if f["kind"] != "crossing":
        continue
    a = f.get("attrs", f)
    at = Point(place(cid))
    if not courses:
        s.error("crossing.no_water", cid, "there is no watercourse for it to cross", hint="name the watercourse in this step's `watercourses`")
        continue
    name, line, bank = min(courses, key=lambda c: c[1].distance(at))
    if line.distance(at) > 40:
        s.error("crossing.dry", cid, f"it is {line.distance(at):.0f} m from '{name}', the nearest watercourse: a crossing stands on water",
                where=(at.x, at.y), hint=f"place it with on = {{ feature = \"{name}\", t = ... }}")
        continue
    st = line.project(at)
    q0, q1 = line.interpolate(max(st - 8, 0)), line.interpolate(min(st + 8, line.length))
    tan = np.array([q1.x - q0.x, q1.y - q0.y]) / max(math.hypot(q1.x - q0.x, q1.y - q0.y), 1e-9)
    nrm = np.array([-tan[1], tan[0]])
    c = np.array(line.interpolate(st).coords[0])
    across = LineString([c - nrm * 150, c + nrm * 150]).intersection(bank)
    across = min(getattr(across, "geoms", [across]), key=lambda g_: g_.distance(Point(c)))
    e = [np.array(across.coords[0]), np.array(across.coords[-1])]
    if np.dot(e[0] - c, nrm) > 0:
        e = e[::-1]                                           # e[0] on the bank the normal points away from
    setback, lead, width = float(a.get("setback", 8.0)), float(a.get("lead", 36.0)), float(a.get("width", 19.0))
    room, thick = float(a.get("clear", 1.3)), 1.2
    heads = [e[0] - nrm * setback, e[1] + nrm * setback]
    level = float(map_coordinates(W, [[(c[1] - fr.y0) / fr.resolution], [(c[0] - fr.x0) / fr.resolution]], order=1, mode="nearest")[0])
    top = level + room + thick + 0.4
    for edge, head, out in ((e[0], heads[0], -nrm), (e[1], heads[1], nrm)):      # raise the ground at each bridgehead
        r = np.hypot(PX - head[0], PY - head[1])
        landward = (PX - edge[0]) * out[0] + (PY - edge[1]) * out[1]
        m = (1 - smooth01(0.7 * lead, 0.7 * lead + float(a.get("shoulder", 38.0)), r)) * smooth01(2.5, 6.5, landward)      # an abutment's face to the water
        H = np.maximum(H, H + (top - H) * m)
    decks[cid] = {"heads": heads, "leads": [heads[0] - nrm * lead, heads[1] + nrm * lead], "width": width}
    s.add("feature", {"id": f"{cid}_deck", "kind": "bridge", "width": width, "rise": 0.45, "ramp": 6.0, "max_grade": 14.0, "smooth": False,
                      "at_crossing": cid, "over": [name + "_water"], "clear": room,
                      "path": [[round(float(x), 1), round(float(y), 1)] for x, y in (heads[0] - nrm * 4.0, heads[1] + nrm * 4.0)]})      # it laps the road at each end
if decks:
    s.write_field("height", H, unit="m")

h = map_coordinates(H, at_posts, order=1, mode="nearest")
wtr = map_coordinates(W, at_posts, order=1, mode="nearest")
in_sea = h < sea + 0.3
wet = binary_dilation((wtr > h + 0.15) & ~in_sea, iterations=2)       # thick enough that no step of the search jumps it

MOVES = [(dy, dx) for dy in (-2, -1, 0, 1, 2) for dx in (-2, -1, 0, 1, 2) if (dy, dx) != (0, 0) and math.gcd(abs(dy), abs(dx)) == 1]
used = np.zeros((ny, nx), bool)


def graph():
    """Every step the search may take, with what it costs: its length, more the steeper it is, far more
    through water, less along a main way already found. The sea cannot be entered. Water is crossed on a
    crossing's deck, which is not part of this search."""
    weight = np.where(wet, float(p.get("water_cost", 40.0)), 1.0) * np.where(used, float(p.get("joined", 0.45)), 1.0)
    rows, cols, costs = [], [], []
    idx = np.arange(ny * nx).reshape(ny, nx)
    for dy, dx in MOVES:
        ys, xs = slice(max(0, -dy), ny - max(0, dy)), slice(max(0, -dx), nx - max(0, dx))
        yd, xd = slice(max(0, dy), ny - max(0, -dy)), slice(max(0, dx), nx - max(0, -dx))
        run = g * math.hypot(dy, dx)
        grade = np.abs(h[yd, xd] - h[ys, xs]) / run
        c = run * (1.0 + (grade / easy) ** 2) * 0.5 * (weight[ys, xs] + weight[yd, xd])
        ok = ~in_sea[yd, xd] & ~in_sea[ys, xs]
        rows.append(idx[ys, xs][ok])
        cols.append(idx[yd, xd][ok])
        costs.append(c[ok])
    return coo_matrix((np.concatenate(costs), (np.concatenate(rows), np.concatenate(cols))), shape=(ny * nx, ny * nx)).tocsr()


def reach(net, a):
    """What it costs to get from a to everywhere, and the way back."""
    ay, ax = cell(*a)
    return dijkstra(net, indices=ay * nx + ax, return_predecessors=True)


def cost(found, b):
    by, bx = cell(*b)
    return found[0][by * nx + bx]


def leg(found, a, b):
    """The cheapest course from a to b, as world points; None if there is none."""
    (ay, ax), (by, bx) = cell(*a), cell(*b)
    dist, back = found
    if not np.isfinite(dist[by * nx + bx]):
        return None
    walk, at = [], by * nx + bx
    while at != ay * nx + ax:
        walk.append(at)
        at = back[at]
    walk.append(at)
    pts = np.asarray([(fr.x0 + (i % nx + 0.5) * g, fr.y0 + (i // nx + 0.5) * g) for i in walk[::-1]], float)
    pts[0], pts[-1] = a, b
    return pts


def eased(pts, every=4.0, span=None):
    """A walk from cell to cell made into a road: resampled evenly and smoothed along its length, its ends kept."""
    span = float(p.get("ease", 34.0)) if span is None else span
    seg = np.hypot(*np.diff(pts, axis=0).T)
    along = np.concatenate([[0.0], np.cumsum(seg)])
    if along[-1] < 3 * every:
        return pts
    t = np.linspace(0.0, along[-1], max(int(along[-1] / every), 3))
    q = np.stack([np.interp(t, along, pts[:, 0]), np.interp(t, along, pts[:, 1])], axis=1)
    sm = gaussian_filter1d(q, span / every, axis=0, mode="nearest")
    lead = np.minimum(t, t[-1] - t) / (1.5 * span)            # 0 at an end, 1 well away from it
    k = np.clip(lead, 0.0, 1.0)[:, None]
    k = k * k * (3 - 2 * k)
    return q * (1 - k) + sm * k


# ---- each main way, in the order written: earlier ones are there for later ones to join
report = []
for f in feats.values():
    if f["kind"] != "main_way":
        continue
    a = f.get("attrs", f)
    names = [f.get("starts_at", "its start"), *f.get("via", []), f.get("ends_at", "its end")]
    stops = [tuple(f["points"][0]), *[place(n) for n in f.get("via", [])], tuple(f["points"][-1])]
    if None in stops:
        missing = [str(n) for n, st in zip(names, stops) if st is None]
        s.error("main_way.place", f["id"], f"`via` names {', '.join(missing)}, which is not a point beneath this step",
                hint="name a point feature (a square's centre, a crossing)")
        continue
    net = graph()
    runs, used_decks, failed = [[]], [], False      # runs: the stretches on land, each a list of raw points; a deck parts two of them
    here = None

    def go(target, name, tail=()):
        """Carry the current stretch on over the land from `here` to `target`, then through `tail`."""
        global here, failed
        pts = leg(reach(net, here), here, target)
        if pts is None:
            s.error("main_way.no_course", f["id"], f"there is no way over the land to '{name}' that stays out of the sea",
                    where=target, hint="move an end onto land, or name a `crossing` on the water between them in `via`")
            failed = True
            return
        runs[-1].extend([tuple(q) for q in pts[(1 if runs[-1] else 0):]] + [tuple(q) for q in tail])
        here = tuple(tail[-1]) if len(tail) else tuple(target)

    for k, (name, stop) in enumerate(zip(names, stops)):
        d = decks.get(name)
        if d is None:
            if here is None:
                here = stop
            else:
                go(stop, name)
        elif here is None:                                    # it starts at a crossing: at the bridgehead on the side it is going
            nxt = decks.get(names[k + 1])
            found = [reach(net, ld) for ld in d["leads"]]
            aims = nxt["leads"] if nxt else [stops[k + 1]]
            side = min((0, 1), key=lambda i: min(cost(found[i], aim) for aim in aims))
            runs[-1].append(tuple(d["heads"][side]))
            here = tuple(d["leads"][side])
            runs[-1].append(here)
        else:
            found = reach(net, here)
            side = min((0, 1), key=lambda i: cost(found, d["leads"][i]))
            go(tuple(d["leads"][side]), name, tail=[tuple(d["heads"][side])])
            if k < len(names) - 1 and not failed:             # over the deck, and on from the far bridgehead
                used_decks.append(name)
                runs.append([tuple(d["heads"][1 - side]), tuple(d["leads"][1 - side])])
                here = tuple(d["leads"][1 - side])
        if failed:
            break
    if failed:
        continue
    course = [eased(np.asarray(r, float)) for r in runs if len(r) > 1]
    line = np.concatenate(course)
    for x, y in line:
        iy, ix = cell(x, y)
        used[max(iy - 1, 0):iy + 2, max(ix - 1, 0):ix + 2] = True
    whole = LineString(line).simplify(1.2)
    width, walk = float(a.get("width", 12.0)), float(a.get("sidewalk", 3.5))
    z = map_coordinates(H, [(line[:, 1] - fr.y0) / fr.resolution, (line[:, 0] - fr.x0) / fr.resolution], order=1, mode="nearest")
    run = np.hypot(*np.diff(line, axis=0).T)
    win = max(int(40.0 / max(run.mean(), 0.1)), 1)             # gradient over 40 m, as a road is felt
    steep = float(np.max(np.abs(z[win:] - z[:-win]) / (run.mean() * win))) if len(z) > win else 0.0
    straight = math.hypot(*(line[-1] - line[0]))
    report.append({"id": f["id"], "length_m": round(whole.length), "straight_m": round(straight), "steepest": round(steep, 3), "decks": used_decks})
    if steep > float(p.get("max_grade", 0.10)):
        k = int(np.argmax(np.abs(z[win:] - z[:-win])))
        s.warn("main_way.steep", f["id"], f"the gentlest course found still climbs at {steep:.0%} near ({line[k][0]:.0f}, {line[k][1]:.0f}); this world allows {float(p.get('max_grade', 0.10)):.0%}",
               where=tuple(line[k]), hint="give it a `via` round the high ground, or accept a steeper way")
    s.add("feature", {"id": f["id"] + "_axis", "kind": "axis", "smooth": False, "of": f["id"], "path": [[round(x, 1), round(y, 1)] for x, y in whole.coords]})
    for k, piece in enumerate(course):
        land = LineString(piece)
        short = float(p.get("stop_short", 6.0))                # a street's own end reaches on to the bridgehead; its line stops short of it
        at_head = [any(math.hypot(q[0] - hd[0], q[1] - hd[1]) < 0.5 for d_ in decks.values() for hd in d_["heads"]) for q in (piece[0], piece[-1])]
        if land.length > 3 * short:
            land = substring(land, short if at_head[0] else 0.0, land.length - (short if at_head[1] else 0.0))
        land = land.simplify(1.2)
        s.add("feature", {"id": f"{f['id']}_{k}", "kind": "street", "width": width, "sidewalk": walk, "arterial": 1, "of": f["id"], "smooth": False,
                          "max_grade": 100 * max(float(p.get("max_grade", 0.10)), steep) + 4,
                          "path": [[round(x, 1), round(y, 1)] for x, y in land.coords]})
        if river is not None:
            over = land.intersection(river)
            if over.length > 6:
                mid = over.interpolate(0.5, normalized=True) if over.geom_type == "LineString" else over.centroid
                s.warn("main_way.wades", f["id"], f"it goes through water at ({mid.x:.0f}, {mid.y:.0f}), where no crossing is named", where=(mid.x, mid.y),
                       hint="place a `crossing` on the water there and name it in `via`")
import json
s.out.mkdir(parents=True, exist_ok=True)
(s.out / "main_ways.json").write_text(json.dumps(report, indent=1))
print("main ways:", "; ".join(f"{r['id']} {r['length_m']} m ({r['length_m'] / max(r['straight_m'], 1):.2f}x the straight line, steepest {r['steepest']:.0%})" for r in report))
s.done()
