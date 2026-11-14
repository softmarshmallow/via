"""The land library's uplands step: a hillside made from noise and the water that runs off it.

A generate step, run as a worker through the SDK, declared in the world in the lowest layer, so
everything above stands on what it makes (a library declares no step, protocol.md 1):

    [step.uplands]
    when    = "generate"
    worker  = true
    command = ["python", "-m", "viasdk", "lib:land/uplands.py"]
    params  = { relief = 70, fall = 135 }

The land is the step's, not the author's: the author says how high and which way it falls, and
the step does the rest, so a new world `seed` makes another hillside and moves what stands on it.
What it does:

1. **Rough land.** At every `cell` metres (a whole number of post spacings: every k-th post, and
   the last row and column) the engine evaluates, in one batch (`expr.eval`, R23), a formula of its own noise:
   `base + relief * (rise + 0.2 * fbm(320, 4) * (0.35 + rise) + 0.1 * (0.5 - abs(fbm(110, 3))) * rise)`
   (the last term ridges: noise folded at its zero),
   where `rise` runs from 0 at the corner the land falls to (`fall`, a bearing in degrees clockwise
   from north) to 1 at the opposite corner. Noise seeds are the step's own (`seed = 'uplands'`), so the land
   follows the world seed.
2. **Running water.** `rounds` times: hollows are filled to their spill (priority-flood, Barnes,
   Lehman and Mulla 2014, from the world's edge, where water leaves; filled cells rise a millimetre
   a cell away from their outlet so every cell drains). Each cell's water is shared among all its
   lower neighbours of eight, in proportion to the drop per metre to each (multiple flow
   directions, Freeman 1991: water spreads over a slope, so valleys need not run in rows and
   diagonals, the spike's recorded failure), and the area each cell gathers is added up from the
   highest cell down. Each cell is then lowered by `erode * sqrt(area) * slope * cell`, `slope` its
   drops per metre to its lower neighbours averaged by their own weight, and by no more than half
   its drop to the lowest of them, so a valley deepens over rounds rather than as a slot; then
   creep moves `creep` of the difference between each cell and the mean of its four neighbours.
   The water runs over the world and `margin` cells beyond each edge (16), so the world's own edge
   is not where streams gather; only the cells inside are returned.
3. **Out.** Two fields of its own, read at every post bilinearly from the cells (fields.md
   1.2's three lines): `land`, the land in metres, tagged `uplands`; and `wet`, 0 to 1, how much
   water passes, `log(area / cell²) / log(most / cell²)` (0 where a cell gathers only its own
   rain), tagged `wet`. One row on the terrain,
   `[field.<terrain>.then.rise]`, `value = "= v + f('land')"`, so the world's own base is
   added (write `base = "= 0"` for the step's land alone). And the main stream as a feature,
   `uplands#stream`, kind `beck` (lib/water) and tier `derived`: through the cell of the world that
   gathers most, up through the neighbour that gathers most while it gathers `stream_area` square metres
   and down by the steepest drop to the world's edge,
   then thinned to the vertices that stray more than half a cell from a straight line
   (Douglas and Peucker 1973), source first. Text stands on it by
   `{ of = "uplands", class = "feature", near = [x, y] }` and it moves with the land. Its
   `width` and `depth` come from the area it gathers at its mouth, by lib/terrain's rule for a
   course (5 * sqrt(km2), 1.5 to 60 m; depth 0.3 + 0.06 * width): one size along its length, as
   a beck has one `width`.

`params`, each optional: `relief` (70, metres from the low corner to the high), `base` (14, metres
at the low corner), `fall` (135, a bearing), `cell` (2, metres), `rounds` (30), `erode` (0.004),
`creep` (0.2), `margin` (16, cells), `stream_area` (3000, square metres); the rough land's shape
(asked by `massif`, so a world can ask for a massif rather than a ramp): `rough` (0.2, the noise's
share of `relief`), `rough_wave` (320, its wavelength in metres), `ridges` (0.1, the ridges' share
of `relief`), `ridge_wave` (110, metres); and `start` (`"noise"`, or `"surface"`: the water cuts
the world's own terrain as it stands in the step's head view instead of the step's noise, R14: the
author states, the step settles. The margin beyond the world carries the edge's ground on; the row
is then `value = "= f('land')"`, the land replacing the terrain's value, so `relief`, `base`,
`fall` and the shape keys are not read). About 8 s for 384 m at 2 m
cells on one core (measured on `massif`'s extent, macOS arm64).

A world that includes `uplands` must include `water` for the stream's kind, and declare a field
tagged `water_level` for the beck's level (lib/water).

Library code imports only viasdk and the standard library (R19b). A practice held across lib/,
to keep a rerun of a step on another system from drifting: cell-local arithmetic is plain Python
over the cells, in a fixed order, using only the operations IEEE 754 rounds correctly (`+`, `-`,
`*`, `/`, `sqrt`, comparisons, `frexp`); a function a platform's maths library computes its own
way (`sin`, `cos`, `tan`, `log`, `hypot`, noise) is the engine's, through `expr.eval` in a batch
(R23), as `weather.py` takes its tangent. So the bearing's sine and cosine are one `expr.eval`,
and `wet`'s logarithm is `log2` below, from `frexp` and squaring. This is a practice, not a
promise: a library step is a user's step (R19b), its result is recorded, not promised (R26,
numbers.md 9), and a rerun that differs is `graph.step_unstable`.

The step's own priority-flood (`fill`) and its stream's run downhill to the edge stay Python over
the cells: they work on the land this step is still making, round by round, which is in no build
the engine's `derive.spill` or `derive.descend` could read (derived.md 4: they read fields of the
build). The stream is traced over the filled cells by the flow the step computed (each cell's
largest share of water), the same water `wet` measures, and thinned as above; a world that wants
the line water follows over the returned land calls `derive.descend` from a step above this one.
"""

import bisect
import heapq
import math

PARAMS_EXPLAIN = ("A param of the uplands step is outside the range it takes: `start` is \"noise\" "
                  "or \"surface\"; `cell`, `rough_wave` and `ridge_wave` are metres above 0; "
                  "`rounds` and `margin` are 0 or more; `creep` is 0 to 0.25.")


class ParamError(ValueError):
    """A param outside what the step accepts: its key, what is wrong, and the range it takes."""

    def __init__(self, key, message, hint):
        super().__init__(message)
        self.key, self.message, self.hint = key, message, hint


def refuse(step, e):
    """Says a bad param at its key (`step.<id>.params`), so the build names the line it is on."""
    step.error("params", f"step.{step.id}.params.{e.key}", e.message, e.hint,
               explain=PARAMS_EXPLAIN)


SQRT2 = math.sqrt(2.0)
EIGHT = [(-1, -1), (0, -1), (1, -1), (-1, 0), (1, 0), (-1, 1), (0, 1), (1, 1)]


def generate(step):
    try:
        p = Params(step.params)
    except ParamError as e:
        refuse(step, e)
        return
    grid = step.grid
    x0, y0, x1, y1 = (int(v) for v in grid["extent_mm"])
    r_mm = int(grid["resolution_mm"])
    k = max(1, int(round(p.cell * 1000)) // r_mm)
    cell_mm = k * r_mm
    ny, nx = int(grid["ny"]), int(grid["nx"])
    ix, jy = ticks(nx, k), ticks(ny, k)
    m = p.margin
    cxs = ([x0 - (m - q) * cell_mm for q in range(m)] + [x0 + i * r_mm for i in ix]
           + [x1 + (q + 1) * cell_mm for q in range(m)])
    cys = ([y0 - (m - q) * cell_mm for q in range(m)] + [y0 + j * r_mm for j in jy]
           + [y1 + (q + 1) * cell_mm for q in range(m)])
    ncx = len(cxs)
    xs = [x for _ in cys for x in cxs]
    ys = [y for y in cys for _ in cxs]
    if p.start == "surface":
        (head,) = step.fields(["role:surface"], view="head")
        rows = head.tolist()
        # The cells on posts read the post; margin cells carry the nearest edge post on.
        pi = [ix[0]] * m + list(ix) + [ix[-1]] * m
        pj = [jy[0]] * m + list(jy) + [jy[-1]] * m
        h = [[float(rows[b][a]) for a in pi] for b in pj]
    else:
        vals = _list(step.eval(p.formula(grid, bearing(step, grid, p.fall)), x=xs, y=ys))
        h = [vals[j * ncx:(j + 1) * ncx] for j in range(len(cys))]
    cell_m = cell_mm / 1000.0
    area = None
    for _ in range(p.rounds):
        h, area = erode(h, cell_m, p)
    if area is None:
        area = accumulate(fill(h), cell_m)
    own = cell_m * cell_m
    inner = [row[m:m + len(ix)] for row in area[m:m + len(jy)]]
    most = max(max(row) for row in inner) / own
    lm = log2(most) if most > 1 else 0.0
    wet = [[log2(a / own) / lm if most > 1 else 0.0 for a in row] for row in inner]
    land = [row[m:m + len(ix)] for row in h[m:m + len(jy)]]
    (probe,) = step.fields(["role:surface"], view="head", window=[0, 0, 1, 1])
    step.out.field("land", upsample(land, ix, jy, nx, ny), tags=["uplands"], unit="m")
    step.out.field("wet", upsample(wet, ix, jy, nx, ny), tags=["wet"])
    step.out.row(probe.name, "rise",
                 value="= f('land')" if p.start == "surface" else "= v + f('land')")
    cells = stream(fill(h), area, p.stream_area, (m, m, m + len(ix) - 1, m + len(jy) - 1))
    line = [(cxs[i], cys[j]) for i, j in cells]
    path = thin(inside(line, (x0, y0, x1, y1)), cell_mm / 2)
    if len(path) >= 2:
        # Its size from the area it gathers at its mouth (asked by `massif`), by lib/terrain's rule
        # for a course: width 5 * sqrt(km2), 1.5 to 60 m; depth 0.3 + 0.06 * width, 0.3 to 3 m.
        mouth = max((area[j][i] for i, j in cells
                     if m <= i < m + len(ix) and m <= j < m + len(jy)), default=0.0)
        width = min(60.0, max(1.5, 5.0 * math.sqrt(mouth / 1.0e6)))
        depth = min(3.0, max(0.3, 0.3 + 0.06 * width))
        step.out.feature("stream", "beck", path=path, authority="derived",
                         width=round(width, 2), depth=round(depth, 2))


def inside(line, box):
    """The longest run of consecutive points of ``line`` inside ``box`` (edges included)."""
    best, run = [], []
    for x, y in line:
        if box[0] <= x <= box[2] and box[1] <= y <= box[3]:
            run.append((x, y))
            if len(run) > len(best):
                best = run[:]
        else:
            run = []
    return best


class Params:
    def __init__(self, params):
        self.relief = float(params.get("relief", 70))
        self.base = float(params.get("base", 14))
        self.fall = float(params.get("fall", 135))
        self.cell = float(params.get("cell", 2))
        self.rounds = int(params.get("rounds", 30))
        self.erode = float(params.get("erode", 0.004))
        self.creep = float(params.get("creep", 0.2))
        self.stream_area = float(params.get("stream_area", 3000))
        self.margin = int(params.get("margin", 16))
        self.rough = float(params.get("rough", 0.2))
        self.rough_wave = float(params.get("rough_wave", 320))
        self.ridges = float(params.get("ridges", 0.1))
        self.ridge_wave = float(params.get("ridge_wave", 110))
        self.start = str(params.get("start", "noise"))
        if self.start not in ("noise", "surface"):
            raise ParamError("start", f"`start` is `{self.start}`, not one of the two starts",
                             'write "noise" (rough land of its own) or "surface" (the surface below)')
        for key, v in (("rough_wave", self.rough_wave), ("ridge_wave", self.ridge_wave),
                       ("cell", self.cell)):
            if v <= 0:
                raise ParamError(key, f"`{key}` is {v}: a length in metres, above 0",
                                 f"write `{key}` in metres, above 0")
        for key, v in (("rounds", self.rounds), ("margin", self.margin)):
            if v < 0:
                raise ParamError(key, f"`{key}` is {v}: a count, 0 or more",
                                 f"write `{key}` as a whole number, 0 or more")
        if not 0 <= self.creep <= 0.25:
            raise ParamError("creep", f"`creep` is {self.creep}: 0 to 0.25, or a round overshoots",
                             "write `creep` from 0 to 0.25; the default is 0.2")

    def formula(self, grid, sin_cos):
        """The rough land, as one formula over x and y in metres; ``sin_cos`` is the fall's."""
        x0, y0, x1, y1 = (int(v) / 1000.0 for v in grid["extent_mm"])
        cx, cy = (x0 + x1) / 2, (y0 + y1) / 2
        # uphill is opposite the fall; a bearing is clockwise from north (+y)
        ux, uy = round(-sin_cos[0], 6), round(-sin_cos[1], 6)
        span = round(abs(ux) * (x1 - x0) + abs(uy) * (y1 - y0), 3)   # corner to corner along it
        rise = f"(0.5 + ((x - {cx}) * {ux} + (y - {cy}) * {uy}) / {span})"
        return (f"= {self.base} + {self.relief} * {rise}"
                f" + {self.rough * self.relief} * fbm({_num(self.rough_wave)}, 4, seed = 'uplands')"
                f" * (0.35 + {rise})"
                f" + {self.ridges * self.relief}"
                f" * (0.5 - abs(fbm({_num(self.ridge_wave)}, 3, seed = 'uplands_ridges'))) * {rise}")


def bearing(step, grid, degrees):
    """The sine and cosine of a bearing in degrees (the function table's `sin` and `cos` take degrees): the engine's, one expr.eval each (R23)."""
    x0, y0 = int(grid["extent_mm"][0]), int(grid["extent_mm"][1])
    (s,) = _list(step.eval(f"= sin({_num(degrees)})", x=[x0], y=[y0]))
    (c,) = _list(step.eval(f"= cos({_num(degrees)})", x=[x0], y=[y0]))
    return float(s), float(c)


def log2(v):
    """The base-2 logarithm of ``v`` > 0 from correctly rounded operations only: the exponent by
    ``frexp`` (exact), then 52 bits of the fraction by squaring (each square correctly rounded),
    which keeps a rerun stable (the lib/ practice above)."""
    m, e = math.frexp(v)
    m, e = m * 2.0, e - 1
    out, bit = float(e), 0.5
    for _ in range(52):
        m = m * m
        if m >= 2.0:
            m = m / 2.0
            out += bit
        bit = bit / 2.0
    return out


def fill(h):
    """Priority-flood from the edge: each cell raised to its spill, and filled cells sloped by a
    millimetre a cell toward the outlet, so every cell has a lower neighbour or is on the edge."""
    ny, nx = len(h), len(h[0])
    out = [row[:] for row in h]
    seen = [[False] * nx for _ in range(ny)]
    q = []
    for j in range(ny):
        for i in range(nx):
            if i in (0, nx - 1) or j in (0, ny - 1):
                heapq.heappush(q, (out[j][i], j, i))
                seen[j][i] = True
    while q:
        z, j, i = heapq.heappop(q)
        for di, dj in EIGHT:
            a, b = i + di, j + dj
            if 0 <= a < nx and 0 <= b < ny and not seen[b][a]:
                seen[b][a] = True
                if out[b][a] <= z:
                    out[b][a] = z + 0.001
                heapq.heappush(q, (out[b][a], b, a))
    return out


def _lower(f, i, j):
    """Each lower neighbour of an inner cell as (a, b, drop per cell length); none on the edge."""
    ny, nx = len(f), len(f[0])
    if i in (0, nx - 1) or j in (0, ny - 1):
        return []
    z = f[j][i]
    out = []
    for di, dj in EIGHT:
        a, b = i + di, j + dj
        d = (z - f[b][a]) / (SQRT2 if di and dj else 1.0)
        if d > 0:
            out.append((a, b, d))
    return out


def accumulate(f, cell_m):
    """Area (m²) gathered by each cell over filled ground ``f``: each cell's water shared among its
    lower neighbours by drop per metre, summed from the highest cell down."""
    ny, nx = len(f), len(f[0])
    order = sorted(((f[j][i], j, i) for j in range(ny) for i in range(nx)), reverse=True)
    area = [[cell_m * cell_m] * nx for _ in range(ny)]
    for _, j, i in order:
        low = _lower(f, i, j)
        if not low:
            continue
        total = sum(d for _, _, d in low)
        a0 = area[j][i]
        for a, b, d in low:
            area[b][a] += a0 * d / total
    return area


def erode(h, cell_m, p):
    """One round: fill, gather, incise by stream power, then creep. Returns the land and the area."""
    f = fill(h)
    area = accumulate(f, cell_m)
    ny, nx = len(h), len(h[0])
    out = [row[:] for row in h]
    for j in range(ny):
        for i in range(nx):
            low = _lower(f, i, j)
            if not low:
                continue
            steep = sum(d * d for _, _, d in low) / sum(d for _, _, d in low) / cell_m
            floor = min(f[b][a] for a, b, _ in low)
            cut = p.erode * math.sqrt(area[j][i]) * steep * cell_m
            z = min(out[j][i], f[j][i])
            out[j][i] = max(z - cut, z - 0.5 * (z - floor) if z > floor else z)
    if p.creep > 0:
        c = [row[:] for row in out]
        for j in range(1, ny - 1):
            for i in range(1, nx - 1):
                mean = (c[j][i - 1] + c[j][i + 1] + c[j - 1][i] + c[j + 1][i]) / 4
                out[j][i] = c[j][i] + p.creep * (mean - c[j][i])
    return out, area


def stream(f, area, threshold, box=None):
    """The main stream as cell indices, source first. It runs through the cell that gathers most
    (within ``box``, cell indices ``i0, j0, i1, j1`` inclusive, when given):
    up from there through the higher neighbour that gathers most while it gathers at least
    ``threshold``, and down from there by the steepest drop until it reaches the world's edge."""
    ny, nx = len(f), len(f[0])
    bi0, bj0, bi1, bj1 = box if box is not None else (0, 0, nx - 1, ny - 1)
    _, i0, j0 = max((area[j][i], i, j) for j in range(bj0, bj1 + 1) for i in range(bi0, bi1 + 1))
    up = [(i0, j0)]
    seen = {(i0, j0)}
    i, j = i0, j0
    while True:
        cand = []
        for di, dj in EIGHT:
            a, b = i + di, j + dj
            if 0 <= a < nx and 0 <= b < ny and (a, b) not in seen and f[b][a] > f[j][i] \
                    and area[b][a] >= threshold:
                cand.append((area[b][a], a, b))
        if not cand:
            break
        _, i, j = max(cand)
        up.append((i, j))
        seen.add((i, j))
    down = []
    i, j = i0, j0
    while True:
        low = _lower(f, i, j)
        if not low:
            break
        _, i, j = max((d, a, b) for a, b, d in low)
        down.append((i, j))
    return list(reversed(up)) + down


def thin(points, tol):
    """Douglas-Peucker: the vertices that stray more than ``tol`` from the line, ends kept."""
    if len(points) < 3:
        return list(points)
    keep = [False] * len(points)
    keep[0] = keep[-1] = True
    stack = [(0, len(points) - 1)]
    while stack:
        s, e = stack.pop()
        (ax, ay), (bx, by) = points[s], points[e]
        ex, ey = bx - ax, by - ay
        ee = ex * ex + ey * ey
        # Squared distances: the farthest post and the tolerance compare the same, from
        # correctly rounded operations only.
        far, at = -1.0, None
        for k in range(s + 1, e):
            px, py = points[k][0] - ax, points[k][1] - ay
            t = 0.0 if ee == 0 else max(0.0, min(1.0, (px * ex + py * ey) / ee))
            dx, dy = px - t * ex, py - t * ey
            d = dx * dx + dy * dy
            if d > far:
                far, at = d, k
        if at is not None and far > tol * tol:
            keep[at] = True
            stack.append((s, at))
            stack.append((at, e))
    return [pt for pt, k in zip(points, keep) if k]


def ticks(n, k):
    """The post indices cells stand on: every k-th, and the last."""
    t = list(range(0, n, k))
    if t[-1] != n - 1:
        t.append(n - 1)
    return t


def upsample(c, ix, jy, nx, ny):
    """Cell values read at every post, bilinear between the four cells around (fields.md 1.2:
    along x on the south and north rows, then between them). Cells stand on posts ``ix``, ``jy``."""
    cols = [_bracket(ix, i) for i in range(nx)]
    out = []
    for j in range(ny):
        b, fy = _bracket(jy, j)
        s_row, n_row = c[b], c[min(b + 1, len(jy) - 1)]
        row = []
        for a, fx in cols:
            a1 = min(a + 1, len(ix) - 1)
            s = (1 - fx) * s_row[a] + fx * s_row[a1]
            n = (1 - fx) * n_row[a] + fx * n_row[a1]
            row.append((1 - fy) * s + fy * n)
        out.append(row)
    return out


def _bracket(ts, i):
    """(index of the tick at or below i, fraction to the next tick)."""
    a = bisect.bisect_right(ts, i) - 1
    if a >= len(ts) - 1:
        return len(ts) - 1, 0.0
    return a, (i - ts[a]) / (ts[a + 1] - ts[a])


def _num(v):
    """A number as formula text: a whole number without its `.0`."""
    return str(int(v)) if float(v).is_integer() else repr(float(v))


def _list(col):
    return col.tolist() if hasattr(col, "tolist") else list(col)
