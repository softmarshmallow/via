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
Dead ends are lib/review/form.py's (`step.form.*` records, data.value; the world declares the lint):
`dead_end_count` the ends on nothing inside the town, `edge_ends` those on the district's outline
or at the gate, apart (form.py, "Dead end and edge end"); `geom.dead_end` stays the number of the
core's records and `geom.dead_end_count` the count they carry.

Doors (quay-walk, 2026-11-15; ours): for each house (`houses#...`) whose asset has a `door` mount,
its floor is the ground at its origin plus its `z`, and its door the mount's point turned by the
instance's `face`; the ground is read 3 m out from the door along the facing direction, on the
footway or the quayside, bilinear between the 0.5 m posts of `fields/height.npy`. `doors_level`
counts doors whose floor lies within 0.3 m of that ground, `doors_off` those more than 1 m off it,
`doors` all of them; `doors_off_median` is the median of |floor - ground| in centimetres.
"""

import json
import math
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
    """A 1-D .npy of millimetres (or millidegrees), int32 or float32, in metres (or degrees)."""
    raw = path.read_bytes()
    hlen = struct.unpack("<H", raw[8:10])[0]
    code = "f" if "<f4" in raw[10:10 + hlen].decode() else "i"
    body = raw[10 + hlen:]
    return [v / 1000.0 for v in struct.unpack(f"<{len(body) // 4}{code}", body)]


def _f32_grid(path):
    """A 2-D little-endian float32 .npy: (rows, columns, flat values)."""
    raw = path.read_bytes()
    hlen = struct.unpack("<H", raw[8:10])[0]
    head = raw[10:10 + hlen].decode()
    rows, cols = (int(v) for v in head.split("'shape': (")[1].split(")")[0].split(",")[:2])
    body = raw[10 + hlen:]
    return rows, cols, struct.unpack(f"<{len(body) // 4}f", body)


DOOR_OUT = 3.0                  # metres out from the door the ground is read (the footway)
DOOR_LEVEL = 0.3                # a door this near that ground is level with it
DOOR_OFF = 1.0                  # one farther off stands over a bank


def doors(d):
    """(level, off, count, median |floor - ground| in cm) of the houses' doors (module doc)."""
    d = Path(d)
    man = json.loads((d / "manifest.json").read_text())
    res = man["frame"]["resolution"]
    ox, oy = man["frame"]["extent"][:2]
    rows, cols, h = _f32_grid(d / "fields/height.npy")
    names = [n for n in (d / "instances/names.txt").read_text().splitlines() if n]
    x, y, z, face = (_i32(d / f"instances/{k}.npy") for k in ("x", "y", "z", "face"))
    asset = [None] * len(names)
    for ch in man["chunks"]:
        for a, (s, n) in ch.get("instances", {}).items():
            for k in range(s, s + n):
                asset[k] = a

    def ground(px, py):
        i = min(max((px - ox) / res, 0.0), cols - 1.000001)
        j = min(max((py - oy) / res, 0.0), rows - 1.000001)
        i0, j0 = int(i), int(j)
        u, v = i - i0, j - j0
        at = lambda a, b: h[b * cols + a]
        return ((at(i0, j0) * (1 - u) + at(i0 + 1, j0) * u) * (1 - v)
                + (at(i0, j0 + 1) * (1 - u) + at(i0 + 1, j0 + 1) * u) * v)

    off = []
    for k, n in enumerate(names):
        door = ((man["assets"].get(asset[k]) or {}).get("mounts") or {}).get("door")
        if not n.startswith("houses#") or door is None:
            continue
        m = math.radians(face[k])
        s, c = math.sin(m), math.cos(m)
        dx = x[k] + door["at"][0] * c + door["at"][1] * s
        dy = y[k] - door["at"][0] * s + door["at"][1] * c
        floor = ground(x[k], y[k]) + z[k]
        off.append(abs(floor - ground(dx + DOOR_OUT * s, dy + DOOR_OUT * c)))
    off.sort()
    med = round(100 * off[len(off) // 2]) if off else None
    return (sum(1 for v in off if v <= DOOR_LEVEL), sum(1 for v in off if v > DOOR_OFF),
            len(off), med)


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
                 "identity.pin_removed", "step.streets.none"):
        m[code] = codes.get(code, 0)
    # `identity.reselected` is a run diagnostic, never in diagnostics.json: it is measured only from
    # --run, and without it stays None, so a row on it fails rather than reading 0.
    m["errors"] = sum(1 for x in diags if x["severity"] == "error")
    m["doors_level"], m["doors_off"], m["doors"], m["doors_off_median"] = doors(d)
    for x in diags:                                   # lib/review/form.py's records (step.form.*)
        data = x.get("data") or {}
        if x["code"].startswith("step.form.") and "value" in data:
            m[x["code"][len("step.form."):]] = data["value"]
    m["geom.dead_end_count"] = sum((x.get("data") or {}).get("count", 0) for x in diags
                                   if x["code"] == "geom.dead_end")
    m.update(meetings(d))
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
