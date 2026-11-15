"""Measures of a city build, for expect.toml (R13, R15, R16, R18).

    python3 examples/city/measure.py BUILD_DIR [--base BASE_DIR] [--check SECTION]

Prints the measures as JSON. With --check, compares them against expect.toml's rows under SECTION
(`base`, `move_avenue`, `move_bridge`, `hand`) and exits 1 when one falls outside its band (Python
3.11+, for tomllib). Reads only the build directory (spec/build.md): features.json,
instances/names.txt, x.npy and y.npy, diagnostics.json, manifest.json and fields/*.npy. Standard
library only.

Counts: a ward, block or lot is a feature of that kind; `streets` the paths of kind `street`;
`houses` the instances named `houses_<district>#...`; `bookmarks` the features of kind `bookmark`. The street
form and the decks are lib/review/form.py's records (`step.form.*`, data.value; `decks`,
`doubled_decks` and `ways_over_water` are per water feature, so with one river one record each);
`reach_held` the reach lint's; `houses`, `lots_held` the houses step's.

Per crossing (every feature of kind `crossing`): `meetings_owned` counts the main ways
(`main_street`) with an end within 0.5 m of a crossing's first or last point; `crossing_ends_met`
the crossing ends at least one main way ends on, of `crossing_ends` (2 per crossing). Per
interchange (every point of kind `interchange`): `interchange_ends` the ends of main ways and
freeways within 0.5 m of it (4 at a cross), the least over the interchanges in
`interchange_ends_min`. `deck_wet` counts deck posts (within `width / 2` of a crossing's line,
between its ends) where water stands above the ground by over 1 mm, over all crossings.

With --base (a change built `--against` the base), what changed, by name and place. The moved
features are the features of the two builds whose points differ (paths, points, regions: a
feature present in one only counts as moved). A ward is *resettled* when its name is in one build
only or its outline changed; `wards_resettled` counts them and `wards_resettled_far` those whose
outline (both outlines, if both) comes no nearer than `FAR_WARD` (5 m) to any moved feature's
points or segments in either build: an edit to one main way that resettles a ward it does not
bound shows here. For blocks, lots and houses (by name): added and removed, and `_far` those
whose centre (a house: its pivot) lies more than `FAR` (60 m: three lots deep) from every moved
feature in either build: names elsewhere must not change (R13, identity.md 2.3). A block whose
outline moved renames its lots, so `_far` lots and houses can lie in a changed block that reaches
past `FAR`; `lot_changed_in_kept_blocks` and `houses_changed_in_kept_blocks` count the lots and
houses added or removed whose centre lies inside a block kept with the same name and outline in
both builds (`blocks_kept` of them): the claim is that these are 0.
"""

import json
import math
import struct
import sys
from collections import Counter
from pathlib import Path

FAR = 60.0
FAR_WARD = 5.0


def npy(path, code):
    raw = Path(path).read_bytes()
    hlen = struct.unpack("<H", raw[8:10])[0]
    body = raw[10 + hlen:]
    return struct.unpack(f"<{len(body) // 4}{code}", body)


def field(d, name):
    raw = (Path(d) / "fields" / f"{name}.npy").read_bytes()
    hlen = struct.unpack("<H", raw[8:10])[0]
    head = raw[10:10 + hlen].decode()
    assert "'<f4'" in head and "'fortran_order': False" in head, head
    ny, nx = (int(v) for v in head.split("'shape': (")[1].split(")")[0].split(",") if v.strip())
    return nx, ny, struct.unpack(f"<{nx * ny}f", raw[10 + hlen:])


def load(d):
    d = Path(d)
    fs = json.loads((d / "features.json").read_text())["features"]
    names = [n for n in (d / "instances/names.txt").read_text().splitlines() if n]
    diags = json.loads((d / "diagnostics.json").read_text())["diagnostics"]
    xs = [v / 1000.0 for v in npy(d / "instances/x.npy", "i")] if names else []
    ys = [v / 1000.0 for v in npy(d / "instances/y.npy", "i")] if names else []
    return fs, dict(zip(names, zip(xs, ys))), diags


def rings(f):
    if f.get("pieces"):
        return [[p[:2] for p in pc["outer"]] for pc in f["pieces"]]
    if f.get("points"):
        return [[p[:2] for p in f["points"]]]
    if f.get("at"):
        return [[f["at"][:2]]]
    return []


def centre(f):
    ring = rings(f)[0]
    return sum(p[0] for p in ring) / len(ring), sum(p[1] for p in ring) / len(ring)


def seg_dist(p, a, b):
    ax, ay = b[0] - a[0], b[1] - a[1]
    n = ax * ax + ay * ay
    t = 0.0 if n == 0 else max(0.0, min(1.0, ((p[0] - a[0]) * ax + (p[1] - a[1]) * ay) / n))
    return math.hypot(p[0] - a[0] - t * ax, p[1] - a[1] - t * ay)


def inside(p, ring):
    """Even-odd test of p against a closed ring."""
    c = False
    for (x0, y0), (x1, y1) in zip(ring, ring[1:] + ring[:1]):
        if (y0 > p[1]) != (y1 > p[1]) and p[0] < x0 + (p[1] - y0) * (x1 - x0) / (y1 - y0):
            c = not c
    return c


def line_dist(p, pts):
    if len(pts) == 1:
        return math.dist(p, pts[0])
    return min(seg_dist(p, a, b) for a, b in zip(pts, pts[1:]))


def skeleton(fs, m):
    ends = lambda f: (f["points"][0][:2], f["points"][-1][:2])
    ways = [f for f in fs if f.get("kind") in ("main_street", "freeway") and f.get("shape") == "path"]
    heads = [e for f in fs if f.get("kind") == "crossing" for e in ends(f)]
    met = [e for e in heads if any(math.dist(e, w) <= 0.5 for f in ways if f["kind"] == "main_street"
                                   for w in ends(f))]
    m["crossings"] = sum(1 for f in fs if f.get("kind") == "crossing")
    m["crossing_ends"], m["crossing_ends_met"] = len(heads), len(met)
    m["meetings_owned"] = sum(1 for f in ways if f["kind"] == "main_street" for w in ends(f)
                              if any(math.dist(w, e) <= 0.5 for e in heads))
    ix = [f["at"][:2] if f.get("at") else f["points"][0][:2] for f in fs if f.get("kind") == "interchange"]
    m["interchanges"] = len(ix)
    counts = [sum(1 for f in ways for w in ends(f) if math.dist(w, p) <= 0.5) for p in ix]
    m["interchange_ends_min"] = min(counts) if counts else 0


def deck_wet(d, fs, m):
    r = json.loads((Path(d) / "manifest.json").read_text())["frame"]["resolution"]
    (nx, ny, h), (_, _, w) = field(d, "height"), field(d, "water")
    posts = wet = 0
    for c in fs:
        if c.get("kind") != "crossing":
            continue
        (x0, y0), (x1, y1) = c["points"][0][:2], c["points"][-1][:2]
        length = math.dist((x0, y0), (x1, y1))
        ux, uy = (x1 - x0) / length, (y1 - y0) / length
        half = float(c.get("attrs", {}).get("width", 6)) / 2
        for j in range(int((min(y0, y1) - half) / r) - 1, int((max(y0, y1) + half) / r) + 2):
            for i in range(int((min(x0, x1) - half) / r) - 1, int((max(x0, x1) + half) / r) + 2):
                px, py = i * r - x0, j * r - y0
                along, across = px * ux + py * uy, abs(py * ux - px * uy)
                if 0 <= along <= length and across <= half and 0 <= i < nx and 0 <= j < ny:
                    posts += 1
                    wet += w[j * nx + i] - h[j * nx + i] > 0.001
    m["deck_posts"], m["deck_wet"] = posts, wet


def measures(d):
    fs, houses, diags = load(d)
    by = {"ward": {}, "block": {}, "lot": {}, "street": {}}
    for f in fs:
        if f.get("kind") in by:
            by[f["kind"]][f["name"]] = f
    m = {k: len(v) for k, v in by.items()}
    m["houses"] = sum(1 for n in houses if n.startswith("houses_"))
    m["bookmarks"] = sum(1 for f in fs if f.get("kind") == "bookmark")
    codes = Counter(x["code"] for x in diags)
    for x in diags:
        c, data = x["code"], x.get("data") or {}
        if c.startswith("step.form.") and "value" in data:
            m[c[len("step.form."):]] = data["value"]
            if c == "step.form.dead_end_count":
                m["stubs"] = data.get("stubs")
            if c == "step.form.edge_ends":
                m["edge_at_edge"], m["edge_at_gate"] = data.get("at_edge"), data.get("at_gate")
        if c.startswith("step.houses_") and c.endswith(".rows"):
            m["lots_held"] = m.get("lots_held", 0) + data.get("lots_held", 0)
            m["houses_" + c.split(".")[1].split("_", 1)[1]] = data.get("houses")
        if c == "step.reach.walked":
            m["reach_held"], m["reach_pairs"] = data.get("held"), data.get("pairs")
    for code in ("place.relief", "place.keep_out", "geom.unowned_crossing", "authority.conflict",
                 "authority.squeezed", "authority.max_change", "geom.dead_end", "field.effect_cut",
                 "step.reach.not_sealed", "step.reach.no_way"):
        m[code] = codes.get(code, 0)
    skeleton(fs, m)
    deck_wet(d, fs, m)
    m["errors"] = sum(1 for x in diags if x["severity"] == "error")
    m["warnings"] = sum(1 for x in diags if x["severity"] == "warning")
    return m, fs, by, houses


class Segs:
    """Segments bucketed in CELL-metre squares, for "is p within r of any of them"."""
    CELL = 20.0

    def __init__(self, lines):
        self.b = {}
        for pts in lines:
            segs = list(zip(pts, pts[1:])) or [(pts[0], pts[0])]
            for a, b in segs:
                for i in range(int(min(a[0], b[0]) // self.CELL), int(max(a[0], b[0]) // self.CELL) + 1):
                    for j in range(int(min(a[1], b[1]) // self.CELL), int(max(a[1], b[1]) // self.CELL) + 1):
                        self.b.setdefault((i, j), []).append((a, b))

    def within(self, p, r):
        i0, i1 = int((p[0] - r) // self.CELL), int((p[0] + r) // self.CELL)
        j0, j1 = int((p[1] - r) // self.CELL), int((p[1] + r) // self.CELL)
        return any(seg_dist(p, a, b) <= r for i in range(i0, i1 + 1) for j in range(j0, j1 + 1)
                   for a, b in self.b.get((i, j), ()))


def changes(m, fs, by, houses, fb, bb, hb):
    now, was = {f["name"]: rings(f) for f in fs}, {f["name"]: rings(f) for f in fb}
    kinds = {f["name"]: f.get("kind") for f in fs + fb}
    moved = [n for n in set(now) | set(was) if now.get(n) != was.get(n) and "#" not in n
             and kinds.get(n) not in ("bookmark", "ward", "block", "lot", "terrace")]
    lines = [r for n in moved for g in (now.get(n), was.get(n)) if g for r in g]
    m["moved"] = sorted(moved)
    idx = Segs(lines)
    far = lambda p: not idx.within(p, FAR)
    wa, wb = by["ward"], bb["ward"]
    res = [n for n in set(wa) | set(wb) if (n in wa) != (n in wb) or rings(wa[n]) != rings(wb[n])]
    m["wards_resettled"] = len(res)

    def ward_far(n):
        wr = [r for g in (wa.get(n), wb.get(n)) if g for r in rings(g)]
        wi = Segs([r + r[:1] for r in wr])
        return not any(idx.within(p, FAR_WARD) for r in wr for p in r) and \
            not any(wi.within(q, FAR_WARD) for r in lines for q in r)
    m["wards_resettled_far"] = sum(1 for n in res if ward_far(n))
    a_lot, b_lot = set(by["lot"]) - set(bb["lot"]), set(bb["lot"]) - set(by["lot"])
    for k in ("block", "lot", "street"):
        a, b = set(by[k]), set(bb[k])
        m[f"{k}_added"], m[f"{k}_removed"] = len(a - b), len(b - a)
        m[f"{k}_added_far"] = sum(1 for n in a - b if far(centre(by[k][n])))
        m[f"{k}_removed_far"] = sum(1 for n in b - a if far(centre(bb[k][n])))
    a = {n for n in houses if n.startswith("houses_")}
    b = {n for n in hb if n.startswith("houses_")}
    m["houses_added"], m["houses_removed"] = len(a - b), len(b - a)
    m["houses_added_far"] = sum(1 for n in a - b if far(houses[n]))
    m["houses_removed_far"] = sum(1 for n in b - a if far(hb[n]))
    kept = [rings(by["block"][n]) for n in set(by["block"]) & set(bb["block"])
            if rings(by["block"][n]) == rings(bb["block"][n])]
    in_kept = lambda p: any(inside(p, r[0]) for r in kept)
    m["lot_changed_in_kept_blocks"] = sum(1 for n in a_lot if in_kept(centre(by["lot"][n]))) + \
        sum(1 for n in b_lot if in_kept(centre(bb["lot"][n])))
    m["houses_changed_in_kept_blocks"] = sum(1 for n in a - b if in_kept(houses[n])) + \
        sum(1 for n in b - a if in_kept(hb[n]))
    m["blocks_kept"] = len(kept)
    m["lots_kept"] = len(set(by["lot"]) & set(bb["lot"]))


def main(argv):
    m, fs, by, houses = measures(argv[0])
    if "--base" in argv:
        _, fb, bb, hb = measures(argv[argv.index("--base") + 1])
        changes(m, fs, by, houses, fb, bb, hb)
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
