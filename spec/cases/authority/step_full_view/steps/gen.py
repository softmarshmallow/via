import numpy as np


def generate(step):
    # The full view of `height` at the step's layer (rank 0), returned as it was read.
    (h,) = step.fields(["height"], view="full")
    step.out.field("seen", np.array(h, dtype=np.float32))
