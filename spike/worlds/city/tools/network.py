"""The road step: everything finer than an arterial, worked out from the arterials.

This is this project's knowledge of roads, not the compiler's. It reads the arterials layer's build
and hands back, as plain features:
  wards          each district, the city's own or one a larger map hands down, cut by the arterials,
                 the river and the other districts into faces, each carrying what its district says
  seam streets   a street where two districts meet and no arterial runs
  the ring       only when the city is built alone, with the whole map as its limits: a street
                 50 m inside the boundary to close it. Given limits by a larger map, there is none;
                 an arterial that leaves by no exit ends on a lane just inside the limits
  waterfronts    for each `waterfront` feature, a street along the bank of the watercourse it names, inside
                 the town; streets that reach the water end on it
  local streets  each ward in its district's pattern: a grid, a grid bent out of true, or rings and
                 spokes about a square; thinned where the map says the city is less built up (`urban`)
                 with `join_within` in this step's params, a street that ends on nothing is carried on to
                 the next street if one lies that near, and cut back to its last junction if not
  parks          a ward a park seed stands in gets no streets, a park region and the `green` field
  bridges        a deck wherever an arterial crosses the river
  ramps          at each interchange, a ramp in every quadrant from the arterial up to the freeway,
                 or a loop where the interchange asks for one
"""
import math

import numpy as np
from PIL import Image, ImageDraw
from shapely import affinity
from shapely.geometry import LineString, Point, Polygon, box
from shapely.ops import linemerge, polygonize, substring, unary_union
from shapely.strtree import STRtree

from viasdk import step

s = step()
p, f = s.params, s.frame
feats = s.build.features
lines = {i: LineString(ft["points"]) for i, ft in feats.items() if ft["shape"] == "path"}
arterials = {i: ft for i, ft in feats.items() if ft["kind"] == "street" and ft.get("arterial")}
river = unary_union([Polygon(ft["points"]) for ft in feats.values() if ft["kind"] == "lake" or (ft["shape"] == "region" and ft.get("water"))])
extent = box(f.x0, f.y0, f.x1, f.y1)
# What a larger map gives this city: where it may be, and how built up. Alone: everywhere, fully.
# A world that plans its own town names the region itself (`limits` in this step's params) and is treated the same.
lim = feats[p.get("limits", "limits")]
limits = Polygon(lim["points"]).buffer(0)
given = bool(lim.get("given")) or "limits" in p
urban = s.build.field("urban")
if not given:
    RING = [[800.0, 50.0], [1520.0, 50.0], [1550.0, 80.0], [1550.0, 1520.0], [1520.0, 1550.0], [80.0, 1550.0], [50.0, 1520.0], [50.0, 80.0], [80.0, 50.0], [800.0, 50.0]]
    s.add("feature", {"id": "ring", "kind": "street", "width": 9.0, "sidewalk": 3.0, "arterial": 1, "smooth": False, "path": RING})
    lines["ring"] = LineString(RING)
    arterials["ring"] = {"attrs": {"width": 9.0, "sidewalk": 3.0}}
main_ways = unary_union([lines[i] for i in arterials])


def built(x, y):
    """How built up a spot is: what the map gives, lifted a little along the arterials, which draw building to them."""
    u = float(urban[f.index(x, y)])
    return min(1.0, u + 0.3 * math.exp(-main_ways.distance(Point(x, y)) / 50.0)) if u > 0.02 else 0.0


def thinned(piece, k, dense=None):
    """Fewer of a grid's streets are laid where less is built: all of them in town, every second further
    out, every fourth at the fringe. Each is kept only along the stretch that is built up enough for it."""
    if dense is not None or not given:      # a district that says how built up it is gets its whole grid
        return [piece]
    need = 0.5 if k % 2 else 0.25 if k % 4 == 2 else 0.1
    n = max(2, int(piece.length // 12))
    pts = [piece.interpolate(i / n, normalized=True) for i in range(n + 1)]
    runs, start = [], None
    for i, ok in enumerate([built(q.x, q.y) >= need for q in pts] + [False]):
        if ok and start is None:
            start = i
        elif not ok and start is not None:
            if (i - 1 - start) * piece.length / n >= 55:
                runs.append(LineString([(q.x, q.y) for q in pts[start:i]]))
            start = None
    return runs
# ---- districts: regions with a character, this city's own or handed down by a larger map. Earlier ones win.
plan = []                                  # (id, the ground it holds once earlier districts have taken theirs, what it says)
taken = None
for ft in feats.values():
    if ft["kind"] != "district":
        continue
    whole = Polygon(ft["points"]).buffer(0).intersection(limits)
    mine = whole if taken is None else whole.difference(taken)
    taken = whole if taken is None else taken.union(whole)
    if not mine.is_empty:
        plan.append((ft["id"], mine, ft))
if not plan:
    s.error("no_districts", "network", "the city has no district to lay out", hint="declare a [[feature]] of kind district, or give some from the larger map")
squares = {i: Polygon(ft["points"]) for i, ft in feats.items() if ft["kind"] == "square"}
for i, sq in squares.items():
    s.add("feature", {"id": f"plaza_{i}", "kind": "lot", "region": [[round(x, 1), round(y, 1)] for x, y in list(sq.exterior.coords)[:-1]]})
    if feats[i].get("landmark"):
        s.add("instance", {"id": f"{i}_landmark", "asset": feats[i]["landmark"], "at": [round(sq.centroid.x, 1), round(sq.centroid.y, 1)]})


def stretched(line, by=12.0):
    """A line carried a little past both ends, so that streets drawn to meet do meet."""
    c = list(line.coords)
    (x0, y0), (x1, y1) = c[0], c[1]
    d = math.hypot(x1 - x0, y1 - y0)
    c[0] = (x0 - (x1 - x0) / d * by, y0 - (y1 - y0) / d * by)
    (x0, y0), (x1, y1) = c[-1], c[-2]
    d = math.hypot(x1 - x0, y1 - y0)
    c[-1] = (x0 - (x1 - x0) / d * by, y0 - (y1 - y0) / d * by)
    return LineString(c)


def lay(line, width, sidewalk, smooth=False, **more):
    s.add("feature", {"id": f"st_{count['street']}", "kind": "street", "width": width, "sidewalk": sidewalk, "smooth": smooth,
                      "path": [[round(x, 1), round(y, 1)] for x, y in line.coords], **more})
    count["street"] += 1


count = {"street": 0, "park": 0, "bridge": 0, "ramp": 0, "ward": 0}
local = []                                 # (line, width, sidewalk, smooth): laid once every ward has had its say
# ---- where two districts meet and no main way runs, a street does: the old edge of one of them
near_main = unary_union([main_ways.buffer(34), river.buffer(20)])
seams = []
for n, (did, mine, ft) in enumerate(plan[:-1]):
    later = unary_union([m for _, m, _ in plan[n + 1:]])
    edge = mine.boundary.intersection(later.buffer(1.0)).difference(near_main)
    for piece in getattr(linemerge(edge) if edge.geom_type.startswith("Multi") else edge, "geoms", [edge]):
        if piece.geom_type == "LineString" and piece.length > 90:
            piece = piece.simplify(5.0)
            seams.append(piece)
            lay(piece, 9.0, 3.0, smooth=True, between=did)
# ---- waterfronts: the bank of a watercourse is a frontage, so a street runs along it and the others end on it
fronts, front_edges = [], []
for ft in feats.values():
    if ft["kind"] != "waterfront":
        continue
    a = ft.get("attrs", ft)
    wet_ft = feats.get(str(ft.get("along")) + "_water")
    if wet_ft is None or wet_ft["shape"] != "region":
        s.error("waterfront.no_water", ft["id"], f"`along` names '{ft.get('along')}', and there is no region '{ft.get('along')}_water' beneath this step",
                hint="name a watercourse the terrain or skeleton step hands back")
        continue
    course = lines[ft["along"]]
    ring = Polygon(wet_ft["points"]).buffer(0).buffer(float(a.get("setback", 17.0)), quad_segs=6).exterior
    want = {"left": (1,), "right": (-1,)}.get(ft.get("sides", "both"), (1, -1))
    for reach_out, keep in ((12.0, front_edges), (-float(a.get("inset", 10.0)), fronts)):      # edges run past the town's edge so that faces close
        cut = ring.intersection(limits.buffer(reach_out)).intersection(Polygon(ft["points"]).buffer(0).buffer(reach_out + 10.0))
        cut = linemerge(cut) if cut.geom_type.startswith("Multi") else cut
        for piece in getattr(cut, "geoms", [cut]):
            if piece.geom_type != "LineString" or piece.length < 80:
                continue
            mid = piece.interpolate(0.5, normalized=True)
            at = course.project(mid)
            c0, c1 = course.interpolate(max(at - 4, 0)), course.interpolate(min(at + 4, course.length))
            side = 1 if (c1.x - c0.x) * (mid.y - c0.y) - (c1.y - c0.y) * (mid.x - c0.x) > 0 else -1
            if side not in want or min(at, course.length - at) < 1.0:      # not the caps the offset draws round the course's ends
                continue
            piece = piece.simplify(1.2)
            keep.append(piece)
            if keep is fronts:
                lay(piece, float(a.get("width", 9.0)), float(a.get("sidewalk", 3.0)), smooth=False, waterfront=ft["id"])
if fronts:
    near_main = unary_union([near_main, *[fr_.buffer(30) for fr_ in fronts]])
ways_to_follow = [lines[i] for i in arterials] + fronts
# ---- a main way that leaves by no exit ends at the edge of town on a lane, not in a field
if given and p.get("close_ways", True):
    exits = [Point(ft.get("at") or ft["points"][0]) for ft in feats.values() if ft["kind"] == "exit"]
    inner = limits.buffer(-16)
    rim_in = max(getattr(inner, "geoms", [inner]), key=lambda g: g.area).exterior
    for i in arterials:
        cut = lines[i].intersection(limits.exterior)
        for pt in getattr(cut, "geoms", [cut]):
            if pt.geom_type != "Point" or any(pt.distance(e) < 120 for e in exits):
                continue
            at = rim_in.project(pt)
            a, b = max(at - 70, 0), min(at + 70, rim_in.length)
            if b - a > 60:
                lane = substring(rim_in, a, b).simplify(3.0)
                seams.append(lane)
                lay(lane, 6.0, 1.5, smooth=True, closes=i)


def bearing_near(pt):
    """The way the nearest main way runs past a spot, in degrees: a grid with no bearing of its own follows it."""
    near = min(ways_to_follow, key=lambda l: l.distance(pt))
    at = near.project(pt)
    a, b = near.interpolate(max(at - 5, 0)), near.interpolate(min(at + 5, near.length))
    return math.degrees(math.atan2(b.y - a.y, b.x - a.x))


def bent(line, amp):
    """A straight street bent out of true, the same way for every street, so that crossings stay crossings."""
    n = max(2, int(line.length // 14))
    out = []
    for k in range(n + 1):
        q = line.interpolate(k / n, normalized=True)
        out.append((q.x + amp * (math.sin(q.y / 44 + 1.3) + 0.5 * math.sin(q.y / 19 + q.x / 61)),
                    q.y + amp * (math.sin(q.x / 39 + 0.7) + 0.5 * math.sin(q.x / 17 + q.y / 53))))
    return LineString(out)


def families(face, ft, centre):
    """The streets a district's pattern asks for across one face, before they are cut to it: (line, its number)."""
    pattern, gap = ft.get("pattern", "grid"), float(ft.get("block", 50.0))
    cx, cy = face.centroid.coords[0]
    reach = math.hypot(*(np.subtract(face.bounds[2:], face.bounds[:2]))) / 2 + 30
    if pattern == "radial":
        ox, oy = centre
        far = max(math.hypot(x - ox, y - oy) for x, y in face.exterior.coords)
        rings = [(LineString([(ox + r * math.cos(t / 48 * 2 * math.pi), oy + r * math.sin(t / 48 * 2 * math.pi)) for t in range(49)]), k)
                 for k, r in enumerate(np.arange(gap * 1.2, far + gap, gap))]
        n = max(6, int(round(2 * math.pi * 0.55 * far / (2.2 * gap))))
        turn = math.radians(float(ft.get("bearing", 0.0)))
        spokes = [(LineString([(ox + gap * 0.6 * math.cos(turn + t / n * 2 * math.pi), oy + gap * 0.6 * math.sin(turn + t / n * 2 * math.pi)),
                               (ox + far * math.cos(turn + t / n * 2 * math.pi), oy + far * math.sin(turn + t / n * 2 * math.pi))]), 2 * t + 1) for t in range(n)]
        return rings + spokes, True
    angle = float(ft["bearing"]) if "bearing" in ft else bearing_near(face.centroid)
    out = []
    for turn, step_m in ((angle, gap), (angle + 90.0, gap * (1.8 if pattern == "organic" else 2.4))):      # two families of parallel streets
        k = -int(reach // step_m) - 1
        while k * step_m <= reach:
            raw = affinity.rotate(LineString([(cx - reach, cy + (k + 0.5) * step_m), (cx + reach, cy + (k + 0.5) * step_m)]), turn, origin=(cx, cy))
            out.append((bent(raw, 0.38 * gap) if pattern == "organic" else raw, k))
            k += 1
    return out, pattern == "organic"


# ---- wards: the faces the main ways, the river and the district edges cut the districts into
edges = [stretched(lines[i]) for i in arterials] + [river.boundary] + seams + front_edges      # not the map's edge: a face must be closed by the city's own ways
edges += [g.exterior for _, mine, _ in plan for g in getattr(mine, "geoms", [mine]) if g.geom_type == "Polygon"]
faces = [fc for fc in polygonize(unary_union(edges)) if fc.area > 2500 and extent.contains(fc.representative_point())
         and limits.contains(fc.representative_point()) and not river.contains(fc.representative_point())]
parks = [Point(ft["at"]) for ft in feats.values() if ft["kind"] == "seed" and ft.get("zone") == "park"]
green = Image.new("L", (f.nx, f.ny), 0)
for face in faces:
    here = face.representative_point()
    did, mine, ft = next(((d, m, t) for d, m, t in plan if m.contains(here)), (None, None, None))
    if did is None:
        continue
    if any(face.contains(pt) for pt in parks):
        inner = face.buffer(-16)
        for part in getattr(inner, "geoms", [inner]):
            if part.is_empty or part.area < 2000:
                continue
            part = part.simplify(1.5)
            s.add("feature", {"id": f"park_{count['park']}", "kind": "park", "region": [[round(x, 1), round(y, 1)] for x, y in list(part.exterior.coords)[:-1]]})
            ImageDraw.Draw(green).polygon([((x - f.x0) / f.resolution, (y - f.y0) / f.resolution) for x, y in part.exterior.coords], fill=1)
            count["park"] += 1
        continue
    dense = ft.get("density")
    s.add("feature", {"id": f"w_{did}_{count['ward']}", "kind": "ward", "district": did, "buildings": ",".join(ft.get("buildings", [])),
                      "floors_lo": float(ft.get("floors", [1, 99])[0]), "floors_hi": float(ft.get("floors", [1, 99])[1]),
                      "density": -1.0 if dense is None else float(dense),
                      "region": [[round(x, 1), round(y, 1)] for x, y in list(face.exterior.coords)[:-1]]})
    count["ward"] += 1
    gap = float(ft.get("block", 50.0))
    sq = squares.get(ft.get("around", ""))
    centre = sq.centroid.coords[0] if sq is not None else Polygon(ft["points"]).centroid.coords[0]
    rim = face.exterior
    wanted, smooth = families(face, ft, centre)
    for raw, k in wanted:
        cut = raw.intersection(face)
        for whole in getattr(cut, "geoms", [cut]):
            if whole.geom_type != "LineString" or whole.length < 55:
                continue
            if whole.intersection(rim.buffer(0.42 * gap)).length > 0.5 * whole.length:
                continue                                      # it would run beside a main way, not between two
            for piece in thinned(whole, k, dense):
                mid = piece.interpolate(0.5, normalized=True)
                lane = dense is None and given and built(mid.x, mid.y) < 0.35      # out here it is a lane, not a street
                local.append([piece.simplify(1.0) if smooth else piece, 5.0 if lane else p["local_width"], 1.0 if lane else p["local_sidewalk"], smooth])

# ---- loose ends. A street that ends on nothing is carried straight on to the next street if one is
# near, and otherwise cut back to its last junction. What is left ending on nothing is a lane that
# is long enough to be meant.
if "join_within" in p:
    within, tol = float(p["join_within"]), 2.5
    fixed = [lines[i] for i in arterials] + fronts + seams
    joined = trimmed = dropped = 0
    for _ in range(3):
        others = fixed + [row[0] for row in local if row[0] is not None]
        tree = STRtree(others)
        for row in local:
            line = row[0]
            if line is None:
                continue
            near = [others[i] for i in tree.query(line.buffer(within)) if others[i] is not line]
            met = unary_union(near) if near else None
            c = list(line.coords)
            for end, nxt in ((0, 1), (-1, -2)):
                tip = Point(c[end])
                if met is not None and met.distance(tip) <= tol:
                    continue
                (x0, y0), (x1, y1) = c[end], c[nxt]
                d = math.hypot(x0 - x1, y0 - y1) or 1.0
                ahead = LineString([c[end], (x0 + (x0 - x1) / d * within, y0 + (y0 - y1) / d * within)])
                hit = ahead.intersection(met) if met is not None else None
                if hit is not None and not hit.is_empty:
                    to = min((Point(q) for g_ in getattr(hit, "geoms", [hit]) for q in g_.coords), key=tip.distance)
                    if not LineString([c[end], (to.x, to.y)]).intersects(river) and limits.buffer(12).contains(to):
                        c[end] = (to.x, to.y) if tip.distance(to) < 1.0 else c[end]
                        if tip.distance(to) >= 1.0:
                            c.insert(0, (to.x, to.y)) if end == 0 else c.append((to.x, to.y))
                        joined += 1
                        continue
                # nothing to join: back to the last junction, if the tail is short
                now = LineString(c)
                cross = now.intersection(met) if met is not None else None
                at = sorted(now.project(Point(q)) for g_ in getattr(cross, "geoms", [cross] if cross is not None and not cross.is_empty else []) for q in g_.coords)
                if not at:
                    c = None
                    dropped += 1
                    break
                tail = at[0] if end == 0 else now.length - at[-1]
                if tail < float(p.get("stub", within)):
                    cut = substring(now, at[0], now.length) if end == 0 else substring(now, 0.0, at[-1])
                    c = list(cut.coords) if cut.geom_type == "LineString" and cut.length > 1 else None
                    trimmed += 1
                    if c is None:
                        dropped += 1
                        break
            row[0] = LineString(c) if c is not None and len(c) > 1 and LineString(c).length >= 30 else None
    print(f"loose ends: {joined} carried on to the next street, {trimmed} cut back to a junction, {dropped} streets dropped")
for line, width, sidewalk, smooth in local:
    if line is not None:
        lay(line, width, sidewalk, smooth=smooth)
s.write_field("green", np.asarray(green, dtype=np.float32)[::1])

# ---- bridges: where an arterial meets the river
for i, ft in arterials.items():
    a = ft["attrs"]
    cut = lines[i].intersection(river.buffer(14))
    for piece in getattr(cut, "geoms", [cut]):
        if piece.geom_type == "LineString" and piece.length > 20:
            s.add("feature", {"id": f"bridge_{count['bridge']}", "kind": "bridge", "width": a["width"] + 2 * a["sidewalk"], "rise": 1.6, "ramp": 16.0,
                              "max_grade": 20.0, "smooth": False, "path": [[round(x, 1), round(y, 1)] for x, y in piece.coords]})
            count["bridge"] += 1

# ---- interchanges: a ramp in each quadrant, or a loop
for ft in feats.values():
    if ft["kind"] != "interchange":
        continue
    fw, cross = lines.get(ft["freeway"]), lines.get(ft["cross"])
    if fw is None or cross is None:
        s.error("unknown_way", ft["id"], f"it names '{ft['freeway']}' and '{ft['cross']}'; one of them is not a way in the arterials layer")
        continue
    hit = fw.intersection(cross)
    if hit.is_empty:
        s.error("no_crossing", ft["id"], f"'{ft['freeway']}' and '{ft['cross']}' never cross", where=ft["at"])
        continue
    j = min(getattr(hit, "geoms", [hit]), key=lambda g: g.distance(Point(ft["at"])))
    at = fw.project(j)
    a, b = fw.interpolate(max(at - 2, 0)), fw.interpolate(min(at + 2, fw.length))
    u = np.array([b.x - a.x, b.y - a.y]) / math.hypot(b.x - a.x, b.y - a.y)       # along the freeway
    v = np.array([-u[1], u[0]])                                                    # to its left
    J = np.array([j.x, j.y])
    half_f = feats[ft["freeway"]]["attrs"]["width"] / 2
    ca = feats[ft["cross"]]["attrs"]
    half_c = ca["width"] / 2 + ca.get("sidewalk", 0.0)
    loop = tuple(ft.get("loop", ()))
    for su in (1, -1):
        for sv in (1, -1):
            rid = f"{ft['id']}_{'n' if sv > 0 else 's'}{'e' if su > 0 else 'w'}"
            if loop == (su, sv):                 # three quarters of a circle, tangent to both ways
                r = p["loop_radius"]
                c = J + v * sv * (half_f + 3.0 + r) + u * su * (half_c + 2.0 + r)
                pts = [c + r * (math.cos(math.radians(d)) * u * su + math.sin(math.radians(d)) * v * sv) for d in range(180, -91, -15)]
                pts = [pts[0] - v * sv * 26.0] + pts          # a lead-in along the arterial
            else:                                # a straight ramp beside the freeway
                L = p["ramp_length"]
                far = fw.interpolate(min(max(at + su * L, 0), fw.length))
                mid = fw.interpolate(min(max(at + su * L * 0.55, 0), fw.length))
                pts = [J + u * su * (half_c + 3.0) + v * sv * (half_f + 11.0),
                       np.array([mid.x, mid.y]) + v * sv * (half_f + 7.5),
                       np.array([far.x, far.y]) + v * sv * (half_f + 3.0)]
            s.add("feature", {"id": rid, "kind": "ramp", "to_on": ft["freeway"], "path": [[round(float(q[0]), 1), round(float(q[1]), 1)] for q in pts]})
            count["ramp"] += 1
s.done()
