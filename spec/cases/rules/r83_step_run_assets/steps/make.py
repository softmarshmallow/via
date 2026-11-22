def generate(step):
    # A run with a list of assets in a returned document (protocol.md 5; rules.md 5, R68): the
    # step protocol passes the table through, so `make#row` takes `assets` as a run in the
    # world's text does.
    step.out.table("run", "row", assets=["box_small", "lamp_post", "fence_post"], on={"ref": "lane"})
