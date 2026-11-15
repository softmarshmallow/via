"""The town library's houses step: one house on each lot, its front to the street.

A generate step, run as a worker through the SDK, declared by the world in a layer above its
`[cells]` table (a step reads a maker's output only from above it, geometry.md 8.3). It reads the
lots through `build.features` (`select` naming the cells table: its cells and rest pieces, with a
`class` column, protocol.md 3.3) and the streets the same way, and places at most one house per lot.
It cuts no lot itself (R19b, R23).

How it places (ours, written for this library):

1. **The frontage side.** Of a lot's outline edges, the longest run of edges on a street
   (`streets`, default every `main_street` and `street`) is its frontage, and its chord stands for
   it; with no edge on a street, the edge whose middle lies nearest one. A lot is cut along its
   block's frontage (geometry.md 3.6), so that side lies on the street's side of the block.
2. **A row, not a house.** A town's houses front their street side by side (a terrace), so the
   step fills the frontage with a row of `assets` standing wall to wall: of every row of at most
   `MAX_ROW` houses whose widths (and `PARTY`, 1 cm, between each two: touching walls that a
   millimetre's rounding cannot make overlap, rules.md 6) sum to at most the frontage's length
   less `gap` at each end, the
   one that covers the most of it (ties: fewer houses, then the assets in the order written). The
   row is centred on the frontage, so the gaps fall at the lot's edges, between one lot's row and
   the next; where it does not stand inside the lot centred, it is tried against the frontage's
   start, then its end (a corner lot cut as a triangle has its square corner at one end: quay had
   49 such lots empty). A row that stands inside the lot no way is passed over for the next; a lot no house fits is left empty, and
   counted (`small`, info). quay (M3 slice 2) placed one detached house per lot, so a street read
   as houses in fields; with `assets` holding one asset this is that rule with the row's count.
3. **Where and which way.** Every house of the row faces the street: `face` is the bearing of the
   frontage chord's outward normal, clockwise from north, in whole millidegrees. Their fronts (the
   asset frame's `+y` bound, assets.md) stand on one line `setback` metres in from the chord.
4. **Clear of the street.** A lot's frontage is the street's centre line, and the street keeps
   houses off its carriageway and footway with its `keep_out` (lib/town/kinds.toml, `verge`):
   a house standing where `keep_out` is above 0 is `place.keep_out` (rules.md 8.3). So the row's
   front line is pushed back from `setback` in steps of `SETBACK_STEP` up to `setback +
   MAX_PUSH`: the land's `keep_out` (every `role:keep_out` field, the largest) is read round each
   house's outline every metre, every lot and every step in one `expr.eval` batch (R23), and the
   least push that keeps the most of the row clear is taken; a house still not clear there is
   dropped (a corner lot's end house beside a cross street: the gap falls at the junction). So the
   setback follows each street's own width, and a house on a bend stands clear of it, with no
   width written here (TODO: "setback by frontage width (keep_out)", the S-bend house in quay
   `move_hill`). Without a `keep_out` field the row stands at `setback`.
5. **Ground.** With `terrace` (a ground kind of regions, lib/town's `terrace`), each run of houses
   standing wall to wall (a row, or the part of it left when a house was dropped) stands on one
   region of that kind the step returns, named `t<digest>_<station>` (with `_<i>` when the run
   starts at the row's `i`th house), so the row has one floor; the houses take no `ground`.
   With `ground` instead (say `"pad"`, lib/land), each house is lent that ground kind (rules.md
   2.1) and levelled at its own mean: on a fall, two neighbours each bank the other's footprint
   (quay: 50 `place.relief` between neighbours), so `ground` suits detached houses.
6. **Relief.** Without `ground`, and with `max_relief` (metres), the land (`role:surface`) is read
   at the footprint's four corners and its middle, every house in one `expr.eval` batch (R23); a
   house whose highest and lowest reading differ by more than `max_relief` is not placed, and
   counted (`steep`, info).
7. **What a pad can level.** With `ground` and `max_cut` (metres), the house is placed only where
   its pad stays within the kind's `max_change`: the pad holds the land at the footprint's mean,
   less or more `grade` (rise over run) per metre out, fading out between `reach` 10 and 25 m
   (lib/land `pad`), so at a reading `d` metres off the footprint it moves the land by
   `(1 - smoothstep(10, 25, d)) * max(0, |h - mean| - grade * d)`. The land is read at the
   footprint's corners and middle, and on a 2 m lattice round it out to 24 m, in one batch; a
   house whose pad would move a reading by more than `max_cut` is not placed (`steep`, info); with
   `terrace`, the same is read round each run with the terrace's fade (4 to 8 m) and a run past
   `max_cut` is left out whole: an
   edit beside a cut would otherwise stop the build (`authority.max_change`). `grade` (default
   0.3) and `max_cut` are the ground kind's own defaults, which a step cannot read; write them as
   the kind has them. This is a prediction, not the core's check. The core now gives way for a
   returned instance whose own `ground` pad passes `max_change` (`authority.gave_way`, warning,
   authority.md 2.2), so for houses grounded one by one the prediction can go, with `max_cut`.
   A terrace is a returned feature, which stays `authority.max_change` (error), so the prediction
   stays for terraces (TODO, lib/town rule 7).

Names are by place (identity.md 2.2): a lot `lots#c<digest>_<station>` gives its row's first house
(from the frontage chord's start) `h<digest>_<station>` and the others `h<digest>_<station>_<i>`,
`i` the place in the row; so a house keeps its name exactly when its lot and its place in the row
do (identity.md 2.3), and a hand edit at a lot (`clear` with `{ of = "lots", class = "cell",
near }`) reaches them.

`params`: `lots` (required, the cells table's id); `streets` (selectors, default
`["kind:main_street", "kind:street"]`); `assets` (default `["house_narrow", "house_low"]`);
`setback` (1 m: the least, rule 4 pushes it back); `gap` (0.5 m, at each end of a lot's row); `ground` (optional, a ground kind lent to each house); `terrace` (optional, a region kind under
each run; not with `ground`); `max_relief` (optional,
metres, 0 or more); `max_cut` and `grade` (optional, with `ground` or `terrace`: that kind's).

Library code imports only viasdk and the standard library (R19b).
"""

import math

from viasdk import RpcError

PARAMS_EXPLAIN = ("A param of the houses step is outside what it takes: `lots` names one cells "
                  "table; `streets` and `assets` are lists of names; `setback`, `gap` and "
                  "`max_relief` are 0 or more; `ground` names a ground kind.")

EXPLAIN = {
    "rows": "How many houses the houses step placed, in rows along the lots' frontage, and on how "
    "many of the lots; `small` lots fit no house, `steep` houses were left out for the land.",
    "small": "The houses step left some lots empty: no asset in `assets` fits the lot's frontage "
    "less `gap` on each side and its depth less `setback`.",
    "no_street": "The houses step found no street to face; it placed nothing.",
    "steep": "The houses step left some houses out: the land under the house rises more than "
    "`max_relief` across its footprint, and no `ground` levels it; or its `ground` pad would move "
    "the land near it by more than `max_cut`.",
}


class ParamError(ValueError):
    def __init__(self, key, message, hint):
        super().__init__(message)
        self.key, self.message, self.hint = key, message, hint


class Params:
    def __init__(self, raw):
        self.lots = raw.get("lots")
        if not isinstance(self.lots, str) or not self.lots:
            raise ParamError("lots", "`lots` is missing", "name the cells table the lots are")
        self.streets = _names(raw, "streets", ["kind:main_street", "kind:street"])
        self.assets = _names(raw, "assets", ["house_narrow", "house_low"])
        self.setback = _number(raw, "setback", 1.0)
        self.gap = _number(raw, "gap", 0.5)
        self.ground = raw.get("ground")
        if self.ground is not None and not isinstance(self.ground, str):
            raise ParamError("ground", "`ground` is not a name", "name a ground kind, as `pad`")
        self.max_relief = None if raw.get("max_relief") is None else _number(raw, "max_relief", 0)
        self.max_cut = None if raw.get("max_cut") is None else _number(raw, "max_cut", 0)
        self.terrace = raw.get("terrace")
        if self.terrace is not None and (not isinstance(self.terrace, str) or self.ground):
            raise ParamError("terrace", "`terrace` is not a kind's name, or `ground` is also set",
                             "name one ground kind for a row (`terrace`), or `ground` per house")
        self.grade = _number(raw, "grade", 0.3)


def _names(raw, key, default):
    v = raw.get(key, default)
    if not isinstance(v, list) or not v or not all(isinstance(s, str) for s in v):
        raise ParamError(key, f"`{key}` is not a list of names", "write a list of names")
    return list(v)


def _number(raw, key, default):
    v = raw.get(key, default)
    if isinstance(v, bool) or not isinstance(v, (int, float)) or not math.isfinite(v) or v < 0:
        raise ParamError(key, f"`{key}` is {v!r}", "write 0 or more metres")
    return float(v)


def generate(step):
    try:
        p = Params(step.params)
    except ParamError as e:
        step.error("params", f"step.{step.id}.params.{e.key}", e.message, e.hint,
                   explain=PARAMS_EXPLAIN)
        return
    lots = read_lots(step, p.lots)
    streets = read_streets(step, p.streets)
    if not streets:
        step.warning("no_street", "step." + step.id, "No street to face; no house placed.",
                     "Check `streets` against the world's street kinds.",
                     explain=EXPLAIN["no_street"])
        return
    sizes = asset_sizes(step, p.assets)
    rows = []                                          # (lot, [(offset, [house]) per push])
    for name, ring in lots:
        frame = lot_frame(ring, streets)
        tries = [] if frame is None else [
            (k, fit_row(ring, frame, sizes, p.setback + k * SETBACK_STEP, p.gap))
            for k in range(int(round(MAX_PUSH / SETBACK_STEP)) + 1)]
        rows.append((name, [(k, r) for k, r in tries if r]))
    chosen = clear_rows(step, rows)
    small = [name for name, row in chosen if not row]
    if p.terrace is not None:
        place_terraces(step, p, chosen, small, len(lots))
        return
    placed = [(house_key(name, i), h) for name, row in chosen for i, h in row]
    steep = []
    if p.ground is None and p.max_relief is not None and placed:
        relief = footprint_relief(step, [got[3] for _, got in placed])
        if relief is not None:
            steep = [name for (name, _), r in zip(placed, relief) if r > p.max_relief]
            placed = [pg for pg, r in zip(placed, relief) if r <= p.max_relief]
    if p.ground is not None and p.max_cut is not None and placed:
        moved = pad_moves(step, [got[3] for _, got in placed], p.grade)
        if moved is not None:
            steep = [name for (name, _), m in zip(placed, moved) if m > p.max_cut]
            placed = [pg for pg, m in zip(placed, moved) if m <= p.max_cut]
    for key, (asset, at, face, _) in placed:
        keys = {"ground": p.ground} if p.ground else {}
        step.out.instance(key, asset, at=_mm(at), face=face, **keys)
    held = {key.rsplit("_", 1)[0] if key.count("_") > 1 else key for key, _ in placed}
    say(step, len(placed), len(held), len(lots), small, steep, 0)


def say(step, houses, held, lots, small, steep, terraces):
    step.info("rows", "step." + step.id,
              f"{houses} houses in rows on {held} of {lots} lots.",
              "Each lot's frontage holds a row of houses side by side (rule 2).",
              data={"houses": houses, "lots": lots, "lots_held": held, "small": len(small),
                    "steep": len(steep), "terraces": terraces}, explain=EXPLAIN["rows"])
    if small:
        step.info("small", "step." + step.id, f"{len(small)} lots too small for a house.",
                  "Lower `gap` or `setback`, add a narrower asset, or lengthen the lots' `spacing`.",
                  data={"count": len(small), "lots": small[:20]}, explain=EXPLAIN["small"])
    if steep:
        step.info("steep", "step." + step.id,
                  f"{len(steep)} houses too steep unlevelled, or for their pads.",
                  "Set `ground` to a levelling kind (`pad`), or raise `max_relief` or `max_cut`.",
                  data={"count": len(steep), "houses": steep[:20]}, explain=EXPLAIN["steep"])


MAX_ROW = 6                                           # houses in one lot's row, at most
PARTY = 0.01                                          # metres between two houses of a row: their
                                                      # walls touch, but a millimetre's rounding of
                                                      # a turned origin would make them overlap
SETBACK_STEP = 0.5                                    # metres a row is pushed back per try (rule 4)
MAX_PUSH = 8.0                                        # metres past `setback`, at most
OUTLINE_STEP = 1.0                                    # metres between keep_out readings round a house


def lot_frame(ring, segs):
    """The frontage chord's frame: (middle, inward normal, along, front, right, length, depth,
    face), or None for a chord of no length."""
    a, b = frontage_chord(ring, segs)
    length = math.dist(a, b)
    if length == 0:
        return None
    t = ((b[0] - a[0]) / length, (b[1] - a[1]) / length)
    n = (-t[1], t[0])                                    # inward: the lot's inside is on the left
    deep = max((v[0] - a[0]) * n[0] + (v[1] - a[1]) * n[1] for v in ring)
    mid = ((a[0] + b[0]) / 2, (a[1] + b[1]) / 2)
    front = (-n[0], -n[1])
    right = (front[1], -front[0])                        # clockwise from the front
    face = round(math.degrees(math.atan2(front[0], front[1])) % 360.0, 3)
    return mid, n, t, front, right, length, deep, face


def rows_by_cover(width, sizes, depth):
    """Every row (a tuple of indices into `sizes`, in the order written within the row) of at most
    MAX_ROW houses no deeper than `depth`, whose widths (and PARTY between each two) sum to at most
    `width`: the most covered first, then fewer houses, then earlier assets."""
    fits = [i for i, (_, x0, y0, x1, y1) in enumerate(sizes) if y1 - y0 <= depth]
    out = []

    def walk(start, row, used):
        if row:
            out.append((-round(used, 6), len(row), tuple(row)))
        if len(row) == MAX_ROW:
            return
        for k in range(start, len(fits)):
            i = fits[k]
            w = sizes[i][3] - sizes[i][1]
            if used + w + (PARTY if row else 0.0) <= width + 1e-9:
                walk(k, row + [i], used + w + (PARTY if row else 0.0))
    walk(0, [], 0.0)
    return [r for _, _, r in sorted(out)]


def fit_row(ring, frame, sizes, setback, gap):
    """The houses of the row that covers the most of the frontage at this setback with every
    house inside the lot: [(i, (asset, origin, face, corners))], i its place from the chord's
    start; [] when none fits."""
    mid, n, t, front, right, length, deep, face = frame
    for row in rows_by_cover(length - 2 * gap, sizes, deep - setback):
        total = sum(sizes[i][3] - sizes[i][1] for i in row) + PARTY * (len(row) - 1)
        slack = length / 2 - gap - total / 2
        for shift in (0.0, -slack, slack):               # centred; at the start; at the end
            houses = row_houses(frame, sizes, row, setback, -total / 2 + shift)
            if all(inside(ring, q) for _, h in houses for q in h[3]):
                return houses
    return []


def row_houses(frame, sizes, row, setback, s):
    """The row's houses, the first one's start `s` metres along the chord from its middle."""
    mid, n, t, front, right, length, deep, face = frame
    houses = []
    for place_i, i in enumerate(row):
        asset, x0, y0, x1, y1 = sizes[i]
        w = x1 - x0
        c = s + w / 2                                    # this house's middle, along the chord
        s += w + PARTY
        cx = (x0 + x1) / 2
        # Its front (asset y = y1) stands `setback` in from the chord; the chord runs along `t`,
        # which is the house's left (-right), so its middle moves by `c` along `t`.
        o = (mid[0] + t[0] * c + n[0] * (setback + y1) - right[0] * cx,
             mid[1] + t[1] * c + n[1] * (setback + y1) - right[1] * cx)
        corners = [(o[0] + right[0] * x + front[0] * y, o[1] + right[1] * x + front[1] * y)
                   for x, y in ((x0, y0), (x1, y0), (x1, y1), (x0, y1))]
        houses.append((place_i, (asset, o, face, corners)))
    return houses


def outline(corners, step=OUTLINE_STEP):
    """Points round a footprint every `step` metres or less, its corners among them."""
    pts = []
    for k in range(4):
        a, b = corners[k], corners[(k + 1) % 4]
        m = max(1, int(math.ceil(math.dist(a, b) / step)))
        pts.extend((a[0] + (b[0] - a[0]) * j / m, a[1] + (b[1] - a[1]) * j / m) for j in range(m))
    return pts


def clear_rows(step, rows):
    """Rule 4: for each lot, the row at the least push that keeps the most of it (by width) where
    `keep_out` is 0, with the houses still not clear dropped: [(lot, [(i, house)])]. One batch."""
    try:
        fields = step.fields(["role:keep_out"], window=[0, 0, 1, 1])
    except (LookupError, RpcError):
        fields = []
    if not fields:
        return [(name, tries[0][1] if tries else []) for name, tries in rows]
    formula = f"f('{fields[0].name}')"
    for fld in fields[1:]:
        formula = f"max({formula}, f('{fld.name}'))"
    ext = (step.grid or {}).get("extent_mm")
    px, py, spans = [], [], []
    for _, tries in rows:
        for _, row in tries:
            for _, h in row:
                a = len(px)
                for q in outline(h[3]):
                    x, y = _mm(q)
                    if ext:
                        x, y = min(max(x, ext[0]), ext[2]), min(max(y, ext[1]), ext[3])
                    px.append(x)
                    py.append(y)
                spans.append((a, len(px)))
    v = [float(x) for x in _list(step.eval("= " + formula, x=px, y=py))] if px else []
    out, k = [], 0
    for name, tries in rows:
        best = None                                    # (-width clear, push, row clear)
        for push, row in tries:
            keep = []
            for i, h in row:
                a, b = spans[k]
                k += 1
                if max(v[a:b]) <= KEEP_OUT_ZERO:
                    keep.append((i, h))
            width = sum(_width(h[3]) for _, h in keep)
            if keep and (best is None or (-width, push) < best[:2]):
                best = (-width, push, keep)
        out.append((name, best[2] if best else []))
    return out


KEEP_OUT_ZERO = 1e-6                                  # keep_out at or below this reads as 0


def _width(corners):
    return math.dist(corners[0], corners[1])


def runs(row):
    """A lot's row split where a house was dropped: lists of consecutive (i, house)."""
    out = []
    for i, h in sorted(row, key=lambda ih: ih[0]):
        if out and out[-1][-1][0] == i - 1:
            out[-1].append((i, h))
        else:
            out.append([(i, h)])
    return out


def run_outline(run):
    """The rectangle round a run of houses standing front to front in a line, counter-clockwise:
    the first house's two corners at the run's start and the last one's two at its end."""
    first, last = run[0][1][3], run[-1][1][3]
    # corners: (x0, y0), (x1, y0), (x1, y1), (x0, y1) in the asset frame, `right` along x1 - x0;
    # the run goes along -right (the chord's direction), so it starts at x1 and ends at x0.
    ring = [first[1], last[0], last[3], first[2]]
    return ring if _area2(ring) > 0 else ring[::-1]


TERRACE_FADE = (4.0, 8.0)                             # lib/town `terrace`: its hold fades out here


def place_terraces(step, p, chosen, small, nlots):
    """Rule 5 with `terrace`: each run of houses stands on one terrace (a region of that kind the
    step returns), and rule 7 is taken per run with the terrace's own fade."""
    all_runs = [(name, run) for name, row in chosen for run in runs(row)]
    steep = []
    if p.max_cut is not None and all_runs:
        moved = pad_moves(step, [run_outline(r) for _, r in all_runs], p.grade,
                          fade=TERRACE_FADE, reach=TERRACE_FADE[1])
        if moved is not None:
            steep = [house_key(n, i) for (n, r), m in zip(all_runs, moved) if m > p.max_cut
                     for i, _ in r]
            all_runs = [nr for nr, m in zip(all_runs, moved) if m <= p.max_cut]
    houses = 0
    for name, run in all_runs:
        key = "t" + house_key(name, run[0][0])[1:]
        step.out.feature(key, p.terrace, region=[_mm(q) for q in run_outline(run)])
        for i, (asset, at, face, _) in run:
            step.out.instance(house_key(name, i), asset, at=_mm(at), face=face)
            houses += 1
    say(step, houses, len({n for n, _ in all_runs}), nlots, small, steep, len(all_runs))


def footprint_relief(step, footprints):
    """Highest less lowest land height over each footprint's corners and middle, in one batch;
    None when the world has no surface field (nothing is skipped then)."""
    try:
        (surface,) = step.fields(["role:surface"], window=[0, 0, 1, 1])
    except (LookupError, RpcError):
        return None
    ext = (step.grid or {}).get("extent_mm")
    px, py = [], []
    for corners in footprints:
        mid = (sum(c[0] for c in corners) / 4, sum(c[1] for c in corners) / 4)
        for c in list(corners) + [mid]:
            x, y = _mm(c)
            if ext:                                    # a reading beyond the world reads its edge
                x, y = min(max(x, ext[0]), ext[2]), min(max(y, ext[1]), ext[3])
            px.append(x)
            py.append(y)
    h = [float(v) for v in _list(step.eval(f"= f('{surface.name}')", x=px, y=py))]
    return [max(h[5 * i:5 * i + 5]) - min(h[5 * i:5 * i + 5]) for i in range(len(footprints))]


PAD_STEP = 2.0                                       # metres between readings round a footprint
PAD_REACH = 24.0                                     # metres off the footprint read, at most


def pad_moves(step, footprints, grade, fade=(10.0, 25.0), reach=PAD_REACH):
    """For each footprint, the most its pad would move the land at the readings of rule 7; None
    when the world has no surface field (nothing is skipped then). `fade` is the pad kind's (lib/land
    `pad` 10 to 25 m, lib/town `terrace` 4 to 8 m), `reach` the metres read round it."""
    try:
        (surface,) = step.fields(["role:surface"], window=[0, 0, 1, 1])
    except (LookupError, RpcError):
        return None
    ext = (step.grid or {}).get("extent_mm")
    px, py, at = [], [], []                              # at: (footprint, d), d None under it
    for i, corners in enumerate(footprints):
        mid = (sum(c[0] for c in corners) / 4, sum(c[1] for c in corners) / 4)
        pts = [(c, None) for c in list(corners) + [mid]]
        x0 = min(c[0] for c in corners) - reach
        y0 = min(c[1] for c in corners) - reach
        nx = int((max(c[0] for c in corners) + reach - x0) // PAD_STEP) + 1
        ny = int((max(c[1] for c in corners) + reach - y0) // PAD_STEP) + 1
        for j in range(ny):
            for k in range(nx):
                q = (x0 + k * PAD_STEP, y0 + j * PAD_STEP)
                d = _off(corners, q)
                if d <= reach:
                    pts.append((q, d))
        for c, d in pts:
            x, y = _mm(c)
            if ext:                                    # a reading beyond the world reads its edge
                x, y = min(max(x, ext[0]), ext[2]), min(max(y, ext[1]), ext[3])
            px.append(x)
            py.append(y)
            at.append((i, d))
    h = [float(v) for v in _list(step.eval(f"= f('{surface.name}')", x=px, y=py))]
    under = [[] for _ in footprints]
    for (i, d), v in zip(at, h):
        if d is None:
            under[i].append(v)
    mean = [sum(u) / len(u) for u in under]
    fade_in, fade_out = fade
    out = [0.0] * len(footprints)
    for (i, d), v in zip(at, h):
        d = d or 0.0                                   # under the footprint: levelled whole
        fade = 1.0 - _smoothstep(fade_in, fade_out, d)
        out[i] = max(out[i], fade * max(0.0, abs(v - mean[i]) - grade * d))
    return out


def _off(corners, q):
    """Metres from q to the footprint (a convex quadrilateral); 0 inside it."""
    if inside(corners, q):
        return 0.0
    return min(_seg_dist(corners[k], corners[(k + 1) % 4], q) for k in range(4))


def _unit(v):
    n = math.hypot(v[0], v[1])
    return (v[0] / n, v[1] / n) if n else (0.0, 0.0)


def _smoothstep(a, b, x):
    t = min(max((x - a) / (b - a), 0.0), 1.0)
    return t * t * (3.0 - 2.0 * t)


def read_lots(step, table):
    """(name, outer ring in metres, counter-clockwise) of every cell of the table."""
    # The core adds the `class` column to a cells table's list by itself (protocol.md 3.3); asking
    # for it in `columns` asks for an attribute of that name instead, which no lot has.
    fl = step.features(select=[table])
    cls = fl.get("class")
    out = []
    for f, name in enumerate(fl.names):
        if cls is not None and str(_list(cls)[f]) != "cell":
            continue
        pieces = fl.rings(f)
        if len(pieces) != 1:
            continue
        ring = _open([(float(v[0]) / 1000.0, float(v[1]) / 1000.0) for v in _list(pieces[0][0])])
        if len(ring) < 3:
            continue
        if _area2(ring) < 0:
            ring.reverse()
        out.append((name, ring))
    return out


def read_streets(step, selectors):
    """Every street's segments, in metres. `build.features` takes `kind:<k>` alone, or a list of
    names (protocol.md 3.3), so each `kind:` selector is its own call and the names one more."""
    kinds = [s for s in selectors if s.startswith("kind:")]
    names = [s for s in selectors if not s.startswith("kind:")]
    lists = [step.features(select=k) for k in kinds]
    if names:
        lists.append(step.features(select=names))
    segs = []
    for fl in lists:
        for f in range(len(fl)):
            for piece in fl.rings(f):
                for r in piece:
                    pts = [(float(v[0]) / 1000.0, float(v[1]) / 1000.0) for v in _list(r)]
                    segs.extend(zip(pts, pts[1:]))
    return segs


def asset_sizes(step, assets):
    """(asset, x0, y0, x1, y1) in metres in the asset frame, the largest footprint first; ties
    in the order written."""
    t = step.pack(select=assets, columns=["bounds"])
    by = {str(i): b for i, b in zip(_list(t["id"]), _list(t["bounds"]))}
    out = []
    for a in assets:
        if a in by:
            b = [float(v) / 1000.0 for v in _list(by[a])]
            out.append((a, b[0], b[1], b[3], b[4]))
    return sorted(out, key=lambda s: -(s[3] - s[1]) * (s[4] - s[2]))   # stable: ties as written


ON_STREET = 0.05                                      # metres: a lot vertex this near a street lies on it


def frontage_chord(ring, segs):
    """The lot's frontage as a chord (from, to), the lot's inside on its left. A lot cut along a
    bending street has its street side in several edges: the longest run of consecutive edges whose
    ends all lie on a street is the frontage, and its chord stands for it. With no such edge, the
    one edge whose middle lies nearest a street."""
    n = len(ring)
    on = [min(_seg_dist(s, e, v) for s, e in segs) <= ON_STREET for v in ring]
    edge_on = [on[i] and on[(i + 1) % n] and math.dist(ring[i], ring[(i + 1) % n]) > 0
               for i in range(n)]
    best = None                                         # (length, first edge, edges)
    if not all(edge_on):
        for i in range(n):
            if edge_on[i] and not edge_on[i - 1]:       # a run starts here
                k = 0
                while edge_on[(i + k) % n]:
                    k += 1
                a, b = ring[i], ring[(i + k) % n]
                if best is None or math.dist(a, b) > best[0] + 1e-9:
                    best = (math.dist(a, b), i, k)
    if best is not None:
        _, i, k = best
        return ring[i], ring[(i + k) % n]
    e = frontage(ring, segs)
    return ring[e], ring[(e + 1) % n]


def frontage(ring, segs):
    """The index of the edge whose middle lies nearest a street; ties to the lower index."""
    best, bd = 0, math.inf
    for i in range(len(ring)):
        a, b = ring[i], ring[(i + 1) % len(ring)]
        if math.dist(a, b) == 0:
            continue
        m = ((a[0] + b[0]) / 2, (a[1] + b[1]) / 2)
        d = min(_seg_dist(s, e, m) for s, e in segs)
        if d < bd - 1e-9:
            best, bd = i, d
    return best


def house_key(lot, i=0):
    """`lots#c<digest>_<station>` gives `h<digest>_<station>` to its row's first house and
    `h<digest>_<station>_<i>` to the house `i` places along."""
    key = lot.split("#", 1)[1] if "#" in lot else lot
    key = "h" + (key[1:] if key.startswith("c") else key)
    return key if i == 0 else f"{key}_{i}"


def inside(ring, pt):
    """Strictly inside, with a millimetre's tolerance on the outline counted as inside."""
    x, y = pt
    for a, b in zip(ring, ring[1:] + ring[:1]):
        if _seg_dist(a, b, pt) <= 0.001:
            return True
    c = False
    for (x0, y0), (x1, y1) in zip(ring, ring[1:] + ring[:1]):
        if (y0 > y) != (y1 > y) and x < (x1 - x0) * (y - y0) / (y1 - y0) + x0:
            c = not c
    return c


def _seg_dist(a, b, p):
    dx, dy = b[0] - a[0], b[1] - a[1]
    l2 = dx * dx + dy * dy
    t = 0.0 if l2 == 0 else max(0.0, min(1.0, ((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / l2))
    return math.hypot(a[0] + t * dx - p[0], a[1] + t * dy - p[1])


def _open(ring):
    """A ring without a closing repeat of its first point, if it carries one."""
    return ring[:-1] if len(ring) > 1 and ring[0] == ring[-1] else ring


def _area2(ring):
    return sum(x0 * y1 - x1 * y0 for (x0, y0), (x1, y1) in zip(ring, ring[1:] + ring[:1]))


def _mm(pt):
    return (int(round(pt[0] * 1000)), int(round(pt[1] * 1000)))


def _list(col):
    return col.tolist() if hasattr(col, "tolist") else list(col)
