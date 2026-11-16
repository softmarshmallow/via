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
- `instances`, `dressing`: every instance, and those that are not wall pieces (`*_wall*` names).
- `on_floor`, `off_floor`, `off_floor_groups`: of the instances placed on a storey (a hand instance
  or set with `surface = "<storey>.floor"`, a run `on` a storey, read from the world's text), those
  whose z is within 1 mm of the storey's level (or level - 0.76 m sink for the pack's `*_top` ids),
  those that are not, and the names of the placements the latter belong to.
- `shell_gave_way`: tower shell members dropped by overlap (`place.gave_way` on `run.tower_wall`).
- `crossings`: `geom.unowned_crossing` warnings (stacked flights crossing in plan).
- With --surfaces: `stair_surfaces`, the surfaces listed at the first point (stair_1's middle,
  [17.45, 32.05]), and `stair_surface_names`, their names in order.
"""

import json
import re
import struct
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
TOP_SINK = 0.76


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
    out["shell_gave_way"] = sum(r["data"]["overlap"] for r in diags
                                if r["code"] == "place.gave_way" and r["subject"] == "run.tower_wall")
    out["crossings"] = sum(1 for r in diags if r["code"] == "geom.unowned_crossing")

    names = (d / "instances" / "names.txt").read_text().split()
    z = npy(d / "instances" / "z.npy")
    out["instances"] = len(names)
    out["dressing"] = sum(1 for n in names if "_wall" not in n and not n.startswith("shrubs#"))
    on = placed_on_floors()
    good, bad, groups = 0, 0, set()
    for n, v in zip(names, z):
        head = re.split(r"[.#]", n)[0]
        st = on.get(head)
        if st is None or st not in level:
            continue
        want = level[st] + (TOP_SINK if n.split(".")[-1] in TOP_ITEMS else 0)
        if abs(v - want) <= 0.001:
            good += 1
        else:
            bad += 1
            groups.add(head)
    out["on_floor"], out["off_floor"] = good, bad
    out["off_floor_groups"] = " ".join(sorted(groups))

    if surfaces:
        s = json.loads(Path(surfaces).read_text())
        at0 = [name for p, name in zip(s["point"], s["surface"]) if p == 0]
        out["stair_surfaces"] = len(at0)
        out["stair_surface_names"] = " ".join(at0)
    return out


# Set items standing on a table top (the pack's `*_top` ids); found by item name.
TOP_ITEMS = {"plate_a", "plate_b", "plate_c", "plate_d", "mug", "candle", "chart", "lamp", "books"}


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
