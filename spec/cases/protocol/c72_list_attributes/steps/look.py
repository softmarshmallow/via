def lint(step):
    fl = step.features(["a", "b"], columns=["doors", "door", "sizes"])
    got = {c: list(fl[c]) for c in ("doors", "doors?", "door", "sizes")}
    got["doors?"] = [int(v) for v in got["doors?"]]
    step.info("read", "step.look", "read the list and table attributes", "nothing to do",
              data=got, explain="Each list and table attribute as build.features gave it.")
