"""Writes the hand-made fixture builds the Godot loader suite reads.

    python3 engines/godot/tests/make_fixture.py          # write them
    python3 engines/godot/tests/make_fixture.py --check  # exit 1 if the files on disk differ

Two build directories in the v0 format of spec/build.md, written without the engine (which may not
exist yet when the loader is tested): `fixture_build/` and `fixture_build+downstream/`, the same
small world before and after its ford moves downstream. Plus `fixture_pack/`, a pack root holding
one material image, which the manifests reach through `locations.pack_root`.

What they exercise: four chunks, the last ones narrower (5); a whole-extent surface field (6.1); a
`material` weights field with four materials, chunks naming different subsets and one unpainted
patch (6.2); a water surface field tagged `water_level` that equals the ground where dry (6.4); a
`keep_out` field no loader draws; every primitive shape (assets.md 6); a set item whose name holds
`.`; collision in `params`; features of every shape; bookmarks, one of them bad in the after build;
meetings. Python's standard library only. Digests are BLAKE3 (3.3), in pure Python below, checked
against the published empty-input vector at start.
"""

import json
import math
import struct
import sys
import zlib
from pathlib import Path

HERE = Path(__file__).resolve().parent

# --- BLAKE3 (O'Connor, Aumasson, Neves, Wilcox-O'Hearn, 2020), hash mode, 32-byte output -----------

IV = [0x6A09E667, 0xBB67AE85, 0x3C6EF372, 0xA54FF53A, 0x510E527F, 0x9B05688C, 0x1F83D9AB, 0x5BE0CD19]
PERM = [2, 6, 3, 10, 7, 0, 4, 13, 1, 11, 12, 5, 9, 14, 15, 8]
CHUNK_START, CHUNK_END, PARENT, ROOT = 1, 2, 4, 8
M32 = 0xFFFFFFFF


def _rotr(x, n):
    return ((x >> n) | (x << (32 - n))) & M32


def _g(s, a, b, c, d, x, y):
    s[a] = (s[a] + s[b] + x) & M32
    s[d] = _rotr(s[d] ^ s[a], 16)
    s[c] = (s[c] + s[d]) & M32
    s[b] = _rotr(s[b] ^ s[c], 12)
    s[a] = (s[a] + s[b] + y) & M32
    s[d] = _rotr(s[d] ^ s[a], 8)
    s[c] = (s[c] + s[d]) & M32
    s[b] = _rotr(s[b] ^ s[c], 7)


def _compress(cv, m, counter, block_len, flags):
    s = list(cv) + IV[:4] + [counter & M32, (counter >> 32) & M32, block_len, flags]
    m = list(m)
    for r in range(7):
        _g(s, 0, 4, 8, 12, m[0], m[1])
        _g(s, 1, 5, 9, 13, m[2], m[3])
        _g(s, 2, 6, 10, 14, m[4], m[5])
        _g(s, 3, 7, 11, 15, m[6], m[7])
        _g(s, 0, 5, 10, 15, m[8], m[9])
        _g(s, 1, 6, 11, 12, m[10], m[11])
        _g(s, 2, 7, 8, 13, m[12], m[13])
        _g(s, 3, 4, 9, 14, m[14], m[15])
        if r < 6:
            m = [m[i] for i in PERM]
    for i in range(8):
        s[i] ^= s[i + 8]
        s[i + 8] ^= cv[i]
    return s


def _words(block):
    return list(struct.unpack("<16I", block.ljust(64, b"\0")))


def _chunk_output(chunk, counter):
    """(cv, block words, counter, block_len, flags) of a chunk's last block, not yet compressed."""
    cv = IV
    blocks = [chunk[i:i + 64] for i in range(0, len(chunk), 64)] or [b""]
    for n, block in enumerate(blocks[:-1]):
        cv = _compress(cv, _words(block), counter, 64, CHUNK_START if n == 0 else 0)[:8]
    flags = CHUNK_END | (CHUNK_START if len(blocks) == 1 else 0)
    return (cv, _words(blocks[-1]), counter, len(blocks[-1]), flags)


def _cv(out):
    cv, m, counter, block_len, flags = out
    return _compress(cv, m, counter, block_len, flags)[:8]


def _parent(left, right):
    return (IV, left + right, 0, 64, PARENT)


def blake3(data):
    chunks = [data[i:i + 1024] for i in range(0, len(data), 1024)] or [b""]
    stack = []
    for n, chunk in enumerate(chunks[:-1]):
        cv = _cv(_chunk_output(chunk, n))
        total = n + 1
        while total & 1 == 0:
            cv = _cv(_parent(stack.pop(), cv))
            total >>= 1
        stack.append(cv)
    out = _chunk_output(chunks[-1], len(chunks) - 1)
    for left in reversed(stack):
        out = _parent(left, _cv(out))
    cv, m, counter, block_len, flags = out
    words = _compress(cv, m, counter, block_len, flags | ROOT)[:8]
    return struct.pack("<8I", *words).hex()


assert blake3(b"") == "af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262"

# --- encodings (build.md 3.1, 3.2) ----------------------------------------------------------------


def canonical(value):
    """Canonical JSON: keys sorted bytewise, no white space, shortest round-trip numbers, LF."""
    return (json.dumps(_norm(value), sort_keys=True, separators=(",", ":"), ensure_ascii=False)
            + "\n").encode("utf-8")


def _norm(v):
    if isinstance(v, float):
        if not math.isfinite(v):
            raise ValueError("no non-finite value enters a build")
        if v == 0:
            return 0
        if v == int(v) and abs(v) < 2 ** 53:
            return int(v)
        return v
    if isinstance(v, dict):
        return {k: _norm(x) for k, x in v.items()}
    if isinstance(v, (list, tuple)):
        return [_norm(x) for x in v]
    return v


def npy(descr, shape, values):
    """An array file: protocol.md 4.2's header, smallest padding, then the data little-endian."""
    sys.path.insert(0, str(HERE.parents[2] / "sdk" / "python"))
    from viasdk._npy import header  # the SDK's header writer: one format, one writer

    fmt = {"<f4": "<%df", "<i4": "<%di", "|u1": "%dB"}[descr]
    count = 1
    for d in shape:
        count *= d
    assert len(values) == count, (descr, shape, len(values))
    if descr == "<f4":
        values = [float(struct.unpack("<f", struct.pack("<f", v))[0]) + 0.0 for v in values]
    return header(descr, shape) + struct.pack(fmt % count, *values)


def f32(v):
    return struct.unpack("<f", struct.pack("<f", v))[0] + 0.0


def png_rgb(width, height, pixel):
    rows = b"".join(b"\0" + bytes(c for x in range(width) for c in pixel(x, y)) for y in range(height))

    def chunk(kind, data):
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data))

    return (b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 2, 0, 0, 0))
            + chunk(b"IDAT", zlib.compress(rows, 9)) + chunk(b"IEND", b""))


# --- the world --------------------------------------------------------------------------------------

EXTENT = (0, 0, 96, 80)
RES = 1.0
NX, NY = 97, 81
K = 64  # chunk side 64 m at 1 m: build.md 5's default


def beck_x(y):
    return 60 + 6 * math.sin(y / 20)


def bank(x, y):
    return 20 + 0.06 * y + 2 * math.sin(x / 13) * math.cos(y / 17)


def ground(x, y):
    d = (x - beck_x(y)) / 4
    return bank(x, y) - 3 * math.exp(-d * d)


def water(x, y):
    """The beck's surface in its channel; elsewhere the ground itself, so depth is never below 0."""
    if abs(x - beck_x(y)) > 6:
        return ground(x, y)
    level = bank(beck_x(y), y) - 1.5
    return max(ground(x, y), level)


def post_xy(i, j):
    return EXTENT[0] + i * RES, EXTENT[1] + j * RES


def grid(fn):
    return [f32(fn(*post_xy(i, j))) for j in range(NY) for i in range(NX)]


def zat(x, y):
    return f32(ground(x, y))


def mm(v):
    return int(round(v * 1000))


def metres(v_mm):
    return v_mm / 1000


# Materials of the pack, in id order. `gravel` has a file, reached through pack_root.
MATERIALS = {
    "grass": {"colour": [0.2, 0.4, 0.1]},
    "gravel": {"colour": [0.55, 0.52, 0.46], "file": "textures/gravel.png", "repeat": 4},
    "mud": {"colour": [0.35, 0.28, 0.2]},
    "rock": {"colour": [0.45, 0.43, 0.4]},
}


def lane_distance(x, y, lanes):
    best = 1e9
    for pts in lanes:
        for (ax, ay, _), (bx, by, _) in zip(pts, pts[1:]):
            dx, dy = bx - ax, by - ay
            t = max(0.0, min(1.0, ((x - ax) * dx + (y - ay) * dy) / (dx * dx + dy * dy)))
            best = min(best, math.hypot(x - ax - t * dx, y - ay - t * dy))
    return best


def weights_at(x, y, lanes):
    """Weights per material in id order, before quantising: 0 or above (fields.md 5.2)."""
    if x < 6 and y > 74:
        return [0.0, 0.0, 0.0, 0.0]  # unpainted: the loader draws its own default
    gx = (ground(x + 1, y) - ground(x - 1, y)) / 2
    gy = (ground(x, y + 1) - ground(x, y - 1)) / 2
    slope = math.hypot(gx, gy)
    depth = water(x, y) - ground(x, y)
    lane = lane_distance(x, y, lanes)
    grass = 1.0
    gravel = 3.0 if lane < 2 else (1.0 if lane < 3 else 0.0)
    mud = 2.0 * min(1.0, depth / 0.5) if depth > 0 else (0.6 if abs(x - beck_x(y)) < 7 else 0.0)
    rock = max(0.0, slope - 0.35) * 6
    return [grass, gravel, mud, rock]


def hamilton(ws):
    """Bytes of one post: largest remainders to 255, ties to the lower material (build.md 6.2)."""
    s = 0.0
    for w in ws:
        s += w
    if s == 0:
        return [0] * len(ws)
    q = [w * 255 / s for w in ws]
    b = [math.floor(v) for v in q]
    order = sorted(range(len(ws)), key=lambda m: (-(q[m] - b[m]), m))
    for m in order[:255 - sum(b)]:
        b[m] += 1
    return b


def chunks():
    out = []
    ncx = max(1, math.ceil((NX - 1) / K))
    ncy = max(1, math.ceil((NY - 1) / K))
    for cy in range(ncy):
        for cx in range(ncx):
            out.append((cx, cy, cx * K, cy * K, min((cx + 1) * K, NX - 1), min((cy + 1) * K, NY - 1)))
    return out


def chunk_of(x_mm, y_mm):
    c = K * RES * 1000
    ncx = max(1, math.ceil((NX - 1) / K))
    ncy = max(1, math.ceil((NY - 1) / K))
    cx = min(max((x_mm - EXTENT[0] * 1000) // int(c), 0), ncx - 1)
    cy = min(max((y_mm - EXTENT[1] * 1000) // int(c), 0), ncy - 1)
    return cx, cy


ASSETS = {
    "bush_round": {"primitive": "sphere", "size": [1.5, 1.5, 1.2], "taxon": "plant/bush",
                   "colour": [0.22, 0.4, 0.15], "solidity": "none", "triangles": 224, "params": {}},
    "box_small": {"primitive": "box", "size": [2, 2, 2], "taxon": "prop/box",
                  "colour": [0.6, 0.5, 0.35], "solidity": "solid", "triangles": 12, "params": {}},
    "fence_post": {"primitive": "box", "size": [0.15, 0.15, 1.2], "taxon": "prop/fence",
                   "colour": [0.4, 0.3, 0.2], "solidity": "solid", "triangles": 12, "params": {}},
    "fern_low": {"primitive": "cone", "size": [0.8, 0.8, 0.5], "taxon": "plant/fern",
                 "colour": [0.25, 0.45, 0.18], "solidity": "none", "triangles": 30, "params": {}},
    "house_low": {"primitive": "box", "size": [8, 6, 4], "taxon": "building/house",
                  "colour": [0.55, 0.42, 0.33], "solidity": "solid", "triangles": 12,
                  "params": {"collision": "box"}, "mounts": {"door": {"at": [0, 3], "face": 0}}},
    "lamp_post": {"primitive": "cylinder", "size": [0.2, 0.2, 3], "taxon": "prop/lamp",
                  "colour": [0.15, 0.15, 0.15], "solidity": "solid", "triangles": 60, "params": {}},
    "rock_a": {"primitive": "sphere", "size": [1.2, 1, 0.6], "taxon": "rock",
               "colour": [0.45, 0.43, 0.4], "solidity": "solid", "triangles": 224,
               "params": {"collision": "mesh"}},
    "shed_gable": {"primitive": "prism", "size": [4, 3, 3], "taxon": "building/shed",
                   "colour": [0.5, 0.36, 0.25], "solidity": "solid", "triangles": 8, "params": {}},
    "tree_oak": {"primitive": "sphere", "size": [5, 5, 7], "taxon": "plant/tree",
                 "colour": [0.18, 0.35, 0.12], "solidity": "solid", "triangles": 224,
                 "params": {"collision": "hull"}},
}


def ring16(size):
    """The 16-sided footprint ring of assets.md 6, in metres."""
    rx, ry = size[0] * 500, size[1] * 500
    pts = []
    for k in range(16):
        a = math.radians(k * 22.5)
        pts.append([round(rx * math.cos(a)) / 1000, round(ry * math.sin(a)) / 1000])
    return pts


def asset_entry(a):
    sx, sy, sz = a["size"]
    bounds = [-sx / 2, -sy / 2, 0, sx / 2, sy / 2, sz]
    if a["primitive"] in ("box", "prism"):
        fp = [[bounds[0], bounds[1]], [bounds[3], bounds[1]], [bounds[3], bounds[4]], [bounds[0], bounds[4]]]
    else:
        fp = ring16(a["size"])
    return {
        "bounds": bounds, "colour": a["colour"], "files": [], "footprint": fp, "mounts": a.get("mounts", {}),
        "origin": [0, 0, 0], "params": a["params"], "primitive": a["primitive"], "size": a["size"],
        "solidity": a["solidity"], "source": "primitive", "tags": [], "taxon": a["taxon"],
        "to_asset": [1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0], "triangles": [a["triangles"]],
    }


def make_world(downstream):
    """Every table of one build, as plain data."""
    ford_y = 24.0 if downstream else 40.0
    ford = (beck_x(ford_y), ford_y)
    ford_mm = (mm(ford[0]), mm(ford[1]))
    ford_pt = [metres(ford_mm[0]), metres(ford_mm[1]), zat(*ford)]

    def path(pts):
        return [[metres(mm(x)), metres(mm(y)), zat(metres(mm(x)), metres(mm(y)))] for x, y in pts]

    beck = path([(beck_x(y), y) for y in range(80, -1, -10)])
    lane_west = path([(0, 36), (24, 38), ford])
    lane_east = path([ford, (80, ford_y + 4), (96, 44)])

    def along(pts, t):
        """A point a fraction t along a polyline, on the lattice."""
        lens = [math.dist(a[:2], b[:2]) for a, b in zip(pts, pts[1:])]
        goal = t * sum(lens)
        for (a, b), l in zip(zip(pts, pts[1:]), lens):
            if goal <= l:
                f = goal / l
                x, y = metres(mm(a[0] + f * (b[0] - a[0]))), metres(mm(a[1] + f * (b[1] - a[1])))
                return [x, y, zat(x, y)]
            goal -= l
        return list(pts[-1])

    # instances: (name, asset, x, y, face, scale, tint)
    inst = [
        ("cottage", "house_low", 30, 52, 90000, 1.0, 1.0),
        ("crate", "box_small", 44, 30, 15000, 1.0, 0.9),
        ("lamp", "lamp_post", 50, 42, 0, 1.0, 1.0),
        ("shed", "shed_gable", 16, 64, 180000, 1.0, 1.1),
        ("fence.p1", "fence_post", 24, 46, 0, 1.0, 1.0),
        ("fence.p2", "fence_post", 26, 46, 0, 1.0, 1.0),
        ("fence.p3", "fence_post", 28, 46, 0, 1.0, 1.0),
    ]
    tree_spots = [(8, 10), (14, 22), (22, 8), (36, 14), (72, 60), (80, 70), (88, 52), (76, 12), (86, 30),
                  (90, 8), (70, 74), (40, 72)]
    if downstream:
        tree_spots = tree_spots[:-1] + [(46, 76)]
    for n, (x, y) in enumerate(tree_spots):
        inst.append((f"trees#t{n}", "tree_oak", x, y, (n * 47000) % 360000, 0.8 + 0.05 * (n % 5),
                     0.85 + 0.05 * (n % 4)))
    for n, (x, y) in enumerate([(64, 50), (57, 20), (66, 70)]):
        inst.append((f"rocks#r{n}", "rock_a", x, y, n * 120000, 1.0 + 0.3 * n, 1.0))
    for n, (x, y) in enumerate([(54, 56), (55, 60), (67, 34), (68, 38), (52, 10)]):
        inst.append((f"ferns#f{n}", "fern_low", x, y, n * 30000, 1.0, 1.0))
    for n, (x, y) in enumerate([(34, 44), (20, 56)]):
        inst.append((f"bushes#b{n}", "bush_round", x, y, 0, 1.0, 1.0))

    rows = []
    for name, asset, x, y, face, scale, tint in inst:
        xm, ym = mm(x), mm(y)
        cx, cy = chunk_of(xm, ym)
        rows.append({"chunk": (cy, cx), "asset": asset, "name": name, "x": xm, "y": ym,
                     "z": zat(metres(xm), metres(ym)), "face": face, "scale": f32(scale), "tint": f32(tint)})
    rows.sort(key=lambda r: (r["chunk"], r["asset"].encode(), r["name"].encode()))

    cottage = next(r for r in rows if r["name"] == "cottage")
    cottage_at = [metres(cottage["x"]), metres(cottage["y"]), cottage["z"]]

    def bookmark(name, at, toward, eye=1.7):
        return {"attrs": {"eye_height": eye, "toward": toward}, "at": at, "authority": "derived",
                "kind": "bookmark", "layer": "world", "name": name, "shape": "point"}

    feats = [
        {"attrs": {"width": 3}, "authority": "anchored", "kind": "beck", "layer": "world", "name": "beck",
         "points": beck, "shape": "path"},
        {"attrs": {}, "authority": "derived", "kind": "ford", "layer": "world", "name": "ford",
         "at": ford_pt, "shape": "point"},
        {"attrs": {"width": 3}, "authority": "derived", "kind": "lane", "layer": "world", "name": "lane_west",
         "points": lane_west, "shape": "path"},
        {"attrs": {"width": 3}, "authority": "derived", "kind": "lane", "layer": "world", "name": "lane_east",
         "points": lane_east, "shape": "path"},
        {"attrs": {"use": "grazing"}, "authority": "shaped", "kind": "meadow", "layer": "world",
         "name": "meadow", "shape": "region",
         "pieces": [{"holes": [[[10, 10], [10, 16], [16, 16], [16, 10]]],
                     "outer": [[4, 4], [30, 4], [30, 30], [4, 30]]}]},
        bookmark("at_ford", along(lane_west, 0.93), {"at": ford_pt, "name": "ford"}),
        bookmark("on_bank", along(beck, 0.45 if not downstream else 0.6), {"at": ford_pt, "name": "ford"}),
        bookmark("to_door", along(lane_west, 0.6), {"at": cottage_at, "name": "cottage"}),
        bookmark("in_trees", along(lane_east, 0.4), {"at": lane_east[-1], "name": "lane_east.end"}),
    ]
    if downstream:
        feats.append(bookmark("downstream_view", path([(70, 6)])[0], {"at": ford_pt, "name": "ford"}, 3.0))
        feats.append(bookmark("broken", path([(40, 40)])[0], "ford"))  # not resolved: load.bad_target
    feats.sort(key=lambda f: f["name"].encode())
    # the cottage's footprint: its rectangle turned by face 90
    hx, hy = 3.0, 4.0
    cxm, cym = metres(cottage["x"]), metres(cottage["y"])
    footprints = [{"attrs": {}, "authority": "derived", "instance": "cottage", "kind": "pad", "layer": "world",
                   "pieces": [{"holes": [], "outer": [[cxm - hx, cym - hy], [cxm + hx, cym - hy],
                                                      [cxm + hx, cym + hy], [cxm - hx, cym + hy]]}]}]
    meets = []
    for end in ("lane_east.start", "lane_west.end"):
        meets.append({"a": "ford", "b": end, "connector_a": None, "connector_b": None, "face_a": None,
                      "face_b": None, "gap": 0, "turn": None, "x": ford_mm[0], "y": ford_mm[1],
                      "z_a": ford_pt[2], "z_b": ford_pt[2]})
    lanes = [lane_west, lane_east]
    return rows, feats, footprints, meets, lanes


def write_build(out, downstream, pack_dir):
    rows, feats, footprints, meets, lanes = make_world(downstream)
    files = {}

    def put(path, data):
        files[path] = data

    def material_entry(m):
        e = {"colour": MATERIALS[m]["colour"], "params": {}}
        if "file" in MATERIALS[m]:
            e["file"] = MATERIALS[m]["file"]
            e["digest"] = blake3((pack_dir / MATERIALS[m]["file"]).read_bytes())
            e["repeat"] = MATERIALS[m]["repeat"]
        return e

    height = grid(ground)
    put("fields/height.npy", npy("<f4", (NY, NX), height))
    wat = grid(water)
    put("fields/water.npy", npy("<f4", (NY, NX), wat))
    clearing = [f32(1.0 if abs(x - 30) < 6 and abs(y - 52) < 6 else 0.0)
                for j in range(NY) for i in range(NX) for x, y in [post_xy(i, j)]]
    put("fields/clearing.npy", npy("<f4", (NY, NX), clearing))
    mat_ids = sorted(MATERIALS)
    post_bytes = {}
    for j in range(NY):
        for i in range(NX):
            post_bytes[(i, j)] = hamilton(weights_at(*post_xy(i, j), lanes))

    def bits(v):  # a single-precision value's bits, as an unsigned integer
        return struct.unpack("<I", struct.pack("<f", v))[0]

    def chunk_digest(i0, j0, i1, j1, mine, wentry):
        # build.md 5.2: the BLAKE3 of one object's canonical JSON (no closing line feed). The
        # fixture has no cut, meshes, features or footprints listed per chunk, so those keys are
        # empty; what the ford's move changes (the beck's ground and water) changes a chunk's.
        def window(values):
            return blake3(b"".join(struct.pack("<f", values[j * NX + i])
                                   for j in range(j0, j1 + 1) for i in range(i0, i1 + 1)))
        rows_ = [[rows[n]["name"], rows[n]["asset"], rows[n]["x"], rows[n]["y"], rows[n]["face"] % 360000,
                  bits(rows[n]["z"]), bits(rows[n]["scale"]), bits(rows[n]["tint"])] for n in mine]
        obj = {
            "assets": {a: blake3(canonical(asset_entry(ASSETS[a]))[:-1])
                       for a in sorted({rows[n]["asset"] for n in mine})},
            "features": {}, "footprints": {},
            "fields": {"clearing": window(clearing), "height": window(height), "water": window(wat)},
            "instances": blake3(canonical(rows_)[:-1]),
            "materials": {m: blake3(canonical(material_entry(m))[:-1]) for m in wentry["materials"]},
            "meshes": {},
            "weights": {"material": blake3(files[wentry["file"]]) if wentry["file"] else None},
        }
        return blake3(canonical(obj)[:-1])

    chunk_entries = []
    for cx, cy, i0, j0, i1, j1 in chunks():
        used = [m for m in range(len(mat_ids))
                if any(post_bytes[(i, j)][m] > 0 for j in range(j0, j1 + 1) for i in range(i0, i1 + 1))]
        wentry = {"file": None, "materials": []}
        if used:
            data = [post_bytes[(i, j)][m] for m in used for j in range(j0, j1 + 1) for i in range(i0, i1 + 1)]
            path = f"weights/material/{cx}_{cy}.npy"
            put(path, npy("|u1", (len(used), j1 - j0 + 1, i1 - i0 + 1), data))
            wentry = {"file": path, "materials": [mat_ids[m] for m in used]}
        mine = [n for n, r in enumerate(rows) if r["chunk"] == (cy, cx)]
        by_asset = {}
        for n in mine:
            a = rows[n]["asset"]
            first, count = by_asset.get(a, (n, 0))
            by_asset[a] = (first, count + 1)
        chunk_entries.append({
            "at": [cx, cy], "count": len(mine),
            "digest": chunk_digest(i0, j0, i1, j1, mine, wentry),
            "instances": {a: [f, c] for a, (f, c) in sorted(by_asset.items())},
            "posts": [i0, j0, i1, j1],
            "triangles": sum(ASSETS[rows[n]["asset"]]["triangles"] for n in mine),
            "weights": {"material": wentry},
        })

    count = len(rows)
    put("instances/x.npy", npy("<i4", (count,), [r["x"] for r in rows]))
    put("instances/y.npy", npy("<i4", (count,), [r["y"] for r in rows]))
    put("instances/z.npy", npy("<f4", (count,), [r["z"] for r in rows]))
    put("instances/face.npy", npy("<i4", (count,), [r["face"] for r in rows]))
    put("instances/scale.npy", npy("<f4", (count,), [r["scale"] for r in rows]))
    put("instances/tint.npy", npy("<f4", (count,), [r["tint"] for r in rows]))
    put("instances/names.txt", "".join(r["name"] + "\n" for r in rows).encode())
    put("features.json", canonical({"features": feats, "footprints": footprints}))
    put("meets.json", canonical({"meets": meets}))

    def field(file, values, role, tags, unit):
        return {"file": file, "max": max(values), "min": min(values), "role": role, "tags": tags,
                "taxon": None, "type": "scalar", "unit": unit}

    used_assets = sorted({r["asset"] for r in rows})
    materials = {m: material_entry(m) for m in mat_ids}
    diagnostics = canonical({"counts": {"error": 0, "failed_on": 0, "info": 0, "warning": 0},
                             "diagnostics": [], "format": "via-diagnostics-1", "lints": [], "status": 0})
    manifest = {
        "assets": {a: asset_entry(ASSETS[a]) for a in used_assets},
        "chunks": chunk_entries,
        "engine": {"epoch": 1, "name": "via"},
        "features": "features.json",
        "fields": {
            "clearing": field("fields/clearing.npy", clearing, "keep_out", [], None),
            "height": field("fields/height.npy", height, "surface", [], "m"),
            "material": {"role": "material", "tags": [], "taxon": None, "type": "weights", "unit": None},
            "water": field("fields/water.npy", wat, None, ["water_level"], "m"),
        },
        "format": "via-build-1",
        "frame": {"axes": "x east, y north, z up", "bearing": "degrees clockwise from north",
                  "extent": list(EXTENT), "posts": [NX, NY], "resolution": RES, "unit": "m"},
        "instances": {"count": count, "face": "instances/face.npy", "names": "instances/names.txt",
                      "scale": "instances/scale.npy", "tint": "instances/tint.npy", "x": "instances/x.npy",
                      "y": "instances/y.npy", "z": "instances/z.npy"},
        "materials": materials,
        "meets": "meets.json",
        "outputs": {"bookmark": sorted(f["name"] for f in feats if f["kind"] == "bookmark")},
        "seed": "00000000000000a5",
        "world": "fixture",
    }
    manifest["files"] = {p: {"bytes": len(b), "digest": blake3(b)} for p, b in sorted(files.items())}
    manifest["id"] = blake3(canonical(manifest))
    manifest["locations"] = {"pack_root": "../fixture_pack"}
    manifest["run"] = {
        "changes": ["downstream"] if downstream else [],
        "flags": {"fail_on": [], "stand_ins": False},
        "inputs": [], "key": blake3(b"fixture" + (b"+downstream" if downstream else b"")),
        "reports": {"diagnostics.json": {"bytes": len(diagnostics), "digest": blake3(diagnostics)}},
        "version": "0.0.0",
    }
    put("diagnostics.json", diagnostics)
    put("manifest.json", canonical(manifest))
    put(".previous", canonical({"format": "via-previous-0", "id": None, "manifest": None}))
    return {out / p: b for p, b in files.items()}


def main():
    check = "--check" in sys.argv[1:]
    pack = HERE / "fixture_pack"
    want = {pack / "textures/gravel.png": png_rgb(
        32, 32, lambda x, y: (v := 120 + ((x * 7 + y * 13) % 5) * 8 + ((x // 4 + y // 4) % 2) * 12,
                              (v, v - 6, v - 16))[1])}
    if not check:
        for p, b in want.items():
            p.parent.mkdir(parents=True, exist_ok=True)
            p.write_bytes(b)
    want.update(write_build(HERE / "fixture_build", False, pack))
    want.update(write_build(HERE / "fixture_build+downstream", True, pack))
    differ = [p for p, b in sorted(want.items()) if not p.exists() or p.read_bytes() != b]
    if check:
        for p in differ:
            print("differs:", p.relative_to(HERE))
        sys.exit(1 if differ else 0)
    for p, b in want.items():
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_bytes(b)
    print(f"wrote {len(want)} files")


if __name__ == "__main__":
    main()
