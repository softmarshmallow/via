"""The land library's weathering step: ground steeper than its angle of repose sheds downhill.

A generate step, run as a worker through the SDK, declared in the world in the layer whose ground
it weathers (a library declares no step, protocol.md 1):

    [step.weather]
    when    = "generate"
    worker  = true
    command = ["python", "-m", "viasdk", "lib:land/weather.py"]
    params  = { repose = 34, rounds = 60 }

What it does:

1. It reads the terrain (`role:surface`) in its head view (authority.md 4.4): the base and every
   row at or below its layer, never an effect. It returns a row on the terrain, so it may read no
   other view (`authority.step_view`). A lane's cut or a pad's level is applied after its row, on
   the weathered ground, exactly once (fields.md 7).
2. It samples that ground every `cell` metres (a whole number of post spacings, at least one), and
   relaxes it `rounds` times: between each two neighbours along x, then along y, a drop steeper than
   `repose` degrees moves `rate` of its excess from the higher to the lower, every pair of a pass
   computed from the ground before that pass. Material is kept: what leaves one cell lands on the
   next.
3. It returns two fields of its own on the world's posts, read bilinearly from the cells
   (fields.md 1.2's three lines): `shed`, metres the ground rose (above 0) or fell (below), tagged
   `shed`; and `talus`, 0 to 1, where shed material came to rest, tagged `talus`: the rise over the
   99.5th percentile of all rises, so a few deep piles do not wash the rest out. And one row,
   `[field.<terrain>.then.settle]`, `value = "= v + f('shed')"`.

Above the step's layer, a scatter reads `f('tag:talus')` to put fallen rock where it lies
(`massif`'s boulders).

What the numbers mean (asked by `massif`). `talus` is relative: 1 is the 99.5th percentile of this
build's rises, so on land with many gully walls a value of 0.15 is a thin skin, and the same value
means another depth under another seed. A threshold chosen from a measure reads the deposit in
metres instead, `max(0, f('tag:shed'))`: for example `smoothstep(0.3, 1.0, f('tag:shed'))` puts
boulders only where at least 30 cm of material came to rest. The angle's tangent is the engine's (`expr.eval`, R23): one call, one value.

Library code imports only viasdk and the standard library (R19b). The relaxation is plain Python
on a grid of a few tens of thousands of cells, adding and comparing doubles in a fixed order.
"""

import bisect

PARAMS_EXPLAIN = ("A param of the weather step is outside the range it takes: `repose` is an angle "
                  "in degrees above 0 and below 90, `rate` above 0 and at most 0.5.")


class ParamError(ValueError):
    """A param outside what the step accepts: its key, what is wrong, and the range it takes."""

    def __init__(self, key, message, hint):
        super().__init__(message)
        self.key, self.message, self.hint = key, message, hint


def refuse(step, e):
    """Says a bad param at its key (`step.<id>.params`), so the build names the line it is on."""
    step.error("params", f"step.{step.id}.params.{e.key}", e.message, e.hint,
               explain=PARAMS_EXPLAIN)


def checked(p):
    """The params read with their defaults: (repose, rounds, rate); ParamError when one is out."""
    repose = float(p.get("repose", 34))
    rounds = int(p.get("rounds", 60))
    rate = float(p.get("rate", 0.2))
    if not 0 < repose < 90:
        raise ParamError("repose", f"`repose` is {repose}: an angle in degrees, above 0 and below 90",
                         "write an angle between 0 and 90 degrees; loose rock rests at about 34")
    if not 0 < rate <= 0.5:
        raise ParamError("rate", f"`rate` is {rate}: above 0 and at most 0.5, or a pass overshoots",
                         "write a rate above 0 and at most 0.5; the default is 0.2")
    return repose, rounds, rate


def generate(step):
    p = step.params
    try:
        repose, rounds, rate = checked(p)
    except ParamError as e:
        refuse(step, e)
        return
    grid = step.grid
    r_mm = int(grid["resolution_mm"])
    cell_mm = int(round(float(p.get("cell", 2)) * 1000))
    k = max(1, cell_mm // r_mm)
    (h,) = step.fields(["role:surface"], view="head")
    name = h.name
    posts = h.tolist()
    coarse = sample(posts, k)
    cell_m = k * r_mm / 1000.0
    (tan,) = _list(step.eval(f"= tan({repose}) * {cell_m}", x=[grid["extent_mm"][0]],
                             y=[grid["extent_mm"][1]]))
    after = relax(coarse, float(tan), rounds, rate)
    shed_c = [[a - b for a, b in zip(ra, rb)] for ra, rb in zip(after, coarse)]
    talus_c = talus(shed_c)
    ny, nx = len(posts), len(posts[0])
    step.out.field("shed", upsample(shed_c, k, nx, ny), tags=["shed"], unit="m")
    step.out.field("talus", upsample(talus_c, k, nx, ny), tags=["talus"])
    step.out.row(name, "settle", value="= v + f('shed')")


def sample(posts, k):
    """Every k-th post in each axis, and the last row and column, as cells (rows from the south)."""
    ny, nx = len(posts), len(posts[0])
    js = _ticks(ny, k)
    is_ = _ticks(nx, k)
    return [[float(posts[j][i]) for i in is_] for j in js]


def _ticks(n, k):
    t = list(range(0, n, k))
    if t[-1] != n - 1:
        t.append(n - 1)
    return t


def relax(h, limit, rounds, rate):
    """``rounds`` passes of repose relaxation along x then y; a new grid, ``h`` untouched."""
    h = [row[:] for row in h]
    ny, nx = len(h), len(h[0])
    for _ in range(rounds):
        # along x: pairs (j, i) and (j, i + 1)
        for j in range(ny):
            row = h[j]
            moves = [_move(row[i] - row[i + 1], limit, rate) for i in range(nx - 1)]
            for i, m in enumerate(moves):
                if m:
                    row[i] -= m
                    row[i + 1] += m
        # along y: pairs (j, i) and (j + 1, i)
        moves = [[_move(h[j][i] - h[j + 1][i], limit, rate) for i in range(nx)] for j in range(ny - 1)]
        for j in range(ny - 1):
            lo, hi, mv = h[j], h[j + 1], moves[j]
            for i in range(nx):
                m = mv[i]
                if m:
                    lo[i] -= m
                    hi[i] += m
    return h


def _move(d, limit, rate):
    if d > limit:
        return rate * (d - limit)
    if d < -limit:
        return rate * (d + limit)
    return 0.0


def talus(shed):
    """Where material came to rest, 0 to 1: each rise over the 99.5th percentile of the rises."""
    gains = sorted(max(0.0, v) for row in shed for v in row)
    if not gains or gains[-1] <= 0:
        return [[0.0 for _ in row] for row in shed]
    top = gains[min(len(gains) - 1, int(0.995 * (len(gains) - 1)))]
    if top <= 0:
        top = gains[-1]
    return [[min(1.0, max(0.0, v) / top) for v in row] for row in shed]


def upsample(c, k, nx, ny):
    """The cells read at every post, bilinear between the four cells around (fields.md 1.2: along
    x on the south and north rows, then between them)."""
    xs = _ticks(nx, k)
    ys = _ticks(ny, k)
    cols = [_bracket(xs, i) for i in range(nx)]
    out = []
    for j in range(ny):
        b, fy = _bracket(ys, j)
        s_row, n_row = c[b], c[min(b + 1, len(ys) - 1)]
        row = []
        for a, fx in cols:
            a1 = min(a + 1, len(xs) - 1)
            s = (1 - fx) * s_row[a] + fx * s_row[a1]
            n = (1 - fx) * n_row[a] + fx * n_row[a1]
            row.append((1 - fy) * s + fy * n)
        out.append(row)
    return out


def _bracket(ticks, i):
    """(index of the tick at or below i, fraction to the next tick)."""
    a = bisect.bisect_right(ticks, i) - 1
    if a >= len(ticks) - 1:
        return len(ticks) - 1, 0.0
    return a, (i - ticks[a]) / (ticks[a + 1] - ticks[a])


def _list(col):
    return col.tolist() if hasattr(col, "tolist") else list(col)
