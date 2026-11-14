import numpy as np


def generate(step):
    # Reads height, returns a point where it is highest and a field of half its height.
    (h,) = step.fields(["height"])
    j, i = np.unravel_index(int(np.argmax(h)), h.shape)
    step.out.feature("top", "mark", at=[int(i) * 1000, int(j) * 1000])
    step.out.field("half", (h / 2).astype(np.float32))
