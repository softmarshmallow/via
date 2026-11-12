"""The asset pack: ground materials and assets, with every fact the compiler needs about them.

Owns `load_pack`, `probe_glb` (bounds and triangle count of a model, read without loading it) and
the triangle counts of the built-in primitives. The world source only ever names assets by ID; this
is the one place an ID becomes a file and declared facts.
"""
from __future__ import annotations

import json
import struct
import tomllib
from pathlib import Path

import numpy as np

from .diagnostics import Diagnostics

PRIMITIVE_TRIS = {"box": 12, "cone": 64, "sphere": 320, "tree": 160, "blob": 320}


def _node_matrix(node: dict) -> np.ndarray:
    if "matrix" in node:
        return np.asarray(node["matrix"], dtype=np.float64).reshape(4, 4).T
    m = np.eye(4)
    if "rotation" in node:
        x, y, z, w = node["rotation"]
        m[:3, :3] = [[1 - 2 * (y * y + z * z), 2 * (x * y - z * w), 2 * (x * z + y * w)],
                     [2 * (x * y + z * w), 1 - 2 * (x * x + z * z), 2 * (y * z - x * w)],
                     [2 * (x * z - y * w), 2 * (y * z + x * w), 1 - 2 * (x * x + y * y)]]
    if "scale" in node:
        m[:3, :3] = m[:3, :3] @ np.diag(node["scale"])
    if "translation" in node:
        m[:3, 3] = node["translation"]
    return m


def probe_glb(path: Path) -> dict:
    """Bounds (y up, as authored) and triangle count of a binary glTF, read from its JSON chunk only."""
    blob = path.read_bytes()
    length = struct.unpack_from("<I", blob, 12)[0]
    gltf = json.loads(blob[20:20 + length])
    lo, hi, tris = np.full(3, 1e30), np.full(3, -1e30), 0

    def visit(index: int, parent: np.ndarray) -> None:
        nonlocal lo, hi, tris
        node = gltf["nodes"][index]
        mat = parent @ _node_matrix(node)
        if "mesh" in node:
            for prim in gltf["meshes"][node["mesh"]]["primitives"]:
                acc = gltf["accessors"][prim["attributes"]["POSITION"]]
                a, b = acc["min"], acc["max"]
                corners = np.array([[x, y, z, 1.0] for x in (a[0], b[0]) for y in (a[1], b[1]) for z in (a[2], b[2])])
                world = (mat @ corners.T).T[:, :3]
                lo, hi = np.minimum(lo, world.min(axis=0)), np.maximum(hi, world.max(axis=0))
                count = gltf["accessors"][prim["indices"]]["count"] if "indices" in prim else acc["count"]
                tris += count // 3
        for child in node.get("children", []):
            visit(child, mat)

    scene = gltf["scenes"][gltf.get("scene", 0)]
    for root in scene["nodes"]:
        visit(root, np.eye(4))
    return {"min": lo.tolist(), "max": hi.tolist(), "tris": int(tris)}


def load_pack(path: Path, diag: Diagnostics) -> dict:
    """Resolve a pack into ground materials and assets with every fact the compiler needs."""
    raw = tomllib.loads(path.read_text())
    root = Path(raw.get("root", path.parent))
    pack = {"path": str(path), "ground": {}, "asset": {}, "surface": raw.get("surface", {})}
    for name, g in raw.get("ground", {}).items():
        entry = {"color": g.get("color", [0.5, 0.5, 0.5]), "scale_m": g.get("scale_m", 4.0),
                 "tint": g.get("tint", [1.0, 1.0, 1.0])}
        if "texture" in g:
            tex = (root / g["texture"]).resolve()
            if tex.exists():
                entry["texture"] = str(tex)
            else:
                diag.error("pack.missing_file", f"ground.{name}", f"texture not found: {g['texture']}")
        pack["ground"][name] = entry
    for aid, a in raw.get("asset", {}).items():
        entry = {
            "tags": a.get("tags", []), **({"mount": a["mount"]} if "mount" in a else {}), "taxon": a.get("taxon", ""), "labels": a.get("labels", []), "avoid": a.get("avoid", []), "sink": float(a.get("sink", 0.0)),
            "max_relief": a.get("max_relief"), "clearance": float(a.get("clearance", 0.0)),
            "color": a.get("color", [0.7, 0.7, 0.7]), **({"roof": a["roof"]} if "roof" in a else {}), "solid": a.get("solid", True),
        }
        if "source" in a:
            src = (root / a["source"]).resolve()
            if not src.exists():
                diag.error("pack.missing_file", aid, f"model not found: {a['source']}")
                continue
            info = probe_glb(src)
            raw_size = np.asarray(info["max"]) - np.asarray(info["min"])      # x, y(up), z
            if "fit_height" in a:
                scale = float(a["fit_height"]) / max(raw_size[1], 1e-6)
            elif "fit_width" in a:
                scale = float(a["fit_width"]) / max(raw_size[0], 1e-6)
            else:
                scale = float(a.get("scale", 1.0))
            entry.update({
                "source": str(src), "scale": scale, "lift": -info["min"][1] * scale, "tris": info["tris"],
                "size": [raw_size[0] * scale, raw_size[2] * scale, raw_size[1] * scale],   # across, depth, height
            })
            if "albedo" in a:
                entry["albedo"] = str((root / a["albedo"]).resolve())
        elif "recipe" in a:
            # an engine-side generator: the compiler cannot open it, so every fact must be declared
            if "size" not in a or "tris" not in a:
                diag.error("pack.undeclared", aid, "a recipe asset cannot be measured; declare `size` and `tris`")
                continue
            rec = dict(a["recipe"])
            for key in ("script", "atlas"):
                if key in rec:
                    rec[key] = str((root / rec[key]).resolve())
                    if not Path(rec[key]).exists():
                        diag.error("pack.missing_file", aid, f"recipe {key} not found: {a['recipe'][key]}")
            entry.update({"recipe": rec, "scale": 1.0, "lift": 0.0, "tris": int(a["tris"]),
                          "size": [float(v) for v in a["size"]]})
        else:
            prim = a.get("primitive", "box")
            entry["round"] = a.get("round", prim in ("cylinder", "blob", "tower", "cone", "tree"))
            entry.update({"primitive": prim, "scale": 1.0, "lift": 0.0, "tris": PRIMITIVE_TRIS.get(prim, 12),
                          "size": [float(v) for v in a["size"]]})
        entry["footprint"] = [float(v) for v in a.get("footprint", entry["size"][:2])]
        entry["module_m"] = a.get("module_m")      # the distance a kit piece repeats at, which is not its bounds
        pack["asset"][aid] = entry
    return pack
