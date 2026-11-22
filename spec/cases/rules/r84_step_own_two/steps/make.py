def generate(step):
    # Two instances in one returned document (protocol.md 5): `make#post` and `make#stone`, 2 m
    # boxes 0.4 m apart. The step wrote both at once, so neither is the firmer (rules.md 3.4,
    # R69): both stay, with `place.crowded`. A third, clear of them, is not named.
    step.out.instance("post", "box_small", at=(6000, 32000))
    step.out.instance("stone", "box_small", at=(6400, 32000))
    step.out.instance("far", "box_small", at=(20000, 32000))
