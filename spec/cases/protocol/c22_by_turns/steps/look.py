def generate(step):
    (h,) = step.fields(["height"])
    step.info("read", "step." + step.id, "read height", "nothing to do",
              data={"sum": float(h.sum())}, explain="The step read height and says its sum.")
