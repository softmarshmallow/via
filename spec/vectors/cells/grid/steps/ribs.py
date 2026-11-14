"""Returns one path of kind `rib` per line of the grid (geometry.md 8.4).

Each rib is keyed by where it stands (`x20`, `y70`), never by a counter (identity.md 2.2), so a
distant edit renames nothing. With `bend`, the northmost rib gains a vertex at x = xs[0] + 25 m,
lifted by `bend` metres: the shape moves and every crossing stays.
"""


def mm(v):
    return int(round(float(v) * 1000))


def generate(step):
    p = step.params
    xs = [mm(v) for v in p["xs"]]
    ys = [mm(v) for v in p["ys"]]
    lo, hi, bend = mm(p["lo"]), mm(p["hi"]), mm(p.get("bend", 0))
    for x in xs:
        step.out.feature("x%d" % (x // 1000), "rib", path=[(x, lo), (x, hi)])
    top = max(ys)
    for y in ys:
        path = [(lo, y), (hi, y)]
        if y == top and bend:
            path = [(lo, y), (xs[0] + 25000, y + bend), (hi, y)]
        step.out.feature("y%d" % (y // 1000), "rib", path=path)
