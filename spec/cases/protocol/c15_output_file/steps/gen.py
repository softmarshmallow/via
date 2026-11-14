import json
import os

import numpy as np

def write(step, array):
    np.save(os.path.join(step.out.folder, "wave.npy"), array, allow_pickle=False)

def generate(step):
    ny, nx = int(step.grid["ny"]), int(step.grid["nx"])
    mode = step.params["mode"]
    name = "wave.npy"
    if mode == "escape":
        name = "../x.npy"
    elif mode == "f8":
        write(step, np.zeros((ny, nx), dtype=np.float64))
    elif mode == "short":
        write(step, np.zeros((ny - 1, nx), dtype=np.float32))
    elif mode == "big":
        write(step, np.zeros((ny, nx), dtype=">f4"))
    elif mode == "fortran":
        write(step, np.asfortranarray(np.zeros((ny, nx), dtype=np.float32) + np.arange(nx, dtype=np.float32)))
    elif mode == "v2":
        with open(os.path.join(step.out.folder, "wave.npy"), "wb") as f:
            np.lib.format.write_array(f, np.zeros((ny, nx), dtype=np.float32), version=(2, 0))
    elif mode == "magic":
        with open(os.path.join(step.out.folder, "wave.npy"), "wb") as f:
            f.write(b"\x93NUMPX" + bytes(120))
    elif mode == "good":
        write(step, np.zeros((ny, nx), dtype=np.float32))
    with open(os.path.join(step.out.folder, "document.toml"), "w", encoding="utf-8") as f:
        f.write("[field.wave]\nfile = %s\noutside = 0\n" % json.dumps(name))
