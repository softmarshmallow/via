def generate(step):
    # Reads height at one post and returns a point `east` metres east of it, as far north as
    # the height there in millimetres.
    (h,) = step.fields(["height"], window=[1, 1, 2, 2])
    step.out.feature("spot", "mark", at=[(1 + int(step.params["east"])) * 1000,
                                         int(round(float(h[0, 0]) * 1000))])
