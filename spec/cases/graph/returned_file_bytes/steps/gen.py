import os

import numpy as np


def generate(step):
    # The document text is the same at every level; only the bytes of the returned file differ.
    ny, nx = int(step.grid["ny"]), int(step.grid["nx"])
    level = float(step.params["level"])
    np.save(os.path.join(step.out.folder, "wave.npy"),
            np.full((ny, nx), level, dtype=np.float32), allow_pickle=False)
    with open(os.path.join(step.out.folder, "document.toml"), "w", encoding="utf-8") as f:
        f.write('[field.wave]\nfile = "wave.npy"\noutside = 0\n')
