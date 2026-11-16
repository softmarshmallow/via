# /// script
# requires-python = ">=3.11"
# dependencies = ["matplotlib>=3.9,<3.10", "numpy>=2.0,<3"]
# ///
"""Draw the M4 sheet, docs/reports/m4-sheet.png, from fresh builds of M4's three worlds and M3's five.

    uv run docs/reports/make_m4_sheet.py [--keep DIR] [--draw-only] [--fresh]

What it runs, in order (every number on the sheet comes from one of these):

1. The release `via` the VIA environment variable names, else `cargo build --release -p via-cli`.
2. A scratch user config (VIA_CONFIG_DIR) with `python` set to the SDK's venv Python, and each
   world's steps trusted (base and every change, and the parts built alone).
3. Per world, in one fresh store: the parts built alone (strait a and b, border's wild), the base
   cold, the base again `--against` itself (unchanged: the memo), then every change in the world's
   changes/ folder `--against` the base. Each build is timed by /usr/bin/time -l (wall seconds,
   peak resident memory) and by its run report's `totals` (graph.md 6.3: ms, ran, reused, nodes).
4. Each world's measure.py `--check` on the base and on every change (bands from expect.toml).
5. For the M4 worlds (strait, county, border): engines/godot/sheet.sh, base against the key change,
   water=tag:water_level, at four bookmarks each. The M3 worlds are timed and checked only; their
   views are on docs/reports/m3-sheet.png.

The drawing helpers (plan view, measure-line parsing, timing) are make_m3_sheet.py's, loaded with
runpy. "Still wrong" is written by hand from the M4 slice 3 reports (review on foot, fix pass, the
three world reports) and from this sheet's views; the lines that are numbers are computed.
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
M3 = runpy.run_path(str(HERE / "make_m3_sheet.py"))
ROOT, LIB, PY, VIA = M3["ROOT"], M3["LIB"], M3["PY"], M3["VIA"]
run, jload, timed, plan_view, m3_rows = M3["run"], M3["jload"], M3["timed"], M3["plan_view"], M3["rows"]
plt, mpimg, np = M3["plt"], M3["mpimg"], M3["np"]
OUT = ROOT / "docs/reports/m4-sheet.png"

# Each world: key change and what it says; measure.py arguments ("alone": --alone id=DIR for the
# sections in `alone_for`; "run": --run <change>.run.json on changes; "base"/"none" as in M3); the
# parts built alone; four bookmarks (empty for a timing-only world).
M4_WORLDS = [
    {"name": "strait", "key": "move_b", "key_says": "island b moved 23 m (16 west, 16 south) and turned 10 degrees; the ferry follows by name",
     "margs": "run", "alone": {"a": "a/world.toml", "b": "b/world.toml"}, "alone_for": ("base", "move_b"),
     "bookmarks": ["arrive_b", "back_to_a", "b_shore_e", "b.at_slip"]},
    {"name": "county", "key": "move_town", "key_says": "the imported town (quay) moved 25 m (to [324, 196]) and turned to face -8 on the county's land",
     "margs": "run", "alone": {}, "alone_for": (),
     "bookmarks": ["at_gate", "town.wall_face", "join_shore", "join_west"]},
    {"name": "border", "key": "move_abbey", "key_says": "the abbey (monastery) moved 30 m east; walks and bank openings follow",
     "margs": "none", "alone": {"wild": "wild/world.toml"}, "alone_for": ("base",),
     "bookmarks": ["at_gate", "south_opening", "west_opening", "bank_south"]},
]
M3_WORLDS = [{"name": n, "margs": m, "alone": {}, "alone_for": (), "bookmarks": []}
              for n, m in (("quay", "run"), ("townland", "base"), ("example", "none"),
                           ("monastery", "base"), ("city", "base"))]


def build(w: dict, root: Path, env: dict, folder: str, wf: Path, change: Path | None = None,
          against: bool = True) -> dict:
    d = root / folder
    diag = root / f"{folder}.diagnostics.json"
    cmd = [VIA, "build", wf, "--lib", LIB, "--store", root / "store", "--out", d, "--diagnostics", diag]
    if change:
        cmd += ["--change", change]
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
    exits = [{"message": x.get("message", ""), "subject": x.get("subject"), "data": x.get("data")}
             for x in rows if x["code"] == "import.exit_unanswered"]
    return {"folder": folder, "status": code, "wall": wall, "rss": rss, "exits": exits,
            "first": next((s for s in text.splitlines() if s.startswith("error")), ""),
            "totals": r.get("totals", {}), "codes": counts}


def measure(w: dict, root: Path, folder: str, section: str) -> tuple[int, list[str]]:
    args = [PY, ROOT / "examples" / w["name"] / "measure.py", root / folder]
    if section in w["alone_for"]:
        for pid in w["alone"]:
            args += ["--alone", f"{pid}={root / ('alone_' + pid)}"]
    if section != "base":
        if w["margs"] in ("base", "run") and w["name"] in ("quay", "townland", "monastery", "city"):
            args += ["--base", root / "base"]
        if w["margs"] == "run":
            args += ["--run", root / (f"{folder}.diagnostics.json" if w["name"] == "quay" else f"{folder}.run.json")]
    code, out = run(args + ["--check", section], env={"VIA": str(VIA), "VIA_LIB": str(LIB)})
    return code, [s for s in out.splitlines() if s.strip()]


def collect(tmp: Path) -> dict:
    env = {"VIA_CONFIG_DIR": str(tmp / "config"), "VIA_LIB": str(LIB)}
    (tmp / "config").mkdir(parents=True, exist_ok=True)
    code, out = run([VIA, "config", "program", "python", PY], env=env)
    if code:
        sys.exit(f"via config program python: {out}")
    _, load0 = run(["sysctl", "-n", "vm.loadavg"])
    data = {"worlds": {}, "load_start": load0.strip().strip("{} ")}
    for w in M4_WORLDS + M3_WORLDS:
        saved = tmp / f"{w['name']}.json"
        if saved.exists() and "--fresh" not in sys.argv:
            data["worlds"][w["name"]] = json.loads(saved.read_text())
            continue
        print(f"{w['name']} ...", flush=True)
        here = ROOT / "examples" / w["name"]
        wf = here / "world.toml"
        changes = sorted(p.stem for p in (here / "changes").glob("*.toml"))
        for target, extra in [(wf, [])] + [(wf, ["--change", here / "changes" / f"{c}.toml"]) for c in changes] \
                + [(here / f, []) for f in w["alone"].values()]:
            code, out = run([VIA, "trust", target, "--lib", LIB, "--yes"] + extra, env=env)
            if code:
                sys.exit(f"via trust {target} {extra}: {out}")
        root = tmp / w["name"]
        if root.exists():
            shutil.rmtree(root)
        root.mkdir(parents=True)
        alone = {pid: build(w, root, env, f"alone_{pid}", here / f, against=False) for pid, f in w["alone"].items()}
        cold = build(w, root, env, "base", wf, against=False)
        if cold["status"]:
            sys.exit(f"{w['name']} base build failed:\n{cold['first']}")
        warm = build(w, root, env, "unchanged", wf)
        ch, ms = {}, {"base": measure(w, root, "base", "base")}
        for c in changes:
            print(f"  {c}", flush=True)
            ch[c] = build(w, root, env, c, wf, here / "changes" / f"{c}.toml")
            ms[c] = measure(w, root, c, c)
        g = [0, ""]
        if w["bookmarks"]:
            out = root / "review"
            g = list(run([ROOT / "engines/godot/sheet.sh", "--before", root / "base", "--after", root / w["key"],
                          "--out", out, "--option", "water=tag:water_level"], env={"SHEET_TIMEOUT": "1500"}))
            g[1] = g[1][-600:]
        m = jload(root / "base/manifest.json")
        data["worlds"][w["name"]] = {
            "alone": alone, "cold": cold, "warm": warm, "changes": ch, "measures": ms, "godot": g,
            "frame": m["frame"], "instances": m["instances"]["count"],
        }
        saved.write_text(json.dumps(data["worlds"][w["name"]], indent=1))
        # Keep only what the sheet draws: the base, the key change and the views.
        shutil.rmtree(root / "store", ignore_errors=True)
        for p in root.iterdir():
            if p.is_dir() and p.name not in ("base", w.get("key"), "review"):
                shutil.rmtree(p)
            elif not w["bookmarks"] and p.is_dir():
                shutil.rmtree(p)
    _, load1 = run(["sysctl", "-n", "vm.loadavg"])
    data["load_end"] = load1.strip().strip("{} ")
    data["godot_version"] = (jload(tmp / "strait/review/sheet.json") or {}).get("godot", "?")
    return data


def rows(d: dict) -> dict[str, dict]:
    """Every measure.py line as section -> measure -> value (value may be a list or a string)."""
    out: dict[str, dict] = {}
    for sec, (_, lines) in d["measures"].items():
        for s in lines:
            mm = re.match(r"^(ok|FAIL)\s+\S+?: (\S+) = (.+?)(, band| in \[| \(|$)", s)
            if mm:
                v = mm.group(3)
                try:
                    v = float(v)
                except ValueError:
                    pass
                out.setdefault(sec, {})[mm.group(2)] = v
    return out


def acceptance(data: dict) -> list[tuple[str, str]]:
    """The M4 row of docs/plan.md, a number for each."""
    W = data["worlds"]
    R = {n: rows(d) for n, d in W.items()}

    def v(world, sec, key):
        x = R.get(world, {}).get(sec, {}).get(key)
        return "?" if x is None else f"{x:g}" if isinstance(x, float) else str(x)

    def tot(world, sec):
        b = W.get(world, {}).get("changes", {}).get(sec) or W.get(world, {}).get("warm") or {}
        t = b.get("totals") or {}
        return f"{t.get('ran', '?')} ran / {t.get('reused', '?')} reused of {t.get('nodes', '?')} nodes"

    def ok(world, sec):
        code, ls = W.get(world, {}).get("measures", {}).get(sec, (1, []))
        n = [s for s in ls if s.startswith(("ok", "FAIL"))]
        bad = sum(s.startswith("FAIL") for s in n)
        return f"{len(n) - bad}/{len(n)} rows"

    def exit_text(world):
        b = W.get(world, {}).get("changes", {}).get("forgotten_exit") or {}
        e = (b.get("exits") or [{}])[0]
        dd = e.get("data") or {}
        return b.get("status", "?"), e.get("message", "?"), dd.get("write", "?")

    out = [("head", "1. An import moved and turned (strait move_b; county move_town)")]
    out.append(("", f"strait move_b ({ok('strait', 'move_b')}): {tot('strait', 'move_b')}; import:a ran "
                    f"{v('strait', 'move_b', 'ran_import_a')}, import:b {v('strait', 'move_b', 'ran_import_b')}, placed:b "
                    f"{v('strait', 'move_b', 'ran_placed_b')}; field chunks re-ran "
                    f"{v('strait', 'move_b', 'fields_ran')} (all: no `brought` node per chunk); ferry_off {v('strait', 'move_b', 'ferry_off')} m, seams "
                    f"{v('strait', 'move_b', 'seam')}. Neither island is built again; only what names b's places re-runs.   HELD"))
    out.append(("", f"county move_town ({ok('county', 'move_town')}): {tot('county', 'move_town')}; import:town ran "
                    f"{v('county', 'move_town', 'ran_import_town')}, placed:town {v('county', 'move_town', 'ran_placed_town')}, "
                    f"field chunks re-ran {v('county', 'move_town', 'fields_ran')} (all); road_off "
                    f"{v('county', 'move_town', 'road_off')} m; one seam (14 pairs, review on foot) at grade "
                    f"{v('county', 'move_town', 'seam_grade')}. Followed with no other edit; every chunk re-runs (no "
                    f"`brought` node per chunk) and the turned quay wall notches.   PARTLY"))
    for world in ("strait", "county"):
        code, msg, write = exit_text(world)
        out.append(("head", f"2. A forgotten exit says what to write ({world} forgotten_exit, exit {code})") if world == "strait"
                    else ("", ""))
        out.append(("", f"{world}: \"{msg}\"  data.write = {write}   HELD"))
    out = [x for x in out if x != ("", "")]
    out.append(("head", "3. A moved square re-runs only its district"))
    out.append(("", "spec case composition/moved_square FAILS (conformance --milestone 4: 816 passed, 1 failed, 28 pending; "
                    "M4 slice 3 fix pass, working tree after e67c061). An import's own steps do not run inside the parent's "
                    "build (import::built builds it eagerly in prepare), and `for_each = \"given:...\"` with a `given:` "
                    "selector is not written. No world shows it; county's town is bare (town_streets "
                    f"{v('county', 'base', 'town_streets')}, town_instances {v('county', 'base', 'town_instances')}).   NOT HELD"))
    out.append(("head", "Worlds meeting by ground (border, rows 54-55)"))
    out.append(("", f"border base ({ok('border', 'base')}): reach {v('border', 'base', 'held')} pairs held, unanswered exits "
                    f"{v('border', 'base', 'unanswered')}, seams {v('border', 'base', 'seam')}, bank crest "
                    f"{v('border', 'base', 'crest')} m, openings {v('border', 'base', 'opening')} m; move_abbey "
                    f"({ok('border', 'move_abbey')}): gate {v('border', 'move_abbey', 'gate')}, gate_off "
                    f"{v('border', 'move_abbey', 'gate_off')} m, {tot('border', 'move_abbey')}; shut ({ok('border', 'shut')}): "
                    f"not_sealed {v('border', 'shut', 'not_sealed')}, openings {v('border', 'shut', 'opening')} m.   HELD on "
                    f"its measures; water stands in the walks (below)"))
    return out


def draw(tmp: Path, data: dict) -> None:
    _, rustc = run(["rustc", "--version"])
    _, head = run(["git", "log", "-1", "--format=%h %ad", "--date=short"])
    _, porcelain = run(["git", "status", "--porcelain"])
    _, cpu = run(["sysctl", "-n", "machdep.cpu.brand_string"])
    machine = f"{platform.system()} {platform.release()} {platform.machine()}, {cpu.strip()}"
    dirty = len(porcelain.splitlines())

    W, H, DPI = 2400, 9000, 100
    fig = plt.figure(figsize=(W / DPI, H / DPI), dpi=DPI, facecolor="white")
    BODY, HEAD, MONO, LH = 10, 14, "DejaVu Sans Mono", 17
    BAD, GREY, OK = "#b03a00", "#444444", "black"

    def axes(x, y, w, h):
        return fig.add_axes([x / W, 1 - (y + h) / H, w / W, h / H])

    def text(x, y, s, size=BODY, mono=True, bold=False, colour="black", ha="left", box=None):
        fig.text(x / W, 1 - y / H, s, fontsize=size, family=MONO if mono else None, va="top", ha=ha,
                 weight="bold" if bold else "normal", color=colour, zorder=5,
                 bbox=dict(facecolor=box, edgecolor="none", pad=1.5, alpha=0.7) if box else None)

    class Col:
        def __init__(self, x: int, y: int, width: int = 200):
            self.x, self.y, self.width = x, y, width

        def head(self, s: str, size=HEAD) -> None:
            self.y += 8
            text(self.x, self.y, s, size=size, mono=False, bold=True)
            self.y += 26

        def line(self, s: str, bold: bool = False, colour: str = "black") -> None:
            for part in textwrap.wrap(s, self.width, subsequent_indent="    ", drop_whitespace=False) or [""]:
                text(self.x, self.y, part, bold=bold, colour=colour)
                self.y += LH

    state = f"working tree: {dirty} uncommitted paths over it" if dirty else "working tree clean"
    text(40, 22, f"via M4 sheet: strait, county, border (and M3's five timed)   branch rewrite, last commit "
         f"{head.strip()}; {state}   made {datetime.date.today().isoformat()}", size=16, mono=False, bold=True)
    text(40, 56, f"Made by docs/reports/make_m4_sheet.py on {machine}; {rustc.strip()}; Godot "
         f"{data.get('godot_version', '?')}. Every number is read from a build, a report or a command the script ran; "
         f"the builds are of the working tree, not of the commit.", size=11, mono=False, colour=GREY)
    text(40, 78, "Per M4 world: eye height (1.7 m, engines/godot/sheet.sh, 1280x720) at four views before (top) and after "
         "(below) its key change; the plan, before black, after orange dashed;\nevery build with wall time, peak memory and "
         "what re-ran; the world's measure.py --check against its expect.toml (ok / FAIL).", size=11, mono=False, colour=GREY)

    c = Col(40, 120, width=225)
    c.head("M4 acceptance (docs/plan.md row M4: \"Worlds built apart compose by name\"), each with its number")
    for kind, s in acceptance(data):
        if kind == "head":
            c.line(s, bold=True)
        else:
            c.line("  " + s, colour=BAD if ("NOT HELD" in s or "PARTLY" in s) else OK)
    y = c.y + 16

    PW = 540
    VW = (W - 80 - PW - 30 - 3 * 8) // 4
    VH = int(VW * 9 / 16)
    for w in M4_WORLDS:
        d = data["worlds"].get(w["name"])
        if not d:
            continue
        root = tmp / w["name"]
        fr = d["frame"]
        ext = fr["extent"]
        text(40, y, f"{w['name']}   {ext[2] - ext[0]:g} x {ext[3] - ext[1]:g} m at {fr['resolution']:g} m posts, "
             f"{d['instances']} instances   key change: {w['key']}, {w['key_says']}", size=15, mono=False, bold=True)
        y += 32
        ax = axes(40, y, PW, PW)
        if (root / "base/manifest.json").exists():
            plan_view(ax, root / "base", root / w["key"], w["bookmarks"])
        vx0 = 40 + PW + 30
        for i, bm in enumerate(w["bookmarks"]):
            x = vx0 + i * (VW + 8)
            text(x, y, bm, size=11, mono=False, bold=True)
            for j, side in enumerate(("before", "after")):
                a = axes(x, y + 20 + j * (VH + 6), VW, VH)
                p = root / "review" / side / f"{bm.replace('.', '-')}.png"  # sheet.sh writes b.at_slip as b-at_slip
                if p.exists():
                    a.imshow(mpimg.imread(p))
                else:
                    a.text(0.5, 0.5, "not rendered", ha="center", va="center", transform=a.transAxes)
                a.set_xticks([])
                a.set_yticks([])
                text(x + 4, y + 24 + j * (VH + 6), side if side == "before" else f"after {w['key']}",
                     size=8, mono=False, colour="white", box="black")
        vy = y + 20 + 2 * (VH + 6)
        text(vx0, vy, f"sheet.sh --option water=tag:water_level; exit {d['godot'][0]}", size=9,
             colour=GREY if not d["godot"][0] else BAD)
        c = Col(vx0, vy + 22, width=175)
        c.line(f"{'build':<16}{'exit':>4}{'wall s':>8}{'peak GB':>8}{'totals.ms':>10}  {'ran/reused/nodes':<20}"
               f"warnings and errors (non-info findings)", bold=True)

        def row(label: str, b: dict, expect_fail: bool = False) -> None:
            t = b.get("totals") or {}
            nodes = f"{t.get('ran', '?')}/{t.get('reused', '?')}/{t.get('nodes', '?')}"
            codes = ", ".join(f"{k} {n}" for k, n in sorted(b["codes"].items(), key=lambda kv: -kv[1]))
            c.line(f"{label:<16}{b['status']:>4}{b['wall']:>8.1f}{b['rss']:>8.2f}{t.get('ms', '?')!s:>10}  "
                   f"{nodes:<20}{codes}", colour=OK if b["status"] == 0 or expect_fail else BAD)

        for pid, b in d["alone"].items():
            row(f"{pid} alone", b)
        row("base (cold)", d["cold"])
        row("unchanged", d["warm"])
        for name, b in d["changes"].items():
            row(name, b, expect_fail=name == "forgotten_exit" and b["status"] == 1)
        y = max(y + PW + 10, c.y + 6)
        lines = []
        for sec, (code, ls) in d["measures"].items():
            lines += [s for s in ls if s.startswith(("ok", "FAIL"))] or [f"FAIL {sec}: exit {code}, no rows"]
        n_bad = sum(1 for s in lines if s.startswith("FAIL"))
        text(40, y, f"measures: examples/{w['name']}/measure.py --check, base and every change: "
             f"{len(lines) - n_bad} ok, {n_bad} FAIL", size=11, mono=False, bold=True, colour=BAD if n_bad else OK)
        y += 22
        third = (len(lines) + 2) // 3
        cols = [Col(40 + k * 780, y, width=88) for k in range(3)]
        for i, s in enumerate(lines):
            s = re.sub(r"^ok\s+", "ok ", s)
            cols[min(i // max(third, 1), 2)].line(s[:88], colour=BAD if s.startswith("FAIL") else OK)
        y = max(cl.y for cl in cols) + 24

    c = Col(40, y, width=225)
    c.head("Build times, M3 and M4 worlds: cold, unchanged and every change (wall seconds, /usr/bin/time -l; one build each)")
    c.line(f"{'world':<10}{'cold s':>8}{'GB':>6}{'unchanged':>11}{'%':>5}  changes --against base: wall s (% of cold), "
           f"exit when not 0; measures ok/rows", bold=True)
    for w in M4_WORLDS + M3_WORLDS:
        d = data["worlds"].get(w["name"])
        if not d:
            continue
        cw = d["cold"]["wall"]
        ch = ", ".join(f"{n} {b['wall']:.1f} ({b['wall'] / cw:.0%})" + (f" exit {b['status']}" if b["status"] else "")
                       for n, b in d["changes"].items())
        al = "".join(f"; {p} alone {b['wall']:.1f}" for p, b in d["alone"].items())
        lines = [s for _, ls in d["measures"].values() for s in ls if s.startswith(("ok", "FAIL"))]
        bad = sum(s.startswith("FAIL") for s in lines)
        c.line(f"{w['name']:<10}{cw:>8.1f}{d['cold']['rss']:>6.2f}{d['warm']['wall']:>11.1f}"
               f"{d['warm']['wall'] / cw:>5.0%}  {ch}{al}; measures {len(lines) - bad}/{len(lines)}",
               colour=BAD if bad else OK)
    c.line("Protocol: release via from the working tree, default threads, VIA_LIB = the repo's lib, python = the SDK venv; "
           "one fresh store per world; order: parts alone, base cold, unchanged (the base again --against base), each change "
           f"--against base, all in that store. Laptop shared with other agents building: load average {data['load_start']} "
           f"at the start, {data['load_end']} at the end. One run each: no spread is claimed; sub-second walls are noise.",
           colour=GREY)
    c.head("Still wrong")
    for s in still_wrong(data):
        c.line(s, colour=BAD)
    bottom = c.y + 30

    fig.canvas.draw()
    img = np.asarray(fig.canvas.buffer_rgba())[: int(bottom)]
    plt.imsave(OUT, img)
    print(f"wrote {OUT}  ({W}x{int(bottom)}; scratch builds in {tmp})")


def still_wrong(data: dict) -> list[str]:
    """Computed lines first (what the sheet's own numbers say), then what was seen."""
    out = []
    for w in M4_WORLDS + M3_WORLDS:
        d = data["worlds"].get(w["name"])
        if not d:
            continue
        for sec, (code, ls) in d["measures"].items():
            for s in ls:
                if s.startswith("FAIL"):
                    out.append(f"{w['name']}: {s[:200]}")
        for name, b in d["changes"].items():
            if b["status"] and not (name == "forgotten_exit" and b["status"] == 1):
                out.append(f"{w['name']} {name}: exit {b['status']}: {b.get('first', '')[:170]}")
        if d["godot"][0]:
            out.append(f"{w['name']}: sheet.sh exit {d['godot'][0]}")
    return out + STILL_WRONG_BY_EYE


# Written by hand from the M4 slice 3 reports (review on foot, fix pass, county, border, join,
# cost; 2026-11-16) and this sheet's views. Numbers with no source here are those reports', each
# taken with release via of the working tree, one fresh store, one run, a shared laptop.
STILL_WRONG_BY_EYE: list[str] = [
    "1. The moved square (acceptance 3) is not shown: an import's own steps do not run inside the parent's build, so",
    "   county's town is bare (grass where the quay's streets and houses stand; the road meets an invisible gate) and",
    "   composition/moved_square fails. Owed: import step nodes `<id>.step:...` in the parent's graph, for_each given:.",
    "2. An import is rebuilt whole in the parent's prepare and merge: county unchanged costs 63% of its cold build (table",
    "   above; merge 2.7 s of 5.4 s node time on the review's run), move_town re-runs every field chunk. Owed: the",
    "   import graph in the parent's graph and a `brought:` node per chunk.",
    "3. A turned import notches sheer ground: county's quay wall is scalloped after move_town, its edge shows a dark",
    "   staircase band (the turned footprint drawn on cells: paint or region edge, heights smooth within 0.2 m over 3 m),",
    "   one seam of 14 pairs at grade 1.61; strait's b slipway apron is notched on one side after move_b.",
    "4. Paint and weights are not brought (import.weights_dropped in every import): no setts, flags or trail paving show.",
    "5. border: brought water stands on the walks (2188 posts above 0.05 m, max 1.02 m; west_opening walk blocked by",
    "   0.69 m): water is fixed at rank 0 before the join layer lowers the walk beds, and a layer cannot dry brought water.",
    "   No measure sees it (walk_wet is owed).",
    "6. border views: at_gate hugs the monastery's enclosure wall 0.75 m off (reads as a cliff; not a join fault),",
    "   south_opening aims through the wall corner; the wild's own look_corner view is cut by the bank; fringe trees",
    "   stand on the bank's slope; the monastery's brought views outside the footprint look at bare grass.",
    "7. What the vocabulary could not say: give a road to an import (given features: three failures), narrow a given",
    "   region's default to a named region, growth from the wild's own rule past its footprint, names for an outline's",
    "   sides, `blend` naming a place.",
    "8. Cost: a far edit still re-runs every step whose reads are whole-extent (city move_avenue all 10 steps; houses_down",
    "   49 s); city unchanged peaks above its cold build's memory and write:changes.json takes seconds.",
    "9. M4 slice 2 took 11 hours; slice 3 ran five agents in 90-minute boxes. Plan step 6 (a fresh agent builds a variation",
    "   from the documents alone) was not run for M4.",
]


def main() -> None:
    keep = sys.argv[sys.argv.index("--keep") + 1] if "--keep" in sys.argv else None
    tmp = Path(keep) if keep else Path(tempfile.mkdtemp(prefix="via-m4-sheet-"))
    if "--draw-only" in sys.argv:
        data = json.loads((tmp / "data.json").read_text())
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
