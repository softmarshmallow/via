"""Places one `shed_gable` in each cell of `plots` (geometry.md 8.3, identity.md 25).

It reads the cells through `build.features` and cuts none itself (R19b, R23). Each instance is
keyed from its cell's place key (`plots#c<digest>_<station>` gives `h<digest>_<station>`), so it
keeps its name exactly when its cell does. It stands at the mean of its cell's outline vertices,
inside every cell of the grid, which are convex.
"""


def generate(step):
    cells = step.features(select=["plots"], columns=[])
    cls = cells.get("class")
    for f, name in enumerate(cells.names):
        if cls is not None:
            if str(cls[f]) != "cell":
                continue
        elif "#c" not in name:
            continue
        ring = cells.rings(f)[0][0]
        n = len(ring)
        x = sum(int(v[0]) for v in ring) // n
        y = sum(int(v[1]) for v in ring) // n
        key = "h" + name.split("#", 1)[1][1:]
        step.out.instance(key, "shed_gable", at=(x, y))
