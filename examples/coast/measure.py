"""Measures of a coast build, for expect.toml (docs/plan.md M5; challenges 7 and 8).

    python3 examples/coast/measure.py BUILD [--surfaces FILE] [--check SECTION]

Prints the measures as JSON. With --check, compares them against expect.toml's rows under SECTION
(`base`, `move_cave`, `long_deck`) and exits 1 when one falls outside its band or names a measure
not taken (Python 3.11+, for tomllib). Reads only the build directory (spec/build.md):
manifest.json, features.json, meets.json, diagnostics.json, fields/*.npy, the cut file and
meshes/*.glb; with --surfaces, the output of `via query WORLD surfaces_at --in cave_points.json
--out FILE` (surfaces.md 2.1, 7). Standard library only.

How each is taken (ours, M5 slice 2). Posts are at (x0 + i r, y0 + j r) from manifest.json's
`frame`; a field is read bilinearly between them; quad (j, i) is the square from post (i, j), cut
when the cut file holds 1 there (build.md 6.5).
- `meetings`, `gap_max_mm`: rows of meets.json and the largest |gap|.
- `errors`: diagnostics of severity error.
- The cave (`sea_cave`; its line from features.json, its half width `width / 2`):
  - `mouth_gap_mm`: |gap| of the meeting `sea_cave.mouth` / `sea_cave.mouth_ground`, -1 if none;
    `mouth_at` its plan point; `jambs_met` the meetings at the cave's two jambs with gap 0 (each
    face's end).
  - `mouth_open`: cut quads whose centre is within the floor's half width of the cave's line and
    whose centre lies within 2 m of the mouth along it (the cliff's step at the mouth).
  - `roof_cut`: cut quads over the cave farther than 2 m in from the mouth (the hill kept whole: 0).
  - `roof_min_m`: the least terrain height less the floor's height (`z0 + (z1 - z0) t`) at the
    posts within the half width and over 2 m in: the rock over the floor.
  - `beach_grade_deg`: the steepest 0.5 m step of the terrain on the straight line from the
    bookmark `at_cave_mouth` to the mouth, in degrees; `beach_cut` the cut quads under that line
    sampled every 0.1 m, `beach_wet_m` the deepest water on it.
  - `floor_grade_deg`: the floor's steepest grade along its line, from its two heights and length.
  - `cave_reached`: 1 when the mouth meeting holds (gap 0), the beach line is walkable by
    lib/review's foot profile (grade at most 34 degrees, water at most 0.7 m, no cut quad) and the
    floor's grade is at most 34 degrees; else 0. A walk by 4.4's rules, written out for one route.
- The cliff (each `sheer_cliff` mesh, meshes/<face>.face.glb, read with its node's translation;
  glTF axes are x, z, -y):
  - `face_triangles`: triangles in the faces' meshes.
  - `face_steep_share`: the share of the faces' surface area in triangles steeper than 80 degrees.
  - `brink_dev_mm`: over the face vertices at the top (z within 1 mm of `top_z`), the spread
    (largest less least) of their signed distance from the cliff's line: 0 when the brink is drawn
    along the line, `lip` from it. `brink_vertices` counts them. Only vertices within `lip` of the
    line count: past it the mesh's skirt (build.md 10, M5 slice 3 fix) lies flat at `top_z` over
    the cut quads, which is no brink.
  - `terrain_brink_dev_mm`, for contrast, what the terrain alone draws: the same spread over the
    posts at `top_z` (within 1 mm) with a 4-neighbour at `foot_z`, within the faces' span; the
    staircase the 0.5 m posts make of an oblique line (up to 0.71 of a post).
  - `face_cut`: cut quads whose centre lies within `lip` of a face's line.
  - `step_left`: quads not cut whose four posts span more than half the drop (a step the terrain
    still draws) within 3 m of a face's line, away from the cave's mouth (4 m).
  - `sliver_m2_per_m`: the area of cut quads (within 3 m of a face's line) that no kind surface's
    mesh covers (strips of the faces, the floor and the deck, sampled on a 0.1 m grid in each quad;
    a face's and the floor's strips grown by the skirt, build.md 10), per metre of face: what is
    open to the sky at the foot and the brink.
- The bridge (`harbour_bridge`, its line and `width / 2`):
  - `deck_posts`, `deck_wet`: posts within the half width between its ends, and of them those where
    the water stands over the ground (depth over 1 mm).
  - `under_depth_m`: water less ground at the deck's middle; `freeboard_m`: `deck_z` less the water
    there.
  - `deck_length_m`: the deck's length, its ends from features.json.
  - `parapet_posts`: instances whose name holds `parapet`.
- With --surfaces: `cave_surfaces`, the surfaces listed at the first point (the cave's middle), and
  `cave_surface_names`, their names in order.
"""

import json
import math
import struct
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent


def npy(path):
    raw = Path(path).read_bytes()
    hlen = struct.unpack("<H", raw[8:10])[0]
    head = raw[10:10 + hlen].decode()
    body = raw[10 + hlen:]
    shape = tuple(int(v) for v in head.split("'shape': (")[1].split(")")[0].split(",") if v.strip())
    if "u1" in head:
        return shape, body
    code = "f" if "f4" in head else ("d" if "f8" in head else "i")
    size = struct.calcsize(code)
    return shape, struct.unpack(f"<{len(body) // size}{code}", body)


class Grid:
    def __init__(self, d, frame):
        self.r = frame["resolution"]
        self.x0, self.y0 = frame["extent"][0], frame["extent"][1]
        self.nx, self.ny = frame["posts"]
        self.f = {n: npy(d / "fields" / f"{n}.npy")[1] for n in ("height", "water")}
        shape, self.cut = npy(d / "fields" / "height.cut.npy")
        assert shape == (self.ny - 1, self.nx - 1), shape

    def post(self, name, i, j):
        return self.f[name][j * self.nx + i]

    def at(self, name, x, y):
        u, v = (x - self.x0) / self.r, (y - self.y0) / self.r
        i, j = min(max(int(u), 0), self.nx - 2), min(max(int(v), 0), self.ny - 2)
        a, b = u - i, v - j
        p = self.post
        return ((1 - a) * (1 - b) * p(name, i, j) + a * (1 - b) * p(name, i + 1, j)
                + (1 - a) * b * p(name, i, j + 1) + a * b * p(name, i + 1, j + 1))

    def is_cut(self, i, j):
        return 0 <= i < self.nx - 1 and 0 <= j < self.ny - 1 and self.cut[j * (self.nx - 1) + i] == 1

    def quad_of(self, x, y):
        return int((x - self.x0) // self.r), int((y - self.y0) // self.r)

    def centre(self, i, j):
        return self.x0 + (i + 0.5) * self.r, self.y0 + (j + 0.5) * self.r


def near_line(p, pts):
    """(distance, arc length of the nearest point, total length) from p to a polyline."""
    best, s0, total = (math.inf, 0.0), 0.0, 0.0
    for a, b in zip(pts, pts[1:]):
        dx, dy = b[0] - a[0], b[1] - a[1]
        L = math.hypot(dx, dy)
        u = max(0.0, min(1.0, ((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / (L * L)))
        dist = math.hypot(p[0] - a[0] - u * dx, p[1] - a[1] - u * dy)
        if dist < best[0]:
            best = (dist, total + u * L)
        total += L
    return best[0], best[1], total


def perp(p, pts):
    """Distance from p to the polyline, measured square to a segment that p projects within; None
    past both ends."""
    out = None
    for a, b in zip(pts, pts[1:]):
        dx, dy = b[0] - a[0], b[1] - a[1]
        L2 = dx * dx + dy * dy
        u = ((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / L2
        if 0 <= u <= 1:
            dist = abs((p[0] - a[0]) * dy - (p[1] - a[1]) * dx) / math.sqrt(L2)
            out = dist if out is None else min(out, dist)
    return out


def signed(p, pts):
    """Signed distance from p to the polyline (left of travel positive), square to a segment that p
    projects within and away from its ends' taper; None past both ends."""
    out = None
    for a, b in zip(pts, pts[1:]):
        dx, dy = b[0] - a[0], b[1] - a[1]
        L2 = dx * dx + dy * dy
        u = ((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / L2
        if 0.05 <= u <= 0.95:
            sd = (dx * (p[1] - a[1]) - dy * (p[0] - a[0])) / math.sqrt(L2)
            out = sd if out is None or abs(sd) < abs(out) else out
    return out


def glb(path):
    """World-frame triangles of a one-node glb (build.md 10)."""
    raw = Path(path).read_bytes()
    jlen = struct.unpack("<I", raw[12:16])[0]
    doc = json.loads(raw[20:20 + jlen])
    bin_ = raw[20 + jlen + 8:]
    node = next(n for n in doc["nodes"] if "mesh" in n)
    tx, ty, tz = node.get("translation", [0, 0, 0])
    prim = doc["meshes"][node["mesh"]]["primitives"][0]

    def acc(k):
        a = doc["accessors"][k]
        v = doc["bufferViews"][a["bufferView"]]
        off = v.get("byteOffset", 0) + a.get("byteOffset", 0)
        comp = {5126: "f", 5125: "I", 5123: "H"}[a["componentType"]]
        n = {"SCALAR": 1, "VEC3": 3}[a["type"]] * a["count"]
        return struct.unpack_from(f"<{n}{comp}", bin_, off)

    p = acc(prim["attributes"]["POSITION"])
    verts = [(p[k] + tx, -(p[k + 2] + tz), p[k + 1] + ty) for k in range(0, len(p), 3)]
    idx = acc(prim["indices"])
    return verts, [tuple(idx[k:k + 3]) for k in range(0, len(idx), 3)]


def measure(build, surfaces=None):
    d = Path(build)
    man = json.loads((d / "manifest.json").read_text())
    g = Grid(d, man["frame"])
    fs = {f["name"]: f for f in json.loads((d / "features.json").read_text())["features"]}
    meets = json.loads((d / "meets.json").read_text())["meets"]
    diags = json.loads((d / "diagnostics.json").read_text())["diagnostics"]
    out = {"meetings": len(meets),
           "gap_max_mm": round(max((abs(m["gap"] or 0) for m in meets), default=0) * 1000),
           "errors": sum(1 for x in diags if x.get("severity") == "error")}

    # ---- the cave
    cave = fs["sea_cave"]
    cl = [p[:2] for p in cave["points"]]
    ca = cave["attrs"]
    half, z0, z1 = ca["width"] / 2, ca["z0"], ca["z1"]
    row = next((m for m in meets if {m["a"], m["b"]} == {"sea_cave.mouth", "sea_cave.mouth_ground"}), None)
    out["mouth_gap_mm"] = round(abs(row["gap"]) * 1000) if row else -1
    out["mouth_at"] = f"{row['x']:g} {row['y']:g}" if row else ""
    out["jambs_met"] = sum(1 for m in meets if m["b"] in ("sea_cave.left_jamb", "sea_cave.right_jamb")
                           and m["gap"] == 0)
    lo = [min(p[k] for p in cl) - half - 1 for k in (0, 1)]
    hi = [max(p[k] for p in cl) + half + 1 for k in (0, 1)]
    i0, j0 = g.quad_of(*lo)
    i1, j1 = g.quad_of(*hi)
    mouth_open = roof_cut = 0
    roof = math.inf
    for j in range(j0, j1 + 1):
        for i in range(i0, i1 + 1):
            c = g.centre(i, j)
            dist, s, total = near_line(c, cl)
            if dist <= half and 0 < s < total:
                if g.is_cut(i, j):
                    if s <= 2:
                        mouth_open += 1
                    else:
                        roof_cut += 1
            px, py = g.x0 + i * g.r, g.y0 + j * g.r
            dist, s, total = near_line((px, py), cl)
            if dist <= half and 2 < s < total:
                roof = min(roof, g.post("height", i, j) - (z0 + (z1 - z0) * s / total))
    out["mouth_open"], out["roof_cut"], out["roof_min_m"] = mouth_open, roof_cut, round(roof, 2)
    a = fs["at_cave_mouth"]["at"][:2]
    m = cl[0]
    L = math.hypot(m[0] - a[0], m[1] - a[1])
    n = int(L / 0.5)
    prof = [g.at("height", a[0] + (m[0] - a[0]) * k / n, a[1] + (m[1] - a[1]) * k / n) for k in range(n + 1)]
    step = L / n
    out["beach_grade_deg"] = round(max(math.degrees(math.atan(abs(q - p) / step)) for p, q in zip(prof, prof[1:])), 2)
    cut = set()
    wet = 0.0
    for k in range(int(L / 0.1)):
        x, y = a[0] + (m[0] - a[0]) * k * 0.1 / L, a[1] + (m[1] - a[1]) * k * 0.1 / L
        i, j = g.quad_of(x, y)
        if g.is_cut(i, j):
            cut.add((i, j))
        wet = max(wet, g.at("water", x, y) - g.at("height", x, y))
    out["beach_cut"], out["beach_wet_m"] = len(cut), round(wet, 3)
    total = near_line(cl[0], cl)[2]
    out["floor_grade_deg"] = round(math.degrees(math.atan(abs(z1 - z0) / total)), 2)
    out["cave_reached"] = int(out["mouth_gap_mm"] == 0 and out["beach_grade_deg"] <= 34 and wet <= 0.7
                              and not cut and out["floor_grade_deg"] <= 34)

    # ---- the cliff
    faces = [f for f in fs.values() if f.get("kind") == "sheer_cliff"]
    tris = steep = area = 0.0
    brink = []
    face_len = 0.0
    for f in faces:
        line = [p[:2] for p in f["points"]]
        lip = f["attrs"]["lip"]
        face_len += near_line(line[0], line)[2]
        verts, faces_ = glb(d / "meshes" / f"{f['name']}.face.glb")
        tris += len(faces_)
        for t in faces_:
            p, q, r = (verts[k] for k in t)
            ux, uy, uz = q[0] - p[0], q[1] - p[1], q[2] - p[2]
            vx, vy, vz = r[0] - p[0], r[1] - p[1], r[2] - p[2]
            nx, ny, nz = uy * vz - uz * vy, uz * vx - ux * vz, ux * vy - uy * vx
            ar = 0.5 * math.sqrt(nx * nx + ny * ny + nz * nz)
            if ar == 0:
                continue
            slope = math.degrees(math.acos(min(1.0, abs(nz) / (2 * ar))))
            area += ar
            steep += ar if slope > 80 else 0
        top, lip = f["attrs"]["top_z"], f["attrs"]["lip"]
        for v in verts:
            if abs(v[2] - top) <= 0.001:
                sd = signed(v[:2], line)
                # The strip's own brink: the skirt's flat past it (build.md 10) is left out.
                if sd is not None and abs(sd) <= lip + 0.001:
                    brink.append(sd)
    out["face_triangles"] = int(tris)
    out["face_steep_share"] = round(steep / area, 3) if area else 0
    out["brink_vertices"] = len(brink)
    out["brink_dev_mm"] = round((max(brink) - min(brink)) * 1000) if brink else -1

    mouth = cl[0]
    # A cutting path owner's mesh covers its cut quads out to its skirt (build.md 10): the strip
    # grown by ceil(0.7072 r) mm. The faces and the cave's floor cut; the deck does not.
    skirt = math.ceil(0.7072 * g.r * 1000) / 1000
    strips = [([p[:2] for p in f["points"]], f["attrs"]["lip"] + skirt) for f in faces]
    strips.append((cl, half + skirt))
    br = fs.get("harbour_bridge")
    if br:
        strips.append(([p[:2] for p in br["points"]], br["attrs"]["width"] / 2))
    face_cut = step_left = 0
    sliver = 0.0
    tbrink = []
    for f in faces:
        line = [p[:2] for p in f["points"]]
        lip, foot, top = f["attrs"]["lip"], f["attrs"]["foot_z"], f["attrs"]["top_z"]
        xs, ys = [p[0] for p in line], [p[1] for p in line]
        i0, j0 = g.quad_of(min(xs) - 3, min(ys) - 3)
        i1, j1 = g.quad_of(max(xs) + 3, max(ys) + 3)
        for j in range(max(j0, 0), min(j1, g.ny - 2) + 1):
            for i in range(max(i0, 0), min(i1, g.nx - 2) + 1):
                c = g.centre(i, j)
                dc = perp(c, line)
                if dc is None or dc > 3:
                    continue
                if g.is_cut(i, j):
                    face_cut += dc <= lip
                    open_ = 0
                    for a_ in range(5):
                        for b_ in range(5):
                            sx = g.x0 + (i + 0.1 + 0.2 * a_) * g.r
                            sy = g.y0 + (j + 0.1 + 0.2 * b_) * g.r
                            if not any(near_line((sx, sy), pts)[0] <= h for pts, h in strips):
                                open_ += 1
                    sliver += open_ / 25 * g.r * g.r
                else:
                    hs = [g.post("height", i + a_, j + b_) for a_ in (0, 1) for b_ in (0, 1)]
                    if max(hs) - min(hs) > (top - foot) / 2 and math.hypot(c[0] - mouth[0], c[1] - mouth[1]) > 4:
                        step_left += 1
                if abs(g.post("height", i, j) - top) <= 0.001 and any(
                        abs(g.post("height", i + a_, j + b_) - foot) <= 0.001
                        for a_, b_ in ((1, 0), (-1, 0), (0, 1), (0, -1))):
                    sd = signed((g.x0 + i * g.r, g.y0 + j * g.r), line)
                    if sd is not None:
                        tbrink.append(sd)
    out["face_cut"], out["step_left"] = face_cut, step_left
    out["sliver_m2_per_m"] = round(sliver / face_len, 3) if face_len else 0
    out["terrain_brink_dev_mm"] = round((max(tbrink) - min(tbrink)) * 1000) if tbrink else -1

    # ---- the bridge
    if br:
        (x0, y0), (x1, y1) = br["points"][0][:2], br["points"][-1][:2]
        length = math.hypot(x1 - x0, y1 - y0)
        ux, uy = (x1 - x0) / length, (y1 - y0) / length
        half_w = br["attrs"]["width"] / 2
        posts = wet_ = 0
        for j in range(g.ny):
            py = g.y0 + j * g.r
            if not min(y0, y1) - half_w - 1 <= py <= max(y0, y1) + half_w + 1:
                continue
            for i in range(g.nx):
                px = g.x0 + i * g.r
                if not min(x0, x1) - half_w - 1 <= px <= max(x0, x1) + half_w + 1:
                    continue
                al, ac = (px - x0) * ux + (py - y0) * uy, abs((py - y0) * ux - (px - x0) * uy)
                if 0 <= al <= length and ac <= half_w:
                    posts += 1
                    wet_ += g.post("water", i, j) - g.post("height", i, j) > 0.001
        mx, my = (x0 + x1) / 2, (y0 + y1) / 2
        w, h = g.at("water", mx, my), g.at("height", mx, my)
        out["deck_posts"], out["deck_wet"] = posts, wet_
        out["under_depth_m"] = round(w - h, 3)
        out["freeboard_m"] = round(br["attrs"]["deck_z"] - w, 3)
        out["deck_length_m"] = round(length, 2)
    names = (d / "instances" / "names.txt").read_text().split()
    out["parapet_posts"] = sum(1 for x in names if "parapet" in x)

    if surfaces:
        s = json.loads(Path(surfaces).read_text())
        at0 = [name for p, name in zip(s["point"], s["surface"]) if p == 0]
        out["cave_surfaces"] = len(at0)
        out["cave_surface_names"] = " ".join(at0)
    return out


def check(got, section):
    import tomllib
    rows = tomllib.loads((HERE / "expect.toml").read_text())[section]
    bad = 0
    for key, row in rows.items():
        if key == "protocol":
            continue
        v = got.get(key)
        if "text" in row:
            ok = v == row["text"]
        else:
            lo, hi = row["band"]
            ok = isinstance(v, (int, float)) and lo <= v <= hi
        bad += not ok
        print(f"{'ok ' if ok else 'BAD'} {section}.{key} = {v!r}  ({row.get('text', row.get('band'))})")
    return bad


def main(argv):
    build, surfaces, section = argv[1], None, None
    if "--surfaces" in argv:
        surfaces = argv[argv.index("--surfaces") + 1]
    if "--check" in argv:
        section = argv[argv.index("--check") + 1]
    got = measure(build, surfaces)
    print(json.dumps(got, indent=1, sort_keys=True))
    if section:
        return 1 if check(got, section) else 0
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
