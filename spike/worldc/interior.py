"""Interiors: rooms on levels, openings in their walls, stairs between levels.

Nothing here is a new primitive. A level is a frame: a named floor height. A room is a region feature
on a level. A door or window is a point feature on a room's outline. A stair is a feature that links
two rooms on different levels. What is new is the producer: it lowers rooms to built solids (walls,
floor slabs, steps) the way scatter lowers a density to instances, and it checks the seams that only
buildings have: sealed rooms, doors that open onto nothing, thresholds that miss the ground, and
furniture that stands in a wall or a doorway.
"""
from __future__ import annotations

import math

import mapbox_earcut as earcut
import numpy as np
from PIL import Image, ImageDraw
from shapely import affinity
from shapely.geometry import Point, Polygon, box
from shapely.geometry.polygon import orient
from shapely.ops import unary_union

from .world import World


def _prism(shape, z0: float, z1: float):
    """Triangles for a 2D shape (holes allowed) extruded between two heights, flat shaded."""
    polys = [shape] if shape.geom_type == "Polygon" else list(getattr(shape, "geoms", []))
    V, N, F = [], [], []
    base = 0
    for poly in polys:
        if poly.is_empty or poly.geom_type != "Polygon" or poly.area < 1e-4:
            continue
        poly = orient(poly, 1.0)
        rings = [np.asarray(poly.exterior.coords)[:-1]] + [np.asarray(r.coords)[:-1] for r in poly.interiors]
        flat = np.vstack(rings)
        ends = np.cumsum([len(r) for r in rings]).astype(np.uint32)
        tris = earcut.triangulate_float64(flat, ends).reshape(-1, 3)
        for z, up in ((z1, 1.0), (z0, -1.0)):
            V.append(np.column_stack([flat, np.full(len(flat), z)]))
            N.append(np.tile([0.0, 0.0, up], (len(flat), 1)))
            F.append((tris if up > 0 else tris[:, ::-1]) + base)
            base += len(flat)
        for ring in rings:
            a, b = ring, np.roll(ring, -1, axis=0)
            d = b - a
            n = np.column_stack([d[:, 1], -d[:, 0], np.zeros(len(d))])
            n /= np.maximum(np.linalg.norm(n, axis=1, keepdims=True), 1e-9)
            quad = np.stack([np.column_stack([a, np.full(len(a), z0)]), np.column_stack([b, np.full(len(a), z0)]),
                             np.column_stack([b, np.full(len(a), z1)]), np.column_stack([a, np.full(len(a), z1)])], axis=1)
            V.append(quad.reshape(-1, 3))
            N.append(np.repeat(n, 4, axis=0))
            idx = base + np.arange(len(a))[:, None] * 4
            F.append(np.vstack([np.hstack([idx, idx + 1, idx + 2]), np.hstack([idx, idx + 2, idx + 3])]))
            base += len(a) * 4
    if not V:
        return None
    return np.vstack(V), np.vstack(N), np.vstack(F)


def build_interiors(world: World, instances: list[dict]) -> dict:
    doc, diag, g = world.doc, world.diag, world.grid
    levels = {lv["id"]: lv for lv in doc.get("level", [])}
    out = {"meshes": [], "lights": [], "plans": {}}
    if not levels:
        return out
    kinds = world.kinds
    rooms, opens, stairs = {}, [], []
    for f in world.features:
        k = kinds.get(f.get("kind", ""), {})
        shape = world.shapes.get(f.get("id"))
        if shape is None:
            continue
        if k.get("encloses"):
            if f.get("level") not in levels:
                diag.error("room.no_level", f["id"], f"a room needs a declared level; '{f.get('level')}' is not one")
                continue
            rooms[f["id"]] = {"feat": f, "poly": Polygon(shape.polygon), "level": levels[f["level"]], "kind": k,
                              "wall": float(world.attrs(f).get("wall", 0.4))}
        elif k.get("opening"):
            opens.append((f, k))
        elif k.get("link") == "stair":
            stairs.append(f)
        if k.get("light") and "at" in f:
            lv = levels.get(f.get("level", ""), {"z": 0.0})
            li = k["light"]
            out["lights"].append({"pos": [f["at"][0], f["at"][1], lv["z"] + float(f.get("h", li.get("h", 2.2)))],
                                  "color": li.get("color", [1.0, 0.8, 0.55]), "range": li.get("range", 7.0),
                                  "energy": li.get("energy", 1.6)})

    # ---- openings: a box through the wall at a point on a room's outline
    edges: dict = {r: set() for r in rooms}           # the room graph, for the sealed-room check
    cuts: dict = {}                                    # level id -> [(box, z0, z1)]
    door_zones = []
    for f, k in opens:
        host = f.get("on", {}).get("feature")
        if host not in rooms:
            diag.error("opening.no_wall", f["id"], f"an opening must be `on` a room's outline; '{host}' is not a room")
            continue
        room = rooms[host]
        p, tan, normal = world.frame_at(f["on"], f["id"])
        width = float(world.attrs(f).get("width", 1.0))
        sill = float(world.attrs(f).get("sill", 0.0))
        head = float(world.attrs(f).get("head", 2.1))
        ang = math.degrees(math.atan2(tan[1], tan[0]))
        depth = room["wall"] * 2 + 1.2
        cut = affinity.translate(affinity.rotate(box(-width / 2, -depth / 2, width / 2, depth / 2), ang, origin=(0, 0)), p[0], p[1])
        z = room["level"]["z"]
        cuts.setdefault(room["level"]["id"], []).append((cut, z + sill, z + head))
        f["floor_z"] = z
        if k["opening"] != "door":
            continue
        door_zones.append((f["id"], room["level"]["id"], affinity.translate(affinity.rotate(box(-width / 2, -1.0, width / 2, 1.0), ang, origin=(0, 0)), p[0], p[1])))
        to = f.get("to", "outside")
        if to == "outside":
            edges[host].add("outside")
            beyond = p + normal * (room["wall"] + 0.8)
            ground = float(g.sample(world.field("height"), [beyond[0]], [beyond[1]])[0])
            if abs(ground - z) > 0.3:
                diag.warn("seam.threshold", f["id"],
                          f"its threshold is at {z:.2f} m but the ground outside is at {ground:.2f} m",
                          where=beyond, hint="move the level, grade the ground to the door, or add steps")
        elif to in rooms:
            gap = rooms[to]["poly"].distance(Point(p[0], p[1]))
            if gap > room["wall"] + rooms[to]["wall"] + 0.3 or rooms[to]["level"]["id"] != room["level"]["id"]:
                diag.error("door.dangling", f["id"], f"it should lead to '{to}', which is {gap:.1f} m away or on another level",
                           where=p, hint="move the door to a wall the two rooms share")
            else:
                edges[host].add(to)
                edges[to].add(host)
        else:
            diag.error("door.dangling", f["id"], f"it leads to '{to}', which is not a room")

    # ---- stairs: steps along an arc or a line, and a well through the floor they arrive at
    wells: dict = {}
    step_meshes = []
    ramp_meshes = []
    stair_zones: dict = {}
    for f in stairs:
        a, b = f.get("from"), f.get("to")
        if a not in rooms or b not in rooms:
            diag.error("stair.dangling", f["id"], "a stair needs `from` and `to` rooms")
            continue
        za, zb = rooms[a]["level"]["z"], rooms[b]["level"]["z"]
        if abs(zb - za) < 0.5:
            diag.error("stair.flat", f["id"], f"'{a}' and '{b}' are on the same level")
            continue
        edges[a].add(b)
        edges[b].add(a)
        n = max(3, int(round(abs(zb - za) / float(f.get("riser", 0.19)))))
        wide = float(f.get("width", 1.0))
        treads = []
        if "arc" in f:
            c, r = f["arc"]["center"], float(f["arc"]["radius"])
            a0, a1 = math.radians(f["arc"]["from_deg"]), math.radians(f["arc"]["to_deg"])
            for s in range(n):
                t0, t1 = a0 + (a1 - a0) * s / n, a0 + (a1 - a0) * (s + 1) / n
                ring = [(c[0] + (r - wide) * math.cos(t0), c[1] + (r - wide) * math.sin(t0)), (c[0] + r * math.cos(t0), c[1] + r * math.sin(t0)),
                        (c[0] + r * math.cos(t1), c[1] + r * math.sin(t1)), (c[0] + (r - wide) * math.cos(t1), c[1] + (r - wide) * math.sin(t1))]
                treads.append(Polygon(ring))
            going = abs(a1 - a0) * (r - wide / 2) / n
        else:
            p0, p1 = np.asarray(f["line"][0], dtype=float), np.asarray(f["line"][1], dtype=float)
            d = (p1 - p0) / n
            side = np.array([-d[1], d[0]]) / max(np.linalg.norm(d), 1e-9) * wide / 2
            for s in range(n):
                q0, q1 = p0 + d * s, p0 + d * (s + 1)
                treads.append(Polygon([q0 - side, q0 + side, q1 + side, q1 - side]))
            going = float(np.linalg.norm(d))
        rise = (zb - za) / n
        if going < 0.22 or abs(rise) > 0.21:
            diag.warn("stair.steep", f["id"], f"each step rises {abs(rise) * 100:.0f} cm over a going of {going * 100:.0f} cm",
                      hint="give the stair a longer run")
        for s, tread in enumerate(treads):      # a smooth ramp over the steps, for feet to walk on
            ring = np.asarray(tread.exterior.coords)[:4]
            zs = [za + rise * s, za + rise * s, za + rise * (s + 1), za + rise * (s + 1)]
            ramp_meshes.append((np.column_stack([ring, zs]), np.tile([0.0, 0.0, 1.0], (4, 1)), np.array([[0, 1, 2], [0, 2, 3]])))
        for s, tread in enumerate(treads):
            m = _prism(tread.buffer(0), za + rise * s - 0.12 if rise > 0 else za + rise * (s + 1) - 0.12, za + rise * (s + 1) if rise > 0 else za + rise * s)
            if m:
                step_meshes.append(m)
        for rid in (a, b):                      # a stair takes floor space at both ends
            stair_zones.setdefault(rid, []).append((f["id"], unary_union(treads).buffer(0.1)))
        upper = rooms[b] if zb > za else rooms[a]
        wells.setdefault(upper["feat"]["id"], []).append(unary_union(treads[1:]).buffer(0.12))
        outside_room = unary_union(treads).difference(rooms[a]["poly"].union(rooms[b]["poly"]).buffer(0.05))
        if outside_room.area > 0.05:
            diag.error("stair.through_wall", f["id"], f"{outside_room.area:.1f} m2 of it lies outside both rooms it joins")
    if step_meshes:
        out["meshes"].append({"id": "stairs", "material": "timber", "z0": 0.0, "parts": step_meshes, "collide": False})
        out["meshes"].append({"id": "stairs_ramp", "material": "timber", "z0": 0.0, "parts": ramp_meshes, "collide": True, "hidden": True})

    # ---- walls and slabs, level by level. Rooms that touch share one wall.
    slab = 0.25
    for lid, lv in levels.items():
        here = [r for r in rooms.values() if r["level"]["id"] == lid]
        if not here:
            continue
        z, top = float(lv["z"]), float(lv["z"]) + float(lv["height"])
        inner = unary_union([r["poly"] for r in here])
        outer = unary_union([r["poly"].buffer(r["wall"], join_style=2) for r in here])
        ring = outer.difference(inner)
        bands = sorted({z, top, *[c[1] for c in cuts.get(lid, [])], *[c[2] for c in cuts.get(lid, [])]})
        wall_parts = []
        for b0, b1 in zip(bands[:-1], bands[1:]):
            if b1 - b0 < 0.01 or b0 < z - 0.01 or b1 > top + 0.01:
                continue
            gone = [c[0] for c in cuts.get(lid, []) if c[1] <= b0 + 0.01 and c[2] >= b1 - 0.01]
            m = _prism(ring.difference(unary_union(gone)) if gone else ring, b0, b1)
            if m:
                wall_parts.append(m)
        out["meshes"].append({"id": f"walls_{lid}", "material": lv.get("wall", "plaster"), "z0": z, "parts": wall_parts})
        floors, roofs = [], []
        for r in here:
            foot = r["poly"].buffer(r["wall"], join_style=2)
            well = unary_union(wells.get(r["feat"]["id"], [])) if wells.get(r["feat"]["id"]) else None
            m = _prism(foot.difference(well) if well is not None else foot, z - slab, z)
            if m:
                floors.append(m)
            above = [o for o in rooms.values() if o["level"]["z"] > lv["z"] + 0.5 and o["poly"].intersection(r["poly"]).area > 0.5 * r["poly"].area]
            if not above:                      # nothing built over it: close it with a roof
                m = _prism(foot.buffer(0.3, join_style=2), top, top + 0.3)
                if m:
                    roofs.append(m)
        out["meshes"].append({"id": f"floors_{lid}", "material": lv.get("floor", "timber"), "z0": z - slab, "parts": floors})
        if roofs:
            out["meshes"].append({"id": f"roof_{lid}", "material": "roof", "z0": top, "parts": roofs})

    # ---- checks only a building has
    seen, todo = {"outside"}, ["outside"]
    graph = {**edges, "outside": {r for r, e in edges.items() if "outside" in e}}
    while todo:
        for nxt in graph.get(todo.pop(), ()):
            if nxt not in seen:
                seen.add(nxt)
                todo.append(nxt)
    for rid, r in rooms.items():
        if rid not in seen:
            diag.error("room.sealed", rid, "no chain of doors and stairs joins it to the outside",
                       where=r["poly"].centroid.coords[0], hint="add a door or a stair")
        module = r["kind"].get("module_m")
        if module:                              # would a kit of fixed-width wall pieces close this room?
            c = np.asarray(r["poly"].exterior.coords)
            spare = [float(l % module) for l in np.linalg.norm(np.diff(c, axis=0), axis=1)]
            worst = max(min(s, module - s) for s in spare)
            if worst > 0.06:
                diag.warn("kit.remainder", rid, f"with {module:g} m wall modules its walls leave up to {worst * 100:.0f} cm over",
                          where=r["poly"].centroid.coords[0], hint="resize the room to the module, or allow a filler piece")
    for inst in instances:
        rid = inst.get("in")
        if not rid:
            continue
        if rid not in rooms:
            diag.error("place.no_room", inst["id"], f"'{rid}' is not a room")
            continue
        foot = Polygon(inst["poly"])
        over = foot.difference(rooms[rid]["poly"]).area
        if over > 0.02:
            diag.error("place.in_wall", inst["id"], f"{over:.2f} m2 of its footprint is outside the room '{rid}'",
                       where=(inst["x"], inst["y"]), hint="move it clear of the wall")
        if inst.get("mount") is not None:       # on a wall: it must not hang across a door or window
            zi = inst["floor_z"]
            for cut, z0, z1 in cuts.get(rooms[rid]["level"]["id"], []):
                if foot.intersects(cut) and z0 - 0.05 < zi < z1 + 0.05:
                    diag.error("place.over_opening", inst["id"], "it hangs across a door or window", where=(inst["x"], inst["y"]),
                               hint="slide it along the wall, or mount it higher")
            continue
        if inst.get("atop"):
            continue
        for sid, zone in stair_zones.get(rid, []):
            if foot.intersects(zone):
                diag.error("place.on_stair", inst["id"], f"it stands on the stair '{sid}' or over its well",
                           where=(inst["x"], inst["y"]), hint="move it clear of the stair")
        for did, lid, zone in door_zones:
            if not world.pack["asset"][inst["asset"]]["solid"]:
                continue                        # a mat or a rug lies in a doorway without blocking it
            if lid == rooms[rid]["level"]["id"] and foot.intersects(zone):
                diag.error("place.blocks_door", inst["id"], f"it stands in the swing of the door '{did}'", where=(inst["x"], inst["y"]))

    # ---- a floor plan per level: the interior's minimap
    for lid in levels:
        here = {i: r for i, r in rooms.items() if r["level"]["id"] == lid}
        if not here:
            continue
        outer = unary_union([r["poly"].buffer(r["wall"], join_style=2) for r in here.values()])
        x0, y0, x1, y1 = outer.buffer(1.5).bounds
        s = 28.0
        img = Image.new("RGB", (int((x1 - x0) * s), int((y1 - y0) * s)), (245, 242, 235))
        d = ImageDraw.Draw(img)

        def px(pt):
            return ((pt[0] - x0) * s, (y1 - pt[1]) * s)

        for geom in (outer.geoms if outer.geom_type == "MultiPolygon" else [outer]):
            d.polygon([px(c) for c in geom.exterior.coords], fill=(60, 56, 52))
        for rid, r in here.items():
            d.polygon([px(c) for c in r["poly"].exterior.coords], fill=(226, 205, 170))
            cx, cy = px(r["poly"].centroid.coords[0])
            d.text((cx - 3 * len(rid), cy - 5), rid, fill=(70, 50, 30))
        for cut, _, _ in cuts.get(lid, []):
            d.polygon([px(c) for c in cut.exterior.coords], fill=(226, 205, 170))
        for rid in here:
            for w in wells.get(rid, []):
                for geom in (w.geoms if w.geom_type == "MultiPolygon" else [w]):
                    d.polygon([px(c) for c in geom.exterior.coords], outline=(120, 90, 60))
        for inst in instances:
            if inst.get("in") in here:
                d.polygon([px(c) for c in inst["poly"]], fill=(150, 120, 90), outline=(60, 40, 20))
        out["plans"][lid] = img
    out["rooms"] = {rid: {"level": r["level"]["id"], "area_m2": round(r["poly"].area, 1), "joins": sorted(edges[rid]),
                          "things": sum(1 for i in instances if i.get("in") == rid)} for rid, r in rooms.items()}
    for rid, r in out["rooms"].items():
        print(f"  room {rid:13s} {r['area_m2']:5.1f} m2  {r['things']:3d} things  {r['things'] / max(r['area_m2'], 0.1):.1f} per m2")
    return out
