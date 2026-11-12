"""A lint step: this project's own rule. Every house stands within reach of a road."""
import math

from viasdk import step

s = step()
roads = [f for f in s.build.features.values() if f.get("kind") == "road"]
for inst in s.build.explicit:
    if not inst["asset"].startswith(("house.", "town.")) or inst["asset"] == "town.well":
        continue
    x, y = inst["pos"][:2]
    near = min(math.hypot(x - px, y - py) for f in roads for px, py in f["points"])
    if near > s.params["max_m"]:
        s.warn("far_from_lane", inst["id"], f"it is {near:.0f} m from the nearest road; this project allows {s.params['max_m']:.0f} m", where=(x, y))
s.done()
