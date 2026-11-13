"""The review library's lint: the measures a ford is judged by beside the eye (R15, R17).

Run as a plain command through the SDK, declared in the world (protocol.md 1):

    [step.checks]
    when = "lint"
    command = ["python", "-m", "viasdk", "lib:review/checks.py"]

Each measure is one ``info`` record per subject, code ``step.checks.<name>``, its number in
``data.value`` (docs/plan.md, "Milestone one: ford"). A measure informs; it is never a target
(R15). The first record of each code in a call carries ``explain`` (protocol.md 3.3).

| name              | subject                  | data.value                                              |
| lane_gap          | each lane                | metres from its nearer end to the nearest ford          |
| approach_grade    | each ford                | steepest rise over run along a lane within 20 m         |
| ford_depth        | each ford                | water level less ground at the ford, metres             |
| water_step        | each beck                | largest jump in the water surface between neighbouring  |
|                   |                          | posts where one of them is wet, metres                  |
| cottage_relief    | each grounded instance   | highest less lowest ground over its footprint feature   |
| trees_in_clearing | each producer of trees   | trees standing where keep_out holds (is 1), a count     |

Fields are found by role and tag, never by name (fields.md 2.3): the ground is ``role:surface``,
the water ``tag:water_level`` (lib/water), "not here" ``role:keep_out``. Kinds are the libraries':
``lane`` and ``ford`` (lib/ways), ``beck`` (lib/water).

Library code imports only viasdk and the standard library (R19b), so arrays are read through
their own methods (``tolist``) and the arithmetic here is plain Python doubles.
"""

import math

from viasdk import RpcError

# The approach is measured along each lane over the ford kind's radius (lib/ways/kinds.toml).
APPROACH_M = 20.0
# Wet-post pairs are given to the nearest beck within its kind's radius (lib/water/kinds.toml).
BECK_RADIUS_M = 16.0
# A lane meets a ford when one of its ends lies this close to it (geometry.md 2.7: ends meet exactly).
MEETS_MM = 1

EXPLAIN = {
    "lane_gap": "Distance in metres from a lane's nearer end to the nearest ford. A lane that "
    "ends at a ford (`ends_at`, `starts_at`) has 0; more is a blunt end short of the water (R18).",
    "approach_grade": "The steepest rise over run of the ground, sampled every post spacing "
    "along each lane that meets the ford, within 20 m of it. What a walker climbs out of the water.",
    "ford_depth": "Water level less ground at the ford's point, in metres: how deep the crossing is "
    "on foot. Read from the fields tagged water_level and with role surface.",
    "water_step": "The largest difference in the water surface between two neighbouring posts, "
    "at least one of them wet, near the beck: a step there is a wall of water along the bank.",
    "cottage_relief": "Highest less lowest ground at the posts inside an instance's footprint "
    "feature (its footprint grown by margin): 0 means the pad left it level.",
    "trees_in_clearing": "How many trees stand where keep_out is 1. The scatter applies keep_out "
    "itself, so anything above 0 is a fault in the scatter or a hand edit.",
}

HINTS = {
    "lane_gap": "End the lane at the ford by name: `ends_at = \"<ford>\"` or `starts_at`.",
    "approach_grade": "Lower the ford kind's `grade`, or move the ford where the banks are lower.",
    "ford_depth": "Move the ford along the beck (`t`), or change the beck's `depth`.",
    "water_step": "Look at the effects writing tag:water_level near this position.",
    "cottage_relief": "Ground the instance with a pad kind (`ground = \"pad\"`), or widen `margin`.",
    "trees_in_clearing": "Look for a pin inside a clearing, or a kind writing keep_out after placement.",
}


def lint(step):
    grid = Grid(step.grid)
    height = _field(step, "role:surface")
    water = _field(step, "tag:water_level")
    keep_out = _field(step, "role:keep_out")
    lanes = _features(step, "kind:lane")
    fords = _features(step, "kind:ford")
    becks = _features(step, "kind:beck")
    grounds = [f for f in _features(step, "*") if f["name"].endswith(".ground") and f["geometry"] == 2]
    pivots = {}
    trees = []
    for row in _instances(step, "*"):
        pivots[row["name"]] = (row["x"], row["y"])
    for row in _instances(step, "@plant/tree"):
        trees.append(row)
    say = Sayer(step)
    for rec in measure(grid, height, water, keep_out, lanes, fords, becks, grounds, pivots, trees):
        say(*rec)


def measure(grid, height, water, keep_out, lanes, fords, becks, grounds, pivots, trees):
    """Every record, as (name, subject, value, position or None, message), in a fixed order."""
    out = []
    ford_points = [(f["name"], f["rings"][0][0][0]) for f in fords]
    for lane in lanes:
        line = lane["rings"][0][0]
        if not ford_points:
            break
        best = None
        for end in (line[0], line[-1]):
            for _, p in ford_points:
                dmm = _dist(end, p)
                if best is None or dmm < best[0]:
                    best = (dmm, end)
        value = round(best[0] / 1000.0, 3)
        out.append(("lane_gap", "feature." + lane["name"], value, _m(best[1]),
                    f"Lane {lane['name']} ends {value} m from the nearest ford."))
    for name, p in ford_points:
        if height is not None:
            steep = None
            for lane in lanes:
                line = lane["rings"][0][0]
                if _dist(line[0], p) <= MEETS_MM:
                    walk = line
                elif _dist(line[-1], p) <= MEETS_MM:
                    walk = list(reversed(line))
                else:
                    continue
                g = _steepest(grid, height, walk)
                if g is not None and (steep is None or g[0] > steep[0]):
                    steep = g
            if steep is not None:
                value = round(steep[0], 4)
                out.append(("approach_grade", "feature." + name, value, _m(steep[1]),
                            f"The steepest approach to {name} rises {value} per metre."))
        if height is not None and water is not None and grid.inside(p):
            depth = round(grid.sample(water, p) - grid.sample(height, p), 3)
            out.append(("ford_depth", "feature." + name, depth, _m(p),
                        f"The water at {name} is {depth} m deep."))
    if height is not None and water is not None and becks:
        for name, (jump, pos) in _water_steps(grid, height, water, becks):
            value = round(jump, 3)
            out.append(("water_step", "feature." + name, value, pos,
                        f"The water surface along {name} jumps at most {value} m between posts."))
    if height is not None:
        for g in sorted(grounds, key=lambda f: f["name"]):
            inst = g["name"][: -len(".ground")]
            vals = _values_inside(grid, height, g["rings"])
            if not vals:
                continue
            value = round(max(vals) - min(vals), 3)
            pos = _m(pivots[inst]) if inst in pivots else None
            out.append(("cottage_relief", "instance." + inst, value, pos,
                        f"The ground under {inst} varies by {value} m."))
    if keep_out is not None and trees:
        counts = {}
        for t in trees:
            by = t["by"]
            p = (t["x"], t["y"])
            held = 1 if grid.inside(p) and grid.sample(keep_out, p) >= 1.0 else 0
            counts[by] = counts.get(by, 0) + held
        for by in sorted(counts):
            subject = by if "." in by else "scatter." + by
            out.append(("trees_in_clearing", subject, counts[by], None,
                        f"{counts[by]} trees of {by} stand where keep_out holds."))
    return out


class Sayer:
    """Sends each record as ``step.info``, the first of each code with its ``explain``."""

    def __init__(self, step):
        self.step = step
        self.explained = set()

    def __call__(self, name, subject, value, position, message):
        kw = {"code": name, "subject": subject, "message": message, "hint": HINTS[name],
              "data": {"value": value}}
        if position is not None:
            kw["position"] = [position[0], position[1]]
        if name not in self.explained:
            kw["explain"] = EXPLAIN[name]
            self.explained.add(name)
        self.step.info(**kw)


class Grid:
    """The world's posts (protocol.md 2: ``grid``), read as fields.md 1.2 reads a field."""

    def __init__(self, g):
        self.x0, self.y0, self.x1, self.y1 = (int(v) for v in g["extent_mm"])
        self.r = int(g["resolution_mm"])
        self.nx = int(g["nx"])
        self.ny = int(g["ny"])

    def inside(self, p):
        return self.x0 <= p[0] <= self.x1 and self.y0 <= p[1] <= self.y1

    def post(self, i, j):
        return (self.x0 + i * self.r, self.y0 + j * self.r)

    def sample(self, a, p):
        """Bilinear between posts, exactly as fields.md 1.2 orders it. ``a`` is rows from the south."""
        ax = int(p[0]) - self.x0
        ay = int(p[1]) - self.y0
        i = min(ax // self.r, self.nx - 2)
        j = min(ay // self.r, self.ny - 2)
        fx = (ax - i * self.r) / self.r
        fy = (ay - j * self.r) / self.r
        p00, p10 = a[j][i], a[j][i + 1]
        p01, p11 = a[j + 1][i], a[j + 1][i + 1]
        s = (1 - fx) * p00 + fx * p10
        n = (1 - fx) * p01 + fx * p11
        return (1 - fy) * s + fy * n


def _steepest(grid, height, walk):
    """Rise over run between samples one post spacing apart, from ``walk[0]`` out to APPROACH_M.
    Returns (grade, midpoint in mm) or None."""
    step_mm = grid.r
    pts = _along(walk, step_mm, int(APPROACH_M * 1000))
    best = None
    prev = None
    for p in pts:
        if not grid.inside(p):
            prev = None
            continue
        h = grid.sample(height, p)
        if prev is not None:
            run = _dist(prev[0], p) / 1000.0
            if run > 0:
                g = abs(h - prev[1]) / run
                if best is None or g > best[0]:
                    best = (g, ((prev[0][0] + p[0]) // 2, (prev[0][1] + p[1]) // 2))
        prev = (p, h)
    return best


def _along(line, step_mm, limit_mm):
    """Points every ``step_mm`` along a polyline from its first vertex, up to ``limit_mm``,
    rounded to the millimetre."""
    out = [tuple(line[0])]
    s = 0
    next_at = step_mm
    for a, b in zip(line, line[1:]):
        seg = _dist(a, b)
        while next_at <= s + seg and next_at <= limit_mm:
            f = (next_at - s) / seg
            out.append((round(a[0] + f * (b[0] - a[0])), round(a[1] + f * (b[1] - a[1]))))
            next_at += step_mm
        s += seg
        if next_at > limit_mm:
            break
    return out


def _water_steps(grid, height, water, becks):
    """Per beck: the largest jump in water between east or north neighbours, one of them wet,
    over pairs whose midpoint lies within BECK_RADIUS_M of that beck and nearest to it."""
    lines = [(b["name"], b["rings"][0][0]) for b in becks]
    cap = BECK_RADIUS_M * 1000.0
    best = {}
    for j in range(grid.ny):
        hrow, wrow = height[j], water[j]
        hup = height[j + 1] if j + 1 < grid.ny else None
        wup = water[j + 1] if j + 1 < grid.ny else None
        for i in range(grid.nx):
            wet = wrow[i] > hrow[i]
            pairs = []
            if i + 1 < grid.nx and (wet or wrow[i + 1] > hrow[i + 1]):
                pairs.append((i + 1, j, wrow[i + 1]))
            if wup is not None and (wet or wup[i] > hup[i]):
                pairs.append((i, j + 1, wup[i]))
            for (i2, j2, w2) in pairs:
                jump = abs(wrow[i] - w2)
                a = grid.post(i, j)
                b = grid.post(i2, j2)
                mid = ((a[0] + b[0]) // 2, (a[1] + b[1]) // 2)
                near = None
                for name, line in lines:
                    d = _dist_to_line(mid, line)
                    if d <= cap and (near is None or d < near[0]):
                        near = (d, name)
                if near is None:
                    continue
                cur = best.get(near[1])
                if cur is None or jump > cur[0]:
                    best[near[1]] = (jump, _m(mid))
    return sorted(best.items())


def _values_inside(grid, a, pieces):
    """Field values at the posts inside a region (outline included), in post order."""
    vals = []
    xs = [p[0] for piece in pieces for ring in piece for p in ring]
    ys = [p[1] for piece in pieces for ring in piece for p in ring]
    i0 = max(0, -(-(min(xs) - grid.x0) // grid.r))
    i1 = min(grid.nx - 1, (max(xs) - grid.x0) // grid.r)
    j0 = max(0, -(-(min(ys) - grid.y0) // grid.r))
    j1 = min(grid.ny - 1, (max(ys) - grid.y0) // grid.r)
    for j in range(j0, j1 + 1):
        for i in range(i0, i1 + 1):
            p = grid.post(i, j)
            if any(_in_piece(p, piece) for piece in pieces):
                vals.append(a[j][i])
    return vals


def _in_piece(p, rings):
    """Inside the outline and outside every hole; on a ring counts as inside. Exact in integers."""
    if not _in_ring(p, rings[0]):
        return False
    return not any(_in_ring(p, h) and not _on_ring(p, h) for h in rings[1:])


def _on_ring(p, ring):
    n = len(ring)
    for k in range(n):
        a, b = ring[k], ring[(k + 1) % n]
        cross = (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0])
        if cross == 0 and min(a[0], b[0]) <= p[0] <= max(a[0], b[0]) and min(a[1], b[1]) <= p[1] <= max(a[1], b[1]):
            return True
    return False


def _in_ring(p, ring):
    if _on_ring(p, ring):
        return True
    wn = 0
    n = len(ring)
    for k in range(n):
        a, b = ring[k], ring[(k + 1) % n]
        cross = (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0])
        if a[1] <= p[1] < b[1] and cross > 0:
            wn += 1
        elif b[1] <= p[1] < a[1] and cross < 0:
            wn -= 1
    return wn != 0


def _dist(a, b):
    return math.hypot(b[0] - a[0], b[1] - a[1])


def _dist_to_line(p, line):
    best = None
    for a, b in zip(line, line[1:]):
        ex, ey = b[0] - a[0], b[1] - a[1]
        wx, wy = p[0] - a[0], p[1] - a[1]
        ee = ex * ex + ey * ey
        t = 0.0 if ee == 0 else max(0.0, min(1.0, (wx * ex + wy * ey) / ee))
        d = math.hypot(wx - t * ex, wy - t * ey)
        if best is None or d < best:
            best = d
    return best if best is not None else math.inf


def _m(p):
    return (p[0] / 1000.0, p[1] / 1000.0)


# Reading through the SDK. Each read is tolerant of what this world lacks: a world with no water
# field has no ford depth, and says nothing about it.


def _rows(a):
    """A field array as rows of Python floats (rows from the south)."""
    return a.tolist() if hasattr(a, "tolist") else a


def _field(step, selector):
    try:
        got = step.fields([selector])
    except (LookupError, RpcError):  # no field matches: the measures that need it are not taken
        return None
    if not got:
        return None
    return _rows(got[0])


def _list(col):
    return col.tolist() if hasattr(col, "tolist") else list(col)


def _features(step, select):
    """A feature list (protocol.md 4.3) as one dict per feature: name, kind, geometry, and
    ``rings`` as pieces of rings of (x, y) millimetre pairs."""
    try:
        fl = step.features(select)
    except (LookupError, RpcError):  # no feature matches
        return []
    names = _list(fl["name"])
    kinds = _list(fl["kind"])
    geometry = _list(fl["geometry"])
    pieces = _list(fl["pieces"])
    rings = _list(fl["rings"])
    starts = _list(fl["ring_starts"])
    coords = [tuple(int(v) for v in c) for c in _list(fl["coords"])]
    out = []
    for f, name in enumerate(names):
        ps = []
        for p in range(pieces[f], pieces[f + 1]):
            ps.append([coords[starts[r]:starts[r + 1]] for r in range(rings[p], rings[p + 1])])
        out.append({"name": name, "kind": kinds[f], "geometry": int(geometry[f]), "rings": ps})
    return out


def _instances(step, select):
    """An instance table (protocol.md 3.3) as one dict per row: name, asset, by, x, y (mm)."""
    try:
        t = step.instances(select)
    except (LookupError, RpcError):  # nothing selected
        return []
    cols = {k: _list(t[k]) for k in ("name", "asset", "by", "x", "y")}
    return [{k: cols[k][r] for k in cols} for r in range(len(cols["name"]))]
