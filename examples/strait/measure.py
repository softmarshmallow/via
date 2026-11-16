"""Measures of a strait build, for expect.toml (docs/plan.md M4: "an import moved and turned").

    python3 examples/strait/measure.py BUILD [--alone a=DIR --alone b=DIR] [--run FILE] [--check SECTION]

Prints the measures as JSON. With --check, compares them against expect.toml's rows under
SECTION (`base`, `move_b`, `harbour`, `forgotten_exit`): prints each row and its measure, and exits
1 when one falls outside its band or names a measure not taken (Python 3.11+, for tomllib).
Reads only build directories (spec/build.md): features.json, instances/names.txt,
diagnostics.json, fields/*.npy and manifest.json; with --run, the run report `<BUILD>.run.json`
(build.md 11.3) of a change built on the base's store. Standard library only.

How each is taken (ours):
- `a_instances`, `b_instances`: instances named `a.*`, `b.*` (composition.md 4: scoped names).
- `ferry_off`: the larger distance, metres, between the ferry's first point and `a.slip`, and its
  last point and `b.slip`, as features.json places them: 0 when the line follows what it names.
- `slip_z_off`: the larger |z - 1| of the two slipways (their `level` is 1 m over the datum).
- `wet_views`: bookmarks whose post holds water deeper than 5 cm (`water - height`, nearest post).
- `toward_off`: for every bookmark whose `toward` names a feature or instance of the build, the
  distance in metres from the looked-at point to that thing's own point (an import's view looking
  at its import-frame point shows here as metres off).
- `seam`, `seam_grade`, `seam_count`: `import.seam` records, the worst `data.grade`, the summed
  `data.count`; `unanswered`: `import.exit_unanswered` records; `errors`: error records.
- `alone_off_<id>`: with --alone, the largest |parent height - island height| over the island's
  posts that stand above 2 m down (where the blend's weight is 1, `world.toml`), the island's
  frame found from its `slip` and `at_slip` points in both builds; metres.
- With --run: `ran_import_<id>` and `ran_placed_<id>`, the nodes `import:<id>` and
  `placed:<id>[/...]` that ran (status `ran`); `fields_ran` and `fields`, field chunk nodes that
  ran and all of them; `shapes_ran`, the shape nodes that ran, by name.
- `walk_wet`, `walk_wet_max`: posts on the treads of the ways (`lane/road/trail` paths: within half their
  `width` of the line) where the water stands more than 0.05 m over the ground, and the deepest
  (composition.md 6.2, a field derived from another). Ours, M4 slice 4.
- `turn_sheer_<id>`, `turn_sheer_off_<id>`: with --alone, the parent posts that land between the
  island's posts (only when it is turned off the quarter turns) inside an island cell whose four
  posts span more than 1 m (a wall, the slipway's sides), and the largest |parent height - the
  island's nearest post| there: how far `sample`'s bilinear rule (composition.md 6.2) bends sheer
  ground it turns. A bound, not a fix: a 1 m lattice turned 10 degrees cannot hold a wall along its
  old line. Ours, M4 slice 4.
- `materials`: the materials the build draws (manifest.json `materials`), each import's paint
  brought with its ground (composition.md 6.2). Ours, M4 slice 4.
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


def measure(d, alone=None, run=None):
    d = Path(d)
    diags = json.loads((d / "diagnostics.json").read_text())["diagnostics"]
    if not (d / "features.json").exists():      # a build that ended in an error writes no data
        return {
            "errors": sum(1 for x in diags if x["severity"] == "error"),
            "unanswered": sum(1 for x in diags if x["code"] == "import.exit_unanswered"),
            "unanswered_write": [x["data"].get("write") for x in diags if x["code"] == "import.exit_unanswered"],
        }
    fs = features(d)
    names = [n for n in (d / "instances/names.txt").read_text().splitlines() if n]
    m = {
        "features": len(fs),
        "instances": len(names),
        "a_instances": sum(1 for n in names if n.startswith("a.")),
        "b_instances": sum(1 for n in names if n.startswith("b.")),
        "bookmarks": sum(1 for f in fs.values() if f.get("kind") == "bookmark"),
        "errors": sum(1 for x in diags if x["severity"] == "error"),
        "warnings": sum(1 for x in diags if x["severity"] == "warning"),
        "unanswered": sum(1 for x in diags if x["code"] == "import.exit_unanswered"),
    }
    seams = [x for x in diags if x["code"] == "import.seam"]
    m["seam"] = len(seams)
    m["seam_grade"] = round(max((x["data"]["grade"] for x in seams), default=0.0), 3)
    m["seam_count"] = sum(x["data"]["count"] for x in seams)
    ferry = fs.get("ferry")
    if ferry:
        p = ferry["points"]
        a, b = fs["a.slip"]["at"], fs["b.slip"]["at"]
        m["ferry_off"] = round(max(math.dist(p[0][:2], a[:2]), math.dist(p[-1][:2], b[:2])), 3)
        m["ferry_length"] = round(sum(math.dist(p[k][:2], p[k + 1][:2]) for k in range(len(p) - 1)), 1)
        m["slip_z_off"] = round(max(abs(a[2] - 1), abs(b[2] - 1)), 3)
    h = grid(d / "fields/height.npy")
    w = grid(d / "fields/water.npy")
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
    m["materials"] = sorted(json.loads((Path(d) / "manifest.json").read_text())["materials"])
    m["walk_wet"], m["walk_wet_max"] = walk_wet(Path(d), fs, ("lane", "road", "trail"))
    for iid, ad in (alone or {}).items():
        m[f"alone_off_{iid}"] = alone_off(d, fs, iid, Path(ad))
        m[f"turn_sheer_{iid}"], m[f"turn_sheer_off_{iid}"] = turn_sheer(d, fs, iid, Path(ad))
    if run:
        nodes = json.loads(Path(run).read_text())["nodes"]
        for iid in ("a", "b"):
            m[f"ran_import_{iid}"] = sum(1 for n in nodes if n["id"] == f"import:{iid}" and n["status"] == "ran")
            m[f"ran_placed_{iid}"] = sum(
                1 for n in nodes
                if (n["id"] == f"placed:{iid}" or n["id"].startswith(f"placed:{iid}/")) and n["status"] == "ran"
            )
        fl = [n for n in nodes if n["family"] == "field"]
        m["fields"] = len(fl)
        m["fields_ran"] = sum(1 for n in fl if n["status"] == "ran")
        m["shapes_ran"] = sorted(
            n["id"].split(":", 1)[1].split("/")[0] for n in nodes if n["family"] == "shape" and n["status"] == "ran"
        )
    return m


def alone_off(d, fs, iid, ad):
    """Largest |parent - island| height over the island's posts above 2 m down (module doc)."""
    home = features(ad)
    p0, p1 = home["slip"]["at"][:2], home["at_slip"]["at"][:2]
    q0, q1 = fs[f"{iid}.slip"]["at"][:2], fs[f"{iid}.at_slip"]["at"][:2]
    ang = math.atan2(q1[1] - q0[1], q1[0] - q0[0]) - math.atan2(p1[1] - p0[1], p1[0] - p0[0])
    c, s = math.cos(ang), math.sin(ang)
    tx = q0[0] - (c * p0[0] - s * p0[1])
    ty = q0[1] - (s * p0[0] + c * p0[1])
    hp = grid(d / "fields/height.npy")
    rows, cols, ha = grid(ad / "fields/height.npy")
    ax0, ay0, ares = frame(ad)
    px0, py0, pres = frame(d)
    prow, pcol, pv = hp
    worst = 0.0
    for j in range(rows):
        for i in range(cols):
            v = ha[j * cols + i]
            if v <= -2:
                continue
            x, y = ax0 + i * ares, ay0 + j * ares
            X, Y = c * x - s * y + tx, s * x + c * y + ty
            I, J = (X - px0) / pres, (Y - py0) / pres
            if abs(I - round(I)) > 1e-6 or abs(J - round(J)) > 1e-6:
                continue              # a turned island's post between the parent's posts
            I, J = round(I), round(J)
            if 0 <= I < pcol and 0 <= J < prow:
                worst = max(worst, abs(pv[J * pcol + I] - v))
    return round(worst, 3)


def turn_sheer(d, fs, iid, ad):
    """`turn_sheer_<id>`, `turn_sheer_off_<id>` (module doc): the parent posts that fall between
    the island's posts (a turned island) inside a cell of the island whose four posts span more
    than 1 m, and the largest |parent height - the island's nearest post| over them."""
    home = features(ad)
    p0, p1 = home["slip"]["at"][:2], home["at_slip"]["at"][:2]
    q0, q1 = fs[f"{iid}.slip"]["at"][:2], fs[f"{iid}.at_slip"]["at"][:2]
    ang = math.atan2(q1[1] - q0[1], q1[0] - q0[0]) - math.atan2(p1[1] - p0[1], p1[0] - p0[0])
    c, s = math.cos(ang), math.sin(ang)
    tx = q0[0] - (c * p0[0] - s * p0[1])
    ty = q0[1] - (s * p0[0] + c * p0[1])
    prow, pcol, pv = grid(d / "fields/height.npy")
    rows, cols, ha = grid(ad / "fields/height.npy")
    ax0, ay0, ares = frame(ad)
    px0, py0, pres = frame(d)
    count, worst = 0, 0.0
    for J in range(prow):
        for I in range(pcol):
            X, Y = px0 + I * pres - tx, py0 + J * pres - ty
            x, y = c * X + s * Y, -s * X + c * Y           # the island's frame
            fi, fj = (x - ax0) / ares, (y - ay0) / ares
            i, j = math.floor(fi + 1e-6), math.floor(fj + 1e-6)
            if not (0 <= i < cols - 1 and 0 <= j < rows - 1):
                continue
            if abs(fi - round(fi)) < 1e-3 and abs(fj - round(fj)) < 1e-3:
                continue                                   # on a post: nothing between
            four = [ha[(j + b) * cols + i + a] for a in (0, 1) for b in (0, 1)]
            if max(four) - min(four) <= 1:
                continue
            count += 1
            near = ha[min(rows - 1, round(fj)) * cols + min(cols - 1, round(fi))]
            worst = max(worst, abs(pv[J * pcol + I] - near))
    return count, round(worst, 3)


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
    args, alone, run, section = [], {}, None, None
    it = iter(argv)
    for a in it:
        if a == "--alone":
            k, v = next(it).split("=", 1)
            alone[k] = v
        elif a == "--run":
            run = next(it)
        elif a == "--check":
            section = next(it)
        else:
            args.append(a)
    m = measure(args[0], alone, run)
    print(json.dumps(m, indent=1, sort_keys=True))
    if section:
        sys.exit(1 if check(m, section) else 0)


if __name__ == "__main__":
    main(sys.argv[1:])
