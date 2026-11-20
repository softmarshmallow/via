def generate(step):
    # The run of r75, and an instance that stands clear of it, under the world's run `fence`.
    step.out.table("run", "row", asset="box_small", count=4, on={"ref": "lane"})
    step.out.instance("stone", "box_small", at=(41400, 48000))
