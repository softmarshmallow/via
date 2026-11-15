"""The town library's houses step: one house on each lot, its front to the street.

A generate step, run as a worker through the SDK, declared by the world in a layer above its
`[cells]` table (a step reads a maker's output only from above it, geometry.md 8.3). It reads the
lots through `build.features` (`select` naming the cells table: its cells and rest pieces, with a
`class` column, protocol.md 3.3) and the streets the same way, and places at most one house per lot.
It cuts no lot itself (R19b, R23).

How it places (ours, written for this library):

1. **The frontage side.** Of a lot's outline edges, the one whose middle lies nearest a street
   (`streets`, default every `main_street` and `street`) is its frontage; a lot is cut along its
   block's frontage (geometry.md 3.6), so that edge lies on the street's side of the block.
2. **The house.** The largest asset of `assets` (by footprint area; ties in the order written)
   that fits: its width across the frontage at most the frontage's length less `gap` on each side,
   its depth at most the lot's depth from that edge less `setback`, and its footprint's four
   corners inside the lot. A lot no asset fits is left empty, and counted (`small`, info). The
   largest first, so a wide lot gets the wide house rather than the first one written (quay placed
   no `house_wide` when it was written second).
3. **Where and which way.** The house's front (its asset frame's `+y`, assets.md) faces the street:
   `face` is the bearing of the frontage edge's outward normal, clockwise from north, in whole
   millidegrees. Its front stands `setback` metres in from the frontage edge's middle.
4. **Ground.** With `ground` (say `"pad"`, lib/land), each house is lent that ground kind
   (rules.md 2.1), so a house on a slope stands on a levelled bank, not on the hillside (TODO,
   "Local streets and lots ignore the land").
5. **Relief.** Without `ground`, and with `max_relief` (metres), the land (`role:surface`) is read
   at the footprint's four corners and its middle, every house in one `expr.eval` batch (R23); a
   house whose highest and lowest reading differ by more than `max_relief` is not placed, and
   counted (`steep`, info): a house left on a slope shows a floor in the air or buried. With
   `ground` the pad levels it, so relief is not read. When pads from a layer above a step that reads
   the ground work (TODO, core), a world sets `ground = "pad"` and drops `max_relief`.
6. **What a pad can level.** With `ground` and `max_cut` (metres), the house is placed only where
   its pad stays within the kind's `max_change`: the pad holds the land at the footprint's mean,
   less or more `grade` (rise over run) per metre out, fading out between `reach` 10 and 25 m
   (lib/land `pad`), so at a reading `d` metres off the footprint it moves the land by
   `(1 - smoothstep(10, 25, d)) * max(0, |h - mean| - grade * d)`. The land is read at the
   footprint's corners and middle, and on a 2 m lattice round it out to 24 m, in one batch; a
   house whose pad would move a reading by more than `max_cut` is not placed (`steep`, info): an
   edit beside a cut would otherwise stop the build (`authority.max_change`). `grade` (default
   0.3) and `max_cut` are the ground kind's own defaults, which a step cannot read; write them as
   the kind has them.

Names are by place (identity.md 2.2): a lot `lots#c<digest>_<station>` gives the house
`h<digest>_<station>`, so a house keeps its name exactly when its lot does (identity.md 2.3), and a
hand edit at a lot (`clear` with `{ of = "lots", class = "cell", near }`) reaches it.

`params`: `lots` (required, the cells table's id); `streets` (selectors, default
`["kind:main_street", "kind:street"]`); `assets` (default `["house_narrow", "house_low"]`);
`setback` (1 m); `gap` (0.5 m); `ground` (optional, a ground kind); `max_relief` (optional,
metres, 0 or more); `max_cut` and `grade` (optional, with `ground`: the ground kind's).

Library code imports only viasdk and the standard library (R19b).
"""

import math

from viasdk import RpcError

PARAMS_EXPLAIN = ("A param of the houses step is outside what it takes: `lots` names one cells "
                  "table; `streets` and `assets` are lists of names; `setback`, `gap` and "
                  "`max_relief` are 0 or more; `ground` names a ground kind.")

EXPLAIN = {
    "small": "The houses step left some lots empty: no asset in `assets` fits the lot's frontage "
    "less `gap` on each side and its depth less `setback`.",
    "no_street": "The houses step found no street to face; it placed nothing.",
    "steep": "The houses step left some lots empty: the land under the house rises more than "
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
    small, placed = [], []
    for name, ring in lots:
        got = place(ring, streets, sizes, p)
        if got is None:
            small.append(name)
            continue
        placed.append((name, got))
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
    for name, (asset, at, face, _) in placed:
        keys = {"ground": p.ground} if p.ground else {}
        step.out.instance(house_key(name), asset, at=_mm(at), face=face, **keys)
    if small:
        step.info("small", "step." + step.id, f"{len(small)} lots too small for a house.",
                  "Lower `gap` or `setback`, add a narrower asset, or lengthen the lots' `spacing`.",
                  data={"count": len(small), "lots": small[:20]}, explain=EXPLAIN["small"])
    if steep:
        step.info("steep", "step." + step.id,
                  f"{len(steep)} lots too steep for an unlevelled house, or for its pad.",
                  "Set `ground` to a levelling kind (`pad`), or raise `max_relief` or `max_cut`.",
                  data={"count": len(steep), "lots": steep[:20]}, explain=EXPLAIN["steep"])


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


def pad_moves(step, footprints, grade):
    """For each footprint, the most its pad would move the land at the readings of rule 6; None
    when the world has no surface field (nothing is skipped then)."""
    try:
        (surface,) = step.fields(["role:surface"], window=[0, 0, 1, 1])
    except (LookupError, RpcError):
        return None
    ext = (step.grid or {}).get("extent_mm")
    px, py, at = [], [], []                              # at: (footprint, d), d None under it
    for i, corners in enumerate(footprints):
        mid = (sum(c[0] for c in corners) / 4, sum(c[1] for c in corners) / 4)
        pts = [(c, None) for c in list(corners) + [mid]]
        x0 = min(c[0] for c in corners) - PAD_REACH
        y0 = min(c[1] for c in corners) - PAD_REACH
        nx = int((max(c[0] for c in corners) + PAD_REACH - x0) // PAD_STEP) + 1
        ny = int((max(c[1] for c in corners) + PAD_REACH - y0) // PAD_STEP) + 1
        for j in range(ny):
            for k in range(nx):
                q = (x0 + k * PAD_STEP, y0 + j * PAD_STEP)
                d = _off(corners, q)
                if d <= PAD_REACH:
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
    out = [0.0] * len(footprints)
    for (i, d), v in zip(at, h):
        d = d or 0.0                                   # under the footprint: levelled whole
        fade = 1.0 - _smoothstep(10.0, 25.0, d)
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


def place(ring, segs, sizes, p):
    """(asset, origin in metres, face in degrees, footprint corners) for one lot, or None when
    nothing fits."""
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
    for asset, x0, y0, x1, y1 in sizes:
        w, d = x1 - x0, y1 - y0
        if w > length - 2 * p.gap or d > deep - p.setback:
            continue
        # The front edge (asset y = y1) stands `setback` in from the frontage's middle.
        cx = (x0 + x1) / 2
        o = (mid[0] + n[0] * (p.setback + y1) - right[0] * cx,
             mid[1] + n[1] * (p.setback + y1) - right[1] * cx)
        corners = [(o[0] + right[0] * x + front[0] * y, o[1] + right[1] * x + front[1] * y)
                   for x, y in ((x0, y0), (x1, y0), (x1, y1), (x0, y1))]
        if all(inside(ring, c) for c in corners):
            return asset, o, face, corners
    return None


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


def house_key(lot):
    """`lots#c<digest>_<station>` gives `h<digest>_<station>`."""
    key = lot.split("#", 1)[1] if "#" in lot else lot
    return "h" + (key[1:] if key.startswith("c") else key)


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
