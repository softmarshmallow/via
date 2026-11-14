"""The vegetation library's clumps step: things placed in groups, by a step (R5, R19b).

A generate step, run as a worker through the SDK; its `params` are lib/vegetation/kinds.toml's.
It does by a step what a scatter's `clump` does in core (rules.md 7.3), to show that a library
reaches no further than a user's step: the one thing it needs the engine for is each formula's value
at many points, asked in one batch (`expr.eval`, R23), and the asset list (`pack.query`).

How it places, in order:

1. **Parents.** One candidate per cell of a grid at `spacing` over the world (or over the region
   `within`), at the cell's corner moved by a draw in each axis, `0 <= u < 1` of the cell. Cell
   `(i, j)` covers `[x0 + i * spacing, x0 + (i + 1) * spacing)` and likewise in y.
2. **Chance.** `density` and the world's `keep_out` are evaluated at every candidate in one call:
   the chance is `clamp(density, 0, 1) * (1 - keep_out)`, times `max(0, 1 - d / radius)` with
   `around`, `d` the distance to the nearest instance of that producer. A parent is kept when its
   `keep` draw is below its chance.
3. **Members.** Each kept parent draws its count in `members`, then each member a point in the disc
   of `radius` around the parent (a draw in the square, kept when inside the disc: plain arithmetic,
   no trigonometry, which keeps a rerun stable; the step's result is recorded, not promised,
   R26). A member is kept when its point lies in the world and `within`, and its own `keep` draw is below `1 - keep_out` there (one more
   batch for every member). Its scale is its draw times `1 - shrink * (distance / radius)`: the
   group thins and shrinks outward.
4. **Assets, facing, scale.** Each parent draws its asset by weight, and its members take it (a
   grove of one tree, as rules.md 7.3's members); each thing draws its facing in whole
   millidegrees and its scale in `scale`.
5. **Clear of `keep_out`.** A thing whose footprint, taken at its widest, meets a post where
   `keep_out` is above 0 is dropped (rules.md 8.3), so the step returns no `place.keep_out`.

Draws are `blake2b` of the step's seed (protocol.md 2: the world seed and the step's id) and the
thing's key, so a draw belongs to a place: moving the cause of one parent changes that parent and
its members, nothing else. Keys are `identity.md` 2.2's place keys (kinds.toml, the header).

The step returns instances only; the core grounds them on the surface and applies holds to them
(protocol.md 5, `identity.md` 4.1). Library code imports only viasdk and the standard library (R19b).
"""

import hashlib
import math

from viasdk import RpcError

PARAMS_EXPLAIN = ("A param of the clumps step is outside what it takes: `assets` names assets or "
                  "taxa the pack files, `spacing` is above 0, `gap` is 0 or more, `shrink` is from 0 "
                  "to below 1, `members` is low first, `within` names one region feature.")


class ParamError(ValueError):
    """A param outside what the step accepts: its key, what is wrong, and the range it takes."""

    def __init__(self, key, message, hint):
        super().__init__(message)
        self.key, self.message, self.hint = key, message, hint


def refuse(step, e):
    """Says a bad param at its key (`step.<id>.params`), so the build names the line it is on."""
    step.error("params", f"step.{step.id}.params.{e.key}", e.message, e.hint,
               explain=PARAMS_EXPLAIN)


EXPLAIN = {
    "empty": "The clumps step kept no parent: every candidate's chance (density times one less "
    "keep_out, times nearness with `around`) lost its draw, or no candidate lay `within`.",
}


def generate(step):
    try:
        p = Params(step.params, step.grid)
        weights = _assets(step, p.assets)
        within = _region(step, p.within) if p.within else None
    except ParamError as e:
        refuse(step, e)
        return
    grid = step.grid
    keep_out = _field_name(step, "role:keep_out")
    cands = candidates(p, grid, step.seed, within)
    if not cands:
        _empty(step, p)
        return
    formula = f"= clamp({_body(p.density)}, 0, 1)"
    if keep_out is not None:
        formula = f"= clamp({_body(p.density)}, 0, 1) * (1 - f('{keep_out}'))"
    chance = _list(step.eval(formula, x=[c["x"] for c in cands], y=[c["y"] for c in cands]))
    if p.around is not None:
        t = step.instances(select=p.around["of"])
        near = Near([(int(x), int(y)) for x, y in zip(_list(t["x"]), _list(t["y"]))],
                    int(p.around["radius"] * 1000))
        chance = [c * near.factor((k["x"], k["y"])) for c, k in zip(chance, cands)]
    parents = [c for c, ch in zip(cands, chance) if c["keep"] < ch]
    if not parents:
        _empty(step, p)
        return
    members = []
    for par in parents:
        members.extend(group(p, grid, step.seed, par, within))
    if members and keep_out is not None:
        ko = _list(step.eval(f"= f('{keep_out}')", x=[m["x"] for m in members],
                             y=[m["y"] for m in members]))
        members = [m for m, k in zip(members, ko) if m["keep"] < 1 - k]
    asset_of = {par["key"]: pick(weights, draw(step.seed, par["key"], "asset")) for par in parents}
    # Give way (rules.md 7.5, as the core's clump does): parents first, then members, each in key
    # order; a thing within `gap` metres of one already kept is dropped, so no two trunks overlap
    # (a returned instance that overlaps another is place.overlap, an error).
    kept = []
    gap2 = int(round(p.gap * 1000)) ** 2
    for thing in sorted(parents, key=lambda r: r["key"]) + sorted(members, key=lambda r: r["key"]):
        if any((thing["x"] - k["x"]) ** 2 + (thing["y"] - k["y"]) ** 2 < gap2 for k in kept):
            continue
        kept.append(thing)
    if kept and keep_out is not None:
        kept = clear_of(step, kept, asset_of, keep_out, grid)
    for thing in sorted(kept, key=lambda r: r["key"]):
        asset = asset_of[thing.get("parent", thing["key"])]
        step.out.instance(thing["key"], asset, (thing["x"], thing["y"]),
                          face=round(draw(step.seed, thing["key"], "face") * 360000) / 1000,
                          scale=thing["scale"])


def clear_of(step, things, asset_of, keep_out, grid):
    """Step 5: what the core would find in `keep_out` (rules.md 8.3: above 0 at its origin or at
    any post under its footprint) is dropped, since a step's instance there is `place.keep_out`.
    The footprint is taken at its widest, the square of the asset's largest plan bound times the
    thing's scale, whatever its facing; so a thing near `keep_out` may be dropped that would have
    cleared it. One `expr.eval` batch for every post of every square."""
    ids = sorted({asset_of[t.get("parent", t["key"])] for t in things})
    tab = step.pack(select=ids, columns=["bounds"])
    reach = {}
    for i, b in zip(_list(tab["id"]), _list(tab["bounds"])):
        b = _list(b)
        reach[i] = max(abs(int(v)) for v in (b[0], b[1], b[3], b[4]))
    res = int(grid["resolution_mm"])
    x0, y0, x1, y1 = (int(v) for v in grid["extent_mm"])
    xs, ys, owner = [], [], []
    for n, t in enumerate(things):
        r = int(math.ceil(reach.get(asset_of[t.get("parent", t["key"])], 0) * t["scale"]))
        xs.append(t["x"])
        ys.append(t["y"])
        owner.append(n)
        for j in range(-(-(t["y"] - r - y0) // res), (t["y"] + r - y0) // res + 1):
            for i in range(-(-(t["x"] - r - x0) // res), (t["x"] + r - x0) // res + 1):
                px, py = x0 + i * res, y0 + j * res
                if x0 <= px <= x1 and y0 <= py <= y1:
                    xs.append(px)
                    ys.append(py)
                    owner.append(n)
    ko = _list(step.eval(f"= f('{keep_out}')", x=xs, y=ys))
    blocked = {o for o, k in zip(owner, ko) if k > 0}
    return [t for n, t in enumerate(things) if n not in blocked]


class Params:
    """The step's params, read once with their defaults (lib/vegetation/kinds.toml)."""

    def __init__(self, params, grid):
        self.assets = params.get("assets") or []
        if not self.assets:
            raise ParamError("assets", "`assets` names nothing to place",
                             "list asset ids or taxa, such as [\"@plant/tree\"]")
        self.spacing = float(params["spacing"])
        if self.spacing <= 0:
            raise ParamError("spacing", f"`spacing` is {self.spacing}: metres, above 0",
                             "write the spacing between parents in metres, above 0")
        self.density = str(params.get("density", "= 1"))
        m = params.get("members", 0)
        self.members = (int(m[0]), int(m[1])) if isinstance(m, list) else (int(m), int(m))
        self.radius = float(params.get("radius", self.spacing / 2))
        self.shrink = float(params.get("shrink", 0))
        s = params.get("scale", [1, 1])
        self.scale = (float(s[0]), float(s[1]))
        self.around = params.get("around")
        self.within = params.get("within")
        self.gap = float(params.get("gap", 2.4))
        if self.gap < 0:
            raise ParamError("gap", f"`gap` is {self.gap}: metres, 0 or more",
                             "write the gap between crowns in metres, 0 or more")
        if not 0 <= self.shrink < 1:
            raise ParamError("shrink", f"`shrink` is {self.shrink}: 0 or more and below 1",
                             "write a shrink from 0 up to, not including, 1")
        if self.members[0] < 0 or self.members[1] < self.members[0]:
            raise ParamError("members", f"`members` is {list(self.members)}: 0 or more, low first",
                             "write `members` as one count or [low, high], low first")


def draw(seed, key, what):
    """A draw in [0, 1) for one thing's one choice: the first 53 bits of a digest."""
    h = hashlib.blake2b(f"{seed}/{key}/{what}".encode("utf-8"), digest_size=8).digest()
    return (int.from_bytes(h, "big") >> 11) / float(1 << 53)


def candidates(p, grid, seed, within=None):
    """Step 1: one candidate per grid cell, as dicts with key, x, y (mm), keep and scale."""
    x0, y0, x1, y1 = (int(v) for v in grid["extent_mm"])
    sp = int(round(p.spacing * 1000))
    out = []
    for j in range(-(-(y1 - y0) // sp)):
        for i in range(-(-(x1 - x0) // sp)):
            key = f"s{sp}_{i}_{j}"
            x = x0 + i * sp + int(draw(seed, key, "x") * sp)
            y = y0 + j * sp + int(draw(seed, key, "y") * sp)
            if x > x1 or y > y1:
                continue
            if within is not None and not _in_region((x, y), within):
                continue
            out.append({"key": key, "x": x, "y": y, "keep": draw(seed, key, "keep"),
                        "scale": _scale(p, draw(seed, key, "scale"), 0.0)})
    return out


def group(p, grid, seed, parent, within=None):
    """Step 3: the members of one kept parent, before their keep_out test."""
    x0, y0, x1, y1 = (int(v) for v in grid["extent_mm"])
    lo, hi = p.members
    n = lo + int(draw(seed, parent["key"], "members") * (hi - lo + 1))
    r = int(round(p.radius * 1000))
    out = []
    for m in range(n):
        key = f"{parent['key']}_m{m}"
        for tries in range(16):
            u = 2 * draw(seed, key, f"u{tries}") - 1
            v = 2 * draw(seed, key, f"v{tries}") - 1
            if u * u + v * v <= 1:
                break
        else:
            u = v = 0.0
        x = parent["x"] + int(round(u * r))
        y = parent["y"] + int(round(v * r))
        if not (x0 <= x <= x1 and y0 <= y <= y1):
            continue
        if within is not None and not _in_region((x, y), within):
            continue
        out.append({"key": key, "parent": parent["key"], "x": x, "y": y,
                    "keep": draw(seed, key, "keep"),
                    "scale": _scale(p, draw(seed, key, "scale"), math.sqrt(u * u + v * v))})
    return out


def _scale(p, u, out_frac):
    s = p.scale[0] + u * (p.scale[1] - p.scale[0])
    return round(s * (1 - p.shrink * out_frac), 3)


def pick(weights, u):
    """The asset whose share of the summed weights holds ``u``, in the order given."""
    total = sum(w for _, w in weights)
    at = u * total
    for asset, w in weights:
        if at < w:
            return asset
        at -= w
    return weights[-1][0]


class Near:
    """Nearness to a producer's instances: ``max(0, 1 - d / radius)``, d to the nearest."""

    def __init__(self, points, radius_mm):
        self.r = max(1, radius_mm)
        self.cells = {}
        for pt in points:
            self.cells.setdefault((pt[0] // self.r, pt[1] // self.r), []).append(pt)

    def factor(self, q):
        ci, cj = q[0] // self.r, q[1] // self.r
        best = None
        for di in (-1, 0, 1):
            for dj in (-1, 0, 1):
                for pt in self.cells.get((ci + di, cj + dj), ()):
                    d2 = (pt[0] - q[0]) ** 2 + (pt[1] - q[1]) ** 2
                    if best is None or d2 < best:
                        best = d2
        if best is None:
            return 0.0
        return max(0.0, 1.0 - math.sqrt(best) / self.r)


def _assets(step, entries):
    """(asset id, weight) pairs, a taxon's assets sharing its weight (rules.md 7.4)."""
    out = []
    for e in entries:
        ident, w = (e["id"], float(e.get("weight", 1))) if isinstance(e, dict) else (e, 1.0)
        if ident.startswith("@"):
            ids = sorted(_list(step.pack(select=ident, columns=())["id"]))
            if not ids:
                raise ParamError("assets", f"`{ident}` files no asset in the pack",
                                 "name a taxon the pack files assets under, or an asset id")
            out.extend((i, w / len(ids)) for i in ids)
        else:
            out.append((ident, w))
    return out


def _field_name(step, selector):
    """The build's name of the one field ``selector`` selects, or None when there is none."""
    try:
        (a,) = step.fields([selector], window=[0, 0, 1, 1])
    except (LookupError, RpcError):
        return None
    return a.name


def _region(step, name):
    fl = step.features(select=[name])
    if len(fl) != 1 or int(_list(fl["geometry"])[0]) != 2:
        raise ParamError("within", f"`within` names `{name}`, which is not one region feature",
                         "name one feature whose shape is a region")
    return fl.rings(0)


def _in_region(p, pieces):
    return any(_in_piece(p, [[tuple(int(v) for v in pt) for pt in _list(r)] for r in piece])
               for piece in pieces)


def _in_piece(p, rings):
    if not _in_ring(p, rings[0]):
        return False
    return not any(_in_ring(p, h) for h in rings[1:])


def _in_ring(p, ring):
    wn = 0
    n = len(ring)
    for k in range(n):
        a, b = ring[k], ring[(k + 1) % n]
        cross = (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0])
        if a[1] <= p[1] < b[1] and cross > 0:
            wn += 1
        elif b[1] <= p[1] < a[1] and cross < 0:
            wn -= 1
    return wn != 0


def _body(formula):
    f = formula.strip()
    return f[1:].strip() if f.startswith("=") else f


def _list(col):
    return col.tolist() if hasattr(col, "tolist") else list(col)


def _empty(step, p):
    step.warning("empty", "step." + step.id, "No parent was kept.",
                 "Raise `density`, lower `spacing`, or check `within` and `around`.",
                 data={"spacing": p.spacing}, explain=EXPLAIN["empty"])
