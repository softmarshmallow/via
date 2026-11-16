def generate(step):
    # A slab over [2, 6] x [2, 6] at the given level.
    p = step.params
    step.out.feature(p["key"], "slab", region=[[2000, 2000], [6000, 2000], [6000, 6000], [2000, 6000]],
                     level=p["level"])
