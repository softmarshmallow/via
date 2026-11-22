def generate(step):
    # A floor at 13.3 and a ledge over the same plan, at `eaves`: two kind surfaces of two
    # features. At one height they are one level (rules.md 8.1, R70), so what stands on one is
    # compared with what stands on the other.
    box = [(20000, 20000), (32000, 20000), (32000, 28000), (20000, 28000)]
    step.out.feature("loft", "storey", region=box, level=13.3)
    step.out.feature("ledge", "roof", region=box, eaves=step.params["eaves"])
    # Two instances, one on each, 0.5 m apart: two of one step's document (R69).
    step.out.instance("chest", "box", at=(30000, 24000), surface="loft.floor")
    step.out.instance("urn", "box", at=(30500, 24000), surface="ledge.garden")
    # A run on the floor, 1 m inside its south edge at x = 21, 23, ... 31, and an instance on
    # the ledge 0.4 m east of its third member: the step's own run and instance (R64).
    step.out.table("run", "row", asset="box", count=6, on={"ref": "loft", "span": [0, 0.3], "offset": 1})
    step.out.instance("post", "box", at=(25400, 21000), surface="ledge.garden")
