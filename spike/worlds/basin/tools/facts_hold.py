"""A lint step: do the stated facts still hold in the finished build?

The terrain step checked them against the model. This checks them again at the end, on the world's
own posts, after every layer above has had its say: a later layer could have built across the river
or lowered the lake's shore. One message for each fact that does not hold, with where.
"""
import numpy as np
import shapely
from scipy.ndimage import binary_dilation, map_coordinates

from viasdk import step

s = step()
fr = s.frame
height, water = s.build.field("height"), s.build.field("water")
sea = float(s.build.doc.get("params", {}).get("sea", 0.0))


def at(field, pts):
    pts = np.asarray(pts, dtype=np.float64)
    return map_coordinates(field, [(pts[:, 1] - fr.y0) / fr.resolution, (pts[:, 0] - fr.x0) / fr.resolution], order=0, mode="nearest")


lakes = {}
X, Y = fr.xy()
for f in s.build.features.values():
    if f.get("kind") != "stated_lake":
        continue
    inside = shapely.contains_xy(shapely.Polygon(f["points"]), X, Y)
    level = float(np.median(water[inside]))
    lakes[f["id"]] = (shapely.Polygon(f["points"]), level)
    stated = f.get("level")
    if stated is not None and abs(level - float(stated)) > 1.5:
        s.error("fact.lake_level", f["id"], f"it was stated at {float(stated):.1f} m and stands at {level:.1f} m", where=f["points"][0])
    dry = inside & (water <= height)
    if dry.mean() > 0.05 * inside.mean():
        iy, ix = np.argwhere(dry)[0]
        s.error("fact.lake_dry", f["id"], f"{dry.sum() / inside.sum():.0%} of it is dry land",
                where=(fr.x0 + ix * fr.resolution, fr.y0 + iy * fr.resolution))
    ring = binary_dilation(inside, iterations=3) & ~inside
    low = ring & (height < level - 0.4) & (water < level - 0.4)
    if low.sum() > 0.10 * ring.sum():
        iy, ix = np.unravel_index(np.argmin(np.where(low, height, np.inf)), height.shape)
        s.error("fact.lake_drains", f["id"], f"{low.sum() / ring.sum():.0%} of its shore is below its surface: it would run out",
                where=(fr.x0 + ix * fr.resolution, fr.y0 + iy * fr.resolution))

for f in s.build.features.values():
    if f.get("kind") != "stated_river":
        continue
    course = s.build.features.get(f["id"] + "_course")          # where the terrain step found the water runs
    if course is None:
        continue                                                # no water ran at all; the terrain step has said why
    pts = np.asarray(course["points"], dtype=np.float64)
    seg = np.hypot(*np.diff(pts, axis=0).T)
    along = np.concatenate([[0.0], np.cumsum(seg)])
    t = np.arange(0.0, along[-1], 5.0)
    line = np.stack([np.interp(t, along, pts[:, 0]), np.interp(t, along, pts[:, 1])], axis=1)
    w, h = at(water, line), at(height, line)
    in_lake = np.array([any(shapely.contains_xy(poly, x, y) for poly, _ in lakes.values()) for x, y in line])
    dry = (w <= h) & ~in_lake
    if dry.mean() > 0.05:
        i = int(np.argmax(dry))
        s.error("fact.river_broken", f["id"], f"{dry.mean():.0%} of its course carries no water; first at ({line[i][0]:.0f}, {line[i][1]:.0f})",
                where=line[i], hint="something above the terrain step has built across it")
    surface = np.where(w > h, w, h)
    rise = surface[1:] - np.minimum.accumulate(surface)[:-1]
    i = int(np.argmax(rise))
    if rise[i] > 0.6:
        s.error("fact.river_uphill", f["id"], f"its surface rises {rise[i]:.1f} m going downstream at ({line[i + 1][0]:.0f}, {line[i + 1][1]:.0f})",
                where=line[i + 1])
    if surface[-1] > sea + 1.0 and not in_lake[-1]:
        s.error("fact.river_end", f["id"], f"it ends {surface[-1] - sea:.1f} m above the sea, at ({line[-1][0]:.0f}, {line[-1][1]:.0f}), and not in a lake",
                where=line[-1])
s.done()
