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
|                   |                          | posts that are both wet, within its ``width`` of its     |
|                   |                          | line, metres                                            |
| cottage_relief    | each grounded instance   | highest less lowest ground over its footprint feature   |
| trees_in_clearing | each scatter of trees    | trees standing where keep_out is 0.9 or more, a count;  |
|                   |                          | a written pin or instance is not counted                |
| water_area        | each lake and pond       | square metres of posts inside it whose water stands     |
|                   |                          | above the ground                                        |
| water_depth       | each lake and pond       | deepest water over the ground inside it, metres         |
| water_islands     | each lake and pond       | holes in its region (islands), a count                  |
| water_shore       | each lake and pond       | length of its outer ring, metres                        |
| trees_on_way      | each road, lane and trail| trees standing within half its ``width`` of its line    |
| crowded_view      | each bookmark            | metres from the eye to the nearest tree or rock, when   |
|                   |                          | one stands within CROWDED_M                             |

A subject whose measure cannot be taken, because a field it reads selects no field or several, gets
one ``warning`` record instead, code ``step.checks.unmeasured``, the measure's name in
``data.measure``; so every subject has one record. Any other failure, an engine or protocol fault
included, is not caught: it fails the step.

Fields are found by role and tag, never by name (fields.md 2.3): the ground is ``role:surface``,
the water ``tag:water_level`` (lib/water), "not here" ``role:keep_out``. Kinds are the libraries':
``lane`` and ``ford`` (lib/ways), ``beck`` (lib/water).

Library code imports only viasdk and the standard library (R19b), so arrays are read through
their own methods (``tolist``) and the arithmetic here is plain Python doubles.
"""

import math

# The approach is measured along each lane out to where the ford's `approach` mask reaches 0
# (`1 - smoothstep(16, 20, d)`, lib/ways/kinds.toml). A coupling, kept by hand: a kind's radius and
# masks are not in the feature list a step reads (protocol.md 4.3), only its attributes. If the
# ford kind's mask changes, change this with it.
APPROACH_M = 20.0
# A lane meets a ford when one of its ends lies this close to it (geometry.md 2.7: ends meet exactly).
MEETS_MM = 1
# Where `clearing` holds, for trees_in_clearing. The scatter keeps a candidate with chance
# `density * (1 - keep_out)` (rules.md 7.1), so a tree can stand wherever keep_out is below 1: a
# lucky draw survives a chance cut to a few percent. Counting only `keep_out = 1` misses it (a tree
# at 0.988 whose crown hung over a lane counted 0). At 0.9 or more the chance is a tenth or less:
# the core of a clearing, where a kind means "none here". Below it is the fade, where a kind lets
# trees thin out by design (lib/ways' verge fades from 5 m, past a crown's reach), so a tree there
# is not counted.
KEPT_OUT = 0.9
# A view is crowded when a tree or a rock stands this near its eye (asked by `vale`): at eye height
# a trunk or a boulder within 3 m fills the frame.
CROWDED_M = 3.0
# The kinds of standing water and of ways (lib/water, lib/ways), measured by name of kind.
WATERS = ("lake", "pond")
WAYS = ("road", "lane", "trail")

EXPLAIN = {
    "lane_gap": "Distance in metres from a lane's nearer end to the nearest ford. A lane that "
    "ends at a ford (`ends_at`, `starts_at`) has 0; more is a blunt end short of the water (R18).",
    "approach_grade": "The steepest rise over run of the ground, sampled every post spacing "
    "along each lane that meets the ford, within 20 m of it. What a walker climbs out of the water.",
    "ford_depth": "Water level less ground at the ford's point, in metres: how deep the crossing is "
    "on foot. Read from the fields tagged water_level and with role surface.",
    "water_step": "The largest difference in the water surface between two neighbouring posts, "
    "both wet, within the beck's `width` of its line: the surface should be level across the channel and fall gently "
    "along it, so a step is water climbing the bank or a wall of water. A wet post beside a dry "
    "one is the water's edge, where the field holds the ground, and is not compared.",
    "cottage_relief": "Highest less lowest ground at the posts inside an instance's footprint "
    "feature (its footprint grown by margin): 0 means the pad left it level.",
    "trees_in_clearing": "How many trees of a scatter stand where keep_out is 0.9 or more. The "
    "scatter keeps a candidate with chance density * (1 - keep_out), so a tree can survive anywhere "
    "keep_out is below 1; at 0.9 or more its chance was a tenth or less, in the core of a clearing "
    "that a kind writes to keep trees off a lane, a cottage or the water. Above 0 means that effect "
    "is not 1 far enough out: a crown can reach what it keeps clear. A written pin or instance is "
    "not counted here: the build reports it as place.keep_out.",
    "water_area": "Square metres of a lake or pond: the posts inside its region whose water "
    "(tag water_level) stands above the ground (role surface), each a post spacing squared.",
    "water_depth": "The deepest water over the ground at a post inside a lake or pond, metres.",
    "water_islands": "How many holes the lake's region has: islands the ground holds above its "
    "level. A view stands on the outer ring (geometry.md 2.6).",
    "water_shore": "The length of a lake's or pond's outer ring, metres: its shore as a walker "
    "follows it from outside.",
    "trees_on_way": "How many scattered trees stand on a way's bed: within half its `width` of its "
    "line. A way's verge keeps trees off with keep_out; above 0 means a draw survived the fade or "
    "the verge does not reach.",
    "crowded_view": "Metres from a bookmark's eye to the nearest tree or rock, reported when one "
    "stands within 3 m: at eye height it fills the view, so the build cannot be judged from there.",
    "unmeasured": "A measure that could not be taken for this subject, because a field it reads "
    "(role surface, tag water_level or role keep_out) selects no field, or several. "
    "`data.measure` names the measure. A fault in the engine or the protocol is not this: it fails "
    "the step (protocol.failed).",
}

HINTS = {
    "lane_gap": "End the lane at the ford by name: `ends_at = \"<ford>\"` or `starts_at`.",
    "approach_grade": "Within the beck's reach its banks set the grade: set a lower `bank_grade` on "
    "the beck; beyond it, a lower `grade` on the ford (keep `bank_grade` at or below `grade`). Or "
    "move the ford along the beck (`t`) where the banks are lower.",
    "ford_depth": "Move the ford along the beck (`t`), or change the beck's `depth`.",
    "water_step": "Lower the beck's `bank_grade`, or change its `depth` or `width`.",
    "cottage_relief": "A measure: 0 means level. Above that, ground the instance with a pad kind (`ground = \"pad\"`), or widen `margin`.",
    "trees_in_clearing": "Hold the mask of the kind effect writing keep_out here at 1 farther from "
    "its feature (a lane's `verge`, a pad's `clear`, a beck's `bank`), out past a crown's reach, "
    "so it falls below 0.9 only where no crown reaches the feature.",
}

SURFACE = "role:surface"
WATER = "tag:water_level"
KEEP_OUT = "role:keep_out"

# The hint of an `unmeasured` warning, by the measure not taken.
HINTS.update({
    "water_area": "Informs only (R15): compare it with what the world means the water to be.",
    "water_depth": "Change the lake's `water_height`, or the ground under it.",
    "water_islands": "Raise the lake's `water_height` to drown a low islet, or keep it as land.",
    "water_shore": "Informs only (R15).",
    "trees_on_way": "Widen the way's verge (its `keep_out` effect), or thin the scatter by "
    "`dist('<way>', cap)`.",
    "crowded_view": "Move the bookmark a few metres, or keep its eye clear (`clear` on the "
    "bookmark kind, lib/review).",
})

UNMEASURED_HINTS = {
    "approach_grade": "Give the world one field with `role = \"surface\"`.",
    "ford_depth": "Give the world one field with `role = \"surface\"` and one tagged "
    "`water_level` (lib/water's beck writes it).",
    "water_step": "Give the world one field with `role = \"surface\"` and one tagged "
    "`water_level` (lib/water's beck writes it).",
    "cottage_relief": "Give the world one field with `role = \"surface\"`.",
    "trees_in_clearing": "Give the world one field with `role = \"keep_out\"`.",
    "water_area": "Give the world one field with `role = \"surface\"` and one tagged "
    "`water_level` (lib/water).",
    "water_depth": "Give the world one field with `role = \"surface\"` and one tagged "
    "`water_level` (lib/water).",
}


def lint(step):
    grid = Grid(step.grid)
    missing = {}
    height = _field(step, "role:surface", missing)
    water = _field(step, "tag:water_level", missing)
    keep_out = _field(step, "role:keep_out", missing)
    every = _features(step)
    lanes = [f for f in every if f["kind"] == "lane"]
    fords = [f for f in every if f["kind"] == "ford"]
    becks = [f for f in every if f["kind"] == "beck"]
    grounds = [f for f in every if f["name"].endswith(".ground") and f["geometry"] == 2]
    pivots = {}
    trees = []
    for row in _instances(step, "*"):
        pivots[row["name"]] = (row["x"], row["y"])
    for row in _instances(step, "@plant/tree"):
        trees.append(row)
    rocks = _instances(step, "@rock")
    say = Sayer(step)
    for rec in measure(grid, height, water, keep_out, lanes, fords, becks, grounds, pivots, trees,
                       missing):
        say(*rec)
    for rec in measure_more(grid, height, water, every, trees, rocks, missing):
        say(*rec)


def measure_more(grid, height, water, every, trees, rocks, missing=None):
    """The valley's measures (asked by `relief` and `vale`): standing water, trees on ways and
    crowded views, as ``measure``'s records, in a fixed order."""
    out = []
    missing = missing or {}
    waters = sorted((f for f in every if f["kind"] in WATERS and f["geometry"] == 2),
                    key=lambda f: f["name"])
    for f in waters:
        subject = "feature." + f["name"]
        outer = f["rings"][0][0] if f["rings"] else []
        pos = _m(outer[0]) if outer else None
        if height is None or water is None:
            sel = SURFACE if height is None else WATER
            for name in ("water_area", "water_depth"):
                out.append(("unmeasured", subject, name, pos,
                            f"{name} of {subject} is not taken: {missing.get(sel, sel)}."))
        else:
            depths = []
            for (h, w) in zip(_values_inside(grid, height, f["rings"]),
                              _values_inside(grid, water, f["rings"])):
                if w > h:
                    depths.append(w - h)
            area = round(len(depths) * (grid.r / 1000.0) ** 2, 1)
            deep = round(max(depths), 3) if depths else 0.0
            out.append(("water_area", subject, area, pos, f"{f['name']} covers {area} m2 of water."))
            out.append(("water_depth", subject, deep, pos, f"{f['name']} is {deep} m deep at most."))
        holes = sum(len(piece) - 1 for piece in f["rings"])
        out.append(("water_islands", subject, holes, pos, f"{f['name']} has {holes} islands."))
        shore = round(sum(_dist(a, b) for a, b in zip(outer, outer[1:] + outer[:1])) / 1000.0, 1)
        out.append(("water_shore", subject, shore, pos, f"{f['name']}'s shore is {shore} m long."))
    scattered = [t for t in trees if t["by"] and "." not in t["by"]]
    ways = sorted((f for f in every if f["kind"] in WAYS and f["geometry"] == 1
                   and "width" in f["attrs"]), key=lambda f: f["name"])
    for w in ways:
        line = w["rings"][0][0]
        half = w["attrs"]["width"] * 500.0
        n = sum(1 for t in scattered if _dist_to_line((t["x"], t["y"]), line) <= half)
        out.append(("trees_on_way", "feature." + w["name"], n, _m(line[0]),
                    f"{n} scattered trees stand on {w['name']}'s bed."))
    solid = [(t["x"], t["y"]) for t in trees] + [(r["x"], r["y"]) for r in rocks]
    marks = sorted((f for f in every if f["kind"] == "bookmark" and f["geometry"] == 0),
                   key=lambda f: f["name"])
    for b in marks:
        eye = b["rings"][0][0][0]
        near = min((_dist(eye, p) for p in solid), default=math.inf) / 1000.0
        if near <= CROWDED_M:
            value = round(near, 2)
            out.append(("crowded_view", "feature." + b["name"], value, _m(eye),
                        f"A tree or rock stands {value} m from {b['name']}'s eye."))
    return out


def measure(grid, height, water, keep_out, lanes, fords, becks, grounds, pivots, trees,
            missing=None):
    """Every record, as (name, subject, value, position or None, message), in a fixed order.

    ``missing`` maps the selector of each field that could not be read to why. A measure whose
    field is missing is not taken; its subject gets one ``unmeasured`` warning instead, whose
    value is the measure's name, so every subject still has one record (docs/plan.md)."""
    out = []
    missing = missing or {}

    def unmeasured(name, subject, pos, needs):
        sel = next(s for s in needs if s in missing)
        out.append(("unmeasured", subject, name, pos,
                    f"{name} of {subject} is not taken: {missing[sel]}."))
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
        else:
            unmeasured("approach_grade", "feature." + name, _m(p), [SURFACE])
        if height is not None and water is not None:
            if grid.inside(p):
                depth = round(grid.sample(water, p) - grid.sample(height, p), 3)
                out.append(("ford_depth", "feature." + name, depth, _m(p),
                            f"The water at {name} is {depth} m deep."))
        else:
            unmeasured("ford_depth", "feature." + name, _m(p), [SURFACE, WATER])
    if height is not None and water is not None:
        if becks:
            for name, (jump, pos) in _water_steps(grid, height, water, becks):
                value = round(jump, 3)
                out.append(("water_step", "feature." + name, value, pos,
                            f"The water surface along {name} jumps at most {value} m between posts."))
    else:
        for b in sorted(becks, key=lambda f: f["name"]):
            if "width" in b["attrs"]:
                unmeasured("water_step", "feature." + b["name"], _m(b["rings"][0][0][0]),
                           [SURFACE, WATER])
    for g in sorted(grounds, key=lambda f: f["name"]):
        inst = g["name"][: -len(".ground")]
        pos = _m(pivots[inst]) if inst in pivots else None
        if height is None:
            unmeasured("cottage_relief", "instance." + inst, pos, [SURFACE])
            continue
        vals = _values_inside(grid, height, g["rings"])
        if not vals:
            continue
        value = round(max(vals) - min(vals), 3)
        out.append(("cottage_relief", "instance." + inst, value, pos,
                    f"The ground under {inst} varies by {value} m."))
    # Only scattered trees: a written instance's `by` is its own statement, `pin.<n>` or
    # `instance.<n>` (protocol.md 2.2), and rules.md 8.3 reports it as place.keep_out already.
    scattered = [t for t in trees if t["by"] and "." not in t["by"]]
    if keep_out is None:
        for by in sorted({t["by"] for t in scattered}):
            unmeasured("trees_in_clearing", "scatter." + by, None, [KEEP_OUT])
    elif scattered:
        counts = {}
        for t in scattered:
            by = t["by"]
            p = (t["x"], t["y"])
            held = 1 if grid.inside(p) and grid.sample(keep_out, p) >= KEPT_OUT else 0
            counts[by] = counts.get(by, 0) + held
        for by in sorted(counts):
            out.append(("trees_in_clearing", "scatter." + by, counts[by], None,
                        f"{counts[by]} trees of {by} stand where keep_out is {KEPT_OUT} or more."))
    return out


class Sayer:
    """Sends each measure as ``step.info`` and each ``unmeasured`` as ``step.warning``, the first
    of each code with its ``explain``."""

    def __init__(self, step):
        self.step = step
        self.explained = set()

    def __call__(self, name, subject, value, position, message):
        if name == "unmeasured":
            kw = {"code": name, "subject": subject, "message": message,
                  "hint": UNMEASURED_HINTS[value], "data": {"measure": value}}
        else:
            kw = {"code": name, "subject": subject, "message": message, "hint": HINTS[name],
                  "data": {"value": value}}
        if position is not None:
            kw["position"] = [position[0], position[1]]
        if name not in self.explained:
            kw["explain"] = EXPLAIN[name]
            self.explained.add(name)
        if name == "unmeasured":
            self.step.warning(**kw)
        else:
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
    """Per beck: the largest jump in water between east or north neighbours, both wet,
    over pairs whose midpoint lies within the beck's ``width`` of its line and nearest to it.

    ``width`` is the beck's documented attribute (lib/water: metres across the water), so the
    reach follows the beck the world wrote: water that climbs the bank as far again as the
    channel's half-width is still compared. A beck without ``width`` is not measured."""
    lines = [(b["name"], b["rings"][0][0], b["attrs"]["width"] * 1000.0)
             for b in becks if "width" in b["attrs"]]
    best = {}
    for j in range(grid.ny):
        hrow, wrow = height[j], water[j]
        hup = height[j + 1] if j + 1 < grid.ny else None
        wup = water[j + 1] if j + 1 < grid.ny else None
        for i in range(grid.nx):
            wet = wrow[i] > hrow[i]
            pairs = []
            if not wet:
                continue
            if i + 1 < grid.nx and wrow[i + 1] > hrow[i + 1]:
                pairs.append((i + 1, j, wrow[i + 1]))
            if wup is not None and wup[i] > hup[i]:
                pairs.append((i, j + 1, wup[i]))
            for (i2, j2, w2) in pairs:
                jump = abs(wrow[i] - w2)
                a = grid.post(i, j)
                b = grid.post(i2, j2)
                mid = ((a[0] + b[0]) // 2, (a[1] + b[1]) // 2)
                near = None
                for name, line, cap in lines:
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
    dx, dy = b[0] - a[0], b[1] - a[1]
    return math.sqrt(dx * dx + dy * dy)


def _dist_to_line(p, line):
    best = None
    for a, b in zip(line, line[1:]):
        ex, ey = b[0] - a[0], b[1] - a[1]
        wx, wy = p[0] - a[0], p[1] - a[1]
        ee = ex * ex + ey * ey
        t = 0.0 if ee == 0 else max(0.0, min(1.0, (wx * ex + wy * ey) / ee))
        dx, dy = wx - t * ex, wy - t * ey
        d = math.sqrt(dx * dx + dy * dy)
        if best is None or d < best:
            best = d
    return best if best is not None else math.inf


def _m(p):
    return (p[0] / 1000.0, p[1] / 1000.0)


# Reading through the SDK. A field the world lacks is the one read this lint tolerates, and it says
# so: the measures that need it are `unmeasured` warnings (Sayer). Features and instances are read
# with `*` and filtered here, so an empty selection is an empty list, never an exception. Any other
# failure, an RpcError from the engine included, propagates and fails the step.


def _rows(a):
    """A field array as rows of Python floats (rows from the south)."""
    return a.tolist() if hasattr(a, "tolist") else a


def _field(step, selector, missing):
    """The one field ``selector`` selects, or None with the reason put in ``missing``. The SDK
    answers a selector that selects no field, or several, with LookupError (its only one here)."""
    try:
        got = step.fields([selector])
    except LookupError as e:
        missing[selector] = f"`{selector}` could not be read ({e})"
        return None
    return _rows(got[0])


def _list(col):
    return col.tolist() if hasattr(col, "tolist") else list(col)


def _features(step):
    """Every feature (protocol.md 4.3) as one dict per feature: name, kind, geometry,
    ``rings`` as pieces of rings of (x, y) millimetre pairs, and ``attrs``, the number attributes
    each feature has (its ``<attribute>?`` column is 1)."""
    fl = step.features("*")
    names = _list(fl["name"])
    kinds = _list(fl["kind"])
    geometry = _list(fl["geometry"])
    pieces = _list(fl["pieces"])
    rings = _list(fl["rings"])
    starts = _list(fl["ring_starts"])
    coords = [tuple(int(v) for v in c) for c in _list(fl["coords"])]
    fixed = {"name", "kind", "authority", "layer", "geometry", "pieces", "rings", "ring_starts", "coords"}
    attrs = {}
    for k in fl:
        if k in fixed or k.endswith("?") or (k + "?") not in fl:
            continue
        has = _list(fl[k + "?"])
        vals = _list(fl[k])
        attrs[k] = (has, vals)
    out = []
    for f, name in enumerate(names):
        mine = {}
        for k, (has, vals) in attrs.items():
            if has[f] and isinstance(vals[f], (int, float)) and not isinstance(vals[f], bool):
                mine[k] = float(vals[f])
        ps = []
        for p in range(pieces[f], pieces[f + 1]):
            ps.append([coords[starts[r]:starts[r + 1]] for r in range(rings[p], rings[p + 1])])
        out.append({"name": name, "kind": kinds[f], "geometry": int(geometry[f]), "rings": ps,
                    "attrs": mine})
    return out


def _instances(step, select):
    """An instance table (protocol.md 3.3) as one dict per row: name, asset, by, x, y (mm). A
    selection of nothing is an empty table (protocol.md 3.3), so nothing is caught here."""
    t = step.instances(select)
    cols = {k: _list(t[k]) for k in ("name", "asset", "by", "x", "y")}
    return [{k: cols[k][r] for k in cols} for r in range(len(cols["name"]))]
