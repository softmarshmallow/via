"""Measures of a college build, for expect.toml (docs/plan.md M5; challenges 20 and 48; R15).

    python3 examples/college/measure.py BUILD_DIR [--base BASE_DIR] [--check SECTION]

Prints the measures as JSON. With --check, compares them against expect.toml's rows under SECTION
(`base`, `move_and_turn`, `no_walk`) and exits 1 when one falls outside its band (Python 3.11+, for
tomllib). With --base, also what changed against the base build. Reads only the build directory
(spec/build.md): manifest.json (`assets`, `chunks`), features.json, meets.json, diagnostics.json and
instances/ (names.txt; x.npy, y.npy int32 millimetres; face.npy int32 millidegrees). Standard
library only.

Protocol of each measure (ours, written for this world):

- **Doors** (challenge 48). A door is an instance's asset mount with a `connector` (manifest
  `assets.<id>.mounts`). Its world point is the instance's origin plus the mount's `at` turned by the
  instance's face (bearing clockwise from north, the asset's front +y): `x + ax cos f + ay sin f`,
  `y - ax sin f + ay cos f`. `doors` counts them; `doors_met` the doors that are the `b` of a meets.json
  row with `gap` 0 and `connector_a` equal to the door's connector; `door_gap_max` the largest
  |gap| of those rows in millimetres; `door_turn_max` the largest `turn`, degrees.
- **Walks follow their doors.** For every path feature whose last point is a door's meeting
  (`<walk>.end` in meets.json, `b` a door): `walk_end_off` the largest distance, in millimetres,
  between the walk's last point (features.json) and the door's world point computed as above from
  the instances and the pack, not from the meeting: 0 means the walk ends where the asset says the
  door is.
- **Walk rise**: `walk_rise_max`, the largest difference in metres between the ground at a door's
  walk's two ends (meets.json `z_a` of `<walk>.start` and `<walk>.end`): a door standing below or
  above the place its walk leaves.
- **Kit joints** (challenge 20). The members of each run of a `college/kit` asset, `<run>#r<k>` in order of `k` (one asset
  each, from the manifest's `chunks`): for each pair of neighbours, the joint is the distance between
  their origins less the asset's width (`bounds` x extent, the axis that lies along the line: the
  runs face their fronts square to it). Positive is a gap, negative an overlap, millimetres.
  `kit_gap_max` and `kit_overlap_max` over every joint of every run; `kit_overlap_wing` over the
  east wing's alone; `kit_pitch_off_max` the largest |width - params.pitch| of a kit asset, mm.
- **Codes**: `errors`, `warnings`, and the count of each code named in expect.toml; `reach_held`,
  `reach_pairs` from `step.reach.walked`.
- **Views**: `bookmarks`, features of kind `bookmark`; `door_views`, those whose `toward` names a
  door (`<instance>.<mount>` of a door).
- **Against a base**: `instances_added`, `instances_removed` by name; `instances_moved` the names in
  both whose x, y or face differ, and `moved_hand` those that are not scattered (no `#s`) and not the
  pavilion; `features_moved` the features in both whose points differ by more than 1 mm, and
  `features_moved_other` those other than `pavilion_walk`; `meets_kept` the meetings (a, b, x, y,
  gap) equal in both, `meets_changed` the rest.
"""

import json
import math
import struct
import sys
from collections import Counter
from pathlib import Path


def npy(path):
    raw = Path(path).read_bytes()
    hlen = struct.unpack("<H", raw[8:10])[0]
    head = raw[10:10 + hlen].decode()
    code = {"'<i4'": "i", "'<f4'": "f"}[head.split("'descr': ")[1].split(",")[0]]
    n = int(head.split("'shape': (")[1].split(",")[0])
    return list(struct.unpack(f"<{n}{code}", raw[10 + hlen:]))


def instances(d):
    names = [n for n in (d / "instances/names.txt").read_text().splitlines() if n]
    x, y, face = (npy(d / f"instances/{c}.npy") for c in ("x", "y", "face"))
    man = json.loads((d / "manifest.json").read_text())
    asset = [None] * len(names)
    for ch in man["chunks"]:                         # each asset's [first, count] in the arrays
        for a, (s, n) in ch.get("instances", {}).items():
            for i in range(s, s + n):
                asset[i] = a
    return {nm: {"x": x[i], "y": y[i], "face": face[i], "asset": asset[i]} for i, nm in enumerate(names)}, man


def mount_at(inst, m):
    f = math.radians(inst["face"] / 1000)
    ax, ay = m["at"][0] * 1000, m["at"][1] * 1000
    return (inst["x"] + ax * math.cos(f) + ay * math.sin(f), inst["y"] - ax * math.sin(f) + ay * math.cos(f))


def measures(d):
    d = Path(d)
    insts, man = instances(d)
    assets = man["assets"]
    fs = json.loads((d / "features.json").read_text())["features"]
    meets = json.loads((d / "meets.json").read_text())["meets"]
    diags = json.loads((d / "diagnostics.json").read_text())["diagnostics"]
    m = {"instances": len(insts)}

    doors = {}
    for nm, i in insts.items():
        for mn, mt in (assets.get(i["asset"]) or {}).get("mounts", {}).items():
            if mt.get("connector"):
                doors[f"{nm}.{mn}"] = (mount_at(i, mt), mt["connector"])
    met = [r for r in meets if r["b"] in doors]
    good = [r for r in met if r["gap"] == 0 and r["connector_a"] == doors[r["b"]][1]]
    m["doors"] = len(doors)
    m["doors_met"] = len({r["b"] for r in good})
    m["door_gap_max"] = max((abs(r["gap"]) for r in met), default=0)
    m["door_turn_max"] = round(max(((r["turn"] or 0) / 1000 for r in met), default=0), 1)
    by = {f["name"]: f for f in fs}
    off = 0.0
    for r in met:
        walk = by.get(r["a"].rsplit(".", 1)[0])
        if walk and r["a"].endswith(".end"):
            (dx, dy), _ = doors[r["b"]]
            p = walk["points"][-1]
            off = max(off, math.hypot(p[0] * 1000 - dx, p[1] * 1000 - dy))
    m["walk_end_off"] = round(off)
    z = {r["a"]: r["z_a"] for r in meets}
    rise = [abs(z[w + ".end"] - z[w + ".start"]) for w in {r["a"].rsplit(".", 1)[0] for r in met}
            if w + ".start" in z and w + ".end" in z]
    m["walk_rise_max"] = round(max(rise, default=0), 2)

    runs = {}
    for nm, i in insts.items():
        if "#r" in nm and (assets.get(i["asset"]) or {}).get("taxon", "").startswith("college/kit"):
            run, k = nm.split("#r")
            runs.setdefault(run, []).append((int(k), i))
    gaps, overs, wing = [0], [0], [0]
    for run, ms in runs.items():
        ms.sort(key=lambda t: t[0])
        for (_, a), (_, b) in zip(ms, ms[1:]):
            bd = assets[a["asset"]]["bounds"]
            j = math.hypot(b["x"] - a["x"], b["y"] - a["y"]) - (bd[3] - bd[0]) * 1000
            (gaps if j >= 0 else overs).append(abs(j))
            if run == "east_wing" and j < 0:
                wing.append(-j)
    m["kit_runs"] = len(runs)
    m["kit_joints"] = sum(len(v) - 1 for v in runs.values())
    m["kit_gap_max"], m["kit_overlap_max"] = round(max(gaps)), round(max(overs))
    m["kit_overlap_wing"] = round(max(wing))
    m["kit_pitch_off_max"] = round(max((abs((a["bounds"][3] - a["bounds"][0]) - a["params"]["pitch"]) * 1000
                                        for a in assets.values() if (a.get("params") or {}).get("pitch")), default=0))

    codes = Counter(x["code"] for x in diags)
    for c in ("place.keep_out", "place.overlap", "place.crowded", "surface.unmet", "surface.connector",
              "surface.gap", "surface.apart", "step.check.kit_pitch", "step.check.turn",
              "step.reach.unreached"):
        m[c] = codes.get(c, 0)
    for x in diags:
        if x["code"] == "step.reach.walked":
            m["reach_held"], m["reach_pairs"] = x["data"].get("held"), x["data"].get("pairs")
        if x["code"] == "surface.unmet":
            m["unmet_hint_writes"] = int(f'ends_at = "{x["data"]["place"]}"' in x.get("hint", ""))
    m["errors"] = sum(1 for x in diags if x["severity"] == "error")
    m["warnings"] = sum(1 for x in diags if x["severity"] == "warning")
    bms = [f for f in fs if f.get("kind") == "bookmark"]
    m["bookmarks"] = len(bms)
    tw = lambda f: ((f.get("attrs") or {}).get("toward") or {})
    m["door_views"] = sum(1 for f in bms if (tw(f).get("name") if isinstance(tw(f), dict) else tw(f)) in doors)
    return m, fs, insts, meets


def shape(f):
    """A feature's plan points in order: a path's points, a region's outer rings, a point's `at`."""
    if "points" in f:
        return [p[:2] for p in f["points"]]
    if "pieces" in f:
        return [p[:2] for pc in f["pieces"] for p in pc["outer"]]
    return [f["at"][:2]] if "at" in f else []


def main(argv):
    m, fs, insts, meets = measures(argv[0])
    if "--base" in argv:
        mb, fb, ib, eb = measures(argv[argv.index("--base") + 1])
        m["instances_added"] = len(set(insts) - set(ib))
        m["instances_removed"] = len(set(ib) - set(insts))
        moved = sorted(n for n in set(insts) & set(ib)
                       if any(insts[n][k] != ib[n][k] for k in ("x", "y", "face")))
        m["instances_moved"] = len(moved)
        m["moved_hand"] = sum(1 for n in moved if "#s" not in n and n != "pavilion")
        m["moved_names"] = [n for n in moved if "#s" not in n]
        pa = {f["name"]: shape(f) for f in fs}
        pb = {f["name"]: shape(f) for f in fb}
        fm = sorted(n for n in set(pa) & set(pb) if len(pa[n]) != len(pb[n]) or any(
            math.hypot(p[0] - q[0], p[1] - q[1]) > 0.001 for p, q in zip(pa[n], pb[n])))
        m["features_moved"] = len(fm)
        m["features_moved_other"] = sum(1 for n in fm if n != "pavilion_walk")
        m["features_moved_names"] = fm
        key = lambda r: (r["a"], r["b"], r["x"], r["y"], r["gap"])
        kb = {key(r) for r in eb}
        m["meets_kept"] = sum(1 for r in meets if key(r) in kb)
        m["meets_changed"] = len(meets) - m["meets_kept"]
    if "--check" in argv:
        sys.exit(check(m, argv[argv.index("--check") + 1]))
    print(json.dumps(m, indent=1, sort_keys=True))


def check(m, section):
    import tomllib
    rows = tomllib.loads((Path(__file__).parent / "expect.toml").read_text())[section]
    bad = 0
    for name, row in rows.items():
        v = m.get(row["measure"])
        lo, hi = row["band"]
        ok = v is not None and lo <= v <= hi
        bad += not ok
        print(f"{'ok  ' if ok else 'FAIL'} {section}.{name}: {row['measure']} = {v}, band [{lo}, {hi}]")
    return 1 if bad else 0


if __name__ == "__main__":
    main(sys.argv[1:])
