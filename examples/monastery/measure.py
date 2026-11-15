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
- **Wall and gate** (lib/town `enclosure`). The court's level is the ground at the well. Walking the
  court's outline every 1 m, 0.75 m inside it: `wall_low` counts samples farther than 3 m from the
  gate where the ground stands less than 3.5 m above the court; `gate_step` is the largest
  difference, in centimetres, between the court's level and the ground across the gate (from 1.5 m
  outside the outline to 1.5 m inside it, on the gate's normal).
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


def wall(fs, h):
    by = {f["name"]: f for f in fs}
    court, gate, well = by.get("court"), by.get("gate"), by.get("well")
    if not (court and gate and well):
        return {}
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
        if math.hypot(x - gx, y - gy) > 3 and h(x + 0.75 * nx_, y + 0.75 * ny_) - level < 3.5:
            low += 1
    nx_, ny_ = normal or (-1.0, 0.0)
    step = max(abs(h(gx + k * 0.5 * nx_, gy + k * 0.5 * ny_) - level) for k in range(-3, 4))
    return {"wall_low": low, "gate_step": round(step * 100), "court_level": round(level, 2)}


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
    m.update(wall(fs, h))
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


if __name__ == "__main__":
    main(sys.argv[1:])
