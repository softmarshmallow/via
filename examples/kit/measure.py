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


def measures(d):
    man, diags, rows = load(d)
    assets = man["assets"]
    feats = {f["name"]: f for f in json.loads((Path(d) / "features.json").read_text())["features"]}
    kit = {a for a, v in assets.items() if any(t.startswith("role:") for t in v["tags"])}
    pieces = {n: r for n, r in rows.items() if r["asset"] in kit}
    m = {"pieces": len(pieces)}
    by_asset = Counter(r["asset"] for r in pieces.values())
    for a in ("wall", "wall_half", "wall_window", "wall_vent", "corner", "corner_in", "door"):
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
    for c in ("fillers_only", "gap_wide", "variant_none", "bay_none"):
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
