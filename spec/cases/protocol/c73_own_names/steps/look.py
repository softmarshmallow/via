def lint(step):
    m = step.meetings()
    fl = step.features(["make#t"], columns=["host"])
    step.info("read", "step.look", "read the meetings and the tag's host", "nothing to do",
              data={"a": list(m["a"]), "b": list(m["b"]), "gap": [int(g) for g in m["gap"]],
                    "host": list(fl["host"])},
              explain="The meetings and a name attribute, as the step returned them.")
