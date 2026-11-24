"""The building library's row step: buildings side by side along a frontage, written in bays.

A generate step, run once as a worker through the SDK. It reads every `row` feature and returns
one `building` feature for each of its lots, which `lay` then walls: `lay` runs over
`kind:building`, so the world declares it in a layer above this step (a step reads another
step's output only from above it, protocol.md 3.3). Design note: README.md, "A row of buildings".

Why (K6 of docs/plan-kits.md, from examples/block). A building's `region` is the outer face of
its walls, and it closes on its kit only when every side is whole bays and two corners: 3 m a bay
and 0.6 m for the posts on the plain and the city kit. Twelve buildings of one block written by
hand were 192 numbers, each a sum of those worked out by the author, and what the build could not
know from a region it did not say: 312 windows and shopfronts stood against a neighbour's wall.
A row is written in the kit's own unit, and knows what stands beside what.

Rules (ours):

1. **A row.** The frontage runs from the row's own point, `at`, to `to` (two numbers, metres):
   the buildings stand on the left of the way there, their fronts to its right, so a block
   walked counter-clockwise has its buildings inside it. The frontage is called the path below.
   A row is a point and not a path feature: a path with a kind is held to own every meeting
   with another (geometry.md 2.7), and every doorway of the row ends on this line. `kit` is the
   kit every building of the row is laid from. `depth` is how many bays deep they are.
   `lots = [{ bays, storeys, door }, ...]` are the buildings in order along the path: `bays`
   wide, `storeys` high, the door in the `door`th bay of the front (0 the first; default 1, or 0
   on a lot one bay wide).
2. **Sizes are the kit's.** A side of `n` bays is `n` times the kit's module and the two corners'
   reach: the kit's stated `module` (else its bay's pitch) and its outside corner's
   `params.reach` (else its stated `wall`, else the bay's thickness). So a lot is `bays * module
   + 2 * reach` along the path and the row's depth the same of `depth`; lot `i` starts where lot
   `i - 1` ends, wall to wall. Its outline is counter-clockwise from the path: side 0 the front,
   side 1 the end towards the path's end, side 2 the back, side 3 the end towards its start.
3. **The row against its frontage.** The lots' lengths are summed against the path's. Within 5
   mm the row closes. Else `row_open` (warning): how far the lots stop short of the path's end
   or run past it, in metres and, where that is whole bays, in bays. Every lot is still
   returned: nothing is stretched or dropped to hide the remainder (R67).
4. **What a side faces.** Each building gets `faces`, a word for each side: side 0 is `front`
   and side 2 `back`. An end side is `party` where a side of another building returned by this
   step, of this row or another, lies along it for a metre or more, else `end`: the open end of
   a row, which at a block's corner is a second front. A back is `party` too where other
   buildings' sides lie along more than half of it (a corner building whose back is the end of
   the row round the corner). The row's `fill` is each building's, so
   `fill = [{ faces = ["party"], pattern = ["blank"] }]` blanks every wall a neighbour stands
   against, and no window is written by hand (lay.py rule 12).
5. **Heights along a street.** A lot that writes no `storeys` takes one of the row's `heights`,
   a list of whole numbers: the one a hash of the row's name and the lot's number picks, or the
   next in the list that differs where that is the number of storeys of the lot before it. So
   neighbours differ wherever the list holds two different numbers, the choice is the same in every build, and a lot
   keeps its height when lots are added after it. With no `heights` it takes the row's
   `storeys`. Source: the rule that neighbours may not repeat (docs/research/Modular kit
   failures in studios.md, section 8).
6. **Door and stair.** The door stands in bay `door` of the front: its middle is the corner's
   reach, `door` modules and half the door piece's pitch along side 0, so the rest of that bay is
   the kit's filler. A building of two storeys or more gets the kit's stair (lay.py rule 11)
   along its back, starting one landing from the corner; where the back is too short for the
   stairwell (a landing, the run and a landing past its head, between the two walls) it goes
   along side 1, then side 3. A lot none of them holds is `row_stair` (warning)
   and is not returned: `lay` would refuse it.
7. **Two rows in one place.** A lot whose outline overlaps a lot of a row named earlier (by name)
   is `row_meets` (warning), naming every lot it stands in and, for a row's first or last lot,
   the `at` or `to` that clears them, and is not returned: two rows that both run to a block's
   corner would stand in each other there.
8. **What is passed on.** `storey_h`, `ground_h`, `top_h`, `plinth`, `porch`, `walk_out`, `roof`,
   `fill`, `setback`, `overhang`, and `ground`, `max_cut` and `grade` (a level plot under each
   building, lay.py rule 19; from T1) of the row are each building's (a `setback` or `overhang`
   entry for a storey a lot has not is not passed to it). Of a `fill` entry's `storeys` a building keeps
   those it has, and an entry none of whose storeys it has is not passed to it: a row's lots
   differ in height, and one line may fill the fifth storey of those that have one. A building is named `<row>_<i>`, `i` its lot's
   number from 0, so a lot keeps its name whatever is written after it.

9. **Off the axes.** A row may run at any angle: its lots are laid from exact lengths along the
   path, what stands beside what (rules 4 and 7) is told from those and not from the returned
   rings, and each returned ring is rounded to the millimetre. What `lay` then does with a
   turned building is `lay`'s (lay.py rules 6 and 9: no floor tile, and its walls meet by
   rounding).
10. **A row along a line** (the fix plan's T1). `along = "<name>"` in place of `to`: the frontage
   is the named line feature, from its first point to its last (from its last to its first with
   `reverse = true`, for the street's other side: a `follows` line runs the way its street does),
   and the row's own point is not read. The line is typically one with no kind that `follows` a street at a setback
   (geometry.md 4), so the core moves it when the street moves, and the row with it. The line
   must be straight: a point of it more than 10 mm off the line from its first to its last is
   `row_bent` (warning), and the row is not laid: a street bent under its rows is not the rows'
   fault. `along` with `to`, a name that is no feature, or a feature of one point, is `params`.
11. **A row that fills its frontage** (T1). `widths = [3, 4, 5]` (bays, whole numbers) in place
   of `lots`: lots are laid from the start, each as wide as a hash of the row's name and the
   lot's number picks from the list (as rule 5 picks heights; never its neighbour's width where
   the list holds two), or, where that one runs past the end, the widest of the list that does
   not and differs from its neighbour's (else the widest that fits), until none fits. The door is rule 1's default. What is left at the end, less than the
   narrowest lot, is not `row_open`: the author asked for the frontage filled, not closed, and a
   frontage is seldom whole bays. A line shorter than the narrowest lot is `row_short`
   (warning), and the row is not laid. What is left is in the `rows` info's data (`left_rows`, the rows by name,
   and `left`, metres, in that order). So a
   longer frontage gets more lots, and the lots before keep their names and widths.

Codes are `step.<id>.<code>`: `params` (error: a row's attribute, or a kit with no bay; said at
that row, and the other rows are laid), `row_open`, `row_stair`, `row_meets`, `row_bent` and
`row_short` (warnings: a row's line that bends or holds no lot is not laid, the others are; it is
not the row's text but a street moved under it), `rows` (info, what was returned).

Library code imports only viasdk and the standard library (R19b).
"""

import hashlib
import math

import viasdk

FIT = 5              # mm a row may differ from its frontage and still close
STRAIGHT = 10        # mm a point of an `along` line may stand off its chord (rule 10)
SHARE = 1000         # mm of side two buildings share for it to be a party wall
PASSED = ("storey_h", "ground_h", "top_h", "plinth", "porch", "walk_out", "roof", "fill", "setback", "overhang",
          "ground", "max_cut", "grade")
ATTRS = ["to", "along", "reverse", "kit", "depth", "lots", "widths", "heights", "storeys", *PASSED]

EXPLAIN = {
    "params": "A row's attribute, or its kit, cannot be laid from. The message names it.",
    "row_open": "A row's lots are each a whole number of the kit's bays and two corners, laid wall to wall "
    "from the path's start. Where their sum is not the path's length the row stops short of the frontage or "
    "runs past it. Change a lot's `bays`, or the path.",
    "row_stair": "A building of two storeys or more needs the kit's stair along one of its sides, and none of "
    "this lot's sides is long enough for the stairwell. Give the lot more bays, the row more depth, or one storey.",
    "row_meets": "A lot of this row stands where a lot of another row does. Two rows that meet at a corner cannot "
    "both run to it: start or end one of them clear of the other's depth.",
    "row_bent": "A row runs along a line (rule 10), and that line is no longer straight: a street it follows "
    "was bent. The row is not laid; the others are. Lay one row along each straight part.",
    "row_short": "A row fills its line from `widths` (rule 11), and its line is now shorter than the narrowest "
    "lot. The row is not laid; the others are.",
    "rows": "What the step returned: the rows read and the buildings written.",
}


class ParamError(Exception):
    def __init__(self, key, message, hint):
        super().__init__(message)
        self.key, self.message, self.hint, self.row = key, message, hint, None


class Unlaid(Exception):
    """A row its frontage cannot hold as it now lies (rules 10, 11): a warning, the row not laid,
    the others laid. Not the row's text: a street moved under it."""
    def __init__(self, code, message, hint, data, at):
        super().__init__(message)
        self.code, self.message, self.hint, self.data, self.at = code, message, hint, data, at


def kit_facts(rows, taxon):
    """{module, reach, wall, door, stair: {lane, landing, run}} in mm, from a kit's pack rows
    (rule 2): what it states (`shared`) before what its pieces measure."""
    def role(r, name):
        return f"role:{name}" in (r.get("tags") or [])

    def plain(r):
        return not any(t.startswith(("variant:", "storey:")) for t in r.get("tags") or [])
    bays = sorted((r for r in rows if role(r, "wall") and r.get("bounds")), key=lambda r: (not plain(r), r["id"]))
    if not bays:
        raise ParamError("kit", f"the kit `{taxon}` has no `role:wall` piece", "name a kit with a bay")
    shared = next((r["shared"] for r in rows if r.get("shared")), {})
    if not isinstance(shared, dict):
        shared = {}

    def mm(v, key="a length"):
        if isinstance(v, bool) or not isinstance(v, (int, float)) or not math.isfinite(v) or v <= 0:
            raise ParamError("kit", f"the kit `{taxon}` states `{key} = {v!r}`, not a length above 0",
                             f"write `{key}` as metres in the kit's `defaults.shared`, or on the piece")
        return int(round(float(v) * 1000))

    def pitch(r):
        v = (r.get("params") or {}).get("pitch")
        return mm(v, "params.pitch") if v is not None else r["bounds"][3] - r["bounds"][0]
    bay = bays[0]
    wall = mm(shared["wall"], "shared.wall") if "wall" in shared else bay["bounds"][4] - bay["bounds"][1]
    module = mm(shared["module"], "shared.module") if "module" in shared else pitch(bay)
    corner = next((r for r in sorted(rows, key=lambda r: (not plain(r), r["id"])) if role(r, "corner")), None)
    reach = (corner.get("params") or {}).get("reach") if corner else None
    door = next((r for r in sorted(rows, key=lambda r: (not plain(r), r["id"])) if role(r, "door") and r.get("bounds")),
                None)
    st = shared.get("stair") or {}
    if not isinstance(st, dict):
        raise ParamError("kit", f"the kit `{taxon}` states `shared.stair = {st!r}`",
                         "write `stair = { lane, width, run, landing }`, each in metres")
    stair = {k: mm(v, f"shared.stair.{k}") for k, v in st.items()}
    return {"module": module, "wall": wall, "taxon": taxon,
            "reach": mm(reach, "params.reach") if reach is not None else wall,
            "door": pitch(door) if door else module // 2, "stair": stair}


def _whole(v, lo=1):
    """Whether `v` is a whole number from `lo`: an attribute read through the engine comes back a
    float (`4.0`), so one with no fraction counts."""
    return isinstance(v, (int, float)) and not isinstance(v, bool) and math.isfinite(v) and v == int(v) and v >= lo


def pick(name, i, choices, before):
    """Rule 5's choice: the entry of `choices` a hash of the row's name and the lot's number picks,
    or the next one that differs from `before`, where one does."""
    k = int.from_bytes(hashlib.sha256(f"{name}/{i}".encode()).digest()[:8], "big") % len(choices)
    n = int(choices[k])
    if n == before:
        n = next((int(choices[(k + d) % len(choices)]) for d in range(1, len(choices))
                  if choices[(k + d) % len(choices)] != before), n)
    return n


def widths_of(name, attrs, length, m, r):
    """Rule 11: the lots that fill a frontage `length` mm long, [{bays, storeys, door}], and what is
    left at its end, mm."""
    widths = attrs.get("widths")
    if not isinstance(widths, list) or not widths or not all(_whole(w) for w in widths):
        raise ParamError("widths", f"`widths` of `{name}` is not a list of whole numbers from 1",
                         "write the bays a lot may be wide: `widths = [3, 4, 5]`")
    out, at = [], 0
    while True:
        w = pick(name, len(out), widths, out[-1]["bays"] if out else None)
        if at + w * m + 2 * r > length + FIT:
            fit = [int(v) for v in widths if at + int(v) * m + 2 * r <= length + FIT]
            w = max((v for v in fit if not out or v != out[-1]["bays"]), default=max(fit, default=None))
        if w is None:
            break
        out.append({"bays": w, "storeys": None, "door": 1 if w > 1 else 0})
        at += w * m + 2 * r
    return out, length - at


def lots_of(name, attrs):
    """The row's lots, checked (rule 1): [{bays, storeys or None, door}]."""
    lots = attrs.get("lots")
    if not lots and attrs.get("widths"):
        lots = [{"bays": 1}]                        # rule 11 lays them; this checks the rest
    if not isinstance(lots, list) or not lots:
        raise ParamError("lots", f"`{name}` has no `lots`",
                         "write `lots = [{ bays = 4 }, { bays = 3, storeys = 2 }]`: the buildings along the path")
    if not _whole(attrs.get("depth", 0)):
        raise ParamError("depth", f"`depth` of `{name}` is {attrs.get('depth')!r}", "write how many bays deep its "
                         "buildings are, a whole number from 1")
    heights = attrs.get("heights") or []
    if not isinstance(heights, list) or not all(_whole(h) for h in heights):
        raise ParamError("heights", f"`heights` of `{name}` is not a list of whole numbers",
                         "write the storeys a lot may have: `heights = [3, 4, 5]`, or leave it out")
    out = []
    for i, lot in enumerate(lots):
        ok = isinstance(lot, dict) and _whole(lot.get("bays")) and (lot.get("storeys") is None or _whole(lot["storeys"]))
        extra = sorted(set(lot) - {"bays", "storeys", "door"}) if isinstance(lot, dict) else []
        door = lot.get("door", 1 if ok and lot["bays"] > 1 else 0) if isinstance(lot, dict) else 0
        if not ok or extra or not _whole(door, 0) or door >= lot["bays"]:
            raise ParamError("lots", f"lot {i} of `{name}` is not `{{ bays, storeys, door }}`"
                             + (f": it has `{extra[0]}`" if extra else ""),
                             "write `bays` as a whole number from 1, `storeys` the same or left out, and `door` "
                             "as the bay of the front the door stands in, counted from 0")
        out.append({"bays": int(lot["bays"]), "storeys": None if lot.get("storeys") is None else int(lot["storeys"]),
                    "door": int(door)})
    return out


def heights_of(name, lots, heights, default):
    """Each lot's storeys (rule 5)."""
    out = []
    for i, lot in enumerate(lots):
        n = lot["storeys"]
        if n is None and heights:
            n = pick(name, i, heights, out[-1] if out else None)
        out.append(int(n if n is not None else default))
    return out


def fill_for(fill, storeys):
    """A row's `fill` as one building's (rule 8): an entry's `storeys` keeps those the building
    has, and an entry none of whose storeys it has is left out. A row's lots differ in height,
    and `lay` refuses a storey a building has not."""
    out = []
    for f in fill:
        if isinstance(f, dict) and isinstance(f.get("storeys"), list):
            mine = [k for k in f["storeys"] if isinstance(k, (int, float)) and k < storeys]
            if not mine:
                continue
            f = dict(f, storeys=mine)
        out.append(f)
    return out


def stair_of(sides_mm, storeys, facts):
    """The stair of a building of `storeys` (rule 6): {side, at} or None for one storey; a
    ParamError-free refusal is `False`. `sides_mm` are the lengths of sides 0 to 3."""
    if storeys < 2:
        return None
    st = facts["stair"]
    landing, run = st.get("landing", 1000), st.get("run")
    if not run:
        raise ParamError("kit", f"the kit `{facts['taxon']}` states no stair (`shared.stair.run`), and a row writes none",
                         "state the kit's stair in its taxon's `defaults.shared`: `stair = { lane, width, run, landing }`")
    # A landing before the flight's foot and one past its head, whatever the storeys: from three
    # storeys the second flight's stairwell runs a landing past the first's, and with two a
    # flight that ends on the far wall leaves `lay` no wall to close there (TODO, lib/building).
    need = facts["wall"] + landing + run + landing + facts["wall"]
    for side in (2, 1, 3):
        if sides_mm[side] + FIT >= need:
            return {"side": side, "at": (facts["wall"] + landing) / 1000}
    return False


def overlap(a, b):
    """Whether two rings' insides meet. Both are rectangles here; tested by separating axes."""
    def axes(r):
        out = []
        for p, q in zip(r, r[1:] + r[:1]):
            L = math.hypot(q[0] - p[0], q[1] - p[1])
            out.append(((q[1] - p[1]) / L, -(q[0] - p[0]) / L))
        return out
    for ax in axes(a) + axes(b):
        pa = [p[0] * ax[0] + p[1] * ax[1] for p in a]
        pb = [p[0] * ax[0] + p[1] * ax[1] for p in b]
        if max(pa) <= min(pb) + 1 or max(pb) <= min(pa) + 1:      # 1 mm: touching is not overlapping
            return False
    return True


def shared_side(a, k, b):
    """mm of side `k` of ring `a` that a side of ring `b` lies along."""
    p, q = a[k], a[(k + 1) % len(a)]
    L = math.hypot(q[0] - p[0], q[1] - p[1])
    e = ((q[0] - p[0]) / L, (q[1] - p[1]) / L)
    best = 0.0
    for u, v in zip(b, b[1:] + b[:1]):
        M = math.hypot(v[0] - u[0], v[1] - u[1])
        f = ((v[0] - u[0]) / M, (v[1] - u[1]) / M)
        if abs(e[0] * f[1] - e[1] * f[0]) > 1e-4 or abs((u[0] - p[0]) * e[1] - (u[1] - p[1]) * e[0]) > 2:
            continue
        s0 = (u[0] - p[0]) * e[0] + (u[1] - p[1]) * e[1]
        s1 = (v[0] - p[0]) * e[0] + (v[1] - p[1]) * e[1]
        best = max(best, min(L, max(s0, s1)) - max(0.0, min(s0, s1)))
    return best


def plan_row(name, path, attrs, facts):
    """One row's buildings and notes (rules 1 to 3, 5, 6), before rows are held against each
    other: ([{key, ring (mm), attrs, lot}], [note]). `attrs["along"]`, where written, has been
    read as `attrs["_line"]`: the line's points (mm), or None where no feature has the name."""
    to = attrs.get("to")
    if attrs.get("along") not in (None, ""):
        (x0, y0), (x1, y1) = frontage(name, attrs)
    else:
        if len(path) != 1:
            raise ParamError("at", f"`{name}` is not a point", "write where the row starts as `at = [x, y]`")
        if not (isinstance(to, list) and len(to) == 2 and all(
                isinstance(v, (int, float)) and not isinstance(v, bool) and math.isfinite(v) for v in to)):
            raise ParamError("to", f"`to` of `{name}` is {to!r}",
                             "write where the row's frontage ends: `to = [x, y]`, metres, or `along` a line")
        (x0, y0), (x1, y1) = path[0], (int(round(to[0] * 1000)), int(round(to[1] * 1000)))
    path = [(x0, y0), (x1, y1)]
    L = math.hypot(x1 - x0, y1 - y0)
    if L < 1:
        raise ParamError("to", f"`{name}` has no length", "write a `to` that is not the row's own point")
    e, n = ((x1 - x0) / L, (y1 - y0) / L), (-(y1 - y0) / L, (x1 - x0) / L)
    if attrs.get("widths") not in (None, []) and attrs.get("lots") not in (None, []):
        raise ParamError("widths", f"`{name}` writes both `lots` and `widths`",
                         "write `lots` to say each building, or `widths` to fill the frontage (rule 11)")
    lots = lots_of(name, attrs)
    left = None
    if attrs.get("widths") not in (None, []):
        lots, left = widths_of(name, attrs, L, facts["module"], facts["reach"])
        if not lots:
            raise Unlaid("row_short", f"no lot of `{name}`'s `widths` fits its frontage, {L / 1000:g} m: "
                         "the row is not laid", "write narrower widths, or a longer frontage",
                         {"frontage": round(L / 1000, 3)}, (x0, y0))
    default = attrs.get("storeys", 1)
    if not _whole(default):
        raise ParamError("storeys", f"`storeys` of `{name}` is {default!r}", "write a whole number from 1")
    storeys = heights_of(name, lots, attrs.get("heights") or [], default)
    m, r = facts["module"], facts["reach"]
    deep = int(attrs["depth"]) * m + 2 * r
    out, notes, at = [], [], 0
    for i, (lot, high) in enumerate(zip(lots, storeys)):
        wide = lot["bays"] * m + 2 * r

        def pt(u, v):
            return (x0 + e[0] * u + n[0] * v, y0 + e[1] * u + n[1] * v)
        # Two rings: `exact`, which rules 4 and 7 test (a ring rounded to the millimetre is no
        # longer parallel to its neighbour's off the axes), and the one returned, on the lattice.
        exact = [pt(at, 0), pt(at + wide, 0), pt(at + wide, deep), pt(at, deep)]
        ring = [(int(round(x)), int(round(y))) for x, y in exact]
        stair = stair_of([wide, deep, wide, deep], high, facts)
        a = {"storeys": high, "kit": attrs["kit"],
             "doors": [{"storey": 0, "side": 0, "at": (r + lot["door"] * m + facts["door"] / 2) / 1000}]}
        for key in PASSED:
            if attrs.get(key) not in (None, "", []):
                a[key] = attrs[key]
        if a.get("fill"):
            a["fill"] = fill_for(a["fill"], high)
        for key in ("setback", "overhang"):      # a storey a lot has not is not passed to it
            if isinstance(a.get(key), list):
                a[key] = [e for e in a[key] if not (isinstance(e, dict) and isinstance(e.get("storey"), (int, float))
                                                    and e["storey"] >= high)]
                if not a[key]:
                    del a[key]
        if stair:
            a["stair"] = stair
        b = {"key": f"{name}_{i}", "ring": ring, "exact": exact, "attrs": a, "lot": i, "lots": len(lots),
             "row": name, "from": at, "to": at + wide}
        if stair is False:
            notes.append({"code": "row_stair", "row": name, "lot": i, "bays": lot["bays"], "storeys": high,
                          "at": ring[0]})
        else:
            out.append(b)
        at += wide
    if left is not None:
        notes.append({"code": "row_left", "row": name, "left": left})
    elif abs(at - L) > FIT:
        notes.append({"code": "row_open", "row": name, "lots": at, "path": L, "module": m, "at": path[1]})
    return out, notes


def frontage(name, attrs):
    """Rule 10: the first and last point of the line `along` names, held straight (mm)."""
    if attrs.get("to") not in (None, []):
        raise ParamError("along", f"`{name}` writes both `to` and `along`",
                         "write `to` for a frontage of two points, or `along` for a line's")
    line, pts = attrs.get("along"), attrs.get("_line")
    if not isinstance(line, str):
        raise ParamError("along", f"`along` of `{name}` is {line!r}", "write the name of a line feature")
    if pts is None:
        raise ParamError("along", f"`{name}` runs along `{line}`, and no feature is named so",
                         "write the name of a line: one that `follows` the street, for one")
    if len(pts) < 2:
        raise ParamError("along", f"`{line}`, which `{name}` runs along, is a point",
                         "name a line of two points or more")
    (x0, y0), (x1, y1) = pts[0], pts[-1]
    L = math.hypot(x1 - x0, y1 - y0)
    if L >= 1:
        off = max(abs((x1 - x0) * (y - y0) - (y1 - y0) * (x - x0)) / L for x, y in pts)
        if off > STRAIGHT:
            raise Unlaid("row_bent", f"`{line}`, which `{name}` runs along, bends: a point of it stands "
                         f"{off / 1000:g} m off the line from its start to its end; the row is not laid",
                         "lay one row along each straight part: a `follows` line's `span` can be one part",
                         {"line": line, "off": round(off / 1000, 3)}, pts[0])
    if attrs.get("reverse") not in (None, True, False):
        raise ParamError("reverse", f"`reverse` of `{name}` is {attrs.get('reverse')!r}", "write `reverse = true`, or leave it out")
    return (pts[-1], pts[0]) if attrs.get("reverse") else (pts[0], pts[-1])


def plan(rows, kits):
    """Every row's buildings (rules 4 and 7): `rows` is [(name, [its point, mm], attrs)], `kits` a
    function of a taxon that gives its `kit_facts`. Returns (buildings, notes)."""
    buildings, notes = [], []
    for name, path, attrs in sorted(rows, key=lambda r: r[0]):
        try:
            if not attrs.get("kit"):
                raise ParamError("kit", f"`{name}` names no `kit`",
                                 "write the taxon of the kit its buildings are laid from")
            got, said = plan_row(name, path, attrs, kits(attrs["kit"]))
        except ParamError as e:
            # One row's fault is said at that row, and the other rows are laid (T1's review: one
            # row's fault had stopped them all).
            notes.append({"code": "params", "row": name, "key": e.key, "message": e.message, "hint": e.hint})
            continue
        except Unlaid as u:
            notes.append({"code": u.code, "row": name, "message": u.message, "hint": u.hint, "data": u.data,
                          "at": u.at})
            continue
        notes += said
        for b in got:
            hits = [o["key"] for o in buildings if o["row"] != name and overlap(b["exact"], o["exact"])]
            if hits:
                # Where the row would clear them: past this lot when it is the row's first, before
                # it when it is the last (`ring[1]` and `ring[0]` are the lot's ends on the path).
                clear = ({"key": "at", "point": b["ring"][1]} if b["lot"] == 0
                         else {"key": "to", "point": b["ring"][0]} if b["lot"] == b["lots"] - 1 else None)
                notes.append({"code": "row_meets", "row": name, "lot": b["lot"], "others": sorted(hits),
                              "from": b["from"], "to": b["to"], "at": b["ring"][0], "clear": clear})
            else:
                buildings.append(b)
    for b in buildings:
        faces = ["front", "end", "back", "end"]
        for k in (1, 2, 3):
            along = sum(shared_side(b["exact"], k, o["exact"]) for o in buildings if o is not b)
            p, q = b["exact"][k], b["exact"][(k + 1) % 4]
            if along >= (SHARE if k != 2 else math.hypot(q[0] - p[0], q[1] - p[1]) / 2 + 1):
                faces[k] = "party"
        b["attrs"]["faces"] = faces
    return buildings, notes


# --- the step ---------------------------------------------------------------------------------


def _list(col):
    return col.tolist() if hasattr(col, "tolist") else list(col)


def read_rows(step):
    try:
        fl = step.features(select="kind:row", columns=ATTRS)
    except Exception:                             # no row in the world
        return []
    out = []
    for f, name in enumerate(_list(fl.get("name", []))):
        pts = [(int(v[0]), int(v[1])) for v in _list(fl.rings(f)[0][0])]
        attrs = viasdk.attrs(fl, f, ATTRS)
        out.append((name, pts, attrs))
    # Rule 10: the lines rows run along, read by name (a name read is in the step's trace, so a
    # moved line re-runs it).
    names = sorted({a["along"] for _, _, a in out if isinstance(a.get("along"), str) and a["along"]})
    lines = {}
    for name in names:
        # One read a name: a name that is no feature is that row's fault alone (T1's review: read
        # together, one missing name lost every row its line).
        try:
            ll = step.features(select=[name])
        except Exception:
            continue
        for f, n in enumerate(_list(ll.get("name", []))):
            if n == name:
                lines[n] = [(int(v[0]), int(v[1])) for v in _list(ll.points(f))]
    for _, _, a in out:
        if isinstance(a.get("along"), str) and a["along"]:
            a["_line"] = lines.get(a["along"])
    return out


def read_kit(step, taxon):
    """The kit's pieces (viasdk.kit)."""
    return viasdk.kit(step, taxon)


def generate(step):
    rows = read_rows(step)
    facts = {}

    def kits(taxon):
        if taxon not in facts:
            facts[taxon] = kit_facts(read_kit(step, taxon), taxon)
        return facts[taxon]
    try:
        buildings, notes = plan(rows, kits)
    except ParamError as e:
        step.error("params", e.row or "step." + step.id, e.message, e.hint, explain=EXPLAIN["params"],
                   data={"attribute": e.key})
        return
    o = step.out
    for b in buildings:
        o.feature(b["key"], "building", region=b["ring"], **b["attrs"])       # the SDK takes millimetres
    said = set()
    x0_, y0_, x1_, y1_ = (step.grid or {}).get("extent_mm") or (-math.inf, -math.inf, math.inf, math.inf)

    def placed(kw, at):
        """`kw` with the note's position, where the world's extent holds it: a row may be written
        past the extent, and a diagnostic may not point there."""
        if x0_ <= at[0] <= x1_ and y0_ <= at[1] <= y1_:
            kw["position"] = (at[0] / 1000, at[1] / 1000)
        return kw
    left = {n["row"]: n["left"] / 1000 for n in notes if n["code"] == "row_left"}
    notes = [n for n in notes if n["code"] != "row_left"]
    for n in notes:
        first = n["code"] not in said
        said.add(n["code"])
        if n["code"] == "row_open":
            over = (n["lots"] - n["path"]) / 1000
            bays = abs(n["lots"] - n["path"]) / n["module"]
            whole = abs(bays - round(bays)) * n["module"] <= FIT and round(bays) >= 1
            how = (f"run {over:g} m past the end of its path" if over > 0
                   else f"stop {-over:g} m short of the end of its path")
            it = "it" if round(bays) == 1 else "them"
            step.warning("row_open", n["row"],
                         f"the lots of `{n['row']}` take {n['lots'] / 1000:g} m and {how} ({n['path'] / 1000:g} m)",
                         (f"that is {round(bays)} bay{'s' if round(bays) != 1 else ''}: "
                          + (f"take {it} from a lot" if over > 0 else f"add {it} to a lot")
                          if whole else f"end the path {n['lots'] / 1000:g} m from its start, or change a lot's "
                          "`bays` and the path together: the difference is no whole number of bays"),
                         **placed({"data": {"lots": n["lots"] / 1000, "path": n["path"] / 1000,
                                            "bays": round(bays, 3)}}, n["at"]),
                         **({"explain": EXPLAIN["row_open"]} if first else {}))
        elif n["code"] == "params":
            step.error("params", n["row"], n["message"], n["hint"], explain=EXPLAIN["params"],
                       data={"attribute": n["key"]})
        elif n["code"] in ("row_bent", "row_short"):
            step.warning(n["code"], n["row"], n["message"], n["hint"], **placed({"data": n["data"]}, n["at"]),
                         **({"explain": EXPLAIN[n["code"]]} if first else {}))
        elif n["code"] == "row_stair":
            step.warning("row_stair", n["row"],
                         f"lot {n['lot']} of `{n['row']}`, {n['bays']} bays wide and {n['storeys']} storeys high, has no "
                         "side long enough for the kit's stair: it is not returned",
                         "give it more bays, the row more depth, or one storey",
                         **placed({"data": {"lot": n["lot"], "bays": n["bays"], "storeys": n["storeys"]}}, n["at"]),
                         **({"explain": EXPLAIN["row_stair"]} if first else {}))
        else:
            c = n.get("clear")
            others = " and ".join(f"`{o}`" for o in n["others"])
            step.warning("row_meets", n["row"],
                         f"lot {n['lot']} of `{n['row']}`, from {n['from'] / 1000:g} to {n['to'] / 1000:g} m along its "
                         f"path, stands where {others} {'does' if len(n['others']) == 1 else 'do'}: it is not returned",
                         (f"take the lot out and write `{c['key']} = [{c['point'][0] / 1000:g}, {c['point'][1] / 1000:g}]`: "
                          "the row then clears the other" if c else "take the lot out, or move the row"),
                         **placed({"data": {"lot": n["lot"], "others": n["others"], "from": n["from"] / 1000,
                                            "to": n["to"] / 1000}}, n["at"]),
                         **({"explain": EXPLAIN["row_meets"]} if first else {}))
    step.info("rows", "step." + step.id, f"{len(rows)} rows, {len(buildings)} buildings", "none needed",
              data={"rows": len(rows), "buildings": len(buildings),
                    **({"left_rows": sorted(left), "left": [left[k] for k in sorted(left)]} if left else {})},
              explain=EXPLAIN["rows"])
