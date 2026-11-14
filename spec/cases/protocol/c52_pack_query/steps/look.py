def lint(step):
    got = step.pack("@plant/tree", columns=["bounds", "collision"])
    step.info("read", "step.look", "read the trees", "nothing to do",
              data={"ids": list(got["id"]), "collision": list(got["collision"])},
              explain="The trees' ids and collision, as pack.query gave them.")
