"""Relief's measures, read from its build directories and held to expect.toml (Python 3.11+, NumPy).

    VIA=target/release/via VIA_LIB=lib python examples/relief/measure.py BUILD_ROOT

BUILD_ROOT holds base/ and one folder per change (moved_hollow/, new_seed/, ...), each built with
`--against BUILD_ROOT/base`, and the run reports beside them (<name>.run.json). Prints one line per
measure, `ok` or `OUT` (or `--` for what is printed for the eye), and exits 1 when any is out or a
build is missing.

Where each is read: the lake's level from the `water_height` of `fill#tarn_lake` in features.json;
area, deepest point, islands and shore from lib/review's `step.checks.water_*` records; mean depth
and the shallow share from `water - height` over the posts where water stands within the lake's box;
trees by producer (`trees#...`); bookmarks from manifest.json's outputs and features.json; steps by
their status in the run report. The trail's bed against the bare ground: `height` along the
track's line every half metre, less the ground the world states before any kind writes it,
`survey - 6 hollow + grain`, which `via eval` evaluates in a change (the bookmark on the tarn's
shore stands on the fill's output, which `via eval` does not run, so the change removes it).
"""

import json
import math
import os
import subprocess
import sys
import tempfile
import tomllib
from collections import Counter
from pathlib import Path

import numpy as np

HERE = Path(__file__).resolve().parent
LAKE = "fill#tarn_lake"
BARE = """[field.bare]
unit = "m"
base = "= f('survey') - 6 * f('hollow') + 0.25 * fbm(9, 3, seed = 'grain')"

[feature.on_shore]
remove = true
"""


class Build:
    def __init__(self, d):
        self.d = d
        self.diags = json.loads((d / "diagnostics.json").read_text())
        self.m = json.loads((d / "manifest.json").read_text())
        self.features = {f["name"]: f for f in json.loads((d / "features.json").read_text())["features"]}
        self.names = (d / "instances/names.txt").read_text().split("\n")[:-1]
        run = d.with_name(d.name + ".run.json")
        self.run = json.loads(run.read_text()) if run.exists() else None

    def check(self, code):
        d = [x for x in self.diags["diagnostics"] if x["code"] == f"step.checks.{code}"]
        return d[0]["data"]["value"] if d else None

    def field(self, name):
        return np.load(self.d / "fields" / f"{name}.npy").astype(np.float64)

    def level(self):
        return self.features[LAKE]["attrs"]["water_height"]

    def depths(self):
        r = self.m["frame"]["resolution"]
        ring = [p for piece in self.features[LAKE]["pieces"] for p in piece["outer"]]
        xs, ys = [p[0] for p in ring], [p[1] for p in ring]
        sl = (slice(int(min(ys) / r), int(max(ys) / r) + 2), slice(int(min(xs) / r), int(max(xs) / r) + 2))
        dep = (self.field("water") - self.field("height"))[sl]
        return dep[dep > 1e-6]

    def trees(self):
        return sum(1 for n in self.names if n.startswith("trees#"))

    def bookmarks(self):
        return {n: [round(v, 2) for v in self.features[n]["at"][:2]]
                for n in self.m["outputs"].get("bookmark", [])}

    def placed(self):
        """Each instance as (name, asset, x, y, scale), metres, the asset from the chunks' runs."""
        xs = np.load(self.d / "instances/x.npy") / 1000.0
        ys = np.load(self.d / "instances/y.npy") / 1000.0
        sc = np.load(self.d / "instances/scale.npy")
        asset = {}
        for c in self.m["chunks"]:
            for a, (first, n) in c["instances"].items():
                for row in range(first, first + n):
                    asset[row] = a
        return [(n, asset.get(k, ""), float(xs[k]), float(ys[k]), float(sc[k])) for k, n in enumerate(self.names)]

    def steep_by_houses(self, reach=15.0, limit=40.0):
        """Posts within `reach` m of a house steeper than `limit` degrees (the steepest of each
        post's four differences): a pad's bank that ends in a cliff (lib/land)."""
        h = self.field("height")
        fr = self.m["frame"]
        r, x0, y0 = fr["resolution"], fr["extent"][0], fr["extent"][1]
        d = np.zeros_like(h)
        d[1:-1, 1:-1] = np.maximum.reduce([np.abs(h[1:-1, 1:-1] - h[1:-1, 2:]), np.abs(h[1:-1, 1:-1] - h[1:-1, :-2]),
                                           np.abs(h[1:-1, 1:-1] - h[2:, 1:-1]), np.abs(h[1:-1, 1:-1] - h[:-2, 1:-1])])
        steep = np.degrees(np.arctan(d / r)) > limit
        jj, ii = np.mgrid[0:h.shape[0], 0:h.shape[1]]
        n = 0
        for name, a, x, y, _ in self.placed():
            if a.startswith("house") or a in ("barn", "shed_gable"):
                near = np.hypot(x0 + ii * r - x, y0 + jj * r - y) <= reach
                n += int(np.sum(steep & near))
        return n

    def crowns_over_houses(self):
        """Trees whose crown (plan bounds times scale, as a disc) meets a house's (a disc of its
        half diagonal)."""
        assets = self.m["assets"]
        rows = self.placed()
        houses = []
        for name, a, x, y, _ in rows:
            if a.startswith("house") or a in ("barn", "shed_gable"):
                b = assets[a]["bounds"]
                houses.append((x, y, math.hypot(b[3] - b[0], b[4] - b[1]) / 2))
        over = 0
        for name, a, x, y, scale in rows:
            if a.startswith("tree_"):
                b = assets[a]["bounds"]
                crown = max(b[3] - b[0], b[4] - b[1]) / 2 * scale
                over += any(math.hypot(x - hx, y - hy) < crown + hr for hx, hy, hr in houses)
        return over

    def steps(self, status):
        if self.run is None:
            return None
        return sorted(n["id"][5:] for n in self.run["nodes"] if n["family"] == "step" and n["status"] == status)


def at(a, r, x, y):
    i, j = x / r, y / r
    i0, j0 = int(math.floor(i)), int(math.floor(j))
    fi, fj = i - i0, j - j0
    p = a[j0, i0] * (1 - fi) + a[j0, i0 + 1] * fi
    q = a[j0 + 1, i0] * (1 - fi) + a[j0 + 1, i0 + 1] * fi
    return float(p * (1 - fj) + q * fj)


def bare_ground():
    via = os.environ.get("VIA", "via")
    with tempfile.TemporaryDirectory() as t:
        ch, out = Path(t) / "bare.toml", Path(t) / "bare.npy"
        ch.write_text(BARE)
        subprocess.run([via, "eval", str(HERE / "world.toml"), "--change", str(ch), "--field", "bare",
                        "--out", str(out)], check=True, capture_output=True)
        return np.load(out).astype(np.float64)


def track_to_ground(b):
    r = b.m["frame"]["resolution"]
    h, bare = b.field("height"), bare_ground()
    pts = [p[:2] for p in b.features["track"]["points"]]
    diffs = []
    for p, q in zip(pts, pts[1:]):
        n = max(1, int(math.dist(p, q) / 0.5))
        for k in range(n + 1):
            x, y = p[0] + (q[0] - p[0]) * k / n, p[1] + (q[1] - p[1]) * k / n
            diffs.append(at(h, r, x, y) - at(bare, r, x, y))
    return [round(min(diffs), 2), round(max(diffs), 2)]


out = []


def say(ok, what, v, want):
    out.append(ok is not False)
    mark = "--  " if ok is None else ("ok  " if ok else "OUT ")
    print(f"{mark} {what}: {v}  (want {want})")


def inside(v, r):
    return v is not None and r[0] <= v <= r[1]


def main(root):
    exp = tomllib.loads((HERE / "expect.toml").read_text())
    if not (root / "base/manifest.json").exists():
        print(f"OUT  no base build at {root / 'base'}")
        return 1
    base = Build(root / "base")
    depth = base.depths()
    taken = {
        "tarn_level": base.level(), "tarn_area": base.check("water_area"),
        "tarn_max_depth": base.check("water_depth"), "tarn_mean_depth": round(float(depth.mean()), 2),
        "tarn_shallow_share": round(float((depth < 1).mean()), 2),
        "tarn_islands": base.check("water_islands"), "tarn_shore": base.check("water_shore"),
        "trees": base.trees(), "bothy": base.names.count("bothy"),
        "bothy_relief": base.check("cottage_relief"), "trees_in_clearing": base.check("trees_in_clearing"),
        "steep_by_houses": base.steep_by_houses(), "crowns_over_houses": base.crowns_over_houses(),
    }
    print("== base")
    for mid, e in exp["measure"].items():
        if mid == "bookmarks":
            bm = base.bookmarks()
            say(len(bm) == e["expect"], "bookmarks", len(bm), e["expect"])
            for n, p in bm.items():
                if n in e:
                    say(math.dist(p, e[n]) <= 1.0, f"  {n} at", p, f"{e[n]} within 1 m (preview)")
            continue
        if mid == "track_to_ground":
            v = track_to_ground(base)
            say(e["expect"][0] <= v[0] and v[1] <= e["expect"][1], mid, v, e["expect"])
            continue
        say(inside(taken[mid], e["expect"]), mid, taken[mid], e["expect"])
    for name, e in exp["build"].items():
        if name == "base":
            continue
        d = root / name
        print(f"== {name}")
        if not (d / "manifest.json").exists():
            say(False, "build", f"none at {d}", "a build")
            continue
        b = Build(d)
        c = b.diags["counts"]
        say(b.diags["status"] == e["exit"], "exit", b.diags["status"], e["exit"])
        if "errors" in e:
            say(c["error"] == e["errors"], "errors", c["error"], e["errors"])
        if "warnings" in e:
            say(c["warning"] == e["warnings"], "warnings", c["warning"], e["warnings"])
        codes = Counter(x["code"] for x in b.diags["diagnostics"] if x["severity"] != "info")
        for code in e.get("diagnostics", []):
            say(codes[code] > 0, f"has {code}", codes[code], "> 0")
        for k, status in (("steps_ran", "ran"), ("steps_reused", "reused")):
            if k in e:
                got = b.steps(status)
                say(got is not None and set(e[k]) <= set(got), k, got, e[k])
        if "tarn_level" in e:
            say(inside(b.level(), e["tarn_level"]), "tarn_level", b.level(), e["tarn_level"])
        if "tarn_area" in e:
            say(inside(b.check("water_area"), e["tarn_area"]), "tarn_area", b.check("water_area"), e["tarn_area"])
        if "trees" in e:
            say(inside(b.trees(), e["trees"]), "trees", b.trees(), e["trees"])
        bm, bb = b.bookmarks(), base.bookmarks()
        if "on_shore" in e:
            say(math.dist(bm["on_shore"], e["on_shore"]) <= 1.5, "on_shore", bm["on_shore"],
                f"{e['on_shore']} within 1.5 m")
        for n in e.get("unchanged", []):
            if n == "bothy":
                say("bothy" in b.names, "bothy stands", "bothy" in b.names, True)
            else:
                say(bm.get(n) == bb.get(n), f"{n} unchanged", bm.get(n), bb.get(n))
        if "bookmarks" in e:
            far = max(math.dist(bm[n], bb[n]) for n in bb)
            say(far <= 0.001, "bookmarks moved at most", round(far, 4), "0.001 m")
        if "house_low_instances" in e:
            rv = {x["subject"]: x for x in b.diags["diagnostics"] if x["code"] == "asset.revised"}
            hl, tp = rv.get("asset.house_low"), rv.get("asset.tree_pine")
            say(hl is not None and hl["data"]["instances"] == e["house_low_instances"],
                "house_low instances", hl and hl["data"]["instances"], e["house_low_instances"])
            say(hl is not None and set(e["house_low_facts"]) <= set(hl["data"]["facts"]),
                "house_low facts", hl and hl["data"]["facts"], e["house_low_facts"])
            pines = sum(ch["instances"].get("tree_pine", [0, 0])[1] for ch in b.m["chunks"])
            say(tp is not None and tp["data"]["count"] == pines, "tree_pine instances, every pine",
                tp and tp["data"]["count"], pines)
            say(tp is not None and set(e["tree_pine_facts"]) <= set(tp["data"]["facts"]),
                "tree_pine facts", tp and tp["data"]["facts"], e["tree_pine_facts"])
    return 0 if all(out) else 1


if __name__ == "__main__":
    sys.exit(main(Path(sys.argv[1])))
