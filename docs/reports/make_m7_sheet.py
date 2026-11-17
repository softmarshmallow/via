# /// script
# requires-python = ">=3.11"
# dependencies = ["matplotlib>=3.9,<3.10", "numpy>=2.0,<3", "pillow>=10,<12"]
# ///
"""Draw the M7 sheet, docs/reports/m7-sheet-<n>.png pages joined into docs/reports/m7-sheet.png, from a fresh atlas build.

    uv run docs/reports/make_m7_sheet.py --outsiders DIR [--keep DIR] [--draw-only] [--collect-only] [--fresh]

What it runs, in order (every number on the sheet that is not marked as hand-written comes from one of these):

1. The release `via` and `via-terrain` the VIA and VIA_TERRAIN environment variables name, else
   `cargo build --release -p via-cli -p via-terrain` into CARGO_TARGET_DIR (default the repo's target).
2. A scratch user config (VIA_CONFIG_DIR): `python` = the SDK's venv Python, `via-terrain` = the release
   binary; atlas trusted for its base and each change.
3. atlas, in one fresh store: the base cold; the base again into the same directory (unchanged); the
   base into a sibling folder (`sibling/`, same depth); the base into a folder two levels deeper
   (`deep/x/base`, another depth); then move_market, move_square, forgotten_exit and over_budget, each
   `--against` the base, in that order. Each build is timed by /usr/bin/time -l (wall seconds, peak
   resident memory), with the load average before and after, and read for its run report and diagnostics.
   Each build's own run report is copied to atlas/<name>.report.json and its diagnostics written to
   atlas/<name>.diagnostics.json: the unchanged build writes into base/ again, so atlas/base.run.json is
   the unchanged build's, not the cold one's (data.json keeps each build's totals).
4. examples/atlas/measure.py `--check <section>` on the base and on each change (rows in or out of band,
   bands from examples/atlas/expect.toml).
5. tools/map_render.py on the base's player map (`--expect`, section `map`).
6. engines/godot/sheet.sh --before <atlas base> --stream 96 --frames 120 --option water=tag:water_level
   (1280x720, eye 1.7 m; walks per spec/build.md 15.1): the atlas views and the walk counts.
7. The before-and-after pair: engines/godot/sheet.sh --before <atlas base> --after <move_market>
   --option water=tag:water_level (whole, not streamed, no frame tour); the sheet shows join_market. If base/ or
   move_market/ is gone (a kept folder without its builds), both are built again first into pair/ with
   a store of their own, timed and recorded as the pair's builds, not as the table's.
8. The conformance runner (tools/conformance) on the release `via`, milestones 3, 4, 5, 6 and 7: every
   milestone not yet in data.json runs at once, side by side (each case in its own temporary folder),
   so a resumed run starts the milestones left as one group; each records its group and load.

The data stays in --keep DIR (data.json and the renders); --draw-only needs it. The store and builds
are not needed to draw and may be deleted.

Not run by this script, and said so on the sheet: the outsiders' views are their own runs' renders
(engines/godot/sheet.sh, run by each outsider in its scratch), read from the folder `--outsiders` names;
the outsider table (time to first build, trips by family and cause) is hand-copied from
docs/challenges.md row 51; rulings R52 to R63 are summarised by hand from docs/doctrine.md; "what does
not hold" is written by hand from TODO.md "From M7 slice 1" and "From M7 close", each with its number
as recorded there (the atlas numbers beside them are this run's).
"""

from __future__ import annotations

import datetime
import json
import os
import platform
import re
import subprocess
import sys
import tempfile
import textwrap
from pathlib import Path

import matplotlib

matplotlib.use("Agg")
import matplotlib.image as mpimg  # noqa: E402
import matplotlib.pyplot as plt  # noqa: E402
import numpy as np  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
TARGET = Path(os.environ.get("CARGO_TARGET_DIR", ROOT / "target"))
VIA = Path(os.environ.get("VIA", TARGET / "release/via"))
TERRAIN = Path(os.environ.get("VIA_TERRAIN", TARGET / "release/via-terrain"))
PY = ROOT / "sdk/python/.venv/bin/python"
LIB = ROOT / "lib"
OUT = ROOT / "docs/reports/m7-sheet.png"
ATLAS = ROOT / "examples/atlas"
CHANGES = ["move_market", "move_square", "forgotten_exit", "over_budget"]
FAILS_BY_DESIGN = {"forgotten_exit", "over_budget"}  # each stops with exit 1 (expect.toml)
RADIUS, FRAMES = 96, 120
MILESTONES = (3, 4, 5, 6, 7)
PAIR_CHANGE, PAIR_VIEW = "move_market", "join_market"  # the change drawn before and after, and the view shown
M6_WALKS = {"arrived": 87, "blocked": 33, "fell": 6, "of": 126}  # docs/reports/m6-sheet.png section 2, hand-copied


def run(cmd: list, cwd: Path = ROOT, env: dict | None = None) -> tuple[int, str]:
    p = subprocess.run([str(c) for c in cmd], cwd=cwd, env=(os.environ | env) if env else None,
                       capture_output=True, text=True)
    return p.returncode, p.stdout + p.stderr


def jload(p: Path):
    try:
        return json.loads(Path(p).read_text())
    except (OSError, ValueError):
        return None


def timed(cmd: list, cwd: Path, env: dict) -> tuple[int, str, float, float]:
    code, text = run(["/usr/bin/time", "-l"] + cmd, cwd=cwd, env=env)
    wall = re.search(r"([\d.]+) real", text)
    rss = re.search(r"(\d+)\s+maximum resident set size", text)
    return code, text, float(wall.group(1)) if wall else float("nan"), int(rss.group(1)) / 1e9 if rss else float("nan")


def loadavg() -> str:
    return run(["sysctl", "-n", "vm.loadavg"])[1].strip().strip("{} ")


def build(root: Path, env: dict, folder: str, wf: Path, change: str | None = None, name: str | None = None) -> dict:
    d = root / folder
    d.parent.mkdir(parents=True, exist_ok=True)
    name = name or folder
    diag = root / f"{name}.diagnostics.json"
    cmd = [VIA, "build", wf, "--lib", LIB, "--store", root / "store", "--out", d, "--diagnostics", diag]
    if change:
        cmd += ["--change", ATLAS / "changes" / f"{change}.toml", "--against", root / "base"]
    load0 = loadavg()
    code, text, wall, rss = timed(cmd, ROOT, env)
    report = root / f"{folder}.run.json"
    if report.exists():  # a later build into the same folder overwrites <folder>.run.json; keep this one's
        (root / f"{name}.report.json").write_bytes(report.read_bytes())
    r = jload(report) or {}
    rows = (jload(diag) or {}).get("diagnostics", [])
    errors = [x for x in rows if x["severity"] == "error"]
    return {"folder": folder, "status": code, "wall": wall, "rss": rss, "load": [load0, loadavg()],
            "totals": r.get("totals", {}), "errors": errors[:3],
            "warnings": sum(1 for x in rows if x["severity"] == "warning")}


def measure(root: Path, folder: str, section: str, env: dict, run_report: bool) -> dict:
    args = [PY, ATLAS / "measure.py", root / folder]
    if run_report:
        args += ["--run", root / f"{folder}.run.json"]
    code, out = run(args + ["--check", section], env=env)
    lines = [s for s in out.splitlines() if s.strip()]
    _, js = run(args, env=env)  # without --check: the measures themselves, as JSON
    try:
        values = json.loads(js[js.index("{"):])
    except ValueError:
        values = {}
    return {"code": code, "lines": lines, "values": values}


def map_render(build_dir: Path, out: Path, extra: list) -> dict:
    code, text = run([sys.executable, ROOT / "tools/map_render.py", build_dir, build_dir / "export/map",
                      "--out", out] + extra)
    values = {}
    for s in text.splitlines():
        if s.startswith("{"):
            try:
                v = json.loads(s)
            except ValueError:
                continue
            if "counts" in v and not values:
                values = v
    return {"code": code, "values": values,
            "text": "\n".join(s for s in text.splitlines() if not s.startswith("{"))[-3000:]}


def save(tmp: Path, data: dict) -> None:
    (tmp / "data.json").write_text(json.dumps(data, indent=1))


def collect(tmp: Path) -> dict:
    data = jload(tmp / "data.json") if "--fresh" not in sys.argv else None
    data = data or {"atlas": {}, "measures": {}, "maps": {}}
    env = {"VIA_CONFIG_DIR": str(tmp / "config"), "VIA_LIB": str(LIB), "TMPDIR": str(tmp / "tmp")}
    (tmp / "config").mkdir(parents=True, exist_ok=True)
    (tmp / "tmp").mkdir(parents=True, exist_ok=True)
    for prog, path in (("python", PY), ("via-terrain", TERRAIN)):
        code, out = run([VIA, "config", "program", prog, path], env=env)
        if code:
            sys.exit(f"via config program {prog}: {out}")
    wf = ATLAS / "world.toml"
    for extra in [[]] + [["--change", ATLAS / "changes" / f"{c}.toml"] for c in CHANGES]:
        code, out = run([VIA, "trust", wf, "--lib", LIB, "--yes"] + extra, env=env)
        if code:
            sys.exit(f"via trust atlas {extra}: {out}")
    a = tmp / "atlas"
    a.mkdir(exist_ok=True)
    data.setdefault("load_start", loadavg())
    ran = False
    steps = ([("base", "base", None), ("unchanged", "base", None), ("sibling", "sibling", None),
              ("depth", "deep/x/base", None)] + [(c, c, c) for c in CHANGES])
    for name, folder, change in steps:
        if name in data["atlas"]:
            continue
        print(f"atlas {name} ...", flush=True)
        data["atlas"][name] = build(a, env, folder, wf, change, name)
        ran = True
        print(f"  exit {data['atlas'][name]['status']}, {data['atlas'][name]['wall']:.1f} s", flush=True)
        save(tmp, data)
        if name in ("unchanged", "sibling", "depth"):
            continue
        data["measures"][name] = measure(a, folder, name, env, change is not None)
        save(tmp, data)
    if "base" not in data["maps"]:
        data["maps"]["base"] = map_render(a / "base", a / "map_base", ["--expect", ATLAS / "expect.toml"])
        save(tmp, data)
    if "godot" not in data:
        print("godot sheet ...", flush=True)
        load0 = loadavg()
        code, out = run([ROOT / "engines/godot/sheet.sh", "--before", a / "base", "--out", a / "review",
                         "--stream", RADIUS, "--frames", FRAMES, "--option", "water=tag:water_level"], env=env)
        data["godot"] = {"status": code, "tail": out[-2000:], "load": [load0, loadavg()]}
        save(tmp, data)
        ran = True
    if "pair" not in data:  # the before-and-after pair: base, then move_market --against it
        pa, builds = a, {}
        if not ((a / "base").is_dir() and (a / PAIR_CHANGE).is_dir()):
            pa = a / "pair"  # the table's builds are gone: build both again, apart from the table
            for name, change in (("base", None), (PAIR_CHANGE, PAIR_CHANGE)):
                print(f"pair {name} ...", flush=True)
                builds[name] = build(pa, env, name, wf, change)
                print(f"  exit {builds[name]['status']}, {builds[name]['wall']:.1f} s", flush=True)
        print("godot pair ...", flush=True)
        load0 = loadavg()
        code, out = run([ROOT / "engines/godot/sheet.sh", "--before", pa / "base", "--after", pa / PAIR_CHANGE,
                         "--out", a / "review_pair", "--option", "water=tag:water_level"], env=env)
        data["pair"] = {"status": code, "tail": out[-2000:], "load": [load0, loadavg()], "builds": builds,
                        "made": datetime.datetime.now().isoformat(timespec="minutes")}
        save(tmp, data)
    data.setdefault("conformance", {})
    todo = [m for m in MILESTONES if str(m) not in data["conformance"]]
    if todo:  # the milestones left run side by side (each case in its own temporary folder); the runner counts, not times
        ran = True
        print(f"conformance {todo} ...", flush=True)
        load0 = loadavg()
        procs = {m: subprocess.Popen(["uv", "run", "python", "-m", "via_conformance", "run", "--via", str(VIA), "--milestone",
                                      str(m)], cwd=ROOT / "tools/conformance", env=os.environ | env,
                                     stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True) for m in todo}
        for m, pr in procs.items():
            out, _ = pr.communicate()
            tail = [s for s in out.splitlines() if re.search(r"\d+ passed", s)]
            ls = out.splitlines()  # a failing case's head line, then its first detail line
            failed = [f"{s.split()[1]}: {ls[i + 1].strip() if i + 1 < len(ls) else ''}"
                      for i, s in enumerate(ls) if re.match(r"(fail|error)\s", s)]
            data["conformance"][str(m)] = {"code": pr.returncode, "summary": tail[-1] if tail else out[-300:],
                                           "failed": failed[:6], "load": load0, "group": todo}
            save(tmp, data)
    if ran or "load_end" not in data:  # the end of the table's run; the pair records its own loads
        data["load_end"] = loadavg()
    save(tmp, data)
    return data


# ---------------------------------------------------------------- hand-written (said so on the sheet)

# docs/challenges.md row 51 (M7 close), hand-copied. Minutes are the outsiders' own clocks.
OUTSIDERS = [
    # line, task, run, first build, built, trips, what the world is
    ("Fable", "mill", "1st", "9 min", "built (19 min of 90)", "a mill that follows its weir, views inside, a bridge, a hamlet"),
    ("Fable", "crossroads", "1st", "5 min", "built (10 min of 90)", "quay and market 410 m apart, a routed road, a grade lint"),
    ("Sonnet", "mill", "2nd", "7 min", "built (15 to 20 min)", "a mill 3 m past its weir, two storeys walked, a bridge, six houses"),
    ("Sonnet", "crossroads", "2nd", "4.5 min", "built (8 min)", "quay and college by a 570 m road, milestones, the player map"),
    ("Sonnet", "both", "1st", "none", "stopped at minute 3", "given the old tree (no spec/, lib/, examples/): the harness's trip"),
]
TRIPS_BY_CAUSE = [  # (cause, first run (Fable both + Sonnet's stop), second run (Sonnet both))
    ("documents", 11, 12),
    ("messages", 7, 4),
    ("engine faults", 2, 0),
    ("SDK or library faults", 0, 2),
    ("path fault (//private for an absolute import)", 0, 1),
    ("library, core or engine gaps", 8, 3),
    ("tools or harness", 3, 0),
]
TRIPS_NOTE = [
    "First run: 31 trips over all four agents; 1 blocker hit by all four (the worktree, outside via), 2 by both Fable agents "
    "(no venv step, no SDK page), the rest by one.",
    "Second run (Sonnet from rewrite): 22 trips, 12 in mill and 10 in crossroads; 1 blocker (lay beside the step that returns its "
    "building said only an info); 8 met by both families (the weir, unowned crossings, the upper-floor walk, trees round houses, an "
    "import's paths and warnings, the lift, knock-on errors, the sheet's options).",
    "Fixed from the first run (row 51): every document and message trip, the two engine faults (--out .. climbing, a place on a "
    "turned import's edge) and the loader's load.bad_file, each fault with a case that fails on the old binary.",
    "Fixed from the second run (row 51): lib/water ships weir (cases that fail at the old radii), the SDK's kindless feature (case "
    "and SDK unit test), kind = \"\"'s hint, a step's report on an import's feature (case), the loader's along (loader test), an "
    "absolute import shown climbing (unit test), for_each_empty's hint naming the layer, and lib/building, lib/review, lib/map, "
    "the SDK README, README and the Godot README; row 51 does not say every document trip of this run. Still open from it: formula.outside_field "
    "naming the library's line, via build --help, knock-on errors after a failed name, an import's warnings unfolded, lay beside "
    "its building's step quiet on the console, the weir drawing no mesh (section 8).",
    "Both lines are Claude models (Fable and Sonnet). A run by another vendor's model is the owner's to make; it is not on this sheet.",
]
# (folder under --outsiders, file under review/<run>/before, caption)
OUTSIDER_VIEWS = [
    ("m7c-fresh-a-fable/review/mill", "at_mill_door.png", "Fable mill: at_mill_door"),
    ("m7c-fresh-a-fable/review/mill", "mill_upper.png", "Fable mill: mill_upper (the upper floor)"),
    ("m7c-fresh-a-sonnet/review/w", "millsite#at_mill_door.png", "Sonnet mill: millsite#at_mill_door"),
    ("m7c-fresh-a-sonnet/review/w", "on_lane.png", "Sonnet mill: on_lane"),
    ("m7c-fresh-b-fable/review/base", "half_way.png", "Fable crossroads: half_way on the routed road"),
    ("m7c-fresh-b-fable/review/base", "at_market.png", "Fable crossroads: at_market"),
    ("m7c-fresh-b-sonnet/review/w", "at_college.png", "Sonnet crossroads: at_college"),
    ("m7c-fresh-b-sonnet/review/w", "half_way.png", "Sonnet crossroads: half_way"),
]

CUTOVER = [
    "via-artifact folded: its seed derivation moved into via-num as sub_seed; the crate dropped; via-terrain holds no second core "
    "(M7 slice 1, ac64d21).",
    "spike/ deleted from the tree (M7 close, dc04433); its record is the tag spike-final, its oracle data fixtures/oracle/. The "
    "oracle runs from a worktree pinned at spike-final; only exported data crosses. spike/local, spike/out and spike/build stay on "
    "disk, gitignored (whether they may go is the owner's).",
    "docs/archive dropped; docs/charter.md, docs/faq.md (34 open questions to 11, each naming who answers) and the ledger's engine "
    "rows checked (36 rows: 8 corrected, 10 confirmed, 1 in names only, 17 not checked) before any outsider read them.",
]

RULINGS = [
    "R52 a library step never opens its kinds.toml; it gets what the file says through the build (xtask check refuses it).",
    "R53 a narrow brought strip cuts as if half a parent quad's diagonal plus 6 mm wide, round at its ends (coast.along_cliff).",
    "R54 an untrusted step's committed program digest stands as recorded; staleness is a trusted via step commit's to find.",
    "R55 a build's leaves hold the build key less locations.pack_root, so a folder at another depth reuses every memo.",
    "R56 a walk blocked against nothing steep is named by what it stands on (ground and its slope, or the surface).",
    "R57 diagnostics: nearest names first (up to five), an import outside the folder shown by a climbing relative path.",
    "R58 a place within a millimetre of a turned import's footprint, inside its extent, is on it, so held.",
    "R59 library defaults from the outsiders: lay's STUB_MIN, end_unjoined within 1 m, a deck under its water stays low_deck.",
    "R60 lib/water ships weir; each effect's radius stands past its mask's zero by more than a post (wall 4, clear 6).",
    "R61 a returned bookmark's along is read in its step's document first, then the world's.",
    "R62 a step's report on an import's feature (<import>.<name> or feature.<import>.<name>) lands on the import.",
    "R63 an absolute import path is shown relative to the world's folder, climbing; another drive or root as written.",
    "All twelve dated 2026-11-17, ours (no outside source), open to the owner (docs/doctrine.md section 5).",
]

NOT_HOLD = [  # TODO.md "From M7 slice 1" and "From M7 close", by hand, each with its number as recorded there
    "coast.in_cave stops 1.95 m short in atlas; alone it arrives, 0.991 m short within the 1 m radius; likely R53's wider band at "
    "the cave's mouth, not proven (this run on page 2).",
    "Atlas rows banded at the value measured, NOT HOLDING: grade_max and road_join_max 0.442 (target 0.25), road_painted_in 0.286, "
    "join_step_max 37.24, bookmarks_wet, exit_in_grade_max 1.122, exit_out_grade_max 0.375 (this run's values on page 2). The places "
    "stand on plinths and in pits: the lift-at-exit trade is the owner's.",
    "city.old_town_street streamed against whole: they part by 1 mm 14 m in and the walk grazes a house corner at 101 m (R48's bound, owner's).",
    "over_budget is counted after the per-chunk nodes run: target 30% of cold not reached (this run's share on page 2).",
    "A build at another depth: 153.1 s with one node run under load (85.0 s at lower load); about 113 s not broken down (this run on page 2).",
    "basin's measure.py: ground_tarn_view 44.36 (oracle 57.35 +-6), ground_middle 55.79 (62.38 +-6); which state made the old bands is open.",
    "From the outsiders, open: the weir draws no mesh; formula.outside_field names lib:ways/kinds.toml:89:12, not the moved import; "
    "via build --help not understood; a building placed by on/face (a 20-line step instead); lay's walls on an 11-degree outline "
    "saw-tooth; no view on a produced storey; a lift at a named place (hint 0.74, wanted 1.66); an import's 5 to 9 own warnings unfolded; "
    "knock-on errors after a misspelt name; lay beside its building's step quiet on the console (\"0 errors, 0 warnings\").",
    "Also from the outsiders: a building's own yard or pad (the doorway's sill loses to a shaped ground kind under it, "
    "surface.gap 29 mm on a terrace).",
    "diag.internal on a pack found late (instance:<id>/v:build \"gave another value when computed again\"); the sheet stalled once "
    "for 3 minutes without the watchdog firing.",
    "Seen on foot, not explained: join_college's eye shows a large grey panel in the field beside the approach (its view is on "
    "page 2); two water surfaces meet as an upright sheet at monastery's tarn outflow; monastery.bm_deck faces its parapet; a 0.48 "
    "lip where the market's blended ground meets the fan.",
    "Checks owed: no count of memo hits or model builds in the run report (two_depths_reuse passes on the old binary, so R55 is "
    "measured, not checked); no via step commit --check (R54's cost: the only staleness check left for a program changed alone).",
    "Owed for the owner: Windows unproven (no hosted runners while nothing is pushed); another vendor's outsider run.",
    "Open in those sections and not drawn here: an upgrade note for a library that starts shipping a kind; a sheet of only the "
    "world's own views; xtask check's kinds-file rule reads text; the Sonnet mill run's leftover scratch; git add of the untracked "
    "files when rewrite is committed.",
]


# ---------------------------------------------------------------- drawing

def walks(sj: dict) -> list:
    return sj.get("walks", [])


def walk_of(sj: dict, name: str) -> dict | None:
    w = next((x for x in walks(sj) if x["name"] == name), None)
    return w and w.get("before")


def walk_text(r: dict | None) -> str:
    if not r:
        return "not walked"
    s = f"{r.get('outcome')} after {r.get('walked', 0):.1f} m"
    if r.get("outcome") == "blocked":
        s += f" by {r.get('by')}" + (f" ({r['slope']:.0f} deg)" if r.get("slope") is not None else "")
    elif r.get("outcome") == "fell" and isinstance(r.get("drop"), (int, float)):
        s += f" (dropped {r['drop']:.1f} m)"
    return s


def walk_counts(sj: dict) -> dict:
    out: dict[str, int] = {}
    for w in walks(sj):
        o = (w.get("before") or {}).get("outcome", "absent")
        out[o] = out.get(o, 0) + 1
    return out


def rows_ok(m: dict) -> tuple[int, int]:
    ls = [s for s in m.get("lines", []) if s.startswith(("ok ", "FAIL", "BAD"))]
    return sum(1 for s in ls if s.startswith("ok ")), len(ls)


def map_rows(m: dict) -> str:
    line = next((s for s in m.get("text", "").splitlines() if s.startswith("map rows: ")), "map rows: not checked")
    return line.removeprefix("map rows: ")


W, PAGE_H, DPI = 2400, 3200, 100
BODY, HEAD, MONO, LH = 11, 15, "DejaVu Sans Mono", 18
BAD, GREY, OK = "#b03a00", "#444444", "black"


class Page:
    def __init__(self, title: str, sub: str):
        self.fig = plt.figure(figsize=(W / DPI, PAGE_H / DPI), dpi=DPI, facecolor="white")
        self.fig.add_artist(plt.Line2D([0, 1], [1 - 3 / PAGE_H] * 2, color="#666666", lw=4))  # where a page starts
        self.text(40, 22, title, size=17, mono=False, bold=True)
        self.y = 56
        for s in textwrap.wrap(sub, 250):
            self.text(40, self.y, s, size=11, mono=False, colour=GREY)
            self.y += 19
        self.y += 14

    def axes(self, x, y, w, h):
        ax = self.fig.add_axes([x / W, 1 - (y + h) / PAGE_H, w / W, h / PAGE_H])
        ax.set_xticks([])
        ax.set_yticks([])
        return ax

    def text(self, x, y, s, size=BODY, mono=True, bold=False, colour="black", ha="left", box=None):
        self.fig.text(x / W, 1 - y / PAGE_H, s, fontsize=size, family=MONO if mono else None, va="top", ha=ha,
                      weight="bold" if bold else "normal", color=colour, zorder=5,
                      bbox=dict(facecolor=box, edgecolor="none", pad=2, alpha=0.8) if box else None)

    def head(self, s: str, x: int = 40) -> None:
        self.y += 12
        self.text(x, self.y, s, size=HEAD, mono=False, bold=True)
        self.y += 32

    def line(self, s: str, bold: bool = False, colour: str = "black", x: int = 40, width: int = 250) -> None:
        for part in textwrap.wrap(s, width, subsequent_indent="    ", drop_whitespace=False) or [""]:
            self.text(x, self.y, part, bold=bold, colour=colour)
            self.y += LH

    def image(self, x, y, w, p: Path, h: int, label: str | None = None, crop=None) -> None:
        if not p.exists():
            ax = self.axes(x, y, w, h)
            ax.text(0.5, 0.5, f"not made: {p.name}", ha="center", va="center", transform=ax.transAxes, color=BAD)
            return
        im = mpimg.imread(p)
        if crop:  # rows r0:r1, columns c0:c1 of the image's own pixels
            im = im[crop[0]:crop[1], crop[2]:crop[3]]
        ax = self.axes(x, y, w, h)
        ax.imshow(im, aspect="auto", interpolation="lanczos")
        if label:
            self.text(x + w - 5, y + h - 22, label, size=8, colour="white", box="black", ha="right")

    def save(self, path: Path) -> int:
        self.fig.canvas.draw()
        bottom = min(PAGE_H, int(self.y) + 30)
        img = np.asarray(self.fig.canvas.buffer_rgba())[:bottom]
        plt.imsave(path, img)
        plt.close(self.fig)
        return bottom


def draw(tmp: Path, data: dict, outsiders: Path) -> None:
    _, rustc = run(["rustc", "--version"])
    _, head = run(["git", "log", "-1", "--format=%h %ad", "--date=short"])
    _, porcelain = run(["git", "status", "--porcelain"])
    _, cpu = run(["sysctl", "-n", "machdep.cpu.brand_string"])
    _, ncpu = run(["sysctl", "-n", "hw.ncpu"])
    machine = f"{platform.system()} {platform.release()} {platform.machine()}, {cpu.strip()}, {ncpu.strip()} cores"
    dirty = [s[3:] for s in porcelain.splitlines() if s.strip()]
    a = tmp / "atlas"
    sj = jload(a / "review/sheet.json") or {}
    state = f"{len(dirty)} uncommitted paths over it ({', '.join(dirty)})" if dirty else "working tree clean"
    proto = (f"Made by docs/reports/make_m7_sheet.py on {machine}; {rustc.strip()}; Godot {sj.get('godot', '?')}; branch rewrite, "
             f"last commit {head.strip()}, {state}; made {datetime.date.today().isoformat()}. Every number is read from a build, a "
             "measure, a map render, a walk or a conformance run this script made, except where a line says it is hand-copied. "
             f"Drawn from the --keep folder {tmp.parent.name}/{tmp.name} (data.json and renders) and the outsiders' m7c-fresh-*/review "
             "runs under --outsiders: scratch folders outside the repo that may be gone; then the sheet is made again by a full "
             "run, not redrawn, and the outsiders' views cannot be.")
    pages = []

    # ---------------- page 1: the row, the cutover, the outsiders
    p = Page("via M7 sheet, page 1 of 3: outsiders build with it; the first cut", proto)
    p.head("1. M7's row (docs/plan.md) and what proves it")
    p.line("Goal: \"Outsiders build with it; first cut\". Proven by: agents from two model families build from the documents alone "
           "(the row's own note: done at M7 close with Fable and Sonnet, both Claude models; a run by another vendor's model is the "
           "owner's); spike/ deleted; main moves at the owner's review. Worlds: all. Challenge row 51.")
    p.line("What proves it here: four outsider worlds built from the documents alone (section 3, hand-copied from row 51, their "
           "views from their own runs); atlas, every example joined, built fresh by this script (page 2); conformance milestones 3 "
           "to 7 on the release binary (page 3). main has not moved: that is the owner's, locally.")
    p.head("2. The cutover (hand-written from the commits ac64d21 and dc04433)")
    for s in CUTOVER:
        p.line("- " + s)
    _, tag = run(["git", "rev-parse", "--short", "spike-final^{commit}"])
    _, spike_files = run(["git", "ls-files", "spike"])
    p.line(f"Checked by this script: tag spike-final -> {tag.strip()}; files tracked under spike/ on this tree: "
           f"{len(spike_files.split())}; crates/via-artifact present: {(ROOT / 'crates/via-artifact').exists()}.", colour=GREY)
    p.head("3. The outsiders: four agents, two Claude model lines, tasks mill and crossroads (hand-copied from docs/challenges.md row 51)")
    p.line(f"{'line':<8}{'task':<12}{'run':<5}{'first build':<13}{'outcome':<24}world", bold=True)
    for ln, task, rn, fb, out, what in OUTSIDERS:
        p.line(f"{ln:<8}{task:<12}{rn:<5}{fb:<13}{out:<24}{what}", colour=BAD if fb == "none" else OK)
    p.y += 6
    p.line(f"{'trips by cause':<50}{'1st run':>8}{'2nd run':>9}", bold=True)
    for cause, r1, r2 in TRIPS_BY_CAUSE:
        p.line(f"{cause:<50}{r1:>8}{r2:>9}")
    t1, t2 = sum(r[1] for r in TRIPS_BY_CAUSE), sum(r[2] for r in TRIPS_BY_CAUSE)
    p.line(f"{'total (summed here from the rows above)':<50}{t1:>8}{t2:>9}", bold=True)
    p.line("Per family: the second run's 22 are Sonnet's (mill 12, crossroads 10). Row 51 does not split the first run's 31 by "
           "agent; that the Sonnet agents met only the blocker all four met (they stopped at minute 3) is our inference, not a count.",
           colour=GREY)
    for s in TRIPS_NOTE:
        p.line("- " + s)
    p.head("Eye-height views from the outsiders' worlds (their own runs of engines/godot/sheet.sh, not made by this script)")
    VW = (W - 80 - 3 * 12) // 4
    VH = int(VW * 9 / 16)
    for i, (folder, f, cap) in enumerate(OUTSIDER_VIEWS):
        x, yy = 40 + (i % 4) * (VW + 12), p.y + (i // 4) * (VH + 52)
        rd = outsiders / folder
        p.image(x, yy, VW, rd / "before" / f, VH, label=f"{folder.split('/')[0]}/.../{f}")
        p.text(x + 5, yy + 5, cap, size=11, mono=False, colour="white", box="black")
        osj = jload(rd / "sheet.json") or {}
        r = walk_of(osj, f.removesuffix(".png").replace("town-", "town.").replace("market-", "market.").replace("college-", "college."))
        for k, part in enumerate(textwrap.wrap(f"walk (its sheet.json): {walk_text(r)}", VW // 9)[:2]):
            p.text(x, yy + VH + 6 + 17 * k, part, size=10, colour=BAD if r and r.get("outcome") != "arrived" else OK)
    p.y += 2 * (VH + 52) + 4
    oc = {}
    for folder in sorted({v[0] for v in OUTSIDER_VIEWS}):
        c = walk_counts(jload(outsiders / folder / "sheet.json") or {})
        oc[folder.split("/")[0].removeprefix("m7c-fresh-")] = c
    p.line("Walks in each outsider's own sheet.json: " + "; ".join(
        f"{k}: " + ", ".join(f"{v} {o}" for o, v in sorted(c.items())) for k, c in oc.items())
        + " (crossroads counts include the imports' own bookmarks; the a- worlds are mill, the b- worlds crossroads).", colour=GREY)
    pages.append(p)

    # ---------------- page 2: atlas fresh
    p = Page("via M7 sheet, page 2 of 3: atlas from a fresh build", proto)
    base_m = data["measures"].get("base", {})
    ok, n = rows_ok(base_m)
    p.head(f"4. atlas, every example joined (fresh store): base rows {ok} of {n} in band (measure.py --check base)")
    b = data["atlas"].get("base", {})
    cold = b.get("wall", float("nan"))
    p.line(f"{'build':<14}{'exit':>5}{'wall s':>9}{'% cold':>8}{'GB':>6}{'ran':>8}{'reused':>8}  {'load 1 min, before -> after':<28}rows",
           bold=True)
    desc = {"base": "cold, fresh store", "unchanged": "same again, same directory", "sibling": "into a sibling folder",
            "depth": "into deep/x/base"}
    for name in ["base", "unchanged", "sibling", "depth"] + CHANGES:
        bb = data["atlas"].get(name)
        if not bb:
            p.line(f"{name:<14} not built", colour=BAD)
            continue
        m = data["measures"].get(name, {})
        ok_, n_ = rows_ok(m) if m else (0, 0)
        rows = f"{ok_}/{n_}" if n_ else "-"
        t = bb.get("totals", {})
        bad = (bb["status"] != 1 if name in FAILS_BY_DESIGN else bb["status"] != 0) or ok_ < n_
        ld = f"{bb['load'][0].split()[0]} -> {bb['load'][1].split()[0]}"
        p.line(f"{name:<14}{bb['status']:>5}{bb['wall']:>9.1f}{format(bb['wall'] / cold, '.1%' if bb['wall'] < 0.1 * cold else '.0%'):>8}{bb['rss']:>6.2f}{t.get('ran', '?'):>8}"
               f"{t.get('reused', '?'):>8}  {ld:<28}{rows}  {desc.get(name, '--against base')}"
               + ("; exit 1 by design" if name in FAILS_BY_DESIGN and bb["status"] == 1 else ""),
               colour=BAD if bad else OK)
    for name in ["base"] + CHANGES:
        for s_ in data["measures"].get(name, {}).get("lines", []):
            if s_.startswith(("FAIL", "BAD")):
                p.line(f"out of band: {s_}", colour=BAD)
    nh = re.findall(r"^\[base\.(\w+)\][^\n]*NOT HOLDING", (ATLAS / "expect.toml").read_text(), re.M)
    bv = base_m.get("values", {})
    p.line(f"In band is not holding for {len(nh)} of the {n} base rows: expect.toml bands them at the value measured and marks "
           "them NOT HOLDING. This run: " + ", ".join(f"{k} {bv.get(k)}" for k in nh)
           + " (targets: grades 0.25; the rest stated in expect.toml).", colour=BAD)
    ob = data["atlas"].get("over_budget", {})
    for e in ob.get("errors", [])[:1]:
        p.line(f"over_budget: {e.get('code')}: {e.get('message')}", colour=GREY)
    wc = walk_counts(sj)
    tot = sum(wc.values())
    p.line(f"Walks, streamed at {RADIUS} m with water (sheet.json): {wc.get('arrived', 0)} arrived, {wc.get('blocked', 0)} blocked, "
           f"{wc.get('fell', 0)} fell of {tot}" + "".join(f", {v} {k}" for k, v in wc.items() if k not in ("arrived", "blocked", "fell"))
           + f"; on the M6 sheet {M6_WALKS['arrived']}, {M6_WALKS['blocked']}, {M6_WALKS['fell']} of {M6_WALKS['of']} (hand-copied "
           "from m6-sheet.png, same protocol).", bold=True)
    fails = [(w["name"], w.get("before") or {}) for w in walks(sj) if (w.get("before") or {}).get("outcome") == "fell"]
    if fails:
        p.line("fell: " + "; ".join(f"{nm} {walk_text(r)}" for nm, r in fails), colour=BAD)
    blk = [(w["name"], w.get("before") or {}) for w in walks(sj) if (w.get("before") or {}).get("outcome") == "blocked"]
    if blk:
        p.line(f"blocked ({len(blk)}), name: by what (an instance's id after # left out), metres walked: " + "; ".join(
            f"{nm}: {str(r.get('by')).split('#')[0]} {r.get('walked', 0):.0f}" for nm, r in blk), colour=GREY)
    fr = ((sj.get("frames") or {}).get("before")) or {}
    st = fr.get("stream") or {}
    fps, fms = fr.get("fps") or {}, fr.get("frame_ms") or {}
    fmt = lambda v: f"{v:.1f}" if isinstance(v, (int, float)) else "?"  # noqa: E731
    p.line(f"Frame tour ({FRAMES} s, streamed): loads {st.get('loads')}, unloads {st.get('unloads')}, most loaded {st.get('most_loaded')} "
           f"of {st.get('chunks')} chunks, stalls {st.get('stalls')}; fps median {fmt(fps.get('median'))}, slowest 1% "
           f"{fmt(fps.get('low_1pct'))}; frame ms p99 {fmt(fms.get('p99'))}.")
    g = data.get("godot", {})
    p.line(f"Protocol: release via and via-terrain built from the tree above, default threads, VIA_LIB = the repo's lib, python = the "
           "SDK venv; one fresh store; builds in the order of the table, each once (no spread claimed); wall and peak memory from "
           "/usr/bin/time -l; ran and reused from each run report's totals. A laptop shared with other agents: load "
           f"{data.get('load_start', '?')} at the start, {data.get('load_end', '?')} at the end. Godot: sheet.sh exit {g.get('status')}, "
           f"load {g.get('load', ['?'])[0]} at its start, 1280x720, eye 1.7 m, no frame cap; fps on a shared laptop says where to look, "
           "not a speed.", colour=GREY)
    p.head("Every join, and coast's cliff and cave, from inside atlas (streamed; the walk's outcome under each)")
    joins = sorted(w["name"] for w in walks(sj) if w["name"].startswith("join_"))
    views = joins + ["coast.along_cliff", "coast.in_cave"]
    VW = (W - 80 - 3 * 12) // 4
    VH = int(VW * 9 / 16)
    for i, name in enumerate(views):
        x, yy = 40 + (i % 4) * (VW + 12), p.y + (i // 4) * (VH + 52)
        fname = name.replace(".", "-") + ".png"
        p.image(x, yy, VW, a / "review/before" / fname, VH, label=f"review/before/{fname}")
        p.text(x + 5, yy + 5, name, size=11, mono=False, colour="white", box="black")
        r = walk_of(sj, name)
        for k, part in enumerate(textwrap.wrap(f"walk: {walk_text(r)}", VW // 9)[:2]):
            p.text(x, yy + VH + 6 + 17 * k, part, size=10, colour=BAD if r and r.get("outcome") != "arrived" else OK)
    p.y += ((len(views) + 3) // 4) * (VH + 52)
    pr = data.get("pair") or {}
    psj = jload(a / "review_pair/sheet.json") or {}
    p.head(f"Before and after: {PAIR_CHANGE} (the market 40 m east and 30 m north), seen from {PAIR_VIEW}")
    PW = (W - 80 - 12) // 2
    PH = int(PW * 9 / 16)
    pw = next((x for x in walks(psj) if x["name"] == PAIR_VIEW), {})
    for k, side in enumerate(("before", "after")):
        x = 40 + k * (PW + 12)
        p.image(x, p.y, PW, a / "review_pair" / side / f"{PAIR_VIEW}.png", PH, label=f"review_pair/{side}/{PAIR_VIEW}.png")
        p.text(x + 5, p.y + 5, f"{side}: {'atlas base' if side == 'before' else PAIR_CHANGE + ' --against base'}", size=11,
               mono=False, colour="white", box="black")
        r = pw.get(side)
        p.text(x, p.y + PH + 6, f"walk: {walk_text(r)}", size=10, colour=BAD if r and r.get("outcome") != "arrived" else OK)
    p.y += PH + 30
    pc = {side: {} for side in ("before", "after")}
    for w in walks(psj):
        for side in pc:
            o = (w.get(side) or {}).get("outcome", "absent")
            pc[side][o] = pc[side].get(o, 0) + 1
    moved = [w["name"] for w in walks(psj)
             if (w.get("before") or {}).get("outcome") != (w.get("after") or {}).get("outcome")]
    pb = pr.get("builds") or {}
    p.line(f"Walks in this pair's sheet (whole, not streamed): before " + ", ".join(f"{v} {o}" for o, v in sorted(pc["before"].items()))
           + "; after " + ", ".join(f"{v} {o}" for o, v in sorted(pc["after"].items()))
           + f"; outcome changed by the move: {', '.join(moved) if moved else 'none'}.")
    p.line(f"Protocol: engines/godot/sheet.sh --before base --after {PAIR_CHANGE} --option water=tag:water_level, 1280x720, eye 1.7 m, "
           f"exit {pr.get('status')}, load {(pr.get('load') or ['?'])[0]} at its start, made {pr.get('made', '?')}. "
           + ("The table's base and change builds were not kept, so both were built again for the pair from the same tree "
              f"(a store of their own: base {pb['base']['wall']:.1f} s, exit {pb['base']['status']}; {PAIR_CHANGE} "
              f"{pb[PAIR_CHANGE]['wall']:.1f} s, exit {pb[PAIR_CHANGE]['status']}); those walls are not the table's."
              if pb else "The pair is drawn from the table's own base and change builds."), colour=GREY)
    pages.append(p)

    # ---------------- page 3: the map, conformance, rulings, what does not hold
    p = Page("via M7 sheet, page 3 of 3: the player map, conformance, rulings, what does not hold", proto)
    p.head("5. The player map of the atlas base (tools/map_render.py on export/map)")
    mp = a / "map_base.ground.png"
    if mp.exists():
        im = mpimg.imread(mp)
        hh = int((W - 80) * im.shape[0] / im.shape[1])
        p.image(40, p.y, W - 80, mp, hh, label="tools/map_render.py <atlas base>: map_base.ground.png (left the map, right "
                "the build's own plan)")
        p.y += hh + 10
        CW, CH = (W - 80 - 20) // 2, 640  # the map at 1:1 (1 px a metre, its label zoom), two windows of the left panel
        for k, (c0, r0, what) in enumerate(((2000, 100, "townland and monastery"),
                                            (200, 900, "the city's west"))):
            p.image(40 + k * (CW + 20), p.y, CW, mp, CH, crop=(r0, r0 + CH, c0, c0 + CW))
            p.text(45 + k * (CW + 20), p.y + 5, f"map_base.ground.png at 1:1, pixels x {c0}..{c0 + CW}, rows {r0}..{r0 + CH}: "
                   f"{what}", size=10, mono=False, colour="white", box="black")
        p.y += CH + 10
        mb = data["maps"].get("base", {}).get("values", {}).get("counts", {})
        p.line(f"Map cell 2 m, {mb.get('tiles')} tiles: {mb.get('classes')} classes, {mb.get('ways')} ways, {mb.get('water')} water "
               f"courses, {mb.get('footprints')} footprints, {mb.get('regions')} areas, {mb.get('labels')} labels; label overlaps "
               f"{mb.get('label_overlaps')}, unplaced {mb.get('labels_unplaced')}; way_on_cells_min {mb.get('way_on_cells_min')}, "
               f"water_on_cells_min {mb.get('water_on_cells_min')}; map rows (expect.toml section map): "
               f"{map_rows(data['maps'].get('base', {}))}.")
    else:
        p.line("map not made", colour=BAD)
    groups: dict[str, list] = {}  # milestones started together: by their recorded group, else by the same load reading
    for m in MILESTONES:
        cf = (data.get("conformance") or {}).get(str(m))
        if cf:
            groups.setdefault(str(cf.get("group") or cf.get("load")), []).append(m)
    p.head("6. Conformance per milestone (tools/conformance on the release via of this tree; run side by side in the groups "
           f"{', '.join(str(g) for g in groups.values())}, as recorded)")
    for m in MILESTONES:
        cf = (data.get("conformance") or {}).get(str(m), {})
        p.line(f"milestone {m}: {cf.get('summary', 'not run')} (exit {cf.get('code')}, load {str(cf.get('load', '?')).split()[0]})",
               colour=BAD if cf.get("code") else OK)
        for x in cf.get("failed", []):
            p.line(f"    failed: {x.strip()[:200]}", colour=BAD)
    marked = [c for c in (ROOT / "spec/cases").rglob("case.toml") if re.search(r"^pending\s*=", c.read_text(), re.M)]
    p.line(f"Pending at milestone 7: the {len(marked)} cases whose case.toml says `pending = \"<what is owed>\"` (`[lock]` and "
           "`via lock` not written, a path across drives that waits on Windows runners, and others), read from spec/cases by this "
           "script; a case of a later milestone is pending below it.", colour=GREY)
    if any("graph/epoch" in x for cf in (data.get("conformance") or {}).values() for x in cf.get("failed", [])):
        p.line("graph/epoch sets VIA_TEST_EPOCH_OFFSET, which only test (debug) builds read (its case.toml says so): on a release "
               "via nothing re-runs, so it fails by its protocol, not by the engine.", colour=GREY)
    p.head("7. Rulings added in M7 (docs/doctrine.md section 5, R52 to R63), summarised by hand")
    for s in RULINGS:
        p.line(s)
    p.head("8. What does not hold (hand-written from TODO.md \"From M7 slice 1\" and \"From M7 close\"; numbers as recorded there)")
    for s in NOT_HOLD:
        p.line("- " + s, colour=BAD)
    pages.append(p)

    sizes = []
    for i, pg in enumerate(pages, 1):
        path = OUT.with_name(f"m7-sheet-{i}.png")
        sizes.append((path, pg.save(path)))
    # one file as the plan asks: the pages stacked, each readable at 100%
    ims = [plt.imread(pth) for pth, _ in sizes]
    whole = np.concatenate(ims, axis=0)
    plt.imsave(OUT, whole)
    for pth, _ in sizes:
        pth.unlink()
    print(f"wrote {OUT} ({W}x{whole.shape[0]}; pages {[h for _, h in sizes]}; renders in {tmp})")


def main() -> None:
    keep = sys.argv[sys.argv.index("--keep") + 1] if "--keep" in sys.argv else None
    if "--draw-only" in sys.argv and not (keep and (Path(keep) / "data.json").exists()):
        sys.exit("--draw-only needs --keep DIR holding the data.json of an earlier run")
    if "--outsiders" not in sys.argv:
        sys.exit("--outsiders DIR: the folder holding the outsiders' m7c-fresh-*/review/ runs")
    outsiders = Path(sys.argv[sys.argv.index("--outsiders") + 1])
    tmp = Path(keep) if keep else Path(tempfile.mkdtemp(prefix="via-m7-sheet-"))
    tmp.mkdir(parents=True, exist_ok=True)
    if "--draw-only" in sys.argv:
        data = jload(tmp / "data.json")
    else:
        if "VIA" not in os.environ:
            print("building via ...", flush=True)
            code, out = run(["cargo", "build", "--release", "-q", "-p", "via-cli", "-p", "via-terrain"])
            if code:
                sys.exit(out)
        data = collect(tmp)
    if "--collect-only" not in sys.argv:
        draw(tmp, data, outsiders)


if __name__ == "__main__":
    main()
