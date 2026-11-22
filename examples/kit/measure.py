"""Measures of a kit build, for expect.toml: what a build of the plain kit placed, how its pieces
cover each building's outline, and what the build said (docs/research/Modular kit failures in
studios.md, section 13).

    python3 examples/kit/measure.py BUILD [--base BASE] [--check SECTION]

Prints the measures as JSON. With --check, compares them against expect.toml's rows under SECTION:
prints each row and its measure, and exits 1 when one falls outside its band or names a measure not
taken (Python 3.11+, for tomllib). With --base, also what changed against the base build, by name.
Reads only the build directory (spec/build.md): manifest.json, features.json, diagnostics.json and
instances/. Standard library only.

How each is taken (ours). None of `lay`'s diagnostics is read: the walls are measured from where
the pieces stand and how large their models are, against the outline the world wrote. Two things
are taken from what `lay` returned, since nothing else states them: how many storeys a building
has and each storey's level.

- A piece is an instance of an asset with a `role:` tag (a laid piece `lay#<building>_s<k>_...`, or
  a pin); its asset from the manifest's chunk ranges. `pieces` counts them, `wall`, `wall_half`,
  `wall_window`, `corner`, `door` by asset.
- A piece's plan is the box of its asset's measured `bounds` (the model: a pack that declares no
  footprint claims exactly this), or its footprint where the pack declares one that is not that
  box (the inside corner's L), turned by its `face` (degrees clockwise from north, the way its
  front, the asset frame's +y, points; +x is a quarter turn clockwise from it) and moved to its
  place.
- A piece belongs to the building whose outline (the `building` feature's region) is nearest its
  origin, and to the storey of that building whose `level` is nearest its `z`. `floor_off_mm` is the
  most a piece's `z` differs from its storey's level.
- **Walls.** For each storey and each side of the building's outline, the part of every piece's
  plan that lies in the wall's band (from `BAND`, 0.5 m, outside the side to the wall's centre
  line inside it: half the thickness of the kit's first `role:wall` piece, so a wall of the next
  side does not count) gives the interval of the side it covers. The side from end to end, less the union of the intervals, is
  open wall: each stretch longer than `TOL` (5 mm, lay's own fit) counts in `open`, with
  `open_max_mm` and `open_sum_mm`. Two intervals that share more than `TOL` are an overlap:
  `overlaps`, `overlap_max_mm`. The whole outline is measured on every storey, so a stairwell's
  side with no wall over it is open wall (lib/building/README.md, "not built").
- **Floors and roofs** (K4). A piece tagged `role:floor`, `role:roof_slope` or `role:roof_gable`
  is no wall piece: it is left out of `pieces` and of every measure above, and counted in
  `floor`, `roof_slope`, `roof_gable`. Each stands on a storey's level or on a roof's eaves (the
  `eaves` of the `roof` feature `lay` returns), whichever its top or its foot is nearest:
  `deck_off_mm` is the most a floor tile's top, or a roof piece's foot, is off that height.
  Cover is sampled, not clipped: points every `STEP` (0.1 m) on a lattice from `STEP / 2` inside
  each building's outline. `slab_open_dm2` counts, over every storey, the points that lie on the
  storey's own region (the `storey` feature's ring, which leaves out the stairwell) and off the
  walls' band (within the wall's thickness inside a side, corners squared), and under no floor tile of that storey; `slab_double_dm2`
  those under two or more; `slab_stray_dm2` the points in the walls' band or off the storey's
  region (over the stairwell) that lie under a tile. The sampling is too coarse for a tile a few
  centimetres out of place, so `slab_lap_mm` is exact: the most two tiles of one floor, or of one
  flat roof, reach into each other, and `deck_outside_mm` the most a corner of any floor or roof
  piece's plan lies outside its building's outline. A slope or a gable is held to the eaves and
  to nothing else: with no roof feature, or stood on a floor, `deck_off_mm` says so (a tile, which
  may lie on any floor, is held to the nearest). `roof_open_dm2` counts, for each building that has a roof piece or a
  tile at its eaves, the points of its whole outline under no roof piece, less those in the
  walls' band where the roof is flat (the walls' tops stand round a flat
  roof); `roof_double_dm2` those under two or more. One point is a square decimetre.
  `eave_off_mm`: the most the middle of a slope piece's front edge lies off its building's
  outline. `ridge_off_mm`: the most the middle of its back edge lies, along its own front, off the
  middle of the building's box. `eaves_gap_mm`: for each building with a roof, the most the top of
  a wall piece of its top storey stands off the eaves. `gable_turned`: gable halves whose high end (the asset's -x) is not the
  end nearer the middle of the building.
- `alike`: the pairs of bays (pieces tagged `role:wall`: the bay and its variants) that touch along
  a side of one storey and are one asset. A wall of the kit's bay alone counts every joint.
- `outside_max_mm`: the most a corner of a piece's plan lies outside its building's outline. It
  tests corners only, so a piece that spans a cut-out of the outline with all its corners inside
  is not seen.
- **Stacking.** `stack_over_mm` is the most a piece's top (its `z` plus its bounds' height) stands
  above the level of the storey over it; `stack_under_mm` the most it stops short of it.
- **What the build said.** `errors`, `warnings`; `wall_open`, `fillers_only`, `gap_wide`
  `variant_none`, `bay_none` (`step.lay.*` records);
  `revised` (`asset.revised` records), `revised_named` (the instances they name) and `revised_all`
  (1 when every record names exactly the build's instances of its asset); `pivot_moved`
  (`asset.pivot_moved`); `gave_way` (the members `place.gave_way` records count); `pin_removed`
  (the instances `identity.pin_removed` names); `overlap`, `crowded` (`place.overlap`,
  `place.crowded`); `kit_pitch`, `kit_corner`, `kit_height`, `kit_module`, `kit_ell`, `kit_slip`,
  `kit_uneven`, `kit_variant`, `door_narrow`, `unreached` (`step.check.*`).
- With --base: `kept`, `added`, `removed` pieces by name; `moved` the kept ones whose origin or face
  differ by more than 1 mm or 0.001 degree.
"""

import json
import math
import struct
import sys
from collections import Counter, defaultdict
from pathlib import Path

TOL = 0.005
BAND = 0.5
STEP = 0.1                     # metres between the points that sample a floor's or a roof's cover
OFF_WALL = ("role:floor", "role:roof_slope", "role:roof_gable", "role:roof")


def _npy(path):
    raw = Path(path).read_bytes()
    hlen = struct.unpack("<H", raw[8:10])[0]
    head = raw[10:10 + hlen].decode()
    body = raw[10 + hlen:]
    if "<f4" in head:                                   # metres (z)
        return list(struct.unpack(f"<{len(body) // 4}f", body))
    return [v / 1000.0 for v in struct.unpack(f"<{len(body) // 4}i", body)]   # millimetres, millidegrees


def load(d):
    d = Path(d)
    man = json.loads((d / "manifest.json").read_text())
    diags = json.loads((d / "diagnostics.json").read_text())["diagnostics"]
    names = [n for n in (d / "instances/names.txt").read_text().splitlines() if n]
    x, y, z, face = (_npy(d / f"instances/{k}.npy") for k in ("x", "y", "z", "face"))
    asset = [None] * len(names)
    for ch in man["chunks"]:
        for a, (s, n) in ch.get("instances", {}).items():
            for k in range(s, s + n):
                asset[k] = a
    rows = {n: {"asset": asset[k], "x": x[k], "y": y[k], "z": z[k], "face": face[k]}
            for k, n in enumerate(names)}
    return man, diags, rows


def plan_of(r, asset):
    """The plan of an instance in world metres: its asset's bounds box, or the footprint a pack
    declared in its place."""
    f = math.radians(r["face"])
    fx, fy = math.sin(f), math.cos(f)            # the front, +y of the asset frame
    rx, ry = math.cos(f), -math.sin(f)           # +x of the asset frame
    x0, y0, _, x1, y1, _ = asset["bounds"]
    pts = [(x0, y0), (x1, y0), (x1, y1), (x0, y1)]
    foot = [tuple(p) for p in asset.get("footprint") or []]
    if len(foot) >= 3 and sorted(foot) != sorted(pts):
        pts = foot
    return [(r["x"] + ax * rx + ay * fx, r["y"] + ax * ry + ay * fy) for ax, ay in pts]


def clip(poly, axis, bound, keep_above):
    """A polygon of `(u, v)` points cut by `u` or `v` (axis 0 or 1) at `bound`."""
    out = []
    for p, q in zip(poly, poly[1:] + poly[:1]):
        ip, iq = (p[axis] >= bound) == keep_above or p[axis] == bound, (q[axis] >= bound) == keep_above or q[axis] == bound
        if ip:
            out.append(p)
        if ip != iq and q[axis] != p[axis]:
            t = (bound - p[axis]) / (q[axis] - p[axis])
            out.append((p[0] + t * (q[0] - p[0]), p[1] + t * (q[1] - p[1])))
    return out


def span(pts, d, o=(0.0, 0.0)):
    v = [(p[0] - o[0]) * d[0] + (p[1] - o[1]) * d[1] for p in pts]
    return min(v), max(v)


def sides(ring):
    """Each side of a counter-clockwise ring: `(start, unit along, unit inward, length)`."""
    out = []
    for p, q in zip(ring, ring[1:] + ring[:1]):
        L = math.hypot(q[0] - p[0], q[1] - p[1])
        e = ((q[0] - p[0]) / L, (q[1] - p[1]) / L)
        out.append((p, e, (-e[1], e[0]), L))
    return out


def outside(ring, p):
    """Metres the point lies outside the ring; 0 inside or on it."""
    inside = False
    best = float("inf")
    for a, b in zip(ring, ring[1:] + ring[:1]):
        if (a[1] > p[1]) != (b[1] > p[1]) and p[0] < a[0] + (p[1] - a[1]) * (b[0] - a[0]) / (b[1] - a[1]):
            inside = not inside
        L2 = (b[0] - a[0]) ** 2 + (b[1] - a[1]) ** 2
        t = max(0.0, min(1.0, ((p[0] - a[0]) * (b[0] - a[0]) + (p[1] - a[1]) * (b[1] - a[1])) / L2))
        best = min(best, math.hypot(p[0] - a[0] - t * (b[0] - a[0]), p[1] - a[1] - t * (b[1] - a[1])))
    return 0.0 if inside or best < 1e-9 else best


def covers(p, e, n, L, pts, half):
    """The interval of a side (start `p`, along `e`, inward `n`, length `L`) that a plan covers
    inside the wall's band, or None."""
    poly = [((q[0] - p[0]) * e[0] + (q[1] - p[1]) * e[1], (q[0] - p[0]) * n[0] + (q[1] - p[1]) * n[1]) for q in pts]
    for axis, bound, above in ((0, 0.0, True), (0, L, False), (1, -BAND, True), (1, half, False)):
        poly = clip(poly, axis, bound, above)
        if len(poly) < 3:
            return None
    us, vs = [q[0] for q in poly], [q[1] for q in poly]
    if max(us) - min(us) > 1e-9 and max(vs) - min(vs) > 1e-9:
        return (min(us), max(us))
    return None


def alike(ring, bays, half):
    """Pairs of bays that touch along a side and are one asset: `bays` are `(asset, plan)`."""
    count = 0
    for p, e, n, L in sides(ring):
        ivs = sorted((iv, a) for a, pts in bays for iv in [covers(p, e, n, L, pts, half)] if iv)
        count += sum(1 for (i0, a0), (i1, a1) in zip(ivs, ivs[1:]) if abs(i1[0] - i0[1]) <= TOL and a0 == a1)
    return count


def walls(ring, plans, half):
    """Open stretches and overlaps along a ring's sides, in metres: `(open, overlaps)`. `half` is
    half the wall's thickness: the band a piece must reach into to cover a side."""
    opens, overs = [], []
    for p, e, n, L in sides(ring):
        ivs = sorted(iv for iv in (covers(p, e, n, L, pts, half) for pts in plans) if iv)
        at = 0.0
        for u0, u1 in ivs:
            if u0 - at > TOL:
                opens.append(u0 - at)
            elif at - u0 > TOL and at > 0.0:
                overs.append(min(at, u1) - u0)
            at = max(at, u1)
        if L - at > TOL:
            opens.append(L - at)
    return opens, overs


def within(ring, p):
    """True inside the ring or on it."""
    return outside(ring, p) == 0.0


def edge_gap(ring, p):
    """Metres from the point to the nearest side of the ring."""
    best = float("inf")
    for a, b in zip(ring, ring[1:] + ring[:1]):
        L2 = (b[0] - a[0]) ** 2 + (b[1] - a[1]) ** 2
        t = max(0.0, min(1.0, ((p[0] - a[0]) * (b[0] - a[0]) + (p[1] - a[1]) * (b[1] - a[1])) / L2))
        best = min(best, math.hypot(p[0] - a[0] - t * (b[0] - a[0]), p[1] - a[1] - t * (b[1] - a[1])))
    return best


def in_band(ring, p, thick):
    """True where the point lies in the walls' band: within `thick` inside some side, between that
    side's ends each carried `thick` on (so the square at a corner, either way it turns, is band)."""
    for a, e, n, L in sides(ring):
        u = (p[0] - a[0]) * e[0] + (p[1] - a[1]) * e[1]
        v = (p[0] - a[0]) * n[0] + (p[1] - a[1]) * n[1]
        if -thick <= u <= L + thick and 0 <= v <= thick:
            return True
    return False


def lattice(ring):
    """Points every `STEP` over the ring's box that lie inside it."""
    xs, ys = [p[0] for p in ring], [p[1] for p in ring]
    nx, ny = int(round((max(xs) - min(xs)) / STEP)), int(round((max(ys) - min(ys)) / STEP))
    pts = [(min(xs) + (i + 0.5) * STEP, min(ys) + (j + 0.5) * STEP) for i in range(nx) for j in range(ny)]
    return [p for p in pts if within(ring, p)]


def laps(tiles):
    """The most two tiles of one floor reach into each other, metres: each `(row, asset, plan)`,
    compared in the first tile's own frame (they share a face on a floor laid as a grid)."""
    worst = 0.0
    for i, (r, a, _) in enumerate(tiles):
        f = math.radians(r["face"])
        front, right = (math.sin(f), math.cos(f)), (math.cos(f), -math.sin(f))
        w, d = a["bounds"][3] - a["bounds"][0], a["bounds"][4] - a["bounds"][1]
        for s, b, _ in tiles[i + 1:]:
            du = abs((s["x"] - r["x"]) * right[0] + (s["y"] - r["y"]) * right[1])
            dv = abs((s["x"] - r["x"]) * front[0] + (s["y"] - r["y"]) * front[1])
            worst = max(worst, min(w - du, d - dv))
    return max(worst, 0.0)


def decks_of(rows, assets, feats, rings, levels, decks, thick, near):
    """The floor and roof measures (module doc, "Floors and roofs")."""
    eaves = {n: None for n in rings}
    floors = {}
    for name, f in feats.items():
        if f.get("kind") == "roof" and name.startswith("lay#"):
            b = min(rings, key=lambda k: near(rings[k], f["pieces"][0]["outer"][0]))
            eaves[b] = f["attrs"]["eaves"]
        if f.get("kind") == "storey" and name.startswith("lay#"):
            floors[name[4:]] = [tuple(p) for p in f["pieces"][0]["outer"]]
    tiles, roofs, off, stray = defaultdict(list), defaultdict(list), 0.0, 0.0
    for n, r in rows.items():
        if r["asset"] not in decks:
            continue
        a = assets[r["asset"]]
        b = min(rings, key=lambda k: near(rings[k], (r["x"], r["y"])))
        pts = plan_of(r, a)
        flat = "role:floor" in a["tags"]
        foot = r["z"] + (a["bounds"][5] if flat else 0.0)        # a tile's top, a roof piece's foot
        stray = max(stray, max(outside(rings[b], q) for q in pts))
        if flat:                                                 # a tile lies on a floor or on the eaves
            heights = [(abs(lv - foot), ("s", k)) for k, lv in enumerate(levels[b])]
            if eaves[b] is not None:
                heights.append((abs(eaves[b] - foot), ("r", 0)))
            d, where = min(heights)
        else:                                                    # a slope or a gable stands on the eaves, nowhere else
            top = levels[b][-1]
            d, where = (abs(eaves[b] - foot) if eaves[b] is not None else abs(foot - top) + 99.0), ("r", 0)
        off = max(off, d)
        (roofs if where[0] == "r" else tiles)[(b, where[1])].append((r, a, pts))
    out = {"deck_off_mm": round(off * 1000), "deck_outside_mm": round(stray * 1000)}
    s_open = s_double = s_stray = r_open = r_double = 0
    lap = 0.0
    eave = ridge = 0.0
    turned = 0
    for b in sorted(rings):
        pts = lattice(rings[b])
        band = {p: in_band(rings[b], p, thick) for p in pts}
        for k in range(len(levels[b])):
            floor = floors.get(f"{b}_s{k}")
            mine = [q for _, _, q in tiles.get((b, k), [])]
            if floor is None or not any(v for (bb, _), v in tiles.items() if bb == b):
                continue                                       # a building with no floor tile is not measured
            for p in pts:
                c = sum(1 for q in mine if within(q, p))
                if band[p] or not within(floor, p):
                    s_stray += c > 0                           # a tile in the wall, or over the stairwell
                    continue
                s_open += c == 0
                s_double += c > 1
            lap = max(lap, laps(tiles.get((b, k), [])))
        lap = max(lap, laps([x for x in roofs.get((b, 0), []) if "role:floor" in x[1]["tags"]]))
        mine = roofs.get((b, 0), [])
        if not mine:
            continue
        flat = all("role:floor" in a["tags"] for _, a, _ in mine)
        for p in pts:
            if flat and band[p]:
                continue
            c = sum(1 for _, _, q in mine if within(q, p))
            r_open += c == 0
            r_double += c > 1
        xs, ys = [q[0] for q in rings[b]], [q[1] for q in rings[b]]
        mid = ((min(xs) + max(xs)) / 2, (min(ys) + max(ys)) / 2)          # the middle of the building's box
        for r, a, q in mine:
            f = math.radians(r["face"])
            front, right = (math.sin(f), math.cos(f)), (math.cos(f), -math.sin(f))
            if "role:roof_slope" in a["tags"]:
                y0, y1 = a["bounds"][1], a["bounds"][4]
                lo = (r["x"] + front[0] * y1, r["y"] + front[1] * y1)
                hi = (r["x"] + front[0] * y0, r["y"] + front[1] * y0)
                eave = max(eave, edge_gap(rings[b], lo))
                ridge = max(ridge, abs((hi[0] - mid[0]) * front[0] + (hi[1] - mid[1]) * front[1]))
            if "role:roof_gable" in a["tags"]:
                x0, x1 = a["bounds"][0], a["bounds"][3]
                high = (r["x"] + right[0] * x0, r["y"] + right[1] * x0)
                low = (r["x"] + right[0] * x1, r["y"] + right[1] * x1)
                turned += math.hypot(high[0] - mid[0], high[1] - mid[1]) >= math.hypot(low[0] - mid[0], low[1] - mid[1])
    out.update({"slab_open_dm2": s_open, "slab_double_dm2": s_double, "slab_stray_dm2": s_stray,
                "slab_lap_mm": round(lap * 1000), "roof_open_dm2": r_open,
                "roof_double_dm2": r_double, "eave_off_mm": round(eave * 1000), "ridge_off_mm": round(ridge * 1000),
                "gable_turned": turned})
    return out


def measures(d):
    man, diags, rows = load(d)
    assets = man["assets"]
    feats = {f["name"]: f for f in json.loads((Path(d) / "features.json").read_text())["features"]}
    kit = {a for a, v in assets.items() if any(t.startswith("role:") for t in v["tags"])}
    decks = {a for a in kit if any(t in OFF_WALL for t in assets[a]["tags"])}
    pieces = {n: r for n, r in rows.items() if r["asset"] in kit - decks}
    m = {"pieces": len(pieces)}
    by_asset = Counter(r["asset"] for r in rows.values() if r["asset"] in kit)
    for a in ("wall", "wall_half", "wall_window", "wall_vent", "corner", "corner_in", "door",
              "ground_wall", "ground_half", "ground_corner", "ground_door", "floor", "roof_slope", "roof_gable"):
        m[a] = by_asset.get(a, 0)

    rings = {n: [tuple(p) for p in f["pieces"][0]["outer"]] for n, f in feats.items() if f.get("kind") == "building"}
    levels = {n: [feats[f"lay#{n}_s{k}"]["attrs"]["level"] for k in range(int(feats[n]["attrs"]["storeys"]))]
              for n in rings}

    def near(ring, p):
        return min(math.hypot(p[0] - a[0] - t * (b[0] - a[0]), p[1] - a[1] - t * (b[1] - a[1]))
                   for a, b in zip(ring, ring[1:] + ring[:1])
                   for t in [max(0.0, min(1.0, ((p[0] - a[0]) * (b[0] - a[0]) + (p[1] - a[1]) * (b[1] - a[1]))
                                          / ((b[0] - a[0]) ** 2 + (b[1] - a[1]) ** 2)))])

    storey = defaultdict(dict)
    off = out = 0.0
    for n, r in pieces.items():
        b = min(rings, key=lambda k: near(rings[k], (r["x"], r["y"])))
        k = min(range(len(levels[b])), key=lambda i: abs(levels[b][i] - r["z"]))
        off = max(off, abs(levels[b][k] - r["z"]))
        pts = plan_of(r, assets[r["asset"]])
        out = max(out, max(outside(rings[b], p) for p in pts))
        storey[(b, k)][n] = (r, pts)
    m["floor_off_mm"], m["outside_max_mm"] = round(off * 1000), round(out * 1000)

    wall = min((a for a, v in assets.items() if "role:wall" in v["tags"]), default=None)
    half = (assets[wall]["bounds"][4] - assets[wall]["bounds"][1]) / 2 if wall else 0.15
    opens, overs = [], []
    for b in sorted(rings):
        for k in range(len(levels[b])):
            o, v = walls(rings[b], [pts for _, pts in storey.get((b, k), {}).values()], half)
            opens += [(x, b, k) for x in o]
            overs += [(x, b, k) for x in v]
    m["alike"] = sum(alike(rings[b], [(r["asset"], pts) for r, pts in storey.get((b, k), {}).values()
                                      if "role:wall" in assets[r["asset"]]["tags"]], half)
                     for b in sorted(rings) for k in range(len(levels[b])))
    m["open"] = len(opens)
    m["open_max_mm"] = round(max((s[0] for s in opens), default=0.0) * 1000)
    m["open_sum_mm"] = round(sum(s[0] for s in opens) * 1000)
    m["overlaps"] = len(overs)
    m["overlap_max_mm"] = round(max((s[0] for s in overs), default=0.0) * 1000)
    m["_open"] = sorted([round(s[0] * 1000), f"{s[1]} storey {s[2]}"] for s in opens)
    m["_overlaps"] = sorted([round(s[0] * 1000), f"{s[1]} storey {s[2]}"] for s in overs)

    m.update(decks_of(rows, assets, feats, rings, levels, decks, half * 2, near))
    gap = 0.0
    for name, f in feats.items():
        if f.get("kind") == "roof" and name.startswith("lay#"):
            b = min(rings, key=lambda k: near(rings[k], f["pieces"][0]["outer"][0]))
            for r, _ in storey.get((b, len(levels[b]) - 1), {}).values():
                if "role:door" not in assets[r["asset"]]["tags"]:
                    gap = max(gap, abs(f["attrs"]["eaves"] - r["z"] - assets[r["asset"]]["bounds"][5]))
    m["eaves_gap_mm"] = round(gap * 1000)

    over = under = 0.0
    for (b, k), ps in storey.items():
        if k + 1 >= len(levels[b]):
            continue
        up = levels[b][k + 1]
        for r, _ in ps.values():
            top = r["z"] + assets[r["asset"]]["bounds"][5]
            over, under = max(over, top - up), max(under, up - top)
    m["stack_over_mm"], m["stack_under_mm"] = round(over * 1000), round(under * 1000)

    codes = Counter(x["code"] for x in diags)
    sev = Counter(x.get("severity") for x in diags)
    m["errors"], m["warnings"] = sev.get("error", 0), sev.get("warning", 0)
    m["wall_open"] = codes.get("step.lay.wall_open", 0)
    m["pivot_moved"] = codes.get("asset.pivot_moved", 0)
    m["overlap"], m["crowded"] = codes.get("place.overlap", 0), codes.get("place.crowded", 0)
    for c in ("kit_pitch", "kit_corner", "kit_height", "kit_module", "kit_ell", "kit_slip", "kit_uneven", "kit_variant",
              "door_narrow", "unreached"):
        m[c] = codes.get("step.check." + c, 0)
    for c in ("fillers_only", "gap_wide", "variant_none", "bay_none", "slab_open", "roof_none"):
        m[c] = codes.get("step.lay." + c, 0)
    m["gave_way"] = sum(int((x.get("data") or {}).get("overlap", 0)) + int((x.get("data") or {}).get("relief", 0))
                        for x in diags if x["code"] == "place.gave_way")
    m["pin_removed"] = sum(int((x.get("data") or {}).get("count", 0)) for x in diags if x["code"] == "identity.pin_removed")
    revised = [x for x in diags if x["code"] == "asset.revised"]
    m["revised"] = len(revised)
    named, whole = 0, 1
    for x in revised:
        got = set((x.get("data") or {}).get("instances") or [])
        named += len(got)
        want = {n for n, r in rows.items() if "asset." + str(r["asset"]) == x.get("subject")}
        whole &= int(got == want)
    m["revised_named"], m["revised_all"] = named, (whole if revised else 0)
    m["_codes"] = dict(sorted(codes.items()))
    return m, pieces


def against(pieces, base):
    kept = set(pieces) & set(base)
    moved = [n for n in kept
             if math.hypot(pieces[n]["x"] - base[n]["x"], pieces[n]["y"] - base[n]["y"]) > 0.001
             or abs(pieces[n]["z"] - base[n]["z"]) > 0.001
             or abs((pieces[n]["face"] - base[n]["face"] + 180) % 360 - 180) > 0.001]
    return {"kept": len(kept), "added": len(set(pieces) - set(base)),
            "removed": len(set(base) - set(pieces)), "moved": len(moved)}


def main(argv):
    args = argv[1:]
    base = check = None
    if "--base" in args:
        i = args.index("--base")
        base = args[i + 1]
        del args[i:i + 2]
    if "--check" in args:
        i = args.index("--check")
        check = args[i + 1]
        del args[i:i + 2]
    if len(args) != 1:
        print(__doc__.split("\n\n")[1], file=sys.stderr)
        return 2
    m, pieces = measures(args[0])
    if base:
        m.update(against(pieces, measures(base)[1]))
    if not check:
        print(json.dumps(m, indent=1, sort_keys=True))
        return 0
    import tomllib
    rows = tomllib.loads((Path(__file__).resolve().parent / "expect.toml").read_text()).get(check)
    if not rows:
        print(f"no section `{check}` in expect.toml", file=sys.stderr)
        return 2
    bad = 0
    for key, row in rows.items():
        v = m.get(row["measure"])
        lo, hi = row["band"]
        ok = v is not None and lo <= v <= hi
        bad += not ok
        print(f"{'ok  ' if ok else 'FAIL'} {check}.{key}: {row['measure']} = {v} in [{lo}, {hi}]")
    print(f"{len(rows) - bad}/{len(rows)} rows")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
