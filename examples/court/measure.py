"""Measures of a court build, for expect.toml: halls laid by posts under roofs that stand beyond
their walls, a yard's wall on the ground, and a plan that is one half and its image
(docs/plan-kits.md, K8).

    python3 examples/court/measure.py BUILD [--base BASE] [--world FILE] [--check SECTION]

Prints the measures as JSON. With --check, compares them against expect.toml's rows under SECTION
(Python 3.11+, for tomllib). With --base, also what changed against the base build, by name.
With --world, also what that world's text writes. Reads only the build directory (spec/build.md):
manifest.json, features.json, diagnostics.json and instances/. Standard library only.

How each is taken (ours).

- `buildings`, `halls` (buildings of one storey or more), `yards` (`yard` features), `instances`;
  `posts` (instances of an asset tagged `role:post`), `panels` (tagged `role:wall` with no
  `storey:` tag, of the halls' kit), `tiles` (`role:floor`), `roof` (`role:roof_slope` or
  `role:roof_hip`), `base` (`storey:base`), `yard_wall` (pieces of the yard's kit: the taxon of
  the asset the gate is).
- **A piece's plan** is the box of its asset's measured bounds, placed by the instance's `at` and
  turned by its `face` to any angle. A point is held by a piece when it lies in that box grown
  5 mm, so two pieces that abut leave no line of points between them.
- **The roofs.** For each `roof` feature (the outline a roof was laid on, the eaves included),
  points every 0.1 m over its region (one is a square decimetre). `roof_dm2` is their number
  over all roofs; `roof_open_dm2` those no roof piece holds; `roof_double_dm2` those two hold;
  `roofed_pct` the share one or more holds. `pitched` is the roofs that hold a slope or a hip,
  `flat` those that hold floor tiles named for a roof (`..._roof_f...`: the flat roof a pitched
  one falls back to).
- `eaves_mm`: for each hall with a roof, half of what the roof's region is longer than the
  hall's outline, the least of the two directions; the least over the halls. 0 for a roof on
  the outline.
- **The floors.** Points every 0.1 m over each hall's outline. `floor_open_dm2` counts those no
  floor tile, post, panel, door or corner piece and no base piece (its top is the floor) holds.
- **The mirror.** The axis is the line through the gate (the instance of the yard kit's
  `role:door` piece) along its `face`. Every other instance is reflected about it and the
  nearest instance of the same asset found: `mirror_mm` is the furthest any image is from its
  twin, in whole millimetres, and `mirror_alone` the instances with no twin within 50 mm. A
  build with no gate has neither. A panel's twin is a panel of the same asset, so the measure
  sees a pattern turned about where it should be reflected only where a wall's pattern has
  more than one word: the wings' ends and backs do (K8's review: with every pattern one word,
  `mirror_alone` was 0 with `court.py`'s `mirrored` taken out).
- **The yard on the land.** `yard_z_mm` is the spread (highest less lowest) of the `z` of the
  yard wall's pieces; `hall_z_mm` the greatest spread of the `z` of one hall's posts. A wall on
  the ground follows it; a hall stands level.
- `errors`, `warnings`: the build's records. `params`, `roof_none`, `slab_open`, `wall_open`,
  `base_short`, `mirror_none` (lay's), `court_params`, `court_meets` (the court step's), `kit_stated`,
  `kit_deck`, `kit_pitch`, `kit_module` (check's), `crowded` (`place.crowded`): each code's
  records, a folded record counted by its `count`. `_codes`: every code's count. `built`: 1,
  or 0 for a build that stopped on an error and left its records and nothing else; such a build
  has only these measures.
- With `--world`: `written_buildings`, the `building` tables in that world's text;
  `written_courts`, its `court` tables; `written_numbers`, the number literals in those tables'
  lines (comments left out), which is what an author works out and types.
"""

import json
import math
import re
import struct
import sys
from collections import Counter
from pathlib import Path

COVER = 0.1
GROW = 0.005
TWIN = 0.05
ROOF = ("role:roof_slope", "role:roof_hip")


def _npy(path):
    raw = Path(path).read_bytes()
    hlen = struct.unpack("<H", raw[8:10])[0]
    head = raw[10:10 + hlen].decode()
    body = raw[10 + hlen:]
    return head, struct.unpack(f"<{len(body) // 4}{'f' if '<f4' in head else 'i'}", body)


def load(d):
    d = Path(d)
    man = json.loads((d / "manifest.json").read_text())
    diags = json.loads((d / "diagnostics.json").read_text())["diagnostics"]
    names = [n for n in (d / "instances/names.txt").read_text().splitlines() if n]
    x, y, face = ([v / 1000.0 for v in _npy(d / f"instances/{k}.npy")[1]] for k in ("x", "y", "face"))
    z = list(_npy(d / "instances/z.npy")[1])
    asset = [None] * len(names)
    for ch in man["chunks"]:
        for a, (s, n) in ch.get("instances", {}).items():
            for k in range(s, s + n):
                asset[k] = a
    rows = {n: {"asset": asset[k], "x": x[k], "y": y[k], "z": z[k], "face": face[k]} for k, n in enumerate(names)}
    return man, diags, rows


def plan_of(r, asset, grow=GROW):
    """An instance's plan: a function of a point that says whether the piece holds it. The box of
    the asset's measured bounds in the piece's own frame (x to its right, y its front), `grow`
    larger all round, turned by `face` (degrees clockwise from north)."""
    f = math.radians(r["face"])
    fx, fy = math.sin(f), math.cos(f)            # the front
    rx, ry = math.cos(f), -math.sin(f)           # the asset's +x
    x0, y0, _, x1, y1, _ = asset["bounds"]
    ox, oy = r["x"], r["y"]

    def holds(p):
        dx, dy = p[0] - ox, p[1] - oy
        a, b = dx * rx + dy * ry, dx * fx + dy * fy
        return x0 - grow <= a <= x1 + grow and y0 - grow <= b <= y1 + grow
    reach = math.hypot(max(abs(x0), abs(x1)), max(abs(y0), abs(y1))) + grow
    return holds, (ox - reach, oy - reach, ox + reach, oy + reach)


def inside(ring, p):
    n, hit = len(ring), False
    for i in range(n):
        (x0, y0), (x1, y1) = ring[i][:2], ring[(i + 1) % n][:2]
        if (y0 > p[1]) != (y1 > p[1]) and p[0] < x0 + (p[1] - y0) * (x1 - x0) / (y1 - y0):
            hit = not hit
    return hit


def points(ring):
    """Points every `COVER` over a ring, in its own box."""
    xs, ys = [p[0] for p in ring], [p[1] for p in ring]
    i = 0
    while min(xs) + (i + 0.5) * COVER < max(xs):
        px = min(xs) + (i + 0.5) * COVER
        j = 0
        while min(ys) + (j + 0.5) * COVER < max(ys):
            p = (px, min(ys) + (j + 0.5) * COVER)
            if inside(ring, p):
                yield p
            j += 1
        i += 1


def held(p, plans):
    """How many of `plans` hold the point."""
    return sum(1 for holds, b in plans if b[0] <= p[0] <= b[2] and b[1] <= p[1] <= b[3] and holds(p))


def key_of(name):
    return "".join(c if c.isalnum() or c in "_-" else "_" for c in str(name))


def sides(ring):
    return [math.dist(ring[i][:2], ring[(i + 1) % len(ring)][:2]) for i in range(len(ring))]


def written(world):
    tables, cur = [], None
    for line in Path(world).read_text().splitlines():
        line = line.split("#", 1)[0].rstrip()
        if line.strip().startswith("["):
            cur = None
            head = re.match(r"\[(feature|instance)\.[A-Za-z0-9_]+\]", line.strip())
            if head:
                cur = {"table": head.group(1), "kind": None, "numbers": 0}
                tables.append(cur)
            continue
        if cur is None or not line.strip():
            continue
        k = re.match(r'\s*kind\s*=\s*"([a-z_]+)"', line)
        if k:
            cur["kind"] = k.group(1)
        cur["numbers"] += len(re.findall(r"(?<![A-Za-z_\"])-?\d+(?:\.\d+)?", line.split("=", 1)[-1]))
    mine = [t for t in tables if t["kind"] in ("building", "court")]
    return {"written_buildings": sum(1 for t in mine if t["kind"] == "building"),
            "written_courts": sum(1 for t in mine if t["kind"] == "court"),
            "written_numbers": sum(t["numbers"] for t in mine)}


def said(diags, m):
    codes = Counter()
    for x in diags:
        codes[x["code"]] += int(x.get("count", 1))
    m["errors"] = sum(1 for x in diags if x["severity"] == "error")
    m["warnings"] = sum(1 for x in diags if x["severity"] == "warning")
    for code in ("params", "roof_none", "slab_open", "wall_open", "base_short", "mirror_none"):
        m[code] = codes.get(f"step.lay.{code}", 0)
    m["court_params"] = codes.get("step.courts.params", 0)
    m["court_meets"] = codes.get("step.courts.court_meets", 0)
    for code in ("kit_stated", "kit_deck", "kit_pitch", "kit_module"):
        m[code] = codes.get(f"step.check.{code}", 0)
    m["crowded"] = codes.get("place.crowded", 0)
    m["_codes"] = dict(sorted(codes.items()))


def measures(d, world=None):
    if not (Path(d) / "manifest.json").exists():
        # A build that stopped on an error leaves its records and nothing else.
        m = {"built": 0}
        said(json.loads((Path(d) / "diagnostics.json").read_text())["diagnostics"], m)
        return m, {}
    man, diags, rows = load(d)
    assets = man["assets"]
    feats = {f["name"]: f for f in json.loads((Path(d) / "features.json").read_text())["features"]}

    def tags(a):
        return assets[a].get("tags") or []

    def ring(f):
        return [tuple(p[:2]) for p in f["pieces"][0]["outer"]]
    buildings = {n: f for n, f in feats.items() if f.get("kind") == "building"}
    halls = {n: f for n, f in buildings.items() if int(f["attrs"].get("storeys", 1)) >= 1}
    roofs = {n: f for n, f in feats.items() if f.get("kind") == "roof"}
    m = {"built": 1, "buildings": len(buildings), "halls": len(halls),
         "yards": sum(1 for f in feats.values() if f.get("kind") == "yard"), "instances": len(rows)}
    yard_taxa = {f["attrs"].get("kit") for n, f in buildings.items() if n not in halls}
    gates = [r for r in rows.values() if "role:door" in tags(r["asset"]) and assets[r["asset"]].get("taxon") in yard_taxa]
    groups = {"posts": [], "panels": [], "tiles": [], "roof": [], "base": [], "yard_wall": [], "floor": [], "rooftiles": []}
    for n, r in rows.items():
        a = r["asset"]
        t = tags(a)
        item = (n, r, plan_of(r, assets[a]))
        storeyed = any(x.startswith("storey:") for x in t)
        if assets[a].get("taxon") in yard_taxa:
            groups["yard_wall"].append(item)
        elif "role:post" in t:
            groups["posts"].append(item)
            groups["floor"].append(item)
        elif "storey:base" in t:
            groups["base"].append(item)
            groups["floor"].append(item)
        elif "role:floor" in t:
            groups["rooftiles" if "_roof_f" in n else "tiles"].append(item)
            if "_roof_f" not in n:
                groups["floor"].append(item)
        elif any(x in ROOF for x in t):
            groups["roof"].append(item)
        elif not storeyed and any(x in t for x in ("role:wall", "role:door", "role:corner")):
            groups["panels" if "role:wall" in t else "floor"].append(item)
            if "role:wall" in t:
                groups["floor"].append(item)
    for k in ("posts", "panels", "tiles", "roof", "base", "yard_wall"):
        m[k] = len(groups[k])
    m["tiles"] += len(groups["rooftiles"])
    said(diags, m)
    if world:
        m.update(written(world))
    # the roofs
    roof_plans = [pl for _, _, pl in groups["roof"]]
    total = opened = twice = 0
    pitched = flat = 0
    eaves = []
    for n, f in sorted(roofs.items()):
        head = n.split("#", 1)[-1]
        pitched += any(x.split("#", 1)[-1].startswith(head + "_") for x, _, _ in groups["roof"])
        flat += any(x.split("#", 1)[-1].startswith(head + "_f") for x, _, _ in groups["rooftiles"])
        for p in points(ring(f)):
            k = held(p, roof_plans)
            total += 1
            opened += k == 0
            twice += k >= 2
        owner = next((b for bn, b in halls.items() if key_of(bn) + "_roof" == head), None)
        if owner is not None:
            a, b = sorted(sides(ring(f))), sorted(sides(ring(owner)))
            eaves.append(min((a[0] - b[0]) / 2, (a[-1] - b[-1]) / 2))
    m["roofs"], m["pitched"], m["flat"] = len(roofs), pitched, flat
    m["roof_dm2"], m["roof_open_dm2"], m["roof_double_dm2"] = total, opened, twice
    m["roofed_pct"] = round(100.0 * (total - opened) / total, 1) if total else 0.0
    if eaves:
        m["eaves_mm"] = int(round(min(eaves) * 1000))
    # the floors
    floor_plans = [pl for _, _, pl in groups["floor"]]
    m["floor_open_dm2"] = sum(1 for f in halls.values() for p in points(ring(f)) if not held(p, floor_plans))
    # the mirror
    if len(gates) == 1:
        g = gates[0]
        f = math.radians(g["face"])
        ax, ay = math.sin(f), math.cos(f)          # along the axis
        by_asset = {}
        for r in rows.values():
            by_asset.setdefault(r["asset"], []).append((r["x"], r["y"]))
        worst, alone = 0.0, 0
        for r in rows.values():
            dx, dy = r["x"] - g["x"], r["y"] - g["y"]
            along, across = dx * ax + dy * ay, dx * ay - dy * ax
            image = (g["x"] + along * ax - across * ay, g["y"] + along * ay + across * ax)
            near = min(math.dist(image, q) for q in by_asset[r["asset"]])
            if near > TWIN:
                alone += 1
            else:
                worst = max(worst, near)
        m["mirror_mm"], m["mirror_alone"] = int(round(worst * 1000)), alone
    # the yard on the land
    if groups["yard_wall"]:
        zs = [r["z"] for _, r, _ in groups["yard_wall"]]
        m["yard_z_mm"] = int(round((max(zs) - min(zs)) * 1000))
    spread = 0.0
    for bn in halls:
        head = key_of(bn) + "_"
        zs = [r["z"] for n, r, _ in groups["posts"] if n.split("#", 1)[-1].startswith(head)]
        if zs:
            spread = max(spread, max(zs) - min(zs))
    m["hall_z_mm"] = int(round(spread * 1000))
    return m, rows


def against(rows, base):
    both = set(rows) & set(base)
    return {"added": len(set(rows) - set(base)), "removed": len(set(base) - set(rows)),
            "moved": sum(1 for n in both if any(rows[n][k] != base[n][k] for k in ("x", "y", "z", "face", "asset")))}


def main(argv):
    args = argv[1:]
    opt = {}
    for flag in ("--base", "--check", "--world"):
        if flag in args:
            i = args.index(flag)
            opt[flag] = args[i + 1]
            del args[i:i + 2]
    if len(args) != 1:
        print(__doc__.split("\n\n")[1], file=sys.stderr)
        return 2
    m, rows = measures(args[0], opt.get("--world"))
    if "--base" in opt:
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
        ok = v is not None and (lo <= v <= hi if not isinstance(v, list) else [lo, hi] == v)
        bad += not ok
        print(f"{'ok  ' if ok else 'FAIL'} {check}.{key}: {row['measure']} = {v} in [{lo}, {hi}]")
    print(f"{len(table) - bad}/{len(table)} rows")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
