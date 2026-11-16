"""Writes a tiny synthetic build with kind surfaces, for the loader's M5 test (test_floor.gd).

    python3 engines/godot/tests/make_floor_fixture.py OUT_DIR

Written without the engine, by spec/build.md's layout: a flat terrain at 0 m (`fields/height.npy`),
its cut file (6.5, `fields/height.cut.npy`), and four generated meshes (section 10,
`meshes/<feature>.<surface>.glb`), each a flat or sloping top on the 1 m lattice:

  hall.floor    x 12..24, y 8..16 at 3 m, material `boards`; the terrain under it is cut (96 quads)
  stair.tread   x 4..12, y 11..13, rising from 0 m at x = 4 to 3 m at x = 12 (20.6 degrees)
  step.floor    x 28..34, y 10..16 at 0.25 m, no material (a step the walker climbs)
  plinth.floor  x 35..39, y 10..16 at 0.6 m (higher than the walker steps)

and two bookmarks: `on_floor` stands on hall.floor (its `at` at 3 m) and `below_floor` stands in the
cut under it (its `at` on the terrain, 0 m, so its eye at 1.7 m is under the floor). Digests are not
computed (the loader does not read them). Python's standard library only.
"""

import json
import struct
import sys
from pathlib import Path

NX, NY = 41, 25   # posts: extent [0, 0, 40, 24] at 1 m


def npy(path, descr, shape, data):
    header = "{'descr': '%s', 'fortran_order': False, 'shape': (%s), }" % (
        descr, ", ".join(str(s) for s in shape) + ("," if len(shape) == 1 else ""))
    pad = 64 - (10 + len(header) + 1) % 64
    header = header + " " * (pad % 64) + "\n"
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(b"\x93NUMPY\x01\x00" + struct.pack("<H", len(header)) + header.encode() + data)


def glb(path, feature, surface, sw, verts, tris, material):
    """One mesh, one primitive, indexed triangles with positions and normals, one node translated
    to the mesh's south-west corner `sw` (world x, y, z); `verts` world (x, y, z) relative to it.
    glTF axes: (X, Y, Z) = (x, z, -y)."""
    pos = [(x, z, -y) for x, y, z in verts]
    nrm = [None] * len(verts)
    for a, b, c in tris:   # flat normals, accumulated per vertex
        pa, pb, pc = pos[a], pos[b], pos[c]
        u = [pb[i] - pa[i] for i in range(3)]
        v = [pc[i] - pa[i] for i in range(3)]
        n = (u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0])
        for k in (a, b, c):
            nrm[k] = n
    nrm = [tuple(c / (sum(q * q for q in n) ** 0.5) for c in n) for n in nrm]
    pbytes = b"".join(struct.pack("<3f", *p) for p in pos)
    nbytes = b"".join(struct.pack("<3f", *n) for n in nrm)
    ibytes = b"".join(struct.pack("<3I", *t) for t in tris)
    buf = pbytes + nbytes + ibytes
    lo = [min(p[i] for p in pos) for i in range(3)]
    hi = [max(p[i] for p in pos) for i in range(3)]
    doc = {
        "asset": {"generator": "via", "version": "2.0"},
        "scene": 0,
        "scenes": [{"nodes": [0]}],
        "nodes": [{"mesh": 0, "name": f"{feature}.{surface}", "translation": [sw[0], sw[2], -sw[1]]}],
        "meshes": [{"name": f"{feature}.{surface}", "extras": {"via": {"feature": feature, "surface": surface}},
                    "primitives": [dict({"attributes": {"POSITION": 0, "NORMAL": 1}, "indices": 2, "mode": 4},
                                        **({"material": 0} if material else {}))]}],
        "accessors": [
            {"bufferView": 0, "componentType": 5126, "count": len(pos), "type": "VEC3", "min": lo, "max": hi},
            {"bufferView": 1, "componentType": 5126, "count": len(pos), "type": "VEC3"},
            {"bufferView": 2, "componentType": 5125, "count": 3 * len(tris), "type": "SCALAR"},
        ],
        "bufferViews": [
            {"buffer": 0, "byteOffset": 0, "byteLength": len(pbytes), "target": 34962},
            {"buffer": 0, "byteOffset": len(pbytes), "byteLength": len(nbytes), "target": 34962},
            {"buffer": 0, "byteOffset": len(pbytes) + len(nbytes), "byteLength": len(ibytes), "target": 34963},
        ],
        "buffers": [{"byteLength": len(buf)}],
    }
    if material:
        doc["materials"] = [{"name": material[0], "pbrMetallicRoughness": {"baseColorFactor": list(material[1]) + [1.0]}}]
    js = json.dumps(doc, separators=(",", ":")).encode()
    js += b" " * (-len(js) % 4)
    buf += b"\0" * (-len(buf) % 4)
    body = struct.pack("<I4s", len(js), b"JSON") + js + struct.pack("<I4s", len(buf), b"BIN\0") + buf
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(struct.pack("<4sII", b"glTF", 2, 12 + len(body)) + body)
    return len(tris)


def grid(x0, y0, x1, y1, z):
    """Posts x0..x1, y0..y1 (1 m) relative to (x0, y0, z(x0, y0)); each quad split south-west to
    north-east, counter-clockwise seen from above (glTF's front face)."""
    cols, rows = x1 - x0 + 1, y1 - y0 + 1
    z0 = z(x0, y0)
    verts = [(i, j, z(x0 + i, y0 + j) - z0) for j in range(rows) for i in range(cols)]
    tris = []
    for j in range(rows - 1):
        for i in range(cols - 1):
            sw = j * cols + i
            se, nw, ne = sw + 1, sw + cols, sw + cols + 1
            tris += [(sw, se, ne), (sw, ne, nw)]
    return (x0, y0, z0), verts, tris


def main(out):
    out = Path(out)
    if out.exists() and any(out.iterdir()) and not (out / "manifest.json").exists():
        sys.exit(f"{out} is not empty and holds no build")
    npy(out / "fields/height.npy", "<f4", [NY, NX], struct.pack(f"<{NX * NY}f", *([0.0] * (NX * NY))))
    cut = bytearray((NX - 1) * (NY - 1))
    for j in range(8, 16):
        for i in range(12, 24):
            cut[j * (NX - 1) + i] = 1
    npy(out / "fields/height.cut.npy", "|u1", [NY - 1, NX - 1], bytes(cut))
    for col, descr in [("x", "<i4"), ("y", "<i4"), ("z", "<f4"), ("face", "<i4"), ("scale", "<f4"), ("tint", "<f4")]:
        npy(out / f"instances/{col}.npy", descr, [0], b"")
    (out / "instances/names.txt").write_text("")
    boards = ("boards", (0.55, 0.36, 0.2))
    meshes = []
    for feature, surface, box, z, material, collides in [
        ("hall", "floor", (12, 8, 24, 16), lambda x, y: 3.0, boards, True),
        ("stair", "tread", (4, 11, 12, 13), lambda x, y: 3.0 * (x - 4) / 8.0, None, True),
        ("step", "floor", (28, 10, 34, 16), lambda x, y: 0.25, None, True),
        ("plinth", "floor", (35, 10, 39, 16), lambda x, y: 0.6, None, True),
    ]:
        sw, verts, tris = grid(*box, z)
        file = f"meshes/{feature}.{surface}.glb"
        n = glb(out / file, feature, surface, sw, verts, tris, material)
        meshes.append({"collides": collides, "feature": feature, "file": file, "material": material[0] if material else None,
                       "name": f"{feature}.{surface}", "surface": surface, "triangles": n})
    features = {"features": [
        {"at": [18, 12, 3.0], "attrs": {"eye_height": 1.7, "toward": {"at": [22, 12, 3.0], "name": "hall_east"}},
         "authority": "derived", "kind": "bookmark", "layer": "world", "name": "on_floor", "shape": "point"},
        {"at": [16, 10, 0.0], "attrs": {"eye_height": 1.7, "toward": {"at": [16, 14, 0.0], "name": "hall_north"}},
         "authority": "derived", "kind": "bookmark", "layer": "world", "name": "below_floor", "shape": "point"},
    ], "footprints": []}
    (out / "features.json").write_text(json.dumps(features, sort_keys=True, separators=(",", ":")) + "\n")
    manifest = {
        "assets": {},
        "chunks": [{"at": [0, 0], "count": 0, "instances": {}, "posts": [0, 0, NX - 1, NY - 1], "triangles": 2 * (NX - 1) * (NY - 1), "weights": {}}],
        "engine": {"epoch": 1, "name": "via"},
        "features": "features.json",
        "fields": {"height": {"cut": "fields/height.cut.npy", "file": "fields/height.npy", "max": 0, "min": 0, "role": "surface",
                              "tags": [], "taxon": None, "type": "scalar", "unit": "m"}},
        "format": "via-build-0",
        "frame": {"axes": "x east, y north, z up", "bearing": "degrees clockwise from north", "extent": [0, 0, NX - 1, NY - 1],
                  "posts": [NX, NY], "resolution": 1, "unit": "m"},
        "id": "0" * 64,
        "instances": {"count": 0, "face": "instances/face.npy", "names": "instances/names.txt", "scale": "instances/scale.npy",
                      "tint": "instances/tint.npy", "x": "instances/x.npy", "y": "instances/y.npy", "z": "instances/z.npy"},
        "locations": {"pack_root": None},
        "materials": {"boards": {"colour": list(boards[1]), "params": {}}},
        "meshes": meshes,
        "outputs": {"bookmark": ["below_floor", "on_floor"]},
        "world": "floor_fixture",
    }
    (out / "manifest.json").write_text(json.dumps(manifest, sort_keys=True, separators=(",", ":")) + "\n")


if __name__ == "__main__":
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    main(sys.argv[1])
