"""A generate step: terraced garden plots below the downhill edge of a host region.

It reads the host's outline from the build beneath it and the height there, finds the downhill side,
and lays `count` strips parallel to that edge. Move or resize the court and the plots follow.
"""
import numpy as np

from viasdk import step

s = step()
p = s.params
host = s.build.features[p["host"]]
pts = np.array(host["points"], float)
x0, y0 = pts.min(axis=0)
x1, y1 = pts.max(axis=0)
h = s.build.field("height")


def ground(x, y):
    return float(h[s.frame.index(x, y)])


# the downhill side is south if the ground falls to the south of the box, else north
south = ground((x0 + x1) / 2, y0 - 30) < ground((x0 + x1) / 2, y1 + 30)
sign = -1.0 if south else 1.0
edge = y0 if south else y1
a, b = x0 + p["inset_m"], x1 - p["inset_m"]
for k in range(int(p["count"])):
    near = edge + sign * (p["clear_m"] + k * (p["depth_m"] + p["gap_m"]))
    far = near + sign * p["depth_m"]
    lo, hi = min(near, far), max(near, far)
    if lo < s.frame.y0 + 4 or hi > s.frame.y1 - 4:
        s.warn("no_room", f"garden_{k}", "the plot would leave the map; it is left out", where=((a + b) / 2, near))
        continue
    n = int(p.get("per_row", 1))
    w = (b - a - (n - 1) * p["gap_m"]) / n
    for j in range(n):
        xa = a + j * (w + p["gap_m"])
        s.add("feature", {
            "id": f"garden_{k}_{j}", "kind": "garden_terrace", "crop": ["beans", "barley", "kale"][(k + j) % 3],
            "region": [[xa, lo], [xa + w, lo], [xa + w, hi], [xa, hi]],
        })
s.done()
