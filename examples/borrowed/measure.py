"""Measures of a borrowed-kit build, for expect.toml: how someone else's kit (Kenney's Modular
Buildings 2.1) is laid by lib/building, and what the build said (docs/plan-kits.md, K10).

    python3 examples/borrowed/measure.py BUILD [--world WORLD] [--change CHANGE] [--base BASE] [--check SECTION]
    python3 examples/borrowed/measure.py --packs PACK.toml ...

Prints the measures as JSON. With --check, compares them against expect.toml's rows under SECTION
(Python 3.11+, for tomllib). With --base, also what changed against the base build, by name. With
--world (and --change), what the author wrote. With --packs, what a kit's author wrote in each
pack (the figures of README.md, "What each author wrote"). Reads only the build directory
(spec/build.md): manifest.json, features.json, diagnostics.json and instances/. Standard library
only.

How each is taken (ours).

- `instances`; `blocks` (instances of an asset tagged `role:wall`, `role:corner` or `role:door`
  that is no `storey:parapet` piece: the building's blocks), `doors` (`role:door`), `corners`
  (`role:corner`, not the parapet's), `roof_pieces` (`role:roof_slope`, `role:roof_hip`),
  `roof_tiles` (`role:roof_flat`), `parapet` (`storey:parapet`).
- **Walls.** For each building and each storey, the pieces `lay` named for it
  (`lay#<building>_s<k>_...`, the building's name with `#` as `_`): a piece covers a side of the
  building's outline between the least and the greatest of its plan's corners that lie within
  0.3 m of the side's line. `walls_open_m` is the length of outline, summed over every storey of
  every building, that no piece covers; `walls_open` the number of such stretches. A doorway's
  door block covers its stretch: a door that stands where a bay would leaves nothing open.
- **Roofs, from the models' triangles.** For each `roof` feature, points every 0.1 m over its
  region (a hipped roof's region is the outline grown by the kit's eaves), the rows offset by
  0.03 m and the columns by 0.05 m so no point lies on a hip's diagonal: a point is held by a
  piece named for that roof (`lay#<building>_roof...`: tiles, parapet, hips, slopes) where it lies
  in the plan of one of the piece's triangles that face up, each model's triangles read from its
  GLB (`packs/kenney/`, or `--models DIR`) through the manifest's `to_asset` and turned by the
  instance's `face`. `roof_open_dm2` counts the points no piece holds inside the building's
  outline, `roof_eaves_open_dm2` those in the eaves outside it, `roof_double_dm2` the points two
  or more pieces hold. One point is a square decimetre. (Until K10's review these were read from
  the boxes of the measured bounds, which are the same however a piece is turned: a hip turned
  the wrong way passed.)
- **Corners, from the models' triangles.** For each instance of a `role:corner` piece, the
  square its box covers at the nearest outline corner (within 1.5 m), sampled every 0.02 m: the
  points of that square no up-facing triangle of the piece holds are where its shape is cut. A
  piece is `corners_cut_out` where a cut point lies within 0.05 m of the outline's corner (Kenney's
  building corner is bevelled there when it faces out), `corners_cut_in` where its cut lies
  elsewhere in the square (the same piece turned the wrong way, or a parapet's corner, whose
  border stands on the outside), and `corners_whole` where nothing is cut.
- `errors`, `warnings`; `wall_open`, `door_narrow`, `variant_none`, `roof_none`, `crowded`
  (`place.crowded`), `unreached`: each code's records, a folded record by its `count`. `_codes`:
  every code's count.
- `gap_left_mm`, `gap_need_mm`: of the `gap_wide` records, the most the wall leaves at a doorway
  and the most a doorway needs (its width and the margin, or the door piece where that is wider).
- **Written and laid.** `written_tables` and `written_numbers` (`--world`): the world's `[...]`
  tables and the numbers in it outside comments, every element of an array counted; with
  `--change`, the change's own (`change_numbers`). `laid_features` (features no author wrote:
  layer `walls` or a step's) and `laid` (those and the instances).
- **Against the base** (`--base`): `added`, `removed`, `moved` instances by name.
- **A pack** (`--packs`): `assets`, `lines` (not blank, not a comment), `numbers` (outside
  comments, every element counted), `tags`, `facts` (the keys of every `defaults.shared` table,
  a stair's keys each), and `turned` (assets that write `front`).
"""

import json
import math
import re
import struct
import sys
from collections import Counter
from pathlib import Path

TOL = 0.005
NEAR = 0.3
COVER = 0.1
ROOF = ("role:roof_slope", "role:roof_hip", "role:roof_gable", "role:roof_valley", "role:roof_flat")


def _npy(path):
    raw = Path(path).read_bytes()
    hlen = struct.unpack("<H", raw[8:10])[0]
    head = raw[10:10 + hlen].decode()
    body = raw[10 + hlen:]
    return head, struct.unpack(f"<{len(body) // 4}{'f' if '<f4' in head else 'i'}", body)


def load(d):
    """A stopped build (exit 1) writes its diagnostics and nothing else: its measures are of
    what it said, and none of what it laid."""
    d = Path(d)
    diags = json.loads((d / "diagnostics.json").read_text())["diagnostics"]
    if not (d / "manifest.json").exists():
        return {"assets": {}, "chunks": []}, diags, [], {}
    man = json.loads((d / "manifest.json").read_text())
    feats = json.loads((d / "features.json").read_text())["features"]
    rows = {}
    names_p = d / "instances/names.txt"
    if names_p.exists():
        names = [n for n in names_p.read_text().splitlines() if n]
        x, y, face = ([v / 1000.0 for v in _npy(d / f"instances/{k}.npy")[1]] for k in ("x", "y", "face"))
        z = list(_npy(d / "instances/z.npy")[1])
        asset = [None] * len(names)
        for ch in man["chunks"]:
            for a, (s, n) in ch.get("instances", {}).items():
                for k in range(s, s + n):
                    asset[k] = a
        rows = {n: {"asset": asset[k], "x": x[k], "y": y[k], "z": z[k], "face": face[k]}
                for k, n in enumerate(names)}
    return man, diags, feats, rows


def sides(ring):
    out = []
    for a, b in zip(ring, ring[1:] + ring[:1]):
        L = math.hypot(b[0] - a[0], b[1] - a[1])
        out.append((a, ((b[0] - a[0]) / L, (b[1] - a[1]) / L), L))
    return out


def open_of(ring, laid):
    """Open stretches (metres) of each side of `ring`, given `laid`, each piece's plan."""
    out = []
    for a, e, L in sides(ring):
        spans = []
        for plan in laid:
            us = [(x - a[0]) * e[0] + (y - a[1]) * e[1] for x, y in plan
                  if abs((x - a[0]) * e[1] - (y - a[1]) * e[0]) <= NEAR]
            if len(us) >= 2 and max(us) > -TOL and min(us) < L + TOL:
                spans.append((max(0.0, min(us)), min(L, max(us))))
        at = 0.0
        for lo, hi in sorted(spans):
            if lo - at > TOL:
                out.append(lo - at)
            at = max(at, hi)
        if L - at > TOL:
            out.append(L - at)
    return out


def inside(ring, p):
    x, y = p
    hit = False
    for (x0, y0), (x1, y1) in zip(ring, ring[1:] + ring[:1]):
        if (y0 > y) != (y1 > y) and x < x0 + (y - y0) * (x1 - x0) / (y1 - y0):
            hit = not hit
    return hit


def plan_of(r, asset):
    """An instance's plan in world metres: the box of its asset's measured bounds, turned by its
    `face` (degrees clockwise from north, the way the asset frame's +y points)."""
    f = math.radians(r["face"])
    fx, fy = math.sin(f), math.cos(f)
    rx, ry = math.cos(f), -math.sin(f)
    x0, y0, _, x1, y1, _ = asset["bounds"]
    return [(r["x"] + ax * rx + ay * fx, r["y"] + ax * ry + ay * fy)
            for ax, ay in ((x0, y0), (x1, y0), (x1, y1), (x0, y1))]


MODELS = Path(__file__).resolve().parent / "packs" / "kenney"


def glb_triangles(path):
    """Every triangle of a GLB file's default scene, in the model's frame, through its nodes'
    transforms (glTF 2.0: little-endian, a JSON chunk then a BIN chunk). Standard library only;
    the file is read as data."""
    raw = Path(path).read_bytes()
    jl = struct.unpack("<I", raw[12:16])[0]
    js = json.loads(raw[20:20 + jl])
    body = raw[20 + jl + 8:]

    def acc(i):
        a = js["accessors"][i]
        bv = js["bufferViews"][a["bufferView"]]
        off = bv.get("byteOffset", 0) + a.get("byteOffset", 0)
        n = {"SCALAR": 1, "VEC2": 2, "VEC3": 3, "VEC4": 4}[a["type"]]
        fmt = {5126: "f", 5123: "H", 5125: "I", 5121: "B"}[a["componentType"]]
        stride = bv.get("byteStride", struct.calcsize(fmt) * n)
        return [struct.unpack_from("<" + fmt * n, body, off + k * stride) for k in range(a["count"])]

    def node_m(n):
        if "matrix" in n:
            m = n["matrix"]
            return [[m[c * 4 + r] for c in range(4)] for r in range(4)]
        t, q, sc = n.get("translation", [0, 0, 0]), n.get("rotation", [0, 0, 0, 1]), n.get("scale", [1, 1, 1])
        x, y, z, w = q
        R = [[1 - 2 * (y * y + z * z), 2 * (x * y - z * w), 2 * (x * z + y * w)],
             [2 * (x * y + z * w), 1 - 2 * (x * x + z * z), 2 * (y * z - x * w)],
             [2 * (x * z - y * w), 2 * (y * z + x * w), 1 - 2 * (x * x + y * y)]]
        return [[R[i][0] * sc[0], R[i][1] * sc[1], R[i][2] * sc[2], t[i]] for i in range(3)] + [[0, 0, 0, 1]]

    def mul(a, b):
        return [[sum(a[i][k] * b[k][j] for k in range(4)) for j in range(4)] for i in range(4)]
    out = []

    def walk(ni, M):
        n = js["nodes"][ni]
        M = mul(M, node_m(n))
        for prim in (js["meshes"][n["mesh"]]["primitives"] if "mesh" in n else []):
            P = acc(prim["attributes"]["POSITION"])
            ix = [i[0] for i in acc(prim["indices"])] if "indices" in prim else list(range(len(P)))
            for k in range(0, len(ix) - 2, 3):
                out.append([tuple(sum(M[r][c] * (P[i][c] if c < 3 else 1) for c in range(4)) for r in range(3))
                            for i in ix[k:k + 3]])
        for c in n.get("children", []):
            walk(c, M)
    eye = [[float(i == j) for j in range(4)] for i in range(4)]
    for r in js["scenes"][js.get("scene", 0)]["nodes"]:
        walk(r, eye)
    return out


_UP = {}


def up_plan(asset_id, asset, models):
    """The plan triangles (asset frame, metres) of a model's triangles that face up."""
    if asset_id in _UP:
        return _UP[asset_id]
    M = asset.get("to_asset") or [1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0]
    out = []
    files = asset.get("files") or []
    if files and (models / files[0]["path"]).exists():
        for t in glb_triangles(models / files[0]["path"]):
            q = [tuple(M[r * 4] * v[0] + M[r * 4 + 1] * v[1] + M[r * 4 + 2] * v[2] + M[r * 4 + 3] for r in range(3))
                 for v in t]
            nz = (q[1][0] - q[0][0]) * (q[2][1] - q[0][1]) - (q[1][1] - q[0][1]) * (q[2][0] - q[0][0])
            if nz > 1e-9:
                out.append(tuple((v[0], v[1]) for v in q))
    _UP[asset_id] = out
    return out


def turn(r, p):
    f = math.radians(r["face"])
    fx, fy = math.sin(f), math.cos(f)
    rx, ry = math.cos(f), -math.sin(f)
    return (r["x"] + p[0] * rx + p[1] * fx, r["y"] + p[0] * ry + p[1] * fy)


def in_tri(t, p):
    (x0, y0), (x1, y1), (x2, y2) = t
    d = (y1 - y2) * (x0 - x2) + (x2 - x1) * (y0 - y2)
    if abs(d) < 1e-12:
        return False
    a = ((y1 - y2) * (p[0] - x2) + (x2 - x1) * (p[1] - y2)) / d
    b = ((y2 - y0) * (p[0] - x2) + (x0 - x2) * (p[1] - y2)) / d
    return a >= -1e-9 and b >= -1e-9 and 1 - a - b >= -1e-9


def key_of(name):
    return name.replace("#", "_")


def count(diags, code):
    return sum(int((r.get("data") or {}).get("count", 1)) for r in diags if r["code"] == code)


def written(text):
    """(tables, numbers) in TOML text outside comments: `[...]` table headers, and every number
    (an array's elements each), not counting numbers inside strings."""
    tables = numbers = 0
    for line in text.splitlines():
        line = re.sub(r'"[^"]*"', '""', line).split("#")[0]
        if re.match(r"\s*\[[^\]]", line) and "=" not in line:
            tables += 1
            continue
        if "=" in line:
            numbers += len(re.findall(r"(?<![\w.])-?\d+(?:\.\d+)?(?![\w.])", line.split("=", 1)[1]))
    return tables, numbers


def measures(d, models=MODELS):
    man, diags, feats, rows = load(d)
    assets = man["assets"]

    def tags(r):
        return (assets.get(r["asset"]) or {}).get("tags") or []
    m = {"errors": sum(1 for r in diags if r["severity"] == "error"),
         "warnings": sum(1 for r in diags if r["severity"] == "warning"),
         "instances": len(rows)}
    m["blocks"] = sum(1 for r in rows.values() if any(t in tags(r) for t in ("role:wall", "role:corner", "role:door"))
                      and "storey:parapet" not in tags(r))
    m["doors"] = sum(1 for r in rows.values() if "role:door" in tags(r))
    m["corners"] = sum(1 for r in rows.values() if "role:corner" in tags(r) and "storey:parapet" not in tags(r))
    m["roof_pieces"] = sum(1 for r in rows.values() if any(t in tags(r) for t in ROOF[:4]))
    m["roof_tiles"] = sum(1 for r in rows.values() if "role:roof_flat" in tags(r))
    m["parapet"] = sum(1 for r in rows.values() if "storey:parapet" in tags(r))
    # Walls of every storey of every building.
    opens = []
    for f in feats:
        if f.get("kind") != "building":
            continue
        ring = [tuple(p[:2]) for p in f["pieces"][0]["outer"]]
        for k in range(int((f.get("attrs") or {}).get("storeys") or 1)):
            pre = f"lay#{key_of(f['name'])}_s{k}_"
            laid = [plan_of(r, assets[r["asset"]]) for n, r in rows.items() if n.startswith(pre)]
            opens += open_of(ring, laid)
    m["walls_open"] = len(opens)
    m["walls_open_m"] = round(sum(opens), 3)
    # Roofs, from the models' triangles that face up.
    open_in = open_eaves = double = 0
    outlines = {f"lay#{key_of(f['name'])}_roof": [tuple(p[:2]) for p in f["pieces"][0]["outer"]]
                for f in feats if f.get("kind") == "building"}
    for f in feats:
        if f.get("kind") != "roof":
            continue
        ring = [tuple(p[:2]) for p in f["pieces"][0]["outer"]]
        own = outlines.get(f["name"], ring)
        pre = f["name"]
        tris = [[tuple(turn(r, v) for v in t) for t in up_plan(r["asset"], assets[r["asset"]], models)]
                for n, r in rows.items() if n.startswith(pre)]
        if not tris:
            continue
        xs, ys = [p[0] for p in ring], [p[1] for p in ring]
        x = min(xs) + COVER / 2
        while x < max(xs):
            y = min(ys) + 0.03
            while y < max(ys):
                if inside(ring, (x, y)):
                    held = sum(1 for ts in tris if any(in_tri(t, (x, y)) for t in ts))
                    if held == 0:
                        if inside(own, (x, y)):
                            open_in += 1
                        else:
                            open_eaves += 1
                    double += held > 1
                y += COVER
            x += COVER
    m["roof_open_dm2"], m["roof_eaves_open_dm2"], m["roof_double_dm2"] = open_in, open_eaves, double
    # Corners, from the models' triangles: where each corner piece's shape is cut.
    corners_ = [p for f in feats if f.get("kind") == "building" for p in (tuple(q[:2]) for q in f["pieces"][0]["outer"])]
    out_ = in_ = whole_ = 0
    for n, r in rows.items():
        a = assets.get(r["asset"]) or {}
        if "role:corner" not in (a.get("tags") or []) or not corners_:
            continue
        v = min(corners_, key=lambda c: math.hypot(c[0] - r["x"], c[1] - r["y"]))
        if math.hypot(v[0] - r["x"], v[1] - r["y"]) > 1.5:
            continue
        b = a["bounds"]
        side = max(b[3] - b[0], b[4] - b[1])
        ux = 1.0 if r["x"] >= v[0] else -1.0
        uy = 1.0 if r["y"] >= v[1] else -1.0
        ts = [tuple(turn(r, q) for q in t) for t in up_plan(r["asset"], a, models)]
        cut, near_ = 0, False
        k = int(round(side / 0.02))
        for i in range(k):
            for j in range(k):
                p = (v[0] + ux * (i + 0.5) * 0.02, v[1] + uy * (j + 0.5) * 0.02)
                if not any(in_tri(t, p) for t in ts):
                    cut += 1
                    near_ = near_ or math.hypot(p[0] - v[0], p[1] - v[1]) <= 0.05
        if cut == 0:
            whole_ += 1
        elif near_:
            out_ += 1
        else:
            in_ += 1
    m["corners_cut_out"], m["corners_cut_in"], m["corners_whole"] = out_, in_, whole_
    codes = Counter()
    for r in diags:
        codes[r["code"]] += int((r.get("data") or {}).get("count", 1))
    m["_codes"] = dict(sorted(codes.items()))
    for word, code in (("wall_open", "step.lay.wall_open"), ("door_narrow", "step.check.door_narrow"),
                       ("variant_none", "step.lay.variant_none"), ("roof_none", "step.lay.roof_none"),
                       ("crowded", "place.crowded"), ("unreached", "step.check.unreached"),
                       ("kit_variant", "step.check.kit_variant"), ("kit_pitch", "step.check.kit_pitch"),
                       ("kit_stated", "step.check.kit_stated"), ("unknown_material", "pack.unknown_material"),
                       ("gap_wide", "step.lay.gap_wide"), ("slab_open", "step.lay.slab_open"),
                       ("kit_deck", "step.check.kit_deck")):
        m[word] = count(diags, code)
    gw = [r for r in diags if r["code"] == "step.lay.gap_wide"]
    m["gap_left_mm"] = max((int(round(float((r.get("data") or {}).get("left", 0)) * 1000)) for r in gw), default=0)
    m["gap_need_mm"] = max((int(round(float((r.get("data") or {}).get("need", 0)) * 1000)) for r in gw), default=0)
    # The hipped roof's depth that `roof_none` names, and the span the slope is made for.
    rn = [r for r in diags if r["code"] == "step.lay.roof_none"]
    m["roof_none_deep_mm"] = int(round(float(re.search(r"roof is ([\d.]+) m deep", rn[0]["message"]).group(1)) * 1000)) \
        if rn and re.search(r"roof is ([\d.]+) m deep", rn[0]["message"]) else 0
    m["laid_features"] = sum(1 for f in feats if f.get("layer") != "world")
    m["laid"] = m["laid_features"] + m["instances"]
    return m, rows


def against(rows, base):
    both = set(rows) & set(base)
    return {"added": len(set(rows) - set(base)), "removed": len(set(base) - set(rows)),
            "moved": sum(1 for n in both if any(rows[n][k] != base[n][k] for k in ("x", "y", "z", "face", "asset")))}


def pack_figures(path):
    text = Path(path).read_text()
    lines = [ln for ln in text.splitlines() if ln.strip() and not ln.strip().startswith("#")]
    _, numbers = written(text)
    tags_ = sum(len(re.findall(r'"[^"]+"', ln.split("=", 1)[1])) for ln in lines if re.match(r"\s*tags\s*=", ln))
    facts, in_shared = 0, False
    for ln in lines:
        s = ln.split("#")[0].strip()
        if s.startswith("["):
            in_shared = ".defaults.shared" in s
            if in_shared and s.endswith(".stair]"):
                continue
            continue
        if in_shared and "=" in s:
            facts += 1
    # An asset's own table, not its mounts' (`[asset.<id>.mount.<name>]`).
    return {"assets": sum(1 for ln in lines if re.match(r"\s*\[asset\.[^.\]]+\]", ln)), "lines": len(lines),
            "numbers": numbers, "tags": tags_, "facts": facts,
            "turned": sum(1 for ln in lines if re.match(r"\s*front\s*=", ln))}


def main(argv):
    args = argv[1:]
    if args[:1] == ["--packs"]:
        print(json.dumps({p: pack_figures(p) for p in args[1:]}, indent=1))
        return 0
    opt = {}
    for flag in ("--base", "--check", "--world", "--change", "--models"):
        if flag in args:
            i = args.index(flag)
            opt[flag] = args[i + 1]
            del args[i:i + 2]
    if len(args) != 1:
        print(__doc__.split("\n\n")[1], file=sys.stderr)
        return 2
    models = Path(opt["--models"]) if "--models" in opt else MODELS
    m, rows = measures(args[0], models)
    if "--base" in opt:
        m.update(against(rows, measures(opt["--base"], models)[1]))
    if "--world" in opt:
        w = Path(opt["--world"])
        t, n = written(w.read_text())
        for layer in re.findall(r'^file\s*=\s*"([^"]+)"', w.read_text(), re.M):
            lt, ln = written((w.parent / layer).read_text())
            t, n = t + lt, n + ln
        m["written_tables"], m["written_numbers"] = t, n
    if "--change" in opt:
        m["change_numbers"] = written(Path(opt["--change"]).read_text())[1]
    check = opt.get("--check")
    if not check:
        print(json.dumps(m, indent=1, sort_keys=True))
        return 0
    import tomllib
    table = tomllib.loads((Path(__file__).resolve().parent / "expect.toml").read_text()).get(check)
    if not table:
        print(f"no section `{check}` in expect.toml", file=sys.stderr)
        return 2
    bad = 0
    for key, row in table.items():
        v = m.get(row["measure"])
        if "equals" in row:
            ok = v == row["equals"]
            want = repr(row["equals"])
        else:
            lo, hi = row["band"]
            ok = v is not None and not isinstance(v, str) and lo <= v <= hi
            want = f"[{lo}, {hi}]"
        bad += not ok
        print(f"{'ok  ' if ok else 'FAIL'} {check}.{key}: {row['measure']} = {v!r} in {want}")
    print(f"{len(table) - bad}/{len(table)} rows")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
