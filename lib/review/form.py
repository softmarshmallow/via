"""The review library's street-form lint: how a town's ways are shaped, as numbers (R15), and the
decks over its water (R18).

Run as a plain command through the SDK, declared in the world (protocol.md 1):

    [step.form]
    when    = "lint"
    command = ["python", "-m", "viasdk", "lib:review/form.py"]
    params  = { ways = ["main_street", "street"], meets = ["waterfront", "crossing"],
                edges = ["old_town"], gates = ["west_gate"], water = ["beck"],
                crossings = ["crossing"] }

A measure informs; it is never a target (R15). Each is one ``info`` record, code
``step.<id>.<name>``, subject the step itself (``step.<id>``) for the town's measures and each
water feature for its decks, its number in ``data.value``, with
``data.reference`` (what real towns measure, and where that comes from) and ``data.protocol`` (how
it was taken). The first record of each code carries ``explain``.

| name            | data.value                                                                 |
| bearing_spread  | entropy of the ways' bearings folded to 0..90 degrees, 18 bins of 5        |
|                 | degrees, weighted by length, over ln 18: 0 is one grid, 1 every way alike  |
|                 | (the spike's measure, FINDINGS "A number for boxy")                        |
| bearing_main    | the same over the main ways only                                           |
| order           | orientation order (Boeing 2019): 1 - ((H - ln 4) / (ln 36 - ln 4))^2, H    |
|                 | the entropy of bearings in both directions over 36 bins of 10 degrees      |
|                 | centred on the compass points, weighted by length: 1 a perfect grid, 0     |
|                 | every way alike                                                            |
| four_way        | share of junctions where four or more ways meet                            |
| three_way       | share where three meet                                                     |
| dead_ends       | share of ends of a way on nothing (degree 1), of every junction and end    |
| dead_end_count  | how many; ``data`` also gives ``at_edge`` (an end on a district outline,   |
|                 | ``edges``), ``at_gate`` (within 2 m of a ``gates`` point) and ``stubs``    |
|                 | (a dead end whose piece of way back to its first junction is under 40 m)   |
| decks           | for each water path, how many crossings stand over it: subject the water   |
| doubled_decks   | for each water path, pairs of crossings whose decks lie within 10 m        |
| ways_over_water | for each water path, places a way (``ways``) crosses its line that are not |
|                 | a crossing's: a deck nobody owns, or a way wading through                  |

**The graph.** Every path feature of a kind in ``ways`` and ``meets`` is a line. Two lines meet
where their segments cross or touch, or where an end lies within 5 cm of the other line (the core
puts a way's end on what it meets exactly, geometry.md 2.7; 5 cm only absorbs the millimetre
lattice). Meeting points within 0.5 m are one junction. A junction's degree is the number of pieces
of line leaving it: 2 where a line passes through, 1 where it ends. Degree 2 (a bend, or one way
running on into another) is not a junction and is not counted. A line of ``meets`` (the waterfront,
the crossing) counts as a way here: a street ending on the quay or at a bridge's end is a T, not a
dead end. Bearings are taken from ``ways`` only.

**References** (``data.reference``). The spike's own numbers are measured and fixed (FINDINGS,
2026-11-13: townland 39% dead ends before R18, 15% after; the city alone 10%). The real-town ranges
are from Boeing (2019), "Urban spatial order", Applied Network Science 4:59, which measured 100
cities' drivable networks with OSMnx; they are recalled, not re-measured here, and are given as
wide ranges to check against the paper before anyone relies on them: orientation order above 0.5
for a planned grid (Chicago about 0.9), under 0.1 for an old organic centre; a dead-end share of
roughly 5 to 30% of nodes and a four-way share from under 20% (organic) to over 50% (grid).

Library code imports only viasdk and the standard library (R19b).
"""

import math

SNAP_M = 0.5            # meeting points this close are one junction
TOUCH_M = 0.05          # an end this close to another line meets it
GATE_M = 2.0            # an end this close to a gate is at the gate
STUB_M = 40.0           # a dead end's piece shorter than this is a stub (the spike's)
DOUBLED_M = 10.0        # two crossings' decks this close are one deck laid twice

REF = {
    "bearing_spread": "spike townland 0.87 before R18, 0.83 after; the city alone 0.45, in atlas 0.76 "
                      "(FINDINGS 2026-11-13)",
    "bearing_main": "spike townland 0.97 before R18, 0.94 after (FINDINGS 2026-11-13)",
    "order": "Boeing 2019 (Applied Network Science 4:59), recalled: above 0.5 for a planned grid "
             "(Chicago about 0.9), under 0.1 for an old organic centre; spike townland 0.27",
    "four_way": "spike townland 42% before R18, 47% after; Boeing 2019, recalled: under 20% in an "
                "organic town to over 50% in a grid",
    "three_way": "spike townland 19% before R18, 38% after",
    "dead_ends": "spike townland 39% before R18, 15% after (R18); the city alone 10%; Boeing 2019, "
                 "recalled: about 5 to 30% of nodes",
    "dead_end_count": "spike townland 120 before R18, 35 after (9 stubs, 5 at gates)",
    "decks": "one per crossing; spike townland had 3 overlapping at its old bridge before R18, 1 after",
    "doubled_decks": "0 (R18)",
    "ways_over_water": "0 (R18: ways arrive at a crossing's ends)",
}

EXPLAIN = {
    "bearing_spread": "How alike the ways' directions are (R15): 0 is one grid, 1 every direction "
    "alike. A measure beside the eye, not a target.",
    "bearing_main": "bearing_spread over the main ways only: whether the skeleton is a grid.",
    "order": "Orientation order (Boeing 2019): 1 a perfect grid, 0 every direction alike.",
    "four_way": "The share of junctions where four or more ways meet.",
    "three_way": "The share of junctions where three ways meet.",
    "dead_ends": "The share of junctions and ends that are a way ending on nothing (R18: a meeting "
    "nobody owns shows as a stub or a blunt end).",
    "dead_end_count": "How many ways end on nothing; at the town's edge, at a gate, and stubs.",
    "decks": "How many crossings stand over this water (R18: one deck per crossing).",
    "doubled_decks": "Pairs of crossings whose decks lie on one another: a deck laid twice.",
    "ways_over_water": "Places a way crosses the water's line with no crossing: a deck nobody owns.",
}

HINT = {
    "bearing_spread": "Compare with the reference; the eye decides (R15).",
    "dead_ends": "End a street at a junction or do not lay it; name what owns the meeting (R18).",
    "decks": "Write one crossing per place a way meets the water, and end the ways on it.",
    "doubled_decks": "Keep one of the two crossings.",
    "ways_over_water": "End the way at a crossing's end (`ends_at = \"<crossing>.start\"`).",
}

PROTOCOL = ("lib/review/form.py on the build's feature list: lines are path features of the kinds "
            "in `ways` and `meets`; junctions snapped within 0.5 m, ends within 0.05 m of a line "
            "meet it; bearings from `ways`, weighted by length")


def lint(step):
    p = step.params or {}
    ways = list(p.get("ways", ["main_street", "street"]))
    meets = list(p.get("meets", []))
    feats = _features(step)
    lines = [(f["name"], f["kind"], f["points"]) for f in feats
             if f["geometry"] == 1 and f["kind"] in ways + meets and len(f["points"]) >= 2]
    main = [l for l in lines if l[1] == ways[0]] if ways else []
    bear = [l for l in lines if l[1] in ways]
    outlines = [f["ring"] for f in feats if f["name"] in set(p.get("edges", [])) and f["ring"]]
    gates = [f["points"][0] for f in feats if f["name"] in set(p.get("gates", [])) and f["points"]]
    water = [f for f in feats if f["geometry"] == 1 and f["kind"] in p.get("water", [])]
    crossings = [f for f in feats if f["geometry"] == 1 and f["kind"] in p.get("crossings", [])]
    say = Sayer(step)
    for rec in measure(lines, bear, main, outlines, gates, f"step.{step.id}"):
        say(*rec)
    for rec in measure_decks(water, crossings, [l for l in lines if l[1] in ways]):
        say(*rec)


def measure(lines, bear, main, outlines, gates, subject="town"):
    """The street-form records, as (name, subject, value, data, message)."""
    out = []
    spread = spread_90([pts for _, _, pts in bear])
    out.append(("bearing_spread", subject, spread, {}, f"Bearing spread {spread:.2f} over {len(bear)} ways."))
    sm = spread_90([pts for _, _, pts in main])
    out.append(("bearing_main", subject, sm, {}, f"Bearing spread of the main ways {sm:.2f} over {len(main)}."))
    od = order([pts for _, _, pts in bear])
    out.append(("order", subject, od, {}, f"Orientation order {od:.2f}."))
    nodes = junctions([pts for _, _, pts in lines])
    counted = [n for n in nodes if n["degree"] != 2]
    total = len(counted) or 1
    four = sum(1 for n in counted if n["degree"] >= 4)
    three = sum(1 for n in counted if n["degree"] == 3)
    dead = [n for n in counted if n["degree"] == 1]
    at_edge = sum(1 for n in dead if any(_dist_ring(n["at"], r) <= 1.0 for r in outlines))
    at_gate = sum(1 for n in dead if any(math.dist(n["at"], g) <= GATE_M for g in gates))
    stubs = sum(1 for n in dead if n["piece"] < STUB_M)
    out.append(("four_way", subject, round(four / total, 4), {"count": four, "junctions": len(counted)},
                f"{four} of {len(counted)} junctions are four-way or more."))
    out.append(("three_way", subject, round(three / total, 4), {"count": three, "junctions": len(counted)},
                f"{three} of {len(counted)} junctions are three-way."))
    out.append(("dead_ends", subject, round(len(dead) / total, 4),
                {"count": len(dead), "junctions": len(counted)},
                f"{len(dead)} of {len(counted)} junctions and ends are dead ends."))
    out.append(("dead_end_count", subject, len(dead),
                {"at_edge": at_edge, "at_gate": at_gate, "stubs": stubs,
                 "inside": len(dead) - at_edge - at_gate},
                f"{len(dead)} dead ends: {at_edge} on a district's edge, {at_gate} at a gate, {stubs} stubs."))
    return out


def measure_decks(water, crossings, ways):
    out = []
    for w in water:
        line = w["points"]
        over = [c for c in crossings if _crossings(c["points"], line)]
        doubled = 0
        for i in range(len(over)):
            for j in range(i + 1, len(over)):
                if _line_gap(over[i]["points"], over[j]["points"]) <= DOUBLED_M:
                    doubled += 1
        wading = 0
        for _, _, pts in ways:
            for x in _crossings(pts, line):
                if not any(_dist_line(x, c["points"]) <= DOUBLED_M for c in over):
                    wading += 1
        names = sorted(c["name"] for c in over)
        out.append(("decks", w["name"], len(over), {"crossings": names},
                    f"{len(over)} crossings over {w['name']}: {', '.join(names) or 'none'}."))
        out.append(("doubled_decks", w["name"], doubled, {},
                    f"{doubled} pairs of crossings over {w['name']} lie within {DOUBLED_M:g} m."))
        out.append(("ways_over_water", w["name"], wading, {},
                    f"{wading} places a way crosses {w['name']} with no crossing."))
    return out


# ------------------------------------------------------------------------------------- bearings

def _segments(paths):
    for pts in paths:
        for a, b in zip(pts, pts[1:]):
            dx, dy = b[0] - a[0], b[1] - a[1]
            n = math.hypot(dx, dy)
            if n > 0:
                yield math.degrees(math.atan2(dx, dy)) % 360.0, n      # clockwise from north


def _entropy(weights):
    s = sum(weights)
    if s <= 0:
        return 0.0
    return -sum(w / s * math.log(w / s) for w in weights if w > 0)


def spread_90(paths):
    """The spike's bearing spread: bearings folded to 0..90, 18 bins, length weighted, / ln 18."""
    bins = [0.0] * 18
    any_ = False
    for b, n in _segments(paths):
        f = b % 90.0
        bins[min(int(f // 5.0), 17)] += n
        any_ = True
    return round(_entropy(bins) / math.log(18), 4) if any_ else 0.0


def order(paths):
    """Boeing's orientation order: 36 bins of 10 degrees centred on 0, 10, ...; both directions."""
    bins = [0.0] * 36
    any_ = False
    for b, n in _segments(paths):
        for d in (b, (b + 180.0) % 360.0):
            bins[int(((d + 5.0) % 360.0) // 10.0) % 36] += n
        any_ = True
    if not any_:
        return 0.0
    h, hmax, hg = _entropy(bins), math.log(36), math.log(4)
    return round(max(0.0, 1.0 - ((h - hg) / (hmax - hg)) ** 2), 4)


# ------------------------------------------------------------------------------------- the graph

def junctions(paths):
    """Each junction or end as {at, degree, piece}: ``piece`` is, for an end, the length of line
    back to the first other meeting on its line (a stub's length)."""
    # meeting positions on each line, as (s, point): s the distance along the line
    marks = [[] for _ in paths]
    for i, pts in enumerate(paths):
        marks[i].append((0.0, pts[0]))
        marks[i].append((_length(pts), pts[-1]))
    for i in range(len(paths)):
        for j in range(i + 1, len(paths)):
            for si, sj, x in _meetings(paths[i], paths[j]):
                marks[i].append((si, x))
                marks[j].append((sj, x))
    # snap meeting points into junctions
    clusters = []                                          # [x, y, n]
    grid = {}

    def find(pt):
        cx, cy = int(pt[0] // SNAP_M), int(pt[1] // SNAP_M)
        for gx in (cx - 1, cx, cx + 1):
            for gy in (cy - 1, cy, cy + 1):
                for k in grid.get((gx, gy), ()):
                    c = clusters[k]
                    if math.dist((c[0], c[1]), pt) <= SNAP_M:
                        return k
        clusters.append([pt[0], pt[1]])
        grid.setdefault((cx, cy), []).append(len(clusters) - 1)
        return len(clusters) - 1

    degree = {}
    piece = {}
    for i, pts in enumerate(paths):
        total = _length(pts)
        seen = {}
        for s, x in sorted(marks[i]):
            k = find(x)
            seen.setdefault(k, []).append(s)
        ks = sorted(seen, key=lambda k: min(seen[k]))
        if len(ks) == 1 and total > 0:
            # a line meeting nothing: two ends, or one closed loop through one point
            k = ks[0]
            degree[k] = degree.get(k, 0) + (2 if pts[0] == pts[-1] else 1)
            continue
        for idx, k in enumerate(ks):
            ss = seen[k]
            ends = sum(1 for s in ss if s <= 1e-9 or s >= total - 1e-9)
            closed = pts[0] == pts[-1]
            if closed or not ends:
                d = 2
            else:
                d = 1
                # an end: the piece of line from it to the next meeting
                if idx == 0:
                    piece[k] = min(piece.get(k, math.inf), min(seen[ks[1]]) - max(ss))
                elif idx == len(ks) - 1:
                    piece[k] = min(piece.get(k, math.inf), min(ss) - max(seen[ks[-2]]))
            degree[k] = degree.get(k, 0) + d
    out = []
    for k, c in enumerate(clusters):
        if k in degree:
            out.append({"at": (c[0], c[1]), "degree": degree[k], "piece": piece.get(k, math.inf)})
    return out


def _length(pts):
    return sum(math.dist(a, b) for a, b in zip(pts, pts[1:]))


def _meetings(p, q):
    """Where lines p and q meet: (s along p, s along q, point), segments crossing or touching, and
    an end of one within TOUCH_M of the other."""
    out = []
    sp = 0.0
    qcum = [0.0]
    for a, b in zip(q, q[1:]):
        qcum.append(qcum[-1] + math.dist(a, b))
    qbox = _box(q)
    for a, b in zip(p, p[1:]):
        la = math.dist(a, b)
        if _overlaps((min(a[0], b[0]), min(a[1], b[1]), max(a[0], b[0]), max(a[1], b[1])), qbox, TOUCH_M):
            for j, (c, d) in enumerate(zip(q, q[1:])):
                hit = _cross(a, b, c, d)
                if hit is not None:
                    u, v, x = hit
                    out.append((sp + u * la, qcum[j] + v * math.dist(c, d), x))
        sp += la
    for end, s_end in ((p[0], 0.0), (p[-1], _length(p))):
        s, x, gap = _nearest(q, end)
        if gap <= TOUCH_M:
            out.append((s_end, s, end))
    for end, s_end in ((q[0], 0.0), (q[-1], qcum[-1])):
        s, x, gap = _nearest(p, end)
        if gap <= TOUCH_M:
            out.append((s, s_end, end))
    return out


def _box(pts):
    xs = [x for x, _ in pts]
    ys = [y for _, y in pts]
    return min(xs), min(ys), max(xs), max(ys)


def _overlaps(a, b, m):
    return a[0] - m <= b[2] and b[0] - m <= a[2] and a[1] - m <= b[3] and b[1] - m <= a[3]


def _cross(a, b, c, d):
    r = (b[0] - a[0], b[1] - a[1])
    s = (d[0] - c[0], d[1] - c[1])
    den = r[0] * s[1] - r[1] * s[0]
    if den == 0:
        return None
    qp = (c[0] - a[0], c[1] - a[1])
    u = (qp[0] * s[1] - qp[1] * s[0]) / den
    v = (qp[0] * r[1] - qp[1] * r[0]) / den
    if 0.0 <= u <= 1.0 and 0.0 <= v <= 1.0:
        return u, v, (a[0] + u * r[0], a[1] + u * r[1])
    return None


def _nearest(pts, p):
    """(s along pts, nearest point, distance)."""
    best = (0.0, pts[0], math.inf)
    s = 0.0
    for a, b in zip(pts, pts[1:]):
        dx, dy = b[0] - a[0], b[1] - a[1]
        n2 = dx * dx + dy * dy
        u = 0.0 if n2 == 0 else max(0.0, min(1.0, ((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / n2))
        x = (a[0] + u * dx, a[1] + u * dy)
        g = math.dist(x, p)
        if g < best[2]:
            best = (s + u * math.sqrt(n2), x, g)
        s += math.sqrt(n2)
    return best


def _dist_line(p, pts):
    return _nearest(pts, p)[2]


def _dist_ring(p, ring):
    return _nearest(ring + ring[:1], p)[2]


def _crossings(p, q):
    """Points where line p crosses line q."""
    out = []
    for a, b in zip(p, p[1:]):
        for c, d in zip(q, q[1:]):
            hit = _cross(a, b, c, d)
            if hit is not None:
                out.append(hit[2])
    return out


def _line_gap(p, q):
    if _crossings(p, q):
        return 0.0
    return min(min(_dist_line(x, q) for x in p), min(_dist_line(x, p) for x in q))


# ------------------------------------------------------------------------------------- reading

def _list(col):
    return col.tolist() if hasattr(col, "tolist") else list(col)


def _features(step):
    """Every feature as {name, kind, geometry, points (metres, first ring), ring (metres, the
    outer ring of a region)}."""
    fl = step.features("*")
    names = _list(fl["name"])
    kinds = _list(fl["kind"])
    geometry = _list(fl["geometry"])
    pieces = _list(fl["pieces"])
    rings = _list(fl["rings"])
    starts = _list(fl["ring_starts"])
    coords = [(int(c[0]) / 1000.0, int(c[1]) / 1000.0) for c in _list(fl["coords"])]
    out = []
    for f, name in enumerate(names):
        pts = []
        if pieces[f + 1] > pieces[f]:
            r = rings[pieces[f]]
            pts = coords[starts[r]:starts[r + 1]]
        g = int(geometry[f])
        out.append({"name": name, "kind": kinds[f], "geometry": g, "points": pts,
                    "ring": pts if g == 2 else []})
    return out


class Sayer:
    def __init__(self, step):
        self.step = step
        self.explained = set()

    def __call__(self, name, subject, value, data, message):
        d = dict(data)
        d["value"] = value
        d["reference"] = REF[name]
        d["protocol"] = PROTOCOL
        kw = {"code": name, "subject": subject, "message": message,
              "hint": HINT.get(name, HINT["bearing_spread"]), "data": d}
        if name not in self.explained:
            kw["explain"] = EXPLAIN[name]
            self.explained.add(name)
        self.step.info(**kw)
