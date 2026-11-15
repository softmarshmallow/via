"""Measures of an example build, and their check against expect.toml.

    python3.11 examples/example/measure.py BUILD_DIR                  # the measures, as JSON
    python3.11 examples/example/measure.py BUILD_DIR --check NAME     # against expect.toml's [NAME]

NAME is `base` or a change's name (`third_cottage`, `move_tarn`, `move_beck`, `clear_one`,
`pin_one`, `winter`). A change's build is made with `--against` the base, so its `changes.json`
reports against the base. Exit status 1 when a measure falls outside its band or cannot be taken.

It reads only the build directory (spec/build.md) and the run report beside it
(`<BUILD_DIR>.run.json`, graph.md 6.3): the manifest, features.json, the instance columns, the
field arrays, diagnostics.json and changes.json. Python's standard library only (3.11, for
tomllib).

How each measure is taken (ours):
- `errors`, `warnings`: the build's diagnostics.json counts, lints included.
- `instances.<maker>`: instances whose name starts `<maker>#`; `instances.by_hand` the rest.
- `outputs.<name>`: the manifest's `outputs` lists (bookmarks, points of interest, spawn zones).
- `tarn.*`: the lake the fill step returned (`fill#tarn_lake`): `water_height` its attribute;
  `area_m2` the posts inside its outline's box grown by 4 m where the water stands more than 2 cm
  over the ground, times the post spacing squared; `depth_max` the deepest of those; `x`, `y`
  their mean.
- `deck.*`: the footbridge's strip (within `width / 2` of its line): `wet` posts where the water
  stands at or over the ground; `step_m` the largest height difference between a deck end and the
  ground 4 m out along the lane that arrives there.
- `reach.*`: lib/review reach's `walked` record (`held`, `pairs`), and how many `unreached` and
  `not_sealed` records it gave.
- `homes.far_m`: the largest distance the project's `homes` lint gave.
- `trees_on_lanes`: scattered trees (`woods`) within `width / 2 + 0.5` of a lane's line.
- `change.*` (a change build only): from changes.json's summary, `added`, `removed` and `moved`
  per maker or table; `steps_ran` the generate steps the run report says ran.
"""

import json
import math
import struct
import sys
import tomllib
from pathlib import Path

HERE = Path(__file__).resolve().parent


def npy(path):
    """A little-endian array file of one or two axes (NumPy format 1.0 to 3.0): (rows, cols, values)."""
    raw = path.read_bytes()
    if raw[6] == 1:
        hlen, off = struct.unpack("<H", raw[8:10])[0], 10
    else:
        hlen, off = struct.unpack("<I", raw[8:12])[0], 12
    header = raw[off:off + hlen].decode("latin1")
    descr = header.split("'descr':")[1].split(",")[0].strip().strip("'\"")
    shape = tuple(int(v) for v in header.split("'shape':")[1].split(")")[0].strip(" (").split(",") if v.strip())
    code = {"<f4": "f", "<f8": "d", "<i4": "i", "<i8": "q"}[descr]
    if len(shape) == 1:
        shape = (1, shape[0])
    n = shape[0] * shape[1]
    return shape[0], shape[1], struct.unpack(f"<{n}{code}", raw[off + hlen:off + hlen + n * struct.calcsize(code)])


class Grid:
    def __init__(self, build):
        self.man = json.loads((build / "manifest.json").read_text())
        fr = self.man["frame"]
        self.x0, self.y0, self.x1, self.y1 = fr["extent"]
        self.r = fr["resolution"]
        self.nx, self.ny = fr["posts"]
        self.build = build
        self._f = {}

    def field(self, name):
        if name not in self._f:
            self._f[name] = npy(self.build / self.man["fields"][name]["file"])
        return self._f[name]

    def post(self, name, i, j):
        _, cols, vals = self.field(name)
        return vals[j * cols + i]

    def sample(self, name, x, y):
        fx, fy = (x - self.x0) / self.r, (y - self.y0) / self.r
        i = min(max(int(math.floor(fx)), 0), self.nx - 2)
        j = min(max(int(math.floor(fy)), 0), self.ny - 2)
        u, v = fx - i, fy - j
        a, b = self.post(name, i, j), self.post(name, i + 1, j)
        c, d = self.post(name, i, j + 1), self.post(name, i + 1, j + 1)
        return (1 - v) * ((1 - u) * a + u * b) + v * ((1 - u) * c + u * d)

    def posts_in(self, box):
        x0, y0, x1, y1 = box
        i0, i1 = max(0, math.ceil((x0 - self.x0) / self.r)), min(self.nx - 1, math.floor((x1 - self.x0) / self.r))
        j0, j1 = max(0, math.ceil((y0 - self.y0) / self.r)), min(self.ny - 1, math.floor((y1 - self.y0) / self.r))
        for j in range(j0, j1 + 1):
            for i in range(i0, i1 + 1):
                yield i, j, self.x0 + i * self.r, self.y0 + j * self.r


def seg_dist(p, a, b):
    dx, dy = b[0] - a[0], b[1] - a[1]
    l2 = dx * dx + dy * dy
    u = 0.0 if l2 == 0 else max(0.0, min(1.0, ((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / l2))
    return math.dist(p, (a[0] + u * dx, a[1] + u * dy))


def line_dist(p, pts):
    return min(seg_dist(p, a, b) for a, b in zip(pts, pts[1:]))


def instances(g):
    inst = g.man["instances"]
    names = [n for n in (g.build / inst["names"]).read_text().split("\n") if n]
    xs = [v / 1000 for v in npy(g.build / inst["x"])[2]]
    ys = [v / 1000 for v in npy(g.build / inst["y"])[2]]
    return list(zip(names, xs, ys))


def measure(build):
    g = Grid(build)
    out = {}
    diag = json.loads((build / "diagnostics.json").read_text())
    out["errors"] = diag["counts"]["error"]
    out["warnings"] = diag["counts"]["warning"]
    feats = {f["name"]: f for f in json.loads((build / "features.json").read_text())["features"]}

    inst = instances(g)
    per = {}
    for n, _, _ in inst:
        k = n.split("#")[0] if "#" in n else "by_hand"
        per[k] = per.get(k, 0) + 1
    out["instances"] = dict(sorted(per.items()), total=len(inst))
    out["outputs"] = {k: len(v) for k, v in sorted(g.man["outputs"].items())}

    lake = feats.get("fill#tarn_lake")
    if lake:
        ring = lake["pieces"][0]["outer"]
        box = (min(p[0] for p in ring) - 4, min(p[1] for p in ring) - 4, max(p[0] for p in ring) + 4, max(p[1] for p in ring) + 4)
        wet = [(x, y, g.post("water", i, j) - g.post("height", i, j)) for i, j, x, y in g.posts_in(box)]
        wet = [w for w in wet if w[2] > 0.02]
        out["tarn"] = {
            "water_height": lake["attrs"]["water_height"],
            "area_m2": round(len(wet) * g.r * g.r, 1),
            "depth_max": round(max((w[2] for w in wet), default=0), 3),
            "x": round(sum(w[0] for w in wet) / len(wet), 1) if wet else None,
            "y": round(sum(w[1] for w in wet) / len(wet), 1) if wet else None,
        }

    fb = feats.get("footbridge")
    if fb:
        a, b = [p[:2] for p in fb["points"][:2]]
        half = fb["attrs"]["width"] / 2
        box = (min(a[0], b[0]) - half, min(a[1], b[1]) - half, max(a[0], b[0]) + half, max(a[1], b[1]) + half)
        strip = [(i, j) for i, j, x, y in g.posts_in(box) if seg_dist((x, y), a, b) <= half]
        wet = sum(1 for i, j in strip if g.post("water", i, j) >= g.post("height", i, j))
        steps = []
        for end, lane in ((a, "lane_west"), (b, "lane_east")):
            pts = [p[:2] for p in feats[lane]["points"]]
            far = pts[-2] if lane == "lane_west" else pts[1]
            d = math.dist(end, far)
            out_pt = (end[0] + (far[0] - end[0]) * 4 / d, end[1] + (far[1] - end[1]) * 4 / d)
            steps.append(abs(g.sample("height", *end) - g.sample("height", *out_pt)))
        out["deck"] = {"posts": len(strip), "wet": wet, "step_m": round(max(steps), 2),
                       "length_m": round(math.dist(a, b), 1)}

    rec = diag["diagnostics"]
    walked = [d for d in rec if d["code"] == "step.reach.walked"]
    if walked:
        data = walked[0]["data"]
        out["reach"] = {"held": data["held"], "pairs": data["pairs"],
                        "unreached": sum(1 for d in rec if d["code"] == "step.reach.unreached"),
                        "not_sealed": sum(1 for d in rec if d["code"] == "step.reach.not_sealed")}
    far = [d["data"]["value"] for d in rec if d["code"] == "step.homes.lane_reach"]
    out["homes"] = {"count": len(far), "far_m": max(far, default=0)}

    lanes = [(f["points"], f["attrs"].get("width", 4)) for f in feats.values() if f.get("kind") == "lane"]
    out["trees_on_lanes"] = sum(
        1 for n, x, y in inst if n.startswith("woods#")
        and any(line_dist((x, y), [p[:2] for p in pts]) <= w / 2 + 0.5 for pts, w in lanes))

    ch = build / "changes.json"
    if ch.exists():
        rep = json.loads(ch.read_text())
        change = {}
        for s in rep["summary"]:
            k = s["maker"] or s["table"]
            for what in ("added", "removed", "moved"):
                if s[what]:
                    change.setdefault(what, {})[k] = change.get(what, {}).get(k, 0) + s[what]
        run = build.parent / (build.name + ".run.json")
        if run.exists():
            r = json.loads(run.read_text())
            change["steps_ran"] = sorted(n["id"].split(":", 1)[1] for n in r["nodes"]
                                         if n["id"] in ("step:fill", "step:moisture") and n["status"] == "ran")
        out["change"] = change
    out["meet"] = meetings(build)
    return out


def get(m, path):
    """The measure at a dotted path. Under `change.`, a count nothing reported is 0."""
    absent = 0 if path.startswith("change.") else None
    for k in path.split("."):
        if not isinstance(m, dict) or k not in m:
            return absent
        m = m[k]
    return m


def check(m, name):
    expect = tomllib.loads((HERE / "expect.toml").read_text())
    rows = expect.get(name)
    if not rows:
        print(f"expect.toml has no [{name}]")
        return 1
    bad = 0
    for row_id, row in rows.items():
        v = get(m, row["measure"])
        if "equals" in row:
            ok = v == row["equals"]
            want = row["equals"]
        else:
            lo, hi = row["band"]
            ok = isinstance(v, (int, float)) and lo <= v <= hi
            want = row["band"]
        print(f"{'ok  ' if ok else 'FAIL'} {name}.{row_id}: {row['measure']} = {v} (want {want})")
        bad += not ok
    return 1 if bad else 0


def main(argv):
    if len(argv) < 2:
        print(__doc__)
        return 2
    m = measure(Path(argv[1]))
    if "--check" in argv:
        return check(m, argv[argv.index("--check") + 1])
    print(json.dumps(m, indent=1))
    return 0


# ---------------------------------------------------------------- meetings: ends, heads, faces (M3 slice 5)
#
# How each is taken (ours, M3 slice 5; the same block in quay, townland, example and monastery):
# heights are fields/height.npy and fields/water.npy read bilinearly between posts at the world's
# resolution from its extent's lower corner (manifest.json `frame.resolution`, `frame.extent`). A way is a path of kind main_street, street, lane,
# road or trail; its end meets a crossing when it lies within 0.5 m of the crossing's first or last
# point, and a waterfront when within 0.5 m of its line. From each such end the way's own centreline
# is walked back 20 m, a sample every 0.5 m:
# - `head_grade` / `way_end_grade`: the steepest metre (|h(s + 1) - h(s)|, m per m) of those 20 m,
#   the largest over the ways that end on a crossing / on a waterfront;
# - `head_drop2` / `way_end_drop3`: |h(2) - h(0)| / |h(3) - h(0)| in metres, the largest.
# For each crossing with `crosses`, along the crossed water's line from where the deck's middle
# projects onto it: `face_pool` the water's depth (water - height) 1 m off each face of the deck
# less its depth 15 m off that face, the larger side, in metres (0 or less: no pool).
# For each waterfront end that meets no crossing end (none within 1 m), the posts within 12 m of
# the end and past it along its end segment: `front_end_slope` the steepest (degrees, central
# differences over two posts), `front_end_pit` how far the lowest dry post lies below the lowest
# water level among them (m, 0 if none dry or none wet); `front_gap` the least distance from a
# waterfront's end to a crossing's end (m), when the world has both.

def _mt_field(d, name):
    raw = (Path(d) / "fields" / f"{name}.npy").read_bytes()
    hlen = struct.unpack("<H", raw[8:10])[0]
    head = raw[10:10 + hlen].decode()
    ny, nx = (int(v) for v in head.split("'shape': (")[1].split(")")[0].split(",") if v.strip())
    return nx, ny, struct.unpack(f"<{nx * ny}f", raw[10 + hlen:])


def _mt_at(g, r, x, y):
    nx, ny, v = g
    fx, fy = x / r, y / r
    i, j = min(max(int(fx), 0), nx - 2), min(max(int(fy), 0), ny - 2)
    a, b = fx - i, fy - j
    return ((1 - a) * (1 - b) * v[j * nx + i] + a * (1 - b) * v[j * nx + i + 1]
            + (1 - a) * b * v[(j + 1) * nx + i] + a * b * v[(j + 1) * nx + i + 1])


def _mt_walk(pts, s):
    for a, b in zip(pts, pts[1:]):
        n = math.dist(a, b)
        if s <= n:
            return a[0] + (b[0] - a[0]) * s / n, a[1] + (b[1] - a[1]) * s / n
        s -= n
    return pts[-1]


def _mt_near(p, pts):
    """(distance, arc length) of the nearest point of polyline pts to p."""
    best, acc = (1e18, 0.0), 0.0
    for a, b in zip(pts, pts[1:]):
        n = math.dist(a, b)
        t = max(0.0, min(1.0, ((p[0] - a[0]) * (b[0] - a[0]) + (p[1] - a[1]) * (b[1] - a[1])) / (n * n)))
        e = math.dist(p, (a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1])))
        best = min(best, (e, acc + t * n))
        acc += n
    return best


def meetings(d):
    d = Path(d)
    fr = json.loads((d / "manifest.json").read_text())["frame"]
    r, x0, y0 = fr["resolution"], fr["extent"][0], fr["extent"][1]
    fs = json.loads((d / "features.json").read_text())["features"]
    by = {f["name"]: f for f in fs}
    h, w = _mt_field(d, "height"), _mt_field(d, "water")
    at = lambda g, p: _mt_at(g, r, p[0] - x0, p[1] - y0)
    heads = [f["points"][k][:2] for f in fs if f.get("kind") == "crossing" for k in (0, -1)]
    fronts = [[p[:2] for p in f["points"]] for f in fs if f.get("kind") == "waterfront"]
    m = {}
    for f in fs:
        if f.get("kind") not in ("main_street", "street", "lane", "road", "trail") or f.get("shape") != "path":
            continue
        pts = [p[:2] for p in f["points"]]
        for line in (pts, pts[::-1]):
            if any(math.dist(line[0], e) <= 0.5 for e in heads):
                key, drop = "head", 4
            elif any(_mt_near(line[0], fr)[0] <= 0.5 for fr in fronts):
                key, drop = "way_end", 6
            else:
                continue
            prof = [at(h, _mt_walk(line, 0.5 * k)) for k in range(41)]
            m[key + "_grade"] = round(max(m.get(key + "_grade", 0), max(abs(prof[k + 2] - prof[k]) for k in range(39))), 2)
            name = key + ("_drop2" if drop == 4 else "_drop3")
            m[name] = round(max(m.get(name, 0), abs(prof[drop] - prof[0])), 2)
    for c in fs:
        cr = (c.get("attrs") or {}).get("crosses")
        water = by.get(cr.get("name") if isinstance(cr, dict) else cr) if c.get("kind") == "crossing" else None
        if not water or water.get("shape") != "path":
            continue
        a, b = c["points"][0][:2], c["points"][-1][:2]
        line = [p[:2] for p in water["points"]]
        sx = _mt_near(((a[0] + b[0]) / 2, (a[1] + b[1]) / 2), line)[1]
        half = float(c["attrs"].get("width", 6)) / 2
        depth = lambda s: at(w, _mt_walk(line, s)) - at(h, _mt_walk(line, s))
        pool = max(depth(sx + k * (half + 1)) - depth(sx + k * (half + 15)) for k in (-1, 1))
        m["face_pool"] = round(max(m.get("face_pool", -9), pool), 2)
    for pts in fronts:
        for p, q in ((pts[0], pts[1]), (pts[-1], pts[-2])):
            if heads:
                m["front_gap"] = round(min(m.get("front_gap", 1e9), min(math.dist(p, e) for e in heads)), 1)
            if any(math.dist(p, e) <= 1 for e in heads):
                continue
            n = math.dist(p, q)
            ux, uy = (p[0] - q[0]) / n, (p[1] - q[1]) / n
            slope, dry, wet = 0.0, [], []
            nx, ny, hv = h
            for j in range(max(1, int((p[1] - y0 - 12) / r)), min(ny - 1, int((p[1] - y0 + 12) / r) + 1)):
                for i in range(max(1, int((p[0] - x0 - 12) / r)), min(nx - 1, int((p[0] - x0 + 12) / r) + 1)):
                    x, y = x0 + i * r, y0 + j * r
                    if math.hypot(x - p[0], y - p[1]) > 12 or (x - p[0]) * ux + (y - p[1]) * uy < 0:
                        continue
                    gx = (hv[j * nx + i + 1] - hv[j * nx + i - 1]) / (2 * r)
                    gy = (hv[(j + 1) * nx + i] - hv[(j - 1) * nx + i]) / (2 * r)
                    slope = max(slope, math.degrees(math.atan(math.hypot(gx, gy))))
                    k = j * nx + i
                    (wet if w[2][k] - hv[k] > 0.001 else dry).append((hv[k], w[2][k]))
            pit = max(0.0, min(v for _, v in wet) - min(g for g, _ in dry)) if wet and dry else 0.0
            m["front_end_slope"] = round(max(m.get("front_end_slope", 0), slope), 1)
            m["front_end_pit"] = round(max(m.get("front_end_pit", 0), pit), 2)
    return m


if __name__ == "__main__":
    sys.exit(main(sys.argv))
