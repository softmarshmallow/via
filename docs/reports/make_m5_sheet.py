# /// script
# requires-python = ">=3.11"
# dependencies = ["matplotlib>=3.9,<3.10", "numpy>=2.0,<3"]
# ///
"""Draw the M5 sheet, docs/reports/m5-sheet.png, from fresh builds of M5's six worlds and M3/M4's eight.

    uv run docs/reports/make_m5_sheet.py [--keep DIR] [--draw-only] [--fresh]

What it runs, in order (every number on the sheet comes from one of these):

1. The release `via` the VIA environment variable names, else `cargo build --release -p via-cli`.
2. A scratch user config (VIA_CONFIG_DIR) with `python` set to the SDK's venv Python, and each
   world's steps trusted (base, every change, every state's change list, the parts built alone).
3. Per world, in one fresh store: the parts built alone (strait a and b, border's wild), the base
   cold, the base again `--against` itself (unchanged: the memo), then every change (or state)
   `--against` the base. market's move_street and move_bend use a second store (examples/market
   README: on the base's store they stop with diag.internal). Each build is timed by
   /usr/bin/time -l (wall seconds, peak resident memory) and read for its run report's `totals`.
4. For coast and station: `via query <world> surfaces_at --in <points> --out sat.json` (cwd as each
   README says), handed to measure.py --surfaces on the base.
5. Each world's measure.py `--check` on the base and on every change (bands from expect.toml); the
   M3/M4 worlds with make_m4_sheet.py's own measure() (--base, --run, --alone as there).
6. For the ten worlds with views: engines/godot/sheet.sh --before base --after <key change>
   --out <world>/review --option water=tag:water_level (1280x720, eye 1.7 m; walks per
   spec/build.md 15.1, read from review/sheet.json). Every image on the sheet is one of these
   renders, named on the sheet by world, side and file (review/<side>/<bookmark>.png or
   review/<side>/walk/<bookmark>_plan.png); none is taken from another agent's folder.

"Still wrong" and "left to M6" are written by hand from the M5 close reports (review on foot, fix
pass, the fresh agent's run; TODO.md "From M5 close") and from this sheet's views; every line that
is a number is computed from this run. The rulings are R37 to R43 of docs/doctrine.md (all dated
2026-11-17, M5), summarised here by hand.
"""

from __future__ import annotations

import datetime
import json
import os
import platform
import re
import runpy
import shutil
import sys
import tempfile
import textwrap
from pathlib import Path

HERE = Path(__file__).resolve().parent
M4 = runpy.run_path(str(HERE / "make_m4_sheet.py"))
M3 = runpy.run_path(str(HERE / "make_m3_sheet.py"))
ROOT, LIB, PY, VIA = M3["ROOT"], M3["LIB"], M3["PY"], M3["VIA"]
run, jload, timed = M3["run"], M3["jload"], M3["timed"]
m4_measure, m4_rows = M4["measure"], M4["rows"]
plt, mpimg, np = M3["plt"], M3["mpimg"], M3["np"]
OUT = ROOT / "docs/reports/m5-sheet.png"

PISTOL_STATES = {
    "racked": ["changes/racked.toml"],
    "rest+optic": ["accessories/optic.toml"],
    "rest+suppressor": ["accessories/suppressor.toml"],
    "rest+light": ["accessories/light.toml"],
    "racked+optic": ["changes/racked.toml", "accessories/optic.toml"],
    "racked+suppressor": ["changes/racked.toml", "accessories/suppressor.toml"],
    "racked+light": ["changes/racked.toml", "accessories/light.toml"],
    "racked+optic+suppressor+light": ["changes/racked.toml", "accessories/optic.toml",
                                      "accessories/suppressor.toml", "accessories/light.toml"],
}

# Each world: kind ("m5": measured here; "m4": make_m4_sheet's measure), the key change rendered
# against the base (None: timed and measured only), and the measure arguments.
WORLDS = [
    {"name": "station", "kind": "m5", "key": "move_stair", "margs": "surfaces", "points": "stair_points.json"},
    {"name": "college", "kind": "m5", "key": "move_and_turn", "margs": "base"},
    {"name": "coast", "kind": "m5", "key": "move_cave", "margs": "surfaces", "points": "cave_points.json"},
    {"name": "market", "kind": "m5", "key": "module", "margs": "base", "own_store": ("move_street", "move_bend")},
    {"name": "pistol", "kind": "m5", "key": "racked+optic+suppressor+light", "margs": "none", "states": PISTOL_STATES,
     "base_section": "rest"},
    {"name": "sidearm", "kind": "m5", "key": "racked", "margs": "none", "states": {"racked": ["changes/racked.toml"]},
     "base_section": "rest"},
    {"name": "monastery", "kind": "m4", "key": "move_bridge", "margs": "base", "inline": ("winter",)},
    {"name": "city", "kind": "m4", "key": "move_bridge", "margs": "base"},
    {"name": "quay", "kind": "m4", "key": "move_hill", "margs": "run"},
    {"name": "townland", "kind": "m4", "key": "move_crossing", "margs": "base"},
    {"name": "example", "kind": "m4", "key": None, "margs": "none", "inline": ("clear_one", "pin_one", "winter")},
    {"name": "strait", "kind": "m4", "key": None, "margs": "run", "alone": {"a": "a/world.toml", "b": "b/world.toml"},
     "alone_for": ("base", "move_b")},
    {"name": "county", "kind": "m4", "key": None, "margs": "run"},
    {"name": "border", "kind": "m4", "key": None, "margs": "none", "alone": {"wild": "wild/world.toml"},
     "alone_for": ("base",)},
]
for _w in WORLDS:
    _w.setdefault("alone", {})
    _w.setdefault("alone_for", ())


def changes_of(w: dict) -> dict[str, list]:
    here = ROOT / "examples" / w["name"]
    if "states" in w:
        return {s: [here / f for f in fs] for s, fs in w["states"].items()}
    out = {p.stem: [p] for p in sorted((here / "changes").glob("*.toml"))}
    return out | {name: [name] for name in w.get("inline", ())}  # `--change <name>`: an inline change


def build(root: Path, env: dict, folder: str, wf: Path, changes: list = (), against: bool = True,
          store: str = "store") -> dict:
    d = root / folder
    diag = root / f"{folder}.diagnostics.json"
    cmd = [VIA, "build", wf, "--lib", LIB, "--store", root / store, "--out", d, "--diagnostics", diag]
    for c in changes:
        cmd += ["--change", c]
    if against:
        cmd += ["--against", root / "base"]
    code, text, wall, rss = timed(cmd, ROOT, env)
    r = jload(root / f"{folder}.run.json") or {}
    rows = (jload(diag) or {}).get("diagnostics", [])
    counts: dict[str, int] = {}
    for x in rows:
        if x["severity"] != "info":
            k = f"{x['code']} ({x['severity'][0]})"
            counts[k] = counts.get(k, 0) + 1
    return {"folder": folder, "status": code, "wall": wall, "rss": rss,
            "first": next((s for s in text.splitlines() if s.startswith("error")), ""),
            "totals": r.get("totals", {}), "codes": counts}


def measure5(w: dict, root: Path, folder: str, section: str, env: dict) -> tuple[int, list[str]]:
    args = [PY, ROOT / "examples" / w["name"] / "measure.py", root / folder]
    if w["margs"] == "surfaces" and section == "base":
        args += ["--surfaces", root / "sat.json"]
    if w["margs"] == "base" and section != "base":
        args += ["--base", root / "base"]
    code, out = run(args + ["--check", section], env=env | {"VIA": str(VIA), "VIA_LIB": str(LIB)})
    return code, [s for s in out.splitlines() if s.strip()]


def collect(tmp: Path) -> dict:
    env = {"VIA_CONFIG_DIR": str(tmp / "config"), "VIA_LIB": str(LIB)}
    (tmp / "config").mkdir(parents=True, exist_ok=True)
    code, out = run([VIA, "config", "program", "python", PY], env=env)
    if code:
        sys.exit(f"via config program python: {out}")
    _, load0 = run(["sysctl", "-n", "vm.loadavg"])
    data = {"worlds": {}, "load_start": load0.strip().strip("{} ")}
    for w in WORLDS:
        saved = tmp / f"{w['name']}.json"
        if saved.exists() and "--fresh" not in sys.argv:
            data["worlds"][w["name"]] = json.loads(saved.read_text())
            continue
        print(f"{w['name']} ...", flush=True)
        here = ROOT / "examples" / w["name"]
        wf = here / "world.toml"
        chs = changes_of(w)
        for target, extra in [(wf, [])] + [(wf, [a for c in cs for a in ("--change", c)]) for cs in chs.values()] \
                + [(here / f, []) for f in w["alone"].values()]:
            code, out = run([VIA, "trust", target, "--lib", LIB, "--yes"] + extra, env=env)
            if code:
                sys.exit(f"via trust {target} {extra}: {out}")
        root = tmp / w["name"]
        if root.exists():
            shutil.rmtree(root)
        root.mkdir(parents=True)
        alone = {pid: build(root, env, f"alone_{pid}", here / f, against=False) for pid, f in w["alone"].items()}
        cold = build(root, env, "base", wf, against=False)
        if cold["status"]:
            sys.exit(f"{w['name']} base build failed:\n{cold['first']}")
        warm = build(root, env, "unchanged", wf)
        if "points" in w:
            run([VIA, "query", "world.toml", "surfaces_at", "--in", w["points"], "--out", root / "sat.json"],
                cwd=here, env=env)
        if w["kind"] == "m5":
            ms = {"base": measure5(w, root, "base", w.get("base_section", "base"), env)}
        else:
            ms = {"base": m4_measure(w, root, "base", "base")}
        ch = {}
        for c, cs in chs.items():
            print(f"  {c}", flush=True)
            ch[c] = build(root, env, c, wf, cs, store="store_move" if c in w.get("own_store", ()) else "store")
            ms[c] = measure5(w, root, c, c, env) if w["kind"] == "m5" else m4_measure(w, root, c, c)
        g = [0, ""]
        if w["key"]:
            sheet = [ROOT / "engines/godot/sheet.sh", "--before", root / "base", "--after", root / w["key"],
                     "--out", root / "review"]
            genv = {"SHEET_TIMEOUT": "900"}
            g = list(run(sheet + ["--option", "water=tag:water_level"], env=genv))
            g.append("water=tag:water_level")
            if "selects 0 scalar" in g[1]:  # a world with no water_level field: render without the option
                shutil.rmtree(root / "review", ignore_errors=True)
                g = list(run(sheet, env=genv)) + ["(none: no water_level field)"]
            g[1] = g[1][-600:]
        m = jload(root / "base/manifest.json") or {}
        data["worlds"][w["name"]] = {
            "alone": alone, "cold": cold, "warm": warm, "changes": ch, "measures": ms, "godot": g,
            "frame": m.get("frame"), "instances": (m.get("instances") or {}).get("count"),
        }
        saved.write_text(json.dumps(data["worlds"][w["name"]], indent=1))
        # Keep only the renders and their sheet.json: builds and stores go.
        for p in root.iterdir():
            if p.is_dir() and p.name != "review":
                shutil.rmtree(p)
    _, load1 = run(["sysctl", "-n", "vm.loadavg"])
    data["load_end"] = load1.strip().strip("{} ")
    data["godot_version"] = (jload(tmp / "station/review/sheet.json") or {}).get("godot", "?")
    return data


# ------------------------------------------------------------------------------------------- draw
def val(data: dict, world: str, sec: str, key: str) -> str:
    """One measure.py value: `sec.key = v  (...)` (M5 worlds) or `sec.name: key = v, band ...` (M3/M4)."""
    _, lines = data["worlds"].get(world, {}).get("measures", {}).get(sec, (1, []))
    for s in lines:
        m = re.search(rf"[.:]\s?{re.escape(key)} = (.+?)(?:, band|, checked|\s+\(|$)", s)
        if m and s.startswith(("ok", "FAIL", "BAD")):
            return m.group(1).strip("'")
    return "?"


def passing(data: dict, world: str, sec: str | None = None) -> tuple[int, int]:
    ms = data["worlds"].get(world, {}).get("measures", {})
    lines = [s for k, (_, ls) in ms.items() if sec in (None, k) for s in ls if s.startswith(("ok", "FAIL", "BAD"))]
    return sum(s.startswith("ok") for s in lines), len(lines)  # border prints BAD, the rest FAIL


def walks(tmp: Path, world: str) -> dict:
    """review/sheet.json's walks by bookmark: {name: {"before": {...}, "after": {...}}}."""
    d = jload(tmp / world / "review/sheet.json") or {}
    return {w["name"]: w for w in d.get("walks", [])}


def walk_text(tmp: Path, world: str, bm: str, side: str) -> str:
    w = walks(tmp, world).get(bm, {}).get(side)
    if not w:
        return "no walk"
    on = w.get("on") or {}
    by = f" by {w['by']}" if w.get("by") and w["outcome"] != "arrived" else ""
    return f"{w['outcome']} {w['walked']:.1f} m, {on.get('from', '?')} -> {on.get('stop', '?')}{by}"


def walk_count(tmp: Path, world: str, side: str) -> tuple[int, int, list[str]]:
    ws = walks(tmp, world)
    rows = [(n, w.get(side) or {}) for n, w in ws.items()]
    bad = [f"{n} {x.get('outcome', '?')}" for n, x in rows if x.get("outcome") != "arrived"]
    return len(rows) - len(bad), len(rows), bad


# Views on the sheet: (world, side, file under review/<side>/, caption). Every one is rendered by
# step 6 above into <keep>/<world>/review/; nothing is copied from another folder.
VIEWS = {
    "kit": [
        ("monastery", "before", "bm_gorge.png", "monastery bm_gorge: sheer gorge faces, eye 1.7 m"),
        ("monastery", "before", "bm_deck.png", "monastery bm_deck: the bridge deck over the gorge"),
        ("city", "before", "ix_central_freeway.png", "city ix_central_freeway: the freeway on its deck"),
        ("city", "before", "walk/ix_central_freeway_plan.png", "its walk up the ramp loop onto the freeway (plan)"),
        ("townland", "before", "walk/way_bridge_plan.png", "townland way_bridge: from the way onto the deck (plan)"),
        ("quay", "before", "corner_houses.png", "quay corner_houses: corner houses at a block's corner"),
        ("quay", "before", "wedge_house.png", "quay wedge_house: shallow houses in a wedge"),
        ("quay", "after", "wedge_house.png", "quay wedge_house after move_hill (a wedge of that build)"),
    ],
    "college": [
        ("college", "before", "bm_pavilion.png", "college bm_pavilion, base"),
        ("college", "after", "bm_pavilion.png", "college bm_pavilion after move_and_turn"),
        ("college", "before", "walk/bm_pavilion_plan.png", "walk to the pavilion's door, base (plan)"),
        ("college", "after", "walk/bm_pavilion_plan.png", "the same walk after move_and_turn (plan)"),
    ],
    "station": [
        ("station", "before", "at_front_door.png", "station at_front_door: ground to the cottage floor"),
        ("station", "before", "walk/at_front_door_plan.png", "its walk through the front door (plan)"),
        ("station", "before", "in_lantern.png", "station in_lantern: the top floor, the lens"),
        ("station", "before", "walk/loft_top_eye.png", "loft_top: down the laid flight (walk eye)"),
        ("station", "before", "walk/up_stair_3_plan.png", "up_stair_3: the spiral flight, tower_2 to 3 (plan)"),
        ("station", "after", "walk/up_stair_2_plan.png", "up_stair_2 after move_stair (plan)"),
        ("station", "before", "in_watch.png", "station in_watch: tower_2's floor and desk"),
        ("station", "after", "in_living.png", "in_living after move_stair (table moved with it)"),
    ],
    "object": [
        ("pistol", "before", "bm_left.png", "pistol at rest"),
        ("pistol", "after", "bm_left.png", "pistol racked, optic, suppressor and light"),
        ("sidearm", "before", "bm_left.png", "sidearm at rest"),
        ("sidearm", "after", "bm_left.png", "sidearm racked"),
    ],
    "coast": [
        ("coast", "before", "in_cave.png", "coast in_cave: on the cave's floor, out to sea"),
        ("coast", "before", "under_deck.png", "coast under_deck: the cliff, the cave, the deck"),
        ("coast", "before", "on_bridge.png", "coast on_bridge: walked onto the deck"),
        ("coast", "after", "at_cave_mouth.png", "coast at_cave_mouth after move_cave"),
    ],
    "market": [
        ("market", "before", "arrive.png", "market arrive: the main street, box kit"),
        ("market", "after", "arrive.png", "market arrive after module (bay 5 m -> other module)"),
        ("market", "before", "square.png", "market square"),
        ("market", "after", "top.png", "market top after module"),
    ],
}

# The rulings M5 added to docs/doctrine.md section 5 (R37 to R43, all 2026-11-17), one line each
# by hand; the decisions taken at the M5 close (fix pass), open to the owner.
RULINGS = [
    "R37 No volume: a floor, a deck, a stair's tread and a cave's floor are each a kind surface; terrain under them is cut from export, never",
    "    from the field. path3 is dropped (a stair, ramp or cave is a path whose kind owns a surface). Open: a surface formula may not read a field.",
    "R38 A meeting is exact in v1 (no allowed step, no threshold); `connector` is a free word compared for equality; a kind surface is a place,",
    "    so a door's or stair's end that names it meets it at its own point.",
    "R39 Which surface a thing stands on: `surface = \"<feature>.<surface>\"` on at-placements, sets, points, scatters; a point `on` a thing stands",
    "    where an instance would. Owner first, three items open (off-bound points, a step's own instances untested).",
    "R40 Clash only on one level (terrain and a floor with `cut` are one level); parts reaching one root are one thing; a library lowering",
    "    keeps its own neighbours apart. Owed: a raised ground floor still clashes.",
    "R41 `passable` is a core value only walks read; what a walk library may assume (one surface at a time, change only at a meeting, cut",
    "    quads and solid footprints are no ground); `along` names a route as loader text (open to the owner: a checked name).",
    "R42 Things with parts, states and accessories: nested sets and parts `on` mounts; a part's height is its asset's negative `sink` (open);",
    "    an accessory is a change file on a named socket; a state is read from the build, never a flag. No holes, tilts or constraints in v1.",
    "R43 A step's document names its own keys first, so one step lays a building's storeys, flights, walls and furniture; numbers come as",
    "    f64, lists as canonical JSON; surfaces_at and sample refuse what lies above the step.",
    "Decided at the M5 close, open to the owner (fix pass): a target that is an instance takes its surface's height and a path end its",
    "    `ends_at` floor's (surfaces.md 4.1); the skirt covers only its own kind's cut cells (build.md 10); the kit's `ends` is renamed",
    "    `caps`; `via query` runs no step (spec changed to the code); walk arrival is never in the air (build.md 15.1); recommended in",
    "    doctrine 6: stations along a stretch by distance from one end, with identity/c27.",
]

# Written by hand from TODO.md "From M5 close" (the M5 close reports, 2026-11-17) and this sheet's
# views; the computed lines (FAILs, non-zero exits, walks not arriving) come first in still_wrong().
LEFT_M6 = [
    "To M6 (TODO \"From M5 close\"):",
    "- `via query` runs no step: a step runner for surfaces_at, sample and meetings over a laid building; `relief` over a kind surface.",
    "- `surface.gap` names what sets each side only by name: the effect that wins the terrain at the end is not named (field provenance owed).",
    "- `lay` returns no place a bookmark can target (a storey, a stair's foot); decide with the produced-name rule (R3).",
    "- A reversed `along` (townland ways_meet stops at the bank); city on_central_bridge and on_harbor_bridge now say `fell` off their decks.",
    "- Parapets on monastery's, city's and townland's decks (only coast runs lib/ways parapet), or each README says why not.",
    "- quay's wedge_house in move_hill is placed by coordinates: a view that follows a produced wedge needs a name for it.",
    "- A lib/building `check` lint for furniture on a flight's line (station's in_flight is a world measure only).",
    "To M7: the loader draws the skirt's cut quads clipped to the strip (monastery bm_tarn fringe).",
    "M6 itself (docs/plan.md): every example joined in one map (atlas); chunks load and unload on a walk; an over-budget chunk fails and",
    "    names the asset; the player map as data, per floor (challenges 4, 23, 24, 42, 53).",
]

STILL_WRONG_BY_EYE = [
    "county and border (M4 worlds) were not rebuilt by the M5 close fix pass; their FAILs above are this run's and are not investigated here.",
    "Seen on this sheet's renders and in the M5 close reviews (known, in TODO \"From M5 close\" Later):",
    "- station close_passage: the sheet still walks through the closed passage (a leaf, not a meeting taken away); `outside` stops at the",
    "  kitchen table. coast cliff_from_sea is blocked by the west face in every build; sky shows through the cave mouth's slot.",
    "- monastery: the deck has no parapet, so bm_deck walks off it (`fell`, above); the tarn's rock fringe is saw-toothed seen from above.",
    "- college's blocks are stand-in boxes with no visible door at *.entry (the walks arrive at the door's point all the same).",
    "- Cost: in city, quay and townland a change costs about as much as a cold build (whole-world houses steps; TODO Later: Performance).",
]


def still_wrong(tmp: Path, data: dict) -> list[str]:
    out = []
    for w in WORLDS:
        d = data["worlds"].get(w["name"])
        if not d:
            continue
        for sec, (code, ls) in d["measures"].items():
            out += [f"{w['name']}: {s[:200]}" for s in ls if s.startswith(("FAIL", "BAD"))]
            if code and not any(s.startswith(("FAIL", "BAD")) for s in ls):
                out.append(f"{w['name']} {sec}: measure.py exit {code}: {(ls or [''])[-1][:160]}")
        for name, b in list(d["changes"].items()) + [("base", d["cold"]), ("unchanged", d["warm"])]:
            if b["status"] and not (name == "forgotten_exit" and b["status"] == 1):  # stops by design (M4)
                out.append(f"{w['name']} {name}: build exit {b['status']}: {b.get('first', '')[:170]}")
        if d["godot"][0]:
            out.append(f"{w['name']}: sheet.sh exit {d['godot'][0]}")
        if w["key"] and w["name"] not in ("pistol", "sidearm"):
            for side in ("before", "after"):
                n, t, bad = walk_count(tmp, w["name"], side)
                if bad:
                    out.append(f"{w['name']} walks {'base' if side == 'before' else w['key']}: {n} of {t} arrive; "
                               f"not: {', '.join(bad)}")
    return out + STILL_WRONG_BY_EYE


def acceptance(tmp: Path, data: dict) -> dict[str, list[str]]:
    """Each M5 acceptance line's numbers, read from this run's measures and walks."""
    v = lambda w, s, k: val(data, w, s, k)  # noqa: E731
    pw = lambda w, s=None: "{}/{} rows".format(*passing(data, w, s))  # noqa: E731
    st_b, st_t, _ = walk_count(tmp, "station", "before")
    st_a, st_ta, _ = walk_count(tmp, "station", "after")
    co_b, co_t, _ = walk_count(tmp, "college", "before")
    co_a, co_ta, _ = walk_count(tmp, "college", "after")
    pub = {}
    try:
        import tomllib
        pub = tomllib.loads((ROOT / "examples/pistol/published.toml").read_text())
    except Exception:  # noqa: BLE001 - the sheet still draws; the line says "?"
        pass
    p = pub.get("published", {})
    travel = pub.get("state", {}).get("racked", {}).get("travel", "?")
    states = [s for s in data["worlds"].get("pistol", {}).get("measures", {})]
    states_ok = sum(1 for s in states if passing(data, "pistol", s)[0] == passing(data, "pistol", s)[1] > 0)
    return {
        "kit": [
            f"Sheer faces (monastery base, {pw('monastery', 'base')}): faces {v('monastery', 'base', 'faces')}, face_gap_max "
            f"{v('monastery', 'base', 'face_gap_max_mm')} mm, quads cut under the faces {v('monastery', 'base', 'face_cut')}, gorge "
            f"step max {v('monastery', 'base', 'gorge_step_max_m')} m; coast: steep share {v('coast', 'base', 'face_steep_share')}, "
            f"sliver {v('coast', 'base', 'sliver_m2_per_m')} m2/m, brink dev {v('coast', 'base', 'brink_dev_mm')} mm.",
            f"Levels and decks: city (base {pw('city', 'base')}, move_bridge {pw('city', 'move_bridge')}) decks {v('city', 'base', 'decks')}, "
            f"doubled {v('city', 'base', 'doubled_decks')}, level meetings "
            f"{v('city', 'base', 'level_meetings')} at gap max {v('city', 'base', 'level_gap_max_mm')} mm, viaducts {v('city', 'base', 'viaducts')}, "
            f"headroom min {v('city', 'base', 'headroom_min_m')} m, viaduct grade max {v('city', 'base', 'viaduct_grade_max')} %; townland (base {pw('townland', 'base')}, "
            f"move_crossing {pw('townland', 'move_crossing')}) decks {v('townland', 'base', 'decks')}, doubled "
            f"{v('townland', 'base', 'doubled_decks')}; monastery decks {v('monastery', 'base', 'decks')}. Walks: city ix_central_freeway "
            f"{walk_text(tmp, 'city', 'ix_central_freeway', 'before')}; townland way_bridge {walk_text(tmp, 'townland', 'way_bridge', 'before')}.",
            f"Corner and wedge houses (quay base {pw('quay', 'base')}, move_hill {pw('quay', 'move_hill')}): lots held by corner houses "
            f"{v('quay', 'base', 'corner_lots')}, by wedge (shallow) houses {v('quay', 'base', 'fallback_lots')}, lots held in all "
            f"{v('quay', 'base', 'lots_held')}, reach held {v('quay', 'base', 'reach_held')} of {v('quay', 'base', 'reach_pairs')} pairs.",
        ],
        "college": [
            f"bm_pavilion walk: base {walk_text(tmp, 'college', 'bm_pavilion', 'before')}; after move_and_turn "
            f"{walk_text(tmp, 'college', 'bm_pavilion', 'after')}. All walks arriving: base {co_b}/{co_t}, after {co_a}/{co_ta}.",
            f"move_and_turn ({pw('college', 'move_and_turn')}): instances moved {v('college', 'move_and_turn', 'instances_moved')}, "
            f"features moved {v('college', 'move_and_turn', 'features_moved')} (other {v('college', 'move_and_turn', 'features_moved_other')}); "
            f"doors met {v('college', 'move_and_turn', 'doors_met')}, unmet {v('college', 'move_and_turn', 'surface.unmet')}, door turn max "
            f"{v('college', 'base', 'door_turn_max')} deg before, {v('college', 'move_and_turn', 'door_turn_max')} after; walk_end_off "
            f"{v('college', 'move_and_turn', 'walk_end_off')}; meetings kept {v('college', 'move_and_turn', 'meets_kept')}; reach held "
            f"{v('college', 'move_and_turn', 'reach_held')}.",
            f"no_walk ({pw('college', 'no_walk')}): a door with no walk is surface.unmet {v('college', 'no_walk', 'surface.unmet')} with a hint "
            f"that says what to write ({v('college', 'no_walk', 'unmet_hint_writes')}).",
        ],
        "station": [
            f"Walks (sheet.sh, spec/build.md 15.1): base {st_b} of {st_t} arrive, move_stair {st_a} of {st_ta}. at_front_door "
            f"{walk_text(tmp, 'station', 'at_front_door', 'before')}; at_back_door {walk_text(tmp, 'station', 'at_back_door', 'before')}; "
            f"down_step {walk_text(tmp, 'station', 'down_step', 'before')}.",
            f"Up the flights: up_stair_1 {walk_text(tmp, 'station', 'up_stair_1', 'before')}; up_stair_3 "
            f"{walk_text(tmp, 'station', 'up_stair_3', 'before')}; loft_top {walk_text(tmp, 'station', 'loft_top', 'before')}.",
            f"base ({pw('station', 'base')}): storeys {v('station', 'base', 'storeys')}, rooms {v('station', 'base', 'rooms')}, flights "
            f"{v('station', 'base', 'flights')}, doors in {v('station', 'base', 'doors_in')}, meetings {v('station', 'base', 'meetings')} at gap "
            f"max {v('station', 'base', 'gap_max_mm')} mm, floors joined {v('station', 'base', 'floors_joined')}, unreached "
            f"{v('station', 'base', 'unreached')}, door clear min {v('station', 'base', 'clear_min_mm')} mm, dressing "
            f"{v('station', 'base', 'dressing')}, in_flight {v('station', 'base', 'in_flight')}; move_stair ({pw('station', 'move_stair')}): "
            f"in_flight {v('station', 'move_stair', 'in_flight')}; close_passage ({pw('station', 'close_passage')}): unreached "
            f"{v('station', 'close_passage', 'unreached')} (every room past the passage).",
        ],
        "object": [
            f"Published (examples/pistol/published.toml, mm): length {p.get('overall_length', '?')}, width {p.get('width', '?')}, height "
            f"{p.get('height', '?')}, sight radius {p.get('sight_radius', '?')}; racked travel {travel}. Built: rest length "
            f"{v('pistol', 'base', 'length')}, width {v('pistol', 'base', 'width')}, height {v('pistol', 'base', 'height')}, sight radius "
            f"{v('pistol', 'base', 'sight_radius')}, off_length/width/height {v('pistol', 'base', 'off_length')}/"
            f"{v('pistol', 'base', 'off_width')}/{v('pistol', 'base', 'off_height')}.",
            f"Racked with all three: state read from the build {v('pistol', 'racked+optic+suppressor+light', 'state')}, length "
            f"{v('pistol', 'racked+optic+suppressor+light', 'length')} (219 + travel 40 - tang 12), fitted "
            f"{v('pistol', 'racked+optic+suppressor+light', 'fitted')}, added length "
            f"{v('pistol', 'racked+optic+suppressor+light', 'added_length')}, added height "
            f"{v('pistol', 'racked+optic+suppressor+light', 'added_height')}. {states_ok} of {len(states)} states pass every row "
            f"({pw('pistol')} in all).",
            f"sidearm (its own numbers, examples/sidearm/numbers.toml): rest {pw('sidearm', 'base')}, racked {pw('sidearm', 'racked')}.",
        ],
        "coast": [
            f"base ({pw('coast', 'base')}, --surfaces from via query surfaces_at): surfaces at the cave's middle "
            f"'{v('coast', 'base', 'cave_surface_names')}', meetings {v('coast', 'base', 'meetings')} at gap max "
            f"{v('coast', 'base', 'gap_max_mm')} mm, mouth open {v('coast', 'base', 'mouth_open')}, roof min {v('coast', 'base', 'roof_min_m')} m, "
            f"cave reached {v('coast', 'base', 'cave_reached')}, deck {v('coast', 'base', 'deck_length_m')} m, freeboard "
            f"{v('coast', 'base', 'freeboard_m')} m, parapet posts {v('coast', 'base', 'parapet_posts')}.",
            f"move_cave ({pw('coast', 'move_cave')}): mouth {v('coast', 'base', 'mouth_at')} -> {v('coast', 'move_cave', 'mouth_at')}, jambs met "
            f"{v('coast', 'move_cave', 'jambs_met')}, roof min {v('coast', 'move_cave', 'roof_min_m')} m; long_deck ({pw('coast', 'long_deck')}): "
            f"deck {v('coast', 'long_deck', 'deck_length_m')} m, parapet posts {v('coast', 'long_deck', 'parapet_posts')}. in_cave "
            f"{walk_text(tmp, 'coast', 'in_cave', 'before')}; on_bridge {walk_text(tmp, 'coast', 'on_bridge', 'before')}.",
        ],
        "market": [
            f"base ({pw('market', 'base')}): pieces {v('market', 'base', 'pieces')} (bays {v('market', 'base', 'bay')}, fillers "
            f"{v('market', 'base', 'filler')}, ends {v('market', 'base', 'end')}, corners {v('market', 'base', 'corner')}), kit gaps "
            f"{v('market', 'base', 'kit_gaps')}, open joints {v('market', 'base', 'kit_open')}, overlaps {v('market', 'base', 'place.overlap')}, "
            f"doors level {v('market', 'base', 'doors_level')}, doors off {v('market', 'base', 'doors_off')}.",
            f"module ({pw('market', 'module')}): pieces {v('market', 'module', 'pieces')}, revised by name {v('market', 'module', 'revised_named')}; "
            f"move_street ({pw('market', 'move_street')}): main moved {v('market', 'move_street', 'moved_main')}, other kept "
            f"{v('market', 'move_street', 'kept_other')}, other moved {v('market', 'move_street', 'moved_other')}; remodel "
            f"({pw('market', 'remodel')}); move_bend ({pw('market', 'move_bend')}).",
        ],
    }


def challenge_status(rows: tuple[str, ...]) -> list[str]:
    """docs/challenges.md's status for M5's rows, read from the file."""
    out = []
    for s in (ROOT / "docs/challenges.md").read_text().splitlines():
        c = [x.strip() for x in s.split("|")]
        if len(c) > 5 and c[1] in rows and c[4] in ("Held", "Partly", "Not held", "Open"):
            if not any(o.startswith(f"{c[1]} ") for o in out):
                out.append(f"{c[1]} {c[4]}")
    return out


def draw(tmp: Path, data: dict) -> None:
    _, rustc = run(["rustc", "--version"])
    _, head = run(["git", "log", "-1", "--format=%h %ad", "--date=short"])
    _, porcelain = run(["git", "status", "--porcelain"])
    _, cpu = run(["sysctl", "-n", "machdep.cpu.brand_string"])
    machine = f"{platform.system()} {platform.release()} {platform.machine()}, {cpu.strip()}"
    dirty = len(porcelain.splitlines())

    W, H, DPI = 2400, 12000, 100
    fig = plt.figure(figsize=(W / DPI, H / DPI), dpi=DPI, facecolor="white")
    BODY, HEAD, MONO, LH = 10, 14, "DejaVu Sans Mono", 17
    BAD, GREY, OK = "#b03a00", "#444444", "black"

    def axes(x, y, w, h):
        return fig.add_axes([x / W, 1 - (y + h) / H, w / W, h / H])

    def text(x, y, s, size=BODY, mono=True, bold=False, colour="black", ha="left", box=None):
        fig.text(x / W, 1 - y / H, s, fontsize=size, family=MONO if mono else None, va="top", ha=ha,
                 weight="bold" if bold else "normal", color=colour, zorder=5,
                 bbox=dict(facecolor=box, edgecolor="none", pad=2, alpha=0.75) if box else None)

    class Col:
        def __init__(self, x: int, y: int, width: int = 270):
            self.x, self.y, self.width = x, y, width

        def head(self, s: str, size=HEAD) -> None:
            self.y += 10
            text(self.x, self.y, s, size=size, mono=False, bold=True)
            self.y += 28

        def line(self, s: str, bold: bool = False, colour: str = "black") -> None:
            for part in textwrap.wrap(s, self.width, subsequent_indent="    ", drop_whitespace=False) or [""]:
                text(self.x, self.y, part, bold=bold, colour=colour)
                self.y += LH

    VW = (W - 80 - 3 * 12) // 4
    VH = int(VW * 9 / 16)

    def views(y: int, items: list) -> int:
        for i, (world, side, f, cap) in enumerate(items):
            x, yy = 40 + (i % 4) * (VW + 12), y + (i // 4) * (VH + 12)
            a = axes(x, yy, VW, VH)
            p = tmp / world / "review" / side / f
            if p.exists():
                a.imshow(mpimg.imread(p))
            else:
                a.text(0.5, 0.5, f"not rendered: {world}/review/{side}/{f}", ha="center", va="center",
                       transform=a.transAxes, color=BAD)
            a.set_xticks([])
            a.set_yticks([])
            text(x + 5, yy + 5, cap, size=10, mono=False, colour="white", box="black")
            text(x + VW - 5, yy + VH - 22, f"{world}/review/{side}/{f}", size=8, colour="white", box="black", ha="right")
        return y + ((len(items) + 3) // 4) * (VH + 12)

    state = f"working tree: {dirty} uncommitted paths over it" if dirty else "working tree clean"
    text(40, 22, f"via M5 sheet: more than one surface; buildings as a library   branch rewrite, last commit "
         f"{head.strip()}; {state}   made {datetime.date.today().isoformat()}", size=16, mono=False, bold=True)
    text(40, 56, f"Made by docs/reports/make_m5_sheet.py on {machine}; {rustc.strip()}; Godot "
         f"{data.get('godot_version', '?')}. Every number is read from a build, a measure or a walk the script ran; the builds "
         f"are of the working tree, not of the commit.", size=11, mono=False, colour=GREY)
    text(40, 78, "Every view: engines/godot/sheet.sh --before <base> --after <key change> (1280x720, eye 1.7 m; walks per "
         "spec/build.md 15.1), run by this script; its file is named at its lower right, under <keep>/<world>/review/.\n"
         "Each world built in one fresh store (base cold, unchanged, every change --against base) and checked by its "
         "measure.py --check against expect.toml (ok / FAIL).", size=11, mono=False, colour=GREY)

    acc = acceptance(tmp, data)
    c = Col(40, 130)
    c.head("M5 (docs/plan.md): \"More than one surface; buildings as a library\". Worlds coast, station, college, market, "
           "pistol, sidearm.")
    c.line("Acceptance: the town kit's three owed changes; a building moved and turned keeps its walk; indoors reachable "
           "through a door; the measured object matches its published numbers.", bold=True)
    c.line("Challenge rows (docs/challenges.md, status as written there): " + ", ".join(
        challenge_status(("7", "8", "14", "15", "18", "20", "33", "46", "47", "48"))) + ".")
    y = c.y + 6

    sections = [
        ("kit", "1. The town kit's three owed changes: sheer faces, levels (freeway and decks), corner and wedge houses"),
        ("college", "2. A building moved and turned keeps its walk (college, move_and_turn: the pavilion moved and turned)"),
        ("station", "3. Indoors reachable through a door (station: a cottage, a passage, a four-storey tower, a laid net loft)"),
        ("object", "4. The measured object matches its published numbers (pistol, nine states; sidearm, two)"),
        ("coast", "M5 world coast: a sea cliff, a cave whose floor is a kind surface, a deck over water"),
        ("market", "M5 world market: a market street from a four-piece box kit, and its three changes"),
    ]
    for key, title in sections:
        text(40, y + 6, title, size=15, mono=False, bold=True)
        y = views(y + 36, VIEWS[key])
        c = Col(40, y + 2)
        for s in acc[key]:
            c.line(s)
        y = c.y + 14

    c = Col(40, y)
    c.head("Worlds: rows passing and build times (wall seconds, /usr/bin/time -l; one build each)")
    c.line(f"{'world':<10}{'rows':>9}{'cold s':>8}{'GB':>6}{'unchanged s':>13}{'%':>5}  changes --against base: wall s "
           f"(% of cold) rows; exit when not 0", bold=True)
    for w in WORLDS:
        d = data["worlds"].get(w["name"])
        if not d:
            c.line(f"{w['name']:<10} not built", colour=BAD)
            continue
        cw = d["cold"]["wall"]
        ok, n = passing(data, w["name"])
        ch = ", ".join(f"{name} {b['wall']:.1f} ({b['wall'] / cw:.0%}) {'{}/{}'.format(*passing(data, w['name'], name))}"
                       + (f" exit {b['status']}" + (" (by design)" if name == "forgotten_exit" else "") if b["status"] else "")
                       for name, b in d["changes"].items())
        al = "".join(f"; {p} alone {b['wall']:.1f}" for p, b in d["alone"].items())
        c.line(f"{w['name']:<10}{f'{ok}/{n}':>9}{cw:>8.1f}{d['cold']['rss']:>6.2f}{d['warm']['wall']:>13.1f}"
               f"{d['warm']['wall'] / cw:>5.0%}  base {'{}/{}'.format(*passing(data, w['name'], 'base'))}; {ch}{al}",
               colour=BAD if ok < n else OK)
    c.line("Protocol: release via from the working tree, default threads, VIA_LIB = the repo's lib, python = the SDK venv "
           "(3.12); one fresh store per world (market's move_street and move_bend on a second store, as its README says); "
           "order: parts alone, base cold, unchanged (the base again --against base), each change --against base. "
           "pistol's states are the change lists in examples/pistol/README.md. Laptop shared with other agents: load "
           f"average {data.get('load_start', '?')} at the start, {data.get('load_end', '?')} at the end. One run each: "
           "no spread is claimed; sub-second walls are noise. Rows are measure.py lines (ok + FAIL; border prints BAD). "
           "The run was resumed twice after fixes to this script (sheet.sh without the water option for a world with no "
           "water_level; the inline changes); each world was built, measured and rendered whole within one pass.", colour=GREY)
    c.head("Rulings added in M5 (docs/doctrine.md section 5, R37 to R43, 2026-11-17)")
    for s in RULINGS:
        c.line(s)
    c.head("Still wrong")
    for s in still_wrong(tmp, data):
        c.line(s, colour=BAD)
    c.head("Left to M6")
    for s in LEFT_M6:
        c.line(s)
    bottom = c.y + 30

    fig.canvas.draw()
    img = np.asarray(fig.canvas.buffer_rgba())[: int(bottom)]
    plt.imsave(OUT, img)
    print(f"wrote {OUT}  ({W}x{int(bottom)}; renders in {tmp})")


def main() -> None:
    keep = sys.argv[sys.argv.index("--keep") + 1] if "--keep" in sys.argv else None
    tmp = Path(keep) if keep else Path(tempfile.mkdtemp(prefix="via-m5-sheet-"))
    if "--draw-only" in sys.argv:
        data = jload(tmp / "data.json") or {
            "worlds": {w["name"]: jload(tmp / f"{w['name']}.json") for w in WORLDS if (tmp / f"{w['name']}.json").exists()}}
        data.setdefault("godot_version", (jload(tmp / "station/review/sheet.json") or {}).get("godot", "?"))
    else:
        tmp.mkdir(parents=True, exist_ok=True)
        if "VIA" not in os.environ:
            print("building via ...", flush=True)
            code, out = run(["cargo", "build", "--release", "-q", "-p", "via-cli"])
            if code:
                sys.exit(out)
        data = collect(tmp)
        (tmp / "data.json").write_text(json.dumps(data, indent=1))
    draw(tmp, data)


if __name__ == "__main__":
    main()
