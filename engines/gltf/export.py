"""The glTF export: the second reader of a build directory (spec/build.md 15.3).

    python3 engines/gltf/export.py BUILD --out FILE.glb [--pack DIR]

Writes the whole build as one glTF 2.0 binary scene. Python's standard library only. It reads the
build directory and the pack files its manifest names, nothing else: no world text, and nothing of
any engine's loader. It knows no output name. Exit 0 written, 1 a build it cannot read (the reason on
standard error), 2 a bad argument.
"""

import argparse
import base64
import json
import math
import os
import struct
import sys
from array import array

DEFAULT_COLOUR = [0.8, 0.8, 0.8]  # assets.md 2.4's default colour
LITTLE = sys.byteorder == "little"


class Refused(Exception):
    pass


# ------------------------------------------------------------------------------ array files (3.2)


def read_npy(path):
    """An array file of build.md 3.2: returns (shape, flat array)."""
    with open(path, "rb") as f:
        data = f.read()
    if data[:6] != b"\x93NUMPY" or data[6] != 1:
        raise Refused(f"{path}: not an array file of format 1.0")
    (hlen,) = struct.unpack_from("<H", data, 8)
    header = data[10 : 10 + hlen].decode("latin-1")
    descr = header.split("'descr':")[1].split("'")[1]
    fortran = header.split("'fortran_order':")[1].strip().startswith("True")
    shape_text = header.split("'shape':")[1].split("(")[1].split(")")[0]
    shape = tuple(int(s) for s in shape_text.split(",") if s.strip())
    if fortran:
        raise Refused(f"{path}: Fortran order")
    code = {"<f4": "f", "<i4": "i", "|u1": "B"}.get(descr)
    if code is None:
        raise Refused(f"{path}: descr {descr} is not one of build.md 3.2's")
    out = array(code)
    out.frombytes(data[10 + hlen :])
    if code != "B" and not LITTLE:
        out.byteswap()
    n = 1
    for s in shape:
        n *= s
    if len(out) != n:
        raise Refused(f"{path}: {len(out)} values for shape {shape}")
    return shape, out


# ------------------------------------------------------------------------------ the glTF writer


class Gltf:
    """One glTF document with one binary buffer, written as .glb."""

    def __init__(self):
        self.doc = {
            "asset": {"version": "2.0", "generator": "via engines/gltf"},
            "scene": 0,
            "scenes": [{"nodes": []}],
            "nodes": [],
            "meshes": [],
            "materials": [],
            "accessors": [],
            "bufferViews": [],
            "buffers": [],
        }
        self.bin = bytearray()

    def view(self, data, target=None, stride=None):
        while len(self.bin) % 4:
            self.bin.append(0)
        v = {"buffer": 0, "byteOffset": len(self.bin), "byteLength": len(data)}
        if target is not None:
            v["target"] = target
        if stride is not None:
            v["byteStride"] = stride
        self.bin.extend(data)
        self.doc["bufferViews"].append(v)
        return len(self.doc["bufferViews"]) - 1

    def accessor(self, view, ctype, count, kind, mn=None, mx=None, offset=0, normalized=False):
        a = {"bufferView": view, "componentType": ctype, "count": count, "type": kind}
        if offset:
            a["byteOffset"] = offset
        if normalized:
            a["normalized"] = True
        if mn is not None:
            a["min"], a["max"] = mn, mx
        self.doc["accessors"].append(a)
        return len(self.doc["accessors"]) - 1

    def vec3(self, values, with_bounds=False):
        """values: flat list of floats, 3 per element. Returns an accessor index."""
        data = array("f", values)
        if not LITTLE:
            data.byteswap()
        count = len(values) // 3
        mn = mx = None
        if with_bounds:
            # Bounds of the stored single-precision values, as the validator reads them.
            stored = array("f", values)
            mn = [min(stored[k::3]) for k in range(3)]
            mx = [max(stored[k::3]) for k in range(3)]
        return self.accessor(self.view(data.tobytes(), 34962), 5126, count, "VEC3", mn, mx)

    def indices(self, values, vertex_count):
        if vertex_count <= 65535:
            data, ctype = array("H", values), 5123
        else:
            data, ctype = array("I", values), 5125
        if not LITTLE:
            data.byteswap()
        return self.accessor(self.view(data.tobytes(), 34963), ctype, len(values), "SCALAR")

    def material(self, name, colour, double_sided=False):
        m = {
            "name": name,
            "pbrMetallicRoughness": {
                "baseColorFactor": [colour[0], colour[1], colour[2], 1.0],
                "metallicFactor": 0.0,
                "roughnessFactor": 1.0,
            },
        }
        if double_sided:
            m["doubleSided"] = True
        self.doc["materials"].append(m)
        return len(self.doc["materials"]) - 1

    def mesh(self, name, primitives, extras=None):
        m = {"name": name, "primitives": primitives}
        if extras:
            m["extras"] = extras
        self.doc["meshes"].append(m)
        return len(self.doc["meshes"]) - 1

    def node(self, name, parent=None, **keys):
        n = {"name": name}
        n.update({k: v for k, v in keys.items() if v is not None})
        self.doc["nodes"].append(n)
        i = len(self.doc["nodes"]) - 1
        if parent is None:
            self.doc["scenes"][0]["nodes"].append(i)
        else:
            self.doc["nodes"][parent].setdefault("children", []).append(i)
        return i

    def write(self, path):
        for key in ["meshes", "materials", "accessors", "bufferViews"]:
            if not self.doc[key]:
                del self.doc[key]
        while len(self.bin) % 4:
            self.bin.append(0)
        if self.bin:
            self.doc["buffers"] = [{"byteLength": len(self.bin)}]
        else:
            del self.doc["buffers"]
        text = json.dumps(self.doc, separators=(",", ":"), allow_nan=False).encode("utf-8")
        while len(text) % 4:
            text += b" "
        total = 12 + 8 + len(text) + (8 + len(self.bin) if self.bin else 0)
        with open(path, "wb") as f:
            f.write(struct.pack("<4sII", b"glTF", 2, total))
            f.write(struct.pack("<II", len(text), 0x4E4F534A))
            f.write(text)
            if self.bin:
                f.write(struct.pack("<II", len(self.bin), 0x004E4942))
                f.write(self.bin)


def gl(x, y, z):
    """World (x east, y north, z up) to glTF (+Y up): (x, z, -y), a rotation (build.md 10)."""
    return (x, z, -y + 0.0)


# ------------------------------------------------------------------------------ reading a glTF file


def read_gltf(path):
    """A model or generated mesh file: (json, [buffer bytes])."""
    with open(path, "rb") as f:
        data = f.read()
    buffers = []
    if data[:4] == b"glTF":
        _, _, total = struct.unpack_from("<4sII", data, 0)
        jlen, jtype = struct.unpack_from("<II", data, 12)
        doc = json.loads(data[20 : 20 + jlen])
        at = 20 + jlen
        glb_bin = b""
        if at + 8 <= total:
            blen, btype = struct.unpack_from("<II", data, at)
            if btype == 0x004E4942:
                glb_bin = data[at + 8 : at + 8 + blen]
        for k, b in enumerate(doc.get("buffers", [])):
            buffers.append(glb_bin if k == 0 and "uri" not in b else _uri(path, b["uri"]))
    else:
        doc = json.loads(data)
        for b in doc.get("buffers", []):
            buffers.append(_uri(path, b["uri"]))
    return doc, buffers


def _uri(path, uri):
    if uri.startswith("data:"):
        return base64.b64decode(uri.split(",", 1)[1])
    with open(os.path.join(os.path.dirname(path), uri.replace("%20", " ")), "rb") as f:
        return f.read()


class Copier:
    """Copies meshes (and their materials, less textures) from one source file into the output."""

    def __init__(self, out, src, buffers, default_material=None):
        self.out, self.src, self.buffers = out, src, buffers
        self.default_material = default_material
        self.views, self.accessors, self.materials = {}, {}, {}

    def _view(self, i):
        if i not in self.views:
            v = self.src["bufferViews"][i]
            off = v.get("byteOffset", 0)
            data = self.buffers[v["buffer"]][off : off + v["byteLength"]]
            self.views[i] = self.out.view(data, v.get("target"), v.get("byteStride"))
        return self.views[i]

    def accessor(self, i):
        if i not in self.accessors:
            a = dict(self.src["accessors"][i])
            if "sparse" in a or "bufferView" not in a:
                raise Refused("a sparse or view-less accessor is not copied")
            a["bufferView"] = self._view(a["bufferView"])
            a.pop("name", None)
            self.out.doc["accessors"].append(a)
            self.accessors[i] = len(self.out.doc["accessors"]) - 1
        return self.accessors[i]

    def material(self, i, tint=1.0):
        if (i, tint) not in self.materials:
            m = self.src["materials"][i]
            pbr = m.get("pbrMetallicRoughness", {})
            c = pbr.get("baseColorFactor", [1, 1, 1, 1])
            new = {
                "name": m.get("name", f"material_{i}"),
                "pbrMetallicRoughness": {
                    "baseColorFactor": [min(1.0, c[0] * tint), min(1.0, c[1] * tint), min(1.0, c[2] * tint), c[3]],
                    "metallicFactor": pbr.get("metallicFactor", 1.0),
                    "roughnessFactor": pbr.get("roughnessFactor", 1.0),
                },
            }
            for key in ("doubleSided", "alphaMode", "alphaCutoff", "emissiveFactor"):
                if key in m:
                    new[key] = m[key]
            self.out.doc["materials"].append(new)
            self.materials[(i, tint)] = len(self.out.doc["materials"]) - 1
        return self.materials[(i, tint)]

    def mesh(self, i, tint=1.0, extras=None):
        m = self.src["meshes"][i]
        prims = []
        for p in m["primitives"]:
            q = {"attributes": {k: self.accessor(v) for k, v in p["attributes"].items()
                                if not k.startswith("TEXCOORD") and not k.startswith("JOINTS")
                                and not k.startswith("WEIGHTS") and k != "TANGENT"}}
            if "indices" in p:
                q["indices"] = self.accessor(p["indices"])
            if "mode" in p:
                q["mode"] = p["mode"]
            if "material" in p:
                q["material"] = self.material(p["material"], tint)
            elif self.default_material is not None:
                q["material"] = self.default_material
            prims.append(q)
        return self.out.mesh(m.get("name", f"mesh_{i}"), prims, extras)

    def nodes(self, i, parent, tint, meshes):
        """Copy node i and its children under parent; meshes caches (mesh, tint) -> index."""
        n = self.src["nodes"][i]
        keys = {k: n[k] for k in ("matrix", "translation", "rotation", "scale") if k in n}
        if "mesh" in n:
            if (n["mesh"], tint) not in meshes:
                meshes[(n["mesh"], tint)] = self.mesh(n["mesh"], tint)
            keys["mesh"] = meshes[(n["mesh"], tint)]
        j = self.out.node(n.get("name", f"node_{i}"), parent, **keys)
        for c in n.get("children", []):
            self.nodes(c, j, tint, meshes)
        return j

    def roots(self):
        scenes = self.src.get("scenes")
        if scenes:
            return scenes[self.src.get("scene", 0)].get("nodes", [])
        children = {c for n in self.src.get("nodes", []) for c in n.get("children", [])}
        return [k for k in range(len(self.src.get("nodes", []))) if k not in children]


# ------------------------------------------------------------------------------ primitives (assets.md 6)


def _ring(cx, cy, rx, ry):
    """The 16-sided ring of assets.md 6: k * 22.5 degrees from +x, rounded to the millimetre."""
    pts = []
    for k in range(16):
        c, s = _cos_sin(22500 * k)
        pts.append((round((cx + rx * c) * 1000) / 1000, round((cy + ry * s) * 1000) / 1000))
    return pts


def _cos_sin(md):
    """cos and sin of md millidegrees, exact at quarter turns (numbers.md 2's quadrant reduction)."""
    q, r = divmod(md, 90000)
    a = math.radians(r / 1000)
    c, s = (1.0, 0.0) if r == 0 else (math.cos(a), math.sin(a))
    for _ in range(q % 4):
        c, s = -s, c
    return c, s


def primitive_triangles(shape, b):
    """Triangles of a primitive in the asset frame, each a tuple of three (x, y, z) points,
    counter-clockwise seen from outside. b = [x0, y0, z0, x1, y1, z1], the asset's bounds."""
    x0, y0, z0, x1, y1, z1 = b
    cx, cy, rx, ry = (x0 + x1) / 2, (y0 + y1) / 2, (x1 - x0) / 2, (y1 - y0) / 2
    tris = []

    def quad(a, b_, c, d):  # a, b, c, d counter-clockwise from outside
        tris.append((a, b_, c))
        tris.append((a, c, d))

    if shape == "box":
        p = [(x, y, z) for z in (z0, z1) for y in (y0, y1) for x in (x0, x1)]
        quad(p[0], p[2], p[3], p[1])  # bottom (seen from below)
        quad(p[4], p[5], p[7], p[6])  # top
        quad(p[0], p[1], p[5], p[4])  # south
        quad(p[3], p[2], p[6], p[7])  # north
        quad(p[2], p[0], p[4], p[6])  # west
        quad(p[1], p[3], p[7], p[5])  # east
    elif shape == "prism":
        sw, se, ne, nw = (x0, y0, z0), (x1, y0, z0), (x1, y1, z0), (x0, y1, z0)
        rw, re = (x0, cy, z1), (x1, cy, z1)
        quad(sw, nw, ne, se)  # bottom
        quad(sw, se, re, rw)  # south slope
        quad(ne, nw, rw, re)  # north slope
        tris.append((nw, sw, rw))  # west gable
        tris.append((se, ne, re))  # east gable
    elif shape in ("cylinder", "cone"):
        ring = _ring(cx, cy, rx, ry)
        bottom = [(x, y, z0) for x, y in ring]
        for k in range(1, 15):  # base, a fan seen from below
            tris.append((bottom[0], bottom[k + 1], bottom[k]))
        if shape == "cylinder":
            top = [(x, y, z1) for x, y in ring]
            for k in range(1, 15):
                tris.append((top[0], top[k], top[k + 1]))
            for k in range(16):
                quad(bottom[k], bottom[(k + 1) % 16], top[(k + 1) % 16], top[k])
        else:
            apex = (cx, cy, z1)
            for k in range(16):
                tris.append((bottom[k], bottom[(k + 1) % 16], apex))
    elif shape == "sphere":
        cz, rz = (z0 + z1) / 2, (z1 - z0) / 2
        rings = []
        for k in range(1, 8):  # 8 up: 7 rings between the poles
            c, s = _cos_sin(-90000 + 22500 * k)
            rings.append([(cx + rx * c * u, cy + ry * c * v, cz + rz * s)
                          for u, v in (_cos_sin(22500 * m) for m in range(16))])
        south, north = (cx, cy, z0), (cx, cy, z1)
        for k in range(16):
            tris.append((south, rings[0][(k + 1) % 16], rings[0][k]))
            tris.append((north, rings[6][k], rings[6][(k + 1) % 16]))
        for r in range(6):
            for k in range(16):
                quad(rings[r][k], rings[r][(k + 1) % 16], rings[r + 1][(k + 1) % 16], rings[r + 1][k])
    else:
        raise Refused(f"primitive `{shape}` is not one of assets.md 6")
    return tris


def flat_mesh(g, tris, name, material):
    """A non-indexed mesh of triangles in the asset frame, with flat normals."""
    pos, nor = [], []
    for a, b, c in tris:
        ux, uy, uz = b[0] - a[0], b[1] - a[1], b[2] - a[2]
        vx, vy, vz = c[0] - a[0], c[1] - a[1], c[2] - a[2]
        n = (uy * vz - uz * vy, uz * vx - ux * vz, ux * vy - uy * vx)
        ln = math.sqrt(n[0] ** 2 + n[1] ** 2 + n[2] ** 2) or 1.0
        n = gl(n[0] / ln, n[1] / ln, n[2] / ln)
        for p in (a, b, c):
            pos.extend(gl(*p))
            nor.extend(n)
    prim = {"attributes": {"POSITION": g.vec3(pos, True), "NORMAL": g.vec3(nor)}, "material": material}
    return g.mesh(name, [prim])


# ------------------------------------------------------------------------------ the export


def quat_face(face_md):
    """The rotation of a facing (build.md 8, R(b)) in glTF axes: a turn of -b about +Y."""
    c, s = _cos_sin(face_md % 360000)
    # half-angle of -b: cos(b/2), -sin(b/2), with the sign chosen so w >= 0
    half_c = math.sqrt(max(0.0, (1 + c) / 2))
    half_s = math.copysign(math.sqrt(max(0.0, (1 - c) / 2)), s)
    return [0.0, -half_s + 0.0, 0.0, half_c]


def export(build, out_path, pack=None):
    with open(os.path.join(build, "manifest.json"), "rb") as f:
        m = json.load(f)
    if m.get("format") != "via-build-1":
        raise Refused(f"manifest format {m.get('format')!r} is not via-build-1")
    if pack is None and m.get("locations", {}).get("pack_root") is not None:
        pack = os.path.join(build, m["locations"]["pack_root"])
    g = Gltf()
    frame = m["frame"]
    ex0, ey0 = frame["extent"][0], frame["extent"][1]
    nx, ny = frame["posts"]
    res = frame["resolution"]

    root = g.node(m["world"], extras={"via": {"id": m["id"], "format": m["format"], "frame": frame,
                                              "seed": m["seed"]}})
    fields = m.get("fields", {})
    terrain = next((k for k, v in fields.items() if v.get("role") == "surface"), None)
    weights = next((k for k, v in fields.items() if v.get("role") == "material" and v.get("type") == "weights"), None)
    carried = [k for k in (terrain, weights) if k]
    g.doc["nodes"][root]["extras"]["via"]["fields_not_drawn"] = sorted(k for k in fields if k not in carried)

    heights = cut = None
    if terrain:
        shape, heights = read_npy(os.path.join(build, fields[terrain]["file"]))
        if shape != (ny, nx):
            raise Refused(f"{fields[terrain]['file']}: shape {shape}, posts {(ny, nx)}")
        if fields[terrain].get("cut"):
            cshape, cut = read_npy(os.path.join(build, fields[terrain]["cut"]))
            if cshape != (ny - 1, nx - 1):
                raise Refused(f"{fields[terrain]['cut']}: shape {cshape}")

    materials = m.get("materials", {})
    ground_plain = g.material("ground", DEFAULT_COLOUR) if terrain else None
    ground_painted = g.material("ground_painted", [1.0, 1.0, 1.0]) if terrain and weights else None

    # Instance columns.
    inst = m.get("instances") or {"count": 0}
    cols = {}
    names = []
    if inst["count"]:
        for c in ("x", "y", "z", "face", "scale", "tint"):
            cols[c] = read_npy(os.path.join(build, inst[c]))[1]
        with open(os.path.join(build, inst["names"]), "rb") as f:
            names = f.read().decode("utf-8").split("\n")[:-1]
        if len(names) != inst["count"]:
            raise Refused("names.txt does not hold one line per instance")

    asset_meshes = {}  # (asset, tint) -> mesh index (primitive or stand-in)
    copiers = {}  # asset -> (Copier, mesh cache)

    def asset_shape(aid, tint):
        a = m["assets"][aid]
        colour = a.get("colour", DEFAULT_COLOUR)
        if (aid, tint) not in asset_meshes:
            mat = g.material(f"{aid}" if tint == 1 else f"{aid}*{tint!r}",
                             [min(1.0, c * tint) for c in colour])
            if (aid, 1.0) in asset_meshes and (aid, tint) != (aid, 1.0):
                base = g.doc["meshes"][asset_meshes[(aid, 1.0)]]["primitives"][0]
                prim = dict(base, material=mat)
                asset_meshes[(aid, tint)] = g.mesh(aid, [prim])
            else:
                shape = a.get("primitive") if a.get("source") == "primitive" and not a.get("stand_in") else "box"
                asset_meshes[(aid, tint)] = flat_mesh(g, primitive_triangles(shape, a["bounds"]), aid, mat)
        return asset_meshes[(aid, tint)]

    def place_model(aid, tint, parent):
        a = m["assets"][aid]
        if aid not in copiers:
            # An asset an import's pack binds lies under that pack's root (build.md 4, 7).
            root = pack
            if isinstance(a.get("pack"), str):
                pr = m.get("locations", {}).get("pack_roots", {}).get(a["pack"])
                root = os.path.join(build, pr) if pr is not None else None
            if root is None:
                raise Refused(f"asset `{aid}` is a model and its pack's root is not known (--pack)")
            path = os.path.join(root, a["files"][0]["path"])
            src, buffers = read_gltf(path)
            copiers[aid] = (Copier(g, src, buffers), {})
        cp, cache = copiers[aid]
        t = a["to_asset"]  # 3 by 4, row by row: model glTF space -> asset frame
        # glTF of the asset frame is P * to_asset, P = (x, y, z) -> (x, z, -y)
        rows = [t[0:4], t[8:12], [-v for v in t[4:8]]]
        mat = []  # column-major 4 by 4
        for c in range(4):
            mat.extend([rows[0][c] + 0.0, rows[1][c] + 0.0, rows[2][c] + 0.0, 1.0 if c == 3 else 0.0])
        holder = g.node("model", parent, matrix=mat)
        for r in cp.roots():
            cp.nodes(r, holder, tint, cache)

    instance_nodes = 0
    for ch in m.get("chunks", []):
        cx, cy = ch["at"]
        cnode = g.node(f"c_{cx}_{cy}", root)
        i0, j0, i1, j1 = ch["posts"]
        if terrain:
            w = ch.get("weights", {}).get(weights) if weights else None
            planes = None
            if w and w.get("file"):
                pshape, planes = read_npy(os.path.join(build, w["file"]))
                if pshape != (len(w["materials"]), j1 - j0 + 1, i1 - i0 + 1):
                    raise Refused(f"{w['file']}: shape {pshape}")
            mesh = terrain_mesh(g, heights, cut, nx, ny, res, i0, j0, i1, j1, ex0, ey0, w, planes,
                                materials, ground_painted if weights else ground_plain, f"c_{cx}_{cy}")
            if mesh is not None:
                ox, oy = ex0 + i0 * res, ey0 + j0 * res
                g.node(terrain, cnode, mesh=mesh, translation=list(gl(ox, oy, 0.0)))
        for aid in sorted(ch.get("instances", {})):
            first, count = ch["instances"][aid]
            a = m["assets"][aid]
            for r in range(first, first + count):
                tint = float(cols["tint"][r])
                s = float(cols["scale"][r])
                keys = {
                    "translation": list(gl(cols["x"][r] / 1000, cols["y"][r] / 1000, float(cols["z"][r]))),
                    "rotation": quat_face(int(cols["face"][r])),
                    "scale": [s, s, s],
                    "extras": {"via": {"instance": names[r], "asset": aid, "face": int(cols["face"][r]),
                                       "tint": tint}},
                }
                if a.get("source") == "model" and not a.get("stand_in"):
                    n = g.node(names[r], cnode, **keys)
                    place_model(aid, tint, n)
                else:
                    g.node(names[r], cnode, mesh=asset_shape(aid, tint), **keys)
                instance_nodes += 1
    if instance_nodes != inst["count"]:
        raise Refused(f"chunks place {instance_nodes} instances; the manifest counts {inst['count']}")

    # Features (9.1).
    with open(os.path.join(build, "features.json"), "rb") as f:
        feats = json.load(f)
    line_mat = g.material("feature_line", [1.0, 0.9, 0.1])
    fnode = g.node("features", root)
    by_name = {}

    def ground_at(x, y):
        if heights is None:
            return 0.0
        return drawn_height(heights, nx, ny, res, ex0, ey0, x, y)

    def feature(f, parent, name):
        extras = {"via": {k: f.get(k) for k in ("kind", "authority", "layer", "shape") if k in f},
                  "attrs": f.get("attrs", {})}
        if f.get("instance") is not None:
            extras["via"]["footprint_of"] = f["instance"]
        shape = f.get("shape", "region")
        if shape == "point":
            x, y, z = f["at"]
            z = ground_at(x, y) if z is None else z
            return g.node(name, parent, translation=list(gl(x, y, z)), extras=extras)
        if shape == "path":
            # a vertex off the extent has z null (9.1): drawn at the drawn ground's edge (15.3)
            pts = [(x, y, ground_at(x, y) if z is None else z) for x, y, z in f["points"]]
            ox, oy, oz = pts[0]
            pos = []
            for x, y, z in pts:
                pos.extend(gl(x - ox, y - oy, z - oz))
            prims = [{"attributes": {"POSITION": g.vec3(pos, True)}, "mode": 3, "material": line_mat}]
            return g.node(name, parent, translation=list(gl(ox, oy, oz)),
                          mesh=g.mesh(name, prims), extras=extras)
        # region: every ring a LINE_LOOP, at the drawn ground (15.3)
        prims = []
        first = f["pieces"][0]["outer"][0] if f.get("pieces") else [0, 0]
        ox, oy = first
        oz = ground_at(ox, oy)
        for piece in f.get("pieces", []):
            for ring in [piece["outer"]] + piece.get("holes", []):
                pos = []
                for x, y in ring:
                    pos.extend(gl(x - ox, y - oy, ground_at(x, y) - oz))
                prims.append({"attributes": {"POSITION": g.vec3(pos, True)}, "mode": 2, "material": line_mat})
        keys = {"translation": list(gl(ox, oy, oz)), "extras": extras}
        if prims:
            keys["mesh"] = g.mesh(name, prims)
        return g.node(name, parent, **keys)

    for f in feats.get("features", []):
        feature(f, fnode, f["name"])
        by_name[f["name"]] = f
    pnode = g.node("footprints", root)
    for f in feats.get("footprints", []):
        feature(dict(f, shape="region"), pnode, f["instance"])

    # Outputs (9.2): an empty node per feature under a node named after the output.
    for out in sorted(m.get("outputs", {})):
        onode = g.node(out, root)
        for name in m["outputs"][out]:
            f = by_name.get(name)
            if f is None:
                raise Refused(f"output `{out}` names `{name}`, which features.json does not hold")
            if f["shape"] in ("point", "path"):
                x, y, z = f["at"] if f["shape"] == "point" else f["points"][0]
                at = gl(x, y, ground_at(x, y) if z is None else z)
            else:
                at = None
            g.node(name, onode, translation=list(at) if at else None,
                   extras={"via": {"kind": f.get("kind"), "shape": f["shape"]}, "attrs": f.get("attrs", {})})

    # Generated meshes (10): copied in, node and material as written.
    if m.get("meshes"):
        mnode = g.node("meshes", root)
        # build.md 10: a mesh with no material is drawn in the reader's default; glTF's own default
        # is a white metal, so the reader names its own: the default colour, not metal, and
        # double-sided as the core writes a mesh's own material (a floor seen from below).
        plain = g.material("mesh_default", DEFAULT_COLOUR, double_sided=True)
        for entry in m["meshes"]:
            src, buffers = read_gltf(os.path.join(build, entry["file"]))
            cp = Copier(g, src, buffers, plain)
            holder = g.node(entry["name"], mnode, extras={"via": {k: entry.get(k) for k in
                            ("feature", "surface", "collides", "material", "triangles")}})
            cache = {}
            for r in cp.roots():
                cp.nodes(r, holder, 1.0, cache)

    g.write(out_path)
    return {"instances": instance_nodes, "nodes": len(g.doc["nodes"]), "meshes": len(g.doc.get("meshes", [])),
            "bytes": os.path.getsize(out_path)}


def drawn_height(h, nx, ny, res, ex0, ey0, x, y):
    """The height of the ground as drawn (15.1's split, south-west to north-east) at (x, y)."""
    u = min(max((x - ex0) / res, 0.0), nx - 1.0)
    v = min(max((y - ey0) / res, 0.0), ny - 1.0)
    i = min(int(u), nx - 2)
    j = min(int(v), ny - 2)
    fu, fv = u - i, v - j
    sw, se = h[j * nx + i], h[j * nx + i + 1]
    nw, ne = h[(j + 1) * nx + i], h[(j + 1) * nx + i + 1]
    if fu >= fv:  # south-east triangle (sw, se, ne)
        return sw + fu * (se - sw) + fv * (ne - se)
    return sw + fv * (nw - sw) + fu * (ne - nw)  # north-west triangle (sw, ne, nw)


def terrain_mesh(g, h, cut, nx, ny, res, i0, j0, i1, j1, ex0, ey0, w, planes, materials, mat, name):
    """One chunk of the terrain: its posts, quads split south-west to north-east, cut quads left out."""
    cols, rows = i1 - i0 + 1, j1 - j0 + 1
    pos, nor, col = [], [], []
    for j in range(j0, j1 + 1):
        for i in range(i0, i1 + 1):
            z = h[j * nx + i]
            pos.extend(gl((i - i0) * res, (j - j0) * res, z))
            # normal from the whole field's neighbours, so chunks meet without a seam in shading
            hx = (h[j * nx + min(i + 1, nx - 1)] - h[j * nx + max(i - 1, 0)]) / (res * (min(i + 1, nx - 1) - max(i - 1, 0)))
            hy = (h[min(j + 1, ny - 1) * nx + i] - h[max(j - 1, 0) * nx + i]) / (res * (min(j + 1, ny - 1) - max(j - 1, 0)))
            ln = math.sqrt(hx * hx + hy * hy + 1)
            nor.extend(gl(-hx / ln, -hy / ln, 1 / ln))
    if w is not None:
        cols_ = [materials.get(mid, {}).get("colour", DEFAULT_COLOUR) for mid in w.get("materials", [])]
        n = rows * cols
        for p in range(n):
            if planes is None:
                col.extend(DEFAULT_COLOUR)
                continue
            r = g_ = b = 0.0
            total = 0
            for k, c in enumerate(cols_):
                v = planes[k * n + p]
                total += v
                r += c[0] * v
                g_ += c[1] * v
                b += c[2] * v
            col.extend(DEFAULT_COLOUR if total == 0 else (r / 255, g_ / 255, b / 255))
    idx = []
    for j in range(j0, j1):
        for i in range(i0, i1):
            if cut is not None and cut[j * (nx - 1) + i]:
                continue
            sw = (j - j0) * cols + (i - i0)
            se, nw = sw + 1, sw + cols
            ne = nw + 1
            idx.extend((sw, se, ne, sw, ne, nw))
    if not idx:
        return None
    attrs = {"POSITION": g.vec3(pos, True), "NORMAL": g.vec3(nor)}
    if col:
        attrs["COLOR_0"] = g.vec3([min(1.0, max(0.0, c)) for c in col])
    prim = {"attributes": attrs, "indices": g.indices(idx, len(pos) // 3), "material": mat}
    return g.mesh(name, [prim])


def main(argv=None):
    p = argparse.ArgumentParser(prog="export.py", description=__doc__.split("\n")[0])
    p.add_argument("build")
    p.add_argument("--out", required=True)
    p.add_argument("--pack")
    try:
        args = p.parse_args(argv)
    except SystemExit as e:
        return 2 if e.code else 0
    try:
        info = export(args.build, args.out, args.pack)
    except (Refused, OSError, KeyError, ValueError) as e:
        print(f"export.py: {e}", file=sys.stderr)
        return 1
    print(json.dumps(info, sort_keys=True))
    return 0


if __name__ == "__main__":
    sys.exit(main())
