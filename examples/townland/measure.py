"""Measures of a townland build, for expect.toml (R13, R15, R18).

    python3 examples/townland/measure.py BUILD_DIR [--base BASE_DIR] [--check SECTION]

Prints the measures as JSON. With --check, compares them against expect.toml's rows under SECTION
(`base`, `move_hill`, `move_crossing`, `edit_west`) and exits 1 when one falls outside its band
(Python 3.11+, for tomllib). With --base, also what changed against the base build, by name:
blocks, lots and houses added and removed, and of those the ones on the east bank (a centre east of
x = 190 m: the old town and the upper town, far from `edit_west`). Reads only the build directory
(spec/build.md): features.json, instances/names.txt and x.npy, diagnostics.json. Standard library only.

The street-form and deck numbers are lib/review/form.py's records (`step.form.*`, data.value);
`houses`, `lots_held` the houses step's; a block is a feature of kind `block`, a lot of kind `lot`.
"""

import json
import struct
import sys
from collections import Counter
from pathlib import Path

EAST_X = 190.0


def load(d):
    d = Path(d)
    fs = json.loads((d / "features.json").read_text())["features"]
    names = [n for n in (d / "instances/names.txt").read_text().splitlines() if n]
    diags = json.loads((d / "diagnostics.json").read_text())["diagnostics"]
    raw = (d / "instances/x.npy").read_bytes()
    hlen = struct.unpack("<H", raw[8:10])[0]
    body = raw[10 + hlen:]
    xs = [v / 1000.0 for v in struct.unpack(f"<{len(body) // 4}i", body)]
    return fs, names, diags, xs


def centre(f):
    ring = f["pieces"][0]["outer"] if f.get("pieces") else (f.get("points") or [f.get("at")])
    return sum(p[0] for p in ring) / len(ring), sum(p[1] for p in ring) / len(ring)


def measures(d):
    fs, names, diags, xs = load(d)
    by = {"block": {}, "lot": {}, "street": {}}
    for f in fs:
        if f.get("kind") in by:
            by[f["kind"]][f["name"]] = centre(f)
    m = {k: len(v) for k, v in by.items()}
    houses = {n: x for n, x in zip(names, xs) if n.startswith("houses#")}
    m["houses"] = len(houses)
    m["terraces"] = sum(1 for f in fs if f.get("kind") == "terrace")
    m["bookmarks"] = sum(1 for f in fs if f.get("kind") == "bookmark")
    codes = Counter(x["code"] for x in diags)
    for x in diags:
        c = x["code"]
        data = x.get("data") or {}
        if c.startswith("step.form.") and "value" in data:
            m[c[len("step.form."):]] = data["value"]
            if c == "step.form.dead_end_count":
                for k in ("at_edge", "at_gate", "stubs", "inside"):
                    m["dead_end_" + k] = data.get(k)
        if c == "step.houses.rows":
            m["lots_held"] = data.get("lots_held")
        if c == "step.reach.walked":
            m["reach_held"], m["reach_pairs"] = data.get("held"), data.get("pairs")
    for code in ("place.relief", "place.keep_out", "geom.unowned_crossing", "authority.conflict",
                 "authority.max_change", "geom.cell_none", "geom.cell_small"):
        m[code] = codes.get(code, 0)
    m["errors"] = sum(1 for x in diags if x["severity"] == "error")
    m["warnings"] = sum(1 for x in diags if x["severity"] == "warning")
    return m, by, houses


def main(argv):
    m, by, houses = measures(argv[0])
    if "--base" in argv:
        mb, bb, hb = measures(argv[argv.index("--base") + 1])
        for k in ("block", "lot", "street"):
            a, b = set(by[k]), set(bb[k])
            m[f"{k}_added"], m[f"{k}_removed"] = len(a - b), len(b - a)
            m[f"{k}_added_east"] = sum(1 for n in a - b if by[k][n][0] > EAST_X)
            m[f"{k}_removed_east"] = sum(1 for n in b - a if bb[k][n][0] > EAST_X)
        a, b = set(houses), set(hb)
        m["houses_added"], m["houses_removed"] = len(a - b), len(b - a)
        m["houses_removed_east"] = sum(1 for n in b - a if hb[n] > EAST_X)
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
