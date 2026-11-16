"""The glTF reader's suite (spec/build.md, Conformance, "Other suites": glTF reader).

    python -m via_conformance.gltf_check FILE.glb [--build DIR] [--source engines/gltf/export.py]

A structural check of the glTF 2.0 specification's MUSTs that a writer of this kind can break, in
place of the Khronos glTF Validator while no copy of it is pinned in the runner (build.md 15.3 asks
for one; none could be had offline). Standard library only. With `--build`, also the suite's row:
instance nodes equal the manifest's `instances.count`. With `--source`, the source check: no import
outside Python's standard library and nothing from `engines/godot/`. Prints one JSON line of what it
counted, every fault on standard error; exit 0 none, 1 any, 2 a bad argument.
"""

import argparse
import ast
import json
import math
import os
import struct
import sys

COMPONENTS = {5120: ("b", 1), 5121: ("B", 1), 5122: ("h", 2), 5123: ("H", 2), 5125: ("I", 4), 5126: ("f", 4)}
TYPES = {"SCALAR": 1, "VEC2": 2, "VEC3": 3, "VEC4": 4, "MAT2": 4, "MAT3": 9, "MAT4": 16}


def read_glb(path, faults):
    with open(path, "rb") as f:
        data = f.read()
    if len(data) < 20:
        faults.append("GLB shorter than its header")
        return None, b""
    magic, version, length = struct.unpack_from("<4sII", data, 0)
    if magic != b"glTF" or version != 2:
        faults.append(f"GLB magic {magic!r} version {version}")
    if length != len(data):
        faults.append(f"GLB length {length}, file {len(data)}")
    jlen, jtype = struct.unpack_from("<II", data, 12)
    if jtype != 0x4E4F534A:
        faults.append("first chunk is not JSON")
    if jlen % 4:
        faults.append("JSON chunk not padded to 4 bytes")
    doc = json.loads(data[20 : 20 + jlen])
    at = 20 + jlen
    bin_ = b""
    if at < len(data):
        blen, btype = struct.unpack_from("<II", data, at)
        if btype != 0x004E4942:
            faults.append("second chunk is not BIN")
        if blen % 4:
            faults.append("BIN chunk not padded to 4 bytes")
        bin_ = data[at + 8 : at + 8 + blen]
        if at + 8 + blen != len(data):
            faults.append("bytes after the BIN chunk")
    return doc, bin_


def check(doc, bin_, faults):
    def ref(where, key, coll, value):
        n = len(doc.get(coll, []))
        if not isinstance(value, int) or not 0 <= value < n:
            faults.append(f"{where}.{key} = {value!r}: no {coll}[{value}]")
            return False
        return True

    if doc.get("asset", {}).get("version") != "2.0":
        faults.append("asset.version is not 2.0")
    for key in ("scenes", "nodes", "meshes", "materials", "accessors", "bufferViews", "buffers"):
        if key in doc and (not isinstance(doc[key], list) or not doc[key]):
            faults.append(f"{key}: present and empty (a glTF array must hold at least one item)")
    # Buffers: the GLB's first buffer has no uri and fits the BIN chunk.
    for k, b in enumerate(doc.get("buffers", [])):
        if k == 0 and "uri" not in b:
            if not len(bin_) - 3 <= b["byteLength"] <= len(bin_):
                faults.append(f"buffers[0].byteLength {b['byteLength']}, BIN chunk {len(bin_)}")
        elif "uri" not in b:
            faults.append(f"buffers[{k}] has no uri")
    for k, v in enumerate(doc.get("bufferViews", [])):
        if ref(f"bufferViews[{k}]", "buffer", "buffers", v.get("buffer")):
            end = v.get("byteOffset", 0) + v["byteLength"]
            if end > doc["buffers"][v["buffer"]]["byteLength"]:
                faults.append(f"bufferViews[{k}] ends at {end}, past its buffer")
        if "byteStride" in v and not (4 <= v["byteStride"] <= 252 and v["byteStride"] % 4 == 0):
            faults.append(f"bufferViews[{k}].byteStride {v['byteStride']}")
    # Accessors: fit their view, aligned, min and max equal to the data.
    values = {}
    for k, a in enumerate(doc.get("accessors", [])):
        where = f"accessors[{k}]"
        if a.get("componentType") not in COMPONENTS or a.get("type") not in TYPES:
            faults.append(f"{where}: componentType {a.get('componentType')} type {a.get('type')}")
            continue
        if a.get("count", 0) < 1:
            faults.append(f"{where}.count < 1")
            continue
        if "bufferView" not in a or not ref(where, "bufferView", "bufferViews", a["bufferView"]):
            continue
        code, size = COMPONENTS[a["componentType"]]
        n = TYPES[a["type"]]
        v = doc["bufferViews"][a["bufferView"]]
        stride = v.get("byteStride", size * n)
        off = a.get("byteOffset", 0)
        if (off + v.get("byteOffset", 0)) % size:
            faults.append(f"{where}: offset not a multiple of its component size")
        need = off + stride * (a["count"] - 1) + size * n
        if need > v["byteLength"]:
            faults.append(f"{where}: needs {need} bytes, view holds {v['byteLength']}")
            continue
        if v["buffer"] != 0 or "uri" in doc["buffers"][0]:
            continue
        base = v.get("byteOffset", 0) + off
        vals = []
        for e in range(a["count"]):
            vals.append(struct.unpack_from("<" + code * n, bin_, base + e * stride))
        values[k] = vals
        if code == "f" and any(not math.isfinite(x) for el in vals for x in el):
            faults.append(f"{where}: a value not finite")
        if "min" in a or "max" in a:
            mn = [min(el[c] for el in vals) for c in range(n)]
            mx = [max(el[c] for el in vals) for c in range(n)]
            if a.get("min") != mn and not _close(a.get("min"), mn):
                faults.append(f"{where}.min {a.get('min')} != data {mn}")
            if a.get("max") != mx and not _close(a.get("max"), mx):
                faults.append(f"{where}.max {a.get('max')} != data {mx}")
    # Materials.
    for k, mat in enumerate(doc.get("materials", [])):
        c = mat.get("pbrMetallicRoughness", {}).get("baseColorFactor", [1, 1, 1, 1])
        if len(c) != 4 or any(not 0 <= x <= 1 for x in c):
            faults.append(f"materials[{k}].baseColorFactor {c}")
    # Meshes.
    triangles = 0
    for k, mesh in enumerate(doc.get("meshes", [])):
        if not mesh.get("primitives"):
            faults.append(f"meshes[{k}] has no primitive")
        for p, prim in enumerate(mesh.get("primitives", [])):
            where = f"meshes[{k}].primitives[{p}]"
            mode = prim.get("mode", 4)
            if mode not in range(7):
                faults.append(f"{where}.mode {mode}")
            counts = set()
            for name, acc in prim.get("attributes", {}).items():
                if not ref(where, name, "accessors", acc):
                    continue
                a = doc["accessors"][acc]
                counts.add(a["count"])
                if name == "POSITION":
                    if a["type"] != "VEC3" or a["componentType"] != 5126:
                        faults.append(f"{where}: POSITION not float VEC3")
                    if "min" not in a or "max" not in a:
                        faults.append(f"{where}: POSITION without min and max")
                if name == "NORMAL" and acc in values:
                    for el in values[acc]:
                        if abs(math.sqrt(sum(x * x for x in el)) - 1) > 0.0005:
                            faults.append(f"{where}: NORMAL not unit length: {el}")
                            break
                if name.startswith("COLOR_") and acc in values and a["componentType"] == 5126:
                    if any(not 0 <= x <= 1 for el in values[acc] for x in el):
                        faults.append(f"{where}: {name} outside [0, 1]")
            if "POSITION" not in prim.get("attributes", {}):
                faults.append(f"{where}: no POSITION")
            if len(counts) > 1:
                faults.append(f"{where}: attributes of different counts {sorted(counts)}")
            vcount = min(counts) if counts else 0
            if "material" in prim:
                ref(where, "material", "materials", prim["material"])
            n = vcount
            if "indices" in prim and ref(where, "indices", "accessors", prim["indices"]):
                a = doc["accessors"][prim["indices"]]
                if a["type"] != "SCALAR" or a["componentType"] not in (5121, 5123, 5125):
                    faults.append(f"{where}: indices not unsigned SCALAR")
                if "byteStride" in doc["bufferViews"][a["bufferView"]]:
                    faults.append(f"{where}: indices in a view with byteStride")
                restart = {5121: 255, 5123: 65535, 5125: 4294967295}.get(a["componentType"])
                if prim["indices"] in values:
                    for (x,) in values[prim["indices"]]:
                        if x >= vcount or x == restart:
                            faults.append(f"{where}: index {x} with {vcount} vertices")
                            break
                n = a["count"]
            if mode == 4:
                if n % 3:
                    faults.append(f"{where}: TRIANGLES with {n} vertices")
                triangles += n // 3
            if mode in (1,) and n % 2:
                faults.append(f"{where}: LINES with {n} vertices")
            if mode in (2, 3) and n < 2:
                faults.append(f"{where}: a line of {n} vertices")
    # Nodes: references, one parent each, no cycle, a matrix or TRS, unit rotations.
    parents = {}
    for k, node in enumerate(doc.get("nodes", [])):
        where = f"nodes[{k}]"
        if "mesh" in node:
            ref(where, "mesh", "meshes", node["mesh"])
        for c in node.get("children", []):
            if ref(where, "children", "nodes", c):
                if c in parents:
                    faults.append(f"nodes[{c}] has two parents")
                parents[c] = k
        if "matrix" in node and any(key in node for key in ("translation", "rotation", "scale")):
            faults.append(f"{where}: matrix with TRS")
        if "matrix" in node and len(node["matrix"]) != 16:
            faults.append(f"{where}.matrix not 16 numbers")
        r = node.get("rotation")
        if r is not None and abs(math.sqrt(sum(x * x for x in r)) - 1) > 0.000005:
            faults.append(f"{where}.rotation not unit: {r}")
    for k in range(len(doc.get("nodes", []))):
        seen, at = set(), k
        while at in parents:
            if at in seen:
                faults.append(f"nodes[{k}]: a cycle")
                break
            seen.add(at)
            at = parents[at]
    for s, scene in enumerate(doc.get("scenes", [])):
        for r in scene.get("nodes", []):
            if ref(f"scenes[{s}]", "nodes", "nodes", r) and r in parents:
                faults.append(f"scenes[{s}] root nodes[{r}] has a parent")
    if "scene" in doc:
        ref("", "scene", "scenes", doc["scene"])
    return triangles


def _close(a, b):
    if not isinstance(a, list) or len(a) != len(b):
        return False
    return all(abs(x - y) <= 1e-6 * max(1.0, abs(y)) for x, y in zip(a, b))



def _stdlib() -> frozenset:
    """Python's standard library's top-level names: `sys.stdlib_module_names` from 3.10; on 3.9,
    which export.py supports, the modules the reader and its tools import, listed here."""
    names = getattr(sys, "stdlib_module_names", None)
    if names is not None:
        return frozenset(names)
    return frozenset({
        "__future__", "argparse", "array", "ast", "base64", "binascii", "collections", "copy",
        "dataclasses", "functools", "hashlib", "io", "itertools", "json", "math", "os", "pathlib",
        "re", "shutil", "struct", "subprocess", "sys", "tempfile", "textwrap", "typing", "zlib",
    })

def source_faults(path):
    faults = []
    with open(path, encoding="utf-8") as f:
        text = f.read()
    for node in ast.walk(ast.parse(text)):
        mods = []
        if isinstance(node, ast.Import):
            mods = [a.name for a in node.names]
        elif isinstance(node, ast.ImportFrom):
            mods = [node.module or ""] if node.level == 0 else ["." + (node.module or "")]
        for mod in mods:
            if mod.split(".")[0] not in _stdlib():
                faults.append(f"{path}: imports `{mod}`, not in the standard library")
    if "godot" in text.lower():
        faults.append(f"{path}: names the Godot loader")
    return faults


def main(argv=None):
    p = argparse.ArgumentParser(prog="gltf_check")
    p.add_argument("glb")
    p.add_argument("--build")
    p.add_argument("--source")
    try:
        args = p.parse_args(argv)
    except SystemExit as e:
        return 2 if e.code else 0
    faults = []
    doc, bin_ = read_glb(args.glb, faults)
    out = {}
    if doc is not None:
        out["triangles"] = check(doc, bin_, faults)
        out["nodes"] = len(doc.get("nodes", []))
        inst = sum(1 for n in doc.get("nodes", []) if "instance" in n.get("extras", {}).get("via", {}))
        out["instance_nodes"] = inst
        if args.build:
            with open(os.path.join(args.build, "manifest.json"), "rb") as f:
                count = json.load(f)["instances"]["count"]
            out["instances_count"] = count
            if inst != count:
                faults.append(f"{inst} instance nodes, instances.count {count}")
    if args.source:
        faults.extend(source_faults(args.source))
    for f in faults[:50]:
        print(f"gltf_check: {f}", file=sys.stderr)
    out["faults"] = len(faults)
    print(json.dumps(out, sort_keys=True))
    return 1 if faults else 0


if __name__ == "__main__":
    sys.exit(main())
