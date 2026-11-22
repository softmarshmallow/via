"""The town library's kit run step: a kit of pieces laid along a path, with ends, corners, fillers
and steps (lib/town; the TODO's "kit contract": "kit run end pieces, filler and spacing" and
"corners, ends and stepped runs are untested").

A generate step, run as a worker through the SDK. A world declares it with `params.runs` (path
features: a garden wall, a town wall, a quay's railing) and `params.kit` (a taxon path whose
pieces carry a `role:` tag and `params.pitch`, the length a piece repeats at along its x axis,
lib/building's convention, assets.md 10). It returns instances; it cuts and levels nothing.

The kit (the pieces under `kit`, the first id bytewise per role):

- `role:wall`, the bay, repeated along a stretch (required);
- `role:filler`, a shorter piece for what whole bays leave (optional);
- `role:end`, the piece at a free end of the run and either side of a gap (optional; without it
  a stretch ends with its last bay);
- `role:corner`, the piece at a turn of the path (optional; without it the path's turns are
  ends of stretches, each with its `end` piece).

How it lays (ours, written for this library; each rule has its test in test_kit.py):

1. **Legs and corners.** The path is cut at each vertex where it turns by `CORNER_TURN` (10
   degrees) or more; a corner piece stands centred on that vertex, turned with the leg before it.
   Each leg is laid on its own; a vertex that turns less is walked through (the pieces follow the
   path by arc length, each turned to the chord between its own ends).
2. **Gaps.** Where the land's `keep_out` (every `role:keep_out` field, the largest) is above 0 on
   the path or `GAP_SIDE` (0.5 m) to either side of it, or half the bay's plan (its footprint's
   depth: the piece a run is made of; a deeper end or corner is the core's to report, rules.md 8.3)
   less `GAP_MARGIN` when that is more (read every half metre across), read every
   `GAP_STEP` (0.25 m) along, nothing
   stands: a way's carriageway and footway, the quay's deck. Each such span is widened by
   `GAP_MARGIN` (0.5 m, a post) each side, so no piece's footprint reads the street's `keep_out`
   (rules.md 8.3). A gap over a corner takes the corner piece away. What is left of a leg are its
   stretches.
3. **Ends inclusive.** A stretch's caps stand on its two ends: an `end` piece centred on an end
   that is the path's end or a gap's edge, so the run reaches its line's last millimetre (as a
   core run's `ends = "both"`, rules.md 5.1); at a corner, the corner piece, which the bays keep
   clear of by half its plan's diagonal (so a bay never overlaps it, whatever the turn).
4. **Bays and fillers.** Between the caps, the most whole bays, then the most fillers in what is
   left (lib/building's `fit`); the fillers split between the two ends (the lower half at the
   start), so the bays stay centred.
5. **Spacing.** What bays and fillers leave (less than one filler, or one bay without a filler)
   goes first to a cap at a gap's edge, which moves into the gap by that much (the gap only
   widens; with a gap at both ends, half each), so the stretch closes exactly. With no such cap
   (between two corners, or a corner and the path's end) it is spread evenly over every joint
   between the stretch's pieces and its caps, so the first and
   last pieces still touch their caps' line and every joint opens the same; at most `joint`
   (0.05 m) a joint. A stretch whose joints would open further is `open` (error), naming the
   two nearest lengths that would close (the TODO's kit contract: "a run that does not close is
   a compile error that names the lengths that would"); it is not laid.
6. **Stepped on a slope.** A piece stands at the ground under its origin, level (the core
   grounds an instance at its origin, rules.md), so on a slope a run steps at its joints. Each
   piece reaches `sink` metres (its pack fact) below its origin: the ground is read at both its
   ends, and a piece whose ends fall or rise more than its `sink` from its origin (a foot in the
   air, or buried past its plinth) is laid as fillers instead, `pitch / filler pitch` of them,
   when the pitches divide; a piece still past it is laid and counted (`steep`, warning). The
   largest step between two neighbours' tops is reported (`max_step`).

7. **Apart on a bend and on the lattice** (ours, asked by `market`: a step's instances are tested
   for overlap as hand instances are, rules.md 8.1, neighbours included). Where two neighbours'
   chords turn by an angle, their inner corners meet: the joint opens to at least `depth *
   tan(turn / 2)` (the deeper piece's plan depth, its footprint's). Then each piece is placed as the core will
   place it (origin to the millimetre, face to 0.001 degree) and, while its plan rectangle
   (pitch by depth, off the axes a millimetre more each way: the core rounds each footprint
   corner to the lattice) still meets its neighbour's inside, the joint opens a millimetre more: on a
   line that is not on the axes, abutting pieces at their exact pitch overlap by rounding. A
   stretch that no longer holds its pieces gives up the filler nearest its end, else the bay
   (`dropped`); `opened` counts the joints this rule opened.

Names are by place (identity.md 2.2): `<run>_c<v>` the corner at the path's vertex `v`;
`<run>_<leg>_<stretch>_<k>` the `k`th piece of a leg's stretch from its start; a bay split by rule
6 gives `<..._k>_<j>`. A gap that moves renames the pieces of the stretches beside it.

`params`: `runs` (names of path features, required); `kit` (taxon path, required); `side`
(`"left"` or `"right"`: which side of the path the pieces' fronts face, default `"left"`);
`joint` (metres, 0 or more, default 0.05); `caps` (`"centred"`, default, or `"inside"`; it was
`ends` until the M5 close, renamed so `ends` keeps the core run's one meaning, rules.md 5.1).

**Caps inside** (`caps = "inside"`, ours, asked by the `market` world's house rows): an `end`
piece stands wholly inside its stretch, its side on the stretch's end, so a row of houses never
reaches into the gap a cross street makes (a 6 m end house centred on the gap's edge stood 3 m
into it). Its cap reach (rule 3) is its whole pitch; and a free end of the path takes what whole
pieces leave as a gap's edge does (rule 5: the row stops short of its line by less than a filler),
since a row's line says where houses may stand, not a length they must close.

Library code imports only viasdk and the standard library (R19b).
"""

import json
import math

from viasdk import RpcError

CORNER_TURN = 10.0                                    # degrees a vertex turns, at least, for a corner
GAP_STEP = 0.25                                       # metres between keep_out readings along a run
GAP_SIDE = 0.5                                        # metres either side of the path also read
GAP_MARGIN = 0.5                                      # metres each gap is widened by, each side
JOINT = 0.05                                          # metres a joint may open, at most (rule 5)
FIT = 0.005                                           # metres: a stretch this near whole pieces closes

EXPLAIN = {
    "kit": "What the kit run step laid: pieces by role, stretches, gaps, bays laid as fillers on a "
    "slope (`split`), pieces still steeper than their plinth (`steep`) and the largest step between "
    "two neighbours' tops (`max_step`, metres).",
    "open": "A stretch of a kit run is no whole number of bays and fillers, and spreading what is "
    "left over its joints would open each by more than `joint`. The data names the nearest "
    "lengths below and above that would close; move a gap or the path, add a filler of another "
    "pitch to the kit, or raise `joint`.",
    "steep": "Some pieces of a kit run stand on a slope steeper than their plinth: the ground at one "
    "end lies more than the piece's `sink` above or below its origin, even as fillers.",
    "params": "A param of the kit run step is outside what it takes: `runs` names path features; "
    "`kit` a taxon path whose pieces carry `role:wall` (and optionally `role:filler`, `role:end`, "
    "`role:corner`); `side` is `left` or `right`; `joint` is 0 or more metres.",
}


class ParamError(ValueError):
    def __init__(self, key, message, hint):
        super().__init__(message)
        self.key, self.message, self.hint = key, message, hint


def params(raw):
    runs = raw.get("runs")
    if not isinstance(runs, list) or not runs or not all(isinstance(r, str) for r in runs):
        raise ParamError("runs", "`runs` is not a list of names", "name the path features to lay")
    kit = raw.get("kit")
    if not isinstance(kit, str) or not kit:
        raise ParamError("kit", "`kit` is not a taxon path", "write the kit's taxon, as `structure/wall`")
    side = raw.get("side", "left")
    if side not in ("left", "right"):
        raise ParamError("side", f"`side` is {side!r}", "write `left` or `right`")
    joint = raw.get("joint", JOINT)
    if isinstance(joint, bool) or not isinstance(joint, (int, float)) or not joint >= 0:
        raise ParamError("joint", f"`joint` is {joint!r}", "write 0 or more metres")
    caps = raw.get("caps", "centred")
    if caps not in ("centred", "inside"):
        raise ParamError("caps", f"`caps` is {caps!r}", "write `centred` or `inside`")
    return runs, kit, side, float(joint), caps == "inside"


# ------------------------------------------------------------------------------------- the kit


def kit_pieces(rows):
    """{role: {id, pitch, width, depth, sink, plan}} (metres) from pack rows (id, tags, bounds in
    mm, params, sink, footprint in mm); the first id bytewise per role, a piece with no
    `variant:` tag before one with (a variant is never the bay: docs/design/kit-contract.md).
    `pitch` is `params.pitch`, else the bounds' width; `plan` the footprint's depth, else the
    bounds'."""
    out = {}
    for r in sorted(rows, key=lambda r: (any(t.startswith("variant:") for t in r["tags"]), r["id"])):
        for tag in r["tags"]:
            if not tag.startswith("role:") or tag[5:] in out:
                continue
            b = r["bounds"] or [0, 0, 0, 0, 0, 0]
            width, depth = (b[3] - b[0]) / 1000.0, (b[4] - b[1]) / 1000.0
            pitch = (r.get("params") or {}).get("pitch")
            fp = [v[1] / 1000.0 for v in (r.get("footprint") or [])]
            out[tag[5:]] = {"id": r["id"], "pitch": float(pitch) if pitch else width,
                            "width": width, "depth": depth, "sink": float(r.get("sink") or 0.0),
                            # the plan's depth the core tests (its footprint), else the bounds'
                            "plan": max(fp) - min(fp) if fp else depth}
    return out


# ------------------------------------------------------------------------------- rules 1 and 2


def legs(points, turn=CORNER_TURN):
    """Rule 1: the path cut at its corners. [(first vertex index, [points])] and the corner
    vertices [(index, point, bearing of the leg before it in degrees)]."""
    corners, out, start = [], [], 0
    for i in range(1, len(points) - 1):
        a = _unit(_sub(points[i], points[i - 1]))
        b = _unit(_sub(points[i + 1], points[i]))
        if math.degrees(math.acos(max(-1.0, min(1.0, _dot(a, b))))) >= turn:
            out.append((start, points[start:i + 1]))
            corners.append((i, points[i], _bearing(a)))
            start = i
    out.append((start, points[start:]))
    return out, corners


def stretches(length, gaps, start_cap, end_cap):
    """Rule 2: a leg of `length` metres less its `gaps` (arc-length spans on it, any order, may
    overlap or reach past its ends): [(s0, s1, start cap, end cap)], each cap `"end"`, `"corner"`
    or None; a cap at a gap's edge is `"gap"` (an end piece that may move into the gap, rule 5)."""
    spans = sorted((max(0.0, a), min(length, b)) for a, b in gaps if b > 0 and a < length)
    out, s, cap = [], 0.0, start_cap
    for a, b in spans:
        if a > s:
            out.append((s, a, cap, "gap"))
        if b > s:
            s, cap = b, "gap"
    if s < length:
        out.append((s, length, cap, end_cap))
    return out


def gap_spans(readings, step=GAP_STEP, margin=GAP_MARGIN):
    """Rule 2: arc-length spans where a reading (one per `step` metres from 0) is above 0, each
    widened by `margin` each side."""
    out, a = [], None
    for k, v in enumerate(readings):
        if v > 1e-6 and a is None:
            a = k * step
        elif v <= 1e-6 and a is not None:
            out.append((a - margin, (k - 1) * step + margin))
            a = None
    if a is not None:
        out.append((a - margin, (len(readings) - 1) * step + margin))
    return out


# ---------------------------------------------------------------------------- rules 3, 4 and 5


def cap_reach(kit, cap, inside=False):
    """Metres a stretch's cap takes from its end (rule 3): half an `end` piece's pitch (its whole
    pitch with ends inside); half a corner's plan diagonal; 0 with no cap or no such piece."""
    if cap in ("end", "gap") and "end" in kit:
        return kit["end"]["pitch"] / (1 if inside else 2)
    if cap == "corner" and "corner" in kit:
        c = kit["corner"]
        return math.hypot(c["width"], c["depth"]) / 2
    return 0.0


def fit(length, pitch, filler, tol=FIT):
    """Rule 4: the most bays `n`, then the most fillers `k`, within `length` (metres): (n, k,
    what is left)."""
    n = int((length + tol) // pitch)
    rest = length - n * pitch
    k = int((rest + tol) // filler) if filler else 0
    return n, k, max(0.0, rest - k * (filler or 0.0))


def lay_stretch(length, kit, start_cap, end_cap, joint=JOINT, inside=False):
    """Rules 3 to 5 on one stretch `length` metres long: ([(role, middle, piece length)], None),
    the middles in metres from the stretch's start, caps included; or (None, (below, above)), the
    nearest lengths that close, when the stretch is open. A stretch shorter than its caps holds
    one `end` piece at its middle when one fits, else nothing. With `inside`, ends stand inside
    the stretch and a free end slides as a gap's edge does (module doc, "Ends inside")."""
    c0, c1 = cap_reach(kit, start_cap, inside), cap_reach(kit, end_cap, inside)
    wall, filler = kit["wall"], kit.get("filler")
    inner = length - c0 - c1
    if inner < -FIT:
        # With ends inside, a corner cap keeps its reach: the one end piece stands in what is left.
        r0 = c0 if inside and start_cap == "corner" else 0.0
        r1 = c1 if inside and end_cap == "corner" else 0.0
        if "end" in kit and length - r0 - r1 >= kit["end"]["pitch"]:
            return [("end", r0 + (length - r0 - r1) / 2, kit["end"]["pitch"])], None
        return [], None
    inner = max(inner, 0.0)
    q = filler["pitch"] if filler else 0.0
    n, k, rest = fit(inner, wall["pitch"], q)
    pieces = n + k
    # A cap at a gap's edge moves into the gap by what whole pieces leave (both, half each).
    slide = [start_cap == "gap" or (inside and start_cap == "end" and "end" in kit),
             end_cap == "gap" or (inside and end_cap == "end" and "end" in kit)]
    shift0 = shift1 = 0.0
    if rest > FIT and any(slide):
        shift0 = rest * (0.5 if all(slide) else 1.0) if slide[0] else 0.0
        shift1 = rest - shift0
        rest = 0.0
    caps, half = [], kit["end"]["pitch"] / 2 if inside and "end" in kit else 0.0
    if start_cap in ("end", "gap") and "end" in kit:
        caps.append(("end", shift0 + half, kit["end"]["pitch"]))
    if end_cap in ("end", "gap") and "end" in kit:
        caps.append(("end", length - shift1 - half, kit["end"]["pitch"]))
    if rest > FIT and rest / (pieces + 1) > joint + 1e-9:
        closes = inner - rest
        return None, (round(closes + c0 + c1, 3), round(closes + (q or wall["pitch"]) + c0 + c1, 3))
    gap = rest / (pieces + 1) if rest > FIT else 0.0
    order = [("filler", q)] * (k // 2) + [("wall", wall["pitch"])] * n + [("filler", q)] * (k - k // 2)
    out, s = [], c0 + shift0
    for role, p in order:
        s += gap
        out.append((role, s + p / 2, p))
        s += p
    return sorted(caps + out, key=lambda r: r[1]), None


# -------------------------------------------------------------------------------------- rule 6


def steps(pieces, ground, kit):
    """Rule 6 on one stretch's pieces [(role, middle, length)] with `ground(s)` the land's height
    at arc length `s`: each piece too steep for its plinth laid as fillers when the pitches
    divide. Returns ([(role, middle, length, i, j)], steep count): `i` the index of the piece in
    `pieces`, `j` None or the filler's place in the piece it was cut from."""
    filler = kit.get("filler")
    out, steep = [], 0
    for i, (role, mid, length) in enumerate(pieces):
        if _fall(ground, mid, length) <= kit[role]["sink"] + 1e-9:
            out.append((role, mid, length, i, None))
            continue
        q = filler["pitch"] if filler else 0.0
        m = int(round(length / q)) if q and role == "wall" else 0
        if m > 1 and abs(m * q - length) <= FIT:
            for j in range(m):
                c = mid - length / 2 + (j + 0.5) * q
                steep += _fall(ground, c, q) > filler["sink"] + 1e-9
                out.append(("filler", c, q, i, j))
        else:
            steep += 1
            out.append((role, mid, length, i, None))
    return out, steep


def asked(pieces, kit):
    """The arc lengths `steps` and `max_step` read the ground at, for one stretch's pieces: each
    piece's middle and ends, and those of the fillers it could be cut into."""
    q = kit["filler"]["pitch"] if "filler" in kit else 0.0
    out = []
    for role, mid, length in pieces:
        out += [mid - length / 2, mid, mid + length / 2]
        if q and role == "wall":
            for j in range(int(round(length / q))):
                c = mid - length / 2 + (j + 0.5) * q
                out += [c - q / 2, c, c + q / 2]
    return out


def _fall(ground, mid, length):
    z = ground(mid)
    return max(abs(ground(mid - length / 2) - z), abs(ground(mid + length / 2) - z))


def max_step(pieces, ground):
    """The largest difference between two neighbouring pieces' origins' ground, metres: the step
    between their tops for pieces of one height."""
    zs = [ground(p[1]) for p in pieces]
    return max((abs(b - a) for a, b in zip(zs, zs[1:])), default=0.0)


# ------------------------------------------------------------------------------------- geometry


def arc(points):
    """Cumulative arc lengths of a polyline (metres)."""
    out = [0.0]
    for a, b in zip(points, points[1:]):
        out.append(out[-1] + math.dist(a, b))
    return out


def at_arc(points, cum, s):
    """The point at arc length `s` (clamped to the line)."""
    s = min(max(s, 0.0), cum[-1])
    for k in range(len(points) - 1):
        if s <= cum[k + 1] or k == len(points) - 2:
            n = cum[k + 1] - cum[k]
            t = 0.0 if n == 0 else (s - cum[k]) / n
            return (points[k][0] + (points[k + 1][0] - points[k][0]) * t,
                    points[k][1] + (points[k + 1][1] - points[k][1]) * t)
    return points[-1]


def placed(points, cum, mid, length, side):
    """A piece's origin and face (degrees) at arc length `mid`: turned to the chord between its
    ends, its front (+y) to `side` of the path, its +x along it (left) or against it (right)."""
    a, b = at_arc(points, cum, mid - length / 2), at_arc(points, cum, mid + length / 2)
    t = _unit(_sub(b, a))
    if t == (0.0, 0.0):
        t = _unit(_sub(points[-1], points[0]))
    front = (-t[1], t[0]) if side == "left" else (t[1], -t[0])
    return at_arc(points, cum, mid), round(_bearing(front), 3)


def _rect(c, face, hx, hy):
    """A plan rectangle (corners, axes) at `c`, its front (+y) to bearing `face` degrees."""
    th = math.radians(face)
    f = (math.sin(th), math.cos(th))
    x = (f[1], -f[0])
    return ([(c[0] + sx * hx * x[0] + sy * hy * f[0], c[1] + sx * hx * x[1] + sy * hy * f[1])
             for sx, sy in ((-1, -1), (1, -1), (1, 1), (-1, 1))], (x, f))


def _meet(a, b):
    """True when two plan rectangles' insides meet (separating axes)."""
    for ax in a[1] + b[1]:
        pa = [p[0] * ax[0] + p[1] * ax[1] for p in a[0]]
        pb = [p[0] * ax[0] + p[1] * ax[1] for p in b[0]]
        if min(max(pa), max(pb)) - max(min(pa), min(pb)) <= 1e-9:
            return False
    return True


def apart(pieces, kit, points, cum, s0, side, limit):
    """Rule 7 on one stretch's pieces [(role, middle, length)] (middles from `s0` along the path):
    (pieces, joints opened, pieces dropped). `limit` is how far from `s0` the last piece's far end
    may reach."""
    base, dropped = list(pieces), 0
    while True:
        out, opened, carry = [base[0]] if base else [], 0, 0.0
        for role, mid, length in base[1:]:
            a = out[-1]
            mid += carry                                      # what the joints before opened
            pa = placed(points, cum, s0 + a[1], a[2], side)
            pb = placed(points, cum, s0 + mid, length, side)
            turn = abs((pb[1] - pa[1] + 180.0) % 360.0 - 180.0)
            want = max(kit[a[0]]["plan"], kit[role]["plan"]) * math.tan(math.radians(turn) / 2)
            gap = (mid - length / 2) - (a[1] + a[2] / 2)
            more = max(0.0, want - gap)
            # Each half-size a millimetre more off the axes: the core rounds each footprint corner
            # to the lattice (on the axes the corners are on it already).
            pad = 0.0 if pa[1] % 90.0 == 0.0 else 0.001
            ra = _rect(tuple(round(v, 3) for v in pa[0]), pa[1], a[2] / 2 + pad, kit[a[0]]["plan"] / 2 + pad)
            for _ in range(2000):
                pb = placed(points, cum, s0 + mid + more, length, side)
                pad = 0.0 if pb[1] % 90.0 == 0.0 else 0.001
                rb = _rect(tuple(round(v, 3) for v in pb[0]), pb[1], length / 2 + pad, kit[role]["plan"] / 2 + pad)
                if not _meet(ra, rb):
                    break
                more += 0.001
            opened += more > 0
            carry += more
            out.append((role, mid + more, length))
        if not out or out[-1][1] + out[-1][2] / 2 <= limit + FIT:
            return out, opened, dropped
        k = next((k for k in range(len(base) - 1, -1, -1) if base[k][0] == "filler"), None)
        if k is None:
            k = next((k for k in range(len(base) - 1, -1, -1) if base[k][0] == "wall"), None)
        if k is None:
            return out, opened, dropped
        gone = base.pop(k)
        base = base[:k] + [(r, m - gone[2], q) for r, m, q in base[k:]]
        dropped += 1


def _sub(a, b):
    return (a[0] - b[0], a[1] - b[1])


def _dot(a, b):
    return a[0] * b[0] + a[1] * b[1]


def _unit(v):
    n = math.hypot(v[0], v[1])
    return (v[0] / n, v[1] / n) if n else (0.0, 0.0)


def _bearing(v):
    return math.degrees(math.atan2(v[0], v[1])) % 360.0


def _mm(p):
    return (int(round(p[0] * 1000)), int(round(p[1] * 1000)))


def _list(col):
    return col.tolist() if hasattr(col, "tolist") else list(col)


def _json(v):
    if isinstance(v, (bytes, str)):
        try:
            return json.loads(v)
        except ValueError:
            return v
    return v


# ------------------------------------------------------------------------------------- the step


def read_kit(step, taxon):
    t = step.pack(select="@" + taxon, columns=["bounds", "params", "sink", "footprint"])
    ids = _list(t.get("id", []))
    rows = []
    for r, i in enumerate(ids):
        tags = _json(_list(t["tags"])[r]) if "tags" in t else []
        b = _list(t["bounds"])[r] if "bounds" in t else None
        rows.append({"id": str(i), "tags": tags if isinstance(tags, list) else [tags],
                     "bounds": [int(v) for v in _list(b)] if b is not None else None,
                     "params": (_json(_list(t["params"])[r]) or {}) if "params" in t else {},
                     # `sink` is a height: f64 metres, not millimetres (protocol.md 3.3).
                     "sink": float(_list(t["sink"])[r]) if "sink" in t else 0.0,
                     "footprint": (_json(_list(t["footprint"])[r]) or []) if "footprint" in t else []})
    return kit_pieces(rows)


def _field(step, role):
    try:
        fields = step.fields([role], window=[0, 0, 1, 1])
    except (LookupError, RpcError):
        return None
    if not fields:
        return None
    formula = f"f('{fields[0].name}')"
    for fld in fields[1:]:
        formula = f"max({formula}, f('{fld.name}'))"
    return formula


def _eval(step, formula, pts):
    if not pts or formula is None:
        return [0.0] * len(pts)
    ext = (step.grid or {}).get("extent_mm")
    px, py = [], []
    for p in pts:
        x, y = _mm(p)
        if ext:
            x, y = min(max(x, ext[0]), ext[2]), min(max(y, ext[1]), ext[3])
        px.append(x)
        py.append(y)
    return [float(v) for v in _list(step.eval("= " + formula, x=px, y=py))]


def generate(step):
    try:
        runs, taxon, side, joint, inside = params(step.params)
    except ParamError as e:
        step.error("params", f"step.{step.id}.params.{e.key}", e.message, e.hint,
                   explain=EXPLAIN["params"])
        return
    kit = read_kit(step, taxon)
    if "wall" not in kit:
        step.error("params", f"step.{step.id}.params.kit", f"no piece under `{taxon}` has `role:wall`",
                   "tag the kit's bay `role:wall`", explain=EXPLAIN["params"])
        return
    fl = step.features(select=runs)
    paths = {}
    for f, name in enumerate(fl.names):
        for piece in fl.rings(f):
            for r in piece:
                paths[name] = [(float(v[0]) / 1000.0, float(v[1]) / 1000.0) for v in _list(r)]
    keep = _field(step, "role:keep_out")
    surface = _field(step, "role:surface")
    count = {"pieces": 0, "wall": 0, "filler": 0, "end": 0, "corner": 0, "stretches": 0,
             "gaps": 0, "split": 0, "steep": 0, "open": 0, "opened": 0, "dropped": 0}
    worst = 0.0
    for run in runs:
        if run not in paths:
            continue
        cut, corners = legs(paths[run])
        # Rule 2: the line and `GAP_SIDE` either side of it, or half the bay's plan depth
        # less `GAP_MARGIN` (a post) when that is more (a row of houses whose fronts would reach a
        # cross street's verge; the core tests the posts inside a footprint, not its edge, rules.md
        # 8.3, and a reading between posts is interpolated: a post half a metre outside still shows).
        half = max(GAP_SIDE, kit["wall"]["plan"] / 2 - GAP_MARGIN)
        sides = sorted({0.0, GAP_SIDE, -GAP_SIDE, half, -half} |               # every post across
                       {sg * k * GAP_STEP * 2 for k in range(1, int(half / (GAP_STEP * 2)) + 1) for sg in (1, -1)})
        legs_ = []                                     # (points, cum, gaps)
        samples = []
        for v0, pts in cut:
            cum = arc(pts)
            for k in range(int(cum[-1] / GAP_STEP) + 1):
                c = at_arc(pts, cum, k * GAP_STEP)
                t = _unit(_sub(at_arc(pts, cum, k * GAP_STEP + 0.01),
                               at_arc(pts, cum, k * GAP_STEP - 0.01)))
                samples += [(c[0] - t[1] * o, c[1] + t[0] * o) for o in sides]
            legs_.append((pts, cum))
        v = _eval(step, keep, samples)
        k0, gapped = 0, []
        for pts, cum in legs_:
            n = int(cum[-1] / GAP_STEP) + 1
            m = len(sides)
            gapped.append(gap_spans([max(v[k0 + m * k:k0 + m * k + m]) for k in range(n)]))
            k0 += m * n
        # A corner a gap reaches is no corner: both legs end there with an end piece.
        gone = [any(a <= cum[-1] <= b for a, b in gapped[li]) or
                any(a <= 0 <= b for a, b in gapped[li + 1])
                for li, (pts, cum) in enumerate(legs_[:-1])]
        laid = []                                      # (leg, stretch, points, cum, s0, pieces)
        for li, (pts, cum) in enumerate(legs_):
            count["gaps"] += len(gapped[li])
            c_start = "end" if li == 0 or gone[li - 1] else "corner"
            c_end = "end" if li == len(legs_) - 1 or gone[li] else "corner"
            for si, (s0, s1, c0, c1) in enumerate(stretches(cum[-1], gapped[li], c_start, c_end)):
                got, closes = lay_stretch(s1 - s0, kit, c0, c1, joint, inside)
                if got is None:
                    count["open"] += 1
                    step.error("open", f"step.{step.id}", f"`{run}` has a stretch of "
                               f"{s1 - s0:.3f} m that whole pieces do not close.",
                               f"Lengths {closes[0]} or {closes[1]} m would close; move the path "
                               "or a gap, or add a filler.", data={"run": run, "leg": li,
                               "stretch": si, "length": round(s1 - s0, 3), "closes": list(closes)},
                               explain=EXPLAIN["open"])
                    continue
                count["stretches"] += 1
                if got:                                # rule 7: how far the last piece may slide
                    far = got[-1][1] + got[-1][2] / 2
                    if inside and c1 in ("end", "gap"):
                        limit = (s1 - s0)
                    elif c1 == "gap":
                        limit = far + (s1 - s0) - got[-1][1]
                    else:
                        limit = far
                    got, opened, dropped = apart(got, kit, pts, cum, s0, side, limit)
                    count["opened"] += opened
                    count["dropped"] += dropped
                laid.append((li, si, pts, cum, s0, got))
        ask = [at_arc(pts, cum, s0 + s) for _, _, pts, cum, s0, got in laid for s in asked(got, kit)]
        heights = dict(zip([_mm(p) for p in ask], _eval(step, surface, ask))) if surface else {}
        for li, si, pts, cum, s0, got in laid:
            def ground(s, pts=pts, cum=cum, s0=s0):
                return heights.get(_mm(at_arc(pts, cum, s0 + s)), 0.0)
            if heights:
                pieces, steep = steps(got, ground, kit)
                worst = max(worst, max_step(pieces, ground))
            else:
                pieces, steep = [g + (i, None) for i, g in enumerate(got)], 0
            count["steep"] += steep
            count["split"] += len({p[3] for p in pieces if p[4] is not None})
            for role, mid, length, i, j in pieces:
                at, face = placed(pts, cum, s0 + mid, length, side)
                key = f"{run}_{li}_{si}_{i}" + ("" if j is None else f"_{j}")
                step.out.instance(key, kit[role]["id"], at=_mm(at), face=face)
                count[role] += 1
                count["pieces"] += 1
        if "corner" in kit:
            for (i, p, bearing), g in zip(corners, gone):
                if g:
                    continue
                step.out.instance(f"{run}_c{i}", kit["corner"]["id"], at=_mm(p), face=round(
                    (bearing - 90.0 if side == "left" else bearing + 90.0) % 360.0, 3))
                count["corner"] += 1
                count["pieces"] += 1
    step.info("kit", "step." + step.id,
              f"{count['pieces']} kit pieces on {count['stretches']} stretches; {count['gaps']} gaps.",
              "Each stretch: end or corner caps, bays, fillers, joints spread (kit.py rules 1 to 6).",
              data=dict(count, max_step=round(worst, 3)), explain=EXPLAIN["kit"])
    if count["steep"]:
        step.warning("steep", "step." + step.id, f"{count['steep']} kit pieces steeper than their plinth.",
                     "Add a shorter filler to the kit, give the pieces a deeper `sink`, or move the run.",
                     data={"count": count["steep"]}, explain=EXPLAIN["steep"])
