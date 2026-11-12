"""A rule of this project's own: every home stands within reach of a road."""
import math

from viasdk import step

s = step()
p = s.params
roads = [f for f in s.build.features.values() if f.get("kind") == p["road_kind"] and f.get("points")]
for inst in s.build.explicit:
    if inst["asset"] not in p["assets"]:
        continue
    x, y = inst["pos"][:2]
    near = min((math.hypot(x - px, y - py), f["id"]) for f in roads for px, py in f["points"]) if roads else (math.inf, "")
    if near[0] > p["max_m"]:
        s.warn("far_from_road", inst["id"], f"it is {near[0]:.0f} m from the nearest road ({near[1] or 'there is none'}); "
               f"this project allows {p['max_m']:.0f} m", where=(x, y), hint="move it, or bring a road to it")
s.done()
