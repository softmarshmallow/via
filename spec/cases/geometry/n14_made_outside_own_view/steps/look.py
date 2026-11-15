def generate(step):
    # Says what `gap` is at the post (30, 30), inside the west area, in its own view.
    (g,) = step.fields(["gap"])
    i0, j0 = g.window[0], g.window[1]
    step.info("gap", "step." + step.id, "read gap", "nothing to do",
              data={"at": float(g[30 - j0, 30 - i0])}, explain="The step says gap at (30, 30).")
