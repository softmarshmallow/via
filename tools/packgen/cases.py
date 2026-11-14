"""Writes the M2 conformance cases of `assets.md` (sections 3, 4, 8, 9 and `collision`) under
`spec/cases/assets/`: each case's `case.toml`, pack, world and the small glTF models it reads.

    python3 tools/packgen/cases.py           # (re)writes the case folders below
    python3 tools/packgen/cases.py --check   # exits 1 when a file on disk differs

Every expectation is taken from the chapter's Conformance row and the rules it cites, never from
the engine's output. The models are made here, from the row's words ("a 1 m cube centred at model
(1, 0.5, 0)"), as `.gltf` text with the buffer in a `data:` URI unless the row is about URIs. The
one digest a case expects (Accept bytes, Data URI) is computed by `blake3.py`, a pure-Python
BLAKE3 apart from the engine.
"""

from __future__ import annotations

import argparse
import base64
import json
import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from blake3 import digest  # noqa: E402

ROOT = Path(__file__).resolve().parents[2] / "spec" / "cases" / "assets"

# --- glTF ---------------------------------------------------------------------------------------


class G:
    """A glTF 2.0 document under construction, its binary data in one buffer."""

    def __init__(self) -> None:
        self.bin = bytearray()
        self.doc: dict = {
            "asset": {"version": "2.0"},
            "accessors": [],
            "bufferViews": [],
            "meshes": [],
            "nodes": [],
            "scene": 0,
            "scenes": [{"nodes": []}],
        }

    def view(self, data: bytes, stride: int | None = None) -> int:
        while len(self.bin) % 4:
            self.bin.append(0)
        v = {"buffer": 0, "byteOffset": len(self.bin), "byteLength": len(data)}
        if stride is not None:
            v["byteStride"] = stride
        self.bin.extend(data)
        self.doc["bufferViews"].append(v)
        return len(self.doc["bufferViews"]) - 1

    def accessor(self, a: dict) -> int:
        self.doc["accessors"].append(a)
        return len(self.doc["accessors"]) - 1

    def positions(self, pts: list[tuple[float, float, float]]) -> int:
        data = b"".join(struct.pack("<3f", *p) for p in pts)
        lo = [min(p[k] for p in pts) for k in range(3)]
        hi = [max(p[k] for p in pts) for k in range(3)]
        lo = [struct.unpack("<f", struct.pack("<f", x))[0] for x in lo]
        hi = [struct.unpack("<f", struct.pack("<f", x))[0] for x in hi]
        return self.accessor(
            {"bufferView": self.view(data), "componentType": 5126, "count": len(pts), "type": "VEC3", "min": lo, "max": hi}
        )

    def indices(self, ix: list[int]) -> int:
        data = struct.pack(f"<{len(ix)}H", *ix)
        return self.accessor({"bufferView": self.view(data), "componentType": 5123, "count": len(ix), "type": "SCALAR"})

    def mesh(self, prims: list[dict], extra: dict | None = None) -> int:
        m = {"primitives": prims}
        if extra:
            m.update(extra)
        self.doc["meshes"].append(m)
        return len(self.doc["meshes"]) - 1

    def node(self, n: dict, root: bool = True) -> int:
        self.doc["nodes"].append(n)
        i = len(self.doc["nodes"]) - 1
        if root:
            self.doc["scenes"][0]["nodes"].append(i)
        return i

    def text(self, uri: str | None = None) -> tuple[str, bytes]:
        """The `.gltf` text, and the buffer's bytes (embedded as a data URI unless `uri`)."""
        while len(self.bin) % 4:
            self.bin.append(0)
        b: dict = {"byteLength": len(self.bin)}
        if uri is None:
            b["uri"] = "data:application/octet-stream;base64," + base64.b64encode(bytes(self.bin)).decode()
        else:
            b["uri"] = uri
        self.doc["buffers"] = [b]
        return json.dumps(self.doc, indent=1, sort_keys=True) + "\n", bytes(self.bin)


CUBE_IX = [
    0, 2, 1, 0, 3, 2,  # -z
    4, 5, 6, 4, 6, 7,  # +z
    0, 1, 5, 0, 5, 4,  # -y
    3, 7, 6, 3, 6, 2,  # +y
    0, 4, 7, 0, 7, 3,  # -x
    1, 2, 6, 1, 6, 5,  # +x
]


def corners(lo: tuple[float, float, float], hi: tuple[float, float, float]) -> list[tuple[float, float, float]]:
    (x0, y0, z0), (x1, y1, z1) = lo, hi
    return [(x0, y0, z0), (x1, y0, z0), (x1, y1, z0), (x0, y1, z0), (x0, y0, z1), (x1, y0, z1), (x1, y1, z1), (x0, y1, z1)]


def cube_gltf(lo, hi, *, node: dict | None = None, material: dict | None = None, extra: dict | None = None, uri: str | None = None, mesh_extra: dict | None = None) -> tuple[str, bytes]:
    """A box over model coordinates `lo` to `hi`: 8 positions, 36 indices, 12 triangles."""
    g = G()
    prim = {"attributes": {"POSITION": g.positions(corners(lo, hi))}, "indices": g.indices(CUBE_IX), "mode": 4}
    if material is not None:
        g.doc["materials"] = [material]
        prim["material"] = 0
    g.node({"mesh": g.mesh([prim], mesh_extra), **(node or {})})
    if extra:
        g.doc.update(extra)
    return g.text(uri)


def unit_cube(**kw) -> str:
    """The 1 m cube on the model origin's plan centre, from height 0 to 1."""
    return cube_gltf((-0.5, 0.0, -0.5), (0.5, 1.0, 0.5), **kw)[0]


# --- case text ----------------------------------------------------------------------------------

WORLD = """name       = "yard"
extent     = [0, 0, 64, 64]
resolution = 1
pack       = "pack.toml"

[field.height]
role = "surface"
base = 20

[instance.a]
asset = "cube"
at    = [20, 20]

[instance.b]
asset = "cube"
at    = [40, 40]
"""

CUBE_PACK = """[asset.cube]
file = "cube.gltf"
"""


def head(summary: str, actions: list[str], pending: str | None = None) -> str:
    t = f'format    = "via-case-1"\nsummary   = {json.dumps(summary)}\nmilestone = 2\nactions   = {json.dumps(actions)}\n'
    if pending:
        t += f"pending   = {json.dumps(pending)}\n"
    return t


CASES: dict[str, dict[str, str | bytes]] = {}


def case(name: str, files: dict[str, str | bytes]) -> None:
    CASES[name] = files


def list_case(name: str, summary: str, model: str, pack: str, checks: str, diag: str = "") -> None:
    case(
        name,
        {
            "case.toml": head(summary, ["list"])
            + '\n[action.list]\nvia         = ["pack", "list", "pack.toml"]\nexit        = 0\ndiagnostics = "includes"\n'
            + checks
            + diag,
            "pack.toml": pack,
            "cube.gltf": model,
        },
    )


def stdout_value(id_: str, pointer: str, value: str, line: int = 1, action: str = "list") -> str:
    return f'\n[action.{action}.file.{id_}]\nstream  = "stdout"\npointer = "{pointer}"\nline    = {line}\nvalue   = {value}\n'


def check_case(name: str, summary: str, exit_: int, files: dict[str, str | bytes], diags: str) -> None:
    case(
        name,
        {
            "case.toml": head(summary, ["check"])
            + f'\n[action.check]\nvia         = ["pack", "check", "pack.toml"]\nexit        = {exit_}\ndiagnostics = "includes"\n'
            + diags,
            **files,
        },
    )


ORIGIN_PACK = CUBE_PACK + 'pivot = "origin"\n'

# glTF axes: a 1 m cube centred at model (1, 0.5, 0). Asset x = -mx, y = mz, z = my (2.3).
AXES_CUBE = cube_gltf((0.5, 0.0, -0.5), (1.5, 1.0, 0.5))[0]
list_case(
    "gltf_axes",
    "A .glb 1 m cube centred at model (1, 0.5, 0), pivot = \"origin\": bounds [-1.5, -0.5, 0, -0.5, 0.5, 1].",
    AXES_CUBE,
    ORIGIN_PACK,
    stdout_value("bounds", "/bounds", "[-1.5, -0.5, 0, -0.5, 0.5, 1]"),
)
list_case(
    "gltf_axes_base_centre",
    "The same cube, default pivot: bounds [-0.5, -0.5, 0, 0.5, 0.5, 1], origin [1, 0, 0], asset.pivot_moved.",
    AXES_CUBE,
    CUBE_PACK,
    stdout_value("bounds", "/bounds", "[-0.5, -0.5, 0, 0.5, 0.5, 1]") + stdout_value("origin", "/origin", "[1, 0, 0]"),
    '\n[action.list.diag.pivot_moved]\ncode    = "asset.pivot_moved"\nsubject = "asset.cube"\n',
)
list_case(
    "z_up_export",
    "The same cube exported Z-up, centred at model (1, 0, 0.5), up = \"+z\", front = \"-y\", pivot = \"origin\": bounds [-1.5, -0.5, 0, -0.5, 0.5, 1].",
    cube_gltf((0.5, -0.5, 0.0), (1.5, 0.5, 1.0))[0],
    ORIGIN_PACK + 'up   = "+z"\nfront = "-y"\n',
    stdout_value("bounds", "/bounds", "[-1.5, -0.5, 0, -0.5, 0.5, 1]"),
)


def noisy_cube() -> str:
    pts = corners((0.5, 0.0, -0.5), (1.5, 1.0, 0.5))
    pts[1] = (1.5000001, pts[1][1], pts[1][2])
    g = G()
    prim = {"attributes": {"POSITION": g.positions(pts)}, "indices": g.indices(CUBE_IX), "mode": 4}
    g.node({"mesh": g.mesh([prim])})
    return g.text()[0]


list_case(
    "float_noise",
    "The glTF axes cube with one vertex at 1.5000001: the same bounds as the clean cube.",
    noisy_cube(),
    ORIGIN_PACK,
    stdout_value("bounds", "/bounds", "[-1.5, -0.5, 0, -0.5, 0.5, 1]"),
)

# Pivot rule and pivot origin: a cube centred in plan, its origin 2 m above its base.
HIGH_CUBE = cube_gltf((-0.5, -2.0, -0.5), (0.5, -1.0, 0.5))[0]
for name, pack, summary, origin, shift, moved in [
    (
        "pivot_rule",
        CUBE_PACK,
        "A model centred in plan, its origin 2 m above its base, base_centre: origin [0, 0, 2], asset.pivot_moved; the lowest point at the placing point minus sink.",
        "[0, 0, 2]",
        2,
        1,
    ),
    (
        "pivot_origin",
        ORIGIN_PACK,
        "The same model, pivot = \"origin\": origin [0, 0, 0], no asset.pivot_moved; the model stands 2 m lower than in the previous case.",
        "[0, 0, 0]",
        0,
        0,
    ),
]:
    case(
        name,
        {
            "case.toml": head(summary, ["build"])
            + "\n# The asset frame from the model (build.md 7 `to_asset`): rows x = -mx, y = mz, z = my, then\n"
            + "# minus the pivot. Under base_centre the pivot is the lowest point, 2 m below the origin, so the\n"
            + "# translation lifts the model by 2 m onto the placing point; under origin it lifts nothing.\n"
            + '\n[action.build]\nvia         = ["build", "world.toml", "--out", "build/w"]\nexit        = 0\nout         = "build/w"\ndiagnostics = "includes"\n'
            + f'\n[action.build.file.origin]\npath    = "build/w/manifest.json"\npointer = "/assets/cube/origin"\nvalue   = {origin}\n'
            + f'\n[action.build.file.to_asset]\npath    = "build/w/manifest.json"\npointer = "/assets/cube/to_asset"\nvalue   = [-1, 0, 0, 0, 0, 0, 1, 0, 0, 1, 0, {shift}]\n'
            + f'\n[action.build.diag.pivot_moved]\ncode    = "asset.pivot_moved"\nsubject = "asset.cube"\nrecords = {moved}\n',
            "pack.toml": pack,
            "cube.gltf": HIGH_CUBE,
            "world.toml": WORLD,
        },
    )


def triangle_count() -> str:
    g = G()
    pos = g.positions([(0.0, 0.0, 0.0), (1.0, 0.0, 0.0), (1.0, 1.0, 0.0), (0.0, 1.0, 0.0), (0.0, 0.0, 1.0), (1.0, 0.0, 1.0)])
    prims = [
        {"attributes": {"POSITION": pos}, "indices": g.indices([0, 1, 2, 0, 2, 3, 0, 4, 5]), "mode": 4},
        {"attributes": {"POSITION": pos}, "indices": g.indices([0, 1, 2, 3, 4]), "mode": 5},
        {"attributes": {"POSITION": pos}, "indices": g.indices([0, 1, 2, 3]), "mode": 6},
    ]
    m = g.mesh(prims)
    g.node({"mesh": m})
    g.node({"mesh": m, "translation": [2, 0, 0]})
    return g.text()[0]


list_case(
    "triangle_count",
    "A mesh with a list of 9 indices, a strip of 5 and a fan of 4, used by two glTF nodes: triangles = 16 (3 + 3 + 2, twice).",
    triangle_count(),
    CUBE_PACK,
    stdout_value("triangles", "/triangles", "16"),
)


def bad_index_count() -> str:
    g = G()
    pos = g.positions(corners((0.0, 0.0, 0.0), (1.0, 1.0, 1.0)))
    g.node({"mesh": g.mesh([{"attributes": {"POSITION": pos}, "indices": g.indices([0, 1, 2, 0, 2, 3, 4]), "mode": 4}])})
    return g.text()[0]


check_case(
    "bad_index_count",
    "Mode 4 with 7 indices: pack.bad_model, data.pointer the primitive.",
    1,
    {"pack.toml": CUBE_PACK, "cube.gltf": bad_index_count()},
    '\n[action.check.diag.bad_model]\ncode    = "pack.bad_model"\nsubject = "asset.cube"\nfile    = "pack:cube.gltf"\ndata    = { pointer = "/meshes/0/primitives/0" }\n',
)


def sparse() -> str:
    g = G()
    pts = corners((-0.5, 0.0, -0.5), (0.5, 1.0, 0.5))
    base = g.positions(pts)
    acc = g.doc["accessors"][base]
    # The sparse corner: vertex 6, (0.5, 1, 0.5), moved 1 m out along +x.
    acc["sparse"] = {
        "count": 1,
        "indices": {"bufferView": g.view(struct.pack("<H", 6)), "componentType": 5123},
        "values": {"bufferView": g.view(struct.pack("<3f", 1.5, 1.0, 0.5))},
    }
    acc["max"] = [1.5, 1.0, 0.5]
    g.node({"mesh": g.mesh([{"attributes": {"POSITION": base}, "indices": g.indices(CUBE_IX), "mode": 4}])})
    return g.text()[0]


list_case(
    "sparse",
    "A cube whose positions are a sparse accessor moving one corner 1 m out: bounds include the moved corner.",
    sparse(),
    ORIGIN_PACK,
    stdout_value("bounds", "/bounds", "[-1.5, -0.5, 0, 0.5, 0.5, 1]"),
)


def quantized() -> str:
    """The unit cube as 16-bit integer millimetres under a node scale of 0.001."""
    g = G()
    pts = corners((-500, 0, -500), (500, 1000, 500))
    data = b"".join(struct.pack("<3hxx", *p) for p in pts)
    pos = g.accessor(
        {"bufferView": g.view(data, stride=8), "componentType": 5122, "count": 8, "type": "VEC3", "min": [-500, 0, -500], "max": [500, 1000, 500]}
    )
    g.node({"mesh": g.mesh([{"attributes": {"POSITION": pos}, "indices": g.indices(CUBE_IX), "mode": 4}]), "scale": [0.001, 0.001, 0.001]})
    g.doc["extensionsUsed"] = ["KHR_mesh_quantization"]
    g.doc["extensionsRequired"] = ["KHR_mesh_quantization"]
    return g.text()[0]


list_case(
    "quantized",
    "The cube with KHR_mesh_quantization required: the same bounds as the float cube.",
    quantized(),
    ORIGIN_PACK,
    stdout_value("bounds", "/bounds", "[-0.5, -0.5, 0, 0.5, 0.5, 1]"),
)

check_case(
    "compressed",
    "KHR_draco_mesh_compression required: pack.bad_model naming it.",
    1,
    {
        "pack.toml": CUBE_PACK,
        "cube.gltf": unit_cube(extra={"extensionsUsed": ["KHR_draco_mesh_compression"], "extensionsRequired": ["KHR_draco_mesh_compression"]}),
    },
    '\n[action.check.diag.bad_model]\ncode    = "pack.bad_model"\nsubject = "asset.cube"\ndata    = { pointer = "/extensionsRequired/0" }\n',
)
check_case(
    "instancing",
    "EXT_mesh_gpu_instancing in extensionsUsed only: pack.bad_model naming it.",
    1,
    {"pack.toml": CUBE_PACK, "cube.gltf": unit_cube(extra={"extensionsUsed": ["EXT_mesh_gpu_instancing"]})},
    '\n[action.check.diag.bad_model]\ncode    = "pack.bad_model"\nsubject = "asset.cube"\ndata    = { pointer = "/extensionsUsed/0" }\n',
)
check_case(
    "tolerated",
    "KHR_materials_emissive_strength required: no error.",
    0,
    {
        "pack.toml": CUBE_PACK,
        "cube.gltf": unit_cube(extra={"extensionsUsed": ["KHR_materials_emissive_strength"], "extensionsRequired": ["KHR_materials_emissive_strength"]}),
    },
    '\n[action.check.diag.bad_model]\ncode    = "pack.bad_model"\nrecords = 0\n',
)


def skinned() -> str:
    g = G()
    pos = g.positions(corners((-0.5, 0.0, -0.5), (0.5, 1.0, 0.5)))
    joints = g.accessor({"bufferView": g.view(bytes(4 * 8)), "componentType": 5121, "count": 8, "type": "VEC4"})
    weights = g.accessor({"bufferView": g.view(struct.pack("<32f", *([1.0, 0.0, 0.0, 0.0] * 8))), "componentType": 5126, "count": 8, "type": "VEC4"})
    ibm = g.accessor({"bufferView": g.view(struct.pack("<16f", 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1)), "componentType": 5126, "count": 1, "type": "MAT4"})
    m = g.mesh([{"attributes": {"POSITION": pos, "JOINTS_0": joints, "WEIGHTS_0": weights}, "indices": g.indices(CUBE_IX), "mode": 4}])
    g.node({"mesh": m, "skin": 0, "translation": [5, 0, 0]})
    joint = g.node({"name": "joint", "translation": [0, 3, 0]})
    g.doc["skins"] = [{"joints": [joint], "inverseBindMatrices": ibm}]
    return g.text()[0]


list_case(
    "skinned",
    "A skinned cube whose glTF node is translated 5 m: bounds as the unskinned cube under that glTF node.",
    skinned(),
    ORIGIN_PACK,
    stdout_value("bounds", "/bounds", "[-5.5, -0.5, 0, -4.5, 0.5, 1]"),
)


def morph() -> str:
    g = G()
    pos = g.positions(corners((-0.5, 0.0, -0.5), (0.5, 1.0, 0.5)))
    target = g.positions([(0.0, 2.0, 0.0)] * 8)
    m = g.mesh(
        [{"attributes": {"POSITION": pos}, "indices": g.indices(CUBE_IX), "mode": 4, "targets": [{"POSITION": target}]}],
        {"weights": [1]},
    )
    g.node({"mesh": m})
    return g.text()[0]


list_case(
    "morph",
    "A cube with a morph target and default weights = [1]: bounds of the base positions.",
    morph(),
    ORIGIN_PACK,
    stdout_value("bounds", "/bounds", "[-0.5, -0.5, 0, 0.5, 0.5, 1]"),
)


def no_scene() -> str:
    g = G()
    pos = g.positions(corners((-0.5, 0.0, -0.5), (0.5, 1.0, 0.5)))
    g.node({"mesh": g.mesh([{"attributes": {"POSITION": pos}, "indices": g.indices(CUBE_IX), "mode": 4}])})
    del g.doc["scene"]
    del g.doc["scenes"]
    return g.text()[0]


check_case(
    "no_scene",
    "A .gltf with meshes and no scenes: asset.empty.",
    1,
    {"pack.toml": CUBE_PACK, "cube.gltf": no_scene()},
    '\n[action.check.diag.empty]\ncode    = "asset.empty"\nsubject = "asset.cube"\n',
)
check_case(
    "measured_only",
    "A model asset with size: asset.declared_measured, exit 1.",
    1,
    {"pack.toml": CUBE_PACK + "size = [1, 1, 1]\n", "cube.gltf": unit_cube()},
    '\n[action.check.diag.declared_measured]\ncode    = "asset.declared_measured"\nsubject = "asset.cube.size"\n',
)

TEXTURED = {"pbrMetallicRoughness": {"baseColorTexture": {"index": 0}}}


def with_image(uri: str) -> str:
    return unit_cube(material=TEXTURED, extra={"textures": [{"source": 0}], "images": [{"uri": uri}]})


check_case(
    "missing_texture",
    "A model referencing a missing image: pack.missing_file naming the URI and the asset.",
    1,
    {"pack.toml": CUBE_PACK, "cube.gltf": with_image("bark.png")},
    '\n[action.check.diag.missing_file]\ncode    = "pack.missing_file"\nsubject = "asset.cube"\nfile    = "pack:cube.gltf"\ndata    = { pointer = "/images/0/uri" }\n',
)

ENC_TEXT, ENC_BIN = cube_gltf((-0.5, 0.0, -0.5), (0.5, 1.0, 0.5), uri="my%20mesh.bin")
check_case(
    "encoded_uri",
    "A buffer URI my%20mesh.bin beside my mesh.bin: no error.",
    0,
    {"pack.toml": CUBE_PACK, "cube.gltf": ENC_TEXT, "my mesh.bin": ENC_BIN},
    '\n[action.check.diag.missing_file]\ncode    = "pack.missing_file"\nrecords = 0\n',
)

# A one-pixel PNG, grey: bytes written here, a file of the case, never read for its pixels.
PNG = bytes.fromhex(
    "89504e470d0a1a0a0000000d4948445200000001000000010802000000907753de"
    "0000000c4944415408d763b8b8b80000039a01cdbe3a1b4c0000000049454e44ae426082"
)
check_case(
    "uri_case",
    "A URI Oak.png beside oak.png: doc.path_case, on every system.",
    1,
    {"pack.toml": CUBE_PACK, "cube.gltf": with_image("Oak.png"), "oak.png": PNG},
    '\n[action.check.diag.path_case]\ncode    = "doc.path_case"\nsubject = "asset.cube"\ndata    = { pointer = "/images/0/uri" }\n',
)

OUT_TEXT, OUT_BIN = cube_gltf((-0.5, 0.0, -0.5), (0.5, 1.0, 0.5), uri="../outside.bin")
check_case(
    "outside_root",
    "A model with a ../ buffer URI out of root: pack.outside_root.",
    1,
    {"pack.toml": 'root = "models"\n\n' + CUBE_PACK, "models/cube.gltf": OUT_TEXT, "outside.bin": OUT_BIN},
    '\n[action.check.diag.outside_root]\ncode    = "pack.outside_root"\nsubject = "asset.cube"\nfile    = "pack:models/cube.gltf"\ndata    = { pointer = "/buffers/0/uri" }\n',
)

DATA_CUBE = unit_cube()
case(
    "data_uri",
    {
        "case.toml": head("A .gltf whose buffer is a data: URI: no error; no bound file besides the model.", ["check", "list"])
        + '\n[action.check]\nvia         = ["pack", "check", "pack.toml"]\nexit        = 0\ndiagnostics = "includes"\n'
        + '\n[action.check.diag.missing_file]\ncode    = "pack.missing_file"\nrecords = 0\n'
        + '\n[action.list]\nvia         = ["pack", "list", "pack.toml"]\nexit        = 0\ndiagnostics = "includes"\n'
        + stdout_value("files", "/files", "{ \"cube.gltf\" = \"" + digest(DATA_CUBE.encode()) + "\" }"),
        "pack.toml": CUBE_PACK,
        "cube.gltf": DATA_CUBE,
    },
)
check_case(
    "untextured",
    "A model whose material has only a base colour factor: asset.untextured, exit 0.",
    0,
    {"pack.toml": CUBE_PACK, "cube.gltf": unit_cube(material={"pbrMetallicRoughness": {"baseColorFactor": [0.5, 0.4, 0.3, 1.0]}})},
    '\n[action.check.diag.untextured]\ncode    = "asset.untextured"\nsubject = "asset.cube"\n',
)


def flat_card() -> str:
    """One quad in the model's x-y plane, which is the asset's x-z plane (2.3)."""
    g = G()
    pos = g.positions([(-0.5, 0.0, 0.0), (0.5, 0.0, 0.0), (0.5, 1.0, 0.0), (-0.5, 1.0, 0.0)])
    g.node({"mesh": g.mesh([{"attributes": {"POSITION": pos}, "indices": g.indices([0, 1, 2, 0, 2, 3]), "mode": 4}])})
    return g.text()[0]


list_case(
    "flat_card",
    "A model of one quad in the asset x-z plane: a point footprint, asset.point_footprint.",
    flat_card(),
    CUBE_PACK,
    stdout_value("footprint", "/footprint", "[]"),
    '\n[action.list.diag.point_footprint]\ncode    = "asset.point_footprint"\nsubject = "asset.cube"\n',
)

check_case(
    "detail_checks",
    "A detail copy 0.1 m wider; another shifted 0.5 m up; detail naming file: asset.detail_bounds, asset.detail_pivot, asset.detail_duplicate.",
    1,
    {
        "pack.toml": CUBE_PACK + 'detail = ["wide.gltf", "up.gltf", "cube.gltf"]\n',
        "cube.gltf": unit_cube(),
        "wide.gltf": cube_gltf((-0.55, 0.0, -0.5), (0.55, 1.0, 0.5))[0],
        "up.gltf": cube_gltf((-0.5, 0.5, -0.5), (0.5, 1.5, 0.5))[0],
    },
    '\n[action.check.diag.detail_duplicate]\ncode    = "asset.detail_duplicate"\nsubject = "asset.cube.detail"\n'
    + '\n[action.check.diag.detail_bounds]\ncode    = "asset.detail_bounds"\nsubject = "asset.cube.detail"\ndata    = { index = 0 }\n'
    + '\n[action.check.diag.detail_pivot]\ncode    = "asset.detail_pivot"\nsubject = "asset.cube.detail"\ndata    = { index = 1 }\n',
)

# Base corner: asset-frame bounds [2, 3, 1] to [4, 5, 2] before the pivot are model x in [-4, -2]
# (asset x = -mx), z in [3, 5] (asset y = mz), y in [1, 2] (asset z = my).
list_case(
    "base_corner",
    "A model with bounds [2, 3, 1] to [4, 5, 2] before the pivot, pivot = \"base_corner\": bounds [0, 0, 0, 2, 2, 1], origin [-2, -3, -1].",
    cube_gltf((-4.0, 1.0, 3.0), (-2.0, 2.0, 5.0))[0],
    CUBE_PACK + 'pivot = "base_corner"\n',
    stdout_value("bounds", "/bounds", "[0, 0, 0, 2, 2, 1]") + stdout_value("origin", "/origin", "[-2, -3, -1]"),
)


def build(id_: str, out: str, exit_: int = 0, against: str | None = None, extra: str = "") -> str:
    args = ["build", "world.toml"]
    if against:
        args += ["--against", against]
    args += ["--out", out]
    return f'\n[action.{id_}]\nvia         = {json.dumps(args)}\nexit        = {exit_}\nout         = "{out}"\ndiagnostics = "includes"\n{extra}'


def accept(id_: str, exit_: int = 0) -> str:
    return f'\n[action.{id_}]\nvia         = ["pack", "accept", "pack.toml"]\nexit        = {exit_}\ndiagnostics = "includes"\n'


WORLD_FILES = {"pack.toml": CUBE_PACK, "cube.gltf": unit_cube(), "world.toml": WORLD}

case(
    "unaccepted",
    {
        "case.toml": head("A model pack with no facts file: via build gives pack.unaccepted once, exit 0.", ["build"])
        + build("build", "build/w")
        + '\n[action.build.diag.unaccepted]\ncode = "pack.unaccepted"\n',
        **WORLD_FILES,
    },
)
case(
    "accept_refused",
    {
        "case.toml": head("A pack with an unreadable model: via pack accept gives pack.accept_refused, exit 1, no facts file written.", ["accept"])
        + accept("accept", 1)
        + '\n[action.accept.diag.refused]\ncode = "pack.accept_refused"\n'
        + '\n[action.accept.file.none]\npath   = "pack.facts.toml"\nexists = false\n',
        "pack.toml": CUBE_PACK,
        "cube.gltf": "this is not glTF\n",
    },
)

ACCEPT_PACK = """[taxon."building/house"]
defaults = { max_relief = 0.6, collision = "hull" }

[asset.cube]
file   = "cube.gltf"
taxon  = "building/house"
tags   = ["landmark"]
sink   = 0.2
params = { tint = "red" }

[asset.cube.mount.door]
at = [0, 0.5]

[asset.marker]
primitive = "box"
size      = [1, 1, 1]
"""
ACCEPT_MODEL = unit_cube()
ACCEPT_FACTS = (
    "# Written by via pack accept. Do not edit.\n"
    "\n"
    "[asset.cube]\n"
    "bounds = [-0.5, -0.5, 0, 0.5, 0.5, 1]\n"
    "origin = [0, 0, 0]\n"
    "footprint = [[-0.5, -0.5], [0.5, -0.5], [0.5, 0.5], [-0.5, 0.5]]\n"
    "triangles = 12\n"
    'front = "+z"\n'
    'up = "+y"\n'
    'pivot = "base_centre"\n'
    'solidity = "solid"\n'
    'collision = "hull"\n'
    "sink = 0.2\n"
    "max_relief = 0.6\n"
    "colour = [0.8, 0.8, 0.8]\n"
    'taxon = "building/house"\n'
    'tags = ["landmark"]\n'
    "mounts = { door = { at = [0, 0.5], face = 0 } }\n"
    f'params = "{digest(json.dumps({"tint": "red"}, separators=(",", ":")).encode())}"\n'
    f'files = {{ "cube.gltf" = "{digest(ACCEPT_MODEL.encode())}" }}\n'
)
case(
    "accept_bytes",
    {
        "case.toml": head("A model pack, via pack accept twice: the facts file equals the vector; the second run writes the same bytes.", ["first", "second"])
        + "\n# The vector is expected/pack.facts.toml, written from assets.md 4.1 by tools/packgen/cases.py:\n"
        + "# every key in 4.1's order, the primitive asset left out, the digests by an independent BLAKE3.\n"
        + accept("first")
        + '\n[action.first.file.bytes]\npath   = "pack.facts.toml"\nequals = "expected/pack.facts.toml"\n'
        + '\n[action.first.file.changed]\nstream = "stdout"\ntext   = "cube\\n"\n'
        + accept("second")
        + '\n[action.second.file.bytes]\npath   = "pack.facts.toml"\nequals = "expected/pack.facts.toml"\n'
        + '\n[action.second.file.changed]\nstream = "stdout"\ntext   = ""\n',
        "pack.toml": ACCEPT_PACK,
        "cube.gltf": ACCEPT_MODEL,
        "expected/pack.facts.toml": ACCEPT_FACTS,
    },
)
case(
    "bad_facts",
    {
        "case.toml": head("A facts file with a syntax error: via build gives pack.bad_facts, exit 1.", ["build"])
        + build("build", "build/w", 1)
        + '\n[action.build.diag.bad_facts]\ncode = "pack.bad_facts"\nfile = "pack:pack.facts.toml"\n',
        **WORLD_FILES,
        "pack.facts.toml": "# Written by via pack accept. Do not edit.\n\n[asset.cube\n",
    },
)
case(
    "stale_facts",
    {
        "case.toml": head("Accept, then delete an asset from the pack: via pack check gives pack.stale_facts naming it, exit 0.", ["accept", "drop", "check"])
        + accept("accept")
        + '\n[action.drop]\nreplace = "pack.toml"\nfind    = "[asset.post]\\nfile = \\"post.gltf\\"\\n"\nwith    = ""\n'
        + '\n[action.check]\nvia         = ["pack", "check", "pack.toml"]\nexit        = 0\ndiagnostics = "includes"\n'
        + '\n[action.check.diag.stale]\ncode = "pack.stale_facts"\ndata = { ids = ["post"] }\n',
        "pack.toml": CUBE_PACK + '\n[asset.post]\nfile = "post.gltf"\n',
        "cube.gltf": unit_cube(),
        "post.gltf": cube_gltf((-0.1, 0.0, -0.1), (0.1, 1.2, 0.1))[0],
    },
)
case(
    "new_asset",
    {
        "case.toml": head("Accept, then add a used model asset: via build gives asset.unaccepted for it.", ["accept", "add", "use", "build"])
        + accept("accept")
        + '\n[action.add]\nwrite = "pack.toml"\ntext  = "[asset.cube]\\nfile = \\"cube.gltf\\"\\n\\n[asset.post]\\nfile = \\"post.gltf\\"\\n"\n'
        + '\n[action.use]\nreplace = "world.toml"\nfind    = "[instance.b]\\nasset = \\"cube\\""\nwith    = "[instance.b]\\nasset = \\"post\\""\n'
        + build("build", "build/w")
        + '\n[action.build.diag.unaccepted]\ncode    = "asset.unaccepted"\nsubject = "asset.post"\n'
        + '\n[action.build.diag.cube]\ncode    = "asset.unaccepted"\nsubject = "asset.cube"\nrecords = 0\n',
        **WORLD_FILES,
        "post.gltf": cube_gltf((-0.1, 0.0, -0.1), (0.1, 1.2, 0.1))[0],
    },
)


def more_triangles() -> str:
    """The unit cube and a smaller box inside it: 24 triangles, the same bounds."""
    g = G()
    outer = g.positions(corners((-0.5, 0.0, -0.5), (0.5, 1.0, 0.5)))
    inner = g.positions(corners((-0.2, 0.2, -0.2), (0.2, 0.6, 0.2)))
    ix = g.indices(CUBE_IX)
    g.node(
        {"mesh": g.mesh([{"attributes": {"POSITION": outer}, "indices": ix, "mode": 4}, {"attributes": {"POSITION": inner}, "indices": ix, "mode": 4}])}
    )
    return g.text()[0]


REVISION_HEAD = accept("accept") + build("base", "build/base")
case(
    "geometry_only_revision",
    {
        "case.toml": head(
            "Accepted; model re-exported with more triangles, same bounds: asset.remodelled with old and new triangles; no instance moved in the change report.",
            ["accept", "base", "remodel", "build"],
        )
        + REVISION_HEAD
        + '\n[action.remodel]\nwrite = "cube.gltf"\nfrom  = "later/more.gltf"\n'
        + build("build", "build/next", against="build/base")
        + '\n[action.build.diag.remodelled]\ncode    = "asset.remodelled"\nsubject = "asset.cube"\ndata    = { old_triangles = 12, new_triangles = 24 }\n'
        + '\n[action.build.diag.revised]\ncode    = "asset.revised"\nrecords = 0\n'
        + '\n[action.build.check.unmoved]\nfn           = "changes_absent"\npath         = "build/next/changes.json"\nentry_names  = ["a", "b"]\n',
        **WORLD_FILES,
        "later/more.gltf": more_triangles(),
    },
)
WIDER = cube_gltf((-0.75, 0.0, -0.5), (0.75, 1.0, 0.5))[0]
CONTRACT = (
    REVISION_HEAD
    + '\n[action.widen]\nwrite = "cube.gltf"\nfrom  = "later/wider.gltf"\n'
    + build("build", "build/next", against="build/base")
    + '\n[action.build.diag.revised]\ncode    = "asset.revised"\nsubject = "asset.cube"\ndata    = { count = 2, instances = ["a", "b"] }\n'
    + '\n[action.build.check.changed]\nfn     = "changes_by_prefix"\npath   = "build/next/changes.json"\nprefix = "a"\nwhat   = ["changed"]\n'
    + '\n[action.build.file.cause]\npath     = "build/next/changes.json"\npointer  = "/entries/0/causes/0"\nincludes = { cause = "asset.cube.bounds" }\n'
)
case(
    "contract_revision",
    {
        "case.toml": head(
            "Accepted; model re-exported 0.5 m wider: asset.revised listing every instance of it; change report entries with cause asset.<asset id>.bounds.",
            ["accept", "base", "widen", "build"],
        )
        + "\n# Instances a and b stand on no ground, so they have no footprint feature: the change report's\n"
        + "# entries are the two instances, `changed` (assets.md 7), entry 0 being instance a.\n"
        + CONTRACT,
        **WORLD_FILES,
        "later/wider.gltf": WIDER,
    },
)
case(
    "accept",
    {
        "case.toml": head(
            "After the contract revision, via pack accept, then an incremental via build: no asset.revised; only the source: node of the facts file and the facts: nodes ran; the facts file diff shows only that asset.",
            ["accept", "base", "widen", "build", "again", "after"],
            pending="the graph has no facts:<asset> nodes yet (assets.md 4, graph.md 1): the facts file is a source leaf of the build key only",
        )
        + CONTRACT
        + accept("again")
        + '\n[action.again.file.changed]\nstream = "stdout"\ntext   = "cube\\n"\n'
        + build("after", "build/next", against="build/next")
        + '\n[action.after.diag.revised]\ncode    = "asset.revised"\nrecords = 0\n'
        + '\n[action.after.nodes]\nran = ["source:pack:pack.facts.toml", "facts:*"]\n',
        **WORLD_FILES,
        "pack.toml": CUBE_PACK + '\n[asset.post]\nfile = "post.gltf"\n',
        "post.gltf": cube_gltf((-0.1, 0.0, -0.1), (0.1, 1.2, 0.1))[0],
        "later/wider.gltf": WIDER,
    },
)
case(
    "refiled",
    {
        "case.toml": head("Accepted; one asset's taxon changed: asset.refiled; no asset.revised.", ["accept", "refile", "build"])
        + accept("accept")
        + '\n[action.refile]\nreplace = "pack.toml"\nfind    = "taxon = \\"plant/tree\\""\nwith    = "taxon = \\"plant/bush\\""\n'
        + build("build", "build/w")
        + '\n[action.build.diag.refiled]\ncode    = "asset.refiled"\nsubject = "asset.cube"\n'
        + '\n[action.build.diag.revised]\ncode    = "asset.revised"\nrecords = 0\n',
        **WORLD_FILES,
        "pack.toml": '[taxon."plant/tree"]\n[taxon."plant/bush"]\n\n' + CUBE_PACK + 'taxon = "plant/tree"\n',
    },
)
case(
    "unused_broken_asset",
    {
        "case.toml": head("A pack with one unreadable model the world does not use: build exit 0; check exit 1 with pack.bad_model.", ["build", "check"])
        + build("build", "build/w")
        + '\n[action.build.diag.bad_model]\ncode    = "pack.bad_model"\nrecords = 0\n'
        + '\n[action.check]\nvia         = ["pack", "check", "pack.toml"]\nexit        = 1\ndiagnostics = "includes"\n'
        + '\n[action.check.diag.bad_model]\ncode    = "pack.bad_model"\nsubject = "asset.junk"\n',
        **WORLD_FILES,
        "pack.toml": CUBE_PACK + '\n[asset.junk]\nfile = "junk.gltf"\n',
        "junk.gltf": "{ not glTF\n",
    },
)
case(
    "stand_ins",
    {
        "case.toml": head(
            "A world and an accepted pack, model files deleted: via build --stand-ins; every instance transform and footprint equal to the build with the files; budgets' triangles equal.",
            ["accept", "files", "drop", "stand_ins"],
        )
        + accept("accept")
        + build("files", "build/files")
        + '\n[action.drop]\ndelete = "cube.gltf"\n'
        + '\n[action.stand_ins]\nvia         = ["build", "world.toml", "--stand-ins", "--out", "build/stand"]\nexit        = 0\nout         = "build/stand"\ndiagnostics = "includes"\n'
        # The runner's `same` compares one path before and after; the two builds are two
        # directories, so each fact the build with the files gave is written out here.
        + '\n[action.stand_ins.check.a]\nfn    = "instance_at"\nbuild = "build/stand"\nname  = "a"\nat    = [20, 20]\n'
        + '\n[action.stand_ins.check.b]\nfn    = "instance_at"\nbuild = "build/stand"\nname  = "b"\nat    = [40, 40]\n'
        + '\n[action.stand_ins.file.triangles]\npath    = "build/stand/manifest.json"\npointer = "/assets/cube/triangles"\nvalue   = [12]\n'
        + '\n[action.stand_ins.file.bounds]\npath    = "build/stand/manifest.json"\npointer = "/assets/cube/bounds"\nvalue   = [-0.5, -0.5, 0, 0.5, 0.5, 1]\n'
        + '\n[action.stand_ins.file.footprint]\npath    = "build/stand/manifest.json"\npointer = "/assets/cube/footprint"\nvalue   = [[-0.5, -0.5], [0.5, -0.5], [0.5, 0.5], [-0.5, 0.5]]\n'
        + '\n[action.stand_ins.file.stand_in]\npath    = "build/stand/manifest.json"\npointer = "/assets/cube/stand_in"\nvalue   = true\n'
        + '\n[action.stand_ins.file.flag]\npath    = "build/stand/manifest.json"\npointer = "/run/flags/stand_ins"\nvalue   = true\n'
        + '\n[action.files.file.stand_in]\npath    = "build/files/manifest.json"\npointer = "/assets/cube/stand_in"\nvalue   = false\n',
        **WORLD_FILES,
    },
)
case(
    "stand_ins_stale",
    {
        "case.toml": head(
            "Accepted; a declared sink then edited in the pack: via build --stand-ins gives asset.stand_in_stale, exit 1.",
            ["accept", "sink", "stand_ins"],
        )
        + accept("accept")
        + '\n[action.sink]\nwrite = "pack.toml"\ntext  = "[asset.cube]\\nfile = \\"cube.gltf\\"\\nsink = 0.3\\n"\n'
        + '\n[action.stand_ins]\nvia         = ["build", "world.toml", "--stand-ins", "--out", "build/stand"]\nexit        = 1\nout         = "build/stand"\ndiagnostics = "includes"\n'
        + '\n[action.stand_ins.diag.stale]\ncode    = "asset.stand_in_stale"\nsubject = "asset.cube"\n',
        **WORLD_FILES,
    },
)
case(
    "stand_ins_no_facts",
    {
        "case.toml": head("--stand-ins on a pack with model assets and no facts file: build.no_facts, exit 1.", ["stand_ins"])
        + '\n[action.stand_ins]\nvia         = ["build", "world.toml", "--stand-ins", "--out", "build/stand"]\nexit        = 1\nout         = "build/stand"\ndiagnostics = "includes"\n'
        + '\n[action.stand_ins.diag.no_facts]\ncode = "build.no_facts"\nfrom = "run"\n',
        **WORLD_FILES,
    },
)
# rules.md 2: `ground` on an asset with a point footprint. The flat card measures one (2.4).
case(
    "point_footprint",
    {
        "case.toml": head("ground on an asset whose measured footprint is a point: place.no_footprint at the instance's ground.", ["build"])
        + '\n[action.build]\nvia         = ["build", "world.toml", "--lib", "{work}/libs", "--out", "build/w"]\nexit        = 1\nout         = "build/w"\ndiagnostics = "includes"\n'
        + '\n[action.build.diag.no_footprint]\ncode    = "place.no_footprint"\nsubject = "instance.a"\nline    = 14\n',
        "pack.toml": CUBE_PACK,
        "cube.gltf": flat_card(),
        "world.toml": WORLD.replace('pack       = "pack.toml"\n', 'pack       = "pack.toml"\ninclude    = ["kinds"]\n').replace('at    = [20, 20]\n', 'at    = [20, 20]\nground = "pad"\n', 1),
        "libs/kinds/kinds.toml": '[kind.pad]\nauthority = "shaped"\nradius    = 4\n\n[kind.pad.effect.level]\nfield = "role:surface"\nvalue = "= under(\'mean\')"\nmask  = "= 1 - smoothstep(0, 3, d)"\n',
    },
)
case(
    "collision_default",
    {
        "case.toml": head(
            'A model asset with no collision; one with collision = "hul": via pack list gives collision = "none"; via pack check gives doc.type, hint hull, exit 1.',
            ["list", "check"],
        )
        + '\n[action.list]\nvia         = ["pack", "list", "pack.toml"]\nexit        = 0\ndiagnostics = "includes"\n'
        + stdout_value("collision", "/collision", '"none"')
        + '\n[action.check]\nvia         = ["pack", "check", "typo.toml"]\nexit        = 1\ndiagnostics = "includes"\n'
        + '\n[action.check.diag.type]\ncode    = "doc.type"\nsubject = "asset.cube.collision"\ndata    = { nearest = "hull" }\n',
        "pack.toml": CUBE_PACK,
        "typo.toml": CUBE_PACK + 'collision = "hul"\n',
        "cube.gltf": unit_cube(),
    },
)
case(
    "collision_by_kit",
    {
        "case.toml": head(
            '[taxon."building"] defaults = { collision = "mesh" }, a house under it, a primitive box with collision = "box": via pack list gives mesh for the house, box for the box; no asset.declared_measured.',
            ["list"],
        )
        + '\n[action.list]\nvia         = ["pack", "list", "pack.toml"]\nexit        = 0\ndiagnostics = "includes"\n'
        + stdout_value("box", "/collision", '"box"', line=1)
        + stdout_value("house", "/collision", '"mesh"', line=2)
        + '\n[action.list.diag.declared_measured]\ncode    = "asset.declared_measured"\nrecords = 0\n',
        "pack.toml": '[taxon."building"]\ndefaults = { collision = "mesh" }\n\n[taxon."building/house"]\n\n'
        + '[asset.house]\nfile  = "cube.gltf"\ntaxon = "building/house"\n\n'
        + '[asset.box]\nprimitive = "box"\nsize      = [1, 1, 1]\ncollision = "box"\n',
        "cube.gltf": unit_cube(),
    },
)
case(
    "collision_revised",
    {
        "case.toml": head(
            "Accepted; one asset's collision edited none to hull: asset.remodelled; no asset.revised; no instance moved in the change report; the build's asset record holds hull.",
            ["accept", "base", "edit", "build"],
        )
        + REVISION_HEAD
        + '\n[action.edit]\nwrite = "pack.toml"\ntext  = "[asset.cube]\\nfile = \\"cube.gltf\\"\\ncollision = \\"hull\\"\\n"\n'
        + build("build", "build/next", against="build/base")
        + '\n[action.build.diag.remodelled]\ncode    = "asset.remodelled"\nsubject = "asset.cube"\n'
        + '\n[action.build.diag.revised]\ncode    = "asset.revised"\nrecords = 0\n'
        + '\n[action.build.check.unmoved]\nfn          = "changes_absent"\npath        = "build/next/changes.json"\nentry_names = ["a", "b"]\n'
        + '\n[action.build.file.hull]\npath    = "build/next/manifest.json"\npointer = "/assets/cube/collision"\nvalue   = "hull"\n',
        **WORLD_FILES,
    },
)
case(
    "usage",
    {
        "case.toml": head(
            "A pack of three assets, a build placing two of them: via pack list PACK --build DIR gives instances on each line, the two counts and 0 for the third; nothing built.",
            ["build", "list"],
        )
        + build("build", "build/w")
        + '\n[action.list]\nvia         = ["pack", "list", "pack.toml", "--build", "build/w"]\nexit        = 0\ndiagnostics = "includes"\nuntouched   = ["build/w"]\n'
        + stdout_value("a", "/instances", "2", line=1)
        + stdout_value("b", "/instances", "1", line=2)
        + stdout_value("c", "/instances", "0", line=3),
        "pack.toml": '[asset.cube]\nprimitive = "box"\nsize      = [1, 1, 1]\n\n[asset.post]\nprimitive = "box"\nsize      = [0.2, 0.2, 1]\n\n[asset.stone]\nprimitive = "sphere"\nsize      = [1, 1, 1]\n',
        "world.toml": WORLD + '\n[instance.c]\nasset = "post"\nat    = [10, 50]\n',
    },
)
case(
    "same_bytes_m2",
    {
        "case.toml": head("The contract revision case built at 1 and 8 threads: same build digest (R26).", ["accept", "widen", "one", "eight"])
        + accept("accept")
        + '\n[action.widen]\nwrite = "cube.gltf"\nfrom  = "later/wider.gltf"\n'
        + '\n[action.one]\nvia         = ["build", "world.toml", "--threads", "1", "--out", "build/one"]\nexit        = 0\nout         = "build/one"\ndiagnostics = "includes"\n'
        + '\n[action.eight]\nvia         = ["build", "world.toml", "--threads", "8", "--out", "build/eight"]\nexit        = 0\nout         = "build/eight"\ndiagnostics = "includes"\n'
        + '\n[action.eight.same.id]\naction = "one"\nwhat   = "id"\n',
        **WORLD_FILES,
        "later/wider.gltf": WIDER,
    },
)


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--check", action="store_true")
    a = ap.parse_args()
    differ = []
    for name, files in sorted(CASES.items()):
        for rel, content in sorted(files.items()):
            path = ROOT / name / rel
            data = content.encode() if isinstance(content, str) else content
            if a.check:
                if not path.is_file() or path.read_bytes() != data:
                    differ.append(f"{name}/{rel}")
            else:
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(data)
    if differ:
        print("differ: " + ", ".join(differ), file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
