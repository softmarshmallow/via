"""Writes the starter pack: small procedural glTF models made by us, so examples need no download.

    python3 tools/packgen/packgen.py            # writes lib/packs/starter/*.glb
    python3 tools/packgen/packgen.py --check    # exits 1 when a file on disk differs

Everything is made here from boxes, prisms, cylinders, cones and ellipsoids: no outside art, so the
pack is ours to redistribute under the repository's licence. The output is byte-stable on one
system: values are computed in a fixed order, packed as IEEE 754 single precision, and the JSON is
written with sorted keys. Python's `math.cos` is the platform's, so another system may differ in a
last bit; the committed files are the reference, and `--check` says when a run would change them.

The pack text, `lib/packs/starter.toml`, is written by hand beside it; its facts file,
`lib/packs/starter.facts.toml`, by `via pack accept lib/packs/starter.toml` after this script.

Models follow glTF's convention (`assets.md` 2.3): +Y up, the front faces +Z, metres, the origin at
the base centre, so every asset takes the default `front`, `up` and `pivot` and no pivot moves. The
ids and taxa are those of the primitive pack (`assets.md` 6.1), with a few sizes more, so a world
swaps between the two packs without a text change.
"""

from __future__ import annotations

import argparse
import json
import math
import struct
import sys
import zlib
from pathlib import Path

HERE = Path(__file__).resolve().parent
OUT = HERE.parent.parent / "lib" / "packs" / "starter"

Tri = tuple[tuple[float, float, float], tuple[float, float, float], tuple[float, float, float]]


# --- shapes -----------------------------------------------------------------------------------


def box(x0: float, y0: float, z0: float, x1: float, y1: float, z1: float) -> list[Tri]:
    """A box, counter-clockwise outward."""
    p = [
        (x0, y0, z0), (x1, y0, z0), (x1, y1, z0), (x0, y1, z0),
        (x0, y0, z1), (x1, y0, z1), (x1, y1, z1), (x0, y1, z1),
    ]
    quads = [
        (0, 3, 2, 1),  # -z
        (4, 5, 6, 7),  # +z
        (0, 1, 5, 4),  # -y
        (3, 7, 6, 2),  # +y
        (0, 4, 7, 3),  # -x
        (1, 2, 6, 5),  # +x
    ]
    out: list[Tri] = []
    for a, b, c, d in quads:
        out.append((p[a], p[b], p[c]))
        out.append((p[a], p[c], p[d]))
    return out


def roof(x0: float, x1: float, y0: float, y1: float, z0: float, z1: float) -> list[Tri]:
    """A gable roof: a prism over the box's top face, its ridge along x at height y1."""
    zm = (z0 + z1) / 2
    a, b = (x0, y0, z0), (x1, y0, z0)
    c, d = (x1, y0, z1), (x0, y0, z1)
    r0, r1 = (x0, y1, zm), (x1, y1, zm)
    return [
        (a, r1, b), (a, r0, r1),  # back slope (-z)
        (d, c, r1), (d, r1, r0),  # front slope (+z)
        (a, d, r0),  # gable -x
        (b, r1, c),  # gable +x
        (a, b, c), (a, c, d),  # underside
    ]


def ring(cx: float, cz: float, r: float, n: int) -> list[tuple[float, float]]:
    return [(cx + r * math.cos(2 * math.pi * k / n), cz - r * math.sin(2 * math.pi * k / n)) for k in range(n)]


def cylinder(cx: float, cz: float, y0: float, y1: float, r: float, n: int) -> list[Tri]:
    pts = ring(cx, cz, r, n)
    out: list[Tri] = []
    for k in range(n):
        (xa, za), (xb, zb) = pts[k], pts[(k + 1) % n]
        out.append(((xa, y0, za), (xb, y0, zb), (xb, y1, zb)))
        out.append(((xa, y0, za), (xb, y1, zb), (xa, y1, za)))
        out.append(((cx, y1, cz), (xa, y1, za), (xb, y1, zb)))
        out.append(((cx, y0, cz), (xb, y0, zb), (xa, y0, za)))
    return out


def cone(cx: float, cz: float, y0: float, y1: float, r: float, n: int) -> list[Tri]:
    pts = ring(cx, cz, r, n)
    out: list[Tri] = []
    for k in range(n):
        (xa, za), (xb, zb) = pts[k], pts[(k + 1) % n]
        out.append(((xa, y0, za), (xb, y0, zb), (cx, y1, cz)))
        out.append(((cx, y0, cz), (xb, y0, zb), (xa, y0, za)))
    return out


def ellipsoid(
    c: tuple[float, float, float],
    r: tuple[float, float, float],
    seg: int,
    rings: int,
    bump: float = 0.0,
    seed: int = 0,
) -> list[Tri]:
    """An ellipsoid of `seg` sides and `rings` bands; `bump` roughens it (a rock) by a fixed hash."""

    def jitter(i: int, j: int) -> float:
        if bump == 0.0 or j in (0, rings):
            return 1.0
        h = (i * 73856093) ^ (j * 19349663) ^ (seed * 83492791)
        h = (h * 2654435761) & 0xFFFFFFFF
        return 1.0 - bump + bump * 2.0 * (h / 0xFFFFFFFF)

    def p(i: int, j: int) -> tuple[float, float, float]:
        lat = math.pi * j / rings - math.pi / 2
        lon = 2 * math.pi * (i % seg) / seg
        s = jitter(i % seg, j)
        return (
            c[0] + r[0] * s * math.cos(lat) * math.cos(lon),
            c[1] + r[1] * math.sin(lat),
            c[2] - r[2] * s * math.cos(lat) * math.sin(lon),
        )

    out: list[Tri] = []
    for j in range(rings):
        for i in range(seg):
            a, b, cc, d = p(i, j), p(i + 1, j), p(i + 1, j + 1), p(i, j + 1)
            if j > 0:
                out.append((a, b, cc))
            if j < rings - 1:
                out.append((a, cc, d))
    return out


def lift(tris: list[Tri]) -> list[Tri]:
    """Moves a shape so its lowest point is at height 0 and its plan is centred on the origin: the
    origin is then the base centre, where `assets.md` 2.4's default pivot puts it."""
    xs = [v[0] for t in tris for v in t]
    zs = [v[2] for t in tris for v in t]
    cx, cz = (min(xs) + max(xs)) / 2, (min(zs) + max(zs)) / 2
    low = min(v[1] for t in tris for v in t)
    return [tuple((v[0] - cx, v[1] - low, v[2] - cz) for v in t) for t in tris]  # type: ignore[misc]


# --- glTF -------------------------------------------------------------------------------------

MATERIALS = {
    "bark": [0.32, 0.22, 0.14, 1.0],
    "leaf": [0.2, 0.38, 0.13, 1.0],
    "needle": [0.12, 0.28, 0.14, 1.0],
    "wall": [0.78, 0.72, 0.6, 1.0],
    "roof": [0.45, 0.2, 0.15, 1.0],
    "timber": [0.5, 0.36, 0.24, 1.0],
    "stone": [0.48, 0.46, 0.43, 1.0],
    "iron": [0.15, 0.15, 0.15, 1.0],
    "fern": [0.25, 0.45, 0.18, 1.0],
}


def png(size: int = 8) -> bytes:
    """A small grey grain texture, multiplied by each material's base colour factor."""
    rows = b""
    for y in range(size):
        row = bytearray([0])
        for x in range(size):
            v = 200 + ((x * 7 + y * 13 + (x * y) % 5) % 5) * 10
            row += bytes([v, v, v])
        rows += bytes(row)

    def chunk(kind: bytes, data: bytes) -> bytes:
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data) & 0xFFFFFFFF)

    ihdr = struct.pack(">IIBBBBB", size, size, 8, 2, 0, 0, 0)
    return b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", ihdr) + chunk(b"IDAT", zlib.compress(rows, 9)) + chunk(b"IEND", b"")


def f32(x: float) -> float:
    return struct.unpack("<f", struct.pack("<f", x))[0]


def glb(parts: list[tuple[str, list[Tri]]]) -> bytes:
    """One mesh, one primitive per part, flat normals, planar UVs, one shared texture."""
    bin_ = bytearray()
    views: list[dict] = []
    accessors: list[dict] = []
    primitives: list[dict] = []
    used = sorted({m for m, _ in parts})
    mat_index = {m: i for i, m in enumerate(used)}

    def view(data: bytes, target: int | None) -> int:
        while len(bin_) % 4:
            bin_.append(0)
        v: dict = {"buffer": 0, "byteOffset": len(bin_), "byteLength": len(data)}
        if target is not None:
            v["target"] = target
        bin_.extend(data)
        views.append(v)
        return len(views) - 1

    for material, tris in parts:
        pos: list[float] = []
        nor: list[float] = []
        uv: list[float] = []
        for a, b, c in tris:
            u = [b[k] - a[k] for k in range(3)]
            w = [c[k] - a[k] for k in range(3)]
            n = [u[1] * w[2] - u[2] * w[1], u[2] * w[0] - u[0] * w[2], u[0] * w[1] - u[1] * w[0]]
            length = math.sqrt(sum(x * x for x in n)) or 1.0
            n = [x / length for x in n]
            for v in (a, b, c):
                pos += [f32(v[0]), f32(v[1]), f32(v[2])]
                nor += n
                uv += [v[0] + v[2], -v[1]]
        count = len(pos) // 3
        lo = [min(pos[k::3]) for k in range(3)]
        hi = [max(pos[k::3]) for k in range(3)]
        pv = view(struct.pack(f"<{len(pos)}f", *pos), 34962)
        nv = view(struct.pack(f"<{len(nor)}f", *nor), 34962)
        tv = view(struct.pack(f"<{len(uv)}f", *uv), 34962)
        base = len(accessors)
        accessors += [
            {"bufferView": pv, "componentType": 5126, "count": count, "type": "VEC3", "min": lo, "max": hi},
            {"bufferView": nv, "componentType": 5126, "count": count, "type": "VEC3"},
            {"bufferView": tv, "componentType": 5126, "count": count, "type": "VEC2"},
        ]
        primitives.append(
            {
                "attributes": {"POSITION": base, "NORMAL": base + 1, "TEXCOORD_0": base + 2},
                "material": mat_index[material],
                "mode": 4,
            }
        )
    image_view = view(png(), None)
    while len(bin_) % 4:
        bin_.append(0)
    doc = {
        "asset": {"version": "2.0", "generator": "via tools/packgen"},
        "scene": 0,
        "scenes": [{"nodes": [0]}],
        "nodes": [{"mesh": 0}],
        "meshes": [{"primitives": primitives}],
        "materials": [
            {
                "name": m,
                "pbrMetallicRoughness": {
                    "baseColorFactor": MATERIALS[m],
                    "baseColorTexture": {"index": 0},
                    "metallicFactor": 0.0,
                    "roughnessFactor": 0.9,
                },
            }
            for m in used
        ],
        "textures": [{"source": 0, "sampler": 0}],
        "samplers": [{"magFilter": 9729, "minFilter": 9987, "wrapS": 10497, "wrapT": 10497}],
        "images": [{"bufferView": image_view, "mimeType": "image/png"}],
        "accessors": accessors,
        "bufferViews": views,
        "buffers": [{"byteLength": len(bin_)}],
    }
    text = json.dumps(doc, sort_keys=True, separators=(",", ":")).encode()
    while len(text) % 4:
        text += b" "
    total = 12 + 8 + len(text) + 8 + len(bin_)
    return (
        struct.pack("<III", 0x46546C67, 2, total)
        + struct.pack("<II", len(text), 0x4E4F534A)
        + text
        + struct.pack("<II", len(bin_), 0x004E4942)
        + bytes(bin_)
    )


# --- the assets -------------------------------------------------------------------------------


def house(width: float, depth: float, wall: float, ridge: float) -> list[tuple[str, list[Tri]]]:
    """A house: walls and a gable roof, the front (+z) with a door."""
    w, d = width / 2, depth / 2
    door = box(-0.5, 0.0, d, 0.5, 2.1, d + 0.05)
    return [
        ("wall", box(-w, 0.0, -d, w, wall, d)),
        ("roof", roof(-w - 0.3, w + 0.3, wall, ridge, -d - 0.3, d + 0.3)),
        ("timber", door),
    ]


def shed(width: float, depth: float, ridge: float) -> list[tuple[str, list[Tri]]]:
    w, d = width / 2, depth / 2
    return [
        ("timber", box(-w, 0.0, -d, w, ridge * 0.55, d)),
        ("roof", roof(-w, w, ridge * 0.55, ridge, -d, d)),
    ]


def broadleaf(height: float, crown: float, seg: int, rings: int) -> list[tuple[str, list[Tri]]]:
    trunk_h = height * 0.4
    return [
        ("bark", cylinder(0.0, 0.0, 0.0, trunk_h + 0.5, 0.25 * height / 7, max(seg // 2, 6))),
        ("leaf", ellipsoid((0.0, trunk_h + (height - trunk_h) / 2, 0.0), (crown / 2, (height - trunk_h) / 2, crown / 2), seg, rings)),
    ]


def conifer(height: float, spread: float, seg: int) -> list[tuple[str, list[Tri]]]:
    return [
        ("bark", cylinder(0.0, 0.0, 0.0, height * 0.3, 0.2, max(seg // 2, 6))),
        ("needle", cone(0.0, 0.0, height * 0.15, height, spread / 2, seg)),
    ]


def bush(width: float, height: float, seg: int, rings: int) -> list[tuple[str, list[Tri]]]:
    return [("leaf", lift(ellipsoid((0.0, 0.0, 0.0), (width / 2, height / 2, width / 2), seg, rings)))]


def fern(width: float, height: float) -> list[tuple[str, list[Tri]]]:
    return [("fern", cone(0.0, 0.0, 0.0, height, width / 2, 10))]


def rock(w: float, d: float, h: float, seed: int) -> list[tuple[str, list[Tri]]]:
    return [("stone", lift(ellipsoid((0.0, 0.0, 0.0), (w / 2, h, d / 2), 10, 6, bump=0.15, seed=seed)))]


def lamp(height: float) -> list[tuple[str, list[Tri]]]:
    return [
        ("iron", cylinder(0.0, 0.0, 0.0, height - 0.3, 0.08, 8)),
        ("iron", box(-0.1, height - 0.3, -0.1, 0.1, height, 0.1)),
    ]


def post(width: float, height: float) -> list[tuple[str, list[Tri]]]:
    return [("timber", box(-width / 2, 0.0, -width / 2, width / 2, height, width / 2))]


def log(length: float, radius: float) -> list[tuple[str, list[Tri]]]:
    """A fallen log lying along x: a cylinder turned onto its side, resting on the ground."""
    c = cylinder(0.0, 0.0, -length / 2, length / 2, radius, 10)
    # The cylinder stands along y; lie it along x (y to x, x to y), then rest it on the ground.
    return [("bark", lift([tuple((v[1], v[0], v[2]) for v in t) for t in c]))]  # type: ignore[misc]


def crate(size: float) -> list[tuple[str, list[Tri]]]:
    s = size / 2
    return [("timber", box(-s, 0.0, -s, s, size, s))]


FILES = {
    "house_low.glb": house(8.0, 6.0, 3.0, 4.6),
    "house_wide.glb": house(11.0, 7.0, 3.2, 5.4),
    "shed_gable.glb": shed(4.0, 3.0, 3.0),
    "tree_oak.glb": broadleaf(7.0, 5.0, 16, 8),
    "tree_oak_low.glb": broadleaf(7.0, 5.0, 8, 4),
    "tree_oak_large.glb": broadleaf(11.0, 8.0, 16, 8),
    "tree_pine.glb": conifer(9.0, 3.6, 12),
    "bush_round.glb": bush(1.5, 1.2, 10, 6),
    "bush_wide.glb": bush(2.6, 1.6, 12, 6),
    "fern_low.glb": fern(0.8, 0.5),
    "rock_a.glb": rock(1.2, 1.0, 0.6, 1),
    "rock_b.glb": rock(2.4, 1.8, 1.3, 2),
    "box_small.glb": crate(2.0),
    "lamp_post.glb": lamp(3.0),
    "fence_post.glb": post(0.15, 1.2),
    "log.glb": log(3.0, 0.25),
    "barn.glb": shed(14.0, 9.0, 8.0),
}


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--check", action="store_true", help="exit 1 when a file on disk differs")
    a = ap.parse_args()
    OUT.mkdir(parents=True, exist_ok=True)
    differ = []
    for name, parts in sorted(FILES.items()):
        data = glb(parts)
        path = OUT / name
        if a.check:
            if not path.is_file() or path.read_bytes() != data:
                differ.append(name)
        else:
            path.write_bytes(data)
    if differ:
        print("differ: " + ", ".join(differ), file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
