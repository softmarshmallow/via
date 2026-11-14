import numpy as np

def generate(step):
    ny, nx = int(step.grid["ny"]), int(step.grid["nx"])
    j, i = np.mgrid[0:ny, 0:nx]
    step.out.field("wave", (i + 0.5 * j).astype(np.float32), tags=["wave"])
    step.out.row("height", "w", value="= f('wave')")
