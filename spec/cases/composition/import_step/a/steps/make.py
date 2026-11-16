# The import's own generate step: returns a field `level` of 3 over the import's extent, and a row
# on the import's field `tone` that reads it, so `tone` is 3 where the import is built.
import numpy as np


def generate(step):
    (height,) = step.fields(["height"])
    step.out.field("level", np.full(np.shape(height), 3.0, dtype="<f4"))
    step.out.row("tone", "make", value="= f('level')")
