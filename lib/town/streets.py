"""The town library's streets step: local streets inside a district, laid on the land.

A generate step, run as a worker through the SDK, declared by the world (lib/town/kinds.toml's
header shows the whole pattern). It returns paths of kind `street` and nothing else: the core nodes
them with the main streets into a network whose areas are blocks, and cuts lots along the blocks'
frontage (geometry.md 8, `[network]`, `[cells]`). The step cuts no block and no lot itself
(R19b, R23); main streets are not its business either: they are written in text by what they
connect and routed over the land by the core (`route`, derived.md 3; R16).

How it lays them (ours, written for this library):

1. **The land, once.** The world's surface (`role:surface`) is sampled every `sample` metres over
   the district's bounds in one `expr.eval` batch (R23). Height and its gradient between samples
   are bilinear, in doubles.
2. **Which way the long streets run.** The district's mean gradient gives the fall of its land.
   Where the land is steeper than `steep` (degrees, as `slope()` in formulas.md reads), a long street
   follows the contour: it runs across the fall and is pulled back to its starting height after
   every `step` metres (one Newton step along the gradient), so it neither climbs nor drops. Where
   the land is flatter it runs straight on, along `bearing` (degrees clockwise from north; default
   the district's long side), keeping the last direction a contour gave it.
3. **Seeds.** Long streets start on a line across them through `origin` (a point the world writes),
   every `spacing` metres, on the land's fall where it is steep. Without `origin`, through the
   district's centre snapped to whole `spacing`s of the world frame; a written origin is the one an
   edit at the district's edge cannot move (R13). Each is traced both ways from its seed.
4. **Ends.** A long street that comes back to its seed (a contour round a knoll) is one loop,
   ending where it began; one shorter than `2 * cross` is not laid, nor one that comes round
   within `0.4 * spacing` of itself anywhere else (a spiral off a summit). Any other ends
   one step past the district's outline (the network's `within` cuts it there, and its outline
   is a source, so the end is a junction), or, where it comes within
   `0.4 * spacing` of a street already laid, on that street: its last point is the nearest point of
   the other street, written into both, so the two meet at a vertex (TODO, "Dead ends tripled":
   a street ends at a junction or is not laid).
5. **Cross streets.** Between each two neighbouring long streets, every `cross` metres along the
   lower one (staggered by half a `cross` on alternate pairs, so they meet as T junctions rather
   than in a line), a short street runs straight to the nearest point of the other: on a contour
   that is the fall line, the shortest way up. Its two ends are written into both long streets.
   One that leaves the district, or is longer than `2 * spacing`, is not laid.
6. **Down to the shore.** With `shore` (a path feature: the waterfront, R18), short streets run
   from the long street lowest on the land straight to the nearest point of the shore, every
   `cross` metres along it (offset by half a `cross` from its cross streets above), so streets end
   on the waterfront and the network nodes them there. Each ends exactly on the shore's line; one
   longer than `2 * spacing`, or whose start or middle lies outside the district, is not laid. Named
   `d<m>`, `m` the metres along that long street where it starts.
7. **Thinned.** Each long street is thinned to the points that stray more than 0.25 m from a
   straight line (Douglas and Peucker 1973) before its junction points are written into it.

Names are by place (identity.md 2.2): a long street is `s<k>` where `k` is its seed's place on the
line across, in whole spacings from the centre (`s0`, `s1`, `sm1`); a cross street is
`x<k>_<m>`, between `s<k>` and its neighbour, `m` the metres along `s<k>` where it starts. So an
edit at one end of the district renames no street at the other, and the network's block and lot
names follow (identity.md 2.3).

`params`: `district` (required, one region feature's name); `shore` (optional, one path feature's
name: the line streets run down to); `origin` (optional, [x, y] metres: the seed line's origin);
`spacing` (40 m between long streets);
`cross` (90 m between cross streets); `steep` (4 degrees); `bearing` (degrees, optional); `step`
(4 m, the tracing step); `sample` (4 m); `min_length` (24 m: a shorter long street is not laid).

Library code imports only viasdk and the standard library (R19b).
"""

import math

from viasdk import RpcError

PARAMS_EXPLAIN = ("A param of the streets step is outside what it takes: `district` names one "
                  "region feature; `spacing`, `cross`, `step` and `sample` are above 0; `steep` is "
                  "0 or more; `bearing` is a number of degrees.")

EXPLAIN = {
    "no_surface": "The streets step found no field with role `surface`, so it cannot read the "
    "land; it lays straight streets along `bearing`.",
    "none": "The streets step laid no street: the district is narrower than `spacing`, or every "
    "long street was shorter than `min_length`.",
}


CLOSED = "\0closed"                                   # trace's answer for a loop back to its seed
SELF = "\0self"                                       # for one that comes round on itself elsewhere


class ParamError(ValueError):
    def __init__(self, key, message, hint):
        super().__init__(message)
        self.key, self.message, self.hint = key, message, hint


class Params:
    def __init__(self, raw):
        self.district = raw.get("district")
        if not isinstance(self.district, str) or not self.district:
            raise ParamError("district", "`district` is missing", "name one region feature")
        self.spacing = _positive(raw, "spacing", 40)
        self.cross = _positive(raw, "cross", 90)
        self.step = _positive(raw, "step", 4)
        self.sample = _positive(raw, "sample", 4)
        self.min_length = _number(raw, "min_length", 24)
        self.steep = _number(raw, "steep", 4)
        if self.steep < 0:
            raise ParamError("steep", f"`steep` is {self.steep}", "write 0 or more degrees")
        b = raw.get("bearing")
        self.bearing = None if b is None else _number(raw, "bearing", 0)
        o = raw.get("origin")
        if o is not None and not (isinstance(o, list) and len(o) == 2
                                  and all(isinstance(v, (int, float)) and not isinstance(v, bool)
                                          and math.isfinite(v) for v in o)):
            raise ParamError("origin", "`origin` is not a point", "write [x, y] in metres")
        self.origin = None if o is None else (float(o[0]), float(o[1]))
        self.shore = raw.get("shore")
        if self.shore is not None and (not isinstance(self.shore, str) or not self.shore):
            raise ParamError("shore", "`shore` is not a name", "name one path feature")


def _number(raw, key, default):
    v = raw.get(key, default)
    if isinstance(v, bool) or not isinstance(v, (int, float)) or not math.isfinite(v):
        raise ParamError(key, f"`{key}` is not a number", "write a number")
    return float(v)


def _positive(raw, key, default):
    v = _number(raw, key, default)
    if v <= 0:
        raise ParamError(key, f"`{key}` is {v}", "write a number above 0")
    return v


def generate(step):
    try:
        p = Params(step.params)
        ring = district_ring(step, p.district)
        shore = shore_line(step, p.shore) if p.shore else None
    except ParamError as e:
        step.error("params", f"step.{step.id}.params.{e.key}", e.message, e.hint,
                   explain=PARAMS_EXPLAIN)
        return
    land = Land.read(step, ring, p.sample)
    if land is None:
        step.warning("no_surface", "step." + step.id, "No surface field; streets are straight.",
                     "Declare a field with role `surface`.", explain=EXPLAIN["no_surface"])
        land = Land.flat(ring, p.sample)
    streets = lay(p, ring, land, shore)
    if not streets:
        step.warning("none", "step." + step.id, "No street was laid.",
                     "Lower `spacing` or `min_length`, or widen the district.",
                     explain=EXPLAIN["none"])
        return
    for key in sorted(streets):
        step.out.feature(key, "street", path=[_mm(pt) for pt in streets[key]])


def district_ring(step, name):
    fl = step.features(select=[name])
    if len(fl) != 1 or int(_list(fl["geometry"])[0]) != 2:
        raise ParamError("district", f"`district` names `{name}`, which is not one region feature",
                         "name one feature whose shape is a region")
    pieces = fl.rings(0)
    if len(pieces) != 1:
        raise ParamError("district", f"`{name}` is a region of {len(pieces)} pieces",
                         "name a region of one piece")
    ring = [(float(v[0]) / 1000.0, float(v[1]) / 1000.0) for v in _list(pieces[0][0])]
    return ring[:-1] if len(ring) > 1 and ring[0] == ring[-1] else ring


def shore_line(step, name):
    fl = step.features(select=[name])
    if len(fl) != 1 or int(_list(fl["geometry"])[0]) != 1:
        raise ParamError("shore", f"`shore` names `{name}`, which is not one path feature",
                         "name one feature whose shape is a path")
    pts = [(float(v[0]) / 1000.0, float(v[1]) / 1000.0) for v in _list(fl.rings(0)[0][0])]
    if len(pts) < 2:
        raise ParamError("shore", f"`{name}` has fewer than two points", "name a path")
    return pts


# --- the land ---------------------------------------------------------------------------------


class Land:
    """Heights sampled on a lattice over the district's bounds; bilinear between, in metres."""

    def __init__(self, x0, y0, d, nx, ny, h):
        self.x0, self.y0, self.d, self.nx, self.ny, self.h = x0, y0, d, nx, ny, h

    @staticmethod
    def lattice(ring, sample):
        xs = [v[0] for v in ring]
        ys = [v[1] for v in ring]
        x0, y0 = min(xs) - 2 * sample, min(ys) - 2 * sample
        nx = int(math.ceil((max(xs) + 2 * sample - x0) / sample)) + 1
        ny = int(math.ceil((max(ys) + 2 * sample - y0) / sample)) + 1
        return x0, y0, nx, ny

    @classmethod
    def read(cls, step, ring, sample):
        name = _surface(step)
        if name is None:
            return None
        x0, y0, nx, ny = cls.lattice(ring, sample)
        ext = (step.grid or {}).get("extent_mm")
        px, py = [], []
        for j in range(ny):
            for i in range(nx):
                x, y = _mm((x0 + i * sample, y0 + j * sample))
                if ext:                      # samples beyond the world read its edge
                    x = min(max(x, ext[0]), ext[2])
                    y = min(max(y, ext[1]), ext[3])
                px.append(x)
                py.append(y)
        h = [float(v) for v in _list(step.eval(f"= f('{name}')", x=px, y=py))]
        return cls(x0, y0, sample, nx, ny, h)

    @classmethod
    def flat(cls, ring, sample):
        x0, y0, nx, ny = cls.lattice(ring, sample)
        return cls(x0, y0, sample, nx, ny, [0.0] * (nx * ny))

    def _cell(self, x, y):
        fx = min(max((x - self.x0) / self.d, 0.0), self.nx - 1.000001)
        fy = min(max((y - self.y0) / self.d, 0.0), self.ny - 1.000001)
        i, j = int(fx), int(fy)
        return i, j, fx - i, fy - j

    def _at(self, i, j):
        return self.h[j * self.nx + i]

    def height(self, x, y):
        i, j, u, v = self._cell(x, y)
        a, b = self._at(i, j), self._at(i + 1, j)
        c, d = self._at(i, j + 1), self._at(i + 1, j + 1)
        return (a * (1 - u) + b * u) * (1 - v) + (c * (1 - u) + d * u) * v

    def gradient(self, x, y):
        """Rise per metre east and north, by the bilinear patch's own derivative."""
        i, j, u, v = self._cell(x, y)
        a, b = self._at(i, j), self._at(i + 1, j)
        c, d = self._at(i, j + 1), self._at(i + 1, j + 1)
        gx = ((b - a) * (1 - v) + (d - c) * v) / self.d
        gy = ((c - a) * (1 - u) + (d - b) * u) / self.d
        return gx, gy


def _surface(step):
    try:
        (a,) = step.fields(["role:surface"], window=[0, 0, 1, 1])
    except (LookupError, RpcError):
        return None
    return a.name


# --- laying -----------------------------------------------------------------------------------


def lay(p, ring, land, shore=None):
    """Every street, by key: a list of points in metres."""
    steep = math.tan(math.radians(p.steep))
    # The seed line's origin: `origin` when written, else the district's centre snapped to whole
    # `spacing`s of the world frame. A written origin is the one an edit at the district's edge
    # cannot move (R13: a distant edit moves no contour street); a snapped centre still moves when
    # the edit carries it across a snap boundary.
    if p.origin is not None:
        cx, cy = p.origin
    else:
        cx, cy = centroid(ring)
        cx = math.floor(cx / p.spacing + 0.5) * p.spacing
        cy = math.floor(cy / p.spacing + 0.5) * p.spacing
    along = _unit(_bearing_vec(p.bearing if p.bearing is not None else long_side(ring)))
    gx, gy = mean_gradient(ring, land)
    if math.hypot(gx, gy) >= steep and math.hypot(gx, gy) > 0:
        across = _unit((gx, gy))
        along = _orient((-across[1], across[0]), along)
    else:
        across = (along[1], -along[0])
    reach = diameter(ring)
    seeds = []
    for k in range(-int(reach / p.spacing) - 1, int(reach / p.spacing) + 2):
        s = (cx + across[0] * k * p.spacing, cy + across[1] * k * p.spacing)
        if inside(ring, s):
            seeds.append((k, s))
    seeds.sort(key=lambda ks: (abs(ks[0]), ks[0]))   # from the centre out: the centre is laid first
    longs = {}                                        # k -> points
    meets = []                                        # (k, other k): k's last point lies on other
    for k, s in seeds:
        if any(_dist_to(longs[o], s) < 0.4 * p.spacing for o in longs):
            continue
        fwd, hit_f = trace(p, ring, land, s, along, steep, longs)
        if hit_f is SELF:
            continue
        if hit_f is CLOSED:                           # round a knoll: one loop, from its seed
            pts = [s] + fwd
            if _length(pts) < 2 * p.cross:            # a loop too small for a cross street
                continue
            longs[k] = thin(pts, 0.25)
            continue
        back, hit_b = trace(p, ring, land, s, (-along[0], -along[1]), steep, longs)
        if hit_b is SELF:
            continue
        if hit_b is CLOSED:                           # cannot close going back and not forward
            hit_b = None
        pts = list(reversed(back)) + [s] + fwd
        if _length(pts) < p.min_length:
            continue
        pts = thin(pts, 0.25)
        longs[k] = pts
        if hit_b is not None:
            meets.append((k, hit_b, 0))
        if hit_f is not None:
            meets.append((k, hit_f, -1))
    # A long street that ends on another: its end is written into the other as a vertex.
    for k, other, end in meets:
        q, longs[other] = insert_nearest(longs[other], longs[k][end])
        longs[k][end] = q
    crosses = {}
    order = sorted(longs)
    for n, (k, kn) in enumerate(zip(order, order[1:])):
        if kn != k + 1:
            continue
        first = p.cross * (0.5 if n % 2 else 1.0)
        m = first
        total = _length(longs[k])
        while m < total - 0.5 * p.cross:
            a = point_at(longs[k], m)
            b = nearest_on(longs[kn], a)
            if (math.dist(a, b) <= 2 * p.spacing and inside(ring, a) and inside(ring, b)
                    and inside(ring, ((a[0] + b[0]) / 2, (a[1] + b[1]) / 2))):
                a, longs[k] = insert_nearest(longs[k], a)
                b, longs[kn] = insert_nearest(longs[kn], b)
                crosses[f"x{_tag(k)}_{int(round(m))}"] = [a, b]
            m += p.cross
    if shore and longs:
        crosses.update(down_to(p, ring, land, longs, shore))
    out = {f"s{_tag(k)}": pts for k, pts in longs.items()}
    out.update(crosses)
    return out


def down_to(p, ring, land, longs, shore):
    """Streets from the long street lowest on the land to the shore (step 6); each start written
    into that long street, each end on the shore's line, to the millimetre."""
    def mean_height(pts):
        return sum(land.height(*v) for v in pts) / len(pts)

    low = min(longs, key=lambda k: (mean_height(longs[k]), k))
    out = {}
    total = _length(longs[low])
    m = 0.5 * p.cross
    while m < total - 0.25 * p.cross:
        a = point_at(longs[low], m)
        b = nearest_on(shore, a)
        b = (round(b[0], 3), round(b[1], 3))
        mid = ((a[0] + b[0]) / 2, (a[1] + b[1]) / 2)
        if math.dist(a, b) <= 2 * p.spacing and inside(ring, a) and inside(ring, mid):
            a, longs[low] = insert_nearest(longs[low], a)
            out[f"d{int(round(m))}"] = [a, b]
        m += p.cross
    return out


def trace(p, ring, land, start, direction, steep, laid):
    """Points from ``start`` (not included) in ``direction``, until one step past the outline or
    onto a street in ``laid``; the street's key when it ends on one, else None."""
    level = land.height(*start)
    pts = []
    pos, d = start, direction
    limit = 4 * diameter(ring)
    walked = 0.0
    while walked < limit:
        gx, gy = land.gradient(*pos)
        g2 = gx * gx + gy * gy
        if g2 > 0 and math.sqrt(g2) >= steep:
            d = _orient(_unit((-gy, gx)), d)          # across the fall, the way it was going
        nxt = (pos[0] + d[0] * p.step, pos[1] + d[1] * p.step)
        if g2 > 0 and math.sqrt(g2) >= steep:          # back to the street's level
            gx, gy = land.gradient(*nxt)
            g2 = gx * gx + gy * gy
            if g2 > 0:
                dh = (level - land.height(*nxt)) / g2
                pull = max(-p.step / 2, min(p.step / 2, dh * math.sqrt(g2)))
                nxt = (nxt[0] + pull * gx / math.sqrt(g2), nxt[1] + pull * gy / math.sqrt(g2))
        walked += math.dist(pos, nxt)
        if walked > 2 * p.spacing and math.dist(nxt, start) < 0.4 * p.spacing:
            pts.append(start)                         # a closed contour: the street is a loop
            return pts, CLOSED
        recent = int(0.5 * p.spacing / p.step) + 2
        if len(pts) > recent + 1 and _dist_to(pts[:-recent], nxt) < 0.4 * p.spacing:
            return pts, SELF                          # it comes round on itself, not to its seed
        if not inside(ring, nxt):
            pts.append(nxt)                           # one step past: the clip cuts it there
            return pts, None
        for o, other in laid.items():
            if _dist_to(other, nxt) < 0.4 * p.spacing:
                pts.append(nearest_on(other, nxt))
                return pts, o
        pts.append(nxt)
        pos = nxt
    return pts, None


# --- plane geometry, in metres (doubles) --------------------------------------------------------


def centroid(ring):
    a = cx = cy = 0.0
    for (x0, y0), (x1, y1) in zip(ring, ring[1:] + ring[:1]):
        c = x0 * y1 - x1 * y0
        a += c
        cx += (x0 + x1) * c
        cy += (y0 + y1) * c
    if a == 0:
        return sum(v[0] for v in ring) / len(ring), sum(v[1] for v in ring) / len(ring)
    return cx / (3 * a), cy / (3 * a)


def long_side(ring):
    """The bearing of the district's longer bounding side: 90 (east) when wider than tall."""
    xs = [v[0] for v in ring]
    ys = [v[1] for v in ring]
    return 90.0 if max(xs) - min(xs) >= max(ys) - min(ys) else 0.0


def diameter(ring):
    xs = [v[0] for v in ring]
    ys = [v[1] for v in ring]
    return math.hypot(max(xs) - min(xs), max(ys) - min(ys))


def mean_gradient(ring, land):
    sx = sy = 0.0
    n = 0
    for j in range(land.ny):
        for i in range(land.nx):
            pt = (land.x0 + i * land.d, land.y0 + j * land.d)
            if inside(ring, pt):
                gx, gy = land.gradient(*pt)
                sx, sy, n = sx + gx, sy + gy, n + 1
    return (sx / n, sy / n) if n else (0.0, 0.0)


def inside(ring, pt):
    x, y = pt
    c = False
    for (x0, y0), (x1, y1) in zip(ring, ring[1:] + ring[:1]):
        if (y0 > y) != (y1 > y) and x < (x1 - x0) * (y - y0) / (y1 - y0) + x0:
            c = not c
    return c


def _seg_nearest(a, b, p):
    dx, dy = b[0] - a[0], b[1] - a[1]
    l2 = dx * dx + dy * dy
    t = 0.0 if l2 == 0 else max(0.0, min(1.0, ((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / l2))
    return (a[0] + t * dx, a[1] + t * dy), t


def nearest_on(pts, p):
    best, bd = pts[0], math.inf
    for a, b in zip(pts, pts[1:]):
        q, _ = _seg_nearest(a, b, p)
        d = math.dist(q, p)
        if d < bd:
            best, bd = q, d
    return best


def insert_nearest(pts, p):
    """The nearest point of ``pts`` to ``p``, snapped to a whole millimetre, and ``pts`` with it
    written in as a vertex (or the vertex it falls on), so two streets meet exactly there."""
    q = nearest_on(pts, p)
    q = (round(q[0], 3), round(q[1], 3))
    for v in pts:
        if math.dist(v, q) < 0.0015:
            return v, pts
    best, bd = 0, math.inf
    for i, (a, b) in enumerate(zip(pts, pts[1:])):
        d = math.dist(_seg_nearest(a, b, q)[0], q)
        if d < bd:
            best, bd = i, d
    return q, pts[:best + 1] + [q] + pts[best + 1:]


def point_at(pts, m):
    for a, b in zip(pts, pts[1:]):
        L = math.dist(a, b)
        if m <= L and L > 0:
            t = m / L
            return (a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1]))
        m -= L
    return pts[-1]


def thin(pts, tol):
    """Douglas and Peucker (1973): the first and last point kept, and every point farther than
    ``tol`` from the line through the two kept around it."""
    if len(pts) < 3:
        return list(pts)
    keep = [False] * len(pts)
    keep[0] = keep[-1] = True
    stack = [(0, len(pts) - 1)]
    while stack:
        i, j = stack.pop()
        far, fd = None, tol
        for k in range(i + 1, j):
            d = math.dist(_seg_nearest(pts[i], pts[j], pts[k])[0], pts[k])
            if d > fd:
                far, fd = k, d
        if far is not None:
            keep[far] = True
            stack += [(i, far), (far, j)]
    return [p for p, k in zip(pts, keep) if k]


def _dist_to(pts, p):
    return math.dist(nearest_on(pts, p), p)


def _length(pts):
    return sum(math.dist(a, b) for a, b in zip(pts, pts[1:]))


def _unit(v):
    n = math.hypot(*v)
    return (v[0] / n, v[1] / n) if n else (1.0, 0.0)


def _orient(v, like):
    return v if v[0] * like[0] + v[1] * like[1] >= 0 else (-v[0], -v[1])


def _bearing_vec(deg):
    r = math.radians(deg)
    return (math.sin(r), math.cos(r))


def _tag(k):
    return str(k) if k >= 0 else f"m{-k}"


def _mm(pt):
    return (int(round(pt[0] * 1000)), int(round(pt[1] * 1000)))


def _list(col):
    return col.tolist() if hasattr(col, "tolist") else list(col)
