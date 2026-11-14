def generate(step):
    got = step.features(select="kind:mark")
    step.info("count", "step.gen", "read kind:mark", "nothing to do", data={"names": got.names},
              explain="The names a kind selection gave the step.")
