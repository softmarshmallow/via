"""Placement: turning one instance spec into one placed instance.

Owns `resolve_position` (where a spec stands, from `at` or from another feature's frame),
`place_explicit` (the spec's asset, host, floor, footprint and the feature its kind makes of it) and
`finish` (grounding instances on the resolved height). Specs arrive here already plain: sets, runs
and the other compact forms are lowered by `producers`.
"""
from __future__ import annotations

import math

import numpy as np
from shapely.geometry import Polygon as _Poly

from .geometry import ellipse_polygon, rect_polygon
from .world import World

# Spec keys the compiler reads itself when it places an instance. Anything else the author wrote on
# the spec is not the compiler's business and travels to the engine untouched, as the instance's attrs.
RESERVED_SPEC_KEYS = (
    "id", "asset", "at", "on", "yaw", "face", "turn", "scale", "tint", "kind", "in", "atop", "offset", "mount", "drop",
    "against", "set", "margin", "group", "beside", "_layer")

# Spec keys that say where and as what the instance stands. The feature an instance's kind makes of
# it has its own shape (the footprint), so these do not carry over to it.
TRANSFORM_SPEC_KEYS = ("asset", "at", "on", "face", "yaw", "scale", "tint")


def resolve_position(world: World, spec: dict, what: str):
    """World position and yaw from `at`, or from `on` in a feature's local frame."""
    if "on" in spec:
        on = spec["on"]
        got = world.frame_at(on, what)
        if got is None:
            return None
        p, tan, left = got
        offset = float(on.get("offset", 0.0))
        pos = p + left * offset + tan * float(on.get("slide", 0.0))
        face = spec.get("face", "toward")
        if face == "in":            # for a region: look into it, back to its edge
            yaw = math.degrees(math.atan2(-left[1], -left[0]))
        elif face == "toward":
            d = -left * (1.0 if offset >= 0 else -1.0)
            yaw = math.degrees(math.atan2(d[1], d[0]))
        elif face == "along":
            yaw = math.degrees(math.atan2(tan[1], tan[0]))
        else:
            yaw = float(face)
        return float(pos[0]), float(pos[1]), yaw + float(spec.get("turn", 0.0))
    if "at" in spec:
        return float(spec["at"][0]), float(spec["at"][1]), float(spec.get("yaw", 0.0))
    world.diag.error("instance.no_position", what, "an instance needs `at` or `on`")
    return None


def place_explicit(world: World, specs: list[dict], deferred: bool = False) -> list[dict]:
    """Resolve plain instance specs. One with a `kind` also becomes a region feature: its footprint.
    Instances placed on a derived feature wait (`deferred`) until that feature exists, so the same
    list of specs is passed twice: once before the terrain is resolved and once after."""
    out = []
    for n, spec in enumerate(specs):
        iid = spec.get("id", f"instance_{n}")
        waits = spec.get("on", {}).get("feature") in world.derived_ids
        if waits != deferred:
            continue
        if waits and spec.get("kind"):
            host = next(f for f in world.features if f.get("id") == spec["on"]["feature"])
            source = host["flood"].get("field", "height")
            if any(e.get("field") == source for e in world.kinds.get(spec["kind"], {}).get("effect", [])):
                world.diag.error("authority.cycle", iid,
                                 f"it stands on '{host['id']}', which is derived from '{source}', and its kind "
                                 f"'{spec['kind']}' reshapes '{source}'", hint="drop the kind, or anchor the host feature")
                continue
        asset = world.pack["asset"].get(spec.get("asset", ""))
        if asset is None:
            world.diag.error("asset.unknown", iid, f"the pack has no asset '{spec.get('asset')}'",
                             hint=f"pack: {world.pack['path']}")
            continue
        floor_z = None
        if "atop" in spec:          # on top of another instance, in that instance's own frame
            host = next((o for o in world.placed if o["id"] == spec["atop"]), None)
            if host is None:
                world.diag.error("place.no_host", iid, f"`atop` names '{spec['atop']}', which is not an instance placed before it")
                continue
            th = math.radians(host["yaw"])
            dx, dy = spec.get("offset", [0.0, 0.0])
            pos = (host["x"] + math.sin(th) * dx + math.cos(th) * dy, host["y"] - math.cos(th) * dx + math.sin(th) * dy,
                   host["yaw"] + float(spec.get("turn", 0.0)))
            floor_z = host.get("floor_z", 0.0) + world.pack["asset"][host["asset"]]["size"][2] * host["scale"]
            if abs(dx) > host["footprint"][0] / 2 or abs(dy) > host["footprint"][1] / 2:
                world.diag.warn("place.off_surface", iid, f"its place on '{host['id']}' is past that surface's edge")
            spec = {**spec, "in": host.get("in")}
        elif "beside" in spec:      # on the ground, in another instance's frame: right of it, in front of it
            host = next((o for o in world.placed if o["id"] == spec["beside"]), None)
            if host is None:
                world.diag.error("place.no_host", iid, f"`beside` names '{spec['beside']}', which is not an instance placed before it")
                continue
            th = math.radians(host["yaw"])
            dx, dy = spec.get("offset", [0.0, 0.0])
            pos = (host["x"] + math.sin(th) * dx + math.cos(th) * dy, host["y"] - math.cos(th) * dx + math.sin(th) * dy,
                   host["yaw"] + float(spec.get("turn", 0.0)))
        else:
            if spec.get("against") and "on" in spec:      # back to the wall: stand in by half its own depth
                depth = asset["footprint"][1] * float(spec.get("scale", 1.0))
                spec = {**spec, "on": {**spec["on"], "offset": -(depth / 2 + 0.03)}, "face": spec.get("face", "in")}
                host = world.shapes.get(spec["on"].get("feature", ""))
                if host is not None and host.polygon is not None:
                    room = _Poly(host.polygon)
                    width = asset["footprint"][0] * float(spec.get("scale", 1.0))
                    for _ in range(25):
                        trial = resolve_position(world, spec, iid)
                        if trial is None or room.contains(_Poly(rect_polygon(trial[:2], [width, depth], trial[2]))):
                            break
                        spec["on"]["offset"] -= 0.02
            pos = resolve_position(world, spec, iid)
        if pos is None:
            continue
        x, y, yaw = pos
        if floor_z is None and spec.get("in"):
            room = next((f for f in world.features if f.get("id") == spec["in"]), None)
            level = next((lv for lv in world.doc.get("level", []) if room and lv["id"] == room.get("level")), None)
            floor_z = float(level["z"]) if level else None
        scale = float(spec.get("scale", 1.0))
        foot = [asset["footprint"][0] * scale, asset["footprint"][1] * scale]
        inst = {"id": iid, "asset": spec["asset"], "x": x, "y": y, "yaw": yaw, "scale": scale,
                "tint": spec.get("tint", [1.0, 1.0, 1.0]),
                "poly": ellipse_polygon((x, y), foot, 12) if asset.get("round") else rect_polygon((x, y), foot, yaw),
                "explicit": True, "kind": spec.get("kind"), "footprint": foot, "in": spec.get("in")}
        mount = spec.get("mount", 0.0)
        if mount == "ceiling":      # a constraint, not a height: its top meets the ceiling of the room it is in
            room = next((f for f in world.features if f.get("id") == spec.get("in")), None)
            level = next((lv for lv in world.doc.get("level", []) if room and lv["id"] == room.get("level")), None)
            if level is None or "height" not in level:
                world.diag.error("place.no_ceiling", iid, "it hangs from the ceiling, but it is in no room with a level that has a height",
                                 hint="give it `in = \"<room>\"`, or a height in metres")
                mount = 0.0
            else:
                mount = float(level["height"]) - asset["size"][2] * scale - float(spec.get("drop", 0.0))
        if floor_z is not None:
            inst["floor_z"] = floor_z + float(mount)      # mounted: this far above the floor
        free = {k: v for k, v in spec.items() if k not in RESERVED_SPEC_KEYS}
        if free:                    # anything else the author wrote travels to the engine untouched
            inst["attrs"] = free
        if "group" in spec:
            inst["group"] = spec["group"]
        if "_layer" in spec:
            inst["layer"] = spec["_layer"]
        if "mount" in spec:
            inst["mount"] = float(mount)
        if "atop" in spec:
            inst["atop"] = spec["atop"]
        world.placed.append(inst)
        if not world.grid.inside(x, y):
            world.diag.error("instance.outside", iid, "the instance is outside the world extent", where=(x, y))
            continue
        if spec.get("kind"):
            margin = float(world.kinds.get(spec["kind"], {}).get("defaults", {}).get("margin", 0.0))
            margin = float(spec.get("margin", margin))
            feat = {k: v for k, v in spec.items() if k not in TRANSFORM_SPEC_KEYS}
            feat.update({"id": iid, "kind": spec["kind"], "of_instance": True,
                         "region": rect_polygon((x, y), [foot[0] + 2 * margin, foot[1] + 2 * margin], yaw)})
            world.add_feature(feat)
        out.append(inst)
    return out


def finish(world: World, instances: list[dict]) -> None:
    """Ground every instance on the resolved height field."""
    g = world.grid
    height = world.field("height")
    if not instances:
        return
    xs = np.array([i["x"] for i in instances])
    ys = np.array([i["y"] for i in instances])
    zs = g.sample(height, xs, ys)
    for inst, z in zip(instances, zs):
        asset = world.pack["asset"][inst["asset"]]
        inst["z"] = (inst["floor_z"] if "floor_z" in inst else float(z)) - asset["sink"] * inst["scale"]
