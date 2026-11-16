"""Measures of a county build, for expect.toml (docs/plan.md M4: the town as a fill on simulated land).

    python3 examples/county/measure.py BUILD [--run FILE] [--check SECTION]

Prints the measures as JSON. With --check, compares them against expect.toml's rows under SECTION
(`base`, `move_town`, `forgotten_exit`): prints each row and its measure, and exits 1 when one falls
outside its band or names a measure not taken (Python 3.11+, for tomllib). Reads only build
directories (spec/build.md): features.json, instances/names.txt, diagnostics.json, fields/*.npy and
manifest.json; with --run, the run report `<BUILD>.run.json` (build.md 11.3) of a change built on
the base's store. Standard library only.

How each is taken (ours):
- `town_features`, `town_instances`, `town_streets`: features, instances and features of kind
  `street` named `town.*` (composition.md 4: scoped names). `town_streets` is the import's own step
  (quay's `streets`): 0 while an import's steps do not run (TODO M4 slice 2, `import_step`).
- `road_off`: metres between the road's last point and `town.gate` as features.json places them: 0
  when the road ends on the exit it answers.
- `deck_off`: the largest |height - 3.5| at the posts nearest the points of `town.quay`, the deck
  (lib/town `waterfront`, `level = 3.5`): the town's quay holds on the county's shore.
- `gate_ground_off`: |height at `town.gate` - the gate's z|: the exit stands on the joined ground.
- `wet_views`: bookmarks whose post holds water deeper than 5 cm (`water - height`, nearest post).
- `toward_off`: the largest distance, metres, from a bookmark's looked-at point to the point of the
  thing its `toward` names, as placed.
- `seam`, `seam_grade`: `import.seam` diagnostics and their largest grade.
- `unanswered`, `unanswered_write`: `import.exit_unanswered` and each one's `data.write`.
- With --run: `ran_import_town` (the node `import:town` that ran), `ran_placed_town` (`placed:town`
  and `placed:town/...` that ran), `fields_ran` and `fields` (field chunk nodes that ran, all of
  them), `shapes_ran` (shape nodes that ran, by name).
"""

import json
import math
import struct
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent


def grid(path):
    """A 2-D little-endian float32 .npy: (rows, columns, flat values)."""
    raw = Path(path).read_bytes()
    hlen = struct.unpack("<H", raw[8:10])[0]
    head = raw[10:10 + hlen].decode()
    rows, cols = (int(v) for v in head.split("'shape': (")[1].split(")")[0].split(",")[:2])
    body = raw[10 + hlen:]
    return rows, cols, struct.unpack(f"<{len(body) // 4}f", body)


def frame(d):
    m = json.loads((Path(d) / "manifest.json").read_text())["frame"]
    return m["extent"][0], m["extent"][1], m["resolution"]


def post(d, g, x, y):
    """The value of grid g of build d at the post nearest (x, y)."""
    x0, y0, res = frame(d)
    rows, cols, v = g
    i = min(max(round((x - x0) / res), 0), cols - 1)
    j = min(max(round((y - y0) / res), 0), rows - 1)
    return v[j * cols + i]


def features(d):
    fs = json.loads((Path(d) / "features.json").read_text())["features"]
    return {f["name"]: f for f in fs}


def where(f):
    if "at" in f:
        return f["at"][:2]
    pts = f.get("points") or []
    return pts[0][:2] if pts else None


def measure(d, run=None):
    d = Path(d)
    diags = json.loads((d / "diagnostics.json").read_text())["diagnostics"]
    m = {
        "errors": sum(1 for x in diags if x["severity"] == "error"),
        "warnings": sum(1 for x in diags if x["severity"] == "warning"),
        "unanswered": sum(1 for x in diags if x["code"] == "import.exit_unanswered"),
        "unanswered_write": [x["data"].get("write") for x in diags if x["code"] == "import.exit_unanswered"],
    }
    seams = [x for x in diags if x["code"] == "import.seam"]
    m["seam"] = len(seams)
    m["seam_grade"] = round(max((x["data"]["grade"] for x in seams), default=0.0), 3)
    if run:
        nodes = json.loads(Path(run).read_text())["nodes"]
        m["ran_import_town"] = sum(1 for n in nodes if n["id"] == "import:town" and n["status"] == "ran")
        m["ran_placed_town"] = sum(
            1 for n in nodes
            if (n["id"] == "placed:town" or n["id"].startswith("placed:town/")) and n["status"] == "ran"
        )
        fl = [n for n in nodes if n["family"] == "field"]
        m["fields_ran"] = sum(1 for n in fl if n["status"] == "ran")
        m["fields"] = len(fl)
        m["shapes_ran"] = sorted(
            n["id"].split(":", 1)[1].split("/")[0] for n in nodes if n["family"] == "shape" and n["status"] == "ran"
        )
    if not (d / "features.json").exists():      # a build that ended in an error writes no data
        return m
    fs = features(d)
    names = [n for n in (d / "instances/names.txt").read_text().splitlines() if n]
    m["town_features"] = sum(1 for n in fs if n.startswith("town."))
    m["town_instances"] = sum(1 for n in names if n.startswith("town."))
    m["town_streets"] = sum(1 for n, f in fs.items() if n.startswith("town.") and f.get("kind") == "street")
    m["bookmarks"] = sum(1 for f in fs.values() if f.get("kind") == "bookmark")
    h = grid(d / "fields/height.npy")
    w = grid(d / "fields/water.npy")
    gate = fs["town.gate"]["at"]
    road = fs["north_road"]["points"]
    m["road_off"] = round(math.dist(road[-1][:2], gate[:2]), 3)
    m["gate_ground_off"] = round(abs(post(d, h, gate[0], gate[1]) - gate[2]), 3) if len(gate) > 2 else None
    m["deck_off"] = round(max(abs(post(d, h, p[0], p[1]) - 3.5) for p in fs["town.quay"]["points"]), 3)
    wet, off = 0, 0.0
    for f in fs.values():
        if f.get("kind") != "bookmark":
            continue
        x, y = f["at"][:2]
        if post(d, w, x, y) - post(d, h, x, y) > 0.05:
            wet += 1
        t = f["attrs"].get("toward")
        if isinstance(t, dict) and t.get("name") in fs:
            own = where(fs[t["name"]])
            if own:
                off = max(off, math.dist(own, t["at"][:2]))
    m["wet_views"] = wet
    m["toward_off"] = round(off, 3)
    return m


def check(m, section):
    import tomllib

    rows = tomllib.loads((HERE / "expect.toml").read_text())[section]
    bad = 0
    for name, row in rows.items():
        got = m.get(row["measure"])
        band = row["band"]
        if band and isinstance(band[0], str):
            ok = got == band
        else:
            ok = isinstance(got, (int, float)) and band[0] <= got <= band[1]
        bad += not ok
        print(f"{'ok  ' if ok else 'FAIL'} {section}.{name}: {row['measure']} = {got}, band {band}")
    return bad


def main(argv):
    args, run, section = [], None, None
    it = iter(argv)
    for a in it:
        if a == "--run":
            run = next(it)
        elif a == "--check":
            section = next(it)
        else:
            args.append(a)
    m = measure(args[0], run)
    print(json.dumps(m, indent=1, sort_keys=True))
    if section:
        sys.exit(1 if check(m, section) else 0)


if __name__ == "__main__":
    main(sys.argv[1:])
