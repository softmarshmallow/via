def generate(step):
    # A run and an instance in one returned document (protocol.md 5): `make#row` and `make#post`.
    # The step wrote both at once, so the run does not give way to the instance (rules.md 3.4,
    # R64): `post` stands 0.4 m east of the member at 6 m, and both stay, with `place.crowded`.
    step.out.table("run", "row", asset="box_small", count=4, on={"ref": "lane"})
    step.out.instance("post", "box_small", at=(6400, 32000))
