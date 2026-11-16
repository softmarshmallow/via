"""Measures of a station build, for expect.toml (docs/plan.md M5: indoors reachable through a door).

    python3 examples/station/measure.py BUILD [--surfaces FILE] [--check SECTION]

Prints the measures as JSON. With --check, compares them against expect.toml's rows under SECTION
(`base`, `move_stair`, `close_passage`) and exits 1 when one falls outside its band or names a
measure not taken (Python 3.11+, for tomllib). Reads only the build directory (spec/build.md):
features.json, meets.json, diagnostics.json and instances/; with --surfaces, the output of
`via query WORLD surfaces_at --in stair_points.json --out FILE` (surfaces.md 2.1, 7). Standard
library only.

How each is taken (ours, M5 slice 1):
- `storeys`, `rooms`, `flights`, `doorways`: features of those kinds (lib/building's).
- `meetings`, `gap_max_mm`, `gaps_open`: rows of meets.json, the largest |gap| and how many are not 0.
- `doors_in`: meetings whose `a` is a doorway's end and whose `b` is a storey's floor (a door).
- `floors_joined`: storeys named as `b` of a meeting (a floor some path steps onto).
- `unreached`, `unreached_names`: lib/building's lint (`step.check.unreached`), storeys and rooms no
  walk from the terrain reaches (check.py rules 1 and 2), and their names, sorted.
- `checked_meetings`: the meetings the lint judged (`step.check.checked`).
- `instances`, `dressing`: every instance, and those that are not wall pieces (`*_wall*` names) and
  not laid by lib/building's `lay` (`lay#*`).
- `laid`, `laid_off_floor` (M5 slice 2): the net loft's instances (`lay#<storey>_*`, a run's
  members `lay#<storey>_<run>#r<k>`), and those whose z is not their storey's `level` within 1 mm.
- `on_floor`, `off_floor`, `off_floor_groups`: of the instances placed on a storey (a hand instance
  or set with `surface = "<storey>.floor"`, a run `on` a storey, read from the world's text), those
  whose z is within 1 mm of the storey's level less their asset's `sink` (pack.toml; the `*_top`
  ids at -0.76 m, the upper shell at -3.25 m), those that are not, and the names of the placements
  the latter belong to.
- `shell_gave_way`: tower shell members dropped by overlap (`place.gave_way` on `run.tower_wall`
  and `run.tower_wall_upper`).
- `wall_gave_way` (M5 slice 3): wall members dropped by overlap over every run whose asset is a
  wall piece (taxon `structure/wall` or `structure/kit` in pack.toml), and every run `lay` returns
  (`run.lay#*`, all kit pieces).
- `open_above_door_mm` (M5 slice 3): at the tower's doorway, the most open wall above the door
  head. Along the `tower_step`'s line every 50 mm, at the samples inside `tower_0` within 0.3 m of
  its outline (the shell's band), the length of the height from the door head (`tower_1`'s level,
  the ground storey's top) to the shell's top (the highest top of a `structure/wall` piece standing
  on `tower_0`'s level or above it, foot + `bounds` height) that no wall piece whose footprint holds
  the sample covers; the most over the samples. 9350 before slice 3 (the gap ran the full height).
- `crossings`: `geom.unowned_crossing` warnings (stacked flights crossing in plan).
- `door_narrow`: lib/building's lint (`step.check.door_narrow`, check.py rule 5 with `kits`), doors
  whose clear width between wall pieces is under 0.9 m; `doors_measured`, the doors it measured.
- `clear_min_mm`, `clear_min_name` (ours, M5 slice 2): along each doorway and the `tower_step`,
  every 50 mm from 0.5 m before its start to 0.5 m past its end, the free width across its line
  between the footprints (manifest `bounds`, scaled, turned by `face`) of wall pieces (taxon
  `structure/wall`) whose foot stands within 0.5 m below to 2 m above the way's height there (its
  `sill_z`, or `z0` to `z1`); a side with no piece within 3 m counts 3 m. The least over all
  samples, and the way it is on. A piece over the line itself gives 0.
- `meshes`: the manifest's generated surface meshes (build.md 10), one per floor and tread.
- With --surfaces: `stair_surfaces`, the surfaces listed at the first point (stair_1's middle,
  [17.45, 32.05]), and `stair_surface_names`, their names in order.
"""

import json
import math
import re
import struct
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent


def npy(path):
    raw = Path(path).read_bytes()
    hlen = struct.unpack("<H", raw[8:10])[0]
    head = raw[10:10 + hlen].decode()
    body = raw[10 + hlen:]
    code = "f" if "f4" in head else "i"
    return struct.unpack(f"<{len(body) // 4}{code}", body)


def placed_on_floors():
    """{placement name: storey} from the world's text: `surface = "<storey>.floor"` on an instance,
    and `on = { ref = "<storey>", ... }` on a run."""
    out = {}
    cur = None
    for line in (HERE / "world.toml").read_text().splitlines():
        m = re.match(r"\[(instance|run)\.(\w+)\]", line)
        if m:
            cur = m.group(2)
            continue
        if line.startswith("["):
            cur = None
            continue
        if cur is None:
            continue
        m = re.match(r'surface\s*=\s*"(\w+)\.floor"', line) or re.match(r'on\s*=\s*\{\s*ref\s*=\s*"(\w+)"', line)
        if m:
            out[cur] = m.group(1)
    return out


def measure(build, surfaces=None):
    d = Path(build)
    fs = json.loads((d / "features.json").read_text())["features"]
    kinds = {f["name"]: f["kind"] for f in fs}
    level = {f["name"]: f["attrs"]["level"] for f in fs if f["kind"] == "storey"}
    meets = json.loads((d / "meets.json").read_text())["meets"]
    diags = json.loads((d / "diagnostics.json").read_text())
    diags = diags.get("diagnostics", diags) if isinstance(diags, dict) else diags
    out = {k + "s": sum(1 for v in kinds.values() if v == k) for k in ("storey", "room", "flight", "doorway")}
    gaps = [abs(m["gap"] or 0) for m in meets]
    out["meetings"] = len(meets)
    out["gap_max_mm"] = round(max(gaps, default=0) * 1000)
    out["gaps_open"] = sum(1 for g in gaps if g)
    out["doors_in"] = sum(1 for m in meets if kinds.get(m["a"].rpartition(".")[0]) == "doorway"
                          and m["b"].endswith(".floor") and kinds.get(m["b"][:-6]) == "storey")
    out["floors_joined"] = len({m["b"][:-6] for m in meets if m["b"].endswith(".floor")})
    un = sorted(r["subject"].split(".", 1)[1] for r in diags if r["code"] == "step.check.unreached")
    out["unreached"] = len(un)
    out["unreached_names"] = " ".join(un)
    chk = [r for r in diags if r["code"] == "step.check.checked"]
    out["checked_meetings"] = chk[0]["data"]["meetings"] if chk else -1
    out["shell_gave_way"] = sum(r["data"]["overlap"] for r in diags if r["code"] == "place.gave_way"
                                and r["subject"] in ("run.tower_wall", "run.tower_wall_upper"))
    walls = wall_runs()
    out["wall_gave_way"] = sum(r["data"]["overlap"] for r in diags if r["code"] == "place.gave_way"
                               and (r["subject"][4:] in walls or r["subject"].startswith("run.lay#")))
    out["open_above_door_mm"] = open_above_door(d, fs, level)
    out["crossings"] = sum(1 for r in diags if r["code"] == "geom.unowned_crossing")

    out["door_narrow"] = sum(1 for r in diags if r["code"] == "step.check.door_narrow")
    out["doors_measured"] = chk[0]["data"].get("doorways", -1) if chk else -1
    out["clear_min_mm"], out["clear_min_name"] = clear_min(d, fs)
    out["meshes"] = len(json.loads((d / "manifest.json").read_text()).get("meshes", []))

    names = (d / "instances" / "names.txt").read_text().split()
    z = npy(d / "instances" / "z.npy")
    out["instances"] = len(names)
    out["dressing"] = sum(1 for n in names if "_wall" not in n and not n.startswith(("shrubs#", "lay#")))
    laid = [(n, v) for n, v in zip(names, z) if n.startswith("lay#")]
    out["laid"] = len(laid)
    out["laid_off_floor"] = sum(1 for n, v in laid
                                if abs(v - level.get(re.match(r"(lay#\w+?_s\d+)", n).group(1), 1e9)) > 0.001)
    on = placed_on_floors()
    sinks = pack_sinks()
    asset_of = row_assets(d)
    good, bad, groups = 0, 0, set()
    for r, (n, v) in enumerate(zip(names, z)):
        head = re.split(r"[.#]", n)[0]
        st = on.get(head)
        if st is None or st not in level:
            continue
        want = level[st] - sinks.get(asset_of.get(r), 0.0)
        if abs(v - want) <= 0.001:
            good += 1
        else:
            bad += 1
            groups.add(head)
    out["on_floor"], out["off_floor"] = good, bad
    out["off_floor_groups"] = " ".join(sorted(groups))
    out["in_flight"], out["in_flight_names"] = in_flight(d, fs, names, z, asset_of)

    if surfaces:
        s = json.loads(Path(surfaces).read_text())
        at0 = [name for p, name in zip(s["point"], s["surface"]) if p == 0]
        out["stair_surfaces"] = len(at0)
        out["stair_surface_names"] = " ".join(at0)
    return out


REACH, LOW, HIGH, STEP, PAST = 3000, 0.5, 2.0, 50, 500


def pack_tables():
    """{asset id: {key: raw value text}} from pack.toml's `[asset.<id>]` tables."""
    out, cur = {}, None
    for line in (HERE / "pack.toml").read_text().splitlines():
        m = re.match(r"\[asset\.(\w+)\]", line)
        if m:
            cur = out.setdefault(m.group(1), {})
            continue
        if line.startswith("["):
            cur = None
            continue
        m = re.match(r"(\w+)\s*=\s*([^#]*)", line)
        if cur is not None and m:
            cur[m.group(1)] = m.group(2).strip()
    return out


def in_flight(d, fs, names, z, asset_of):
    """Dressing standing in a flight's way: an instance on the floor at the flight's foot (its z
    within 0.05 m of `z0` less its sink) whose plan disc (half its primitive's larger side) comes
    within the walker's radius (0.3 m, walk.gd) of the flight's line at a station where the tread
    is lower than the thing's top: the walker on the line meets it. Stations every 0.1 m along the
    line, the rise by length. Walls, lay# pieces and shrubs are left out (the shell, not dressing).
    Reason: move_stair laid stair_2's foot at the living table and no lint said so."""
    tables = pack_tables()
    sinks = pack_sinks()
    xs, ys = npy(d / "instances" / "x.npy"), npy(d / "instances" / "y.npy")
    hit = set()
    for f in fs:
        if f["kind"] != "flight":
            continue
        pts = [(p[0], p[1]) for p in f["points"]]
        z0, z1 = f["attrs"]["z0"], f["attrs"]["z1"]
        segs = list(zip(pts, pts[1:]))
        total = sum(math.dist(a, b) for a, b in segs)
        stations, run = [], 0.0
        for a, b in segs:
            L = math.dist(a, b)
            for k in range(int(L * 10) + 1):
                s_ = k / 10 / L if L else 0
                stations.append(((a[0] + (b[0] - a[0]) * s_, a[1] + (b[1] - a[1]) * s_), (run + k / 10) / total * (z1 - z0)))
            run += L
        for r, (n, v) in enumerate(zip(names, z)):
            if "_wall" in n or n.startswith(("lay#", "shrubs#")):
                continue
            a = asset_of.get(r)
            t = tables.get(a, {})
            if "size" not in t:
                continue
            sx, sy, sz = (float(c) for c in t["size"].strip("[]").split(","))
            if abs(v - (z0 - sinks.get(a, 0.0))) > 0.05:
                continue
            p = (xs[r] / 1000, ys[r] / 1000)
            if any(rise < sz and math.dist(p, q) < 0.3 + max(sx, sy) / 2 for q, rise in stations):
                hit.add(n)
    return len(hit), " ".join(sorted(hit))


def pack_sinks():
    return {a: float(t["sink"]) for a, t in pack_tables().items() if "sink" in t}


def wall_runs():
    """Names of the world's runs whose asset is a wall piece (`structure/wall` or `structure/kit`)."""
    walls = {a for a, t in pack_tables().items() if t.get("taxon", "").strip('"') in ("structure/wall", "structure/kit")}
    out, cur = set(), None
    for line in (HERE / "world.toml").read_text().splitlines():
        m = re.match(r"\[run\.(\w+)\]", line)
        if m:
            cur = m.group(1)
            continue
        if line.startswith("["):
            cur = None
            continue
        m = re.match(r'asset\s*=\s*"(\w+)"', line)
        if cur is not None and m and m.group(1) in walls:
            out.add(cur)
    return out


def row_assets(d):
    """{instance row: asset id} from the manifest's chunk ranges."""
    man = json.loads((d / "manifest.json").read_text())
    return {r: aid for ch in man["chunks"] for aid, (first, n) in ch["instances"].items() for r in range(first, first + n)}


def inside(p, ring):
    x, y = p
    k = False
    for (ax, ay), (bx, by) in zip(ring, ring[1:] + ring[:1]):
        if (ay > y) != (by > y) and x < ax + (y - ay) * (bx - ax) / (by - ay):
            k = not k
    return k


def to_edge(p, ring):
    best = math.inf
    for (ax, ay), (bx, by) in zip(ring, ring[1:] + ring[:1]):
        dx, dy = bx - ax, by - ay
        if not dx and not dy:
            continue
        t = max(0.0, min(1.0, ((p[0] - ax) * dx + (p[1] - ay) * dy) / (dx * dx + dy * dy)))
        best = min(best, math.hypot(p[0] - ax - t * dx, p[1] - ay - t * dy))
    return best


def open_above_door(d, fs, level):
    """The most open wall above the tower's door head, mm (see `open_above_door_mm` above)."""
    by = {f["name"]: f for f in fs}
    ring = [(p[0] * 1000, p[1] * 1000) for p in by["tower_0"]["pieces"][0]["outer"]] if "pieces" in by["tower_0"] else None
    if ring is None:
        return -1
    head, base = level["tower_1"] * 1000, level["tower_0"] * 1000
    boxes = [(pts, zp * 1000, h * 1000) for pts, zp, h in wall_boxes(d, heights=True)]
    top = max((zp + h for pts, zp, h in boxes if zp >= base - 1 and any(inside(q, ring) for q in pts)), default=head)
    path = [(p[0] * 1000, p[1] * 1000) for p in by["tower_step"]["points"]]
    (ax, ay), (bx, by_) = path[0], path[-1]
    L = math.hypot(bx - ax, by_ - ay)
    dx, dy = (bx - ax) / L, (by_ - ay) / L
    most = 0
    for i in range(int((L + 2 * PAST) // STEP) + 1):
        s = -PAST + i * STEP
        p = (ax + dx * s, ay + dy * s)
        if not inside(p, ring) or to_edge(p, ring) > 300:
            continue
        spans = sorted((zp, zp + h) for pts, zp, h in boxes if inside(p, pts))
        covered, at = 0.0, head
        for lo, hi in spans:
            lo, hi = max(lo, at), min(hi, top)
            if hi > lo:
                covered += hi - lo
                at = hi
        most = max(most, round(top - head - covered))
    return most


def wall_boxes(d, heights=False):
    """[(corners in mm, foot z)] of every instance of a `structure/wall` asset; with `heights`,
    [(corners, foot z, height in metres)]."""
    man = json.loads((d / "manifest.json").read_text())
    x, y = npy(d / "instances" / "x.npy"), npy(d / "instances" / "y.npy")
    z, face = npy(d / "instances" / "z.npy"), npy(d / "instances" / "face.npy")
    scale = npy(d / "instances" / "scale.npy")
    out = []
    for ch in man["chunks"]:
        for aid, (first, n) in ch["instances"].items():
            a = man["assets"][aid]
            if a.get("taxon") != "structure/wall":
                continue
            b = a["bounds"]
            for r in range(first, first + n):
                k, th = scale[r], math.radians(face[r] / 1000.0)
                c, s_ = math.cos(th), math.sin(th)
                pts = []
                for px, py in ((b[0], b[1]), (b[3], b[1]), (b[3], b[4]), (b[0], b[4])):
                    px, py = px * k * 1000, py * k * 1000
                    pts.append((x[r] + px * c + py * s_, y[r] - px * s_ + py * c))
                out.append((pts, z[r], (b[5] - b[2]) * k) if heights else (pts, z[r]))
    return out


def clear_min(d, fs):
    """The least free width across a door's or the step's line (see `clear_min_mm` above)."""
    boxes = wall_boxes(d)
    best = (None, "")
    for f in fs:
        if not (f["kind"] == "doorway" or f["name"] == "tower_step"):
            continue
        path = [(p[0] * 1000, p[1] * 1000) for p in f["points"]]
        at = f["attrs"]
        z0 = float(at.get("sill_z", at.get("z0", 0)))
        z1 = float(at.get("sill_z", at.get("z1", 0)))
        (ax, ay), (bx, by) = path[0], path[-1]
        L = math.hypot(bx - ax, by - ay)
        dx, dy = (bx - ax) / L, (by - ay) / L
        for i in range(int((L + 2 * PAST) // STEP) + 1):
            s = -PAST + i * STEP
            px, py = ax + dx * s, ay + dy * s
            zw = z0 + (z1 - z0) * min(1.0, max(0.0, s / L))
            left, right = -REACH, REACH
            for pts, zp in boxes:
                if not zw - LOW <= zp <= zw + HIGH:
                    continue
                qs = [(x - px) * dx + (y - py) * dy for x, y in pts]
                cs = [-(x - px) * dy + (y - py) * dx for x, y in pts]
                if max(qs) < 0 or min(qs) > 0:
                    continue
                if min(cs) < 0 < max(cs):
                    left = right = 0
                    break
                if max(cs) <= 0:
                    left = max(left, max(cs))
                else:
                    right = min(right, min(cs))
            w = round(right - left)
            if best[0] is None or w < best[0]:
                best = (w, f["name"])
    return best



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
