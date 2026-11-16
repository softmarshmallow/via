def generate(step):
    at = step.surfaces_at([4000], [4000])
    step.info("asked", "step.look", "asked surfaces_at", "nothing to do",
              data={"surface": list(at["surface"]), "z": [float(v) for v in at["z"]]},
              explain="The surfaces the engine listed at [4, 4].")
