"""Measures of an atlas build, for expect.toml (docs/plan.md M6: every example joined in one map).

    python3 examples/atlas/measure.py BUILD [--run FILE] [--world FILE] [--check SECTION]

Prints the measures as JSON. With --check, compares them against expect.toml's rows under SECTION
and exits 1 when one falls outside its band or names a measure not taken (Python 3.11+, for
tomllib). Reads only the build directory (spec/build.md): manifest.json, diagnostics.json,
features.json, fields/height.npy; the world file (default this folder's world.toml; a change's
moved import is read from --world, a copy of world.toml with the change's keys) for where each
import stands; with --run, the run report (build.md 11.3). Standard library only.

How each is taken (ours):
- `errors`, `warnings`, `codes`: diagnostics by severity, and a count per code.
- `over_budget`, `over_budget_asset`, `over_budget_chunks`: `build.over_budget` records, the
  dominant asset each names (`data.asset`, sorted, unique) and the chunks (`data.chunk`).
- `unanswered`, `unanswered_write`: `import.exit_unanswered` records and each one's `data.write`.
- `exit_gap`: for every atlas way (kind road, lane, trail) whose `starts_at` or `ends_at` names an
  import's place, the largest distance, metres, from the way's first or last point to that place
  as features.json places it: 0 when every way ends on the place it answers (R9).
- `grade_max`, `grade_p95`: over every atlas way, the rise over run of the finished ground
  between its points taken every 4 m along it (height at the nearest post): the steepest step, and
  the 95th percentile of all steps. A way's own bed is shaped, so this is what a walker climbs.
- `join_step_max`, `join_step`: per import, the median over its outline (as placed, every 8 m) of
  the finished height 4 m outside the footprint minus 28 m inside it (inside the blend's
  fade, the import's own ground); the largest absolute of these. What the lift and blend leave.
- `seam`, `seam_grade`: `import.seam` records and their largest `data.grade` (composition.md 6.6).
- `chunks`, `tri_max`, `tri_max_at`, `budget`: chunks in the manifest, the largest `triangles` of
  any chunk and its `at`, and the world's `[budget] triangles`.
- `bookmarks`: features of kind `bookmark`.
- `road_painted`, `road_painted_in`: of the points every 4 m along the atlas's own ways (not an
  import's), outside every import's outline as placed and inside one, the share where `ground`'s
  `earth` plane is the largest at the nearest post (build.md 6.2 chunk files): the road seen on the
  ground, not only in features.json. Inside an import its own ground is brought over the plain's
  rows, so a road's last stretch to an import's place is that import's paint.
- With --run: `ran` (nodes that ran), `ran_imports` (import ids with any node `import:<id>` or
  `placed:<id>...` that ran, sorted), `fields_ran` and `fields` (field chunk nodes that ran, all
  of them), `shapes_ran` (shape nodes that ran, by name), `wall_s` (the run's wall seconds, when
  the report holds it).
"""

import json
import math
import struct
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
WAYS = ("road", "lane", "trail")


def grid(path):
    """A 2-D little-endian float32 .npy: (rows, columns, flat values)."""
    raw = Path(path).read_bytes()
    hlen = struct.unpack("<H", raw[8:10])[0]
    head = raw[10:10 + hlen].decode()
    rows, cols = (int(v) for v in head.split("'shape': (")[1].split(")")[0].split(",")[:2])
    body = raw[10 + hlen:]
    return rows, cols, struct.unpack(f"<{len(body) // 4}f", body)


def toml(path):
    import tomllib

    return tomllib.loads(Path(path).read_text())


def placed(world):
    """Each import's outline as placed: id -> four corners, from its extent, `at` and `face`."""
    w = toml(world)
    out = {}
    for iid, imp in w.get("import", {}).items():
        ex = toml(Path(world).parent / imp["file"])["extent"]
        ax, ay = imp["at"]
        a = math.radians(imp.get("face", 0))
        c, s = math.cos(a), math.sin(a)

        def put(x, y):                         # +y turned to the bearing `face`, clockwise from north
            return (ax + x * c + y * s, ay - x * s + y * c)

        out[iid] = [put(ex[0], ex[1]), put(ex[2], ex[1]), put(ex[2], ex[3]), put(ex[0], ex[3])]
    return out


def sampler(d):
    m = json.loads((d / "manifest.json").read_text())["frame"]
    x0, y0, res = m["extent"][0], m["extent"][1], m["resolution"]
    rows, cols, v = grid(d / "fields" / "height.npy")

    def h(x, y):
        i = min(max(round((x - x0) / res), 0), cols - 1)
        j = min(max(round((y - y0) / res), 0), rows - 1)
        return v[j * cols + i]

    return h


def u8planes(path):
    """A 3-D `|u1` .npy (planes, rows, columns): (planes, rows, columns, bytes)."""
    raw = Path(path).read_bytes()
    hlen = struct.unpack("<H", raw[8:10])[0]
    head = raw[10:10 + hlen].decode()
    p, r, c = (int(v) for v in head.split("'shape': (")[1].split(")")[0].split(",")[:3])
    return p, r, c, raw[10 + hlen:]


def earth_at(d, man):
    """A function (x, y) -> whether `earth` is the largest plane of `ground` at the nearest post."""
    fr = man["frame"]
    x0, y0, res = fr["extent"][0], fr["extent"][1], fr["resolution"]
    chunks = []
    for c in man.get("chunks", []):
        w = c.get("weights", {}).get("ground")
        if w and w.get("file") and "earth" in w.get("materials", []):
            chunks.append((c["posts"], w["materials"].index("earth"), d / w["file"]))
    cache = {}

    def at(x, y):
        i, j = round((x - x0) / res), round((y - y0) / res)
        for posts, k, f in chunks:
            if posts[0] <= i <= posts[2] and posts[1] <= j <= posts[3]:
                if f not in cache:
                    cache[f] = u8planes(f)
                p, r, cc, b = cache[f]
                ci, cj = i - posts[0], j - posts[1]
                vals = [b[q * r * cc + cj * cc + ci] for q in range(p)]
                return vals[k] > 0 and vals[k] == max(vals)
        return False

    return at


def inside(x, y, q):
    """Whether (x, y) lies inside the polygon q (even-odd rule)."""
    hit = False
    for (ax, ay), (bx, by) in zip(q, q[1:] + q[:1]):
        if (ay > y) != (by > y) and x < ax + (y - ay) * (bx - ax) / (by - ay):
            hit = not hit
    return hit


def join_steps(h, outlines):
    steps = {}
    for iid, q in outlines.items():
        cx = sum(p[0] for p in q) / 4
        cy = sum(p[1] for p in q) / 4
        diffs = []
        for (ax, ay), (bx, by) in zip(q, q[1:] + q[:1]):
            L = math.dist((ax, ay), (bx, by))
            nx, ny = (by - ay) / L, -(bx - ax) / L             # outward for a counter-clockwise outline
            if (ax + nx - cx) ** 2 + (ay + ny - cy) ** 2 < (ax - cx) ** 2 + (ay - cy) ** 2:
                nx, ny = -nx, -ny
            for k in range(1, int(L // 8)):
                t = k * 8 / L
                px, py = ax + t * (bx - ax), ay + t * (by - ay)
                if min(px, py) < 8 or px > 3192 or py > 2552:  # the extent's edge: no outside
                    continue
                diffs.append(h(px + 4 * nx, py + 4 * ny) - h(px - 28 * nx, py - 28 * ny))
        diffs.sort()
        steps[iid] = round(diffs[len(diffs) // 2], 2) if diffs else None
    return steps


def measure(d, run=None, world=None):
    d = Path(d)
    diags = json.loads((d / "diagnostics.json").read_text())["diagnostics"]
    codes = {}
    for x in diags:
        codes[x["code"]] = codes.get(x["code"], 0) + 1
    ob = [x for x in diags if x["code"] == "build.over_budget"]
    m = {
        "errors": sum(1 for x in diags if x["severity"] == "error"),
        "warnings": sum(1 for x in diags if x["severity"] == "warning"),
        "codes": dict(sorted(codes.items())),
        "over_budget": len(ob),
        "over_budget_asset": sorted({x["data"].get("asset") for x in ob}),
        "over_budget_chunks": sorted(x["data"].get("chunk") for x in ob),
        "unanswered": codes.get("import.exit_unanswered", 0),
        "unanswered_write": [x["data"].get("write") for x in diags if x["code"] == "import.exit_unanswered"],
    }
    seams = [x for x in diags if x["code"] == "import.seam"]
    m["seam"] = len(seams)
    m["seam_grade"] = round(max((x["data"]["grade"] for x in seams), default=0.0), 3)
    if run:
        rr = json.loads(Path(run).read_text())
        nodes = rr["nodes"]
        m["ran"] = sum(1 for n in nodes if n["status"] == "ran")
        ids = set()
        for n in nodes:
            if n["status"] == "ran" and n["id"].split(":", 1)[0] in ("import", "placed"):
                ids.add(n["id"].split(":", 1)[1].split("/")[0].split(".")[0])
        m["ran_imports"] = sorted(ids)
        fl = [n for n in nodes if n["family"] == "field"]
        m["fields_ran"] = sum(1 for n in fl if n["status"] == "ran")
        m["fields"] = len(fl)
        m["shapes_ran"] = sorted(
            {n["id"].split(":", 1)[1].split("/")[0] for n in nodes if n["family"] == "shape" and n["status"] == "ran"}
        )
        if "wall_s" in rr:
            m["wall_s"] = rr["wall_s"]
    if not (d / "features.json").exists():      # a build that ended in an error writes no data
        return m
    fs = {f["name"]: f for f in json.loads((d / "features.json").read_text())["features"]}
    h = sampler(d)
    earth = earth_at(d, json.loads((d / "manifest.json").read_text()))
    outlines = placed(world or HERE / "world.toml")
    gap, steps, painted, sampled = 0.0, [], [0, 0], [0, 0]
    for name, f in fs.items():
        if f.get("kind") not in WAYS or "." in name or f.get("shape") != "path":
            continue
        pts = [p[:2] for p in f["points"]]
        for key, end in (("starts_at", pts[0]), ("ends_at", pts[-1])):
            ref = f["attrs"].get(key)
            ref = ref.get("name") if isinstance(ref, dict) else ref
            if not ref or "." not in ref:
                continue
            base = ref.rsplit(".", 1)
            tgt = fs.get(ref)
            if tgt and "at" in tgt:
                q = tgt["at"][:2]
            elif base[0] in fs and base[1] in ("start", "end"):
                p = fs[base[0]]["points"]
                q = (p[0] if base[1] == "start" else p[-1])[:2]
            else:
                continue
            gap = max(gap, math.dist(end, q))
        walked = 0.0
        for (ax, ay), (bx, by) in zip(pts, pts[1:]):
            L = math.dist((ax, ay), (bx, by))
            k = max(1, int(L // 4))
            for i in range(k):
                t0, t1 = i / k, (i + 1) / k
                z0 = h(ax + t0 * (bx - ax), ay + t0 * (by - ay))
                z1 = h(ax + t1 * (bx - ax), ay + t1 * (by - ay))
                steps.append(abs(z1 - z0) / (L / k))
                px, py = ax + t0 * (bx - ax), ay + t0 * (by - ay)
                k_in = any(inside(px, py, q) for q in outlines.values())
                sampled[k_in] += 1
                painted[k_in] += earth(px, py)
            walked += L
    steps.sort()
    m["exit_gap"] = round(gap, 3)
    m["road_painted"] = round(painted[0] / sampled[0], 3) if sampled[0] else None
    m["road_painted_in"] = round(painted[1] / sampled[1], 3) if sampled[1] else None
    m["grade_max"] = round(steps[-1], 3) if steps else None
    m["grade_p95"] = round(steps[int(0.95 * (len(steps) - 1))], 3) if steps else None
    js = join_steps(h, placed(world or HERE / "world.toml"))
    m["join_step"] = js
    m["join_step_max"] = max(abs(v) for v in js.values() if v is not None)
    man = json.loads((d / "manifest.json").read_text())
    ch = man.get("chunks", [])
    m["chunks"] = len(ch)
    top = max(ch, key=lambda c: c.get("triangles", 0)) if ch else {}
    m["tri_max"] = top.get("triangles", 0)
    m["tri_max_at"] = top.get("at")
    m["budget"] = toml(world or HERE / "world.toml").get("budget", {}).get("triangles")
    m["bookmarks"] = sum(1 for f in fs.values() if f.get("kind") == "bookmark" and "." not in f["name"])
    return m


def check(m, section):
    rows = toml(HERE / "expect.toml")[section]
    bad = 0
    for name, row in rows.items():
        got = m.get(row["measure"])
        band = row["band"]
        if got is None:
            ok = False
        elif band and isinstance(band[0], (str, list)) or not band:
            ok = got == band
        else:
            ok = band[0] <= got <= band[1]
        bad += not ok
        print(f"{'ok ' if ok else 'BAD'} {section}.{name}: {row['measure']} = {got} in {band}")
    return bad


if __name__ == "__main__":
    args = sys.argv[1:]

    def opt(k):
        if k in args:
            i = args.index(k)
            v = args[i + 1]
            del args[i:i + 2]
            return v
        return None

    run, world, section = opt("--run"), opt("--world"), opt("--check")
    m = measure(args[0], run, world)
    print(json.dumps(m, indent=1))
    if section:
        sys.exit(1 if check(m, section) else 0)
