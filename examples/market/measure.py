"""Measures of a market build, for expect.toml (the town kit's three changes, TODO "Dogfood: the
market-street district"; docs/plan.md M5).

    python3 examples/market/measure.py BUILD [--base BASE] [--check SECTION]

Prints the measures as JSON. With --check, compares them against expect.toml's rows under SECTION
(`base`, `remodel`, `module`, `move_street`): prints each row and its measure, and exits 1 when one
falls outside its band or names a measure not taken (Python 3.11+, for tomllib). With --base, also
what changed against the base build, by name. Reads only the build directory (spec/build.md):
manifest.json, diagnostics.json and instances/. Standard library only.

How each is taken (ours):
- A row piece is an instance named `rows_left#...` or `rows_right#...` (the two kit steps); its
  asset from the manifest's chunk ranges. `bay`, `filler`, `end`, `corner` count them by asset
  (`row_bay`, `row_filler`, `row_end`, `row_corner`).
- `kit_*`: the two steps' `step.<id>.kit` records (lib/town/kit.py), summed (`kit_max_step` the
  larger): stretches, gaps, open stretches, joints opened and pieces dropped by rule 7, bays split
  and pieces too steep by rule 6, the largest step between neighbours' tops (metres).
- `max_joint_cm` / `mean_joint_cm`: along each stretch (pieces `<run>_<leg>_<stretch>_<k>` in `k`
  order), the distance between neighbours' origins less half of each one's `params.pitch` (the
  manifest's), in centimetres: what a joint opens. A chord on a bend reads a little short of the arc.
- `revised`: `asset.revised` records; `revised_named` the instances they name;
  `revised_all` 1 when, for every record, the names it lists are exactly the build's instances of
  that asset (assets.md 4.2: "names every instance"), else 0.
- `place.*`, `errors`, `geom.unowned_crossing`: counts of records by code or severity.
- Doors (quay's way): each piece's `door` mount turned by its face; its floor is the ground at its
  origin plus its `z`; the ground is read 3 m out from the door (the footway), bilinear between the
  0.5 m posts of `fields/height.npy`. `doors_level` counts doors within 0.3 m of that ground,
  `doors_off` those more than 1 m off it.
- With --base: `kept`, `added`, `removed` row pieces by name; `moved` the kept ones whose origin or
  face differ by more than 1 mm or 0.001 degree; each split by the street a row follows: `_main`
  for runs `row_main_*` (the main street's rows), `_other` for the rest.
"""

import json
import math
import struct
import sys
from collections import Counter, defaultdict
from pathlib import Path

ROLES = {"row_bay": "bay", "row_filler": "filler", "row_end": "end", "row_corner": "corner"}
DOOR_OUT, DOOR_LEVEL, DOOR_OFF = 3.0, 0.3, 1.0


def _npy(path):
    raw = Path(path).read_bytes()
    hlen = struct.unpack("<H", raw[8:10])[0]
    head = raw[10:10 + hlen].decode()
    code = "f" if "<f4" in head else "i"
    body = raw[10 + hlen:]
    return head, struct.unpack(f"<{len(body) // 4}{code}", body)


def load(d):
    d = Path(d)
    man = json.loads((d / "manifest.json").read_text())
    diags = json.loads((d / "diagnostics.json").read_text())["diagnostics"]
    names = [n for n in (d / "instances/names.txt").read_text().splitlines() if n]
    x, y, z, face = ([v / 1000.0 for v in _npy(d / f"instances/{k}.npy")[1]] for k in ("x", "y", "z", "face"))
    asset = [None] * len(names)
    for ch in man["chunks"]:
        for a, (s, n) in ch.get("instances", {}).items():
            for k in range(s, s + n):
                asset[k] = a
    rows = {n: {"asset": asset[k], "x": x[k], "y": y[k], "z": z[k], "face": face[k]}
            for k, n in enumerate(names) if n.startswith(("rows_left#", "rows_right#"))}
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
        at = lambda a, b: h[b * cols + a]  # noqa: E731
        return ((at(i0, j0) * (1 - u) + at(i0 + 1, j0) * u) * (1 - v)
                + (at(i0, j0 + 1) * (1 - u) + at(i0 + 1, j0 + 1) * u) * v)
    return ground


def run_of(name):
    """(run, stretch key, k) of a piece name `<step>#<run>_<leg>_<stretch>_<k>[_<j>]`, or None for
    a corner (`<run>_c<v>`)."""
    body = name.split("#", 1)[1]
    parts = body.split("_")
    if parts[-1].startswith("c"):
        return None
    if len(parts) >= 5 and all(p.isdigit() for p in parts[-4:]):      # a bay split by rule 6
        parts = parts[:-1]
    return "_".join(parts[:-3]), "_".join(parts[:-1]), int(parts[-1])


def measures(d):
    man, diags, rows = load(d)
    codes = Counter(x["code"] for x in diags)
    m = {"pieces": len(rows)}
    by_asset = Counter(r["asset"] for r in rows.values())
    for a, role in ROLES.items():
        m[role] = by_asset.get(a, 0)
    kits = [x.get("data") or {} for x in diags if x["code"] in ("step.rows_left.kit", "step.rows_right.kit")]
    for k in ("stretches", "gaps", "open", "opened", "dropped", "split", "steep"):
        m["kit_" + k] = sum(r.get(k, 0) for r in kits) if kits else None
    m["kit_max_step"] = max((r.get("max_step", 0.0) for r in kits), default=None)
    # Joints along each stretch.
    pitch = {a: (v.get("params") or {}).get("pitch") for a, v in man["assets"].items()}
    stretch = defaultdict(list)
    for n, r in rows.items():
        k = run_of(n)
        if k:
            stretch[k[1]].append((k[2], r))
    joints = []
    for pieces in stretch.values():
        pieces.sort(key=lambda p: p[0])
        for (_, a), (_, b) in zip(pieces, pieces[1:]):
            joints.append(math.dist((a["x"], a["y"]), (b["x"], b["y"])) - (pitch[a["asset"]] + pitch[b["asset"]]) / 2)
    m["max_joint_cm"] = round(100 * max(joints), 1) if joints else None
    m["mean_joint_cm"] = round(100 * sum(joints) / len(joints), 2) if joints else None
    m["min_joint_cm"] = round(100 * min(joints), 1) if joints else None
    # asset.revised: names every instance.
    rev = [x for x in diags if x["code"] == "asset.revised"]
    m["revised"] = len(rev)
    m["revised_named"] = sum(len((x.get("data") or {}).get("instances", [])) for x in rev)
    ok = 1
    for x in rev:
        a = x["subject"].split(".", 1)[1]
        named = set((x.get("data") or {}).get("instances", []))
        ok &= named == {n for n, r in rows.items() if r["asset"] == a}
    m["revised_all"] = ok if rev else None
    for code in ("place.keep_out", "place.overlap", "place.crowded", "place.relief", "geom.unowned_crossing",
                 "diag.internal"):
        m[code] = codes.get(code, 0)
    m["errors"] = sum(1 for x in diags if x["severity"] == "error")
    m["bookmarks"] = sum(1 for f in json.loads((Path(d) / "features.json").read_text())["features"]
                         if f.get("kind") == "bookmark")
    # Doors.
    ground = ground_of(d, man)
    off = []
    for r in rows.values():
        door = ((man["assets"].get(r["asset"]) or {}).get("mounts") or {}).get("door")
        if door is None:
            continue
        t = math.radians(r["face"])
        s, c = math.sin(t), math.cos(t)
        dx = r["x"] + door["at"][0] * c + door["at"][1] * s
        dy = r["y"] - door["at"][0] * s + door["at"][1] * c
        floor = ground(r["x"], r["y"]) + r["z"]
        off.append(abs(floor - ground(dx + DOOR_OUT * s, dy + DOOR_OUT * c)))
    m["doors"] = len(off)
    m["doors_level"] = sum(1 for v in off if v <= DOOR_LEVEL)
    m["doors_off"] = sum(1 for v in off if v > DOOR_OFF)
    return m, rows


def main(argv):
    m, rows = measures(argv[0])
    if "--base" in argv:
        _, rb = measures(argv[argv.index("--base") + 1])
        side = lambda n: "main" if n.split("#", 1)[1].startswith("row_main_") else "other"  # noqa: E731
        for part in ("main", "other"):
            a = {n for n in rows if side(n) == part}
            b = {n for n in rb if side(n) == part}
            kept = a & b
            m["kept_" + part] = len(kept)
            m["added_" + part] = len(a - b)
            m["removed_" + part] = len(b - a)
            m["moved_" + part] = sum(1 for n in kept if abs(rows[n]["x"] - rb[n]["x"]) > 0.001
                                     or abs(rows[n]["y"] - rb[n]["y"]) > 0.001
                                     or abs(rows[n]["face"] - rb[n]["face"]) > 0.001)
            m["swapped_" + part] = sum(1 for n in kept if rows[n]["asset"] != rb[n]["asset"])
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
