import os

import numpy as np

def generate(step):
    ny, nx = int(step.grid["ny"]), int(step.grid["nx"])
    # Written with numpy.save, not the SDK, which already writes -0.0 as +0.0.
    np.save(os.path.join(step.out.folder, "zero.npy"), np.full((ny, nx), -0.0, dtype=np.float32))
    with open(os.path.join(step.out.folder, "document.toml"), "w", encoding="utf-8") as f:
        f.write('[field.zero]\nfile = "zero.npy"\noutside = 0\n')
