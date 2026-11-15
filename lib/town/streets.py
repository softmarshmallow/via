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
2. **Which way the long streets run.** The district's mean gradient gives the fall of its land
   (with `origin` written, the mean within `spacing` of the origin only, so an edit at the
   district's far edge turns no seed line: R13).
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
   on the district's outline, 5 mm past the crossing (the network's `within` cuts it there, and its
   outline is a source, so the end is a junction; the bed and the paint read the path the step
   returns, so it must not run on into the fields), or, where it comes within
   `0.4 * spacing` of a street already laid, on that street: its last point is the nearest point of
   the other street, written into both, so the two meet at a vertex (TODO, "Dead ends tripled":
   a street ends at a junction or is not laid). Where it comes within `join` metres of a main way
   (`ways`) while running beside it (within 30 degrees of the way's direction there), it ends on
   the way's nearest point; one that crosses a main way runs on across it (the network nodes the
   crossing). The main ways are taken as given: the step never moves or splits them.
   With `shore` and `lot_depth`, a long street that comes nearer the shore than `2 * lot_depth`
   is not laid, so a row of lots faces the waterfront and another faces the lowest long street.
   **4b.** An end within `spacing / 2` of an end of a long street laid before it is moved onto
   that end (the nearest; ties by key): one that meets no street (on the outline, or on a main
   way), and one that ends on another street whose own end lies within `spacing / 2` of the meeting
   (not a loop). So two long streets leaving the district side by side end at one node, not two a
   metre apart, and no stub is left past a junction (townland `move_crossing`:
   `upper_streets#sm1` ended on `s0` 1.7 m from `s0`'s start).
5. **Cross streets.** Between each two neighbouring long streets, every `cross` metres along the
   lower one (staggered by half a `cross` on alternate pairs, so they meet as T junctions rather
   than in a line), a short street runs straight to the nearest point of the other: on a contour
   that is the fall line, the shortest way up. Its two ends are written into both long streets.
   One that leaves the district, or is longer than `2 * spacing`, is not laid.
   **5b.** Each end lies on the node of a street end laid before it (a long street's end, or a
   cross street's) on the same long street within `spacing / 4`, when there is one (the far end
   only while the grade still holds), and a cross street shorter than `spacing / 4` is not laid:
   so streets meeting a long street from either side meet at one crossroads, and no sliver lot is
   left where two long streets converge (city old town, M3 slice 5 review on foot: ten ends 0.1
   to 3 m apart, `x2_330` 7 m long).
   **The grade.** Where the land along that line rises more than `max_grade` (rise over run) over
   any `GRADE_RUN` (10 m) of it, read every metre, the street climbs aslant instead: its far end
   moves along the upper street `SLANT_STEP` (2 m) at a time, forward first then back at each
   distance, to the first point whose line holds `max_grade`, crosses no main way nor another
   cross street, and whose middle lies `spacing / 4` or more from every main way (a street beside
   the way it would double); with none within `2 * spacing`, it is not laid (quay `x0_140`
   climbed 34.5 % over 10 m; aslant, it ran 96 m back across the main way). Ours.
6. **Down to the shore.** With `shore` (a path feature: the waterfront, R18), short streets run
   from the long street lowest on the land straight to the nearest point of the shore, every
   `cross` metres along it (offset by half a `cross` from its cross streets above), so streets end
   on the waterfront and the network nodes them there. Each ends exactly on the shore's line; one
   shorter than a metre or longer than `2 * spacing`, or whose start or middle lies outside the
   district, is not laid. One that crosses a main way (`ways`) ends on the way where it first
   meets it (the network nodes the T), if that leaves it a metre or more and its start lies `join`
   metres or more from every main way (not a cut across a way's own bend); one that passes within
   `spacing / 2` of a main way without crossing it is not laid (the way already goes down there).
   Named `d<m>`, `m` the metres along that long street where it starts.
7. **Thinned.** Each long street is thinned to the points that stray more than 0.25 m from a
   straight line (Douglas and Peucker 1973) before its junction points are written into it.
8. **Joined.** A street end that meets nothing (not another street, a main way, the shore, nor
   the district's outline) and has another street or a main way within `spacing / 2` ahead of it
   (the join turns less than 90 degrees from the street's last segment, and its middle lies in the
   district) runs on straight to that line's nearest point, written into a street it joins as a
   vertex. So no street stops a few metres short of a junction (quay-walk).
9. **Length.** Every street carries `length`, its path's length in metres (from the millimetre
   points returned), so the kind's bed can fade to nothing over its last metres and stop at its
   ends instead of cutting a round cap past them (lib/town/kinds.toml, `street`).

Names are by place (identity.md 2.2): a long street is `s<k>` where `k` is its seed's place on the
line across, in whole spacings from the centre (`s0`, `s1`, `sm1`); a cross street is
`x<k>_<m>`, between `s<k>` and its neighbour, `m` the metres along `s<k>` where it starts. So an
edit at one end of the district renames no street at the other, and the network's block and lot
names follow (identity.md 2.3).

`params`: `district` (required, one region feature's name); `shore` (optional, one path feature's
name: the line streets run down to); `origin` (optional, [x, y] metres: the seed line's origin);
`spacing` (40 m between long streets);
`cross` (90 m between cross streets); `steep` (4 degrees); `bearing` (degrees, optional); `step`
(4 m, the tracing step); `sample` (4 m); `min_length` (24 m: a shorter long street is not laid);
`ways` (selectors of the main ways, default `["kind:main_street"]`; none found is no error);
`join` (8 m: a long street running beside a main way this near ends on it); `lot_depth` (0 m, off:
with `shore`, the depth of the lots, so the lowest long street keeps `2 * lot_depth` from the
shore); `max_grade` (0.2, rise over run: a cross street's steepest 10 m, step 5; 0 is off);
`pattern` (`"contour"`, the steps above, or `"grid"`, below).

**`pattern = "grid"`** (a district of a planned town, cut by main ways it is bounded by): long
streets run straight along `bearing` (default the district's long side) every `spacing` metres
across it, and cross streets straight across them every `cross` metres, both through `origin` (or
the district's centre snapped to whole `spacing`s), each cut `PAST` beyond the outline; a line the
outline cuts in several pieces gives one street per piece, and one shorter than `min_length` is not
laid. The land does not bend a grid (nor `max_grade`, `shore`, `steep`): the network nodes every
crossing. Long streets are `s<k>`, cross streets `c<j>` (`j` whole `cross`es from the origin), a
later piece of one line `_<i>`. Step 8 (joined) and 9 (length) apply.

Library code imports only viasdk and the standard library (R19b).
"""

import math

from viasdk import RpcError

PARAMS_EXPLAIN = ("A param of the streets step is outside what it takes: `district` names one "
                  "region feature; `spacing`, `cross`, `step` and `sample` are above 0; `steep`, "
                  "`join`, `lot_depth` and `max_grade` are 0 or more; `bearing` is a number of "
                  "degrees; `ways` is a list of names and `kind:` selectors; `pattern` is "
                  "`contour` or `grid`.")

EXPLAIN = {
    "no_surface": "The streets step found no field with role `surface`, so it cannot read the "
    "land; it lays straight streets along `bearing`.",
    "none": "The streets step laid no street: the district is narrower than `spacing`, or every "
    "long street was shorter than `min_length`.",
}


CLOSED = "\0closed"                                   # trace's answer for a loop back to its seed
SELF = "\0self"                                       # for one that comes round on itself elsewhere
WAY = "\0way"                                         # for one that ends on a main way beside it
BESIDE = math.cos(math.radians(30))                   # within 30 degrees of a way's direction
PAST = 0.005                                          # metres an end runs past the outline it is cut on
PATTERNS = ("contour", "grid")
GRADE_RUN = 10.0                                      # metres a cross street's grade is read over
SLANT_STEP = 2.0                                      # metres along the upper street per slant tried


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
        self.ways = raw.get("ways", ["kind:main_street"])
        if not isinstance(self.ways, list) or not all(isinstance(w, str) and w for w in self.ways):
            raise ParamError("ways", "`ways` is not a list of names", "write a list of selectors")
        self.join = _number(raw, "join", 8)
        self.lot_depth = _number(raw, "lot_depth", 0)
        self.max_grade = _number(raw, "max_grade", 0.2)
        self.pattern = raw.get("pattern", "contour")
        if self.pattern not in PATTERNS:
            raise ParamError("pattern", f"`pattern` is {self.pattern!r}",
                             "write \"contour\" or \"grid\"")
        for key in ("join", "lot_depth", "max_grade"):
            if getattr(self, key) < 0:
                raise ParamError(key, f"`{key}` is {getattr(self, key)}", "write 0 or more metres")


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
    streets = lay(p, ring, land, shore, read_ways(step, p.ways))
    if not streets:
        step.warning("none", "step." + step.id, "No street was laid.",
                     "Lower `spacing` or `min_length`, or widen the district.",
                     explain=EXPLAIN["none"])
        return
    for key in sorted(streets):
        mm = [_mm(pt) for pt in streets[key]]
        length = sum(math.dist(a, b) for a, b in zip(mm, mm[1:])) / 1000.0
        step.out.feature(key, "street", path=mm, length=round(length, 3))


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


def read_ways(step, selectors):
    """Every main way's points, in metres: each path piece one polyline. A selector that selects
    nothing gives none (a district may have no main way). `build.features` takes `kind:<k>` alone,
    or a list of names (protocol.md 3.3)."""
    kinds = [s for s in selectors if s.startswith("kind:")]
    names = [s for s in selectors if not s.startswith("kind:")]
    asks = list(kinds) + ([names] if names else [])
    out = []
    for sel in asks:
        try:
            fl = step.features(select=sel)
        except (LookupError, RpcError):
            continue
        geom = _list(fl["geometry"])
        for f in range(len(fl)):
            if int(geom[f]) != 1:
                continue
            for piece in fl.rings(f):
                pts = [(float(v[0]) / 1000.0, float(v[1]) / 1000.0) for v in _list(piece[0])]
                if len(pts) >= 2:
                    out.append(pts)
    return out


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


def lay(p, ring, land, shore=None, ways=()):
    """Every street, by key: a list of points in metres. ``ways``: the main ways' polylines."""
    if p.pattern == "grid":
        return join_ends(p, ring, lay_grid(p, ring), ways, shore)
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
    # The fall: with a written origin, the land's mean gradient within `spacing` of it (a disc
    # inside the district), so an edit at the district's far edge turns no seed line either.
    gx, gy = mean_gradient(ring, land, (cx, cy, p.spacing) if p.origin is not None else None)
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
        fwd, hit_f = trace(p, ring, land, s, along, steep, longs, ways)
        if hit_f is SELF:
            continue
        if hit_f is CLOSED:                           # round a knoll: one loop, from its seed
            pts = [s] + fwd
            if _length(pts) < 2 * p.cross or near_shore(p, ring, shore, pts):
                continue                              # too small for a cross street, or the shore's
            longs[k] = thin(pts, 0.25)
            continue
        back, hit_b = trace(p, ring, land, s, (-along[0], -along[1]), steep, longs, ways)
        if hit_b is SELF:
            continue
        if hit_b is CLOSED:                           # cannot close going back and not forward
            hit_b = None
        pts = list(reversed(back)) + [s] + fwd
        if _length(pts) < p.min_length or near_shore(p, ring, shore, pts):
            continue
        pts = thin(pts, 0.25)
        for end, hit in ((0, hit_b), (-1, hit_f)):    # step 4b: an end beside a node takes it
            if hit is None or hit is WAY:
                pts[end] = snap_end(p, pts[end], longs)
        longs[k] = pts
        if hit_b is not None and hit_b is not WAY:
            meets.append((k, hit_b, 0))
        if hit_f is not None and hit_f is not WAY:
            meets.append((k, hit_f, -1))
    # A long street that ends on another: its end is written into the other as a vertex.
    for k, other, end in meets:
        longs[k][end], longs[other] = meet_point(p, longs[other], longs[k][end])
    crosses = {}
    order = sorted(longs)
    for n, (k, kn) in enumerate(zip(order, order[1:])):
        if kn != k + 1:
            continue
        first = p.cross * (0.5 if n % 2 else 1.0)
        m = first
        total = _length(longs[k])
        while m < total - 0.5 * p.cross:
            nodes = [q for o in longs.values() for q in (o[0], o[-1])]
            nodes += [q for c in crosses.values() for q in (c[0], c[-1])]
            a = snap_node(p, point_at(longs[k], m), longs[k], nodes)
            b = cross_to(p, ring, land, longs[kn], a, ways, crosses.values())
            if b is not None:
                b2 = snap_node(p, b, longs[kn], nodes)
                if b2 != b and (p.max_grade <= 0 or grade_of(land, a, b2) <= p.max_grade) \
                        and inside(ring, b2):
                    b = b2
            if b is not None and math.dist(a, b) >= p.spacing / 4:
                a, longs[k] = insert_nearest(longs[k], a)
                b, longs[kn] = insert_nearest(longs[kn], b)
                crosses[f"x{_tag(k)}_{int(round(m))}"] = [a, b]
            m += p.cross
    if shore and longs:
        crosses.update(down_to(p, ring, land, longs, shore, ways))
    out = {f"s{_tag(k)}": pts for k, pts in longs.items()}
    out.update(crosses)
    return join_ends(p, ring, out, ways, shore)


ON_LINE = 0.002                                       # metres: an end this near a line lies on it


def meet_point(p, other, pt):
    """Where a street ending at ``pt`` on ``other`` meets it, and ``other`` with it written in:
    its nearest point (written in as a vertex), or, with that point within `spacing / 2` of an
    end of ``other`` (not a loop), that end (step 4b: no stub past the junction)."""
    q0 = nearest_on(other, pt)
    tips = [] if other[0] == other[-1] else sorted(
        (math.dist(e, q0), e) for e in (other[0], other[-1]) if math.dist(e, q0) <= p.spacing / 2)
    if tips:
        return tips[0][1], other
    return insert_nearest(other, pt)


def snap_end(p, end, longs):
    """Step 4b: an end that meets no street, within `spacing / 2` of an end of a long street
    already laid, is that end (the nearest; ties by key), so two streets leaving the district
    side by side end at one node, not two a metre apart."""
    best = None
    for o in sorted(longs):
        for q in (longs[o][0], longs[o][-1]):
            d = math.dist(q, end)
            if ON_LINE < d <= p.spacing / 2 and (best is None or d < best[0]):
                best = (d, q)
    return end if best is None else best[1]


def snap_node(p, pt, line, nodes):
    """Step 5b: a cross street's end ``pt`` on ``line`` is the nearest of ``nodes`` (street ends
    laid before it) that lies on ``line`` within `spacing / 4` of it, so streets meeting one street
    from either side meet at one node, not two a metre apart; else ``pt``."""
    best = None
    for q in nodes:
        d = math.dist(q, pt)
        if d <= p.spacing / 4 and _dist_to(line, q) <= ON_LINE and (best is None or d < best[0]):
            best = (d, q)
    return pt if best is None else best[1]


def grade_of(land, a, b, run=GRADE_RUN):
    """The largest rise over run of the land along segment ``a``-``b``, read every metre or less,
    over `run` metres (the whole segment when shorter)."""
    L = math.dist(a, b)
    if L == 0:
        return 0.0
    n = max(1, int(math.ceil(L)))
    h = [land.height(a[0] + (b[0] - a[0]) * i / n, a[1] + (b[1] - a[1]) * i / n)
         for i in range(n + 1)]
    w = max(1, min(n, int(round(run / (L / n)))))
    return max(abs(h[i + w] - h[i]) for i in range(n - w + 1)) / (w * L / n)


def arc_at(pts, q):
    """The arc length along ``pts`` of its nearest point to ``q``."""
    best, bd, walked = 0.0, math.inf, 0.0
    for a, b in zip(pts, pts[1:]):
        r, t = _seg_nearest(a, b, q)
        d = math.dist(r, q)
        if d < bd:
            best, bd = walked + t * math.dist(a, b), d
        walked += math.dist(a, b)
    return best


def cross_to(p, ring, land, upper, a, ways=(), crosses=()):
    """Step 5's far end for a cross street from ``a``: the nearest point of the upper street when
    the land along it is no steeper than `max_grade` over `GRADE_RUN` metres; else the nearest
    point along the upper street, `SLANT_STEP` metres at a time either way (the forward way first
    at each distance), whose line holds it, crosses no main way (``ways``) nor another cross
    street (``crosses``), and whose middle lies `spacing / 4` or more from every main way (not a
    street beside the way), so the street climbs aslant; None when none within `2 * spacing` does,
    or the street leaves the district."""
    def fits(b):
        return (math.dist(a, b) <= 2 * p.spacing and inside(ring, a) and inside(ring, b)
                and inside(ring, ((a[0] + b[0]) / 2, (a[1] + b[1]) / 2)))

    b0 = nearest_on(upper, a)
    if not fits(b0):
        return None
    if p.max_grade <= 0 or grade_of(land, a, b0) <= p.max_grade:
        return b0
    s0, total = arc_at(upper, b0), _length(upper)
    n = 1
    while n * SLANT_STEP <= 2 * p.spacing:
        near = False
        for s in (s0 + n * SLANT_STEP, s0 - n * SLANT_STEP):
            if 0 <= s <= total:
                b = point_at(upper, s)
                if math.dist(a, b) <= 2 * p.spacing:
                    near = True
                    mid = ((a[0] + b[0]) / 2, (a[1] + b[1]) / 2)
                    if (fits(b) and grade_of(land, a, b) <= p.max_grade
                            and first_crossing(a, b, list(ways) + list(crosses)) is None
                            and all(_dist_to(w, mid) >= p.spacing / 4 for w in ways)):
                        return b
        if not near:
            break
        n += 1
    return None


def lay_grid(p, ring):
    """`pattern = "grid"`: long streets straight along `bearing` (default the district's long
    side) every `spacing` metres across it, and cross streets straight across them every `cross`
    metres along it, both through `origin` (or the district's centre snapped to whole
    `spacing`s), each cut at the outline `PAST` beyond its crossing. A line the outline cuts into
    several pieces gives one street per piece; one shorter than `min_length` is not laid. The
    network nodes every crossing (geometry.md 8.1), so no vertex is written here. Long streets are
    `s<k>`, cross streets `c<j>` (`j` whole `cross`es along from the origin), a later piece of
    the same line `_<i>`."""
    if p.origin is not None:
        cx, cy = p.origin
    else:
        cx, cy = centroid(ring)
        cx = math.floor(cx / p.spacing + 0.5) * p.spacing
        cy = math.floor(cy / p.spacing + 0.5) * p.spacing
    along = _unit(_bearing_vec(p.bearing if p.bearing is not None else long_side(ring)))
    across = (along[1], -along[0])
    reach = diameter(ring) + math.dist((cx, cy), centroid(ring))
    out = {}
    for prefix, gap, d, n in (("s", p.spacing, along, across), ("c", p.cross, across, along)):
        for k in range(-int(reach / gap) - 1, int(reach / gap) + 2):
            o = (cx + n[0] * k * gap, cy + n[1] * k * gap)
            pieces = [s for s in clip_line(ring, o, d, reach) if math.dist(*s) >= p.min_length]
            for i, seg in enumerate(pieces):
                out[f"{prefix}{_tag(k)}" + (f"_{i}" if i else "")] = list(seg)
    return out


def clip_line(ring, o, d, reach):
    """The pieces of the line through ``o`` along ``d`` (from `-reach` to `reach`) inside the
    ring, in order along ``d``, each end carried `PAST` beyond the outline."""
    a = (o[0] - d[0] * reach, o[1] - d[1] * reach)
    r = (2 * d[0] * reach, 2 * d[1] * reach)
    ts = []
    for c, e in zip(ring, ring[1:] + ring[:1]):
        s_ = (e[0] - c[0], e[1] - c[1])
        den = r[0] * s_[1] - r[1] * s_[0]
        if den == 0:
            continue
        w = (c[0] - a[0], c[1] - a[1])
        t = (w[0] * s_[1] - w[1] * s_[0]) / den
        u = (w[0] * r[1] - w[1] * r[0]) / den
        if 0 <= t <= 1 and 0 <= u < 1:
            ts.append(t)
    ts.sort()
    past = PAST / (2 * reach)
    at = lambda t: (a[0] + r[0] * t, a[1] + r[1] * t)  # noqa: E731
    out = []
    for t0, t1 in zip(ts, ts[1:]):
        if t1 - t0 > 2 * past and inside(ring, at((t0 + t1) / 2)):
            out.append((at(t0 - past), at(t1 + past)))
    return out


def join_ends(p, ring, streets, ways=(), shore=None):
    """Step 8: every free end within `spacing / 2` of another street or a main way, ahead of it,
    runs on to that line's nearest point. ``streets``: key -> points, changed in place and
    returned. Ends are taken in key order, the first end before the last."""
    def free(key, end):
        pt = streets[key][end]
        if not inside(ring, pt):
            return False                               # on the outline, cut there (step 4)
        lines = [v for k, v in streets.items() if k != key] + list(ways)
        lines += [shore] if shore else []
        return all(_dist_to(line, pt) > ON_LINE for line in lines)

    for key in sorted(streets):
        for end in (0, -1):
            pts = streets[key]
            if len(pts) < 2 or pts[0] == pts[-1] or not free(key, end):   # a loop has no end
                continue
            tip, prev = pts[end], pts[1 if end == 0 else -2]
            ahead = _unit((tip[0] - prev[0], tip[1] - prev[1]))
            best = None                                # (distance, point, street key or None)
            for k, line in [(k, v) for k, v in sorted(streets.items()) if k != key] + \
                    [(None, w) for w in ways]:
                q = nearest_on(line, tip)
                dq = math.dist(q, tip)
                mid = ((q[0] + tip[0]) / 2, (q[1] + tip[1]) / 2)
                if (ON_LINE < dq <= p.spacing / 2 and inside(ring, mid)
                        and (q[0] - tip[0]) * ahead[0] + (q[1] - tip[1]) * ahead[1] > 0
                        and (best is None or dq < best[0])):
                    best = (dq, q, k)
            if best is None:
                continue
            _, q, k = best
            if k is None:
                q = (round(q[0], 3), round(q[1], 3))
            else:
                q, streets[k] = insert_nearest(streets[k], q)
            streets[key] = pts + [q] if end == -1 else [q] + pts
    return streets


def near_shore(p, ring, shore, pts):
    """Whether a long street comes nearer the shore than `2 * lot_depth` inside the district."""
    if not shore or p.lot_depth <= 0:
        return False
    return any(_dist_to(shore, q) < 2 * p.lot_depth for q in pts if inside(ring, q))


def down_to(p, ring, land, longs, shore, ways=()):
    """Streets from the long street lowest on the land to the shore (step 6); each start written
    into that long street, each end on the shore's line, to the millimetre. One passing within
    `spacing / 2` of a main way is not laid: the way goes down there already."""
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
        if 1 <= math.dist(a, b) <= 2 * p.spacing and inside(ring, a) and inside(ring, mid):
            hit = first_crossing(a, b, ways)
            if hit is not None:                        # it meets a main way: it ends there
                b = (round(hit[0], 3), round(hit[1], 3))
            if (hit is not None and math.dist(a, b) >= 1
                    and all(_dist_to(w, a) >= p.join for w in ways)) or (
                    hit is None and not any(_seg_to_line(a, b, w) < p.spacing / 2 for w in ways)):
                a, longs[low] = insert_nearest(longs[low], a)
                out[f"d{int(round(m))}"] = [a, b]
        m += p.cross
    return out


def first_crossing(a, b, lines):
    """The point nearest ``a`` where segment ``a``-``b`` crosses or touches one of ``lines``, or
    None."""
    best, bt = None, math.inf
    r = (b[0] - a[0], b[1] - a[1])
    for pts in lines:
        for c, e in zip(pts, pts[1:]):
            s_ = (e[0] - c[0], e[1] - c[1])
            den = r[0] * s_[1] - r[1] * s_[0]
            if den == 0:
                continue
            w = (c[0] - a[0], c[1] - a[1])
            t = (w[0] * s_[1] - w[1] * s_[0]) / den
            u = (w[0] * r[1] - w[1] * r[0]) / den
            if 0 <= t <= 1 and 0 <= u <= 1 and t < bt:
                best, bt = (a[0] + t * r[0], a[1] + t * r[1]), t
    return best


def trace(p, ring, land, start, direction, steep, laid, ways=()):
    """Points from ``start`` (not included) in ``direction``, until the outline (ending `PAST`
    beyond its crossing), onto a street in ``laid`` (its key returned), or onto a main way it runs
    beside (`WAY`); else None."""
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
            pts.append(exit_point(ring, pos, nxt))    # on the outline: the clip cuts it there
            return pts, None
        for way in ways:                              # beside a main way: end on it
            q, tangent = nearest_with_tangent(way, nxt)
            step_dir = _unit((nxt[0] - pos[0], nxt[1] - pos[1]))
            if (math.dist(q, nxt) < p.join
                    and abs(step_dir[0] * tangent[0] + step_dir[1] * tangent[1]) >= BESIDE):
                pts.append((round(q[0], 3), round(q[1], 3)))
                return pts, WAY
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


def mean_gradient(ring, land, disc=None):
    """The mean gradient over the samples inside ``ring``; with ``disc`` (x, y, r), only those
    within ``r`` of (x, y), falling back to the whole ring when the disc holds none."""
    sx = sy = 0.0
    n = 0
    for j in range(land.ny):
        for i in range(land.nx):
            pt = (land.x0 + i * land.d, land.y0 + j * land.d)
            if disc is not None and math.dist(pt, disc[:2]) > disc[2]:
                continue
            if inside(ring, pt):
                gx, gy = land.gradient(*pt)
                sx, sy, n = sx + gx, sy + gy, n + 1
    if not n and disc is not None:
        return mean_gradient(ring, land)
    return (sx / n, sy / n) if n else (0.0, 0.0)


def inside(ring, pt):
    x, y = pt
    c = False
    for (x0, y0), (x1, y1) in zip(ring, ring[1:] + ring[:1]):
        if (y0 > y) != (y1 > y) and x < (x1 - x0) * (y - y0) / (y1 - y0) + x0:
            c = not c
    return c


def exit_point(ring, a, b):
    """Where the step from ``a`` (inside) to ``b`` (outside) first crosses the outline, carried
    `PAST` metres on along the step, so the end lies just outside and the clip makes the junction."""
    best = 1.0
    for c, e in zip(ring, ring[1:] + ring[:1]):
        r = (b[0] - a[0], b[1] - a[1])
        s = (e[0] - c[0], e[1] - c[1])
        den = r[0] * s[1] - r[1] * s[0]
        if den == 0:
            continue
        w = (c[0] - a[0], c[1] - a[1])
        t = (w[0] * s[1] - w[1] * s[0]) / den
        u = (w[0] * r[1] - w[1] * r[0]) / den
        if 0 <= t <= 1 and 0 <= u <= 1:
            best = min(best, t)
    L = math.dist(a, b)
    t = min(1.0, best + (PAST / L if L else 0))
    return (a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1]))


def nearest_with_tangent(pts, p):
    """The nearest point of a polyline to ``p`` and the unit direction of its segment there."""
    best, bd, tan = pts[0], math.inf, (1.0, 0.0)
    for a, b in zip(pts, pts[1:]):
        q, _ = _seg_nearest(a, b, p)
        d = math.dist(q, p)
        if d < bd:
            best, bd, tan = q, d, _unit((b[0] - a[0], b[1] - a[1]))
    return best, tan


def _seg_to_line(a, b, pts):
    """The least distance from segment ``a``-``b`` to a polyline, sampled every metre."""
    n = max(1, int(math.ceil(math.dist(a, b))))
    return min(_dist_to(pts, (a[0] + (b[0] - a[0]) * i / n, a[1] + (b[1] - a[1]) * i / n))
               for i in range(n + 1))


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
