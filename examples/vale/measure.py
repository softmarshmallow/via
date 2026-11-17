"""Measures of a vale build, and their comparison with the spike's (fixtures/oracle/vale*/).

    python3 examples/vale/measure.py BUILD_DIR                   # the measures, as JSON
    python3 examples/vale/measure.py BUILD_DIR --check NAME      # compared with expect.toml's [NAME]

NAME is `base` or a change's name (`move_road`, `winter`, ...). The comparison is of measures,
never of bytes (docs/plan.md: R15, "compare measures"): each row of expect.toml gives the spike's
number, the band ours must fall in, and why the band is what it is. Exit status 1 when a measure
falls outside its band or cannot be taken, 0 otherwise.

It reads only the build directory (spec/build.md): the manifest, features.json, the instance
columns, the field arrays and, when the build was made with `--against`, changes.json. Python's
standard library only (3.11, for tomllib).

How each measure is taken (ours, after the spike's, which took them from its own grids):
- A water body is a feature of kind `lake` or `pond`. Its wet posts are those within its outline's
  box grown by 8 m whose `water` stands above `height`; area is their count times the post
  spacing squared; depth is `water - height` there; shallow is a depth below 1 m. Its shore is the
  length of its outline's rings, holes included, and its islands the count of holes. The spike
  traced the 0 m depth contour instead, so shore lengths differ by the lattice's stair-step.
- Ground at a spike view: the spike's eye height less its eye, at the same plan point, against
  our `height` there. It checks the land, not the cameras, since our views have new names and some
  new places.
- Trees on a way: woods trees whose origin is within 4.5 m of a road's line (its width 5 m, so the
  bed and half a metre beyond).
"""

import json
import math
import struct
import sys
import tomllib
from pathlib import Path

HERE = Path(__file__).resolve().parent


def npy(path):
    """A little-endian float32, float64 or int32 array file of one or two axes (NumPy format 1.0 to
    3.0), as (rows, cols, values); one axis reads as one row."""
    raw = path.read_bytes()
    major = raw[6]
    if major == 1:
        hlen = struct.unpack("<H", raw[8:10])[0]
        off = 10
    else:
        hlen = struct.unpack("<I", raw[8:12])[0]
        off = 12
    header = raw[off:off + hlen].decode("latin1")
    descr = header.split("'descr':")[1].split(",")[0].strip().strip("'\"")
    shape = tuple(int(v) for v in header.split("'shape':")[1].split(")")[0].strip(" (").split(",") if v.strip())
    fortran = "True" in header.split("'fortran_order':")[1].split(",")[0]
    if fortran:
        raise SystemExit(f"{path}: Fortran order is not read")
    code = {"<f4": "f", "<f8": "d", "<i4": "i"}[descr]
    if len(shape) == 1:
        shape = (1, shape[0])
    n = shape[0] * shape[1]
    vals = struct.unpack(f"<{n}{code}", raw[off + hlen:off + hlen + n * struct.calcsize(code)])
    return shape[0], shape[1], vals


class Grid:
    def __init__(self, build):
        man = json.loads((build / "manifest.json").read_text())
        fr = man["frame"]
        self.x0, self.y0, self.x1, self.y1 = fr["extent"]
        self.r = fr["resolution"]
        self.nx, self.ny = fr["posts"]
        self.man = man
        self.build = build
        self._fields = {}

    def field(self, name):
        if name not in self._fields:
            rows, cols, vals = npy(self.build / self.man["fields"][name]["file"])
            self._fields[name] = (rows, cols, vals)
        return self._fields[name]

    def at_post(self, name, i, j):
        """Post (i, j): i along x, j along y. The array is row-major; which axis is rows is read
        from its shape against the frame's posts."""
        rows, cols, vals = self.field(name)
        return vals[j * cols + i]

    def sample(self, name, x, y):
        fx = (x - self.x0) / self.r
        fy = (y - self.y0) / self.r
        i = min(max(int(math.floor(fx)), 0), self.nx - 2)
        j = min(max(int(math.floor(fy)), 0), self.ny - 2)
        u, v = fx - i, fy - j
        a, b = self.at_post(name, i, j), self.at_post(name, i + 1, j)
        c, d = self.at_post(name, i, j + 1), self.at_post(name, i + 1, j + 1)
        return (1 - v) * ((1 - u) * a + u * b) + v * ((1 - u) * c + u * d)


def ring_length(ring):
    return sum(math.dist(ring[k], ring[(k + 1) % len(ring)]) for k in range(len(ring)))


def seg_dist(p, a, b):
    dx, dy = b[0] - a[0], b[1] - a[1]
    l2 = dx * dx + dy * dy
    u = 0.0 if l2 == 0 else max(0.0, min(1.0, ((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / l2))
    return math.dist(p, (a[0] + u * dx, a[1] + u * dy))


def key(name):
    """A water body by its own key: `fill#tarn_lake` (made by the fill step) is `tarn_lake`."""
    return name.split("#")[-1]


def water_bodies(g, feats, water, ground):
    out = {}
    for f in feats:
        if f.get("kind") not in ("lake", "pond") or not f.get("pieces"):
            continue
        rings = [p["outer"] for p in f["pieces"]] + [h for p in f["pieces"] for h in p["holes"]]
        xs = [pt[0] for p in f["pieces"] for pt in p["outer"]]
        ys = [pt[1] for p in f["pieces"] for pt in p["outer"]]
        i0 = max(0, int((min(xs) - 8 - g.x0) / g.r))
        i1 = min(g.nx - 1, int((max(xs) + 8 - g.x0) / g.r) + 1)
        j0 = max(0, int((min(ys) - 8 - g.y0) / g.r))
        j1 = min(g.ny - 1, int((max(ys) + 8 - g.y0) / g.r) + 1)
        depths = []
        level = None
        for j in range(j0, j1 + 1):
            for i in range(i0, i1 + 1):
                w = g.at_post(water, i, j)
                h = g.at_post(ground, i, j)
                if w - h > 0:
                    depths.append(w - h)
                    level = w if level is None else max(level, w)
        if not depths:
            out[key(f["name"])] = {"wet": False}
            continue
        area = len(depths) * g.r * g.r
        shore = sum(ring_length(r) for r in rings)
        out[key(f["name"])] = {
            "authority": f["authority"],
            "level_m": round(level, 2),
            "area_m2": round(area),
            "shore_m": round(shore),
            "shoreline_development": round(shore / (2 * math.sqrt(math.pi * area)), 2),
            "islands": sum(len(p["holes"]) for p in f["pieces"]),
            "pieces": len(f["pieces"]),
            "max_depth_m": round(max(depths), 2),
            "mean_depth_m": round(sum(depths) / len(depths), 2),
            "shallow_share": round(sum(1 for d in depths if d < 1) / len(depths), 2),
        }
    return out


def instances(g):
    names = (g.build / g.man["instances"]["names"]).read_text().split("\n")
    xs = npy(g.build / g.man["instances"]["x"])[2]
    ys = npy(g.build / g.man["instances"]["y"])[2]
    per_asset, per_maker, rows = {}, {}, []
    for ch in g.man["chunks"]:
        for asset, (first, count) in ch["instances"].items():
            per_asset[asset] = per_asset.get(asset, 0) + count
            for n in range(first, first + count):
                maker = names[n].split("#")[0] if "#" in names[n] else "hand"
                per_maker[maker] = per_maker.get(maker, 0) + 1
                rows.append((names[n], asset, maker, xs[n] / 1000, ys[n] / 1000))
    return dict(sorted(per_asset.items())), dict(sorted(per_maker.items())), rows


BUILDINGS = ("house_low", "house_wide", "barn", "shed_gable")


def steep_by_houses(g, ground, rows, reach=15.0, limit=40.0):
    """Posts steeper than `limit` degrees within `reach` metres of a building's origin: a pad's bank
    that ends in a cliff (lib/land). The slope is the steepest of the post's four differences to
    its neighbours, per metre."""
    steep = 0
    for name, asset, maker, x, y in rows:
        if asset not in BUILDINGS:
            continue
        i0, i1 = int((x - reach - g.x0) / g.r), int((x + reach - g.x0) / g.r) + 1
        j0, j1 = int((y - reach - g.y0) / g.r), int((y + reach - g.y0) / g.r) + 1
        for j in range(max(1, j0), min(g.ny - 1, j1 + 1)):
            for i in range(max(1, i0), min(g.nx - 1, i1 + 1)):
                px, py = g.x0 + i * g.r, g.y0 + j * g.r
                if math.hypot(px - x, py - y) > reach:
                    continue
                h = g.at_post(ground, i, j)
                most = max(abs(h - g.at_post(ground, i + di, j + dj))
                           for di, dj in ((1, 0), (-1, 0), (0, 1), (0, -1))) / g.r
                if math.degrees(math.atan(most)) > limit:
                    steep += 1
    return steep


def crowns_over_buildings(g, rows):
    """Trees whose crown (the asset's plan bounds, times its scale) meets a building's plan bounds,
    read from the manifest's assets; the building's turned by its facing as a disc of its half
    diagonal, the crown as a disc of its half width."""
    assets = g.man["assets"]
    scales = npy(g.build / g.man["instances"]["scale"])[2]
    names = (g.build / g.man["instances"]["names"]).read_text().split("\n")
    index = {n: k for k, n in enumerate(names)}
    houses = []
    for name, asset, maker, x, y in rows:
        if asset in BUILDINGS:
            b = assets[asset]["bounds"]
            houses.append((x, y, math.hypot(b[3] - b[0], b[4] - b[1]) / 2))
    over = 0
    for name, asset, maker, x, y in rows:
        if not asset.startswith("tree_"):
            continue
        b = assets[asset]["bounds"]
        crown = max(b[3] - b[0], b[4] - b[1]) / 2 * float(scales[index[name]])
        if any(math.hypot(x - hx, y - hy) < crown + hr for hx, hy, hr in houses):
            over += 1
    return over


def measure(build):
    build = Path(build)
    g = Grid(build)
    feats = json.loads((build / "features.json").read_text())["features"]
    fields = g.man["fields"]
    ground = next(n for n, v in fields.items() if v.get("role") == "surface")
    water = next(n for n, v in fields.items() if "water_level" in v.get("tags", []))
    per_asset, per_maker, rows = instances(g)
    kinds = {}
    for f in feats:
        k = f.get("kind") or "guide"
        kinds[k] = kinds.get(k, 0) + 1
    views = {}
    for f in feats:
        if f.get("kind") == "bookmark":
            x, y, z = f["at"]
            views[f["name"]] = {"eye": [round(x, 2), round(y, 2), round(z + f["attrs"]["eye_height"], 2)],
                                "toward": f["attrs"]["toward"]["name"]}
    oracle = json.loads((HERE.parents[1] / "fixtures/oracle/vale/measures.json").read_text())
    eyes = {"bm_road_south": 1.8, "bm_hamlet": 2.2, "bm_tarn": 2.2, "bm_cliff": 1.8, "bm_overlook": 3.0,
            "bm_mill": 2.0}          # the spike's `eye` per view (spike-final:spike/worlds/vale), ground views only
    ground_at = {}
    for b in oracle["bookmarks"]:
        if b["id"] in eyes:
            x, y, z = b["eye"]
            theirs = z - eyes[b["id"]]
            ours = g.sample(ground, x, y)
            ground_at[b["id"]] = {"spike_m": round(theirs, 2), "ours_m": round(ours, 2),
                                  "diff_m": round(ours - theirs, 2)}
    roads = [f for f in feats if f.get("kind") == "road"]
    on_way = 0
    for name, asset, maker, x, y in rows:
        if maker != "woods":
            continue
        for r in roads:
            path = r["points"]
            if min(seg_dist((x, y), path[k], path[k + 1]) for k in range(len(path) - 1)) < 4.5:
                on_way += 1
                break
    out = {
        "steep_by_houses": steep_by_houses(g, ground, rows),
        "crowns_over_buildings": crowns_over_buildings(g, rows),
        "water": water_bodies(g, feats, water, ground),
        "instances": sum(per_asset.values()),
        "instances_per_asset": per_asset,
        "instances_per_maker": per_maker,
        "features_per_kind": dict(sorted(kinds.items())),
        "ground_at_spike_views": ground_at,
        "woods_on_roads": on_way,
        "views": views,
    }
    diags = json.loads((build / "diagnostics.json").read_text())
    rows = diags["diagnostics"] if isinstance(diags, dict) else diags
    out["diagnostics"] = {s: sum(1 for d in rows if d["severity"] == s) for s in ("error", "warning")}
    out["codes"] = sorted({d["code"] for d in rows if d["severity"] != "info"})
    ch = build / "changes.json"
    if ch.is_file():
        c = json.loads(ch.read_text())
        if c.get("against"):
            counts = {}
            for e in c["entries"]:
                who = e["name"].split("#")[0] if "#" in e["name"] else e["table"]
                for what in e["what"]:
                    counts.setdefault(who, {}).setdefault(what, 0)
                    counts[who][what] += 1
            out["changes"] = dict(sorted(counts.items()))
            out["changed_names"] = sorted(f"{e['table']}:{e['name']}" for e in c["entries"] if "#" not in e["name"])
    return out


def get(m, path):
    for part in path.split("."):
        if not isinstance(m, dict) or part not in m:
            return None
        m = m[part]
    return m


def check(m, name):
    exp = tomllib.loads((HERE / "expect.toml").read_text())
    rows = exp.get(name)
    if rows is None:
        raise SystemExit(f"expect.toml has no [{name}]")
    bad = 0
    for key, row in rows.items():
        if not isinstance(row, dict):
            continue
        ours = get(m, row["measure"])
        if ours is None and row["measure"].startswith("changes.") and "changes" in m:
            ours = 0                      # the report has no entry of that table and kind
        lo, hi = row["band"]
        ok = ours is not None and lo <= ours <= hi
        bad += not ok
        spike = row.get("spike", "-")
        print(f"{'ok ' if ok else 'OUT'} {name}.{key}: ours {ours}, band [{lo}, {hi}], spike {spike}")
    return 1 if bad else 0


def main():
    if len(sys.argv) < 2:
        raise SystemExit(__doc__)
    m = measure(sys.argv[1])
    if len(sys.argv) == 4 and sys.argv[2] == "--check":
        sys.exit(check(m, sys.argv[3]))
    print(json.dumps(m, indent=1, sort_keys=True))


if __name__ == "__main__":
    main()
