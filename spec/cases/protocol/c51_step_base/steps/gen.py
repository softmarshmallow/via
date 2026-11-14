import numpy as np

def generate(step):
    ny, nx = int(step.grid["ny"]), int(step.grid["nx"])
    step.out.field("height", np.zeros((ny, nx), dtype=np.float32))
