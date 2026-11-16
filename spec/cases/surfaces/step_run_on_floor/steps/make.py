def generate(step):
    # A storey and a run `on` it, in one returned document (protocol.md 5, Names): the run is
    # `make#edge`, so its members are `make#edge#r<k>`, and each stands on the storey's floor.
    step.out.feature("loft", "storey", region=[(20000, 20000), (32000, 20000), (32000, 28000), (20000, 28000)],
                     level=13.3)
    step.out.table("run", "edge", asset="box", count=3, ends="both",
                   on={"ref": "loft", "span": [0, 0.25], "offset": 1})
