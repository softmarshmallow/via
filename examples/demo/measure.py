"""Measures of a demo build, for expect.toml and the demo's pictures: a market town of Kenney's kit
in a wooded valley (docs/design/demo.md; D of docs/plan-fixes.md). The house measures are
examples/high_street's, taken the same way; the growth's are added.

    python3 examples/demo/measure.py BUILD [--world WORLD] [--base BASE] [--run RUN.json] [--check SECTION]

Prints the measures as JSON. With --check, compares them against expect.toml's rows under SECTION
(Python 3.11+, for tomllib). With --base, what changed against the base build, by name. With
--world, what the author wrote. With --run, the build's time from its run record. Reads only the
build directory (spec/build.md): manifest.json, features.json, diagnostics.json and instances/.
Standard library only.

How each is taken (ours).

- `buildings`: features of kind `building` a step returned (`rows#...`); `rows`: the `row`
  features; `lots_<row>`: the buildings of each row; `storeys_<n>`: buildings of `n` storeys;
  `widest_m`, `narrowest_m`: the longest and shortest front of a building, the side of its
  outline its door stands nearest.
- `left_m_max`: the most any row left unfilled at its end (the `rows` info's `left`, row.py rule
  11), metres.
- `instances`; `blocks` (instances of an asset tagged `role:wall`, `role:corner` or `role:door`
  that is no `storey:parapet` piece), `doors` (`role:door`), `roof_tiles` (`role:roof_flat`),
  `parapet` (`storey:parapet`), by the manifest's asset tags.
- `doors_on_street`: door instances whose middle lies within 1 m of the line 8 m off the high
  street's line or 6.5 m off the church lane's (the frontages of world.toml, 6 m and 4.5 m, and
  half a door block in): every door opens on its street. `doors_off_street` the others.
- `overlap_m2`: the area two buildings' outlines share, summed over every pair (0: no two stand in
  one place); `party_walls`: the sides a building calls `party`.
- `errors`, `warnings`, and each code's records (a folded record by its `count`): `plot_lost`,
  `ground_over`, `crowded` (`place.crowded`), `overlap` (`place.overlap`), `keep_out`
  (`place.keep_out`), `unowned_crossing`, `row_open`, `row_stair`, `row_meets`, `row_bent`,
  `row_short`, `roof_none`,
  `wall_open`, `slab_open`, `untrusted` (`protocol.untrusted`), `remodelled` and `revised`
  (`asset.remodelled`, `asset.revised`: a piece of the pack changed since it was accepted).
- **Written** (`--world`): `written_features` (`[feature.*]` tables in the world and its layers),
  `written_numbers` (numbers in them outside comments, every element of an array counted),
  `hand_placed` (tables there that place a thing by hand: an `[instance.*]` or a `[pin.*]`, and a
  feature of kind `building`; houses come from rows and trees from scatters, so 0 is the claim).
- **Against the base** (`--base`): `added`, `removed`, `moved` instances by name;
  `buildings_added`, `buildings_removed`, `buildings_moved` (a building of one name whose outline
  differs), `buildings_kept`.
- `build_ms`, `nodes`, `nodes_ran` (`--run`): the run record's totals.
- **Growth.** `trees`: instances of the starter pack's trees (`s_tree_*`, taxon `plant/tree`);
  `plants`: of any `plant/...` asset; `trees_on_plots`: trees whose trunk stands inside a house's
  plot (`kind:plot`: the house and 1.5 m round it but along a party wall). Against the base: `trees_added`,
  `trees_removed` by name (a scatter's instance is named by its cell, identity.md 2.2, so a tree
  that stays is the same name) and `trees_kept` (a name in both, at the same x, y and asset; its
  height may differ where the ground was levelled).
"""

import json
import math
import re
import struct
import sys
from collections import Counter
from pathlib import Path

CODES = {"plot_lost": "step.check.plot_lost", "ground_over": "step.check.ground_over",
         "crowded": "place.crowded", "overlap": "place.overlap", "keep_out": "place.keep_out",
         "unowned_crossing": "geom.unowned_crossing", "row_open": "step.rows.row_open",
         "row_stair": "step.rows.row_stair", "row_meets": "step.rows.row_meets",
         "row_bent": "step.rows.row_bent", "row_short": "step.rows.row_short",
         "roof_none": "step.lay.roof_none", "wall_open": "step.lay.wall_open",
         "slab_open": "step.lay.slab_open", "untrusted": "protocol.untrusted",
         "remodelled": "asset.remodelled", "revised": "asset.revised"}
# Where a door block's middle stands: (street, metres off its line), its frontage in world.toml
# and half a block (2 m) in, the door block being a block of the wall.
FRONTS = (("high_street", 8.0), ("church_lane", 6.5))
NEAR = 1.0


def _npy(path):
    raw = Path(path).read_bytes()
    hlen = struct.unpack("<H", raw[8:10])[0]
    head = raw[10:10 + hlen].decode()
    body = raw[10 + hlen:]
    return head, struct.unpack(f"<{len(body) // 4}{'f' if '<f4' in head else 'i'}", body)


def instances(d, man):
    d = Path(d)
    names = [n for n in (d / "instances/names.txt").read_text().splitlines() if n]
    if not names:
        return {}
    x, y = ([v / 1000.0 for v in _npy(d / f"instances/{k}.npy")[1]] for k in ("x", "y"))
    z, face = list(_npy(d / "instances/z.npy")[1]), list(_npy(d / "instances/face.npy")[1])
    asset = [None] * len(names)
    for ch in man["chunks"]:
        for a, (s, n) in ch.get("instances", {}).items():
            for k in range(s, s + n):
                asset[k] = a
    return {n: {"asset": asset[k], "x": x[k], "y": y[k], "z": z[k], "face": face[k]} for k, n in enumerate(names)}


def ring_of(f):
    pieces = f.get("pieces") or []
    return [tuple(p) for p in pieces[0]["outer"]] if pieces else []


def path_of(f):
    return [tuple(p) for p in f.get("points") or f.get("path") or []]


def area2(ring):
    return sum(a[0] * b[1] - b[0] * a[1] for a, b in zip(ring, ring[1:] + ring[:1]))


def clip(subject, clipper):
    """The part of convex polygon `subject` inside convex polygon `clipper` (both
    counter-clockwise): Sutherland-Hodgman."""
    out = list(subject)
    for a, b in zip(clipper, clipper[1:] + clipper[:1]):
        if not out:
            break
        cur, out = out, []

        def side(p):
            return (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0])
        for p, q in zip(cur, cur[1:] + cur[:1]):
            sp, sq = side(p), side(q)
            if sp >= 0:
                out.append(p)
            if (sp >= 0) != (sq >= 0):
                t = sp / (sp - sq)
                out.append((p[0] + (q[0] - p[0]) * t, p[1] + (q[1] - p[1]) * t))
    return out


def inside(ring, p):
    x, y, odd = p[0], p[1], False
    for (x0, y0), (x1, y1) in zip(ring, ring[1:] + ring[:1]):
        if (y0 > y) != (y1 > y) and x < x0 + (y - y0) * (x1 - x0) / (y1 - y0):
            odd = not odd
    return odd


def dist_to(path, p):
    best = math.inf
    for a, b in zip(path, path[1:]):
        dx, dy = b[0] - a[0], b[1] - a[1]
        L2 = dx * dx + dy * dy
        t = 0.0 if L2 == 0 else max(0.0, min(1.0, ((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / L2))
        best = min(best, math.hypot(p[0] - a[0] - dx * t, p[1] - a[1] - dy * t))
    return best


def written(world):
    world = Path(world)
    text = world.read_text()
    files = [world] + [world.parent / f for f in re.findall(r'\[layer\.[a-z_]+\]\s*\n\s*file\s*=\s*"([^"]+)"', text)]
    tables = numbers = hand = 0
    for path in files:
        cur = False
        for line in path.read_text().splitlines():
            line = line.split("#", 1)[0].rstrip()
            if line.strip().startswith("["):
                cur = bool(re.match(r"\[feature\.[A-Za-z0-9_]+\]", line.strip()))
                tables += cur
                # A thing placed by hand: an instance or a pin (instances.md), anywhere.
                hand += bool(re.match(r"\[(instance|pin)\.", line.strip()))
                continue
            # A house written by hand: a feature of kind `building`.
            if cur and re.match(r'kind\s*=\s*"building"', line.strip()):
                hand += 1
            if cur and line.strip():
                numbers += len(re.findall(r"(?<![A-Za-z_\"'])-?\d+(?:\.\d+)?", line.split("=", 1)[-1]))
    return {"written_features": tables, "written_numbers": numbers, "hand_placed": hand}


def measures(build, world=None, run=None):
    d = Path(build)
    man = json.loads((d / "manifest.json").read_text())
    feats = json.loads((d / "features.json").read_text())["features"]
    diags = json.loads((d / "diagnostics.json").read_text())["diagnostics"]
    m = {}
    codes = Counter()
    for x in diags:
        codes[x["code"]] += int(x.get("count", 1))
    m["errors"] = sum(1 for x in diags if x["severity"] == "error")
    m["warnings"] = sum(1 for x in diags if x["severity"] == "warning")
    for k, code in CODES.items():
        m[k] = codes.get(code, 0)
    left = [x for x in diags if x["code"] == "step.rows.rows"]
    lefts = (left[0].get("data") or {}).get("left") or [] if left else []
    m["left_m_max"] = round(max(lefts), 3) if lefts else 0.0

    by = {}
    for f in feats:
        by.setdefault(f["kind"], []).append(f)
    houses = [f for f in by.get("building", []) if f["name"].startswith("rows#")]
    m["rows"] = len(by.get("row", []))
    m["buildings"] = len(houses)
    for r in by.get("row", []):
        m[f"lots_{r['name']}"] = sum(1 for f in houses if f["name"].rsplit("_", 1)[0] == "rows#" + r["name"])
    storeys = Counter(int(f["attrs"].get("storeys") or 0) for f in houses)
    for n in (1, 2, 3):
        m[f"storeys_{n}"] = storeys.get(n, 0)
    m["party_walls"] = sum(json.loads(f["attrs"].get("faces") or "[]").count("party") for f in houses)
    rings = [ring_of(f) for f in houses]
    over = 0.0
    for i in range(len(rings)):
        for j in range(i + 1, len(rings)):
            part = clip(rings[i], rings[j])
            if len(part) >= 3:
                over += abs(area2(part)) / 2
    m["overlap_m2"] = round(over, 3)

    rows = instances(d, man)
    tags = {a: set(v.get("tags") or []) for a, v in man["assets"].items()}

    def has(name, *want):
        t = tags.get(rows[name]["asset"], set())
        return any(w in t for w in want)
    m["instances"] = len(rows)
    m["blocks"] = sum(1 for n in rows if has(n, "role:wall", "role:corner", "role:door") and not has(n, "storey:parapet"))
    m["doors"] = sum(1 for n in rows if has(n, "role:door"))
    m["roof_tiles"] = sum(1 for n in rows if has(n, "role:roof_flat"))
    m["parapet"] = sum(1 for n in rows if has(n, "storey:parapet"))
    streets = {f["name"]: path_of(f) for f in feats if f["name"] in dict(FRONTS)}
    on = 0
    for n in rows:
        if has(n, "role:door"):
            p = (rows[n]["x"], rows[n]["y"])
            if any(abs(dist_to(streets[s], p) - off) <= NEAR for s, off in FRONTS if s in streets):
                on += 1
    m["doors_on_street"] = on
    # A house's front: the side of its outline its door stands nearest (features.json writes a
    # ring from its least point, so its side 0 is not the row's: T1's review).
    fronts = []
    for f in houses:
        ring = ring_of(f)
        key = "lay#" + f["name"].replace("#", "_") + "_s0_door"
        doors = [rows[n] for n in rows if n.startswith(key) and has(n, "role:door")]
        if not ring or not doors:
            continue
        p = (doors[0]["x"], doors[0]["y"])
        sides = list(zip(ring, ring[1:] + ring[:1]))
        a, b = min(sides, key=lambda s: dist_to(list(s), p))
        fronts.append(math.hypot(b[0] - a[0], b[1] - a[1]))
    m["widest_m"] = round(max(fronts), 3) if fronts else 0.0
    m["narrowest_m"] = round(min(fronts), 3) if fronts else 0.0
    m["doors_off_street"] = m["doors"] - on
    plots = [ring_of(f) for f in feats if f["kind"] == "plot"]
    trees = [n for n in rows if (rows[n]["asset"] or "").startswith("s_tree")]
    m["trees"] = len(trees)
    m["plants"] = sum(1 for n in rows if any(t.startswith("plant") for t in [man["assets"].get(rows[n]["asset"], {}).get("taxon") or ""]))
    m["trees_on_plots"] = sum(1 for n in trees if any(inside(r, (rows[n]["x"], rows[n]["y"])) for r in plots))
    if world:
        m.update(written(world))
    if run and Path(run).exists():
        t = json.loads(Path(run).read_text())["totals"]
        m["build_ms"], m["nodes"], m["nodes_ran"] = t["ms"], t["nodes"], t["ran"]
    return m, rows, {f["name"]: ring_of(f) for f in houses}


def against(rows, base, houses, base_houses):
    both = set(rows) & set(base)
    hb = set(houses) & set(base_houses)
    tr = {n for n in rows if (rows[n]["asset"] or "").startswith("s_tree")}
    tb = {n for n in base if (base[n]["asset"] or "").startswith("s_tree")}
    return {"trees_added": len(tr - tb), "trees_removed": len(tb - tr),
            "trees_kept": sum(1 for n in tr & tb if all(rows[n][k] == base[n][k] for k in ("x", "y", "asset"))),
            "added": len(set(rows) - set(base)), "removed": len(set(base) - set(rows)),
            "moved": sum(1 for n in both if any(rows[n][k] != base[n][k] for k in ("x", "y", "z", "face", "asset"))),
            "buildings_added": len(set(houses) - set(base_houses)),
            "buildings_removed": len(set(base_houses) - set(houses)),
            "buildings_moved": sum(1 for n in hb if houses[n] != base_houses[n]),
            "buildings_kept": sum(1 for n in hb if houses[n] == base_houses[n])}


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
    m, rows, houses = measures(args[0], opt.get("--world"), opt.get("--run"))
    if "--base" in opt:
        _, brows, bhouses = measures(opt["--base"])
        m.update(against(rows, brows, houses, bhouses))
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
