def lint(step):
    f = step.features("kind:mark", columns=["name"])
    step.info("counted", "step." + step.id, "counted the marks", "nothing to do",
              data={"count": len(f.names)}, explain="The lint counts the features of kind mark.")
