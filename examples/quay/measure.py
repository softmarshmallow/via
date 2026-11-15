"""Measures of a quay build, for expect.toml (R13's four obligations; R18's meetings).

    python3 examples/quay/measure.py BUILD_DIR [--base BASE_DIR] [--run FILE] [--check SECTION]

Prints the measures as JSON. With --check, compares them against expect.toml's rows under
SECTION (`base`, `clear_one`, `pin_one`, `unrelated`, `move_hill`): prints each row and its
measure, and exits 1 when one falls outside its band or names a measure not taken (Python 3.11+,
for tomllib). With --base, also what changed against the base build, by name:
blocks, lots and houses added and removed, and of those, the ones whose centre lies west of
x = 200 m (`*_west`: far from the east-edge edit of `unrelated`). Reads only the build directory
(spec/build.md): features.json, instances/names.txt and diagnostics.json, and with --run the file
`via build --diagnostics FILE` wrote: run diagnostics (`identity.reselected` among them) are never
in diagnostics.json (spec/build.md 11.1), so R13's obligation 3 is counted from that file.
Standard library only.

How each is taken (ours): a block is a feature of kind `block`, a lot of kind `lot`, a rest piece a
region of the cells table with no kind; a house an instance named `houses#...`; a pin's instance
is named by its pin id. A centre is the mean of a region's outer ring's vertices. `lots_held` is the
houses step's own count (its `step.houses.rows` info: lots with at least one house of a row on them),
`terraces` the regions of kind `terrace` it returns (one per run of houses wall to wall).
"""

import json
import struct
import sys
from collections import Counter
from pathlib import Path


def load(d):
    d = Path(d)
    fs = json.loads((d / "features.json").read_text())["features"]
    names = [n for n in (d / "instances/names.txt").read_text().splitlines() if n]
    diags = json.loads((d / "diagnostics.json").read_text())["diagnostics"]
    xs = _i32(d / "instances/x.npy")
    return fs, names, diags, xs


def _i32(path):
    raw = path.read_bytes()
    hlen = struct.unpack("<H", raw[8:10])[0]
    body = raw[10 + hlen:]
    return [v / 1000.0 for v in struct.unpack(f"<{len(body) // 4}i", body)]


def centre(f):
    if f.get("pieces"):
        ring = f["pieces"][0]["outer"]
    else:
        ring = f.get("points") or [f.get("at")]
    return sum(p[0] for p in ring) / len(ring), sum(p[1] for p in ring) / len(ring)


def classes(fs):
    out = {"block": {}, "lot": {}, "rest": {}, "street": {}}
    for f in fs:
        k, n = f.get("kind"), f["name"]
        if k == "block":
            out["block"][n] = centre(f)
        elif k == "lot":
            out["lot"][n] = centre(f)
        elif n.startswith("lots#"):
            out["rest"][n] = centre(f)
        elif k == "street":
            out["street"][n] = centre(f)
    return out


def measures(d):
    fs, names, diags, xs = load(d)
    c = classes(fs)
    codes = Counter(x["code"] for x in diags)
    m = {k: len(v) for k, v in c.items()}
    m["houses"] = sum(1 for n in names if n.startswith("houses#"))
    m["instances"] = len(names)
    m["bookmarks"] = sum(1 for f in fs if f.get("kind") == "bookmark")
    m["terraces"] = sum(1 for f in fs if f.get("kind") == "terrace")
    rows = [x.get("data") or {} for x in diags if x["code"] == "step.houses.rows"]
    m["lots_held"] = rows[0].get("lots_held") if rows else None      # lots with a house on them
    for code in ("place.relief", "place.keep_out", "geom.dead_end", "geom.unowned_crossing",
                 "identity.pin_removed", "identity.reselected", "step.streets.none"):
        m[code] = codes.get(code, 0)
    m["errors"] = sum(1 for x in diags if x["severity"] == "error")
    return m, c, set(names)


def run_codes(path):
    """Counts by code in a `--diagnostics FILE` document (diagnostics.md 7): its `diagnostics`
    hold the world's records and the run's."""
    doc = json.loads(Path(path).read_text())
    return Counter(x["code"] for x in doc["diagnostics"])


def main(argv):
    m, c, names = measures(argv[0])
    if "--run" in argv:
        rc = run_codes(argv[argv.index("--run") + 1])
        m["identity.reselected"] = rc.get("identity.reselected", 0)
    if "--base" in argv:
        mb, cb, nb = measures(argv[argv.index("--base") + 1])
        for k in ("block", "lot", "rest", "street"):
            a, b = set(c[k]), set(cb[k])
            m[f"{k}_added"], m[f"{k}_removed"] = len(a - b), len(b - a)
            west = lambda s, cc: {n for n in s if cc[n][0] < 200}
            m[f"{k}_added_west"] = len(west(a - b, c[k]))
            m[f"{k}_removed_west"] = len(west(b - a, cb[k]))
        m["houses_added"] = len({n for n in names - nb if n.startswith("houses#")})
        m["houses_removed"] = len({n for n in nb - names if n.startswith("houses#")})
        _, names_b, _, xs_b = load(argv[argv.index("--base") + 1])
        xb = dict(zip(names_b, xs_b))
        m["houses_removed_west"] = len({n for n in nb - names
                                        if n.startswith("houses#") and xb[n] < 200})
        m["other_added"] = sorted(n for n in names - nb if not n.startswith("houses#"))
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
