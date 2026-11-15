def generate(step):
    # Offers two places, each with a pad, and predicts nothing about the ground.
    step.fields(["height"])
    step.out.instance("small", "hut", at=[12000, 8000], ground="pad")
    step.out.instance("big", "hall", at=[32000, 8000], ground="pad")
