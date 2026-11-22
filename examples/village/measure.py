"""Measures of a village build, for expect.toml: how a kit's buildings meet sloping ground, and
what the build said (docs/plan-kits.md, K7; docs/research/Modular kit failures in studios.md,
section 4, "Meeting the ground", and section 13, "House on a slope").

    python3 examples/village/measure.py BUILD [--base BASE] [--check SECTION]

Prints the measures as JSON. With --check, compares them against expect.toml's rows under SECTION
(Python 3.11+, for tomllib). With --base, also what changed against the base build, by name.
Reads only the build directory (spec/build.md): manifest.json, features.json, diagnostics.json,
instances/ and fields/height.npy. Standard library only.

How each is taken (ours).

- `buildings`, `storeys`, `instances`, `pieces` (instances of an asset with a `role:` tag that is
  not `role:floor`), `tiles`, `roof` (pieces tagged `role:roof_slope` or `role:roof_gable`), `base`
  (pieces tagged `storey:base`), `steps` (pieces tagged `role:steps`).
- **The ground at a wall.** The built ground is `fields/height.npy`, read between its posts. A
  building's outline is walked every `STEP` (0.5 m), and at each point the ground is read `OUT`
  (0.6 m) outside the outline: beside the wall, clear of the floor the storey cuts. The building's
  floor is its ground storey's `level`. `hang` at a point is the floor less the ground: how far
  the wall's foot stands above the ground there. `hang_max_mm` is the most over every building;
  `hang_m` the length of outline, summed, where it is more than `SEEN` (0.05 m); `hang_houses`
  the buildings with any such stretch. `buried_max_mm` is the most the ground stands above a
  floor at its wall.
- **What covers the hang.** A point's hang is covered when a piece tagged `storey:base` of that
  building stands within 0.3 m of the side there, spans the point along the side, and reaches
  the ground or below it: its `z`, which is its foot since the pack sinks it by its height, is
  at or under the ground read there. `open_foot_m` is the outline's length with a hang over `SEEN` and no
  such piece; `open_foot_max_mm` the largest uncovered hang.
- **Doors.** For each `doorway` feature `lay` returned, its sill is `sill_z` and the ground is
  read at the path's outer end (the porch's far end) and `OUT` outside the wall. `door_step_max_mm`
  is the most the sill stands above the ground `OUT` outside the wall; `doors_high`, the doors
  where that is more than `STEP_UP` (0.25 m, one riser and a little: a walker's step); and
  `doors_served`, those of them with a piece tagged `role:steps` of the building within 1.5 m.
- `row_step_max_mm`: the most two buildings whose outlines share a metre of side differ in floor.
- **Walls.** As examples/block's measure: each building, storey and side, the pieces named for
  it; `open`, `open_sum_mm`.
- `errors`, `warnings`: the build's records. `base_short`, `foot_open`, `roof_none`: the
  buildings `lay` said each of, the `count` of the folded record. `off_bound`:
  `surface.off_bound` records (a thing partly off the floor it stands on). `gave_way`: members
  of runs that gave way by overlap (`place.gave_way`, summed). `_codes`: every code's count.
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
STEP = 0.5
OUT = 0.6
SEEN = 0.05
STEP_UP = 0.25


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
        i = min(max((px - ox) / res, 0.0), cols - 1.000001)
        j = min(max((py - oy) / res, 0.0), rows - 1.000001)
        i0, j0 = int(i), int(j)
        u, v = i - i0, j - j0
        return ((h[j0 * cols + i0] * (1 - u) + h[j0 * cols + i0 + 1] * u) * (1 - v)
                + (h[(j0 + 1) * cols + i0] * (1 - u) + h[(j0 + 1) * cols + i0 + 1] * u) * v)
    return ground


def sides(ring):
    out = []
    for a, b in zip(ring, ring[1:] + ring[:1]):
        L = math.hypot(b[0] - a[0], b[1] - a[1])
        out.append((a, ((b[0] - a[0]) / L, (b[1] - a[1]) / L), L))
    return out


def shared(r1, r2):
    best = 0.0
    for a, e, L in sides(r1):
        for b, f, M in sides(r2):
            if abs(e[0] * f[1] - e[1] * f[0]) > 1e-6 or abs((b[0] - a[0]) * e[1] - (b[1] - a[1]) * e[0]) > 0.01:
                continue
            u0 = (b[0] - a[0]) * e[0] + (b[1] - a[1]) * e[1]
            u1 = u0 + M * (e[0] * f[0] + e[1] * f[1])
            best = max(best, min(L, max(u0, u1)) - max(0.0, min(u0, u1)))
    return best


def open_of(ring, laid):
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


def measures(d):
    man, diags, rows = load(d)
    assets = man["assets"]
    ground = ground_of(d, man)
    feats = {f["name"]: f for f in json.loads((Path(d) / "features.json").read_text())["features"]}

    def ring_of(f):
        return [tuple(p) for p in f["pieces"][0]["outer"]]

    def tags(a):
        return assets[a].get("tags") or []
    buildings = {n: f for n, f in feats.items() if f.get("kind") == "building"}
    rings = {n: ring_of(f) for n, f in buildings.items()}
    key = {n: re.sub(r"[^A-Za-z0-9_-]", "_", n) for n in buildings}
    by_key = sorted(((v, k) for k, v in key.items()), key=lambda t: -len(t[0]))

    def owner(name):
        tail = name.split("#", 1)[-1]
        return next(((b, tail[len(k) + 1:]) for k, b in by_key if name.startswith("lay#") and tail.startswith(k + "_")), None)
    floor = {}
    doors = []
    for n, f in feats.items():
        o = owner(n)
        if not o:
            continue
        if f.get("kind") == "storey" and o[1] == "s0":
            floor[o[0]] = f["attrs"]["level"]
        if f.get("kind") == "doorway":
            doors.append((o[0], [(p[0], p[1]) for p in f["points"]], f["attrs"]["sill_z"]))
    kit = {a for a in assets if any(t.startswith("role:") for t in tags(a))}
    m = {"buildings": len(buildings), "storeys": sum(int(f["attrs"].get("storeys") or 1) for f in buildings.values()),
         "instances": len(rows), "pieces": 0, "tiles": 0, "roof": 0, "base": 0, "steps": 0}
    laid, bases, stoops = {}, {}, {}
    for n, r in rows.items():
        a = r["asset"]
        if a not in kit:
            continue
        o = owner(n)
        b0 = assets[a]["bounds"]
        if "role:floor" in tags(a):
            m["tiles"] += 1
            continue
        m["pieces"] += 1
        if "role:roof_slope" in tags(a) or "role:roof_gable" in tags(a):
            m["roof"] += 1
        elif "role:steps" in tags(a):
            m["steps"] += 1
            if o:
                stoops.setdefault(o[0], []).append((r["x"], r["y"]))
        elif "storey:base" in tags(a):
            m["base"] += 1
            if o:
                bases.setdefault(o[0], []).append((r["x"], r["y"], (b0[3] - b0[0]) / 2, r["z"], b0[5] - b0[2]))
        elif o:
            st = re.match(r"s(\d+)_", o[1])
            if st:
                laid.setdefault((o[0], int(st.group(1))), []).append((r["x"], r["y"], (b0[3] - b0[0]) / 2))

    hang_max = buried = open_max = 0.0
    hang_m = open_m = 0.0
    hang_houses = 0
    for b, ring in sorted(rings.items()):
        if b not in floor:
            continue
        any_hang = False
        for a, e, L in sides(ring):
            n_out = (e[1], -e[0])                 # the outline is counter-clockwise: its right is outside
            k = max(1, int(round(L / STEP)))
            for i in range(k):
                u = (i + 0.5) * L / k
                px, py = a[0] + e[0] * u, a[1] + e[1] * u
                g = ground(px + n_out[0] * OUT, py + n_out[1] * OUT)
                hang = floor[b] - g
                buried = max(buried, -hang)
                hang_max = max(hang_max, hang)
                if hang <= SEEN:
                    continue
                any_hang = True
                hang_m += L / k
                # A base piece is sunk by its own height (the pack's `sink`), so an instance's `z`
                # is its foot: it reaches the ground where its foot is at or under it.
                covered = any(abs((x - px) * e[0] + (y - py) * e[1]) <= half + TOL
                              and abs((x - px) * e[1] - (y - py) * e[0]) <= NEAR and z <= g + TOL
                              for x, y, half, z, high in bases.get(b, []))
                if not covered:
                    open_m += L / k
                    open_max = max(open_max, hang)
        hang_houses += any_hang
    m.update({"hang_max_mm": int(round(hang_max * 1000)), "hang_m": round(hang_m, 1), "hang_houses": hang_houses,
              "buried_max_mm": int(round(max(buried, 0.0) * 1000)), "open_foot_m": round(open_m, 1),
              "open_foot_max_mm": int(round(open_max * 1000))})

    step_max, high, served = 0.0, 0, 0
    for b, path, sill in doors:
        (x0, y0), (x1, y1) = path[0], path[-1]     # from outside to the wall
        L = math.hypot(x1 - x0, y1 - y0) or 1.0
        e = ((x0 - x1) / L, (y0 - y1) / L)        # out of the wall
        up = sill - ground(x1 + e[0] * OUT, y1 + e[1] * OUT)
        step_max = max(step_max, up)
        if up > STEP_UP:
            high += 1
            served += any(math.hypot(x - x1, y - y1) <= 1.5 for x, y in stoops.get(b, []))
    m.update({"doors": len(doors), "door_step_max_mm": int(round(step_max * 1000)), "doors_high": high,
              "doors_served": served})

    names = sorted(b for b in buildings if b in floor)
    m["row_step_max_mm"] = int(round(1000 * max([abs(floor[p] - floor[q]) for i, p in enumerate(names)
                                                 for q in names[i + 1:] if shared(rings[p], rings[q]) >= 1.0] or [0.0])))
    gaps = [g for b, f in sorted(buildings.items()) for k in range(int(f["attrs"].get("storeys") or 1))
            for g in open_of(rings[b], laid.get((b, k), []))]
    m["open"] = len(gaps)
    m["open_sum_mm"] = int(round(sum(gaps) * 1000))
    # A step's records of one code on the things it returned are folded into one, with a `count`
    # (diagnostics.md): the count is what is measured.
    codes = Counter()
    for x in diags:
        codes[x["code"]] += int(x.get("count", 1))
    m["errors"] = sum(1 for x in diags if x["severity"] == "error")
    m["warnings"] = sum(1 for x in diags if x["severity"] == "warning")
    for code in ("base_short", "foot_open", "roof_none"):
        m[code] = codes.get(f"step.lay.{code}", 0)
    m["off_bound"] = codes.get("surface.off_bound", 0)
    m["gave_way"] = sum(int((x.get("data") or {}).get("overlap", 0)) for x in diags if x["code"] == "place.gave_way")
    m["_codes"] = dict(sorted(codes.items()))
    return m, rows


def against(rows, base):
    both = set(rows) & set(base)
    return {"added": len(set(rows) - set(base)), "removed": len(set(base) - set(rows)),
            "moved": sum(1 for n in both if any(rows[n][k] != base[n][k] for k in ("x", "y", "z", "face", "asset")))}


def main(argv):
    args = argv[1:]
    opt = {}
    for flag in ("--base", "--check"):
        if flag in args:
            i = args.index(flag)
            opt[flag] = args[i + 1]
            del args[i:i + 2]
    if len(args) != 1:
        print(__doc__.split("\n\n")[1], file=sys.stderr)
        return 2
    m, rows = measures(args[0])
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
