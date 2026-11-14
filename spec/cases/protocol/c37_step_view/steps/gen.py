def generate(step):
    step.fields(["height"], view="full")
    step.out.row("height", "r", value="= v + 1")
