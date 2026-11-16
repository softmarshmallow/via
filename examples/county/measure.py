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
- `walk_wet`, `walk_wet_max`: posts on the treads of the ways (`road/street/main_street/lane/trail` paths: within half their
  `width` of the line) where the water stands more than 0.05 m over the ground, and the deepest
  (composition.md 6.2, a field derived from another). Ours, M4 slice 4.
- `materials`: the materials the build draws (manifest.json `materials`), each import's paint
  brought with its ground (composition.md 6.2). Ours, M4 slice 4.
- `wall_mid`, `wall_mid_off`: posts half-way down a wall (two neighbours across one axis more than
  2 m apart, the post more than 0.5 m from each), and the largest such gap: the quay wall of a
  turned town read through `sample` (composition.md 6.2, wall cells). Ours, M4 slice 4.
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



def wall_mid(d):
    """`wall_mid`, `wall_mid_off`: posts half-way down a wall: a post whose two neighbours across
    one axis (east and west, or north and south) differ by more than 2 m while it stands more than
    0.5 m from each; and the largest of the smaller of those two gaps, metres. 0 where a wall runs
    from a post at its top straight to one at its foot."""
    rows, cols, h = grid(Path(d) / "fields" / "height.npy")
    n, worst = 0, 0.0
    for j in range(1, rows - 1):
        for i in range(1, cols - 1):
            p = h[j * cols + i]
            for a, b in ((h[j * cols + i - 1], h[j * cols + i + 1]), (h[(j - 1) * cols + i], h[(j + 1) * cols + i])):
                hi, lo = max(a, b), min(a, b)
                if hi - lo > 2 and hi - p > 0.5 and p - lo > 0.5:
                    n += 1
                    worst = max(worst, min(hi - p, p - lo))
                    break
    return n, round(worst, 3)


def walk_wet(d, fs, kinds):
    """`walk_wet`, `walk_wet_max`: the posts within half a way's `width` of its line, for every
    path feature of `kinds`, where `water - height` exceeds 0.05 m; their count and the deepest,
    metres. Every post of the extent's lattice, nearest-segment distance in the plane."""
    h, w = grid(d / "fields" / "height.npy"), grid(d / "fields" / "water.npy")
    rows, cols, hv = h
    wv = w[2]
    wet, worst = set(), 0.0
    for f in fs.values():
        if f.get("kind") not in kinds or f.get("shape") != "path":
            continue
        half = f["attrs"].get("width", 2) / 2
        pts = [tuple(p[:2]) for p in f["points"]]
        for (ax, ay), (bx, by) in zip(pts, pts[1:]):
            L2 = (bx - ax) ** 2 + (by - ay) ** 2
            for j in range(max(0, int(min(ay, by) - half) - 1), min(rows, int(max(ay, by) + half) + 2)):
                for i in range(max(0, int(min(ax, bx) - half) - 1), min(cols, int(max(ax, bx) + half) + 2)):
                    t = 0 if L2 == 0 else max(0, min(1, ((i - ax) * (bx - ax) + (j - ay) * (by - ay)) / L2))
                    if math.hypot(i - ax - t * (bx - ax), j - ay - t * (by - ay)) > half:
                        continue
                    dep = wv[j * cols + i] - hv[j * cols + i]
                    if dep > 0.05:
                        wet.add(j * cols + i)
                        worst = max(worst, dep)
    return len(wet), round(worst, 3)

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
    m["wall_mid"], m["wall_mid_off"] = wall_mid(d)
    m["materials"] = sorted(json.loads((Path(d) / "manifest.json").read_text())["materials"])
    m["walk_wet"], m["walk_wet_max"] = walk_wet(Path(d), fs, ("road", "street", "main_street", "lane", "trail"))
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
