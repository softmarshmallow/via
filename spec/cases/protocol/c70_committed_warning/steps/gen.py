def generate(step):
    # A worker: reads the ground, says the spot is dry, and returns it.
    step.fields(["height"])
    step.warning("dry", "world", "the spot holds no water", "state a hollow",
                 explain="A spot the step could not wet.")
    step.out.feature("spot", "mark", at=[3000, 4000])
