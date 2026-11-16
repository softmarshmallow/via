"""The building library's lay step: one building feature becomes storeys, rooms, flights, doorways,
footpaths, its wall runs and its roof, in one returned document.

A generate step, `for_each = "kind:building"`, run as a worker through the SDK (kinds.toml shows
the table a world restates). It reads the building's outline and attributes (`build.features`),
the ground under the outline (`expr.eval` of the terrain), and the kit (`pack.query`). Everything it
returns is keyed by the building's name, so no distant edit shifts a key (identity.md 2.2), and
everything is in one document because a produced name is never written in text (R3,
`identity.key_in_text`): a second step could not name this one's storeys in `on` or `surface`.

The rules (ours, written for this library; README.md is the design note):

1. **Outline.** The building's region, one ring, counter-clockwise (a clockwise ring is read
   reversed from its first point). `t` along it is arc length from its first point, as `on` takes
   it (geometry.md 2.6). Each edge has its direction `e` and its inward normal `n` (the left one).
2. **Levels.** `level_k = ground + plinth + k * storey_h`, in world metres, where `ground` is the
   mean of the terrain read at every post inside the outline (edges included) and at its vertices,
   rounded to the millimetre, and every level is rounded to the millimetre so a floor and the f32
   post under its sill round to one millimetre (surfaces.md 3.4, `gap`).
3. **Stair.** `stair = { t, width, run }`: the flights climb along the outline edge holding `t`,
   from `s0` (the point at `t`, along that edge) to `s1 = s0 + run`, in lanes `CLEAR` (0.1 m) off
   the wall's inner face and off each other: lane 0's centre is `T + CLEAR + width / 2` in, lane
   1's one lane further, where `T` is the kit's wall thickness (0 with no kit: surfaces.md 6.4's
   `[21, 20.6]`). Measured from the outline, a 0.3 m wall stood 0.2 m into a 1 m flight. Flight `k` joins storey `k` to storey `k + 1`: an even flight climbs lane 0 from `s0`
   to `s1`, an odd one lane 1 back from `s1` to `s0` (a switchback). Storey `k + 1` loses its
   stairwell, a notch from the wall: for an even flight `[s0 - LANDING, s1]` one lane deep, for an
   odd one `[s0, s1 + LANDING]` two lanes deep and `T` more (both clamped to the edge), so each flight's end
   lies on the notch's side (on the outline: inside, surfaces.md 1.2) and the next flight starts on
   the floor. A notch that reaches a corner needs the next edge square to this one, or it is
   refused (`params`). Every flight `starts_at = "<storey k>.floor"`, `ends_at = "<storey
   k+1>.floor"`, with `z0`, `z1` the two levels (surfaces.md 6.4).
4. **Doors.** `doors = [{ storey, t, width }]`: a doorway square to the outline at `t`, from
   `porch` metres outside to the outline, `ends_at = "<storey>.floor"`, `sill_z` its level, `flat`
   at least the posts' diagonal so the sill effect holds every post round the end (kinds.toml). A
   `footpath` runs from `walk_out` metres beyond the doorway's start and `ends_at` its start; its
   own start is left free (surfaces.md 3.4: a free end is answered by nothing and checked for
   nothing), for a hand path or a way's footpath to meet. A door on an upper storey needs a
   balcony (README) and is refused for now; a door inside the stairwell's stretch is refused.
5. **Rooms.** `rooms = [{ storey, shares }]`: the storey's floor is cut across the footprint's
   longest edge (the first of equal ones) at the cumulative `shares`, each piece a `room` region
   with `storey` naming its storey. A cut that crosses the outline more than twice (the room would
   be two pieces) is refused. Two rooms beside each other share an opening `inner_door` (0.9 m)
   wide at the middle of their cut: the wall pieces there leave it open (the walk crosses it).
6. **Walls** (with `kit`, a taxon path of pieces tagged `role:wall`, `role:corner`, `role:filler`,
   `role:door`, `role:roof`): along every stretch of a storey's outline that lies on the footprint's
   outline, less the corners and a clear gap for each doorway and flight that crosses it, one `run` of wall pieces at their pitch
   (`params.pitch`, else their `bounds` width) and one of fillers for what is left: the most wall
   pieces such that the rest is a whole number of fillers within `FIT` (5 mm). A stretch that does
   not close is `wall_open` (warning) naming the lengths near it that would. Runs stand on the
   storey's floor (`on` a storey: its one surface, surfaces.md 4.1), `offset` half a wall's
   thickness inward, their front (+y) outward. A corner piece stands in each corner's square, a
   door leaf in each doorway, an inner wall between rooms leaving its opening, each `at` with
   `surface = "<storey>.floor"` (surfaces.md 4.1, M5).
   **The clear gap.** A doorway or a flight crosses a stretch where its centreline comes within
   `width / 2` of the wall's band (from the outline to `T` in). The wall leaves it `width + margin`
   (`params.margin`, 0.1 m) clear, centred on where it crosses, and each side of the gap is pulled
   back by the overhang of the pieces beside it (half a piece's `bounds` width past its pitch), so
   the clear width between wall instances is never less than the path's width plus the margin.
   Before, the gap was the door's width to the pitch, and a piece wider than its pitch closed it
   (TODO, M5 slice 1: the station's front door 0.20 m clear, its tower step 0.29 m).
7. **Roof.** `roof = "garden"`: a `roof` region over the footprint whose `garden` is at the top
   storey's level plus `storey_h`. Otherwise, with a `role:roof` piece in the kit, one instance at
   the footprint's middle on the top floor, facing out of the longest edge.

`params` of the step: `inner_door` (0.9 m), `margin` (0.1 m). Codes are `step.<id>.<code>`: `params` (error, the
building's attribute), `wall_open` (warning), `laid` (info, what was returned).

Library code imports only viasdk and the standard library (R19b).
"""

import json
import math

from viasdk import RpcError

CLEAR = 100          # mm between a flight and the wall, and between two lanes
LANDING = 1000       # mm of stairwell before an even flight's start and past an odd one's
FIT = 5              # mm a stretch may differ from its pieces' pitches and still close
INNER_DOOR = 900     # mm, an opening between two rooms
MIN_FLAT = 1500      # mm, the least flat ground beside a doorway
MARGIN = 100         # mm a wall's gap is wider than the doorway or flight through it

EXPLAIN = {
    "params": "An attribute of a building is outside what the building library's lay step takes; "
    "the message names the attribute and what it must be (lib/building/lay.py, its rules).",
    "wall_open": "A stretch of wall cannot be closed with whole wall pieces and fillers at their "
    "pitch: the pieces would leave a gap or overlap. The data lists lengths near it that would "
    "close; move a door or a corner, or add a filler of another pitch to the kit.",
    "laid": "What the building library's lay step returned for one building: storeys, rooms, "
    "flights, doorways, footpaths, wall runs and pieces.",
}


class ParamError(ValueError):
    def __init__(self, key, message, hint):
        super().__init__(message)
        self.key, self.message, self.hint = key, message, hint


# --- geometry, in millimetres ----------------------------------------------------------------


def area2(ring):
    return sum(a[0] * b[1] - b[0] * a[1] for a, b in zip(ring, ring[1:] + ring[:1]))


def ccw(ring):
    """The ring without a closing repeat, counter-clockwise; a clockwise one reversed from its
    first point, so `t = 0` stays there."""
    ring = [(int(p[0]), int(p[1])) for p in ring]
    if len(ring) > 1 and ring[0] == ring[-1]:
        ring = ring[:-1]
    if area2(ring) < 0:
        ring = [ring[0]] + ring[:0:-1]
    return ring


def edges(ring):
    """One record per edge: `a`, `b`, length `L` (mm, float), unit `e`, inward normal `n`."""
    out = []
    for a, b in zip(ring, ring[1:] + ring[:1]):
        dx, dy = b[0] - a[0], b[1] - a[1]
        L = math.hypot(dx, dy)
        e = (dx / L, dy / L)
        out.append({"a": a, "b": b, "L": L, "e": e, "n": (-e[1], e[0])})
    return out


def at_t(es, t):
    """(edge index, mm along it) of the point at `t` (a fraction of the outline's arc length);
    a point on a vertex is held by the outgoing edge, `t = 1` by the last (geometry.md 2.6)."""
    total = sum(e["L"] for e in es)
    s = t * total
    c = 0.0
    for k, e in enumerate(es):
        if s < c + e["L"] or k == len(es) - 1:
            return k, min(max(s - c, 0.0), e["L"])
        c += e["L"]
    raise AssertionError("unreachable")


def local(e, s, v):
    """The point `s` mm along edge `e` and `v` mm in from it."""
    return (e["a"][0] + e["e"][0] * s + e["n"][0] * v, e["a"][1] + e["e"][1] * s + e["n"][1] * v)


def rnd(p):
    return (int(round(p[0])), int(round(p[1])))


def bearing(v):
    """Degrees clockwise from north of a plan vector, to the millidegree, in [0, 360)."""
    b = math.degrees(math.atan2(v[0], v[1])) % 360.0
    b = round(b, 3)
    return 0.0 if b >= 360.0 else b


def outward(e):
    return bearing((-e["n"][0], -e["n"][1]))


def inside(ring, p):
    """Point in ring, the edge counting as inside (surfaces.md 1.2)."""
    x, y = p
    hit = False
    for a, b in zip(ring, ring[1:] + ring[:1]):
        cross = (b[0] - a[0]) * (y - a[1]) - (b[1] - a[1]) * (x - a[0])
        if cross == 0 and min(a[0], b[0]) <= x <= max(a[0], b[0]) and min(a[1], b[1]) <= y <= max(a[1], b[1]):
            return True
        if (a[1] > y) != (b[1] > y):
            xi = a[0] + (y - a[1]) * (b[0] - a[0]) / (b[1] - a[1])
            if x < xi:
                hit = not hit
    return hit


def same_dir(u, v):
    return abs(u[0] * v[1] - u[1] * v[0]) < 1e-9 and u[0] * v[0] + u[1] * v[1] > 0


# --- levels -----------------------------------------------------------------------------------


def levels(ground_m, plinth, storey_h, storeys):
    g = round(ground_m, 3)
    return [round(g + plinth + k * storey_h, 3) for k in range(storeys)]


def posts_in(ring, grid):
    """Every post of the grid inside the ring (edges included), then the ring's vertices."""
    out = []
    if grid and grid.get("resolution_mm") and grid.get("extent_mm"):
        x0, y0 = int(grid["extent_mm"][0]), int(grid["extent_mm"][1])
        r = int(grid["resolution_mm"])
        nx, ny = int(grid.get("nx", 0)), int(grid.get("ny", 0))
        i0 = max(0, -(-(min(p[0] for p in ring) - x0) // r))
        i1 = min(nx - 1, (max(p[0] for p in ring) - x0) // r)
        j0 = max(0, -(-(min(p[1] for p in ring) - y0) // r))
        j1 = min(ny - 1, (max(p[1] for p in ring) - y0) // r)
        for j in range(j0, j1 + 1):
            for i in range(i0, i1 + 1):
                p = (x0 + i * r, y0 + j * r)
                if inside(ring, p):
                    out.append(p)
    return out + list(ring)


# --- stair ------------------------------------------------------------------------------------


def stair_plan(ring, stair, storeys, wall_t=0):
    """Flights and stairwells (rule 3). Returns {flights: [(path mm, lower, upper)], notches:
    {storey: (edge, a, b, depth)}} with `a`, `b` mm along the edge."""
    if storeys < 2:
        return {"flights": [], "notches": {}, "edge": None}
    if stair is None:
        raise ParamError("stair", f"a building of {storeys} storeys has no `stair`",
                         "write `stair = { t, width, run }`: where along the outline the flights climb")
    es = edges(ring)
    k, s0 = at_t(es, float(stair["t"]))
    e = es[k]
    w = int(round(float(stair.get("width", 1)) * 1000))
    run = int(round(float(stair["run"]) * 1000))
    s1 = s0 + run
    if w <= 0 or run <= 0 or s1 > e["L"] + 1e-6:
        raise ParamError("stair", f"the stair runs {run / 1000:g} m from {s0 / 1000:g} m along an edge "
                         f"{e['L'] / 1000:g} m long", "move `t` or shorten `run` so the flight stays on one edge")
    lane = [wall_t + CLEAR + w / 2, wall_t + CLEAR + w / 2 + (w + CLEAR)]
    depth = [wall_t + w + 2 * CLEAR, wall_t + 2 * w + 3 * CLEAR]
    flights, notches = [], {}
    for f in range(storeys - 1):
        if f % 2 == 0:
            path = [rnd(local(e, s0, lane[0])), rnd(local(e, s1, lane[0]))]
            notch = (k, max(0.0, s0 - LANDING), s1, depth[0])
        else:
            path = [rnd(local(e, s1, lane[1])), rnd(local(e, s0, lane[1]))]
            notch = (k, s0, min(e["L"], s1 + LANDING), depth[1])
        flights.append((path, f, f + 1))
        notches[f + 1] = notch
    return {"flights": flights, "notches": notches, "edge": k, "span": (s0, s1)}


def notch_ring(ring, notch):
    """The ring less a rectangle on edge `k` from `a` to `b` along it, `depth` in (rule 3). At a
    corner the rectangle takes the corner, which must be square."""
    k, a, b, depth = notch
    es = edges(ring)
    e = es[k]
    n = len(ring)
    start_corner = a <= 0.5
    end_corner = b >= e["L"] - 0.5
    prev, nxt = es[(k - 1) % n], es[(k + 1) % n]
    if start_corner and not same_dir(prev["e"], (-e["n"][0], -e["n"][1])):
        raise ParamError("stair", "the stairwell reaches a corner that is not square",
                         "move the stair's `t` off the corner")
    if end_corner and not same_dir(nxt["e"], e["n"]):
        raise ParamError("stair", "the stairwell reaches a corner that is not square",
                         "move the stair's `t` or shorten `run`")
    if (start_corner and prev["L"] <= depth) or (end_corner and nxt["L"] <= depth):
        raise ParamError("stair", "the stairwell is deeper than the wall beside it",
                         "make the building deeper or the stair narrower")
    bite = []
    if not start_corner:
        bite.append(rnd(local(e, a, 0)))
    bite.append(rnd(local(e, a, depth)))
    bite.append(rnd(local(e, b, depth)))
    if not end_corner:
        bite.append(rnd(local(e, b, 0)))
    out = []
    for i, p in enumerate(ring):
        if i == k and start_corner:
            pass                                 # the corner vertex goes; the bite starts there
        elif i == (k + 1) % n and end_corner:
            continue
        else:
            out.append(p)
        if i == k:
            out.extend(bite)
    return out


# --- doors ------------------------------------------------------------------------------------


def door_plan(ring, door, porch, walk_out):
    """A doorway path (outside to the outline), a footpath (further out, ending at the doorway's
    start), the edge and the interval along it the door opens (rule 4)."""
    es = edges(ring)
    k, s = at_t(es, float(door["t"]))
    e = es[k]
    w = int(round(float(door.get("width", 1)) * 1000))
    if s - w / 2 < 0 or s + w / 2 > e["L"]:
        raise ParamError("doors", f"a door {w / 1000:g} m wide at t = {door['t']} runs past a corner",
                         "move `t` along the wall or narrow the door")
    on = rnd(local(e, s, 0))
    out = rnd(local(e, s, -porch))
    far = rnd(local(e, s, -(porch + walk_out)))
    return {"edge": k, "s": s, "width": w, "doorway": [out, on], "footpath": [far], "at": on}


# --- rooms ------------------------------------------------------------------------------------


def long_axis(ring):
    es = edges(ring)
    best = max(range(len(es)), key=lambda i: (es[i]["L"], -i))
    return es[best]["e"]


def _u(p, ax):
    return p[0] * ax[0] + p[1] * ax[1]


def _cut(p, q, ax, c):
    up, uq = _u(p, ax), _u(q, ax)
    lam = (c - up) / (uq - up)
    return rnd((p[0] + lam * (q[0] - p[0]), p[1] + lam * (q[1] - p[1])))


def clip(ring, ax, c, keep_above):
    """Sutherland-Hodgman against the half-plane `u >= c` (or `<=`)."""
    def ok(p):
        return _u(p, ax) >= c if keep_above else _u(p, ax) <= c
    out = []
    for i, cur in enumerate(ring):
        prev = ring[i - 1]
        if ok(cur):
            if not ok(prev):
                out.append(_cut(prev, cur, ax, c))
            out.append(cur)
        elif ok(prev):
            out.append(_cut(prev, cur, ax, c))
    dedup = []
    for p in out:
        if not dedup or dedup[-1] != p:
            dedup.append(p)
    if len(dedup) > 1 and dedup[0] == dedup[-1]:
        dedup.pop()
    return simplify(dedup)


def simplify(ring):
    """The ring less every vertex in line with its two neighbours: a straight one, or a spike a
    cut along an edge leaves (the cut through a stairwell's side)."""
    ring = list(ring)
    changed = True
    while changed and len(ring) > 3:
        changed = False
        for i in range(len(ring)):
            a, p, b = ring[i - 1], ring[i], ring[(i + 1) % len(ring)]
            if (p[0] - a[0]) * (b[1] - p[1]) - (p[1] - a[1]) * (b[0] - p[0]) == 0:
                del ring[i]
                changed = True
                break
    return ring


def crossings(ring, ax, c):
    """The points where the line `u = c` crosses the ring's edges, ordered along it."""
    pts = []
    for i, cur in enumerate(ring):
        prev = ring[i - 1]
        up, uc = _u(prev, ax) - c, _u(cur, ax) - c
        if (up < 0 < uc) or (uc < 0 < up):
            pts.append(_cut(prev, cur, ax, c))
        elif uc == 0 and up != 0:
            pts.append(cur)
    norm = (-ax[1], ax[0])
    return sorted(set(pts), key=lambda p: _u(p, norm))


def room_plan(ring, footprint, shares, inner=INNER_DOOR):
    """Rooms of one storey (rule 5): their rings, and the openings between neighbours as
    `(i, j, centre, half-chord unit, width, cut ends)`."""
    shares = [float(s) for s in shares]
    if not shares or any(s <= 0 for s in shares):
        raise ParamError("rooms", "`shares` must be a list of numbers above 0",
                         "write one share per room, such as [2, 1]")
    ax = long_axis(footprint)
    us = [_u(p, ax) for p in ring]
    lo, hi = min(us), max(us)
    total = sum(shares)
    cum, cuts = 0.0, []
    for s in shares[:-1]:
        cum += s
        cuts.append(lo + (hi - lo) * cum / total)
    rooms, openings = [], []
    for i in range(len(shares)):
        r = ring
        if i > 0:
            r = clip(r, ax, cuts[i - 1], True)
        if i < len(shares) - 1:
            r = clip(r, ax, cuts[i], False)
        rooms.append(r)
    for i, c in enumerate(cuts):
        pts = crossings(ring, ax, c)
        if len(pts) != 2:
            raise ParamError("rooms", f"the cut between rooms {i} and {i + 1} crosses the storey's "
                             f"outline {len(pts)} times", "change `shares` so each room is one piece")
        a, b = pts
        L = math.dist(a, b)
        if L < inner:
            raise ParamError("rooms", f"rooms {i} and {i + 1} meet along {L / 1000:g} m, less than "
                             "their opening", "change `shares`")
        mid = ((a[0] + b[0]) / 2, (a[1] + b[1]) / 2)
        openings.append({"rooms": (i, i + 1), "at": rnd(mid), "ends": (a, b), "width": inner})
    return rooms, openings


# --- the kit and the walls --------------------------------------------------------------------


def fit(length, pitch, filler, tol=FIT):
    """The most wall pieces `n` and fillers `k` with `n * pitch + k * filler` within `tol` of
    `length` (all mm), or None and the nearest lengths below and above that would close."""
    best = None
    for n in range(int(length // pitch), -1, -1):
        rest = length - n * pitch
        if filler:
            k = int(round(rest / filler))
            if abs(rest - k * filler) <= tol:
                best = (n, k)
                break
        elif abs(rest) <= tol:
            best = (n, 0)
            break
    if best is not None:
        return best, None
    closes = sorted({n * pitch + k * (filler or 0) for n in range(int(length // pitch) + 2)
                     for k in (range(0, int(length // filler) + 2) if filler else (0,))})
    below = max((c for c in closes if c < length), default=None)
    above = min((c for c in closes if c > length), default=None)
    return None, (below, above)


def kit_pieces(rows):
    """{role: {id, pitch (mm), thick (mm)}} from pack rows (id, tags, bounds, params); the first
    id bytewise per role."""
    out = {}
    for r in sorted(rows, key=lambda r: r["id"]):
        for tag in r["tags"]:
            if not tag.startswith("role:"):
                continue
            role = tag[5:]
            if role in out:
                continue
            b = r["bounds"]
            width = b[3] - b[0] if b else 0
            thick = b[4] - b[1] if b else 0
            pitch = r["params"].get("pitch")
            out[role] = {"id": r["id"], "pitch": int(round(float(pitch) * 1000)) if pitch else width,
                         "thick": thick, "width": width}
    return out


def overhang(kit):
    """mm a wall or filler piece reaches past its pitch at each end (the larger of the two)."""
    return max([0.0] + [max(0.0, (p.get("width", p["pitch"]) - p["pitch"]) / 2)
                        for p in (kit.get("wall"), kit.get("filler")) if p])


def crossing_gap(e, path, width, T, half):
    """The interval `(u0, u1)` mm along edge `e` a path of `width` (mm) crossing the wall's band
    (from the outline to `T` in) needs clear, `half` mm each side of where its centreline lies
    within `width / 2` of the band (rule 6); None where it does not come near."""
    lo, hi = -width / 2, T + width / 2
    us = []
    for p, q in zip(path, path[1:]):
        up = (p[0] - e["a"][0]) * e["e"][0] + (p[1] - e["a"][1]) * e["e"][1]
        uq = (q[0] - e["a"][0]) * e["e"][0] + (q[1] - e["a"][1]) * e["e"][1]
        vp = (p[0] - e["a"][0]) * e["n"][0] + (p[1] - e["a"][1]) * e["n"][1]
        vq = (q[0] - e["a"][0]) * e["n"][0] + (q[1] - e["a"][1]) * e["n"][1]
        if abs(vq - vp) < 1e-9:
            if not lo <= vp <= hi:
                continue
            l0, l1 = 0.0, 1.0
        else:
            a, b = (lo - vp) / (vq - vp), (hi - vp) / (vq - vp)
            l0, l1 = max(0.0, min(a, b)), min(1.0, max(a, b))
            if l0 > l1:
                continue
        us += [up + (uq - up) * l0, up + (uq - up) * l1]
    if not us or max(us) < -half or min(us) > e["L"] + half:
        return None
    return (min(us) - half, max(us) + half)


def wall_stretches(ring, footprint, crossers=(), T=0, margin=MARGIN, over=0.0):
    """Stretches of `ring` on the footprint's outline, as `(ring edge, a, b)` mm along it, with the
    gaps the doorways and flights crossing it need (`crossers`: `(path, width)` in mm; rule 6); and
    the footprint corners the ring keeps, as `(point, edge before, edge after, convex)`."""
    fes = edges(footprint)
    res = edges(ring)
    out, corners = [], []
    fset = {p: i for i, p in enumerate(footprint)}
    for i, re in enumerate(res):
        host = None
        for k, fe in enumerate(fes):
            if not same_dir(re["e"], fe["e"]):
                continue
            da = (re["a"][0] - fe["a"][0]) * fe["n"][0] + (re["a"][1] - fe["a"][1]) * fe["n"][1]
            if abs(da) > 0.5:
                continue
            off = (re["a"][0] - fe["a"][0]) * fe["e"][0] + (re["a"][1] - fe["a"][1]) * fe["e"][1]
            if -0.5 <= off <= fe["L"] + 0.5:
                host = (k, off)
                break
        if host is None:
            continue
        gaps = []
        for path, w in crossers:
            g = crossing_gap(re, path, w, T, (w + margin) / 2 + over)
            if g is not None:
                gaps.append(g)
        out.append({"edge": i, "L": re["L"], "gaps": sorted(gaps), "start_corner": re["a"] in fset,
                    "end_corner": re["b"] in fset, "e": re})
    for i, p in enumerate(ring):
        if p not in fset:
            continue
        before, after = res[i - 1], res[i]
        turn = before["e"][0] * after["e"][1] - before["e"][1] * after["e"][0]
        corners.append({"at": p, "before": before, "after": after, "convex": turn > 0})
    return out, corners


def convex_at(corners, p):
    for c in corners:
        if c["at"] == p:
            return c["convex"]
    return False


def wall_plan(ring, footprint, doors, kit, prefix, storey_key, crossers=(), margin=MARGIN):
    """Runs and instances for one storey's walls (rule 6). Returns (runs, instances, open)."""
    wall, filler = kit.get("wall"), kit.get("filler")
    runs, insts, opens = [], [], []
    if wall is None:
        return runs, insts, opens
    T = wall["thick"]
    stretches, corners = wall_stretches(ring, footprint, crossers, T, margin, overhang(kit))
    res = edges(ring)
    starts, c = [], 0.0
    for e in res:
        starts.append(c)
        c += e["L"]
    total = c
    for st in stretches:
        e = st["e"]
        a = T if st["start_corner"] and convex_at(corners, e["a"]) else 0.0
        b = st["L"] - (T if st["end_corner"] and convex_at(corners, e["b"]) else 0.0)
        pieces, at, after_gap = [], a, False
        for g0, g1 in st["gaps"]:
            pieces.append((at, min(g0, b), after_gap, True))
            at, after_gap = max(at, g1), True
        pieces.append((at, b, after_gap, False))
        step_max = max(wall["pitch"], filler["pitch"] if filler else 0)
        for j, (p0, p1, gap0, gap1) in enumerate(pieces):
            L = p1 - p0
            if L <= FIT:
                continue
            got, near = fit(L, wall["pitch"], filler["pitch"] if filler else 0)
            if got is None and (gap0 or gap1) and near[0] and L - near[0] <= step_max:
                # A stretch beside a gap is shortened to the next length that closes, from its
                # gap's side: the gap only widens, so the clear width only grows (rule 6).
                cut = L - near[0]
                if gap0 and gap1:
                    p0, p1 = p0 + cut / 2, p1 - cut / 2
                elif gap1:
                    p1 -= cut
                else:
                    p0 += cut
                L = p1 - p0
                got, near = fit(L, wall["pitch"], filler["pitch"] if filler else 0)
            key = f"{prefix}_w{st['edge']}_{j}"
            if got is None:
                opens.append({"key": key, "length": L, "near": near, "at": rnd(local(e, (p0 + p1) / 2, 0))})
                continue
            n, k = got
            s0 = starts[st["edge"]] + p0
            s_mid = s0 + n * wall["pitch"]
            s1 = starts[st["edge"]] + p1
            common = {"face": outward(e), "offset": round(T / 2000, 3), "ends": "half"}
            if n:
                runs.append({"key": key, "asset": wall["id"], "count": n, "ref": storey_key,
                             "span": [round(s0 / total, 9), round((s_mid if k else s1) / total, 9)], **common})
            if k:
                runs.append({"key": key + "f", "asset": filler["id"], "count": k, "ref": storey_key,
                             "span": [round(s_mid / total, 9), round(s1 / total, 9)], **common})
    corner = kit.get("corner")
    if corner is not None:
        for c_ in corners:
            nb, na = c_["before"]["n"], c_["after"]["n"]
            p = (c_["at"][0] + (nb[0] + na[0]) * T / 2, c_["at"][1] + (nb[1] + na[1]) * T / 2)
            insts.append({"key": f"{prefix}_c{ring.index(c_['at'])}", "asset": corner["id"], "at": rnd(p),
                          "face": outward(c_["after"])})
    leaf = kit.get("door")
    fes = edges(footprint)
    if leaf is not None:
        for i, d in enumerate(doors):
            fe = fes[d["edge"]]
            insts.append({"key": f"{prefix}_door{i}", "asset": leaf["id"], "at": rnd(local(fe, d["s"], T / 2)),
                          "face": outward(fe)})
    return runs, insts, opens


def inner_walls(openings, footprint, kit, prefix):
    """Instances of wall pieces along each cut between two rooms, from the outer walls' inner face,
    leaving the opening (rule 6)."""
    wall, filler = kit.get("wall"), kit.get("filler")
    insts, opens = [], []
    if wall is None:
        return insts, opens
    T = wall["thick"]
    ax = long_axis(footprint)
    for o in openings:
        a, b = o["ends"]
        L = math.dist(a, b)
        d = ((b[0] - a[0]) / L, (b[1] - a[1]) / L)
        e = {"a": a, "e": d, "n": (-d[1], d[0]), "L": L}
        half = o["width"] / 2
        for j, (p0, p1) in enumerate(((T, L / 2 - half), (L / 2 + half, L - T))):
            length = p1 - p0
            if length <= FIT:
                continue
            got, near = fit(length, wall["pitch"], filler["pitch"] if filler else 0)
            key = f"{prefix}_i{o['rooms'][0]}_{j}"
            if got is None:
                opens.append({"key": key, "length": length, "near": near, "at": rnd(local(e, (p0 + p1) / 2, 0))})
                continue
            n, k = got
            s = p0
            for m, (piece, count) in enumerate(((wall, n), (filler, k))):
                for q in range(count):
                    insts.append({"key": f"{key}_{m}{q}", "asset": piece["id"],
                                  "at": rnd(local(e, s + piece["pitch"] / 2, 0)), "face": bearing(ax)})
                    s += piece["pitch"]
    return insts, opens


# --- the plan of one building -----------------------------------------------------------------


def _num(attrs, key, default, lo=None, integer=False):
    v = attrs.get(key, default)
    try:
        v = int(v) if integer else float(v)
    except (TypeError, ValueError):
        raise ParamError(key, f"`{key}` is not a number", f"write `{key}` as a number")
    if lo is not None and v < lo:
        raise ParamError(key, f"`{key}` is {v}, below {lo}", f"write `{key}` as {lo} or more")
    return v


def plan(name, ring, attrs, ground_m, grid=None, kit=None, inner=INNER_DOOR, margin=MARGIN):
    """Everything one building returns, as plain data (the step writes it): rules 1 to 7."""
    footprint = ccw(ring)
    prefix = key_of(name)
    storeys = _num(attrs, "storeys", 1, 1, integer=True)
    storey_h = _num(attrs, "storey_h", 3.2, 0.5)
    plinth = _num(attrs, "plinth", 0.3, 0)
    porch = int(round(_num(attrs, "porch", 2, 0.5) * 1000))
    walk_out = int(round(_num(attrs, "walk_out", 3, 0) * 1000))
    lv = levels(ground_m, plinth, storey_h, storeys)
    stair = attrs.get("stair") or None
    wall_t = (kit or {}).get("wall", {}).get("thick", 0) if kit and kit.get("wall") else 0
    sp = stair_plan(footprint, stair, storeys, wall_t)
    rings = [footprint]
    for k in range(1, storeys):
        rings.append(notch_ring(footprint, sp["notches"][k]))
    out = {"storeys": [], "rooms": [], "flights": [], "doorways": [], "footpaths": [], "runs": [],
           "instances": [], "open": [], "openings": [], "roof": None}
    for k in range(storeys):
        out["storeys"].append({"key": f"{prefix}_s{k}", "region": rings[k], "level": lv[k]})
    for path, lo, hi in sp["flights"]:
        out["flights"].append({"key": f"{prefix}_f{lo}", "path": path,
                               "starts_at": f"{prefix}_s{lo}.floor", "ends_at": f"{prefix}_s{hi}.floor",
                               "z0": lv[lo], "z1": lv[hi], "width": float(stair.get("width", 1))})
    res_mm = int((grid or {}).get("resolution_mm") or 1000)
    flat = max(MIN_FLAT, int(math.ceil(res_mm * math.sqrt(2))))
    doors_by_storey = {}
    for i, d in enumerate(attrs.get("doors") or []):
        st = int(d.get("storey", 0))
        if st != 0:
            raise ParamError("doors", f"door {i} is on storey {st}; a door above the ground storey "
                             "needs a balcony, not laid yet", "put the door on storey 0")
        dp = door_plan(footprint, d, porch, walk_out)
        if sp.get("edge") == dp["edge"]:
            k = sp["edge"]
            lo = min(n[1] for n in sp["notches"].values())
            hi = max(n[2] for n in sp["notches"].values())
            if dp["s"] + dp["width"] / 2 > lo and dp["s"] - dp["width"] / 2 < hi and k == dp["edge"]:
                raise ParamError("doors", f"door {i} opens into the stairwell",
                                 "move the door's `t` or the stair's")
        doors_by_storey.setdefault(st, []).append(dp)
        key = f"{prefix}_d{i}"
        out["doorways"].append({"key": key, "path": dp["doorway"], "ends_at": f"{prefix}_s{st}.floor",
                                "sill_z": lv[st], "flat": round(flat / 1000, 3)})
        if walk_out > 0:
            out["footpaths"].append({"key": f"{prefix}_p{i}", "path": dp["footpath"],
                                     "ends_at": f"{key}.start"})
    for r in attrs.get("rooms") or []:
        st = int(r.get("storey", 0))
        if not 0 <= st < storeys:
            raise ParamError("rooms", f"rooms name storey {st}; the building has {storeys}",
                             f"write a storey from 0 to {storeys - 1}")
        rooms, openings = room_plan(rings[st], footprint, r.get("shares") or [], inner)
        for i, ring_ in enumerate(rooms):
            out["rooms"].append({"key": f"{prefix}_s{st}_r{i}", "region": ring_, "storey": f"{prefix}_s{st}"})
        for o in openings:
            o["storey"] = st
            out["openings"].append(o)
        if kit:
            insts, opens = inner_walls(openings, footprint, kit, f"{prefix}_s{st}")
            for it in insts:
                it["surface"] = f"{prefix}_s{st}.floor"
            out["instances"].extend(insts)
            out["open"].extend(opens)
    if kit:
        for k in range(storeys):
            crossers = [(d["path"], dp["width"]) for d, dp in zip(
                [d for d in out["doorways"] if d["ends_at"] == f"{prefix}_s{k}.floor"], doors_by_storey.get(k, []))]
            crossers += [(f["path"], int(round(f["width"] * 1000))) for f in out["flights"]]
            runs, insts, opens = wall_plan(rings[k], footprint, doors_by_storey.get(k, []), kit,
                                           f"{prefix}_s{k}", f"{prefix}_s{k}", crossers, margin)
            for it in insts:
                it["surface"] = f"{prefix}_s{k}.floor"
            out["runs"].extend(runs)
            out["instances"].extend(insts)
            out["open"].extend(opens)
    top = storeys - 1
    if attrs.get("roof") == "garden":
        out["roof"] = {"key": f"{prefix}_roof", "region": footprint, "eaves": round(lv[top] + storey_h, 3)}
    elif kit and kit.get("roof"):
        cx = sum(p[0] for p in footprint) / len(footprint)
        cy = sum(p[1] for p in footprint) / len(footprint)
        es = edges(footprint)
        longest = max(range(len(es)), key=lambda i: (es[i]["L"], -i))
        out["instances"].append({"key": f"{prefix}_roof", "asset": kit["roof"]["id"], "at": rnd((cx, cy)),
                                 "face": outward(es[longest]), "surface": f"{prefix}_s{top}.floor"})
    return out


def key_of(name):
    """A key from the building's name: characters an id may not hold become `_` (identity.md 2.1)."""
    return "".join(c if c.isalnum() or c in "_-" else "_" for c in str(name))


# --- the step ---------------------------------------------------------------------------------


ATTRS = ["storeys", "storey_h", "plinth", "porch", "walk_out", "doors", "stair", "rooms", "kit", "roof"]


def _list(col):
    return col.tolist() if hasattr(col, "tolist") else list(col)


def _json(v):
    if isinstance(v, str):
        try:
            return json.loads(v) if v else None
        except ValueError:
            return v
    return v


def read_building(step, name):
    fl = step.features(select=[name], columns=ATTRS)
    if len(fl) != 1 or int(_list(fl["geometry"])[0]) != 2:
        raise ParamError("region", f"`{name}` is not one region feature", "write the building as a `region`")
    pieces = fl.rings(0)
    if len(pieces) != 1 or len(pieces[0]) != 1:
        raise ParamError("region", f"`{name}` is not one ring", "write the building as one ring with no hole")
    ring = [(int(v[0]), int(v[1])) for v in _list(pieces[0][0])]
    attrs = {}
    for a in ATTRS:
        if a in fl and (f"{a}?" not in fl or _list(fl[f"{a}?"])[0]):
            attrs[a] = _json(_list(fl[a])[0])
    return ring, attrs


def read_ground(step, ring):
    """The mean terrain height over the outline's posts and vertices (rule 2), metres."""
    try:
        (surface,) = step.fields(["role:surface"], window=[0, 0, 1, 1])
    except (LookupError, RpcError):
        return 0.0
    pts = posts_in(ring, step.grid)
    h = [float(v) for v in _list(step.eval(f"= f('{surface.name}')", x=[p[0] for p in pts],
                                           y=[p[1] for p in pts]))]
    return sum(h) / len(h)


def read_kit(step, taxon):
    if not taxon:
        return None
    t = step.pack(select="@" + taxon, columns=["bounds", "params"])
    ids = _list(t.get("id", []))
    rows = []
    for r, i in enumerate(ids):
        tags = _json(_list(t["tags"])[r]) if "tags" in t else []
        b = _list(t["bounds"])[r] if "bounds" in t else None
        rows.append({"id": i, "tags": tags if isinstance(tags, list) else [tags],
                     "bounds": [int(v) for v in _list(b)] if b is not None else None,
                     "params": _json(_list(t["params"])[r]) or {} if "params" in t else {}})
    return kit_pieces(rows)


def write(step, out):
    o = step.out
    for s in out["storeys"]:
        o.feature(s["key"], "storey", region=s["region"], level=s["level"])
    for r in out["rooms"]:
        o.feature(r["key"], "room", region=r["region"], storey=r["storey"])
    for f in out["flights"]:
        o.feature(f["key"], "flight", path=f["path"], starts_at=f["starts_at"], ends_at=f["ends_at"],
                  z0=f["z0"], z1=f["z1"], width=f["width"])
    for d in out["doorways"]:
        o.feature(d["key"], "doorway", path=d["path"], ends_at=d["ends_at"], sill_z=d["sill_z"], flat=d["flat"])
    for p in out["footpaths"]:
        o.feature(p["key"], "footpath", path=p["path"], ends_at=p["ends_at"])
    if out["roof"]:
        r = out["roof"]
        o.feature(r["key"], "roof", region=r["region"], eaves=r["eaves"])
    for r in out["runs"]:
        o.table("run", r["key"], asset=r["asset"], count=r["count"], ends=r["ends"], face=r["face"],
                on={"ref": r["ref"], "span": r["span"], "offset": r["offset"]})
    for it in out["instances"]:
        o.instance(it["key"], it["asset"], at=it["at"], face=it["face"], surface=it["surface"])


def generate(step):
    name = step.for_each_item
    try:
        inner = int(round(float((step.params or {}).get("inner_door", INNER_DOOR / 1000)) * 1000))
        margin = int(round(float((step.params or {}).get("margin", MARGIN / 1000)) * 1000))
        ring, attrs = read_building(step, name)
        kit = read_kit(step, attrs.get("kit"))
        out = plan(name, ring, attrs, read_ground(step, ring), step.grid, kit, inner, margin)
    except ParamError as e:
        step.error("params", name, f"{name}: {e.message}", e.hint, explain=EXPLAIN["params"],
                   data={"attribute": e.key})
        return
    write(step, out)
    for i, w in enumerate(out["open"]):
        below, above = w["near"]
        step.warning("wall_open", name,
                     f"a wall stretch of {w['length'] / 1000:g} m does not close with the kit's pieces",
                     "move a door or a corner, or add a filler of another pitch",
                     position=(w["at"][0] / 1000, w["at"][1] / 1000),
                     data={"length": round(w["length"] / 1000, 3),
                           "closes": [round(v / 1000, 3) for v in (below, above) if v is not None]},
                     **({"explain": EXPLAIN["wall_open"]} if i == 0 else {}))
    step.info("laid", name,
              f"{len(out['storeys'])} storeys, {len(out['rooms'])} rooms, {len(out['flights'])} flights, "
              f"{len(out['doorways'])} doorways, {len(out['runs'])} wall runs, {len(out['instances'])} pieces",
              "none needed", explain=EXPLAIN["laid"],
              data={k: len(out[k]) for k in ("storeys", "rooms", "flights", "doorways", "footpaths", "runs",
                                             "instances")})
