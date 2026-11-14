def generate(step):
    got = step.route("= 1 + 0 * f('height')", [[(0, 0), (8000, 0)]])
    step.info("course", "step.gen", "found a course", "nothing to do", data={"cost": float(got["cost"][0])},
              explain="The cost of the course the step found.")
