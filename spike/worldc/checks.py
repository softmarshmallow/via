"""Structural checks that are always on: seams between what was placed and everything else.

Owns `check_explicit` (overlapping footprints, uneven ground under an asset, standing where a field
says to keep clear) and `check_joins` (a path that says it joins another must reach it). Checks that
belong to one domain live with it (interiors, volumes, parts); opt-in checks live in `lints`.
"""
from __future__ import annotations

import numpy as np

from .geometry import polygons_overlap
from .world import World


def check_explicit(world: World, explicit: list[dict]) -> None:
    """Seams between explicit instances and everything else."""
    g = world.grid
    height = world.field("height")
    assets = world.pack["asset"]
    for a_i, a in enumerate(explicit):
        for b in explicit[a_i + 1:]:
            if abs(a.get("floor_z", 0.0) - b.get("floor_z", 0.0)) > 0.05 or a.get("in") != b.get("in"):
                continue
            if a.get("group") and a.get("group") == b.get("group"):
                continue            # pieces of one kit run are meant to meet
            if assets[a["asset"]]["solid"] and assets[b["asset"]]["solid"] and polygons_overlap(a["poly"], b["poly"]):
                world.diag.error("place.overlap", a["id"], f"its footprint overlaps '{b['id']}'",
                                 where=(a["x"], a["y"]), hint="move one of them, or mark an asset `solid = false`")
    for inst in explicit:
        if "floor_z" in inst:
            continue
        asset = assets[inst["asset"]]
        poly = np.asarray(inst["poly"])
        pts = np.vstack([poly, poly.mean(axis=0, keepdims=True), (poly + np.roll(poly, 1, axis=0)) / 2])
        h = g.sample(height, pts[:, 0], pts[:, 1])
        relief = float(h.max() - h.min())
        inst["relief"] = relief
        if asset["max_relief"] is not None and relief > float(asset["max_relief"]):
            world.diag.warn("seam.relief", inst["id"],
                            f"the ground under its footprint varies by {relief:.1f} m; the asset tolerates "
                            f"{float(asset['max_relief']):.1f} m", where=(inst["x"], inst["y"]),
                            hint="give the instance a kind that levels the ground, or move it")
        for name in asset["avoid"]:
            if name in world.doc.get("field", {}) and not inst.get("kind"):
                if float(g.sample(world.field(name), [inst["x"]], [inst["y"]])[0]) > 0.5:
                    world.diag.warn("place.avoid", inst["id"], f"it stands where '{name}' says to keep clear",
                                    where=(inst["x"], inst["y"]))


def check_joins(world: World) -> None:
    """A path that declares it joins another feature must have an end on it."""
    for feat in world.features:
        shape = world.shapes.get(feat.get("id"))
        for target in feat.get("joins", []):
            other = world.shapes.get(target)
            if shape is None or shape.path is None or other is None or other.path is None:
                world.diag.error("frame.unknown", feat["id"], f"`joins` needs two paths; '{target}' is not one")
                continue
            ends = shape.path.pts[[0, -1]]
            gaps = other.path.tree.query(ends)[0]
            reach = 0.5 * float(world.attrs(world.shapes[target].feat).get("width", 2.0)) + 1.0
            if gaps.min() > reach:
                k = int(gaps.argmin())
                world.diag.warn("seam.junction", feat["id"],
                                f"it declares it joins '{target}', but its nearest end is {gaps.min():.0f} m away",
                                where=ends[k], hint="move the end onto the other path")
