"""A generate step: the land, simulated under the facts the author stated.

It is handed the build of the facts layer: the starting surface (the `height` field) and the
stated features (`stated_lake`, `stated_river`). It gives those to the Rust terrain stage of this
repository as its conditions:

    the starting surface, with       ->  --initial   the surface the model begins from
      a hollow under each lake
      a falling groove along each river
    no uplift                        ->  --uplift    zeros: the land is only worn down, never raised
    each lake's shore, at its level  ->  --held      cells the model may not change, where water ends
    the land's edge, as it lies

The model floods the hollow to its held shore, sends water down the groove, and cuts valleys and
tributaries toward both. This script then follows the model's own water from each river's source
and measures each lake, and says where a stated fact does not hold. It writes `height`, `water`,
`wet` (how much water passes a place), `damp` (the valley floors) and `silt` (where sediment
settled), and hands back each river's course as a feature for the layers above.

Nothing here is the compiler's. The Rust side is run as a command; only rasters of floats cross.
"""
import heapq
import json
import struct
import subprocess
import time
from pathlib import Path

import numpy as np
import shapely
from scipy.ndimage import binary_dilation, distance_transform_edt, gaussian_filter, map_coordinates, minimum_filter

from viasdk import step

s = step()
p = s.params
fr = s.frame
res = fr.resolution
cell = float(p["cell"])
pad = int(p["margin"])
sea = float(s.build.doc.get("params", {}).get("sea", 0.0))
n = int(round((fr.x1 - fr.x0) / cell))
if abs(n * cell - (fr.x1 - fr.x0)) > 1e-6 or abs((fr.y1 - fr.y0) - (fr.x1 - fr.x0)) > 1e-6:
    s.error("terrain.frame", s.id, f"the world must be square and a whole number of {cell} m cells across")
    s.done()
    raise SystemExit(0)
size = n + 2 * pad


def smoothstep(a, b, x):
    t = np.clip((np.asarray(x, dtype=np.float64) - a) / (b - a), 0.0, 1.0)
    return t * t * (3 - 2 * t)


def cell_of(x, y):
    """The model cell holding a world position, as (row, column) in the padded grid."""
    return int(np.floor((y - fr.y0) / cell)) + pad, int(np.floor((x - fr.x0) / cell)) + pad


def world_of(iy, ix):
    return fr.x0 + (ix - pad + 0.5) * cell, fr.y0 + (iy - pad + 0.5) * cell


# ---- the starting surface, on the model's grid (beyond the world's edge it carries on level)
start_posts = s.build.field("height").astype(np.float64)
cy, cx = np.mgrid[0:size, 0:size].astype(np.float64)
h0 = map_coordinates(start_posts, [((cy - pad + 0.5) * cell) / res, ((cx - pad + 0.5) * cell) / res], order=1, mode="nearest")
X = fr.x0 + (cx - pad + 0.5) * cell
Y = fr.y0 + (cy - pad + 0.5) * cell


def ground_at(pts):
    """The starting surface at world positions."""
    pts = np.asarray(pts, dtype=np.float64)
    return map_coordinates(start_posts, [(pts[:, 1] - fr.y0) / res, (pts[:, 0] - fr.x0) / res], order=1, mode="nearest")


def least_cost(a, b):
    """A course from a to b over the starting surface that a river could take: it prefers to go
    down, pays heavily to climb, and so goes round high ground instead of through it."""
    (ay, ax), (by, bx) = cell_of(*a), cell_of(*b)
    best = np.full((size, size), np.inf)
    back = {}
    best[ay, ax] = 0.0
    heap = [(0.0, ay, ax)]
    while heap:
        d, y, x = heapq.heappop(heap)
        if (y, x) == (by, bx):
            break
        if d > best[y, x]:
            continue
        for dy, dx in ((1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (1, -1), (-1, 1), (-1, -1)):
            yy, xx = y + dy, x + dx
            if not (pad <= yy < size - pad and pad <= xx < size - pad):
                continue
            run = cell * (1.4142 if dy and dx else 1.0)
            climb = max(h0[yy, xx] - h0[y, x], 0.0)
            nd = d + run * (1.0 + 0.02 * max(h0[yy, xx] - sea, 0.0)) + 60.0 * climb
            if nd < best[yy, xx]:
                best[yy, xx] = nd
                back[(yy, xx)] = (y, x)
                heapq.heappush(heap, (nd, yy, xx))
    course, at = [], (by, bx)
    while at != (ay, ax):
        course.append(world_of(*at))
        at = back[at]
    course.append(tuple(a))
    course = np.asarray(course[::-1])
    course[-1] = b
    k = 4                                                    # ease the staircase of an 8-way walk
    sm = course.copy()
    for i in range(1, len(course) - 1):
        sm[i] = course[max(0, i - k):i + k + 1].mean(axis=0)
    return sm


def resample(pts, every):
    pts = np.asarray(pts, dtype=np.float64)
    seg = np.hypot(*np.diff(pts, axis=0).T)
    along = np.concatenate([[0.0], np.cumsum(seg)])
    t = np.linspace(0.0, along[-1], max(int(along[-1] / every), 2))
    return np.stack([np.interp(t, along, pts[:, 0]), np.interp(t, along, pts[:, 1])], axis=1), t / along[-1]


# ---- the stated facts become the model's conditions
# Two kinds of condition, because the model has two ways of taking one. A HELD cell is fixed and
# water ends at it: right for the edge of the map and the sea, wrong for anything that should carry
# water on. A lake or a river is therefore given as SHAPE in the starting surface (a hollow with a
# rim, a falling groove): the model floods the hollow and sends the water down the groove itself,
# and what it makes of them is checked afterwards.
held = np.full((size, size), np.nan)
edge = np.zeros((size, size), bool)
edge[0, :] = edge[-1, :] = edge[:, 0] = edge[:, -1] = True
held[edge & (h0 > sea + 0.5)] = h0[edge & (h0 > sea + 0.5)]      # the land carries on beyond the map as it lies
start = h0.copy()

lakes, rivers = [], []
for f in s.build.features.values():
    if f.get("kind") != "stated_lake":
        continue
    poly = shapely.Polygon(f["points"])
    inside = shapely.contains_xy(poly, X, Y)
    ring = binary_dilation(inside, structure=np.ones((3, 3), bool)) & ~inside      # its shore, one cell wide, with no gap at a corner
    level = f.get("level")
    if level is None:                                           # "a lake exists here": it sits in the land as found
        level = float(np.percentile(h0[ring], 15)) - 0.5
    level = float(level)
    lift = np.clip(level - h0, 0.0, None) * ring
    dam = ring & (lift > float(p["max_bank"]))
    is_dam = bool(dam.sum() > 0.15 * ring.sum())
    if is_dam:
        iy, ix = np.unravel_index(np.argmax(lift), lift.shape)
        x, y = world_of(iy, ix)
        s.error("stated.lake_dam", f["id"],
                f"to hold water at {level:.1f} m, {dam.sum() / ring.sum():.0%} of its shore would have to be raised by more than "
                f"{float(p['max_bank']):.0f} m, and by {lift[iy, ix]:.1f} m at ({x:.0f}, {y:.0f}): the land falls away from it, so that is a dam, not a lake",
                where=(x, y), hint=f"a level of about {np.percentile(h0[ring], 15) - 0.5:.0f} m would be held here; or move it into a hollow")
    # The shore is HELD at the lake's level: a sill the model cannot cut through, where the streams
    # that reach the lake end. Inside it the ground is a hollow, which the model floods to the sill.
    din = distance_transform_edt(inside) * cell
    bed = level - float(f.get("depth", 4.0)) * smoothstep(0.0, float(f.get("shore", 30.0)), din)
    start = np.where(inside, np.minimum(start, bed), start)
    start[ring] = level
    held[ring] = level
    lakes.append({"f": f, "poly": poly, "inside": inside, "level": level, "dam": is_dam})

for f in s.build.features.values():
    if f.get("kind") != "stated_river":
        continue
    pts = np.asarray(f["points"], dtype=np.float64)
    if f.get("course") == "find":                               # two ends only: the course is found over the land
        pts = least_cost(pts[0], pts[-1])
    line, t = resample(pts, cell * 0.4)
    # A river that starts on an `exit` at the map's edge comes from beyond the map: its groove is
    # carried on outward into the model's margin, so the water arrives instead of beginning.
    enters = s.build.features.get(f.get("starts_at") or "", {}).get("kind") == "exit"
    if enters:
        out = line[0] - line[min(8, len(line) - 1)]
        out /= max(float(np.hypot(*out)), 1e-9)
        lead = line[0] + out * np.arange(pad * cell * 0.5, 0.0, -cell * 0.4)[:, None]
        line = np.concatenate([lead, line])
    z = ground_at(line)
    rise = z[-1] - z[0]
    if rise > 0:
        s.error("stated.uphill", f["id"],
                f"the land at its mouth ({line[-1][0]:.0f}, {line[-1][1]:.0f}) is {z[-1]:.1f} m and at its source "
                f"({line[0][0]:.0f}, {line[0][1]:.0f}) is {z[0]:.1f} m: the river would have to run uphill by {rise:.1f} m",
                where=line[-1], hint="swap its ends, or move the mouth to lower ground")
    ends_in = next((l for l in lakes if shapely.contains_xy(l["poly"], line[-1][0], line[-1][1])), None)
    if ends_in is None and z[-1] > sea + 1.5 and rise <= 0:
        s.error("stated.mouth_dry", f["id"],
                f"it ends at ({line[-1][0]:.0f}, {line[-1][1]:.0f}), where the land is {z[-1] - sea:.1f} m above the sea and no lake is stated: "
                "a river must end in water", where=line[-1], hint="end it at the shore or in a stated lake")
    # A groove that only ever falls, and lies below everything within reach of it, so that water
    # in it cannot spill out sideways where the line crosses a slope.
    lowest = minimum_filter(h0, size=2 * int(p["hold_cells"]) + 1)
    zlow = map_coordinates(lowest, [(line[:, 1] - fr.y0) / cell - 0.5 + pad, (line[:, 0] - fr.x0) / cell - 0.5 + pad], order=0, mode="nearest")
    run = np.concatenate([[0.0], np.cumsum(np.hypot(*np.diff(line, axis=0).T))])
    floor = np.minimum.accumulate(zlow - float(p["incise"]))
    floor = np.minimum(floor, floor[0] - float(p["min_fall"]) * run)
    if ends_in is not None:
        floor = np.maximum(floor, ends_in["level"])
    else:
        floor = np.maximum(floor, sea - 1.0)
    cut = z - floor
    worst = int(np.argmax(cut))
    if cut[worst] > float(p["max_cut"]) and rise <= 0:
        s.warn("stated.gorge", f["id"],
               f"to keep falling it must cut {cut[worst]:.0f} m through the high ground at ({line[worst][0]:.0f}, {line[worst][1]:.0f}); "
               f"this world allows {float(p['max_cut']):.0f} m", where=line[worst],
               hint="lead the course round that ground, or state only its ends with course = \"find\"")
    mark = np.ones((size, size), bool)
    floor_on = np.zeros((size, size))
    for (x, y), lv in zip(line, floor):
        iy, ix = cell_of(x, y)
        if 0 <= iy < size and 0 <= ix < size:
            floor_on[iy, ix] = lv if mark[iy, ix] else min(lv, floor_on[iy, ix])
            mark[iy, ix] = False
    d, (ny_, nx_) = distance_transform_edt(mark, return_indices=True)
    d *= cell
    near = floor_on[ny_, nx_]
    groove = np.where(start > near, near + (start - near) * smoothstep(cell * 0.7, cell * 0.7 + float(p["valley"]), d), start)
    in_lake = np.zeros((size, size), bool)
    for l in lakes:
        in_lake |= binary_dilation(l["inside"], iterations=3)
    start = np.where(in_lake, start, np.minimum(start, groove))
    rivers.append({"f": f, "line": line, "off": d, "uphill": rise > 0, "enters": enters, "stated_ends": (pts[0], pts[-1])})

# ---- run the model
work = s.out / "model"
work.mkdir(parents=True, exist_ok=True)
start.astype("<f4").tofile(work / "initial.f32")
np.zeros((size, size), "<f4").tofile(work / "uplift.f32")
held.astype("<f4").tofile(work / "held.f32")
config = {
    "seed": fr.seed, "size": size, "cell_size_m": cell, "steps": int(p["steps"]), "dt_years": float(p["dt_years"]),
    "k_spl": float(p["erodibility"]), "kappa": float(p["diffusivity"]), "g_deposition": float(p["deposition"]),
    "sea_level_m": sea, "base_depth_m": sea - 30.0,
    "river_min_area_km2": 0.03, "fluvial_min_area_km2": 0.008,
}
(work / "config.json").write_text(json.dumps(config, indent=1))
engine = Path(p["engine"]).resolve()
if not engine.is_file():
    s.error("terrain.engine", s.id, f"the terrain engine is not built: {engine} is missing",
            hint="from the repository root: cargo build --release -p via")
    s.done()
    raise SystemExit(0)
t0 = time.time()
ran = subprocess.run([str(engine), "terrain", "--config", str(work / "config.json"), "--out", str(work / "run"),
                      "--initial", str(work / "initial.f32"), "--uplift", str(work / "uplift.f32"), "--held", str(work / "held.f32")],
                     capture_output=True, text=True)
took = time.time() - t0
if ran.returncode != 0:
    s.error("terrain.engine", s.id, f"the terrain engine failed: {(ran.stderr or ran.stdout).strip()[-400:]}")
    s.done()
    raise SystemExit(0)


def vrast(name, dtype):
    raw = (work / "run" / f"{name}.vrast").read_bytes()
    _magic, _version, _tag, w, hh, _cell_cm = struct.unpack("<4sIIIII", raw[:24])
    return np.frombuffer(raw, dtype=dtype, offset=24).reshape(hh, w)


h1 = vrast("heights_cm", "<i4").astype(np.float64) / 100.0
flow = vrast("discharge", "<f4").astype(np.float64)
silt = vrast("sediment", "<f4").astype(np.float64)
depth = vrast("water_depth", "<f4").astype(np.float64)
receiver = vrast("receivers", "<u4")
area = vrast("area_cells", "<u8").astype(np.float64)
world = np.zeros((size, size), bool)
world[pad:-pad, pad:-pad] = True
land_cells = int((world & (h1 > sea)).sum())

# ---- check each stated fact against what the model made
report = {"engine_s": round(took, 2), "cells": size * size, "lakes": {}, "rivers": {}}
for l in lakes:
    f, level = l["f"], l["level"]
    wet_cells = l["inside"] & (depth > 0.2)
    flooded = float(wet_cells.sum()) / max(int(l["inside"].sum()), 1)
    stands = float(np.median((h1 + depth)[wet_cells])) if wet_cells.any() else float("nan")
    outer = binary_dilation(l["inside"], structure=np.ones((3, 3), bool), iterations=2) & ~binary_dilation(l["inside"], structure=np.ones((3, 3), bool))
    below = outer & (h1 < level - 1.0)
    falls = float(below.sum()) / max(int(outer.sum()), 1)
    report["lakes"][f["id"]] = {"stated": level, "stands": round(stands, 2), "flooded": round(flooded, 3), "shore_falls_away": round(falls, 3)}
    if falls > 0.4 and not l["dam"]:                             # a dam was already reported, with the level that would hold
        iy, ix = np.unravel_index(np.argmin(np.where(below, h1, np.inf)), h1.shape)
        x, y = world_of(iy, ix)
        s.error("stated.lake_perched", f["id"],
                f"the land falls away below its level along {falls:.0%} of its shore, by {level - h1[iy, ix]:.1f} m at ({x:.0f}, {y:.0f}): "
                "it stands above its surroundings like a tank", where=(x, y),
                hint=f"a level of about {np.percentile(h1[outer], 30):.0f} m would sit in this ground; or move it into a hollow")
    cx_, cy_ = l["poly"].centroid.x, l["poly"].centroid.y
    if flooded < 0.7:
        s.error("stated.lake_drained", f["id"],
                f"after the model ran, only {flooded:.0%} of it holds water: it silted up or ran out", where=(cx_, cy_),
                hint="state it in a hollow, or lower its level")
    elif abs(stands - level) > float(p["level_within"]):
        s.error("stated.lake_level", f["id"],
                f"it was stated at {level:.1f} m and the model leaves it at {stands:.1f} m", where=(cx_, cy_),
                hint=f"state {stands:.0f} m, or accept a wider `level_within` than {float(p['level_within']):.1f} m")
    else:
        l["level"] = level                                      # within tolerance: the author's number stands

for r in rivers:
    f, line = r["f"], r["line"]
    if r["enters"]:                                             # checked from where it comes into the map
        line = line[np.argmax([world[cell_of(x, y)] if 0 <= cell_of(x, y)[0] < size and 0 <= cell_of(x, y)[1] < size else False for x, y in line]):]
    iy, ix = cell_of(*line[0])
    path, seen = [(iy, ix)], set()
    while True:                                                 # follow the model's own water down from the source
        i = iy * size + ix
        nxt = int(receiver[iy, ix])
        if nxt == i or i in seen or not world[iy, ix] or h1[iy, ix] + depth[iy, ix] <= sea:
            break
        seen.add(i)
        iy, ix = divmod(nxt, size)
        path.append((iy, ix))
    course = np.asarray([world_of(a, b) for a, b in path])
    off = np.asarray([r["off"][a, b] for a, b in path])
    stray = np.flatnonzero(off > float(p["follow_within"]))
    total = float(np.hypot(*np.diff(line, axis=0).T).sum())
    end_gap = float(np.hypot(*(course[-1] - line[-1])))
    kept = len(path) if len(stray) == 0 else int(stray[0])
    followed = float(np.hypot(*np.diff(course[:max(kept, 2)], axis=0).T).sum()) if kept > 1 else 0.0
    last_land = next(((a, b) for a, b in reversed(path) if h1[a, b] > sea and world[a, b]), path[0])
    gathers = float(area[last_land]) / max(land_cells, 1)
    report["rivers"][f["id"]] = {"stated_m": round(total), "followed_m": round(followed), "ends_off_by_m": round(end_gap),
                                 "gathers": round(gathers, 3), "falls_m": round(float(h1[path[0]] - max(h1[last_land], sea)), 1)}
    if len(stray) and end_gap > 4 * float(p["follow_within"]):
        x, y = course[kept]
        s.error("stated.river_leaves", f["id"],
                f"the water follows it for {followed:.0f} m of {total:.0f} m, then leaves at ({x:.0f}, {y:.0f}) and ends at "
                f"({course[-1][0]:.0f}, {course[-1][1]:.0f}), {end_gap:.0f} m from where it was stated to end", where=(x, y),
                hint="the land beside the course is lower there: move the course into it, or state only its ends with course = \"find\"")
    elif len(stray):
        x, y = course[kept]
        s.warn("stated.river_wanders", f["id"],
               f"the water strays up to {off.max():.0f} m from the stated line (first at ({x:.0f}, {y:.0f})) and rejoins it; "
               f"this world allows {float(p['follow_within']):.0f} m", where=(x, y))
    if gathers < float(p["min_catchment"]) and not r["uphill"]:
        s.warn("stated.river_dry", f["id"],
               f"only {gathers:.0%} of the land drains to it; this world expects {float(p['min_catchment']):.0%}",
               where=tuple(line[len(line) // 2]), hint="it runs along high ground: water flows away from it")
    r["course"] = course
    r["bed"] = np.minimum.accumulate(np.asarray([h1[a, b] + depth[a, b] for a, b in path]))
    r["flow"] = np.asarray([flow[a, b] for a, b in path])
(s.out / "report.json").write_text(json.dumps(report, indent=1))
print(f"terrain: {size}x{size} cells at {cell} m, {int(p['steps'])} steps, engine {took:.1f} s; {json.dumps(report['rivers'])} {json.dumps(report['lakes'])}")

# ---- hand the ground back on the world's posts
py, px = np.mgrid[0:fr.ny, 0:fr.nx].astype(np.float64)
at = [py * res / cell - 0.5 + pad, px * res / cell - 0.5 + pad]
height = map_coordinates(h1, at, order=3, mode="nearest")
wet = map_coordinates(np.log10(1.0 + flow), at, order=1, mode="nearest")
wet = np.clip(wet / np.log10(1.0 + 0.03e6 / (cell * cell) * 4), 0.0, 1.0)      # 1 where a real stream runs
silt_posts = np.clip(map_coordinates(silt, at, order=1, mode="nearest") / 2.0, 0.0, 1.0)

rng = np.random.default_rng(fr.seed)                             # relief finer than the model's grid
for span, share in ((30.0, 0.7), (9.0, 0.3)):
    g = gaussian_filter(rng.standard_normal((fr.ny, fr.nx)), span / res)
    height += float(p["detail"]) * share * g / g.std() * smoothstep(sea, sea + 3, height)

water = np.full((fr.ny, fr.nx), -100.0)
water[height < sea] = sea
wx, wy = fr.xy()
for l in lakes:
    f = l["f"]
    inside = shapely.contains_xy(l["poly"], wx, wy)
    din = distance_transform_edt(inside) * res
    bed = l["level"] - float(f.get("depth", 4.0)) * smoothstep(0.0, float(f.get("shore", 30.0)), din)
    height = np.where(binary_dilation(inside, iterations=4) & ~inside, np.maximum(height, l["level"] + 0.3), height)
    height = np.where(inside, bed, height)                      # the author's bed: the model's deeper basin is not kept
    wetted = binary_dilation(inside, iterations=2) & (height < l["level"])
    water[wetted] = l["level"]
def bends(course, width_at, swing, level, seed):
    """Bends finer than the model's grid, in proportion to the river: it swings `swing` widths to
    each side about every twelve widths, less where higher ground stands beside it, and not at all
    at its ends. The same river bends the same way every build."""
    pts, tt = resample(course, 2.0)
    run = tt * float(np.hypot(*np.diff(pts, axis=0).T).sum())
    w = width_at(tt)
    rng = np.random.default_rng(seed)
    jitter = gaussian_filter(rng.standard_normal(len(pts)), 60.0, mode="nearest")
    jitter = 0.3 * jitter / max(float(np.abs(jitter).max()), 1e-9)
    phase = rng.uniform(0, 2 * np.pi) + np.cumsum(2 * np.pi * 2.0 / (12.0 * w) * (1 + jitter))
    tan = np.gradient(pts, axis=0)
    tan /= np.maximum(np.hypot(tan[:, 0], tan[:, 1]), 1e-9)[:, None]
    nrm = np.stack([-tan[:, 1], tan[:, 0]], axis=1)
    amp = swing * w * np.minimum(smoothstep(0.0, 120.0, run), smoothstep(0.0, 120.0, run[-1] - run))
    lv = np.interp(tt, np.linspace(0, 1, len(level)), level)
    for side in (1.0, -1.0):                                    # high ground beside it holds it in
        q = pts + side * nrm * (amp + w)[:, None]
        zs = map_coordinates(height, [(q[:, 1] - fr.y0) / res, (q[:, 0] - fr.x0) / res], order=1, mode="nearest")
        amp = amp * (1.0 - smoothstep(2.5, 7.0, gaussian_filter(zs - lv, 15.0, mode="nearest")))
    return pts + nrm * (amp * np.sin(phase))[:, None]


def sinuosity(pts):
    return float(np.hypot(*np.diff(pts, axis=0).T).sum() / max(float(np.hypot(*(pts[-1] - pts[0]))), 1e-9))


for r in rivers:
    f = r["f"]
    if len(r["course"]) < 3:
        continue
    deep, bank = float(f.get("depth", 1.2)), float(f.get("bank", 14.0))
    wide = f.get("width", 9.0)
    w0, w1 = (float(wide[0]), float(wide[-1])) if isinstance(wide, (list, tuple)) else (float(wide), float(wide))
    k = 3                                                       # the model's course is a walk from cell to cell: ease it
    c = r["course"]
    eased = np.asarray([c[max(0, i - k):i + k + 1].mean(axis=0) for i in range(len(c))])
    eased[0], eased[-1] = c[0], c[-1]
    if r["enters"]:                                             # from the map's edge itself
        eased = np.concatenate([[r["stated_ends"][0]], eased[2:]])
    length = float(np.hypot(*np.diff(eased, axis=0).T).sum())
    widen, widen_run = float(f.get("estuary", 1.0)), float(f.get("estuary_run", 160.0))

    def width_at(tt, length=length):
        return (w0 + (w1 - w0) * tt) * (1.0 + (widen - 1.0) * smoothstep(length - widen_run, length, tt * length))
    was = sinuosity(eased)
    if float(f.get("bends", 0.0)) > 0:
        eased = bends(eased, width_at, float(f["bends"]), r["bed"], fr.seed + len(f["id"]))
    report["rivers"][f["id"]]["sinuosity"] = [round(was, 3), round(sinuosity(eased), 3)]
    fine, tt = resample(eased, res * 0.5)
    lv = np.interp(tt, np.linspace(0, 1, len(r["bed"])), r["bed"])
    hw = width_at(tt) / 2
    mark = np.ones((fr.ny, fr.nx), bool)
    level_on = np.zeros((fr.ny, fr.nx))
    half_on = np.zeros((fr.ny, fr.nx))
    iy = np.clip(np.round((fine[:, 1] - fr.y0) / res).astype(int), 0, fr.ny - 1)
    ix = np.clip(np.round((fine[:, 0] - fr.x0) / res).astype(int), 0, fr.nx - 1)
    mark[iy, ix] = False
    level_on[iy, ix] = lv
    half_on[iy, ix] = hw
    d, (ny_, nx_) = distance_transform_edt(mark, return_indices=True)
    d *= res
    near = gaussian_filter(level_on[ny_, nx_], 2.0)             # the water's level beside each stretch, with no step across it
    half = gaussian_filter(half_on[ny_, nx_], 2.0)
    # The cross-section: a bed under the water, the waterline at the stated half width, a lip a
    # little above the water just beyond it, and from there back to the ground as it was.
    lip = 0.45 * smoothstep(sea - 0.2, sea + 0.8, height)       # no bank is built out into the sea
    shape = np.where(d < half, near - deep * (1 - smoothstep(half - 3.0, half, d)), near + lip * smoothstep(half, half + 2.5, d))
    back = smoothstep(half + 2.5, half + bank, d)
    cut = np.where(d < half + 2.5, shape, np.minimum(height, shape + (height - shape) * back))
    held_up = np.where(d < half + 2.5, shape, shape - 40.0 * smoothstep(half + 0.5 * bank, half + bank, d))
    height = np.where(d < half + bank, np.where(height > shape, cut, np.where(height > sea - 0.3, np.maximum(height, held_up), height)), height)
    # The water's surface is carried under the banks, so the shore is where a smooth surface meets
    # the ground, and not the edge of a cell.
    carried = (d < half + 4.5) & (water < near)
    water[carried] = near[carried]
    s.add("feature", {"id": f["id"] + "_course", "kind": "guide", "smooth": False,      # where the water runs, for the layers above
                      "path": [[round(float(x), 1), round(float(y), 1)] for x, y in fine[::4]]})
    strip = shapely.unary_union([shapely.LineString(fine[i:i + 13]).buffer(float(hw[i:i + 13].mean())) for i in range(0, len(fine) - 1, 12)])
    strip = max(getattr(strip, "geoms", [strip]), key=lambda g_: g_.area).simplify(0.8)
    s.add("feature", {"id": f["id"] + "_course_water", "kind": "guide", "water": 1,       # the water itself, as wide as it is there
                      "region": [[round(float(x), 1), round(float(y), 1)] for x, y in list(strip.exterior.coords)[:-1]]})
(s.out / "report.json").write_text(json.dumps(report, indent=1))
print("rivers:", json.dumps({k_: v.get("sinuosity") for k_, v in report["rivers"].items()}))

s.write_field("height", height, unit="m")
s.write_field("water", water, unit="m")
s.write_field("wet", wet)
s.write_field("silt", silt_posts)
damp = gaussian_filter(smoothstep(0.35, 0.8, wet), 45.0 / res)
s.write_field("damp", np.clip(damp / max(float(np.percentile(damp, 99.5)), 1e-6), 0.0, 1.0))      # the valley floors
s.done()
