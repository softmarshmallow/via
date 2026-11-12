"""The player-facing map, as data first.

The compiler does not paint a map. It emits what any map is made from: a class per cell, masks, a
plain base picture, the things to mark, and the transform from world to map. Style is a separate,
small layer on top (a painter or an image model), and labels and icons are always drawn from data.
"""
from __future__ import annotations

import hashlib
import json
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw

from .world import World

CLASSES = ["deep", "shallow", "sand", "grass", "forest", "rock", "paving", "road", "building"]
PALETTE = {"deep": (40, 70, 120), "shallow": (90, 140, 180), "sand": (225, 205, 150), "grass": (150, 185, 110),
           "forest": (85, 130, 80), "rock": (140, 135, 130), "paving": (205, 195, 175), "road": (170, 130, 90),
           "building": (90, 60, 50)}
MATERIAL_CLASS = {"grass": "grass", "meadow": "grass", "dry_grass": "grass", "forest": "forest", "sand": "sand",
                  "shore": "sand", "rock": "rock", "scree": "rock", "paving": "paving", "path": "road", "snow": "rock"}


def emit_map(world: World, instances: list[dict], out: Path, tiles: int = 8) -> None:
    g = world.grid
    out.mkdir(parents=True, exist_ok=True)
    weights = world.field("ground")
    lead = np.argmax(weights, axis=0)
    cls = np.zeros(lead.shape, dtype=np.uint8)
    for k, name in enumerate(world.materials):
        cls[lead == k] = CLASSES.index(MATERIAL_CLASS.get(name, "grass"))
    height = world.field("height")
    water = world.fields.get("water")
    if water is not None:
        depth = water - height
        cls[depth > 0] = CLASSES.index("shallow")
        cls[depth > 2.5] = CLASSES.index("deep")
    img = Image.fromarray(cls[::-1])          # north up
    draw = ImageDraw.Draw(img)

    def px(x, y):
        return ((x - g.x0) / g.res, g.ny - 1 - (y - g.y0) / g.res)

    footprints = []
    for inst in instances:
        asset = world.pack["asset"][inst["asset"]]
        if inst.get("explicit") and asset.get("solid", True) and max(inst.get("footprint", [0, 0])) >= 4.0 and inst.get("poly"):
            draw.polygon([px(x, y) for x, y in inst["poly"]], fill=CLASSES.index("building"))
            footprints.append({"id": inst["id"], "polygon": [[round(x, 2), round(y, 2)] for x, y in inst["poly"]]})
    cls_up = np.asarray(img)
    img.save(out / "class_index.png")
    rgb = np.zeros(cls_up.shape + (3,), dtype=np.uint8)
    for k, name in enumerate(CLASSES):
        rgb[cls_up == k] = PALETTE[name]
    Image.fromarray(rgb).save(out / "class.png")
    # the plain base: classes with relief, nothing else. This is what a painter or a model starts from.
    gy, gx = np.gradient(height[::-1], g.res)
    shade = np.clip(0.75 + 0.5 * (-gx + (-gy)) / np.sqrt(1 + gx * gx + gy * gy), 0.55, 1.2)
    wet = cls_up <= 1
    base = rgb.astype(np.float32) * np.where(wet, 1.0, shade)[..., None]
    Image.fromarray(np.clip(base, 0, 255).astype(np.uint8)).save(out / "base.png")

    icons, labels = [], []
    for feat in world.features:
        if feat.get("kind") == "poi" and "at" in feat:
            icons.append({"id": feat["id"], "type": "poi", "pos": [round(feat["at"][0], 1), round(feat["at"][1], 1)], "label": feat.get("label", "")})
    for inst in instances:
        if inst.get("explicit") and "landmark" in world.pack["asset"][inst["asset"]].get("tags", []):
            icons.append({"id": inst["id"], "type": "landmark", "pos": [round(inst["x"], 1), round(inst["y"], 1)], "label": inst["id"].split(".")[-1].replace("_", " ").title()})
    for part in world.doc.get("part", []):
        if "at" in part and "footprint" in part:
            f, a = part["footprint"], part["at"]
            labels.append({"text": part["id"].title(), "pos": [a[0] + (f[0] + f[2]) / 2, a[1] + (f[1] + f[3]) / 2], "class": "region"})
    # which stretch of the map each tile covers, and a fingerprint of what is in it
    th, tw = cls_up.shape[0] // tiles, cls_up.shape[1] // tiles
    manifest = {f"{r},{c}": hashlib.sha1(cls_up[r * th:(r + 1) * th, c * tw:(c + 1) * tw].tobytes()).hexdigest()[:12]
                for r in range(tiles) for c in range(tiles)}
    (out / "map.json").write_text(json.dumps({
        "transform": {"extent": [g.x0, g.y0, g.x1, g.y1], "width_px": int(cls_up.shape[1]), "height_px": int(cls_up.shape[0]),
                      "px_per_m": 1.0 / g.res, "north": "up"},
        "classes": CLASSES, "palette": PALETTE, "icons": icons, "labels": labels, "footprints": footprints,
        "tiles": {"grid": tiles, "hash": manifest}}, indent=1))
    print(f"  map: {cls_up.shape[1]}x{cls_up.shape[0]} px at {1 / g.res:g} px per m, {len(icons)} icons, {len(labels)} labels, "
          f"{len(footprints)} footprints, {tiles * tiles} tiles")
