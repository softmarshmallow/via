"""Measures of a townland build, for expect.toml (R13, R15, R18).

    python3 examples/townland/measure.py BUILD_DIR [--base BASE_DIR] [--check SECTION]

Prints the measures as JSON. With --check, compares them against expect.toml's rows under SECTION
(`base`, `move_hill`, `move_crossing`, `edit_west`) and exits 1 when one falls outside its band
(Python 3.11+, for tomllib). With --base, also what changed against the base build, by name:
blocks, lots and houses added and removed, and of those the ones on the east bank (a centre east of
x = 190 m: the old town and the upper town, far from `edit_west`). Reads only the build directory
(spec/build.md): features.json, instances/names.txt and x.npy, diagnostics.json. Standard library only.

`deck_wet` counts the posts of the crossing's deck (within `width / 2`, 3 m, of its line, between its
ends) where the water field stands above the height field (depth over 1 mm), of `deck_posts`;
`ends_on_embankment` counts street ends within 5 cm of the embankment's line (fields/*.npy, read
as little-endian float32, rows by y, at the world's resolution from manifest.json).

The street-form and deck numbers are lib/review/form.py's records (`step.form.*`, data.value);
`houses`, `lots_held` the houses step's; a block is a feature of kind `block`, a lot of kind `lot`.
"""

import json
import math
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


def grid(d, name):
    raw = (Path(d) / "fields" / f"{name}.npy").read_bytes()
    hlen = struct.unpack("<H", raw[8:10])[0]
    head = raw[10:10 + hlen].decode()
    assert "'<f4'" in head and "'fortran_order': False" in head, head
    shape = tuple(int(v) for v in head.split("'shape': (")[1].split(")")[0].split(",") if v.strip())
    return shape, struct.unpack(f"<{shape[0] * shape[1]}f", raw[10 + hlen:])


def seg_dist(p, a, b):
    ax, ay = b[0] - a[0], b[1] - a[1]
    t = max(0.0, min(1.0, ((p[0] - a[0]) * ax + (p[1] - a[1]) * ay) / (ax * ax + ay * ay)))
    return ((p[0] - a[0] - t * ax) ** 2 + (p[1] - a[1] - t * ay) ** 2) ** 0.5


def decks_and_ends(d, fs):
    by = {f["name"]: f for f in fs}
    m = {}
    bridge = by.get("old_bridge")
    if bridge:
        (shape, h), (_, w) = grid(d, "height"), grid(d, "water")
        r = 1.0                                   # world.toml's resolution; posts at (i * r, j * r)
        (x0, y0), (x1, y1) = bridge["points"][0][:2], bridge["points"][-1][:2]
        length = ((x1 - x0) ** 2 + (y1 - y0) ** 2) ** 0.5
        ux, uy = (x1 - x0) / length, (y1 - y0) / length
        half = float(bridge.get("attrs", {}).get("width", 6)) / 2
        posts = wet = 0
        for j in range(int(min(y0, y1) - half) - 1, int(max(y0, y1) + half) + 2):
            for i in range(int(min(x0, x1) - half) - 1, int(max(x0, x1) + half) + 2):
                px, py = i * r - x0, j * r - y0
                along, across = px * ux + py * uy, abs(py * ux - px * uy)
                if 0 <= along <= length and across <= half:
                    posts += 1
                    k = j * shape[1] + i
                    wet += w[k] - h[k] > 0.001
        m["deck_posts"], m["deck_wet"] = posts, wet
    quay = by.get("embankment")
    if quay:
        line = [p[:2] for p in quay["points"]]
        m["ends_on_embankment"] = sum(
            1 for f in fs if f.get("kind") == "street"
            for e in (f["points"][0], f["points"][-1])
            if min(seg_dist(e, a, b) for a, b in zip(line, line[1:])) <= 0.05)
    return m


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
    m.update(decks_and_ends(d, fs))
    m.update(meetings(d))
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


# ---------------------------------------------------------------- meetings: ends, heads, faces (M3 slice 5)
#
# How each is taken (ours, M3 slice 5; the same block in quay, townland, example and monastery):
# heights are fields/height.npy and fields/water.npy read bilinearly between posts at the world's
# resolution from its extent's lower corner (manifest.json `frame.resolution`, `frame.extent`). A way is a path of kind main_street, street, lane,
# road or trail; its end meets a crossing when it lies within 0.5 m of the crossing's first or last
# point, and a waterfront when within 0.5 m of its line. From each such end the way's own centreline
# is walked back 20 m, a sample every 0.5 m:
# - `head_grade` / `way_end_grade`: the steepest metre (|h(s + 1) - h(s)|, m per m) of those 20 m,
#   the largest over the ways that end on a crossing / on a waterfront;
# - `head_drop2` / `way_end_drop3`: |h(2) - h(0)| / |h(3) - h(0)| in metres, the largest.
# For each crossing with `crosses`, along the crossed water's line from where the deck's middle
# projects onto it: `face_pool` the water's depth (water - height) 1 m off each face of the deck
# less its depth 15 m off that face, the larger side, in metres (0 or less: no pool).
# For each waterfront end that meets no crossing end (none within 1 m), the posts within 12 m of
# the end and past it along its end segment: `front_end_slope` the steepest (degrees, central
# differences over two posts), `front_end_pit` how far the lowest dry post lies below the lowest
# water level among them (m, 0 if none dry or none wet); `front_gap` the least distance from a
# waterfront's end to a crossing's end (m), when the world has both.

def _mt_field(d, name):
    raw = (Path(d) / "fields" / f"{name}.npy").read_bytes()
    hlen = struct.unpack("<H", raw[8:10])[0]
    head = raw[10:10 + hlen].decode()
    ny, nx = (int(v) for v in head.split("'shape': (")[1].split(")")[0].split(",") if v.strip())
    return nx, ny, struct.unpack(f"<{nx * ny}f", raw[10 + hlen:])


def _mt_at(g, r, x, y):
    nx, ny, v = g
    fx, fy = x / r, y / r
    i, j = min(max(int(fx), 0), nx - 2), min(max(int(fy), 0), ny - 2)
    a, b = fx - i, fy - j
    return ((1 - a) * (1 - b) * v[j * nx + i] + a * (1 - b) * v[j * nx + i + 1]
            + (1 - a) * b * v[(j + 1) * nx + i] + a * b * v[(j + 1) * nx + i + 1])


def _mt_walk(pts, s):
    for a, b in zip(pts, pts[1:]):
        n = math.dist(a, b)
        if s <= n:
            return a[0] + (b[0] - a[0]) * s / n, a[1] + (b[1] - a[1]) * s / n
        s -= n
    return pts[-1]


def _mt_near(p, pts):
    """(distance, arc length) of the nearest point of polyline pts to p."""
    best, acc = (1e18, 0.0), 0.0
    for a, b in zip(pts, pts[1:]):
        n = math.dist(a, b)
        t = max(0.0, min(1.0, ((p[0] - a[0]) * (b[0] - a[0]) + (p[1] - a[1]) * (b[1] - a[1])) / (n * n)))
        e = math.dist(p, (a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1])))
        best = min(best, (e, acc + t * n))
        acc += n
    return best


def meetings(d):
    d = Path(d)
    fr = json.loads((d / "manifest.json").read_text())["frame"]
    r, x0, y0 = fr["resolution"], fr["extent"][0], fr["extent"][1]
    fs = json.loads((d / "features.json").read_text())["features"]
    by = {f["name"]: f for f in fs}
    h, w = _mt_field(d, "height"), _mt_field(d, "water")
    at = lambda g, p: _mt_at(g, r, p[0] - x0, p[1] - y0)
    heads = [f["points"][k][:2] for f in fs if f.get("kind") == "crossing" for k in (0, -1)]
    fronts = [[p[:2] for p in f["points"]] for f in fs if f.get("kind") == "waterfront"]
    m = {}
    for f in fs:
        if f.get("kind") not in ("main_street", "street", "lane", "road", "trail") or f.get("shape") != "path":
            continue
        pts = [p[:2] for p in f["points"]]
        for line in (pts, pts[::-1]):
            if any(math.dist(line[0], e) <= 0.5 for e in heads):
                key, drop = "head", 4
            elif any(_mt_near(line[0], fr)[0] <= 0.5 for fr in fronts):
                key, drop = "way_end", 6
            else:
                continue
            prof = [at(h, _mt_walk(line, 0.5 * k)) for k in range(41)]
            m[key + "_grade"] = round(max(m.get(key + "_grade", 0), max(abs(prof[k + 2] - prof[k]) for k in range(39))), 2)
            name = key + ("_drop2" if drop == 4 else "_drop3")
            m[name] = round(max(m.get(name, 0), abs(prof[drop] - prof[0])), 2)
    for c in fs:
        cr = (c.get("attrs") or {}).get("crosses")
        water = by.get(cr.get("name") if isinstance(cr, dict) else cr) if c.get("kind") == "crossing" else None
        if not water or water.get("shape") != "path":
            continue
        a, b = c["points"][0][:2], c["points"][-1][:2]
        line = [p[:2] for p in water["points"]]
        sx = _mt_near(((a[0] + b[0]) / 2, (a[1] + b[1]) / 2), line)[1]
        half = float(c["attrs"].get("width", 6)) / 2
        depth = lambda s: at(w, _mt_walk(line, s)) - at(h, _mt_walk(line, s))
        pool = max(depth(sx + k * (half + 1)) - depth(sx + k * (half + 15)) for k in (-1, 1))
        m["face_pool"] = round(max(m.get("face_pool", -9), pool), 2)
    for pts in fronts:
        for p, q in ((pts[0], pts[1]), (pts[-1], pts[-2])):
            if heads:
                m["front_gap"] = round(min(m.get("front_gap", 1e9), min(math.dist(p, e) for e in heads)), 1)
            if any(math.dist(p, e) <= 1 for e in heads):
                continue
            n = math.dist(p, q)
            ux, uy = (p[0] - q[0]) / n, (p[1] - q[1]) / n
            slope, dry, wet = 0.0, [], []
            nx, ny, hv = h
            for j in range(max(1, int((p[1] - y0 - 12) / r)), min(ny - 1, int((p[1] - y0 + 12) / r) + 1)):
                for i in range(max(1, int((p[0] - x0 - 12) / r)), min(nx - 1, int((p[0] - x0 + 12) / r) + 1)):
                    x, y = x0 + i * r, y0 + j * r
                    if math.hypot(x - p[0], y - p[1]) > 12 or (x - p[0]) * ux + (y - p[1]) * uy < 0:
                        continue
                    gx = (hv[j * nx + i + 1] - hv[j * nx + i - 1]) / (2 * r)
                    gy = (hv[(j + 1) * nx + i] - hv[(j - 1) * nx + i]) / (2 * r)
                    slope = max(slope, math.degrees(math.atan(math.hypot(gx, gy))))
                    k = j * nx + i
                    (wet if w[2][k] - hv[k] > 0.001 else dry).append((hv[k], w[2][k]))
            pit = max(0.0, min(v for _, v in wet) - min(g for g, _ in dry)) if wet and dry else 0.0
            m["front_end_slope"] = round(max(m.get("front_end_slope", 0), slope), 1)
            m["front_end_pit"] = round(max(m.get("front_end_pit", 0), pit), 2)
    return m


if __name__ == "__main__":
    main(sys.argv[1:])
