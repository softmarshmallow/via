# The import's own generate step: returns a field tone of 3 over the import's extent.
import numpy as np


def generate(step):
    (height,) = step.fields(["height"])
    step.out.field("tone", np.full(np.shape(height), 3.0, dtype="<f4"))
