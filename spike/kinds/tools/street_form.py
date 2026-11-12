"""How a street network is shaped, as numbers. Town library knowledge, not the compiler's.

    bearing_spread   0 when every street runs along one grid, 1 when streets run every way alike.
                     Each street is cut into straight pieces; a piece's bearing is folded to 0..90
                     degrees (a grid's two directions are one), weighted by its length, and put in
                     18 bins of 5 degrees. The spread is the entropy of those bins over ln 18.
    order            the orientation order of the street-network literature (Boeing, "Urban spatial
                     order", 2019): 36 bins of 10 degrees over the full circle, both directions of
                     every street, 1 for a perfect grid and 0 for none. Given for comparison.
    main_*           the same two, for the main ways alone (streets marked `arterial`, and roads).
    junctions        where streets meet: the share that are four-way (or more), three-way, and
                     dead ends. A planned grid is mostly four-way; a grown town mostly three-way.
                     `dead_ends` is their number, and `stubs` the number of those whose street is
                     shorter than 40 m from its last junction: a street that goes nowhere and is
                     too short to have been meant. A bridge counts as a street here, so a way that
                     crosses water does not end at each bank.

As a lint step it reads the finished build and says the numbers as an `info` diagnostic and in
`street_form.json`. Run by hand it measures any build:

    uv run python kinds/tools/street_form.py build/atlas --ids city.
"""
import json
import math
import os
import sys
from pathlib import Path

import numpy as np
from shapely.geometry import LineString
from shapely.ops import unary_union

WAYS = ("street", "road", "motorway", "fade", "lane", "bridge")
MAIN = ("road", "motorway")


def spread(bearings, lengths):
    """(bearing_spread, order) of pieces given as bearings in degrees and lengths."""
    if not len(bearings):
        return None, None
    b, w = np.asarray(bearings, float), np.asarray(lengths, float)
    folded = np.histogram((b + 2.5) % 90.0, bins=18, range=(0, 90), weights=w)[0]
    p = folded[folded > 0] / folded.sum()
    one = float(-(p * np.log(p)).sum() / math.log(18))
    both = np.concatenate([b % 360.0, (b + 180.0) % 360.0])
    full = np.histogram((both + 5.0) % 360.0, bins=36, range=(0, 360), weights=np.concatenate([w, w]))[0]
    q = full[full > 0] / full.sum()
    h, hg, hmax = float(-(q * np.log(q)).sum()), math.log(4), math.log(36)
    order = 1.0 - ((max(h, hg) - hg) / (hmax - hg)) ** 2
    return round(one, 3), round(order, 3)


def pieces(line):
    c = np.asarray(line.coords)
    d = np.diff(c, axis=0)
    return np.degrees(np.arctan2(d[:, 1], d[:, 0])), np.hypot(d[:, 0], d[:, 1])


def junctions(lines, reach=9.0, stub=14.0, merge=11.0, short=40.0):
    """Degrees of the places where the lines meet. Ends are carried `reach` further so that a street
    drawn up to another's edge meets its centre line; the stubs that leaves are dropped again."""
    longer = []
    for l in lines:
        c = list(l.coords)
        if len(c) < 2 or l.length < 1:
            continue
        for end, nxt in ((0, 1), (-1, -2)):
            (x0, y0), (x1, y1) = c[end], c[nxt]
            d = math.hypot(x1 - x0, y1 - y0) or 1.0
            c[end] = (x0 - (x1 - x0) / d * reach, y0 - (y1 - y0) / d * reach)
        longer.append(LineString(c))
    net = unary_union(longer)
    segs = [g for g in getattr(net, "geoms", [net]) if g.geom_type == "LineString"]
    ends = np.asarray([p for g in segs for p in (g.coords[0], g.coords[-1])])
    if not len(ends):
        return {}
    # merge ends that lie within `merge` of each other into one place
    label = -np.ones(len(ends), int)
    cell = {}
    for i, (x, y) in enumerate(ends):
        cell.setdefault((int(x // merge), int(y // merge)), []).append(i)
    n = 0
    for i in range(len(ends)):
        if label[i] >= 0:
            continue
        label[i] = n
        todo = [i]
        while todo:
            j = todo.pop()
            cx, cy = int(ends[j][0] // merge), int(ends[j][1] // merge)
            for k in (k for dx in (-1, 0, 1) for dy in (-1, 0, 1) for k in cell.get((cx + dx, cy + dy), [])):
                if label[k] < 0 and math.hypot(*(ends[k] - ends[j])) <= merge:
                    label[k] = n
                    todo.append(k)
        n += 1
    edges = [(label[2 * i], label[2 * i + 1], g.length) for i, g in enumerate(segs)]
    deg = np.zeros(n, int)
    for a, b, _ in edges:
        if a != b:
            deg[a] += 1
            deg[b] += 1
    for a, b, ln in edges:                      # drop the stubs the carrying-on left: a short edge to a place nothing else reaches
        if a != b and ln < stub:
            for u, v in ((a, b), (b, a)):
                if deg[u] == 1:
                    deg[u] -= 1
                    deg[v] -= 1
    four, three, dead = int((deg >= 4).sum()), int((deg == 3).sum()), int((deg == 1).sum())
    total = max(four + three + dead, 1)
    stubs = sum(1 for a, b, ln in edges if a != b and ln < short and (deg[a] == 1 or deg[b] == 1))
    return {"places": four + three + dead, "four_way": round(four / total, 3), "three_way": round(three / total, 3), "dead_end": round(dead / total, 3),
            "dead_ends": dead, "stubs": stubs}


def measure(features, ids=""):
    ways = [f for f in features if f.get("shape") == "path" and f.get("kind") in WAYS and str(f["id"]).startswith(ids) and len(f["points"]) > 1]
    lines = [LineString(f["points"]) for f in ways]
    main = [l for f, l in zip(ways, lines) if f.get("arterial") or f.get("kind") in MAIN]

    def of(ls):
        if not ls:
            return [], []
        b, w = zip(*(pieces(l) for l in ls))
        return np.concatenate(b), np.concatenate(w)
    s_all, o_all = spread(*of(lines))
    s_main, o_main = spread(*of(main))
    return {"streets": len(lines), "length_m": round(sum(l.length for l in lines)), "bearing_spread": s_all, "order": o_all,
            "main_ways": len(main), "main_length_m": round(sum(l.length for l in main)), "main_bearing_spread": s_main, "main_order": o_main,
            "junctions": junctions(lines)}


def sentence(m):
    j = m["junctions"]
    return (f"{m['streets']} streets, {m['length_m'] / 1000:.1f} km: bearing spread {m['bearing_spread']} (0 one grid, 1 every way), "
            f"main ways alone {m['main_bearing_spread']}; order {m['order']}; junctions {j.get('four_way', 0):.0%} four-way, "
            f"{j.get('three_way', 0):.0%} three-way, {j.get('dead_end', 0):.0%} dead ends ({j.get('dead_ends', 0)}, of which {j.get('stubs', 0)} stubs under 40 m)")


if os.environ.get("VIA_STEP"):
    sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "sdk"))
    from viasdk import step
    s = step()
    m = measure(list(s.build.features.values()), str(s.params.get("ids", "")))
    s.out.mkdir(parents=True, exist_ok=True)
    (s.out / "street_form.json").write_text(json.dumps(m, indent=1))
    s._say("info", "street_form", s.params.get("ids") or "streets", sentence(m), None, "")
    least = s.params.get("min_spread")
    if least is not None and m["bearing_spread"] is not None and m["bearing_spread"] < float(least):
        s.warn("one_grid", "streets", f"bearing spread is {m['bearing_spread']}; this world asks for at least {float(least)}: the town reads as one grid",
               hint="turn a district's `bearing`, or state main ways by their ends so they follow the land")
    print("street_form:", sentence(m))
    s.done()
elif __name__ == "__main__":
    folder = Path(sys.argv[1])
    ids = sys.argv[sys.argv.index("--ids") + 1] if "--ids" in sys.argv else ""
    m = measure(json.loads((folder / "world.json").read_text())["features"], ids)
    print(json.dumps(m))
    print(sentence(m))
