def generate(step):
    # A plain command at the world's only layer: it returns a feature of kind `lift`, whose
    # effect writes `height`, a field it is handed. Its own return is no higher layer.
    step.out.feature("bump", "lift", at=[12000, 8000])
