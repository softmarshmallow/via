def generate(step):
    step.out.feature("k" if step.id == "a" else "j", "mark", at=(1000, 1000))
