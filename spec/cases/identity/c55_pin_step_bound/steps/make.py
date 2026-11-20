def generate(step):
    # A flat tread at 12, clear of the ground, `width` wide, and a box on it 0.4 m off its centre
    # line: at width 1 the box reaches 0.4 m past the bound (y 23.9 to 24.9, the bound to 24.5).
    step.out.feature("ramp", "flight", path=[(20000, 24000), (30000, 24000)],
                     z0=12, z1=12, width=step.params["width"])
    step.out.instance("pot", "box", at=(25000, 24400), surface="ramp.tread")
