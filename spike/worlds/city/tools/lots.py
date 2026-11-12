"""Fills street frontages with rows of lots. This is the project's own rule for where buildings go.

It is handed the build of everything below it: arterials, the road step's local streets and wards. For every street, on both sides, it walks the frontage one lot
at a time and keeps a lot only where its footprint is clear of every way, the river, the park and
the lots already kept. Which kit piece a lot takes is its district's word: the places in the pack's
taxonomy it names, between the floors it names, the tallest only where it is most built up. What it hands back is plain kit runs, which
the compiler places like any others.
"""
import math

from shapely.geometry import LineString, Point, Polygon, box
from shapely.ops import unary_union
from shapely.prepared import prep

from viasdk import step

s = step()
p, f = s.params, s.frame
import tomllib
from pathlib import Path

# The pieces a lot may take come from the pack by taxonomy. A step cannot ask the compiler for the
# pack, so this reads the committed one itself: a build against another pack would still be laid out for this one.
PACK = tomllib.loads((Path(__file__).resolve().parents[3] / "packs" / "primitive.toml").read_text())["asset"]
FLOOR_M = 3.4
PIECES = {}        # taxon -> [(asset id, frontage module, depth, floors)]
for aid, a in PACK.items():
    if "taxon" in a and "module_m" in a:
        PIECES.setdefault(a["taxon"], []).append((aid, float(a["module_m"]), float(a["size"][1]) + 2.0, a["size"][2] / FLOOR_M))

wards = []
for ft in s.build.features.values():
    if ft["kind"] == "ward":
        have = [pc for t in ft["buildings"].split(",") if t for tx, pcs in PIECES.items() if tx.startswith(t.lstrip("@")) for pc in pcs]
        have = [pc for pc in have if ft["floors_lo"] - 0.5 <= pc[3] <= ft["floors_hi"] + 0.5]
        if not have:
            s.warn("no_buildings", ft["id"], f"nothing in the pack under '{ft['buildings']}' has between {ft['floors_lo']:g} and {ft['floors_hi']:g} floors; its lots stay empty")
        wards.append((prep(Polygon(ft["points"])), ft, sorted(have, key=lambda pc: pc[3])))


def ward(x, y):
    """The ward a lot stands in says what it may take. Parks and ground outside every district take nothing."""
    here = Point(x, y)
    return next(((ft, have) for poly, ft, have in wards if poly.contains(here)), (None, []))


def half(feat):
    a = feat.get("attrs", {})
    return a.get("width", 8.0) / 2 + a.get("sidewalk", a.get("verge", 2.0))


keep_out, streets = [], []
for feat in s.build.features.values():
    if feat["shape"] == "path" and feat["kind"] in ("street", "motorway", "railway", "freeway", "bridge", "rail_bridge", "ramp"):
        line = LineString(feat["points"])
        keep_out.append(line.buffer(half(feat) + 0.5, cap_style="flat"))
        if feat["kind"] == "street":
            streets.append((feat, line))
    elif feat["shape"] == "region" and (feat["kind"] in ("lake", "park", "lot", "square") or feat.get("water")):
        keep_out.append(Polygon(feat["points"]).buffer(8.0 if feat["kind"] == "lake" or feat.get("water") else 1.0))
blocked = prep(unary_union(keep_out))
inside = box(f.x0 + 6, f.y0 + 6, f.x1 - 6, f.y1 - 6)
# What a larger map gives this city: where it may be, and how built up. Alone: everywhere, fully.
limits = s.build.features[p.get("limits", "limits")]
given = bool(limits.get("given")) or "limits" in p
if given:
    inside = prep(inside.intersection(Polygon(limits["points"]).buffer(0).buffer(-4)))
urban = s.build.field("urban")
main_ways = unary_union([line for feat, line in streets if feat.get("arterial")])


def built(x, y):
    """How built up a spot is: what the map gives, lifted a little along the arterials."""
    u = float(urban[f.index(x, y)])
    return min(1.0, u + 0.3 * math.exp(-main_ways.distance(Point(x, y)) / 50.0)) if u > 0.02 else 0.0


def chance(x, y):
    """The same number between 0 and 1 for the same spot, every build."""
    return ((int(x * 7.3) * 73856093) ^ (int(y * 7.3) * 19349663)) % 1000 / 1000.0
kept: list = []
runs = 0


def clear(rect):
    if not inside.contains(rect) or blocked.intersects(rect):
        return False
    x0, y0, x1, y1 = rect.bounds
    return not any(b[0] < x1 and b[2] > x0 and b[1] < y1 and b[3] > y0 and r.intersects(rect) for r, b in kept)


for feat, line in sorted(streets, key=lambda st: -st[0].get("attrs", {}).get("width", 0)):      # the widest streets take their frontage first
    for side in (1, -1):
        at, run = 0.0, None

        def close():
            global runs
            if run:
                a, b = run["a"], run["b"]
                s.add("kitrun", {"id": f"lots_{runs}", "from": [round(a[0], 2), round(a[1], 2)], "to": [round(b[0], 2), round(b[1], 2)],
                                 "bay": run["bays"], "faces": "right" if side == 1 else "left", "kind": "lot",
                                 "margin": 2.0, "seed": runs})
                runs += 1

        while at < line.length:
            a, b = line.interpolate(at), line.interpolate(min(at + 1.0, line.length))
            tx, ty = b.x - a.x, b.y - a.y
            ln = math.hypot(tx, ty)
            if ln < 1e-6:
                break
            tx, ty = tx / ln, ty / ln
            nx, ny = -ty * side, tx * side
            ft, have = ward(a.x + nx * 24, a.y + ny * 24)
            z = None
            if have:      # less built up: lower buildings, then gaps between them, then none
                u = ft["density"] if ft["density"] >= 0 else (built(a.x + nx * 24, a.y + ny * 24) if given else 1.0)
                top = have[0][3] + (have[-1][3] - have[0][3]) * min(1.0, u / 0.8)
                ok = [pc for pc in have if pc[3] <= top + 0.01]
                module = max(ok, key=lambda pc: pc[3])[1]          # the tallest it may have sets the lot's frontage
                ok = [pc for pc in ok if pc[1] == module]
                if chance(a.x, a.y) < (u - 0.05) / 0.45:
                    z = (ft["district"], module)
                    depth = max(pc[2] for pc in ok)
                    bays = [{"id": pc[0], "weight": 1 + int(pc[3] >= ok[-1][3] - 2)} for pc in ok]
            if z is None:
                close()
                run = None
                at += 8.0
                continue
            back = half(feat) + 1.5
            c0 = (a.x + nx * back, a.y + ny * back)
            c1 = (c0[0] + tx * module, c0[1] + ty * module)
            rect = Polygon([c0, c1, (c1[0] + nx * depth, c1[1] + ny * depth), (c0[0] + nx * depth, c0[1] + ny * depth)])
            if at + module <= line.length and clear(rect):
                kept.append((rect, rect.bounds))
                mid0 = (c0[0] + nx * depth / 2, c0[1] + ny * depth / 2)
                mid1 = (c1[0] + nx * depth / 2, c1[1] + ny * depth / 2)
                if run and run["zone"] == z and math.hypot(run["b"][0] - mid0[0], run["b"][1] - mid0[1]) < 0.01:
                    run["b"] = mid1
                else:
                    close()
                    run = {"zone": z, "a": mid0, "b": mid1, "bays": bays}
                at += module
            else:
                close()
                run = None
                at += 4.0
        close()

if not kept:
    s.warn("no_lots", "lots", "no frontage was free for a single lot")
s.done()
