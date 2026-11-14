def lint(step):
    (h,) = step.fields(["height"])
    step.info("summed", "step." + step.id, "read height", "nothing to do",
              data={"sum": float(h.sum())}, explain="The lint read height and says its sum.")
