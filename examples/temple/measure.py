"""Measures of a temple build, for expect.toml: a colonnade outside a building's walls, the roof
over both, and what the author had to write (docs/plan-kits.md, K8).

    python3 examples/temple/measure.py BUILD [--base BASE] [--world FILE] [--check SECTION]

Prints the measures as JSON. With --check, compares them against expect.toml's rows under SECTION
(Python 3.11+, for tomllib). With --base, also what changed against the base build, by name.
With --world, also what that world's text writes. Reads only the build directory (spec/build.md):
manifest.json, features.json, diagnostics.json and instances/. Standard library only.

How each is taken (ours).

- `buildings`, `instances`; `columns` (instances of an asset tagged `role:post`), `lintels`
  (tagged `storey:colonnade` and `role:wall`), `walls` (tagged `role:wall`, `role:filler` or
  `role:door` with no `storey:` tag), `tiles` (`role:floor`), `roof` (`role:roof_slope` or
  `role:roof_gable`), `base` (`storey:base`).
- **The temple as it stands.** Its outline is the box, in plan, of every column, lintel and
  wall piece (their plans: the box of each asset's measured bounds, turned by the instance's
  `face`). So the same outline is measured whether the columns were laid by the building or
  written by hand beside it. `outline_dm` is its two sides, in decimetres, as `[short, long]`.
- **What covers it.** Points every 0.1 m over that outline (one is a square decimetre).
  `roofed_pct` is the share that the plan of a roof piece, or of a floor tile named for a roof
  (`..._roof_f...`: the flat roof a gable falls back to), holds; `pitched_pct` the share a roof
  slope or a gable half holds; `roof_double_dm2` the points two roof pieces hold.
  `floor_open_dm2` counts the points no floor tile, no column, wall, door or corner piece and
  no base piece (its top is the floor) holds: the floor a walker would see through. A piece's plan is taken 5 mm larger all round, so
  two that abut leave no line of points between them.
- `podium_mm`: the lowest ground storey's `level` less the ground under the outline's corners
  (the mean of `fields/height.npy` at the four posts nearest each): how high the floor stands.
- `columns_low_mm`: the most a column's own `z` stands below that floor. A column written on
  the terrain beside a building with a plinth stands that much lower than the walls.
- **The door.** For the doorway that starts furthest from the building (the one from outside),
  the columns whose middles lie within `AHEAD` (4 m) of its line's end, along it, and within
  3 m of the line across it: `door_clear_mm` is the gap between the nearest column's face on
  each side of the line, 0 where a column stands on the line, -1 where a side has none.
- `mirror_off`: of the wall pieces of each side of each storey's outline (those whose plan's
  middle lies within 0.6 m of the side's line), the pieces whose image (the same distance from the
  side's other end, within 2 mm) is no piece or another asset; a pair of two assets counts two.
  Every side is measured, whether or not a fill asks it to mirror: an asymmetry anywhere counts
  (fix plan F2, its review; as first written only a pair of two assets counted, once, and a side
  ending in a half bay read 0). A side whose two halves are each other's image has none.
- **The way in**, from the build's own meetings (`meets.json`, gap 0), not from `check`:
  `onto_platform`, the doorways that end on a platform's floor; `from_platform`, the doorways
  that end on a storey's floor and start inside a platform's outline. A temple entered through
  its colonnade has one of each. `unreached` below is `check`'s word and says only that the
  room is reached at all: with the platform unread the same walk would start on the terrain and
  the row would still be 0 (K8's review), so the walk over the platform is held by these two,
  and `check`'s own reading of it by lib/building/test_k8fix.py (`ReadPlatform`).
- `errors`, `warnings`: the build's records. `params`, `wall_open`, `roof_none`, `slab_open`,
  `colonnade_none`, `mirror_none` (lay's), `kit_unit`, `kit_pitch`, `kit_stated`, `unreached`, `door_narrow`
  (check's): each code's records, a folded record counted by its `count`. `none_at`:
  `surface.none_at`. `_codes`: every code's count. `built`: 1, or 0 for a build that stopped on
  an error and left its records and nothing else; such a build has only these measures.
- With `--world`: `written_buildings`, the `building` tables in that world's text;
  `written_instances`, its `[instance.<id>]` tables; `written_numbers`, the number literals in
  those tables' lines (comments left out), which is what an author works out and types.
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
AHEAD = 4.0
ROOF = ("role:roof_slope", "role:roof_gable")


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


def ground_of(d, man):
    head, h = _npy(Path(d) / "fields/height.npy")
    rows, cols = (int(v) for v in head.split("'shape': (")[1].split(")")[0].split(",")[:2])
    res = man["frame"]["resolution"]
    ox, oy = man["frame"]["extent"][:2]

    def ground(px, py):
        i = min(max(int(round((px - ox) / res)), 0), cols - 1)
        j = min(max(int(round((py - oy) / res)), 0), rows - 1)
        return h[j * cols + i]
    return ground


def box_of(r, asset, grow=0.0):
    """An instance's plan in world metres as an axis box `(x0, y0, x1, y1)`: the box of its
    asset's measured bounds, turned by its `face` (a quarter turn or none: this kit's pieces
    stand square), `grow` larger all round."""
    f = math.radians(r["face"])
    fx, fy = math.sin(f), math.cos(f)
    rx, ry = math.cos(f), -math.sin(f)
    x0, y0, _, x1, y1, _ = asset["bounds"]
    pts = [(r["x"] + ax * rx + ay * fx, r["y"] + ax * ry + ay * fy) for ax, ay in ((x0, y0), (x1, y0), (x1, y1), (x0, y1))]
    xs, ys = [p[0] for p in pts], [p[1] for p in pts]
    return (min(xs) - grow, min(ys) - grow, max(xs) + grow, max(ys) + grow)


def holds(box, p):
    return box[0] <= p[0] <= box[2] and box[1] <= p[1] <= box[3]


def within(p, ring):
    """True where `p` lies in `ring` or on its edge (a doorway's first point on a platform)."""
    x, y = p
    inside = False
    for (ax, ay), (bx, by) in zip(ring, ring[1:] + ring[:1]):
        if abs((bx - ax) * (y - ay) - (by - ay) * (x - ax)) < 1e-6 and min(ax, bx) - 1e-6 <= x <= max(ax, bx) + 1e-6 \
                and min(ay, by) - 1e-6 <= y <= max(ay, by) + 1e-6:
            return True
        if (ay > y) != (by > y) and x < ax + (bx - ax) * (y - ay) / (by - ay):
            inside = not inside
    return inside


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
    mine = [t for t in tables if t["table"] == "instance" or t["kind"] == "building"]
    return {"written_buildings": sum(1 for t in mine if t["kind"] == "building"),
            "written_instances": sum(1 for t in mine if t["table"] == "instance"),
            "written_numbers": sum(t["numbers"] for t in mine)}


def said(diags, m):
    codes = Counter()
    for x in diags:
        codes[x["code"]] += int(x.get("count", 1))
    m["errors"] = sum(1 for x in diags if x["severity"] == "error")
    m["warnings"] = sum(1 for x in diags if x["severity"] == "warning")
    for code in ("params", "wall_open", "roof_none", "slab_open", "colonnade_none", "mirror_none"):
        m[code] = codes.get(f"step.lay.{code}", 0)
    for code in ("kit_unit", "kit_pitch", "kit_stated", "unreached", "door_narrow"):
        m[code] = codes.get(f"step.check.{code}", 0)
    m["none_at"] = codes.get("surface.none_at", 0)
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

    def has(a, *words):
        return all(w in tags(a) for w in words)

    def storeyed(a):
        return any(t.startswith("storey:") for t in tags(a))
    m = {"built": 1, "buildings": sum(1 for f in feats.values() if f.get("kind") == "building"), "instances": len(rows)}
    groups = {"columns": [], "lintels": [], "walls": [], "tiles": [], "roof": [], "base": [], "corners": [], "rooftiles": []}
    for n, r in rows.items():
        a = r["asset"]
        if not any(t.startswith("role:") for t in tags(a)):
            continue
        b = box_of(r, assets[a], GROW)
        item = (n, r, b)
        if has(a, "role:post"):
            groups["columns"].append(item)
        elif has(a, "storey:colonnade", "role:wall"):
            groups["lintels"].append(item)
        elif has(a, "storey:base"):
            groups["base"].append(item)
        elif has(a, "role:floor"):
            groups["rooftiles" if "_roof_f" in n else "tiles"].append(item)
        elif any(t in ROOF for t in tags(a)):
            groups["roof"].append(item)
        elif not storeyed(a) and has(a, "role:corner"):
            groups["corners"].append(item)
        elif not storeyed(a) and any(has(a, t) for t in ("role:wall", "role:filler", "role:door")):
            groups["walls"].append(item)
    for k in ("columns", "lintels", "walls", "tiles", "roof", "base"):
        m[k] = len(groups[k])
    m["tiles"] += len(groups["rooftiles"])
    standing = groups["columns"] + groups["lintels"] + groups["walls"] + groups["corners"]
    said(diags, m)
    if world:
        m.update(written(world))
    if not standing:
        return m, rows
    x0, y0 = min(b[0] for _, _, b in standing) + GROW, min(b[1] for _, _, b in standing) + GROW
    x1, y1 = max(b[2] for _, _, b in standing) - GROW, max(b[3] for _, _, b in standing) - GROW
    m["outline_dm"] = sorted([int(round((x1 - x0) * 10)), int(round((y1 - y0) * 10))])
    floor_boxes = [b for k in ("tiles", "columns", "walls", "corners", "base") for _, _, b in groups[k]]
    roof_boxes = [b for _, _, b in groups["roof"]]
    flat_boxes = [b for _, _, b in groups["rooftiles"]]
    total = roofed = pitched = twice = floor_open = 0
    i = 0
    while x0 + (i + 0.5) * COVER < x1:
        px = x0 + (i + 0.5) * COVER
        j = 0
        while y0 + (j + 0.5) * COVER < y1:
            p = (px, y0 + (j + 0.5) * COVER)
            k = sum(1 for b in roof_boxes if holds(b, p))
            total += 1
            pitched += k >= 1
            twice += k >= 2
            roofed += k >= 1 or any(holds(b, p) for b in flat_boxes)
            floor_open += not any(holds(b, p) for b in floor_boxes)
            j += 1
        i += 1
    m["roofed_pct"] = round(100.0 * roofed / total, 1)
    m["pitched_pct"] = round(100.0 * pitched / total, 1)
    m["roof_double_dm2"] = twice
    m["floor_open_dm2"] = floor_open
    storeys = sorted((f for f in feats.values() if f.get("kind") == "storey"), key=lambda f: f["attrs"]["level"])
    if storeys:
        ground = ground_of(d, man)
        level = storeys[0]["attrs"]["level"]
        g = sum(ground(x, y) for x, y in ((x0, y0), (x1, y0), (x1, y1), (x0, y1))) / 4
        m["podium_mm"] = int(round((level - g) * 1000))
        m["columns_low_mm"] = int(round(max([level - r["z"] for _, r, _ in groups["columns"]], default=0.0) * 1000))
    doors = [f for f in feats.values() if f.get("kind") == "doorway"]
    # the way in, from the build's own meetings (not from `check`)
    meets_file = Path(d) / "meets.json"
    meets = [x for x in (json.loads(meets_file.read_text())["meets"] if meets_file.exists() else []) if x["gap"] == 0]

    def owner(place):
        return feats.get(place.rpartition(".")[0], {})
    platforms = [[tuple(p[:2]) for p in f["pieces"][0]["outer"]] for f in feats.values() if f.get("kind") == "platform"]
    m["onto_platform"] = sum(1 for x in meets if owner(x["a"]).get("kind") == "doorway" and owner(x["b"]).get("kind") == "platform")
    m["from_platform"] = sum(1 for x in meets if owner(x["a"]).get("kind") == "doorway" and owner(x["b"]).get("kind") == "storey"
                             and any(within(tuple(owner(x["a"])["points"][0][:2]), ring) for ring in platforms))
    if doors:
        cx, cy = (x0 + x1) / 2, (y0 + y1) / 2
        f = max(doors, key=lambda f: math.hypot(f["points"][0][0] - cx, f["points"][0][1] - cy))
        (ax, ay), (bx, by) = f["points"][0][:2], f["points"][-1][:2]
        L = math.hypot(bx - ax, by - ay)
        dx, dy = (bx - ax) / L, (by - ay) / L
        left, right = [], []
        for _, r, b in groups["columns"]:
            q = (r["x"] - bx) * dx + (r["y"] - by) * dy          # along the doorway, from its end
            c = (r["x"] - bx) * -dy + (r["y"] - by) * dx         # across it
            half = (b[2] - b[0]) / 2 - GROW
            if abs(q) > AHEAD or abs(c) > 3.0:
                continue
            if abs(c) < half:
                left, right = [0.0], [0.0]
                break
            (left if c < 0 else right).append(abs(c) - half)
        m["door_clear_mm"] = int(round((min(left) + min(right)) * 1000)) if left and right else -1
    off = 0
    for f in storeys:
        ring = [tuple(p) for p in f["pieces"][0]["outer"]]
        for a, b in zip(ring, ring[1:] + ring[:1]):
            L = math.hypot(b[0] - a[0], b[1] - a[1])
            e = ((b[0] - a[0]) / L, (b[1] - a[1]) / L)
            along = {}
            for _, r, _ in groups["walls"]:
                v = (r["x"] - a[0]) * -e[1] + (r["y"] - a[1]) * e[0]
                if abs(v) <= 0.6 and assets[r["asset"]].get("tags") and "role:door" not in tags(r["asset"]):
                    along[int(round(((r["x"] - a[0]) * e[0] + (r["y"] - a[1]) * e[1]) * 1000))] = r["asset"]
            for u, asset in along.items():
                image = int(round(L * 1000)) - u
                twin = next((along[w] for w in range(image - 2, image + 3) if w in along), None)
                # A piece with nothing at its image is off too (a half bay ending one flank): as first
                # written only a pair of two assets counted, and such a side read 0 (fix plan F2).
                off += twin != asset
    m["mirror_off"] = off
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
