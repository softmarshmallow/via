"""A lint: measure the built object and compare it with the numbers it was modelled from."""
import numpy as np

from viasdk import step

s = step()
want = s.build.doc["params"]
mesh = np.fromfile(s.build.folder / "vol_bench.verts.f32", dtype=np.float32).reshape(-1, 3)
size = mesh.max(axis=0) - mesh.min(axis=0)
cell = next(v["cell_m"] for v in s.build.doc["volumes"] if v["id"] == "bench")
for name, got, target in (("overall_length", size[0], want["overall_length"]), ("width", size[1], want["width"]), ("height", size[2], want["height"])):
    off = got - target
    say = s.warn if abs(off) > 1.5 * cell else None
    line = f"built {got:.2f}, stated {target:.2f}, off by {off:+.2f} (one cell is {cell:g})"
    print(f"  measure {name}: {line}")
    if say:
        say("off_spec", name, line)
s.done()
