"""What leaves the compiler: the files an engine and the review tooling load.

Owns `emit`, which writes the field rasters, the generated meshes, the manifest (`world.json`) with
the instance list, one file per layer, the budgets, the water measurements, the minimap and the
diagnostics. Also owns `bookmarks`, `budgets` and `water_bodies`, which work out what `emit` writes
and raise diagnostics while they do; the pipeline calls them before anything is written, and `emit`
itself raises none. The player-facing map is a separate output and lives in `playmap`.
"""
from __future__ import annotations

import json
import math
from pathlib import Path

import numpy as np
from contourpy import contour_generator
from PIL import Image, ImageDraw

from .world import World


def ground_colors(world: World) -> np.ndarray:
    cols = []
    for name in world.materials:
        g = world.pack["ground"].get(name, {})
        col = g.get("color", [1.0, 0.0, 1.0])
        if "texture" in g:
            px = np.asarray(Image.open(g["texture"]).convert("RGB").resize((8, 8)), dtype=np.float32) / 255.0
            col = ((px.reshape(-1, 3).mean(axis=0) ** 2.2) * np.asarray(g.get("tint", [1, 1, 1]))).tolist()
        cols.append(col)
    return np.asarray(cols, dtype=np.float32)


def bookmarks(world: World, instances: list[dict]) -> list[dict]:
    """A bookmark is a point feature the review tooling reads: an eye, a target, a lens."""
    g = world.grid
    height = world.field("height")
    water = world.fields.get("water")
    out = []
    for feat in world.features:
        if feat.get("kind") != "bookmark" or feat.get("id") not in world.shapes:
            continue
        ex, ey = feat["at"]
        look = feat.get("look", [ex, ey + 50.0])
        if isinstance(look, dict):
            shape = world.shapes.get(look.get("feature", ""))
            if shape is None or shape.path is None:
                world.diag.error("frame.unknown", feat["id"], f"`look` names '{look.get('feature')}', which is not a path")
                continue
            p, _, _ = shape.path.at(float(look.get("t", 0.5)))
            look = [float(p[0]), float(p[1])]
        ground = float(g.sample(height, [ex], [ey])[0])
        indoors = "floor_z" in feat
        if indoors:
            ground = float(feat["floor_z"])
        if feat.get("under") and "bed" in world.fields:      # stand beneath the ground, on the lowest floor
            ground = float(g.sample(world.fields["bed"], [ex], [ey])[0])
        if water is not None:
            ground = max(ground, float(g.sample(water, [ex], [ey])[0]))
        ez = ground + float(feat.get("eye", 1.7))
        tz = (ground if indoors else float(g.sample(height, [look[0]], [look[1]])[0])) + float(feat.get("look_height", 1.5))
        ts = np.linspace(0.02, 0.98, 96)
        rx, ry, rz = ex + (look[0] - ex) * ts, ey + (look[1] - ey) * ts, ez + (tz - ez) * ts
        blocked = np.nonzero(g.sample(height, rx, ry) > rz + 0.3)[0]
        if blocked.size and not feat.get("under") and not indoors and "cut_z" not in feat:
            k = int(blocked[0])
            world.diag.warn("bookmark.blocked", feat["id"],
                            f"terrain hides the target {math.hypot(rx[k] - ex, ry[k] - ey):.0f} m from the eye",
                            where=(rx[k], ry[k]), hint="raise `eye`, or move the bookmark")
        # anything standing in the first metres of the view
        view = np.array([look[0] - ex, look[1] - ey])
        view = view / max(float(np.linalg.norm(view)), 1e-6)
        crowd = []
        for inst in instances:
            asset = world.pack["asset"][inst["asset"]]
            radius = max(asset["size"][0], asset["size"][1]) * 0.5 * inst["scale"]
            if asset["size"][2] * inst["scale"] < float(feat.get("eye", 1.7)) * 0.8:
                continue
            rel = np.array([inst["x"] - ex, inst["y"] - ey])
            fwd = float(rel @ view)
            if -radius < fwd < 22.0 and abs(float(rel[0] * view[1] - rel[1] * view[0])) < radius + 0.6 + 0.25 * max(fwd, 0.0):
                crowd.append((fwd, inst))
        if crowd and not feat.get("under") and not indoors and "cut_z" not in feat:
            fwd, inst = min(crowd, key=lambda c: c[0])
            world.diag.warn("bookmark.crowded", feat["id"],
                            f"{len(crowd)} instance(s) stand in the first 22 m of the view; the nearest is a "
                            f"'{inst['asset']}' {max(fwd, 0.0):.0f} m ahead", where=(inst["x"], inst["y"]),
                            hint="move the bookmark, or give its kind an effect that clears vegetation")
        for inst in instances:
            if inst.get("explicit") and not indoors and math.hypot(inst["x"] - ex, inst["y"] - ey) < max(inst["footprint"]) * 0.5:
                world.diag.warn("bookmark.inside", feat["id"], f"the eye is inside the footprint of '{inst['id']}'",
                                where=(ex, ey))
        out.append({"id": feat["id"], "eye": [ex, ey, ez], "target": [look[0], look[1], tz],
                    "fov": float(feat.get("fov", 55.0)), "note": feat.get("note", ""), "cut_z": feat.get("cut_z")})
    return out


def budgets(world: World, instances: list[dict]) -> dict:
    """Static per-chunk counts. These are source numbers, not what an engine ends up drawing."""
    g = world.grid
    chunk = float(world.doc["world"].get("chunk", 64))
    limits = world.doc.get("budget", {})
    chunks: dict = {}
    for inst in instances:
        key = (int((inst["x"] - g.x0) // chunk), int((inst["y"] - g.y0) // chunk))
        c = chunks.setdefault(key, {"instances": 0, "source_tris": 0, "assets": set()})
        c["instances"] += 1
        c["source_tris"] += world.pack["asset"][inst["asset"]]["tris"]
        c["assets"].add(inst["asset"])
    out = []
    for (cx, cy), c in sorted(chunks.items()):
        row = {"chunk": [cx, cy], "instances": c["instances"], "source_tris": c["source_tris"],
               "unique_assets": len(c["assets"])}
        out.append(row)
        where = (g.x0 + (cx + 0.5) * chunk, g.y0 + (cy + 0.5) * chunk)
        for key, label in (("instances", "max_instances_per_chunk"), ("source_tris", "max_source_tris_per_chunk")):
            if label in limits and row[key] > limits[label]:
                world.diag.warn("budget.exceeded", f"chunk {cx},{cy}",
                                f"{key.replace('_', ' ')} {row[key]:,} exceeds the declared {limits[label]:,}", where=where)
    return {"chunk_m": chunk, "chunks": out,
            "total": {"instances": len(instances),
                      "source_tris": int(sum(world.pack["asset"][i["asset"]]["tris"] for i in instances))}}


def water_bodies(world: World) -> list[dict]:
    """Measurements an agent can steer by. Shoreline development is the limnological index
    D_L = L / (2 * sqrt(pi * A)): 1 for a circle, higher for a more intricate shore."""
    g = world.grid
    height, water = world.field("height"), world.fields.get("water")
    out = []
    if water is None:
        return out
    for feat in world.features:
        shape = world.shapes.get(feat.get("id"))
        if world.kinds.get(feat.get("kind", ""), {}).get("measure") != "water" or shape is None:
            continue
        fr = shape.frame(16.0)
        win = fr["win"]
        depth = np.where(fr["sd"] < 8.0, water[win] - height[win], -1.0)
        wet = depth > 0
        if not wet.any():
            world.diag.warn("water.dry", feat["id"], "it holds no water after compiling")
            continue
        area = float(wet.sum()) * g.res * g.res
        loops = [l for l in contour_generator(x=g.X[win][0], y=g.Y[win][:, 0], z=depth).lines(0.0) if len(l) > 6]
        shore = float(sum(np.linalg.norm(np.diff(l, axis=0), axis=1).sum() for l in loops))
        closed = [l for l in loops if np.allclose(l[0], l[-1])]
        row = {"id": feat["id"], "authority": "derived" if "flood" in feat else feat.get("authority", world.kinds[feat["kind"]].get("authority", "shaped")),
               "level_m": round(float(water[win][wet].max()), 2), "area_m2": round(area), "shore_m": round(shore),
               "shoreline_development": round(shore / (2.0 * math.sqrt(math.pi * area)), 2),
               "islands": max(0, len(closed) - 1), "max_depth_m": round(float(depth.max()), 2),
               "mean_depth_m": round(float(depth[wet].mean()), 2), "shallow_share": round(float((depth[wet] < 1.0).mean()), 2)}
        out.append(row)
    return out


def minimap(world: World, instances: list[dict], marks: list[dict], path: Path, size: int = 1024) -> None:
    g = world.grid
    height = world.field("height")
    cols = ground_colors(world)
    weights = world.field("ground")
    rgb = np.tensordot(weights.transpose(1, 2, 0), cols, axes=1) ** (1 / 2.2)
    gy, gx = np.gradient(height, g.res)
    shade = np.clip((1.0 - 0.9 * gx + 0.9 * gy) / np.sqrt(1 + gx * gx + gy * gy), 0.25, 1.6)
    rgb = rgb * (0.35 + 0.65 * shade[..., None])
    water = world.fields.get("water")
    if water is not None:
        depth = np.clip((water - height) / 4.0, 0, 1)
        wet = (water > height)[..., None]
        rgb = np.where(wet, np.array([0.30, 0.52, 0.62]) * (1 - 0.6 * depth[..., None]) + 0.1, rgb)
    img = Image.fromarray((np.clip(rgb, 0, 1)[::-1] * 255).astype(np.uint8)).resize((size, size), Image.LANCZOS)
    draw = ImageDraw.Draw(img)
    sx, sy = size / (g.x1 - g.x0), size / (g.y1 - g.y0)

    def px(x, y):
        return ((x - g.x0) * sx, size - (y - g.y0) * sy)

    for inst in instances:
        if inst.get("explicit"):
            draw.polygon([px(x, y) for x, y in inst["poly"]], fill=(214, 198, 170), outline=(60, 50, 40))
    for feat in world.features:
        style = world.kinds.get(feat.get("kind", ""), {}).get("map")
        shape = world.shapes.get(feat["id"])
        if style and shape is not None:
            col = tuple(int(c * 255) for c in style.get("color", [1, 1, 1]))
            if shape.type == "path":
                draw.line([px(x, y) for x, y in shape.path.pts[::8]], fill=col, width=int(style.get("width", 2)))
            elif shape.type == "region":
                draw.polygon([px(x, y) for x, y in shape.polygon], outline=col)
            else:
                x, y = px(*shape.point)
                draw.ellipse([x - 5, y - 5, x + 5, y + 5], fill=col, outline=(20, 20, 20))
                draw.text((x + 8, y - 6), feat["id"], fill=(255, 255, 255), stroke_width=2, stroke_fill=(0, 0, 0))
    for m in marks:
        x, y = px(m["eye"][0], m["eye"][1])
        tx, ty = px(m["target"][0], m["target"][1])
        ang = math.atan2(ty - y, tx - x)
        draw.line([x, y, x + 26 * math.cos(ang), y + 26 * math.sin(ang)], fill=(255, 235, 120), width=2)
        draw.ellipse([x - 4, y - 4, x + 4, y + 4], fill=(255, 235, 120), outline=(0, 0, 0))
        draw.text((x + 7, y + 4), m["id"], fill=(255, 245, 190), stroke_width=2, stroke_fill=(0, 0, 0))
    img.save(path)


def report_changes(world: World, instances: list[dict], out: Path) -> None:
    """What this build placed, by name, and how that differs from the build it replaces. Written as
    `placed.json` and `changes.json`, and said in one line per rule, so a change can be read before
    it is accepted. A thing is the same thing when its name is the same."""
    now = {}
    for n, i in enumerate(instances):
        name = i.get("id") or f"{i.get('by', 'thing')}@{i['x']:.2f},{i['y']:.2f}"
        now[name] = {"asset": i["asset"], "x": round(i["x"], 2), "y": round(i["y"], 2), "by": i.get("by", "hand"),
                     **({"edited": True} if i.get("edited") else {}), **({"locked": i["locked"]} if i.get("locked") else {})}
    before_file = out / "placed.json"
    before = json.loads(before_file.read_text()) if before_file.is_file() else None
    before_file.write_text(json.dumps(now, indent=0))
    if before is None:
        return
    added = [k for k in now if k not in before]
    removed = [k for k in before if k not in now]
    moved = [k for k in now if k in before and (abs(now[k]["x"] - before[k]["x"]) > 0.05 or abs(now[k]["y"] - before[k]["y"]) > 0.05)]
    swapped = [k for k in now if k in before and now[k]["asset"] != before[k]["asset"]]
    per: dict = {}
    for kind, names, src in (("added", added, now), ("removed", removed, before), ("moved", moved, now), ("swapped", swapped, now)):
        for k in names:
            per.setdefault(src[k]["by"], {"added": 0, "removed": 0, "moved": 0, "swapped": 0})[kind] += 1
    touched = [now[k] for k in added + moved] + [before[k] for k in removed]
    box = [min(t["x"] for t in touched), min(t["y"] for t in touched), max(t["x"] for t in touched), max(t["y"] for t in touched)] if touched else None
    (out / "changes.json").write_text(json.dumps({"added": added, "removed": removed, "moved": moved, "swapped": swapped,
                                                  "by": per, "within": box, "unchanged": len(now) - len(added) - len(moved)}, indent=0))
    if not touched and not swapped:
        print(f"  changes: none; {len(now)} things as before")
    for by, c in sorted(per.items()):
        print(f"  changes: {by}: +{c['added']} -{c['removed']} moved {c['moved']} swapped {c['swapped']}")
    if box:
        print(f"  changes: all within x {box[0]:.0f}..{box[2]:.0f}, y {box[1]:.0f}..{box[3]:.0f}; {len(now) - len(added) - len(moved)} things untouched")


def emit(world: World, instances: list[dict], out: Path, volumes: list, built: dict,
         marks: list[dict], budget: dict, bodies: list[dict]) -> None:
    """Write every output file into `out`. Nothing is checked here: the bookmarks, budgets and water
    measurements arrive worked out, so the diagnostics are complete before the first file is written."""
    out.mkdir(parents=True, exist_ok=True)
    report_changes(world, instances, out)
    g = world.grid
    doc = world.doc
    fields = {}
    vol_rows = []
    for vol in volumes or []:
        vol["verts"].tofile(out / f"vol_{vol['id']}.verts.f32")
        vol["normals"].tofile(out / f"vol_{vol['id']}.normals.f32")
        vol["faces"].tofile(out / f"vol_{vol['id']}.faces.u32")
        vol["light"].tofile(out / f"vol_{vol['id']}.light.f32")
        vol_rows.append({k: v for k, v in vol.items() if k not in ("verts", "normals", "faces", "light")})
    mesh_rows = []
    for m in (built or {}).get("meshes", []):
        if not m["parts"]:
            continue
        v = np.vstack([p[0] for p in m["parts"]])
        n = np.vstack([p[1] for p in m["parts"]])
        offs = np.cumsum([0] + [len(p[0]) for p in m["parts"]])[:-1]
        f = np.vstack([p[2] + o for p, o in zip(m["parts"], offs)])
        v.astype("<f4").tofile(out / f"mesh_{m['id']}.verts.f32")
        n.astype("<f4").tofile(out / f"mesh_{m['id']}.normals.f32")
        f.astype("<u4").tofile(out / f"mesh_{m['id']}.faces.u32")
        mesh_rows.append({"id": m["id"], "material": m["material"], "z0": m["z0"], "tris": int(len(f)),
                          "collide": m.get("collide", True), "hidden": m.get("hidden", False)})
    for lid, img in (built or {}).get("plans", {}).items():
        img.save(out / f"plan_{lid}.png")
    extra = [n for n in ("hole", "bed") if n in world.fields]
    for name in ["height"] + [n for n in doc.get("export", {}).get("fields", []) if n != "height"] + extra:
        if name in world.fields and world.fields[name].ndim == 2:
            world.fields[name].astype("<f4").tofile(out / f"{name}.f32")
            arr = world.fields[name]
            fields[name] = {"file": f"{name}.f32", "min": float(arr.min()), "max": float(arr.max())}
    weights = world.field("ground")
    planes = np.zeros((8, g.ny, g.nx), dtype=np.float32)
    planes[:min(8, len(world.materials))] = weights[:8]
    total = np.maximum(planes.sum(axis=0, keepdims=True), 1e-6)
    packed = np.clip(planes / total * 255.0 + 0.5, 0, 255).astype(np.uint8)
    packed[:4].transpose(1, 2, 0).tofile(out / "ground_0.rgba8")
    packed[4:].transpose(1, 2, 0).tofile(out / "ground_1.rgba8")

    by_asset: dict = {}          # asset -> chunk -> rows: the unit an engine can cull and stream
    for inst in instances:
        cell = "%d,%d" % (int((inst["x"] - g.x0) // budget["chunk_m"]), int((inst["y"] - g.y0) // budget["chunk_m"]))
        by_asset.setdefault(inst["asset"], {}).setdefault(cell, []).append(
            [round(inst["x"], 3), round(inst["y"], 3), round(inst["z"], 3), round(inst["yaw"], 2),
             round(inst["scale"], 3)] + [round(float(t), 3) for t in inst["tint"]])
    features = []
    for feat in world.features:
        shape = world.shapes.get(feat["id"])
        if shape is None or feat.get("of_instance"):
            continue
        entry = {k: v for k, v in feat.items() if k not in ("path", "path3", "region", "ellipse", "smooth", "_mask")}
        entry["shape"] = shape.type
        if doc.get("export", {}).get("feature_attrs"):      # for steps: the kind's defaults under the feature's own numbers
            entry["attrs"] = world.attrs(feat)
        if shape.type == "path":
            entry["points"] = np.round(shape.path.pts[::8], 2).tolist()
        elif shape.type == "region":
            entry["points"] = np.round(np.asarray(shape.polygon), 2).tolist()
        features.append(entry)
    cols = ground_colors(world)
    manifest = {
        "name": doc["world"].get("name", "world"),
        "frame": {"extent": [g.x0, g.y0, g.x1, g.y1], "resolution": g.res, "nx": g.nx, "ny": g.ny,
                  "chunk": budget["chunk_m"], "axes": "x east, y north, z up, metres"},
        "fields": fields,
        "ground": [{"name": n, "color": cols[k].tolist(), **{a: b for a, b in world.pack["ground"].get(n, {}).items()
                                                               if a != "color"}}
                   for k, n in enumerate(world.materials)],
        "assets": {aid: world.pack["asset"][aid] for aid in by_asset},
        "instances": by_asset,
        "explicit": [{"id": i["id"], "asset": i["asset"], "pos": [i["x"], i["y"], i["z"]], "yaw": i["yaw"],
                      "relief": round(i.get("relief", 0.0), 2), **({"in": i["in"]} if i.get("in") else {}),
                      **({"attrs": i["attrs"]} if i.get("attrs") else {}),
                      **({"mounts": world.instance_mounts(i["asset"], i["x"], i["y"], i["yaw"], i.get("scale", 1.0))}
                         if world.pack["asset"].get(i["asset"], {}).get("mount") else {})} for i in instances if i.get("explicit")],
        "features": features,
        "volumes": vol_rows,
        "meshes": mesh_rows,
        "lights": (built or {}).get("lights", []),
        "rooms": (built or {}).get("rooms", {}),
        "surfaces": world.pack.get("surface", {}),
        "bookmarks": marks,
        "look": doc.get("look", {}),
        "params": world.params,
    }
    for lid in doc.get("_layers", []):      # each layer leaves the compiler as its own file
        rows = [{"id": i["id"], "asset": i["asset"], "pos": [round(i["x"], 3), round(i["y"], 3), round(i["z"], 3)],
                 "yaw": round(i["yaw"], 2), **({"attrs": i["attrs"]} if i.get("attrs") else {})}
                for i in instances if i.get("layer") == lid]
        feats = [f for f in features if f.get("_layer") == lid]
        (out / f"layer_{lid}.json").write_text(json.dumps({"layer": lid, "instances": rows, "features": feats}))
        print(f"  layer {lid:10s} {len(rows):4d} placed, {len(feats):3d} features")
    (out / "world.json").write_text(json.dumps(manifest))
    (out / "report.json").write_text(json.dumps({"water": bodies}, indent=1))
    for b in bodies:
        print(f"  water {b['id']} ({b['authority']}): level {b['level_m']} m, {b['area_m2']:,} m2, shore {b['shore_m']} m, "
              f"shoreline development {b['shoreline_development']}, islands {b['islands']}, "
              f"depth max {b['max_depth_m']} mean {b['mean_depth_m']} m, shallow {int(b['shallow_share'] * 100)}%")
    (out / "budgets.json").write_text(json.dumps(budget, indent=1))
    minimap(world, instances, marks, out / "minimap.png")
    (out / "diagnostics.json").write_text(json.dumps(world.diag.items, indent=1))
