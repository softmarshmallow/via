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
2. **The house.** The first asset of `assets` (in the order written: the preferred first) that
   fits: its width across the frontage at most the frontage's length less `gap` on each side, its
   depth at most the lot's depth from that edge less `setback`, and its footprint's four corners
   inside the lot. A lot no asset fits is left empty, and counted (`small`, info).
3. **Where and which way.** The house's front (its asset frame's `+y`, assets.md) faces the street:
   `face` is the bearing of the frontage edge's outward normal, clockwise from north, in whole
   millidegrees. Its front stands `setback` metres in from the frontage edge's middle.
4. **Ground.** With `ground` (say `"pad"`, lib/land), each house is lent that ground kind
   (rules.md 2.1), so a house on a slope stands on a levelled bank, not on the hillside (TODO,
   "Local streets and lots ignore the land").

Names are by place (identity.md 2.2): a lot `lots#c<digest>_<station>` gives the house
`h<digest>_<station>`, so a house keeps its name exactly when its lot does (identity.md 2.3), and a
hand edit at a lot (`clear` with `{ of = "lots", class = "cell", near }`) reaches it.

`params`: `lots` (required, the cells table's id); `streets` (selectors, default
`["kind:main_street", "kind:street"]`); `assets` (default `["house_narrow", "house_low"]`);
`setback` (1 m); `gap` (0.5 m); `ground` (optional, a ground kind).

Library code imports only viasdk and the standard library (R19b).
"""

import math

PARAMS_EXPLAIN = ("A param of the houses step is outside what it takes: `lots` names one cells "
                  "table; `streets` and `assets` are lists of names; `setback` and `gap` are 0 or "
                  "more; `ground` names a ground kind.")

EXPLAIN = {
    "small": "The houses step left some lots empty: no asset in `assets` fits the lot's frontage "
    "less `gap` on each side and its depth less `setback`.",
    "no_street": "The houses step found no street to face; it placed nothing.",
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
    small = []
    for name, ring in lots:
        got = place(ring, streets, sizes, p)
        if got is None:
            small.append(name)
            continue
        asset, at, face = got
        keys = {"ground": p.ground} if p.ground else {}
        step.out.instance(house_key(name), asset, at=_mm(at), face=face, **keys)
    if small:
        step.info("small", "step." + step.id, f"{len(small)} lots too small for a house.",
                  "Lower `gap` or `setback`, add a narrower asset, or lengthen the lots' `spacing`.",
                  data={"count": len(small), "lots": small[:20]}, explain=EXPLAIN["small"])


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
    """(asset, x0, y0, x1, y1) in metres in the asset frame, in the order written."""
    t = step.pack(select=assets, columns=["bounds"])
    by = {str(i): b for i, b in zip(_list(t["id"]), _list(t["bounds"]))}
    out = []
    for a in assets:
        if a in by:
            b = [float(v) / 1000.0 for v in _list(by[a])]
            out.append((a, b[0], b[1], b[3], b[4]))
    return out


def place(ring, segs, sizes, p):
    """(asset, origin in metres, face in degrees) for one lot, or None when nothing fits."""
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
            return asset, o, face
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
