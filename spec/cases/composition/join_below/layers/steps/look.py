# A step below the import's layer: reads the surface in its stratum, without the import's ground.
import numpy as np


def generate(step):
    (seen,) = step.fields(["height"])
    step.out.field("seen", np.array(seen, dtype="<f4"))
