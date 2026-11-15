import numpy as np


def generate(step):
    # The full view of `height` at the step's layer, returned as it was read, and a feature of
    # kind `lift`, whose effect writes `height` once the step's returns join the world.
    (h,) = step.fields(["height"], view="full")
    step.out.field("seen", np.array(h, dtype=np.float32))
    step.out.feature("bump", "lift", at=[12000, 8000])
