"""Massif's measures, read from its build directories and held to expect.toml (Python 3.11+, NumPy).

    python examples/massif/measure.py BUILD_ROOT    # BUILD_ROOT holds base/ and one folder per change

Each [measure.<id>] of expect.toml is taken from BUILD_ROOT/base and compared with its `expect`
(a range, a pair of ranges, a value or a list); each [change.<id>] with a `file` is read from
BUILD_ROOT/<id> and its run report BUILD_ROOT/<id>.run.json, and held to its recorded `steps` (each
step node's status in the run report), `counts` (instance counts after the change) and `revised`
(the instance count of each `asset.revised` finding). Prints `ok`, `OUT` or `--` per line and exits
1 when a compared measure or change row is out or a build is missing.

Where each is read: instance counts from manifest.json's chunks; features by kind and bookmark eyes
(`at` z plus `eye_height`) from features.json; relief from `height`; findings from diagnostics.json.
"""

import json
import sys
import tomllib
from collections import Counter
from pathlib import Path

import numpy as np

HERE = Path(__file__).resolve().parent


class Build:
    def __init__(self, d):
        self.d = d
        self.diags = json.loads((d / "diagnostics.json").read_text())
        self.m = json.loads((d / "manifest.json").read_text())
        self.features = {f["name"]: f for f in json.loads((d / "features.json").read_text())["features"]}

    def count(self, *assets):
        return sum(c["instances"].get(a, [0, 0])[1] for c in self.m["chunks"] for a in assets)

    def kinds(self):
        return dict(Counter(f["kind"] for f in self.features.values() if f.get("kind")))

    def eye(self, name):
        f = self.features[name]
        return round(f["at"][2] + float(f["attrs"].get("eye_height", 1.7)), 2)

    def found(self, code):
        return [x for x in self.diags["diagnostics"] if x["code"] == code]


def take(mid, b):
    if mid == "trees":
        return b.count("tree_oak", "tree_pine")
    if mid == "tree_mix":
        t = b.count("tree_oak", "tree_pine")
        return round(b.count("tree_oak") / t, 2) if t else None
    if mid == "rocks":
        return b.count("rock_a", "rock_b")
    if mid == "houses":
        return b.count("house_low", "house_wide")
    if mid == "features":
        k = b.kinds()
        return {x: k.get(x, 0) for x in ("bookmark", "beck", "road")}
    if mid == "relief":
        h = np.load(b.d / "fields/height.npy")
        return [round(float(h.min()), 1), round(float(h.max()), 1)]
    if mid == "road_eye":
        return b.eye("up_the_road")
    if mid == "overlook_eye":
        return b.eye("over_valley")
    if mid == "far_home":
        return sorted(x["subject"].split(".", 1)[1].strip('"') for x in b.found("step.homes.far_from_road"))
    if mid == "seam_grade":
        return len(b.found("seam.grade"))
    if mid == "trees_in_clearing":
        d = b.found("step.checks.trees_in_clearing")
        return sum(float(x["data"].get("value", 0)) for x in d)
    if mid == "pad_relief":
        return {x["subject"].split(".", 1)[1]: float(x["data"].get("value", 0))
                for x in b.found("step.checks.cottage_relief")}
    if mid == "beck_wet_ratio":
        # Wet posts (water above ground) within 8 m of the beck's line, times the post area,
        # over its length times its width: 1 when the water is the channel the bed cut.
        f = next((x for x in b.features.values() if x.get("kind") == "beck"), None)
        if f is None:
            return None
        pts = np.array([[q[0], q[1]] for q in f["points"]])
        width = float(f["attrs"].get("width", 6))
        length = float(np.sum(np.hypot(*np.diff(pts, axis=0).T)))
        fr = b.m["frame"]
        r, x0, y0 = fr["resolution"], fr["extent"][0], fr["extent"][1]
        h = np.load(b.d / "fields/height.npy").astype(np.float64)
        w = np.load(b.d / "fields/water.npy").astype(np.float64)
        jj, ii = np.nonzero(w - h > 1e-4)
        xs, ys = x0 + ii * r, y0 + jj * r
        near = np.full(xs.shape, np.inf)
        for (ax, ay), (bx, by) in zip(pts[:-1], pts[1:]):
            dx, dy = bx - ax, by - ay
            l2 = dx * dx + dy * dy
            t = np.clip(((xs - ax) * dx + (ys - ay) * dy) / l2, 0, 1) if l2 else np.zeros(xs.shape)
            near = np.minimum(near, np.hypot(xs - ax - t * dx, ys - ay - t * dy))
        wet = float(np.sum(near <= 8)) * r * r
        return round(wet / (length * width), 2)
    if mid == "water_step":
        d = b.found("step.checks.water_step")
        return max((float(x["data"].get("value", 0)) for x in d), default=None)
    return None


def compare(want, v):
    if v is None:
        return False
    if isinstance(want, list) and len(want) == 2 and all(isinstance(w, list) for w in want):
        return all(w[0] <= x <= w[1] for w, x in zip(want, v))
    if isinstance(want, list) and len(want) == 2 and all(isinstance(w, (int, float)) for w in want):
        if isinstance(v, dict):
            return all(want[0] <= x <= want[1] for x in v.values())
        return want[0] <= v <= want[1]
    return want == v


def main(root):
    exp = tomllib.loads((HERE / "expect.toml").read_text())
    base_dir = root / "base"
    if not (base_dir / "manifest.json").exists():
        print(f"OUT  no base build at {base_dir}")
        return 1
    base = Build(base_dir)
    bad = False
    for mid, e in exp["measure"].items():
        v = take(mid, base)
        ok = compare(e["expect"], v)
        bad |= not ok
        print(f"{'ok  ' if ok else 'OUT '} {mid}: {v}  (expect {e['expect']}, oracle {e.get('oracle', '-')})")
    for cid, e in exp.get("change", {}).items():
        if "file" not in e:
            continue
        d = root / cid
        if not (d / "manifest.json").exists():
            print(f"OUT  {cid}: no build at {d}")
            bad = True
            continue
        b = Build(d)
        c = b.diags["counts"]
        revised = {x["subject"].split(".", 1)[-1]: x["data"]["count"] for x in b.found("asset.revised")}
        print(f"--   {cid}: exit {b.diags['status']}, errors {c['error']}, warnings {c['warning']}; "
              f"trees {take('trees', base)} -> {take('trees', b)}, rocks {take('rocks', base)} -> "
              f"{take('rocks', b)}; far homes {take('far_home', b)}"
              + (f"; asset.revised {revised}" if revised else ""))
        bad |= b.diags["status"] != 0
        run = root / f"{cid}.run.json"
        if "steps" in e:
            if not run.exists():
                print(f"OUT  {cid} steps: no run report at {run}")
                bad = True
            else:
                nodes = {n["id"]: n["status"] for n in json.loads(run.read_text())["nodes"]}
                got = {k: nodes.get(f"step:{k}") for k in e["steps"]}
                ok = got == e["steps"]
                bad |= not ok
                print(f"{'ok  ' if ok else 'OUT '} {cid} steps: {got}  (recorded {e['steps']})")
        if "counts" in e:
            got = {k: take(k, b) for k in e["counts"]}
            ok = got == e["counts"]
            bad |= not ok
            print(f"{'ok  ' if ok else 'OUT '} {cid} counts: {got}  (recorded {e['counts']})")
        if "revised" in e:
            ok = revised == e["revised"]
            bad |= not ok
            print(f"{'ok  ' if ok else 'OUT '} {cid} asset.revised: {revised}  (recorded {e['revised']})")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main(Path(sys.argv[1])))
