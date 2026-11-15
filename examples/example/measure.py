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


if __name__ == "__main__":
    sys.exit(main(sys.argv))
