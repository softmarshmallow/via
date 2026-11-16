"""Measures of a border build, for expect.toml (docs/plan.md M4; composition.md 6, R10).

    python3 examples/border/measure.py BUILD [--alone wild=DIR] [--check SECTION]

Prints the measures as JSON. With --check, compares them against expect.toml's rows under SECTION
(`base`, `move_abbey`, `shut`) and exits 1 when one falls outside its band or names a measure not
taken (Python 3.11+, for tomllib). Reads only build directories (spec/build.md): features.json,
instances/names.txt, diagnostics.json, fields/*.npy. Standard library only.

How each is taken (ours):
- `errors`, `seam` (import.seam records), `unanswered` (import.exit_unanswered), `blended`
  (import.place_blended), `unowned` (geom.unowned_crossing).
- `unreached`, `not_sealed`: lib/review reach records; `held`: `data.held` of its `walked` record.
- `crest`: the bank's least height over the ground beside it: along each bank feature, every 2 m,
  skipping the 10 m at each end, points inside the extent, and points within 16 m of a walk, the
  height there less the mean of the heights 14 m either side across the line; the least, metres.
- `opening`: the same at each walk's crossing of its bank (the largest of the two): about 0 open.
- `crossing_<walk>`: the [x, y] where the walk crosses its bank.
- `gate`: abbey.gate's plan point; `gate_off`: metres from south_walk's first point to it;
  `fork_off`: metres from the fork to the south walk's line.
- `wild_trees`, `fringe_trees`: instances named `wild.trees#*` and `fringe_trees#*`.
- `spread_<m>`: the join step's `fringe#spread` at the south walk's crossing moved m metres
  north-west... read at (crossing x - 40, crossing y + m), on the bare ground.
- `alone_off_wild`: with --alone, the largest |parent height - wild alone height| over the posts
  where the wild's weight is 1 (inside its footprint, `off_corner` 16 m or more); metres.
"""

import json
import math
import struct
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
WILD = [(0, 0), (640, 0), (640, 220), (250, 220), (250, 560), (0, 560)]


def grid(path):
    raw = Path(path).read_bytes()
    hlen = struct.unpack("<H", raw[8:10])[0]
    head = raw[10:10 + hlen].decode()
    rows, cols = (int(v) for v in head.split("'shape': (")[1].split(")")[0].split(",")[:2])
    body = raw[10 + hlen:]
    return rows, cols, struct.unpack(f"<{len(body) // 4}f", body)


def post(g, x, y):
    rows, cols, v = g
    i = min(max(round(x), 0), cols - 1)
    j = min(max(round(y), 0), rows - 1)
    return v[j * cols + i]


def inside(poly, x, y):
    c = False
    for (x1, y1), (x2, y2) in zip(poly, poly[1:] + poly[:1]):
        if (y1 > y) != (y2 > y) and x < x1 + (y - y1) * (x2 - x1) / (y2 - y1):
            c = not c
    return c


def samples(pts, step=2.0):
    """(x, y, nx, ny, along) every `step` metres along a polyline; n the unit normal."""
    out, run = [], 0.0
    for (x1, y1), (x2, y2) in zip(pts, pts[1:]):
        L = math.hypot(x2 - x1, y2 - y1)
        if L == 0:
            continue
        k = 0.0
        while k < L:
            out.append((x1 + (x2 - x1) * k / L, y1 + (y2 - y1) * k / L, -(y2 - y1) / L, (x2 - x1) / L, run + k))
            k += step
        run += L
    return out, run


def seg_dist(px, py, a, b):
    (x1, y1), (x2, y2) = a, b
    L2 = (x2 - x1) ** 2 + (y2 - y1) ** 2
    t = 0 if L2 == 0 else max(0, min(1, ((px - x1) * (x2 - x1) + (py - y1) * (y2 - y1)) / L2))
    return math.hypot(px - x1 - t * (x2 - x1), py - y1 - t * (y2 - y1))


def line_dist(px, py, pts):
    return min(seg_dist(px, py, a, b) for a, b in zip(pts, pts[1:]))


def cross(p, q):
    """The first crossing of polylines p and q, or None."""
    for a, b in zip(p, p[1:]):
        for c, d in zip(q, q[1:]):
            den = (b[0] - a[0]) * (d[1] - c[1]) - (b[1] - a[1]) * (d[0] - c[0])
            if den == 0:
                continue
            t = ((c[0] - a[0]) * (d[1] - c[1]) - (c[1] - a[1]) * (d[0] - c[0])) / den
            u = ((c[0] - a[0]) * (b[1] - a[1]) - (c[1] - a[1]) * (b[0] - a[0])) / den
            if 0 <= t <= 1 and 0 <= u <= 1:
                return (a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1]))
    return None


def measure(d, alone=None):
    d = Path(d)
    diags = json.loads((d / "diagnostics.json").read_text())["diagnostics"]
    codes = [x["code"] for x in diags]
    m = {
        "errors": sum(1 for x in diags if x["severity"] == "error"),
        "seam": codes.count("import.seam"),
        "unanswered": codes.count("import.exit_unanswered"),
        "blended": codes.count("import.place_blended"),
        "unowned": codes.count("geom.unowned_crossing"),
        "unreached": codes.count("step.reach.unreached"),
        "not_sealed": codes.count("step.reach.not_sealed"),
        "held": sum(x["data"].get("held", 0) for x in diags if x["code"] == "step.reach.walked"),
    }
    if not (d / "features.json").exists():
        return m
    fs = {f["name"]: f for f in json.loads((d / "features.json").read_text())["features"]}
    h = grid(d / "fields" / "height.npy")
    walks = {w: [tuple(p[:2]) for p in fs[w]["points"]] for w in ("south_walk", "west_walk")}
    crest, opening = math.inf, -math.inf
    for bank, walk in (("bank_s", "south_walk"), ("bank_w", "west_walk")):
        pts = [tuple(p[:2]) for p in fs[bank]["points"]]
        ss, total = samples(pts)
        for x, y, nx, ny, a in ss:
            if a < 10 or a > total - 10 or not (0 <= x <= 640 and 0 <= y <= 560):
                continue
            if min(line_dist(x, y, w) for w in walks.values()) < 16:
                continue
            side = (post(h, x + 14 * nx, y + 14 * ny) + post(h, x - 14 * nx, y - 14 * ny)) / 2
            crest = min(crest, post(h, x, y) - side)
        c = cross(walks[walk], pts)
        m[f"crossing_{walk}"] = [round(c[0], 2), round(c[1], 2)] if c else None
        if c:
            (x1, y1), (x2, y2) = min(zip(pts, pts[1:]), key=lambda s: seg_dist(c[0], c[1], *s))
            L = math.hypot(x2 - x1, y2 - y1)
            nx, ny = -(y2 - y1) / L, (x2 - x1) / L
            side = (post(h, c[0] + 14 * nx, c[1] + 14 * ny) + post(h, c[0] - 14 * nx, c[1] - 14 * ny)) / 2
            opening = max(opening, post(h, c[0], c[1]) - side)
    m["crest"] = round(crest, 3)
    m["opening"] = round(opening, 3)
    gate = fs["abbey.gate"]["at"][:2]
    m["gate"] = gate
    m["gate_off"] = round(math.hypot(walks["south_walk"][0][0] - gate[0], walks["south_walk"][0][1] - gate[1]), 3)
    fk = fs["fork"]["at"][:2]
    m["fork_off"] = round(line_dist(fk[0], fk[1], walks["south_walk"]), 3)
    names = (d / "instances" / "names.txt").read_text().split()
    m["wild_trees"] = sum(1 for n in names if n.startswith("wild.trees#"))
    m["fringe_trees"] = sum(1 for n in names if n.startswith("fringe_trees#"))
    c = m.get("crossing_south_walk")
    sp = d / "fields" / "fringe#spread.npy"
    if c and sp.exists():
        g = grid(sp)
        for k in (0, 20, 40):
            m[f"spread_{k}"] = round(post(g, c[0] - 40, c[1] + k), 3)
    if alone and "wild" in alone:
        a = grid(Path(alone["wild"]) / "fields" / "height.npy")
        oc = grid(d / "fields" / "off_corner.npy")
        rows, cols, _ = h
        worst = 0.0
        for j in range(rows):
            for i in range(cols):
                if post(oc, i, j) >= 16 and inside(WILD, i, j):
                    worst = max(worst, abs(post(h, i, j) - post(a, i, j)))
        m["alone_off_wild"] = round(worst, 4)
    return m


def main(argv):
    args, alone, section = [], {}, None
    it = iter(argv)
    for a in it:
        if a == "--alone":
            k, v = next(it).split("=", 1)
            alone[k] = v
        elif a == "--check":
            section = next(it)
        else:
            args.append(a)
    m = measure(args[0], alone)
    print(json.dumps(m, indent=1))
    if section is None:
        return 0
    import tomllib
    rows = tomllib.loads((HERE / "expect.toml").read_text())[section]
    bad = 0
    for name, row in rows.items():
        got = m.get(row["measure"])
        band = row["band"]
        ok = got is not None and (got == band[0] if not isinstance(band[0], (int, float)) else band[0] <= got <= band[1])
        bad += not ok
        print(f"{'ok ' if ok else 'BAD'} {section}.{name}: {row['measure']} = {got} in {band}")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
