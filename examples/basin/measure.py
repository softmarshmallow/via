"""Basin's measures, read from its build directories and held to expect.toml (Python 3.11+, NumPy).

    python examples/basin/measure.py BUILD_ROOT    # BUILD_ROOT holds base/, move_haven/, ... by change name

Each [measure.<id>] of expect.toml names the build it reads. A measure with a number or a point and
`within` is compared with its `oracle`; the others are printed for the eye. Prints one line per
measure, `ok`, `OUT` or `--` (printed, not compared), and exits 1 when any compared measure is out
or a build it names is missing.

How each is taken: a course point at `t` is the point at that share of the length of the path of
`terrain#river_course` in features.json, in plan; ground is `height` bilinear between posts; a level
is the mean of `water` over the posts inside the tarn's region; findings are diagnostics.json's
counts; instances are counted by their producer (`woods#...`) and asset (manifest chunks).
"""

import json
import math
import sys
import tomllib
from collections import Counter
from pathlib import Path

import numpy as np

HERE = Path(__file__).resolve().parent
COURSE = "terrain#river_course"


class Build:
    def __init__(self, d):
        self.d = d
        self.m = json.loads((d / "manifest.json").read_text()) if (d / "manifest.json").exists() else None
        self.diags = json.loads((d / "diagnostics.json").read_text())
        self.features = (
            {f["name"]: f for f in json.loads((d / "features.json").read_text())["features"]}
            if self.m
            else {}
        )
        ch = d / "changes.json"
        self.changes = json.loads(ch.read_text()) if ch.exists() else None
        self._h = None

    def field(self, name):
        return np.load(self.d / "fields" / f"{name}.npy").astype(np.float64)

    def ground(self, x, y):
        if self._h is None:
            self._h = self.field("height")
        f = self.m["frame"]
        r = f["resolution"]
        i, j = (x - f["extent"][0]) / r, (y - f["extent"][1]) / r
        i0, j0 = int(math.floor(i)), int(math.floor(j))
        fi, fj = i - i0, j - j0
        h = self._h
        a = h[j0, i0] * (1 - fi) + h[j0, i0 + 1] * fi
        b = h[j0 + 1, i0] * (1 - fi) + h[j0 + 1, i0 + 1] * fi
        return float(a * (1 - fj) + b * fj)

    def course_at(self, t):
        f = self.features.get(COURSE)
        if f is None:
            return None
        p = [(q[0], q[1]) for q in f["points"]]
        seg = [math.dist(a, b) for a, b in zip(p, p[1:])]
        want, acc = t * sum(seg), 0.0
        for (a, b), s in zip(zip(p, p[1:]), seg):
            if acc + s >= want and s > 0:
                u = (want - acc) / s
                return [round(a[0] + (b[0] - a[0]) * u, 1), round(a[1] + (b[1] - a[1]) * u, 1)]
            acc += s
        return [round(p[-1][0], 1), round(p[-1][1], 1)]

    def tarn_level(self):
        f = self.features["tarn"]
        ring = f["pieces"][0]["outer"]
        fr = self.m["frame"]
        r = fr["resolution"]
        xs = [q[0] for q in ring]
        ys = [q[1] for q in ring]
        w = self.field("water")
        vals = []
        for j in range(int(min(ys) / r), int(max(ys) / r) + 1):
            for i in range(int(min(xs) / r), int(max(xs) / r) + 1):
                if inside(ring, i * r, j * r):
                    vals.append(w[j, i])
        return round(float(np.mean(vals)), 2)

    def codes(self, severity=None):
        return [x for x in self.diags["diagnostics"] if severity is None or x["severity"] == severity]

    def asset_count(self, asset):
        return sum(c["instances"].get(asset, [0, 0])[1] for c in self.m["chunks"])

    def steps(self):
        """Each step node's status in the build's run report (`graph.md` 6.3), when it is kept."""
        run = self.d.with_name(self.d.name + ".run.json")
        if not run.exists():
            return None
        nodes = json.loads(run.read_text())["nodes"]
        return {n["id"]: n["status"] for n in nodes if n["family"] == "step"}

    def solids(self):
        """Each solid instance as (name, x, y, half-width of its footprint times its scale), in
        metres; x and y are integer millimetres in the columns (build.md 8)."""
        names = (self.d / "instances/names.txt").read_text().split("\n")[:-1]
        xs = np.load(self.d / "instances/x.npy")
        ys = np.load(self.d / "instances/y.npy")
        sc = np.load(self.d / "instances/scale.npy")
        asset_of = {}
        for c in self.m["chunks"]:
            for a, (first, n) in c["instances"].items():
                for row in range(first, first + n):
                    asset_of[row] = a
        out = []
        for row, n in enumerate(names):
            a = self.m["assets"].get(asset_of.get(row, ""), {})
            if a.get("solidity") != "solid":
                continue
            half = max(max(abs(q[0]), abs(q[1])) for q in a["footprint"]) * float(sc[row])
            out.append((n, xs[row] / 1000.0, ys[row] / 1000.0, half))
        return out

    def in_water(self):
        """Solid instances whose footprint reaches the river's channel (within the course's
        `width / 2` of its line), and those standing in more than 5 cm of water at any corner
        or the centre of their footprint (water less ground, bilinear)."""
        f = self.features.get(COURSE)
        line = [(q[0], q[1]) for q in f["points"]] if f else []
        width = float(f["attrs"].get("width", 4)) if f else 0
        w = self.field("water")
        h = self.field("height")
        fr = self.m["frame"]
        r = fr["resolution"]

        def at(v, x, y):
            i, j = (x - fr["extent"][0]) / r, (y - fr["extent"][1]) / r
            i0, j0 = int(math.floor(i)), int(math.floor(j))
            i0, j0 = min(max(i0, 0), v.shape[1] - 2), min(max(j0, 0), v.shape[0] - 2)
            fi, fj = i - i0, j - j0
            a = v[j0, i0] * (1 - fi) + v[j0, i0 + 1] * fi
            b = v[j0 + 1, i0] * (1 - fi) + v[j0 + 1, i0 + 1] * fi
            return float(a * (1 - fj) + b * fj)

        def to_line(x, y):
            best = math.inf
            for (ax, ay), (bx, by) in zip(line, line[1:]):
                dx, dy = bx - ax, by - ay
                l2 = dx * dx + dy * dy
                t = 0 if l2 == 0 else max(0, min(1, ((x - ax) * dx + (y - ay) * dy) / l2))
                best = min(best, math.hypot(x - ax - t * dx, y - ay - t * dy))
            return best

        on_course, wet = [], []
        for n, x, y, half in self.solids():
            if line and to_line(x, y) - half * math.sqrt(2) < width / 2:
                on_course.append(n)
            pts = [(x, y)] + [(x + sx * half, y + sy * half) for sx in (-1, 1) for sy in (-1, 1)]
            if any(at(w, px, py) - at(h, px, py) > 0.05 for px, py in pts):
                wet.append(n)
        return on_course, wet

    def producers(self):
        names = (self.d / "instances/names.txt").read_text().split("\n")[:-1]
        return Counter(n.split("#")[0] for n in names if "#" in n)


def inside(ring, x, y):
    c = False
    for (x1, y1, *_), (x2, y2, *_) in zip(ring, ring[1:] + ring[:1]):
        if (y1 > y) != (y2 > y) and x < (x2 - x1) * (y - y1) / (y2 - y1) + x1:
            c = not c
    return c


T = {"course_t040": 0.40, "course_t035": 0.35, "course_t085": 0.85, "course_t030": 0.30,
     "mouth_moves": 0.85, "mouth_moves_upper_reach_stays": 0.40, "two_ends_course": 0.40}
AT = {"ground_tarn_view": (1260, 1740), "ground_middle": (1500, 1800)}


def take(mid, e, b, builds):
    """The measured value of one measure, and whether it is compared."""
    if mid in T:
        return b.course_at(T[mid])
    if mid in AT:
        return round(b.ground(*AT[mid]), 2)
    if mid == "ground_bank":
        a = b.features["on_bank"]["at"]
        return round(b.ground(a[0], a[1]), 2)
    if mid == "tarn_level":
        return b.tarn_level()
    if mid in ("findings", "mouth_findings"):
        c = b.diags["counts"]
        return c["error"] + c["warning"]
    if mid == "course_ends":
        f = b.features.get(COURSE)
        return f["attrs"].get("ends") if f else None
    if mid == "cannot_hold_exit":
        return b.diags["status"]
    if mid in ("cannot_hold_lake", "cannot_hold_river"):
        # The finding's position; the river's is compared with the spike's (`within`).
        code = "step.facts.lake_perched" if mid.endswith("lake") else "step.facts.river_uphill"
        d = [x for x in b.codes() if x["code"] == code and x["severity"] == "error"]
        return d[0].get("position") if d else None
    if mid == "reseed_moves":
        base = builds["base"]
        return (f"steps {b.steps()}; ground at (1260, 1740) {b.ground(1260, 1740):.2f} against {base.ground(1260, 1740):.2f}; "
                f"findings {b.diags['counts']['error'] + b.diags['counts']['warning']}; "
                f"course at t = 0.40 {b.course_at(0.4)} against {base.course_at(0.4)}")
    if mid == "revised_oak":
        d = [x for x in b.codes() if x["code"] == "asset.revised" and x["subject"] == "asset.tree_oak"]
        n = len(d[0]["data"]["instances"]) if d else 0
        oaks = b.asset_count("tree_oak")
        entries = len(b.changes["entries"]) if b.changes else None
        return f"asset.revised lists {n} instances, the build has {oaks} oaks; changes.json {entries} entries"
    if mid == "hold_moved":
        return len([x for x in b.codes("error") if x["code"] == "step.facts.hold_moved"])
    if mid == "solid_on_course":
        return len(b.in_water()[0])
    if mid == "solid_in_water":
        return len(b.in_water()[1])
    if mid == "woods":
        return b.producers()["woods"]
    if mid == "thin_woods":
        base = builds["base"]
        moved = sorted({en["table"] + ":" + (en.get("maker") or en["name"].split("#")[0])
                        for en in b.changes["entries"]})
        return (f"steps {b.steps()}; woods {b.producers()['woods']} against {base.producers()['woods']}; "
                f"changed in the report: {', '.join(moved[:12])}")
    return None


EQUAL = {"course_ends", "findings", "mouth_findings", "cannot_hold_exit", "solid_on_course",
         "solid_in_water", "hold_moved"}


def compare(mid, e, v):
    if mid in EQUAL:
        return v == e["oracle"]
    if not isinstance(e.get("within"), (int, float)):
        return None
    if v is None:
        return False
    o = e["oracle"]
    if isinstance(o, str):
        o = [float(t) for t in o.rsplit("(", 1)[1].rstrip(")").split(",")]
    if isinstance(o, list):
        return math.dist(o, v) <= e["within"]
    return abs(v - o) <= e["within"]


def main(root):
    exp = tomllib.loads((HERE / "expect.toml").read_text())
    builds, bad = {}, False
    for mid, e in exp.get("measure", {}).items():
        name = e["build"]
        if name not in builds:
            d = root / name
            builds[name] = Build(d) if (d / "diagnostics.json").exists() else None
        b = builds[name]
        if b is None:
            print(f"OUT  {mid}: no build at {root / name}")
            bad = True
            continue
        v = take(mid, e, b, builds)
        ok = compare(mid, e, v)
        mark = "--  " if ok is None else ("ok  " if ok else "OUT ")
        bad |= ok is False
        print(f"{mark} {mid} [{name}]: {v}  (oracle {e.get('oracle')}, within {e.get('within', '-')})")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main(Path(sys.argv[1])))
