"""Weathering, after the terrain layer: slopes steeper than the angle of repose shed material downhill.

It is handed the terrain layer's build, so the height it reads already has the road cut and the
house platforms in it. It hands back the height, changed, and `talus`: where the shed material
came to rest. The compiler then applies the road and the platforms again on the new ground.
"""
import numpy as np

from viasdk import step

s = step()
p = s.params
h = s.build.field("height").astype(np.float64)
before = h.copy()
limit = np.tan(np.radians(float(p.get("repose_deg", 34.0)))) * s.frame.resolution

for _ in range(int(p.get("rounds", 60))):
    for axis in (0, 1):
        a = [slice(None)] * 2
        b = [slice(None)] * 2
        a[axis], b[axis] = slice(0, -1), slice(1, None)
        a, b = tuple(a), tuple(b)
        d = h[a] - h[b]
        move = 0.2 * np.sign(d) * np.clip(np.abs(d) - limit, 0, None)
        h[a] -= move
        h[b] += move

gained = np.clip(h - before, 0, None)
talus = np.clip(gained / max(float(np.percentile(gained, 99.5)), 1e-6), 0, 1)
s.write_field("height", h)
s.write_field("talus", talus)
s.done()
