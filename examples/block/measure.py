"""Measures of a block build, for expect.toml: how the kit's pieces wall each building of a city
block, what stands against a neighbour, and what the build said (docs/research/Modular kit
failures in studios.md, section 13; docs/plan-kits.md, K6).

    python3 examples/block/measure.py BUILD [--world FILE] [--base BASE] [--check SECTION]

Prints the measures as JSON. With --check, compares them against expect.toml's rows under SECTION:
prints each row and its measure, and exits 1 when one falls outside its band or names a measure not
taken (Python 3.11+, for tomllib). With --base, also what changed against the base build, by name.
With --world, also what the author wrote. Reads only the build directory (spec/build.md):
manifest.json, features.json, diagnostics.json and instances/. Standard library only.

How each is taken (ours). Buildings here stand wall to wall, so a piece is not given to the
nearest outline, as examples/kit's measure does: it is given to the building and storey its name
says (`lay#<building>_s<k>_...`, or `lay#<building>_roof_...`).

- `buildings`: `building` features, written or returned by a step. `storeys`: the sum of their
  `storeys`. `instances`: every instance. `pieces`: the instances of an asset with a `role:` tag
  that is not `role:floor`. `tiles`: those of `role:floor`. `parapet`: the pieces tagged
  `storey:parapet`.
- **Walls.** For each building, storey and side of its outline, each wall piece whose origin lies
  within `NEAR` (0.3 m) of the side's line covers the side from its origin less half its asset's
  measured length to its origin plus half, along the side; a corner post covers its own width.
  The side from end to end, less the union, is open wall: each stretch longer than `TOL` (5 mm)
  counts in `open`, with `open_sum_mm`. `parapet_open` and `parapet_open_sum_mm` are the same
  for the parapet of each building that has any of one, on the building's outline. A storey above a set-back is measured on its own outline
  (the `storey` feature's ring), since it is not the building's.
- **What a side faces.** For each piece with an opening (a `role:wall` piece with no
  `variant:blank` or `storey:parapet` tag, or a `role:door` piece), the point `PROBE` (1 m) out from its origin along
  its front. `party_windows`: the point lies inside another building's outline, at a height that
  building reaches (below its eaves): a window or a shopfront against a neighbour's wall.
  `yard_shopfronts`: the piece is a ground storey's bay with an opening, the point lies in no
  building and inside the box of all the buildings shrunk by `PROBE`: a shopfront on the yard.
  `street_blank`: `variant:blank` pieces whose point lies outside that box: a blank wall on a
  street. `blank_above`: `variant:blank` pieces whose point lies over another building's outline
  and at or above its eaves: a blank bay that looks out over a lower neighbour's roof.
- `repeats`: pairs of buildings whose outlines share a stretch of side 1 m long or more and that
  have the same number of storeys and the same outline size, turned or not.
- `crowded`: `place.crowded` records. `stacked`: `geom.unowned_crossing` records that name two
  flights of one building. `open_said`: `step.lay.wall_open` records. `row_open`, `row_stair`,
  `row_meets`: the row step's records of those codes; `gap_wide`, `slab_open`: `lay`'s. `errors`,
  `warnings`.
- With `--world`: `written_buildings`, the `building` tables in the world's text; `written_rows`,
  the `row` tables; `written_numbers`, the number literals in those tables' lines (comments
  left out; the tables of views and other features are not counted): what the author worked out
  and wrote.
- With `--base`: `added`, `removed`, instance names not in the base and names only in it;
  `moved`, names in both whose place, facing or asset differs.
"""

import json
import math
import re
import struct
import sys
from collections import Counter
from pathlib import Path

TOL = 0.005
NEAR = 0.3
PROBE = 1.0


def _npy(path):
    raw = Path(path).read_bytes()
    hlen = struct.unpack("<H", raw[8:10])[0]
    head = raw[10:10 + hlen].decode()
    body = raw[10 + hlen:]
    if "<f4" in head:                                   # metres (z)
        return list(struct.unpack(f"<{len(body) // 4}f", body))
    return [v / 1000.0 for v in struct.unpack(f"<{len(body) // 4}i", body)]   # millimetres, millidegrees


def load(d):
    d = Path(d)
    man = json.loads((d / "manifest.json").read_text())
    diags = json.loads((d / "diagnostics.json").read_text())["diagnostics"]
    names = [n for n in (d / "instances/names.txt").read_text().splitlines() if n]
    x, y, z, face = (_npy(d / f"instances/{k}.npy") for k in ("x", "y", "z", "face"))
    asset = [None] * len(names)
    for ch in man["chunks"]:
        for a, (s, n) in ch.get("instances", {}).items():
            for k in range(s, s + n):
                asset[k] = a
    rows = {n: {"asset": asset[k], "x": x[k], "y": y[k], "z": z[k], "face": face[k]}
            for k, n in enumerate(names)}
    return man, diags, rows


def inside(ring, p):
    """Whether `p` lies strictly inside `ring` (even-odd)."""
    x, y = p
    hit = False
    for (x0, y0), (x1, y1) in zip(ring, ring[1:] + ring[:1]):
        if (y0 > y) != (y1 > y) and x < x0 + (y - y0) * (x1 - x0) / (y1 - y0):
            hit = not hit
    return hit


def sides(ring):
    out = []
    for a, b in zip(ring, ring[1:] + ring[:1]):
        L = math.hypot(b[0] - a[0], b[1] - a[1])
        out.append((a, ((b[0] - a[0]) / L, (b[1] - a[1]) / L), L))
    return out


def open_of(ring, laid):
    """Open stretches (metres) of each side of `ring`, given `laid`: [(x, y, half length)]."""
    out = []
    for a, e, L in sides(ring):
        spans = []
        for x, y, half in laid:
            u = (x - a[0]) * e[0] + (y - a[1]) * e[1]
            v = abs((x - a[0]) * e[1] - (y - a[1]) * e[0])
            if v <= NEAR and -half - TOL <= u <= L + half + TOL:
                spans.append((max(0.0, u - half), min(L, u + half)))
        at = 0.0
        for lo, hi in sorted(spans):
            if lo - at > TOL:
                out.append(lo - at)
            at = max(at, hi)
        if L - at > TOL:
            out.append(L - at)
    return out


def shared(r1, r2):
    """The longest stretch (metres) two outlines' sides share, lying on one line."""
    best = 0.0
    for a, e, L in sides(r1):
        for b, f, M in sides(r2):
            if abs(e[0] * f[1] - e[1] * f[0]) > 1e-6:
                continue
            if abs((b[0] - a[0]) * e[1] - (b[1] - a[1]) * e[0]) > 0.01:
                continue
            u0 = (b[0] - a[0]) * e[0] + (b[1] - a[1]) * e[1]
            u1 = u0 + M * (e[0] * f[0] + e[1] * f[1])
            best = max(best, min(L, max(u0, u1)) - max(0.0, min(u0, u1)))
    return best


def size_of(ring):
    return tuple(sorted(round(L, 2) for _, _, L in sides(ring)))


def measures(d, world=None):
    man, diags, rows = load(d)
    assets = man["assets"]
    feats = {f["name"]: f for f in json.loads((Path(d) / "features.json").read_text())["features"]}

    def ring_of(f):
        return [tuple(p) for p in f["pieces"][0]["outer"]]
    buildings = {n: f for n, f in feats.items() if f.get("kind") == "building"}
    rings = {n: ring_of(f) for n, f in buildings.items()}
    key = {n: re.sub(r"[^A-Za-z0-9_-]", "_", n) for n in buildings}      # the name `lay` writes (its `key_of`)
    by_key = {v: k for k, v in key.items()}
    lay_of = {}                                  # produced feature -> (building, rest of its name)
    for n in feats:
        tail = n.split("#")[-1]
        for kk in sorted(by_key, key=len, reverse=True):
            if n.startswith("lay#") and tail.startswith(kk + "_"):
                lay_of[n] = (by_key[kk], tail[len(kk) + 1:])
                break
    eaves = {}
    storey_ring = {}
    for n, (b, rest) in lay_of.items():
        f = feats[n]
        if f.get("kind") == "roof":
            eaves[b] = f["attrs"]["eaves"]
        m_ = re.fullmatch(r"s(\d+)", rest)
        if m_ and f.get("kind") == "storey":
            storey_ring[(b, int(m_.group(1)))] = ring_of(f)

    def tags(a):
        return assets[a].get("tags") or []
    kit = {a for a in assets if any(t.startswith("role:") for t in tags(a))}
    m = {"buildings": len(buildings), "storeys": sum(int(f["attrs"].get("storeys") or 1) for f in buildings.values()),
         "instances": len(rows), "pieces": 0, "tiles": 0, "parapet": 0}
    laid = {}                                    # (building, storey) -> [(x, y, half)]
    party = yard = blank = above = 0
    xs = [p[0] for r in rings.values() for p in r]
    ys = [p[1] for r in rings.values() for p in r]
    box = (min(xs) + PROBE, min(ys) + PROBE, max(xs) - PROBE, max(ys) - PROBE) if rings else None
    for n, r in rows.items():
        a = r["asset"]
        if a not in kit:
            continue
        if "role:floor" in tags(a):
            m["tiles"] += 1
            continue
        m["pieces"] += 1
        m["parapet"] += "storey:parapet" in tags(a)
        tail = n.split("#", 1)[-1]
        owner = next(((by_key[kk], tail[len(kk) + 1:]) for kk in sorted(by_key, key=len, reverse=True)
                      if n.startswith("lay#") and tail.startswith(kk + "_")), None)
        st = re.match(r"s(\d+)_", owner[1]) if owner else None
        b0 = assets[a]["bounds"]
        if owner and st:
            laid.setdefault((owner[0], int(st.group(1))), []).append((r["x"], r["y"], (b0[3] - b0[0]) / 2))
        elif owner and owner[1].startswith("roof_") and "storey:parapet" in tags(a):
            laid.setdefault((owner[0], "roof"), []).append((r["x"], r["y"], (b0[3] - b0[0]) / 2))
        f = math.radians(r["face"])
        probe = (r["x"] + math.sin(f) * PROBE, r["y"] + math.cos(f) * PROBE)
        opening = ("role:wall" in tags(a) and "variant:blank" not in tags(a)
                   and "storey:parapet" not in tags(a)) or "role:door" in tags(a)
        in_box = box and box[0] < probe[0] < box[2] and box[1] < probe[1] < box[3]
        beside = [o for o, ring in rings.items() if (not owner or o != owner[0]) and inside(ring, probe)]
        if opening:
            if any(r["z"] < eaves.get(o, float("inf")) - 0.5 for o in beside):
                party += 1
            elif in_box and "storey:ground" in tags(a) and "role:wall" in tags(a):
                yard += 1
        elif "variant:blank" in tags(a):
            if box and not in_box:
                blank += 1
            elif beside and all(r["z"] >= eaves.get(o, float("inf")) - 0.5 for o in beside):
                above += 1
    m.update({"party_windows": party, "yard_shopfronts": yard, "street_blank": blank, "blank_above": above})

    gaps = []
    for b, f in sorted(buildings.items()):
        for k in range(int(f["attrs"].get("storeys") or 1)):
            ring = rings[b]
            own = storey_ring.get((b, k))
            # A storey set back from the outline is walled on its own ring; one that only leaves
            # out a stairwell is walled on the building's.
            if own and all(inside(ring, p) or min(abs(p[0] - q[0]) + abs(p[1] - q[1]) for q in ring) < 20
                           for p in own) and len(own) == 4 and size_of(own) != size_of(ring):
                ring = own
            gaps += open_of(ring, laid.get((b, k), []))
    m["open"] = len(gaps)
    m["open_sum_mm"] = int(round(sum(gaps) * 1000))
    # The parapet round a flat roof, where the build laid any of it: measured as a storey's walls.
    gaps = [g for b in sorted(buildings) if (b, "roof") in laid for g in open_of(rings[b], laid[(b, "roof")])]
    m["parapet_open"] = len(gaps)
    m["parapet_open_sum_mm"] = int(round(sum(gaps) * 1000))

    names = sorted(buildings)
    m["repeats"] = sum(1 for i, p in enumerate(names) for q in names[i + 1:]
                       if shared(rings[p], rings[q]) >= 1.0 and size_of(rings[p]) == size_of(rings[q])
                       and int(buildings[p]["attrs"].get("storeys") or 1) == int(buildings[q]["attrs"].get("storeys") or 1))

    codes = Counter(x["code"] for x in diags)
    m["errors"] = sum(1 for x in diags if x["severity"] == "error")
    m["warnings"] = sum(1 for x in diags if x["severity"] == "warning")
    m["crowded"] = codes.get("place.crowded", 0)
    m["stacked"] = sum(1 for x in diags if x["code"] == "geom.unowned_crossing" and re.search(r"_f\d+", x.get("subject", "")))
    m["open_said"] = codes.get("step.lay.wall_open", 0)
    for code in ("row_open", "row_stair", "row_meets"):
        m[code] = codes.get(f"step.rows.{code}", 0)
    for code in ("gap_wide", "slab_open"):
        m[code] = codes.get(f"step.lay.{code}", 0)
    m["_codes"] = dict(sorted(codes.items()))
    if world:
        tables, cur = [], None                   # [(kind, number literals)] for each feature table
        for line in Path(world).read_text().splitlines():
            line = line.split("#", 1)[0].rstrip()
            if line.strip().startswith("["):
                cur = None
                if re.match(r"\[feature\.[A-Za-z0-9_]+\]", line.strip()):
                    cur = {"kind": None, "numbers": 0}
                    tables.append(cur)
                continue
            if cur is None or not line.strip():
                continue
            k = re.match(r'\s*kind\s*=\s*"([a-z_]+)"', line)
            if k:
                cur["kind"] = k.group(1)
            cur["numbers"] += len(re.findall(r"(?<![A-Za-z_\"])-?\d+(?:\.\d+)?", line.split("=", 1)[-1]))
        m["written_buildings"] = sum(1 for t in tables if t["kind"] == "building")
        m["written_rows"] = sum(1 for t in tables if t["kind"] == "row")
        m["written_numbers"] = sum(t["numbers"] for t in tables if t["kind"] in ("building", "row"))
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
        ok = v is not None and lo <= v <= hi
        bad += not ok
        print(f"{'ok  ' if ok else 'FAIL'} {check}.{key}: {row['measure']} = {v} in [{lo}, {hi}]")
    print(f"{len(table) - bad}/{len(table)} rows")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
