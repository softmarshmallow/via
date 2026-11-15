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
    for iid, ad in (alone or {}).items():
        m[f"alone_off_{iid}"] = alone_off(d, fs, iid, Path(ad))
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
