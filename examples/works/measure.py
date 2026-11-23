"""Measures of a works build, for expect.toml: two buildings on two modules joined by pipes at a
height, and a ring of whole rooms (docs/plan-kits.md, K9).

    python3 examples/works/measure.py BUILD [--base BASE] [--world FILE] [--check SECTION]

Prints the measures as JSON. With --check, compares them against expect.toml's rows under SECTION
(Python 3.11+, for tomllib). With --base, also what changed against the base build, by name.
With --world, also what that world's text writes. Reads only the build directory (spec/build.md):
manifest.json, features.json, meets.json, diagnostics.json, instances/ and fields/height.npy.
Standard library only.

How each is taken (ours).

- `instances`; `legs` (features of kind `pipe_leg`); `straights`, `slips`, `elbows`, `risers`,
  `flanges`, `caps`, `supports` (instances of an asset tagged `role:pipe`, `role:slip`,
  `role:elbow`, `role:riser`, `role:flange`, `role:cap`, `role:support`); `ports` (instances of
  an asset that has a mount named `port`); `rooms` (instances of the taxon `interior/room`).
- **A place on a piece** is a point of its asset's frame (x to its right, y its front, z up),
  put at the instance's `x`, `y`, `z` and turned by its `face`. `z` is the height the engine
  gave the instance: for a pipe piece, its leg's surface.
- **Meetings**, from meets.json. A side `<name>.<mount>` is a mount of an instance where
  `<name>` is an instance. `joints`: rows whose two sides are mounts of two instances (R76).
  `port_joints`: joints with a mount named `port` on one side and `plug` on the other.
  `door_joints`: joints of two `rooms`. `riser_meets`: rows with a mount named `top` on one
  side and the end of a `pipe_leg` on the other. `gap_max_mm`: the greatest `gap` of any row.
- **The pipe, end to end.** Not read from the step's plan: each pipe piece's open ends are
  worked out from its instance and its asset's own facts. A straight or a slip piece has an end
  half its `params.pitch` either way along its x; an elbow one `params.reach` to its back and
  one to its right; a riser one `reach` to its back and one `reach` to its front, `params.rise`
  higher; a flange and a cap one `reach` to their back. Within one pipe (the pieces named for
  it), two ends within 1 mm of each other in space are a join. `pipe_joins` counts them;
  `pipe_loose` the ends left with no partner; `pipe_gap_mm` the furthest any loose end is from
  the nearest other loose end of its pipe, in whole millimetres (0 with none loose, -1 where a
  loose end has no other). A flange's plate and a cap's closed end are not open ends.
  `plugs_joined`: flanges whose `plug` is a side of a joint.
- **Supports.** `support_foot_mm`: for each support, its instance's `z` less its
  `params.height`, against the built ground (`fields/height.npy`, read between its posts) at
  its place; the greatest difference, in whole millimetres. 0: every foot is on the ground.
- **The ring.** Every mount with the connector `door` on a room is a door. `doors`;
  `doors_joined`, those that are a side of a joint; `ring_miss_mm`: for each door that is not,
  the distance in plan to the nearest door of another room, the greatest, in whole millimetres
  (0 with every door joined).
- `errors`, `warnings`: the build's records. `gap`, `unmet`, `connector`, `facing`,
  `joint_many`, `none_at` (the engine's `surface.*`), `crossing` (`geom.unowned_crossing`),
  `crowded` (`place.crowded`), `pipe_params`, `pipe_open`, `support_off`, `support_none` (the
  pipes step's), `lay_params`, `wall_open`, `bay_none` (lay's), `turn`, `kit_stated`, `kit_unit`,
  `kit_pitch` (check's): each code's records, a folded record counted by its `count`.
  `unmet_miss_mm`, `unmet_miss_z_mm`: the greatest `miss` and `miss_z` the engine's
  `surface.unmet` records name (-1 where none names another mount). `_codes`: every code's
  count. `built`: 1, or 0 for a build that stopped on an error and left its records and nothing
  else; such a build has only these measures.
- With `--world`: `written_buildings`, `written_pipes`, `written_instances`: the `building` and
  `pipe` feature tables and the `instance` tables in that world's text and its layers' files;
  `written_numbers`, the number literals in those tables' lines (comments left out), which is
  what an author works out and types.
"""

import json
import math
import re
import struct
import sys
from collections import Counter
from pathlib import Path

JOIN = 0.001
ROLES = {"pipe": "straights", "slip": "slips", "elbow": "elbows", "riser": "risers", "flange": "flanges", "cap": "caps",
         "support": "supports"}
PIECE = re.compile(r"^[^#]+#(.+?)_(?:l\d+_[sf]|l\d+_p\d+|e\d+|r\d+|end[01])(?:#r\d+)?$")
SAID = {"gap": "surface.gap", "unmet": "surface.unmet", "connector": "surface.connector", "facing": "surface.facing",
        "joint_many": "surface.joint_many", "none_at": "surface.none_at", "crossing": "geom.unowned_crossing",
        "crowded": "place.crowded", "pipe_params": "step.pipes.params", "pipe_open": "step.pipes.pipe_open",
        "support_off": "step.pipes.support_off", "support_none": "step.pipes.support_none",
        "lay_params": "step.lay.params", "wall_open": "step.lay.wall_open", "bay_none": "step.lay.bay_none",
        "turn": "step.check.turn", "kit_stated": "step.check.kit_stated", "kit_unit": "step.check.kit_unit",
        "kit_pitch": "step.check.kit_pitch"}


def _npy(path):
    raw = Path(path).read_bytes()
    hlen = struct.unpack("<H", raw[8:10])[0]
    head = raw[10:10 + hlen].decode()
    body = raw[10 + hlen:]
    return head, struct.unpack(f"<{len(body) // 4}{'f' if '<f4' in head else 'i'}", body)


def load(d):
    d = Path(d)
    man = json.loads((d / "manifest.json").read_text())
    names = [n for n in (d / "instances/names.txt").read_text().splitlines() if n]
    x, y, face = ([v / 1000.0 for v in _npy(d / f"instances/{k}.npy")[1]] for k in ("x", "y", "face"))
    z = list(_npy(d / "instances/z.npy")[1])
    asset = [None] * len(names)
    for ch in man["chunks"]:
        for a, (s, n) in ch.get("instances", {}).items():
            for k in range(s, s + n):
                asset[k] = a
    rows = {n: {"asset": asset[k], "x": x[k], "y": y[k], "z": z[k], "face": face[k]} for k, n in enumerate(names)}
    return man, rows


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


def place(r, p):
    """A point of an asset's frame, on the instance `r`: (x, y, z) in the world."""
    f = math.radians(r["face"])
    fx, fy = math.sin(f), math.cos(f)            # the front
    rx, ry = math.cos(f), -math.sin(f)           # the asset's +x
    return (r["x"] + p[0] * rx + p[1] * fx, r["y"] + p[0] * ry + p[1] * fy, r["z"] + (p[2] if len(p) > 2 else 0.0))


def open_ends(asset):
    """A pipe piece's open ends in its own frame, from its role and its model's measured box
    (`bounds`: x0, y0, z0, x1, y1, z1), never from what its pack states: a piece re-exported
    another length has its ends where its model has them."""
    tags, b = asset.get("tags") or [], asset.get("bounds")
    if not b:
        return []
    if "role:pipe" in tags or "role:slip" in tags:
        return [(b[0], 0, 0), (b[3], 0, 0)]             # it lies along its x, about the pipe's middle line
    if "role:elbow" in tags:
        return [(0, b[1], 0), (b[3], 0, 0)]             # one arm to its back, one to its right
    if "role:riser" in tags:
        return [(0, b[1], 0), (0, b[4], b[5] - (b[3] - b[0]) / 2)]      # its top arm's middle line: the box's top less half the pipe
    if "role:flange" in tags or "role:cap" in tags:
        return [(0, b[1], 0)]
    return []


def written(world):
    world = Path(world)
    text = world.read_text()
    files = [world] + [world.parent / f for f in re.findall(r'^\s*file\s*=\s*"([^"]+)"', text, re.M)]
    tables, cur = [], None
    for path in files:
        for line in path.read_text().splitlines():
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
    mine = [t for t in tables if t["table"] == "instance" or t["kind"] in ("building", "pipe")]
    return {"written_buildings": sum(1 for t in mine if t["kind"] == "building"),
            "written_pipes": sum(1 for t in mine if t["kind"] == "pipe"),
            "written_instances": sum(1 for t in mine if t["table"] == "instance"),
            "written_numbers": sum(t["numbers"] for t in mine)}


def said(diags, m):
    codes = Counter()
    for x in diags:
        codes[x["code"]] += int(x.get("count", 1))
    m["errors"] = sum(1 for x in diags if x["severity"] == "error")
    m["warnings"] = sum(1 for x in diags if x["severity"] == "warning")
    for key, code in SAID.items():
        m[key] = codes.get(code, 0)
    unmet = [x.get("data") or {} for x in diags if x["code"] == "surface.unmet"]
    m["unmet_miss_mm"] = max((d["miss"] for d in unmet if d.get("miss") is not None), default=-1)
    m["unmet_miss_z_mm"] = max((abs(d["miss_z"]) for d in unmet if d.get("miss_z") is not None), default=-1)
    m["unmet_named"] = sum(1 for d in unmet if d.get("near"))
    gaps = [abs((x.get("data") or {}).get("gap", 0)) for x in diags if x["code"] == "surface.gap"]
    m["gap_max_mm"], m["gap_min_mm"] = max(gaps, default=0), min(gaps, default=0)
    refused = [x.get("data") or {} for x in diags if x["code"] == "step.pipes.params"]
    m["params_turn_mdeg"] = max((int(round(d["turn"] * 1000)) for d in refused if "turn" in d), default=-1)
    m["params_climb_mm"] = max((int(round(d["climb"] * 1000)) for d in refused if "climb" in d), default=-1)
    m["params_leg_rise_mm"] = max((int(round(abs(d["to"] - d["from"]) * 1000)) for d in refused if "to" in d), default=-1)
    words = [{d.get("connector_a"), d.get("connector_b")} for d in (x.get("data") or {} for x in diags if x["code"] == "surface.connector")]
    m["connector_of_doors"] = sum(1 for w in words if "door" in w)
    m["connector_of_pipes"] = sum(1 for w in words if "pipe" in w)
    m["_codes"] = dict(sorted(codes.items()))


def measures(d, world=None):
    d = Path(d)
    m = {}
    said(json.loads((d / "diagnostics.json").read_text())["diagnostics"], m)
    if world:
        m.update(written(world))
    m["built"] = int((d / "manifest.json").exists())
    if not m["built"]:
        return m, {}
    man, rows = load(d)
    assets = man["assets"]
    feats = json.loads((d / "features.json").read_text())["features"]
    meets = json.loads((d / "meets.json").read_text())["meets"]
    m["instances"] = len(rows)
    legs = {f["name"] for f in feats if f["kind"] == "pipe_leg"}
    m["legs"] = len(legs)

    def tags(a):
        return assets[a].get("tags") or []
    for role, key in ROLES.items():
        m[key] = sum(1 for r in rows.values() if f"role:{role}" in tags(r["asset"]))
    m["ports"] = sum(1 for r in rows.values() if "port" in (assets[r["asset"]].get("mounts") or {}))
    rooms = {n for n, r in rows.items() if assets[r["asset"]].get("taxon") == "interior/room"}
    m["rooms"] = len(rooms)

    # ---- meetings
    def side(s):
        name, _, mount = s.rpartition(".")
        return (name, mount) if name in rows else (None, None)
    joints = []
    m["riser_meets"] = 0
    for r in meets:
        (an, am), (bn, bm) = side(r["a"]), side(r["b"])
        if an and bn and an != bn:
            joints.append(((an, am), (bn, bm)))
        for (xn, xm), other in (((an, am), r["b"]), ((bn, bm), r["a"])):
            if xn and xm == "top" and other.rpartition(".")[0] in legs:
                m["riser_meets"] += 1
    joined = {s for j in joints for s in j}
    m["joints"] = len(joints)
    m["port_joints"] = sum(1 for a, b in joints if {a[1], b[1]} == {"port", "plug"})
    m["door_joints"] = sum(1 for a, b in joints if a[0] in rooms and b[0] in rooms)

    # ---- the pipe, end to end
    pipes = {}
    for n, r in rows.items():
        hit = PIECE.match(n)
        if hit and any(t.startswith("role:") and t[5:] in ROLES for t in tags(r["asset"])):
            pipes.setdefault(hit.group(1), []).extend(place(r, e) for e in open_ends(assets[r["asset"]]))
    joins, loose, gap = 0, 0, 0
    for ends in pipes.values():
        left = list(ends)
        while left:
            e = left.pop()
            near = min(left, key=lambda o: math.dist(e, o), default=None)
            if near is not None and math.dist(e, near) <= JOIN + 1e-9:
                left.remove(near)
                joins += 1
            else:
                loose += 1
                others = [o for o in ends if o is not e and not any(o is not q and math.dist(o, q) <= JOIN + 1e-9 for q in ends)]
                far = min((math.dist(e, o) for o in others), default=None)
                gap = -1 if far is None or gap == -1 else max(gap, int(round(far * 1000)))
    m["pipe_joins"], m["pipe_loose"], m["pipe_gap_mm"] = joins, loose, gap
    m["plugs_joined"] = sum(1 for n, r in rows.items() if "role:flange" in tags(r["asset"]) and (n, "plug") in joined)

    # ---- supports
    ground = ground_of(d, man)
    feet = [abs(r["z"] + assets[r["asset"]]["bounds"][2] - ground(r["x"], r["y"])) for r in rows.values()
            if "role:support" in tags(r["asset"])]
    m["support_foot_mm"] = int(round(max(feet, default=0.0) * 1000))

    # ---- the ring
    doors = {}
    for n in rooms:
        for name, mount in (assets[rows[n]["asset"]].get("mounts") or {}).items():
            if mount.get("connector") == "door":
                doors[(n, name)] = place(rows[n], mount["at"])
    m["doors"] = len(doors)
    m["doors_joined"] = sum(1 for k in doors if k in joined)
    miss = 0.0
    for k, p in doors.items():
        if k in joined:
            continue
        miss = max(miss, min((math.dist(p[:2], q[:2]) for o, q in doors.items() if o[0] != k[0]), default=0.0))
    m["ring_miss_mm"] = int(round(miss * 1000))
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
    if "--base" in opt and m["built"]:
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
