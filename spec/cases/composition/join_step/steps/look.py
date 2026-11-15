# A step at a join: reads the joined surface and the import's own, returns the import's as `own`.
import numpy as np


def generate(step):
    joined, own = step.fields(["height", "a.height"])
    step.out.field("own", np.array(own, dtype="<f4"))
