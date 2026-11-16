"""Measures of a monastery build, for expect.toml (R13, R15, R16, R18).

    python3 examples/monastery/measure.py BUILD_DIR [--base BASE_DIR] [--check SECTION]

Prints the measures as JSON. With --check, compares them against expect.toml's rows under SECTION
(`base`, `guesthouse`, `move_bridge`, `no_bridge`, `winter`) and exits 1 when one falls outside its
band (Python 3.11+, for tomllib). With --base, also what changed against the base build. Reads only
the build directory (spec/build.md): features.json, instances/names.txt, diagnostics.json and
fields/height.npy and water.npy (little-endian float32, rows by y, posts every 1 m: world.toml's
resolution). Standard library only.

Protocol of each measure:

- **Road grade** (`road_*`, R16). The road's course as built (features.json `road_up`, its points in
  metres) is walked every 1 m; at each step the ground (`height`, bilinear between posts) is read.
  `road_grade_max` is the steepest rise over run over any 10 m of the course, in percent (rise over
  the 10 m measured along the course); `road_over_10` the metres of course whose 10 m window starting
  there climbs over 10 %. `road_length`, `road_rise` in metres; `road_turns` the places the course
  turns back on itself (the heading 15 m ahead differs from the heading 15 m behind by more than
  120 degrees), counted once per run of such samples. `road_self_near` counts the pairs of 1 m
  stations 40 m or more apart along the course that lie closer than its `width` (attrs, 4 m) plus
  2 m: two legs of one switchback running on top of each other (the walk of slice 4 found the first
  two legs 0.27 m apart at (256, 20), 97 such pairs).
- **Deck** (R18). `decks`, features of kind `crossing`; `deck_wet` the deck's posts (within half its
  `width` of its line, between its ends) where the water stands over the ground by more than 1 mm.
- **Wall and gate** (lib/town `enclosure`; since M5 slice 3 the wall is lib/town kit.py's pieces
  along `court_wall`, not the terrain). The court's level is the ground at the well. Walking the
  court's outline every 1 m, 0.25 m inside it: `wall_low` counts samples farther than 3 m from the
  gate where the ground stands less than 3.5 m above the court and no wall piece covers the point
  (its plan bounds from the pack's facts, lib/packs/starter.facts.toml, times its scale, turned by
  its face; instances/x, y, face, scale.npy, the asset from manifest.json's chunks); `wall_members`
  counts the pieces.
  `gate_open_m` is the longest stretch of the outline (sampled every 0.1 m within 6 m of the gate)
  that no member covers: the gap a walker passes. `gate_step` is the largest difference, in
  centimetres, between the court's level and the ground across the gate (from 1.5 m outside the
  outline to 1.5 m inside it, on the gate's normal).
- **Gorge faces** (M5 slice 3; lib/land `sheer_cliff`, layers/gorge.toml). Quad (j, i) is the square
  from post (i, j), cut when fields/height.cut.npy holds 1 there (build.md 6.5); its span is its
  four posts' highest less lowest, in metres. `gorge_step_max_m` is the largest span of a quad not
  cut whose centre lies within 20 m of the stream's line (features.json), away from the bridge (20
  m of its middle) and the tarn's water: the stair-step the terrain still draws there;
  `gorge_steep` counts those quads with a span over 2 m. `faces` counts features of kind
  `sheer_cliff`; `face_cut` the cut quads whose centre lies within 0.75 m (the faces' `lip`) of a
  face's line; `face_gap_max_mm` the largest |gap| over meets.json rows between two faces.
- **Reach** (lib/review reach): `reach_held`, `reach_pairs` from `step.reach.walked`; `unreached`,
  `not_sealed`, `stranded` the counts of those records.
- **Against a base**: `instances_added`, `instances_removed` by name; `road_kept` the number of the
  road's course points, from its start, equal to the base's (to the millimetre) before the first
  that differs, and `road_points` its point count.
"""

import json
import math
import struct
import sys
from collections import Counter
from pathlib import Path


def grid(d, name):
    raw = (Path(d) / "fields" / f"{name}.npy").read_bytes()
    hlen = struct.unpack("<H", raw[8:10])[0]
    head = raw[10:10 + hlen].decode()
    assert "'<f4'" in head and "'fortran_order': False" in head, head
    shape = tuple(int(v) for v in head.split("'shape': (")[1].split(")")[0].split(",") if v.strip())
    return shape, struct.unpack(f"<{shape[0] * shape[1]}f", raw[10 + hlen:])


def sampler(shape, v):
    ny, nx = shape

    def at(x, y):
        x = min(max(x, 0.0), nx - 1.0)
        y = min(max(y, 0.0), ny - 1.0)
        i, j = min(int(x), nx - 2), min(int(y), ny - 2)
        fx, fy = x - i, y - j
        k = j * nx + i
        return ((v[k] * (1 - fx) + v[k + 1] * fx) * (1 - fy)
                + (v[k + nx] * (1 - fx) + v[k + nx + 1] * fx) * fy)
    return at


def walk(points, step=1.0):
    """Points every `step` metres along a polyline, with the heading at each."""
    out = []
    carry = 0.0
    for (x0, y0), (x1, y1) in zip(points, points[1:]):
        seg = math.hypot(x1 - x0, y1 - y0)
        if seg == 0:
            continue
        s = carry
        while s <= seg:
            out.append((x0 + (x1 - x0) * s / seg, y0 + (y1 - y0) * s / seg, math.atan2(y1 - y0, x1 - x0)))
            s += step
        carry = s - seg
    return out


def road(fs, h):
    by = {f["name"]: f for f in fs}
    f = by.get("road_up")
    if not f:
        return {}
    pts = [p[:2] for p in f["points"]]
    s = walk(pts)
    z = [h(x, y) for x, y, _ in s]
    win = 10
    grades = [abs(z[i + win] - z[i]) / win * 100 for i in range(len(z) - win)]
    turns, inside = 0, False
    for i in range(15, len(s) - 15):
        a, b = s[i - 15][2], s[i + 15][2]
        dt = abs((b - a + math.pi) % (2 * math.pi) - math.pi)
        back = dt > math.radians(120)
        turns += back and not inside
        inside = back
    return {
        "road_length": round(len(s) - 1, 1),
        "road_rise": round(z[-1] - z[0], 2),
        "road_grade_max": round(max(grades), 2),
        "road_over_10": sum(1 for g in grades if g > 10),
        "road_turns": turns,
        "road_self_near": self_near(s, float(f.get("attrs", {}).get("width", 4)) + 2),
        "road_points": len(pts),
    }


def self_near(s, near, apart=40):
    """Pairs of stations at least `apart` stations along the course and closer than `near` m."""
    return sum(1 for i in range(len(s)) for j in range(i + apart, len(s))
               if math.hypot(s[j][0] - s[i][0], s[j][1] - s[i][1]) < near)


def deck(fs, h, w):
    m = {"decks": sum(1 for f in fs if f.get("kind") == "crossing")}
    b = next((f for f in fs if f.get("kind") == "crossing"), None)
    if not b:
        return m
    (x0, y0), (x1, y1) = b["points"][0][:2], b["points"][-1][:2]
    length = math.hypot(x1 - x0, y1 - y0)
    ux, uy = (x1 - x0) / length, (y1 - y0) / length
    half = float(b.get("attrs", {}).get("width", 6)) / 2
    posts = wet = 0
    for j in range(int(min(y0, y1) - half) - 1, int(max(y0, y1) + half) + 2):
        for i in range(int(min(x0, x1) - half) - 1, int(max(x0, x1) + half) + 2):
            px, py = i - x0, j - y0
            along, across = px * ux + py * uy, abs(py * ux - px * uy)
            if 0 <= along <= length and across <= half:
                posts += 1
                wet += w(i, j) - h(i, j) > 0.001
    m["deck_posts"], m["deck_wet"] = posts, wet
    return m


def npy(path):
    raw = Path(path).read_bytes()
    hlen = struct.unpack("<H", raw[8:10])[0]
    head = raw[10:10 + hlen].decode()
    code = {"<i4": "i", "<f4": "f", "<f8": "d", "|u1": "B"}[head.split("'descr': '")[1].split("'")[0]]
    body = raw[10 + hlen:]
    return struct.unpack(f"<{len(body) // struct.calcsize(code)}{code}", body)


def wall_bays(d, names):
    """Each piece of the wall (`wall#court_wall...`, lib/town kit.py) as (x, y, front, right, x0, x1,
    y0, y1): its plan bounds from the pack's facts (lib/packs/starter.facts.toml `bounds`) times its
    scale, its asset from manifest.json's chunks."""
    import tomllib
    ins = Path(d) / "instances"
    xs, ys, fa, sc = (npy(ins / f"{k}.npy") for k in ("x", "y", "face", "scale"))
    asset = {}
    for c in json.loads((Path(d) / "manifest.json").read_text()).get("chunks", []):
        for a, (start, count) in c.get("instances", {}).items():
            for k in range(start, start + count):
                asset[k] = a
    facts = tomllib.loads((Path(__file__).resolve().parents[2] / "lib/packs/starter.facts.toml").read_text())["asset"]
    out = []
    for k, n in enumerate(names):
        if n.startswith(("wall#court_wall", "court_wall#")):
            b = math.radians(fa[k] / 1000)
            bx = facts[asset[k]]["bounds"]
            out.append((xs[k] / 1000, ys[k] / 1000, (math.sin(b), math.cos(b)), (math.cos(b), -math.sin(b)),
                        bx[0] * sc[k], bx[3] * sc[k], bx[1] * sc[k], bx[4] * sc[k]))
    return out


def covered(bays, x, y):
    for cx, cy, fr, rt, x0, x1, y0, y1 in bays:
        dx, dy = x - cx, y - cy
        u, v = dx * rt[0] + dy * rt[1], dx * fr[0] + dy * fr[1]
        if x0 - 1e-6 <= u <= x1 + 1e-6 and y0 - 1e-6 <= v <= y1 + 1e-6:
            return True
    return False


def wall(fs, h, d, names):
    by = {f["name"]: f for f in fs}
    court, gate, well = by.get("court"), by.get("gate"), by.get("well")
    if not (court and gate and well):
        return {}
    bays = wall_bays(d, names)
    level = h(*well["at"][:2])
    ring = [p[:2] for p in court["pieces"][0]["outer"]]
    if ring[0] != ring[-1]:
        ring.append(ring[0])
    gx, gy = gate["at"][:2]
    low = 0
    # inward is to the left of travel on an outer ring written counter-clockwise; test both sides
    # by which one stands inside the court's bounds
    xs, ys = [p[0] for p in ring], [p[1] for p in ring]
    cx, cy = sum(xs) / len(xs), sum(ys) / len(ys)
    normal = None
    for x, y, a in walk(ring):
        nx_, ny_ = -math.sin(a), math.cos(a)
        if (cx - x) * nx_ + (cy - y) * ny_ < 0:
            nx_, ny_ = -nx_, -ny_
        if math.hypot(x - gx, y - gy) < 0.5:
            normal = (nx_, ny_)
        px, py = x + 0.25 * nx_, y + 0.25 * ny_
        if math.hypot(x - gx, y - gy) > 3 and h(px, py) - level < 3.5 and not covered(bays, px, py):
            low += 1
    nx_, ny_ = normal or (-1.0, 0.0)
    step = max(abs(h(gx + k * 0.5 * nx_, gy + k * 0.5 * ny_) - level) for k in range(-3, 4))
    run = best = 0.0
    for x, y, a in walk(ring, 0.1):
        if math.hypot(x - gx, y - gy) > 6:
            run = 0.0
            continue
        nx_, ny_ = -math.sin(a), math.cos(a)
        if (cx - x) * nx_ + (cy - y) * ny_ < 0:
            nx_, ny_ = -nx_, -ny_
        run = 0.0 if covered(bays, x + 0.25 * nx_, y + 0.25 * ny_) else run + 0.1
        best = max(best, run)
    return {"wall_low": low, "gate_step": round(step * 100), "court_level": round(level, 2),
            "wall_members": len(bays), "gate_open_m": round(best, 1)}


def seg_dist(p, pts):
    best = math.inf
    for a, b in zip(pts, pts[1:]):
        ax, ay = b[0] - a[0], b[1] - a[1]
        L2 = ax * ax + ay * ay or 1e-12
        u = min(max(((p[0] - a[0]) * ax + (p[1] - a[1]) * ay) / L2, 0.0), 1.0)
        best = min(best, math.hypot(p[0] - a[0] - u * ax, p[1] - a[1] - u * ay))
    return best


def faces(d, fs, shape, hv, wv):
    d = Path(d)
    ny, nx = shape
    cutp = d / "fields" / "height.cut.npy"
    cut = npy(cutp) if cutp.exists() else None
    by = {f["name"]: f for f in fs}
    lines = [[p[:2] for p in f["points"]] for f in fs if f.get("kind") == "sheer_cliff"]
    stream = [p[:2] for p in by["stream"]["points"]]
    br = by.get("bridge")
    mid = None
    if br:
        a, b = br["points"][0], br["points"][-1]
        mid = ((a[0] + b[0]) / 2, (a[1] + b[1]) / 2)
    worst, steep, face_cut = 0.0, 0, 0
    for j in range(ny - 1):
        for i in range(nx - 1):
            c = (i + 0.5, j + 0.5)
            is_cut = cut is not None and cut[j * (nx - 1) + i] == 1
            if is_cut and lines and min(seg_dist(c, ln) for ln in lines) <= 0.75:
                face_cut += 1
            if is_cut or abs(c[0] - 200) > 40:
                continue
            if seg_dist(c, stream) > 20 or (mid and math.dist(c, mid) < 20):
                continue
            ks = (j * nx + i, j * nx + i + 1, (j + 1) * nx + i, (j + 1) * nx + i + 1)
            if any(wv[k] - hv[k] > 0.001 for k in ks):
                continue
            span = max(hv[k] for k in ks) - min(hv[k] for k in ks)
            worst = max(worst, span)
            steep += span > 2
    gap = 0
    mp = d / "meets.json"
    if mp.exists():
        for r in json.loads(mp.read_text()).get("meets", []):
            if r["a"].startswith("gorge_") and r["b"].startswith("gorge_"):
                gap = max(gap, abs(r["gap"]))
    return {"gorge_step_max_m": round(worst, 2), "gorge_steep": steep, "faces": len(lines),
            "face_cut": face_cut, "face_gap_max_mm": gap}


def measures(d):
    d = Path(d)
    fs = json.loads((d / "features.json").read_text())["features"]
    names = [n for n in (d / "instances/names.txt").read_text().splitlines() if n]
    diags = json.loads((d / "diagnostics.json").read_text())["diagnostics"]
    shape, hv = grid(d, "height")
    _, wv = grid(d, "water")
    h, w = sampler(shape, hv), sampler(shape, wv)
    codes = Counter(x["code"] for x in diags)
    m = {"instances": len(names)}
    for x in diags:
        if x["code"] == "step.reach.walked":
            m["reach_held"], m["reach_pairs"] = x["data"].get("held"), x["data"].get("pairs")
    for c in ("unreached", "not_sealed", "stranded"):
        m[c] = codes.get("step.reach." + c, 0)
    for c in ("geom.unowned_crossing", "place.relief", "place.keep_out", "authority.conflict",
              "authority.max_change"):
        m[c] = codes.get(c, 0)
    m["lakes"] = sum(1 for f in fs if f.get("kind") == "lake")
    m.update(road(fs, h))
    m.update(deck(fs, h, w))
    m.update(wall(fs, h, d, names))
    m.update(faces(d, fs, shape, hv, wv))
    m.update(meetings(d))
    m["errors"] = sum(1 for x in diags if x["severity"] == "error")
    m["warnings"] = sum(1 for x in diags if x["severity"] == "warning")
    return m, fs, names


def main(argv):
    m, fs, names = measures(argv[0])
    if "--base" in argv:
        mb, fb, nb = measures(argv[argv.index("--base") + 1])
        m["instances_added"] = len(set(names) - set(nb))
        m["instances_removed"] = len(set(nb) - set(names))
        a = next((f["points"] for f in fs if f["name"] == "road_up"), [])
        b = next((f["points"] for f in fb if f["name"] == "road_up"), [])
        kept = 0
        for p, q in zip(a, b):
            if [round(v, 3) for v in p[:2]] != [round(v, 3) for v in q[:2]]:
                break
            kept += 1
        m["road_kept"] = kept
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
