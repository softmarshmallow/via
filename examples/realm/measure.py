"""Measures of a realm build, for expect.toml: three settlements from three kits on land a step
made, joined by ways and a ford, with the player map as data (docs/plan-kits.md, K11).

    python3 examples/realm/measure.py BUILD [--base BASE] [--world FILE] [--run FILE] [--check SECTION]

Prints the measures as JSON. With --check, compares them against expect.toml's rows under SECTION
(Python 3.11+, for tomllib). With --base, also what changed against the base build, by name.
With --world, also what that world and its layers write. With --run, the build's own record of
its time (`BUILD.run.json`, beside the build). Reads only the build directory (spec/build.md):
manifest.json, features.json, meets.json, diagnostics.json, instances/, fields/height.npy,
fields/water.npy and, where the map was exported, export/map/. Standard library only.

How each is taken (ours).

- `built`: 1, or 0 for a build that stopped on an error and left its records and nothing else;
  such a build has only the records' measures.
- **Counts.** `features`, `instances`; `laid_features`, the features no one wrote (a `#` in the
  name: what a step returned); `buildings` (kind `building`), `halls` (those of one storey or
  more), `storeys`, `plots`, `doors` (kind `doorway`).
- **The ground under a wall.** For each hall, a point every 0.5 m along each side of its
  outline (from 0.25 m in). A point 0.4 m outside which lies in another hall is a party wall and
  is left out. `hang` is the ground storey's floor (its `level`) less the ground there
  (height.npy, read between its four posts). `foot_max_mm`: the greatest hang; `foot_bare_dm`:
  the outline, in decimetres, whose hang is more than the depth of its kit's base (the height of
  that kit's `role:wall` piece tagged `storey:base`; 0 with none) and 0.05 m; `buried_max_mm`:
  the most the ground stands over a floor. Each over all halls.
- **The court.** `court_floor_spread_mm`: the highest less the lowest ground floor among the
  halls a `court` step returned (names `courts#...`). `yard_z_mm`: the spread of the `z` of the
  yard wall's pieces (assets of the taxon the `storeys = 0` building names as its kit).
- **The plots.** `plot_off_mm`: over every post inside a `plot`'s region (height.npy's posts),
  the most the ground stands off the plot's `level`, leaving out posts within 4 m of a doorway's
  line (a sill raises the ground to the floor there, after the plot, by design).
- **Water.** Depth is water.npy less height.npy at a point. `ford_depth_mm`: at the `ford`.
  `pieces_in_water`: instances whose point stands in more than 0.05 m; `halls_in_water`: halls
  with such a point on their outline; `ways_wet_dm`: decimetres of `road`, `street` and `trail`
  whose line stands in more than 0.3 m (a walker's `wade`), further than 12 m from a ford.
- **The ways.** For the ways named `street`, `east_road`, `senhal_road` and `fane_path`, where
  the build has them: `<name>_m`, its length in plan; `<name>_grade_pct`, the steepest rise
  over any 10 m of it (its points carry their heights). `walk_m`: the three from the village to
  the court together. `ways_in_halls_dm`: decimetres of any way's line inside a hall's outline.
- **Doors.** `doors_met`: the doorways whose end has a meeting in meets.json with a gap of 0.
- **The walk.** lib/review's reach lint says one record (`step.reach.walked`): `walk_pairs`, the
  pairs of places the world states, and `walk_held`, those a walker makes (grade 0.5, a step of
  0.2 m, 0.6 m wide, wading 0.3 m: the world's profile). `walk_cut`: its warnings.
- **The map.** `map_labels` (labels.json's entries with a text), `map_ways`, `map_water`,
  `map_regions` (vectors.json), `map_floors` (map.json); absent with no export.
- `errors`, `warnings`: the build's records. `surface_gap`, `unowned_crossing`, `keep_out`,
  `max_change`, `squeezed`, `conflict`, `internal`, `outside_field` (the core's); `base_short`,
  `foot_open`, `wall_open`, `roof_none`, `slab_open`, `params` (lay's); `court_meets` (the court
  step's); `unreached`, `kit_stated`, `plot_lost` (check's): each code's records, a folded record
  counted by its `count`. `_codes`: every code's count.
- With `--world`: `written_features`, the `feature` tables in that world's text and its
  layers'; `written_numbers`, the number literals in their lines (comments left out), which is
  what an author works out and types; `written_numbers_places`, those of the places and ways
  layers alone (the settlements and what joins them: not the views).
- With `--run`: `build_ms`, `nodes`, `nodes_ran`.
"""

import json
import math
import re
import struct
import sys
from collections import Counter
from pathlib import Path

STEP = 0.5
SEEN = 0.05
WADE = 0.3
WAYS = ("road", "street", "trail")
NAMED = ("street", "east_road", "senhal_road", "fane_path")
CORE = {"surface_gap": "surface.gap", "unowned_crossing": "geom.unowned_crossing", "keep_out": "place.keep_out",
        "max_change": "authority.max_change", "squeezed": "authority.squeezed", "conflict": "authority.conflict",
        "internal": "diag.internal", "outside_field": "formula.outside_field"}


def _npy(path):
    raw = Path(path).read_bytes()
    hlen = struct.unpack("<H", raw[8:10])[0]
    head = raw[10:10 + hlen].decode()
    body = raw[10 + hlen:]
    return head, struct.unpack(f"<{len(body) // 4}{'f' if '<f4' in head else 'i'}", body)


def field_of(d, man, name):
    """A scalar field as a function of a point, read between its four posts."""
    head, h = _npy(Path(d) / f"fields/{name}.npy")
    rows, cols = (int(v) for v in head.split("'shape': (")[1].split(")")[0].split(",")[:2])
    res = man["frame"]["resolution"]
    ox, oy = man["frame"]["extent"][:2]

    def at(px, py):
        i = min(max((px - ox) / res, 0.0), cols - 1.000001)
        j = min(max((py - oy) / res, 0.0), rows - 1.000001)
        i0, j0 = int(i), int(j)
        u, v = i - i0, j - j0
        return ((h[j0 * cols + i0] * (1 - u) + h[j0 * cols + i0 + 1] * u) * (1 - v)
                + (h[(j0 + 1) * cols + i0] * (1 - u) + h[(j0 + 1) * cols + i0 + 1] * u) * v)
    return at


def instances(d, man):
    d = Path(d)
    names = [n for n in (d / "instances/names.txt").read_text().splitlines() if n]
    x, y = ([v / 1000.0 for v in _npy(d / f"instances/{k}.npy")[1]] for k in ("x", "y"))
    z, face = list(_npy(d / "instances/z.npy")[1]), list(_npy(d / "instances/face.npy")[1])
    asset = [None] * len(names)
    for ch in man["chunks"]:
        for a, (s, n) in ch.get("instances", {}).items():
            for k in range(s, s + n):
                asset[k] = a
    return {n: {"asset": asset[k], "x": x[k], "y": y[k], "z": z[k], "face": face[k]} for k, n in enumerate(names)}


def inside(ring, p):
    x, y, odd = p[0], p[1], False
    for (x0, y0), (x1, y1) in zip(ring, ring[1:] + ring[:1]):
        if (y0 > y) != (y1 > y) and x < x0 + (y - y0) * (x1 - x0) / (y1 - y0):
            odd = not odd
    return odd


def key_of(name):
    return "".join(c if c.isalnum() or c in "_-" else "_" for c in str(name))


def area2(ring):
    return sum(a[0] * b[1] - b[0] * a[1] for a, b in zip(ring, ring[1:] + ring[:1]))


def outline(ring, step=STEP):
    """Points along a ring with the unit normal out of it at each: [(x, y, nx, ny, metres)]."""
    turn = 1.0 if area2(ring) > 0 else -1.0
    out = []
    for a, b in zip(ring, ring[1:] + ring[:1]):
        L = math.hypot(b[0] - a[0], b[1] - a[1])
        if L < 1e-9:
            continue
        e = ((b[0] - a[0]) / L, (b[1] - a[1]) / L)
        n = max(1, int(round(L / step)))
        for i in range(n):
            s = (i + 0.5) * L / n
            out.append((a[0] + e[0] * s, a[1] + e[1] * s, e[1] * turn, -e[0] * turn, L / n))
    return out


def stretch(points, keep):
    """Metres of a line whose pieces' middles satisfy `keep`."""
    total = 0.0
    for a, b in zip(points, points[1:]):
        if keep(((a[0] + b[0]) / 2, (a[1] + b[1]) / 2)):
            total += math.hypot(b[0] - a[0], b[1] - a[1])
    return total


def steepest(points, over=10.0):
    """The steepest rise over run across any `over` metres of a line whose points carry z."""
    s = [0.0]
    for a, b in zip(points, points[1:]):
        s.append(s[-1] + math.hypot(b[0] - a[0], b[1] - a[1]))
    best, j = 0.0, 0
    for i in range(len(points)):
        while j < len(points) - 1 and s[j] - s[i] < over:
            j += 1
        if s[j] - s[i] >= over:
            best = max(best, abs(points[j][2] - points[i][2]) / (s[j] - s[i]))
    return best


def written(world):
    world = Path(world)
    text = world.read_text()
    files = {"world": world}
    for name, f in re.findall(r'\[layer\.([a-z_]+)\]\s*\n\s*file\s*=\s*"([^"]+)"', text):
        files[name] = world.parent / f
    tables, per = 0, Counter()
    for layer, path in files.items():
        cur = False
        for line in path.read_text().splitlines():
            line = line.split("#", 1)[0].rstrip()
            if line.strip().startswith("["):
                cur = bool(re.match(r"\[feature\.[A-Za-z0-9_]+\]", line.strip()))
                tables += cur
                continue
            if cur and line.strip():
                per[layer] += len(re.findall(r"(?<![A-Za-z_\"'])-?\d+(?:\.\d+)?", line.split("=", 1)[-1]))
    return {"written_features": tables, "written_numbers": sum(per.values()),
            "written_numbers_places": per["places"] + per["ways"]}


def said(diags, m):
    codes = Counter()
    for x in diags:
        codes[x["code"]] += int(x.get("count", 1))
    m["errors"] = sum(1 for x in diags if x["severity"] == "error")
    m["warnings"] = sum(1 for x in diags if x["severity"] == "warning")
    for k, code in CORE.items():
        m[k] = codes.get(code, 0)
    for code in ("base_short", "foot_open", "wall_open", "roof_none", "slab_open", "params"):
        m[code] = codes.get(f"step.lay.{code}", 0)
    m["court_meets"] = codes.get("step.courts.court_meets", 0)
    for code in ("unreached", "kit_stated", "plot_lost"):
        m[code] = codes.get(f"step.check.{code}", 0)
    walked = [x for x in diags if x["code"] == "step.reach.walked"]
    if walked:
        m["walk_pairs"], m["walk_held"] = walked[0]["data"]["pairs"], walked[0]["data"]["held"]
    m["walk_cut"] = sum(int(x.get("count", 1)) for x in diags
                        if x["code"].startswith("step.reach.") and x["severity"] == "warning")
    m["_codes"] = dict(sorted(codes.items()))


def measures(d, world=None, run=None):
    d = Path(d)
    diags = json.loads((d / "diagnostics.json").read_text())["diagnostics"]
    m = {"built": int((d / "manifest.json").exists())}
    said(diags, m)
    if world:
        m.update(written(world))
    if run and Path(run).exists():
        t = json.loads(Path(run).read_text())["totals"]
        m["build_ms"], m["nodes"], m["nodes_ran"] = t["ms"], t["nodes"], t["ran"]
    if not m["built"]:
        return m, {}
    man = json.loads((d / "manifest.json").read_text())
    feats = json.loads((d / "features.json").read_text())["features"]
    rows = instances(d, man)
    ground, water = field_of(d, man, "height"), field_of(d, man, "water")
    by = {}
    for f in feats:
        by.setdefault(f.get("kind"), []).append(f)
    named = {f["name"]: f for f in feats}

    def depth(p):
        return water(p[0], p[1]) - ground(p[0], p[1])

    m["features"], m["instances"] = len(feats), len(rows)
    m["laid_features"] = sum(1 for f in feats if "#" in f["name"])
    buildings = by.get("building", [])
    halls = [b for b in buildings if b["attrs"].get("storeys", 1) >= 1]
    m["buildings"], m["halls"] = len(buildings), len(halls)
    m["storeys"], m["plots"], m["doors"] = len(by.get("storey", [])), len(by.get("plot", [])), len(by.get("doorway", []))
    ring_of = {b["name"]: [tuple(p[:2]) for p in b["pieces"][0]["outer"]] for b in buildings}

    # ---- the ground under a wall
    base = {}
    for a in man["assets"].values():
        if "role:wall" in a["tags"] and "storey:base" in a["tags"]:
            base[a["taxon"]] = a["bounds"][5] - a["bounds"][2]
    worst, bare, buried, wet_halls, floors = 0.0, 0.0, 0.0, 0, {}
    for b in halls:
        s0 = named.get(f"lay#{key_of(b['name'])}_s0")
        if s0 is None:
            continue
        floor = floors[b["name"]] = s0["attrs"]["level"]
        deep, wet = base.get(b["attrs"].get("kit"), 0.0), False
        for x, y, nx, ny, L in outline(ring_of[b["name"]]):
            wet = wet or depth((x, y)) > SEEN
            if any(n != b["name"] and inside(ring_of[n], (x + 0.4 * nx, y + 0.4 * ny)) for n in ring_of
                   if named[n]["attrs"].get("storeys", 1) >= 1):
                continue
            hang = floor - ground(x, y)
            worst, buried = max(worst, hang), max(buried, -hang)
            if hang - deep > SEEN:
                bare += L
        wet_halls += wet
    m["foot_max_mm"], m["foot_bare_dm"], m["buried_max_mm"] = round(worst * 1000), round(bare * 10), round(buried * 1000)
    m["halls_in_water"] = wet_halls

    # ---- the court
    court = [v for n, v in floors.items() if n.startswith("courts#")]
    if court:
        m["court_floor_spread_mm"] = round((max(court) - min(court)) * 1000)
    yard_kits = {b["attrs"].get("kit") for b in buildings if b["attrs"].get("storeys", 1) == 0}
    yz = [r["z"] for r in rows.values() if man["assets"].get(r["asset"], {}).get("taxon") in yard_kits]
    if yz:
        m["yard_z_mm"] = round((max(yz) - min(yz)) * 1000)

    # ---- the plots: the ground under each, against its level, away from the doors' sills
    if by.get("plot"):
        doors = [f["points"] for f in by.get("doorway", [])]

        def near_door(p):
            return any(min(math.hypot(p[0] - q[0], p[1] - q[1]) for q in pts) < 4.0 for pts in doors)
        res, (ox, oy) = man["frame"]["resolution"], man["frame"]["extent"][:2]
        off = 0.0
        for pl in by["plot"]:
            ring = [tuple(p[:2]) for p in pl["pieces"][0]["outer"]]
            xs, ys = [p[0] for p in ring], [p[1] for p in ring]
            i0, i1 = math.ceil((min(xs) - ox) / res), math.floor((max(xs) - ox) / res)
            j0, j1 = math.ceil((min(ys) - oy) / res), math.floor((max(ys) - oy) / res)
            for j in range(j0, j1 + 1):
                for i in range(i0, i1 + 1):
                    p = (ox + i * res, oy + j * res)
                    if inside(ring, p) and not near_door(p):
                        off = max(off, abs(ground(*p) - pl["attrs"]["level"]))
        m["plot_off_mm"] = round(off * 1000)

    # ---- water
    fords = [f["at"] for f in by.get("ford", [])]
    if fords:
        m["ford_depth_mm"] = round(max(depth(f) for f in fords) * 1000)
    m["pieces_in_water"] = sum(1 for r in rows.values() if depth((r["x"], r["y"])) > SEEN)
    ways = [f for k in WAYS for f in by.get(k, [])]

    def deep_water(p):
        return depth(p) > WADE and all(math.hypot(p[0] - f[0], p[1] - f[1]) > 12 for f in fords)
    m["ways_wet_dm"] = round(sum(stretch(w["points"], deep_water) for w in ways) * 10)
    m["ways_in_halls_dm"] = round(sum(stretch(w["points"], lambda p: any(inside(ring_of[b["name"]], p) for b in halls))
                                      for w in ways) * 10)

    # ---- the ways
    for n in NAMED:
        if n in named:
            pts = named[n]["points"]
            m[f"{n}_m"] = round(stretch(pts, lambda p: True))
            m[f"{n}_grade_pct"] = round(steepest(pts) * 100)
    if all(f"{n}_m" in m for n in NAMED[:3]):
        m["walk_m"] = sum(m[f"{n}_m"] for n in NAMED[:3])

    # ---- doors
    meets = json.loads((d / "meets.json").read_text())["meets"] if (d / "meets.json").exists() else []
    ends = {x["a"] for x in meets if x["gap"] == 0} | {x["b"] for x in meets if x["gap"] == 0}
    m["doors_met"] = sum(1 for f in by.get("doorway", []) if f["name"] + ".end" in ends)

    # ---- the map
    mp = d / "export/map"
    if (mp / "map.json").exists():
        labels = json.loads((mp / "labels.json").read_text())
        labels = labels.get("labels", labels) if isinstance(labels, dict) else labels
        vec = json.loads((mp / "vectors.json").read_text())
        m["map_labels"] = sum(1 for x in labels if x.get("text"))
        m["map_ways"], m["map_water"], m["map_regions"] = len(vec.get("ways", [])), len(vec.get("water", [])), len(vec.get("regions", []))
        m["map_floors"] = len(json.loads((mp / "map.json").read_text()).get("floors", []))
    return m, rows


def against(rows, base):
    both = set(rows) & set(base)
    return {"added": len(set(rows) - set(base)), "removed": len(set(base) - set(rows)),
            "moved": sum(1 for n in both if any(rows[n][k] != base[n][k] for k in ("x", "y", "z", "face", "asset")))}


def main(argv):
    args = argv[1:]
    opt = {}
    for flag in ("--base", "--check", "--world", "--run"):
        if flag in args:
            i = args.index(flag)
            opt[flag] = args[i + 1]
            del args[i:i + 2]
    if len(args) != 1:
        print(__doc__.split("\n\n")[1], file=sys.stderr)
        return 2
    m, rows = measures(args[0], opt.get("--world"), opt.get("--run"))
    if "--base" in opt and rows:
        m.update(against(rows, measures(opt["--base"])[1]))
    check = opt.get("--check")
    if not check:
        print(json.dumps(m, indent=1, sort_keys=True))
        return 0
    import tomllib
    table = tomllib.loads((Path(__file__).resolve().parent / "expect.toml").read_text()).get(check)
    if not table:
        print(f"no section `{check}` in expect.toml", file=sys.stderr)
        return 2
    bad = 0
    for key, row in table.items():
        v = m.get(row["measure"])
        lo, hi = row["band"]
        ok = v is not None and lo <= v <= hi
        bad += not ok
        print(f"{'ok  ' if ok else 'FAIL'} {check}.{key}: {row['measure']} = {v} in [{lo}, {hi}]")
    print(f"{len(table) - bad}/{len(table)} rows")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
