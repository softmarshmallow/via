# /// script
# requires-python = ">=3.11"
# dependencies = ["matplotlib>=3.9,<3.10", "numpy>=2.0,<3", "pillow>=10,<12"]
# ///
"""Draw the M6 sheet, docs/reports/m6-sheet.png, from fresh builds of atlas (base and every change) and station.

    uv run docs/reports/make_m6_sheet.py [--keep DIR] [--draw-only] [--fresh]

What it runs, in order (every number on the sheet comes from one of these):

1. The release `via` and `via-terrain` the VIA and VIA_TERRAIN environment variables name, else
   `cargo build --release -p via-cli -p via-terrain` into the repo's target.
2. A scratch user config (VIA_CONFIG_DIR): `python` = the SDK's venv Python (3.12), `via-terrain` =
   the release binary; atlas trusted for its base and each change, station for its base.
3. atlas, in one fresh store: the base cold, the base again into the same directory (unchanged),
   then move_market, move_square, forgotten_exit and over_budget, each `--against` the base, in that
   order. Each build is timed by /usr/bin/time -l (wall seconds, peak resident memory) and read for
   its run report and its diagnostics (the over-budget message is quoted from them).
4. examples/atlas/measure.py `--check <section>` on the base and on each change (with `--run` for a
   change), bands from examples/atlas/expect.toml.
5. tools/map_render.py on the base's player map (`--expect`, section `map`) and on move_market's
   (`--against` the base, section `map_move_market`): the map images and the changed tiles.
6. station built alone (its own store) and tools/map_render.py on its map: the per-floor plans.
7. engines/godot/sheet.sh --before <atlas base> --stream 96 --frames 120 --option
   water=tag:water_level (1280x720, eye 1.7 m; walks per spec/build.md 15.1; the frame tour per
   sheet.gd `_frames`): every view on the sheet is one of its renders, named on the sheet by file.
8. The conformance runner (tools/conformance) on the release `via`, milestones 6 and 5.

"What does not hold" is written by hand from TODO.md "From M6 close" and the README of atlas; every
line that is a number is computed from this run. Rulings R44 to R51 of docs/doctrine.md are
summarised by hand.
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
OUT = ROOT / "docs/reports/m6-sheet.png"
ATLAS = ROOT / "examples/atlas"
CHANGES = ["move_market", "move_square", "forgotten_exit", "over_budget"]
FAILS_BY_DESIGN = {"forgotten_exit", "over_budget"}  # each stops with exit 1 (expect.toml)
RADIUS, FRAMES = 96, 120


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


def build(root: Path, env: dict, folder: str, wf: Path, change: str | None = None, store: str = "store") -> dict:
    d = root / folder
    diag = root / f"{folder}.diagnostics.json"
    cmd = [VIA, "build", wf, "--lib", LIB, "--store", root / store, "--out", d, "--diagnostics", diag]
    if change:
        cmd += ["--change", ATLAS / "changes" / f"{change}.toml", "--against", root / "base"]
    load0 = loadavg()
    code, text, wall, rss = timed(cmd, ROOT, env)
    r = jload(root / f"{folder}.run.json") or {}
    rows = (jload(diag) or {}).get("diagnostics", [])
    counts: dict[str, int] = {}
    for x in rows:
        if x["severity"] != "info":
            k = f"{x['code']} ({x['severity'][0]})"
            counts[k] = counts.get(k, 0) + 1
    errors = [x for x in rows if x["severity"] == "error"]
    return {"folder": folder, "status": code, "wall": wall, "rss": rss, "load": [load0, loadavg()],
            "first": next((s for s in text.splitlines() if s.startswith("error")), ""),
            "totals": r.get("totals", {}), "codes": counts, "errors": errors[:4]}


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
    values, floors = {}, []
    for s in text.splitlines():  # the report is one JSON line; per-floor checks follow, one JSON line each
        if s.startswith("{"):
            try:
                v = json.loads(s)
            except ValueError:
                continue
            if "counts" in v and not values:
                values = v
            elif "floor" in v:
                floors.append(v)
    return {"code": code, "values": values, "floors": floors,
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
    steps = [("base", None), ("unchanged", None)] + [(c, c) for c in CHANGES]
    for name, change in steps:
        if name in data["atlas"]:
            continue
        print(f"atlas {name} ...", flush=True)
        folder = "base" if name == "unchanged" else name
        data["atlas"][name] = build(a, env, folder, wf, change)
        print(f"  exit {data['atlas'][name]['status']}, {data['atlas'][name]['wall']:.1f} s", flush=True)
        save(tmp, data)
        if name == "unchanged":
            continue
        data["measures"][name] = measure(a, folder, name, env, change is not None)
        save(tmp, data)
    if "base" not in data["maps"]:
        data["maps"]["base"] = map_render(a / "base", a / "map_base", ["--expect", ATLAS / "expect.toml"])
        data["maps"]["move_market"] = map_render(
            a / "move_market", a / "map_move_market",
            ["--against", a / "base", a / "base/export/map", "--expect", ATLAS / "expect.toml",
             "--section", "map_move_market"])
        save(tmp, data)
    if "station" not in data:
        print("station ...", flush=True)
        s = tmp / "station"
        s.mkdir(exist_ok=True)
        swf = ROOT / "examples/station/world.toml"
        code, out = run([VIA, "trust", swf, "--lib", LIB, "--yes"], env=env)
        if code:
            sys.exit(f"via trust station: {out}")
        b = build(s, env, "base", swf)
        data["station"] = {"build": b, "map": map_render(s / "base", s / "map", ["--expect", ROOT / "examples/station/expect.toml",
                                                                                 "--section", "map"])}
        save(tmp, data)
    if "godot" not in data:
        print("godot sheet ...", flush=True)
        load0 = loadavg()
        code, out = run([ROOT / "engines/godot/sheet.sh", "--before", a / "base", "--out", a / "review",
                         "--stream", RADIUS, "--frames", FRAMES, "--option", "water=tag:water_level"], env=env)
        data["godot"] = {"status": code, "tail": out[-2000:], "load": [load0, loadavg()]}
        save(tmp, data)
    if "conformance" not in data:
        print("conformance ...", flush=True)
        data["conformance"] = {}
        for m in (6, 5):
            code, out = run(["uv", "run", "python", "-m", "via_conformance", "run", "--via", VIA, "--milestone", m],
                            cwd=ROOT / "tools/conformance", env=env)
            tail = [s for s in out.splitlines() if re.search(r"\d+ passed", s)]
            ls = out.splitlines()  # a failing case's head line, then its first detail line
            failed = [f"{s.split()[1]}: {ls[i + 1].strip() if i + 1 < len(ls) else ''}"
                      for i, s in enumerate(ls) if re.match(r"(fail|error)\s", s)]
            data["conformance"][str(m)] = {"code": code, "summary": tail[-1] if tail else out[-300:], "failed": failed[:6]}
        save(tmp, data)
    data["load_end"] = loadavg()
    save(tmp, data)
    return data


# The views placed (sheet.sh --before <atlas base>, streamed): the atlas's own joins and two places' own bookmarks,
# walked inside atlas under their scoped names.
VIEWS = [
    ("join_market", "join: the market road at the market"),
    ("join_college", "join: the college road at the college (water under the held place)"),
    ("join_harbour", "join: the harbour road at quay's gate"),
    ("join_vale", "join: the hill trail at the vale (the trench)"),
    ("city.on_central_bridge", "city's central bridge, inside atlas"),
    ("city.on_north_bridge", "city's north bridge, inside atlas"),
    ("coast.in_cave", "coast's sea cave, inside atlas"),
    ("on_bridge", "the atlas's own creek bridge"),
]

RULINGS = [
    "R44 budgets: the world declares [budget] per chunk, no default; over is always build.over_budget (an error) naming the dominant asset.",
    "R45 export steps write only under export/<step id>/, names checked (protocol.output_file). R46 an import's own pack's assets come "
    "into the build with its instances; the parent's text never resolves them.",
    "R47 the player map's fixed rules (floors within 1.6 m merge, walls cut at 1.2 m, 4 m discovery grid, label placement). R48 a "
    "streamed walk equals the whole one (outcome, blocker, stop within two frames' pace); reload by chunk digest; the far layer is the reader's.",
    "R49 an import's kind surfaces, cut and meetings come with it (nested too). R50 water and ground at joins: the parent's water by "
    "its depth over its own ground (head view); the import's ground held 12 m round a joined place. R51 a parent keeps a change it asks "
    "of an import in its own folder.",
    "All eight dated 2026-11-17, ours (no source), open to the owner (docs/doctrine.md section 5).",
]

NOT_HOLD = [
    "The vale meets the plain in a trench: hill_trail@vale road_join {rj_vale}, the trail's last 30 m fall at {exit_out} (exit_out_grade_max); "
    "join_vale is blocked (view above). Meet it at the mouth: a lower plain, a lift taken at the exit, or a ramp the plain owns.",
    "The wild's west exit is a scarp (west_road@wild {rj_wild}; join_wild_west and wild.look_corner blocked).",
    "coast.along_cliff falls through atlas: the brought cut is single 2 m quads in a staircase along the cliff foot.",
    "market.arrive is blocked by a 1.7 m drop where the earth road meets the paved street; join_college stands 8 m short of 0.69 m of "
    "the parent's water under a held place.",
    "coast.under_deck is blocked by the brought deck; city's two bridge views and monastery.bm_deck walk off decks with no parapet.",
    "Far stand-ins pop at the load edge: 11 of 123 atlas views differ whole against streamed by more than 1% (vale aerial 6.79%; M6 close, "
    "not re-taken here).",
    "Change builds cost about half a cold build or more (numbers above); over_budget fails only at the end (the budget is checked "
    "after instances are written).",
    "Roads inside a place take the place's paint (road_painted_in {rp_in}); the atlas paints its roads with a step because a layer may "
    "not write a row on a field world.toml declares (owner's to rule).",
    "Map: the river narrows at the city's south edge; the minimap is in no viewer and atlas has no floors for test_minimap.",
    "Owed: a case building a world from its own folder; trust re-keys when a library's kinds.toml comment changes; "
    "move_market.shapes_ran is banded NOT HOLDING at every routed road, but on this fresh store only the market's own "
    "shapes ran (out of band above, the good way): its band is to re-take.",
]


def walk_of(tmp: Path, name: str) -> dict | None:
    sj = jload(tmp / "atlas/review/sheet.json") or {}
    w = next((x for x in sj.get("walks", []) if x["name"] == name), None)
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


def walk_counts(tmp: Path) -> dict:
    sj = jload(tmp / "atlas/review/sheet.json") or {}
    out: dict[str, int] = {}
    for w in sj.get("walks", []):
        o = (w.get("before") or {}).get("outcome", "absent")
        out[o] = out.get(o, 0) + 1
    return out


def rows_ok(m: dict) -> tuple[int, int]:
    ls = [s for s in m.get("lines", []) if s.startswith(("ok ", "FAIL", "BAD"))]
    return sum(1 for s in ls if s.startswith("ok ")), len(ls)


def map_rows(m: dict) -> str:
    """tools/map_render.py's own verdict line ("map rows: N in band" or the rows out of band)."""
    line = next((s for s in m.get("text", "").splitlines() if s.startswith("map rows: ")), "map rows: not checked")
    return line.removeprefix("map rows: ")


def draw(tmp: Path, data: dict) -> None:
    _, rustc = run(["rustc", "--version"])
    _, head = run(["git", "log", "-1", "--format=%h %ad", "--date=short"])
    _, porcelain = run(["git", "status", "--porcelain"])
    _, cpu = run(["sysctl", "-n", "machdep.cpu.brand_string"])
    machine = f"{platform.system()} {platform.release()} {platform.machine()}, {cpu.strip()}"
    dirty = len(porcelain.splitlines())
    a = tmp / "atlas"
    base = a / "base"
    man = jload(base / "manifest.json") or {}
    sj = jload(a / "review/sheet.json") or {}

    W, H, DPI = 2400, 12000, 100
    fig = plt.figure(figsize=(W / DPI, H / DPI), dpi=DPI, facecolor="white")
    BODY, HEAD, MONO, LH = 10, 14, "DejaVu Sans Mono", 17
    BAD, GREY, OK = "#b03a00", "#444444", "black"

    def axes(x, y, w, h):
        ax = fig.add_axes([x / W, 1 - (y + h) / H, w / W, h / H])
        ax.set_xticks([])
        ax.set_yticks([])
        return ax

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

    def image(x, y, w, p: Path, crop=None, h=None):
        if not p.exists():
            ax = axes(x, y, w, h or 200)
            ax.text(0.5, 0.5, f"not made: {p.relative_to(tmp) if p.is_relative_to(tmp) else p}", ha="center",
                    va="center", transform=ax.transAxes, color=BAD)
            return ax, y + (h or 200)
        im = mpimg.imread(p)
        if crop:
            r0, r1, c0, c1 = crop
            im = im[r0:r1, c0:c1]
        hh = h or int(w * im.shape[0] / im.shape[1])
        ax = axes(x, y, w, hh)
        ax.imshow(im, aspect="auto", interpolation="lanczos")
        return ax, y + hh

    state = f"working tree: {dirty} uncommitted paths over it" if dirty else "working tree clean"
    text(40, 22, f"via M6 sheet: whole, and handed over   branch rewrite, last commit {head.strip()}; {state}   made "
         f"{datetime.date.today().isoformat()}", size=16, mono=False, bold=True)
    text(40, 56, f"Made by docs/reports/make_m6_sheet.py on {machine}; {rustc.strip()}; Godot {sj.get('godot', '?')}. Every "
         "number is read from a build, a measure, a map render or a walk the script ran; the builds are of the working tree "
         "(the M6 close fix pass), not of the commit.", size=11, mono=False, colour=GREY)
    text(40, 78, "M6 (docs/plan.md): \"Every example joined in one map; chunks load and unload on a walk; an over-budget chunk "
         "fails and names the asset; the player map as data, per floor\". World atlas; challenge rows 4, 23, 24, 42, 53.\n"
         f"Views: engines/godot/sheet.sh on the atlas base, streamed at {RADIUS} m with the water option (1280x720, eye 1.7 m; "
         "walks per spec/build.md 15.1); each file is named at its lower right, under <keep>/atlas/review/.", size=11,
         mono=False, colour=GREY)

    # 1. the whole atlas: a plan with each import named, and the chunks against the budget
    y = 130
    text(40, y, "1. Every example joined in one map: atlas, 3200 x 2560 m, thirteen imports on the plain it owns", size=15,
         mono=False, bold=True)
    y += 36
    PW = 1480
    PH = int(PW * 2560 / 3200)
    ax = axes(40, y, PW, PH)
    hgt = np.load(base / "fields/height.npy") if (base / "fields/height.npy").exists() else None
    if hgt is not None:
        ext = man["frame"]["extent"]
        gy, gx = np.gradient(hgt.astype(float), 2.0)
        shade = np.clip(0.75 + (-gx * 0.6 + gy * 0.6) / 4.0, 0.3, 1.0)
        rgb = np.dstack([0.78 * shade, 0.84 * shade, 0.66 * shade])
        wat = base / "fields/water.npy"
        if wat.exists():
            wl = np.load(wat).astype(float)
            wet = (wl - hgt) > 0.05
            rgb[wet] = [0.55, 0.72, 0.88]
        ax.imshow(rgb, origin="lower", extent=[ext[0], ext[2], ext[1], ext[3]], interpolation="bilinear")
        feats = (jload(base / "features.json") or {}).get("features", [])
        for f in feats:
            if f.get("shape") == "path" and f.get("kind") in ("road", "trail", "street", "main_street", "freeway",
                                                             "lane", "footpath", "crossing", "viaduct", "ramp"):
                p = np.array(f["points"])
                own = "." not in f["name"]
                ax.plot(p[:, 0], p[:, 1], color="#7a3b12" if own else "#333333", lw=2.2 if own else 0.6, zorder=3)
        vec = jload(base / "export/map/vectors.json") or {}
        for r in vec.get("regions", []):
            for piece in r["pieces"]:
                ring = np.array(piece[0])
                ax.plot(ring[:, 0], ring[:, 1], color="#b03a00", lw=1.2, ls="--", zorder=4)
                cx, cy = ring[:, 0].mean(), ring[:, 1].mean()
                ax.text(cx, cy, r["name"].removeprefix("area_"), ha="center", va="center", fontsize=13, weight="bold",
                        zorder=6, bbox=dict(facecolor="white", edgecolor="#b03a00", alpha=0.85, pad=2))
        for f in feats:
            if f.get("kind") == "bookmark" and "." not in f["name"]:
                ax.plot(f["at"][0], f["at"][1], "o", ms=4, color="black", zorder=7)
        ax.set_xlim(ext[0], ext[2])
        ax.set_ylim(ext[1], ext[3])
    text(40, y + PH + 4, "Plan of the atlas base (fields/height.npy shaded; water where fields/water.npy stands > 0.05 m over it); "
         "dashed: each import's area as the map names it (vectors.json regions); brown: the atlas's own roads; grey: the "
         "places' own ways; dots: the atlas's 13 bookmarks.", size=9, mono=False, colour=GREY)

    # the chunks against the budget
    cx0, cw = 40 + PW + 30, W - 40 - (40 + PW + 30)
    chunks = man.get("chunks", [])
    budget = 100000
    try:
        import tomllib
        budget = tomllib.loads((ATLAS / "world.toml").read_text()).get("budget", {}).get("triangles", budget)
    except Exception:  # noqa: BLE001 - the panel still draws with the stated number
        pass
    nx, ny = man.get("frame", {}).get("chunks", [25, 20])
    grid = np.zeros((ny, nx))
    for c in chunks:
        grid[c["at"][1], c["at"][0]] = c.get("triangles", 0)
    hh = int(cw * ny / nx)
    text(cx0, y, f"Source triangles per 128 m chunk ({len(chunks)} chunks), against [budget] triangles = {budget:,}",
         size=12, mono=False, bold=True)
    axc = axes(cx0, y + 26, cw, hh)
    im = axc.imshow(grid / budget, origin="lower", cmap="viridis", vmin=0, vmax=1.0, aspect="auto")
    top = np.unravel_index(np.argmax(grid), grid.shape)
    axc.add_patch(plt.Rectangle((top[1] - 0.5, top[0] - 0.5), 1, 1, fill=False, ec="red", lw=2))
    for (j, i), v in np.ndenumerate(grid):
        if v > 0.5 * budget:
            axc.text(i, j, f"{v / 1000:.0f}k", ha="center", va="center", fontsize=6, color="white" if v < 0.8 * budget else "black")
    cb = fig.add_axes([(cx0) / W, 1 - (y + 26 + hh + 22) / H, cw / W, 8 / H])
    fig.colorbar(im, cax=cb, orientation="horizontal")
    cb.tick_params(labelsize=8)
    c = Col(cx0, y + 26 + hh + 50, width=96)
    tm = data["measures"].get("base", {}).get("values", {})
    c.line(f"Heaviest: chunk {list(int(v) for v in top[::-1])} with {int(grid.max()):,} triangles, "
           f"{grid.max() / budget:.0%} of the budget (measure.py tri_max {tm.get('tri_max')} at {tm.get('tri_max_at')}); "
           f"chunks over half the budget: {int((grid > 0.5 * budget).sum())}. Instances counted as the asset's triangles "
           "times its instances (R44); generated meshes not counted yet.")
    c.head("An over-budget chunk fails and names the asset", size=12)
    ob = data["atlas"].get("over_budget", {})
    obm = data["measures"].get("over_budget", {}).get("values", {})
    c.line(f"changes/over_budget.toml (chunk = 160): exit {ob.get('status')} after {ob.get('wall', float('nan')):.1f} s; "
           f"chunks over: {obm.get('over_budget_chunks')}; asset: {obm.get('over_budget_asset')}.")
    for e in ob.get("errors", [])[:2]:
        c.line(f"{e.get('severity')} {e.get('code')}: {e.get('message')}", colour=BAD)
        if e.get("hint"):
            c.line(f"  hint: {e['hint']}", colour=BAD)
        if e.get("data"):
            c.line(f"  data: {json.dumps(e['data'])[:600]}", colour=GREY)
    y = max(y + PH + 20, c.y) + 20

    # 2. views from inside atlas, streamed
    text(40, y, f"2. From inside atlas, streamed at {RADIUS} m: four joins, city's bridges, coast's cave, the creek bridge "
         "(eye height; each walk's outcome under it)", size=15, mono=False, bold=True)
    y += 36
    VW = (W - 80 - 3 * 12) // 4
    VH = int(VW * 9 / 16)
    for i, (name, cap) in enumerate(VIEWS):
        x, yy = 40 + (i % 4) * (VW + 12), y + (i // 4) * (VH + 40)
        fname = name.replace(".", "-")  # sheet.gd names an import's bookmark file <import>-<name>.png
        p = a / "review/before" / f"{fname}.png"
        ax, _ = image(x, yy, VW, p, h=VH)
        text(x + 5, yy + 5, f"{name}: {cap}", size=10, mono=False, colour="white", box="black")
        text(x + VW - 5, yy + VH - 22, f"review/before/{fname}.png", size=8, colour="white", box="black", ha="right")
        r = walk_of(tmp, name)
        text(x, yy + VH + 4, f"walk: {walk_text(r)}", size=9, colour=BAD if r and r.get("outcome") != "arrived" else OK)
    y += 2 * (VH + 40) + 4
    wc = walk_counts(tmp)
    fr = ((sj.get("frames") or {}).get("before")) or {}
    c = Col(40, y, width=270)
    c.line(f"All {sum(wc.values())} bookmarks walked streamed (sheet.json): " + ", ".join(f"{v} {k}" for k, v in sorted(wc.items()))
           + f". Streaming on the frame tour: loads {(fr.get('stream') or {}).get('loads')}, unloads "
           f"{(fr.get('stream') or {}).get('unloads')}, most loaded at once {(fr.get('stream') or {}).get('most_loaded')} of "
           f"{(fr.get('stream') or {}).get('chunks')} chunks, stalls {(fr.get('stream') or {}).get('stalls')}.")
    y = c.y + 14

    # 3. the player map as data
    text(40, y, "3. The player map as data, per floor: atlas's map (left: the map's classes, ways, footprints, areas and "
         "placed labels; right: the build's own plan) and station's floors", size=15, mono=False, bold=True)
    y += 36
    _, y = image(40, y, W - 80, a / "map_base.ground.png")
    text(48, y - 24, "tools/map_render.py <atlas base> export/map: map_base.ground.png", size=8, colour="white", box="black")
    y += 10
    HW2 = (W - 80 - 20) // 2
    _, y1 = image(40, y, HW2, tmp / "station/map.floors.png")
    text(48, y1 - 24, "tools/map_render.py <station base> export/map: map.floors.png (one plan per floor, then the section)",
         size=8, colour="white", box="black")
    # the atlas map's own drawing at its label zoom (1 px a metre), 1:1, round townland and monastery
    _, y2 = image(40 + HW2 + 20, y, HW2, a / "map_base.ground.png", crop=(110, 110 + int(HW2 * 0.75), 2000, 2000 + HW2),
                  h=int(HW2 * 0.75))
    text(48 + HW2 + 20, y + 5, "map_base.ground.png at 1:1 (its label zoom, 1 px a metre), x 2000..%d m, y %d..2450 m: "
         "labels in the boxes the map placed them in" % (2000 + HW2, 2450 - int(HW2 * 0.75)), size=9, mono=False,
         colour="white", box="black")
    y = max(y1, y2)
    mb = data["maps"].get("base", {}).get("values", {}).get("counts", {})
    st = data.get("station", {}).get("map", {}).get("values", {}).get("counts", {})
    c = Col(40, y + 6)
    c.line(f"atlas map (cell 2 m, {mb.get('tiles')} tiles): {mb.get('classes')} classes, {mb.get('ways')} ways, "
           f"{mb.get('water')} water courses, {mb.get('footprints')} footprints, {mb.get('regions')} areas, {mb.get('labels')} labels; "
           f"way_on_cells_min {mb.get('way_on_cells_min')}, water_on_cells_min {mb.get('water_on_cells_min')}, label overlaps "
           f"{mb.get('label_overlaps')}, unplaced {mb.get('labels_unplaced')}; rows: {map_rows(data['maps'].get('base', {}))}.")
    c.line(f"station map (cell 0.5 m): floors {st.get('floors')}, walls {st.get('walls')}, links {st.get('links')}, label "
           f"overlaps {st.get('label_overlaps')}; rows: {map_rows(data.get('station', {}).get('map', {}))}.")
    y = c.y + 14

    # 4. move_market before and after
    text(40, y, "4. A change: move_market (the market 40 m east, 30 m north), the player map before and after, the "
         "map tiles that changed outlined", size=15, mono=False, bold=True)
    y += 36
    mm = data["maps"].get("move_market", {}).get("values", {})
    tiles = mm.get("tiles_changed", [])
    win = (2300, 3200, 250, 900)  # metres: x0, x1, y0, y1 round the market
    HW = (W - 80 - 20) // 2
    HHt = int(HW * (win[3] - win[2]) / (win[1] - win[0]))
    for k, (folder, label) in enumerate((("base", "before (base)"), ("move_market", "after (move_market)"))):
        x = 40 + k * (HW + 20)
        ax = axes(x, y, HW, HHt)
        g = base.parent / folder / "export/map/ground.npy"
        mj = jload(base.parent / folder / "export/map/map.json") or {}
        if g.exists():
            cls = np.load(g)
            names = [cc.get("name", "") for cc in mj.get("classes", [])]
            pal = np.array([PALETTE_FOR(n) for n in names] or [(0.9, 0.9, 0.9)])
            ax.imshow(pal[cls], origin="lower", extent=[0, 3200, 0, 2560], interpolation="nearest")
            vv = jload(base.parent / folder / "export/map/vectors.json") or {}
            for f in vv.get("footprints", []):
                for piece in f.get("pieces", []):
                    ring = np.array(piece[0])
                    ax.fill(ring[:, 0], ring[:, 1], color="#8a6a50", lw=0)
            for t in tiles:
                if t.get("plan") == "ground":
                    i, j = t["at"]
                    ax.add_patch(plt.Rectangle((i * 64, j * 64), 64, 64, fill=False, ec="red", lw=1.6, zorder=5))
        ax.set_xlim(win[0], win[1])
        ax.set_ylim(win[2], win[3])
        text(x + 5, y + 5, f"{label}: export/map/ground.npy and vectors.json footprints, x {win[0]}..{win[1]} m, y "
             f"{win[2]}..{win[3]} m; red: the {len(tiles)} changed tiles", size=10, mono=False, colour="white", box="black")
    y += HHt + 8
    c = Col(40, y)
    ex = {}
    for t in tiles:
        for wh in t.get("why", []):
            key = wh.split(":")[0] if wh.startswith("cells") else wh
            ex[key] = ex.get(key, 0) + 1
    mc = mm.get("counts", {})
    c.line(f"tiles_changed {mc.get('tiles_changed')}, unexplained {mc.get('tiles_unexplained')}, cells only "
           f"{mc.get('tiles_cells_only')}; rows: {map_rows(data['maps'].get('move_market', {}))}. What the "
           f"tiles name (count of tiles): " + ", ".join(f"{k} {v}" for k, v in sorted(ex.items(), key=lambda kv: -kv[1])[:14]) + ".")
    mmv = data["measures"].get("move_market", {}).get("values", {})
    c.line(f"move_market measures: errors {mmv.get('errors')}, exit_gap {mmv.get('exit_gap')}, shapes_ran "
           f"{mmv.get('shapes_ran')}, ran_imports {mmv.get('ran_imports')}.")
    y = c.y + 14

    # 5. numbers
    c = Col(40, y)
    c.head("Numbers (this run)")
    b = data["atlas"].get("base", {})
    cw_ = b.get("wall", float("nan"))
    c.line(f"Chunks {len(chunks)} (25 x 20 of 128 m); max triangles per chunk {int(grid.max()):,} against the budget "
           f"{budget:,} ({grid.max() / budget:.0%}); bookmarks in atlas {len((sj.get('walks') or []))} (13 its own).")
    c.line(f"{'build':<16}{'exit':>5}{'wall s':>9}{'% cold':>8}{'GB':>6}{'nodes ran':>11}  rows (measure.py --check)", bold=True)
    for name in ["base", "unchanged"] + CHANGES:
        bb = data["atlas"].get(name)
        if not bb:
            c.line(f"{name:<16} not built", colour=BAD)
            continue
        m = data["measures"].get(name if name != "unchanged" else "", {})
        ok, n = rows_ok(m) if m else (0, 0)
        rows = f"{ok}/{n}" if n else "-"
        bad = (bb["status"] and name not in FAILS_BY_DESIGN) or ok < n
        c.line(f"{name:<16}{bb['status']:>5}{bb['wall']:>9.1f}{bb['wall'] / cw_:>8.0%}{bb['rss']:>6.2f}"
               f"{bb['totals'].get('ran', '?'):>11}  {rows}" + ("  (exit 1 by design)" if name in FAILS_BY_DESIGN and bb['status'] == 1 else ""),
               colour=BAD if bad else OK)
    for name in ["base"] + CHANGES:
        for s_ in data["measures"].get(name, {}).get("lines", []):
            if s_.startswith(("FAIL", "BAD")):
                c.line(f"out of band: {s_}", colour=BAD)
    for nm in ("base", "move_market"):
        c.line(f"map rows {nm}: {map_rows(data['maps'].get(nm, {}))}")
    sb = data.get("station", {}).get("build", {})
    c.line(f"station (alone, its own store): exit {sb.get('status')}, {sb.get('wall', float('nan')):.1f} s, nodes ran "
           f"{sb.get('totals', {}).get('ran')} of {sb.get('totals', {}).get('nodes')}; map rows: "
           f"{map_rows(data.get('station', {}).get('map', {}))}")
    fps, fms = fr.get("fps") or {}, fr.get("frame_ms") or {}
    fmt = lambda v: f"{v:.1f}" if isinstance(v, (int, float)) else "?"  # noqa: E731
    c.line(f"Frame tour (sheet.gd _frames, streamed at {RADIUS} m): {fr.get('frames')} frames in {fr.get('seconds')} s at "
           f"{fr.get('speed')} m/s over {fmt(fr.get('toured_m'))} m; fps median {fmt(fps.get('median'))}, slowest 1% "
           f"{fmt(fps.get('low_1pct'))}, min {fmt(fps.get('min'))}; frame ms median {fmt(fms.get('median'))}, p99 "
           f"{fmt(fms.get('p99'))}; streamer work ms p99 {fmt(((fr.get('stream') or {}).get('work_ms') or {}).get('p99'))}.")
    for m, cf in (data.get("conformance") or {}).items():
        c.line(f"Conformance milestone {m} (tools/conformance, release via): {cf.get('summary')}"
               + (f"; failed: {'; '.join(x.strip()[:160] for x in cf.get('failed', []))}" if cf.get("failed") else ""),
               colour=BAD if cf.get("code") else OK)
    if any("graph/epoch" in x for cf in (data.get("conformance") or {}).values() for x in cf.get("failed", [])):
        c.line("graph/epoch sets VIA_TEST_EPOCH_OFFSET, which test (debug) builds read only (its case.toml says so): on this "
               "release via nothing re-runs, so the case fails by its protocol, not by the engine. The M6 close fix pass's "
               "debug run had it passing (912 of 912 run, M6).", colour=GREY)
    g = data.get("godot", {})
    c.line("Protocol: release via and via-terrain of the working tree, default threads, VIA_LIB = the repo's lib, python = "
           "the SDK venv (3.12); atlas in one fresh store: base cold, the same again into the same directory (unchanged), "
           "then move_market, move_square, forgotten_exit, over_budget, each --against the base, in that order; one run each, "
           "so no spread is claimed. Laptop shared with other agents: load average "
           f"{data.get('load_start', '?')} at the start, {data.get('load_end', '?')} at the end; per build "
           + "; ".join(f"{k} {v.get('load', ['?'])[0].split()[0]}" for k, v in data["atlas"].items()) + ". "
           f"Godot render and frame tour: sheet.sh exit {g.get('status')}, load {g.get('load', ['?'])[0]} at its start, "
           "1280x720, no frame cap, vsync off, the first second not counted; fps on a shared laptop says where to look, "
           "not a speed.", colour=GREY)
    c.head("What does not hold (TODO.md \"From M6 close\", to M7)")
    bv = data["measures"].get("base", {}).get("values", {})
    rj = bv.get("road_join", {})
    for s in NOT_HOLD:  # the numbers in it are this run's (measure.py on the base)
        c.line("- " + s.format(rj_vale=rj.get("hill_trail@vale"), rj_wild=rj.get("west_road@wild"),
                               exit_out=bv.get("exit_out_grade_max"), rp_in=bv.get("road_painted_in")), colour=BAD)
    c.head("Rulings added in M6 (docs/doctrine.md section 5, R44 to R51)")
    for s in RULINGS:
        c.line(s)
    bottom = c.y + 30

    fig.canvas.draw()
    img = np.asarray(fig.canvas.buffer_rgba())[: int(bottom)]
    plt.imsave(OUT, img)
    print(f"wrote {OUT}  ({W}x{int(bottom)}; renders in {tmp})")


def PALETTE_FOR(name: str) -> tuple[float, float, float]:
    """A colour per map class, by its name (the map's classes say what each is, lib/map README)."""
    group, _, n = name.lower().partition(":")
    if group == "water":
        return (0.47, 0.67, 0.86)
    if group == "way":
        return (0.30, 0.28, 0.26) if n != "bridge" else (0.55, 0.20, 0.10)
    if group == "footprint":
        return (0.59, 0.47, 0.39)
    if group == "none":
        return (0.93, 0.91, 0.87)
    return {"sand": (0.91, 0.84, 0.63), "rock": (0.65, 0.63, 0.61), "gravel": (0.72, 0.70, 0.66),
            "setts": (0.62, 0.58, 0.55), "flags": (0.78, 0.76, 0.72), "earth": (0.76, 0.66, 0.50),
            "litter": (0.55, 0.62, 0.42), "meadow": (0.66, 0.80, 0.52)}.get(n, (0.73, 0.84, 0.59))


def main() -> None:
    keep = sys.argv[sys.argv.index("--keep") + 1] if "--keep" in sys.argv else None
    tmp = Path(keep) if keep else Path(tempfile.mkdtemp(prefix="via-m6-sheet-"))
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
    draw(tmp, data)


if __name__ == "__main__":
    main()
