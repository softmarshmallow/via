"""Returns `spread`, 1 everywhere, tagged `fringe`: the field the scatter's density reads."""

import numpy as np


def generate(step):
    ny, nx = int(step.grid["ny"]), int(step.grid["nx"])
    step.out.field("spread", np.ones((ny, nx), dtype="<f4"), tags=["fringe"])
