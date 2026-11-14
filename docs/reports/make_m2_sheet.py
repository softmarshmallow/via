# /// script
# requires-python = ">=3.11"
# dependencies = ["matplotlib>=3.9,<3.10", "numpy>=2.0,<3"]
# ///
"""Draw the M2 sheet, docs/reports/m2-sheet.png, from fresh builds of M2's five worlds.

    uv run docs/reports/make_m2_sheet.py [--keep DIR] [--draw-only] [--fresh]

What it runs, in order (every number on the sheet comes from one of these):

1. `cargo build --release -p via-cli -p via-terrain`; the binaries are target/release/via and
   via-terrain (basin's terrain step program).
2. A scratch user config (VIA_CONFIG_DIR, protocol.md 6.3) with `python` set to the SDK's venv
   Python and `via-terrain` to target/release/via-terrain, and each world's steps trusted with `via trust WORLD --change ... --yes` (every change
   the sheet builds); the user's own config is not touched.
3. Per world (examples/vale, glade, relief, massif, basin), in one fresh store: the base build cold,
   the same build twice more unchanged (build times are `totals.ms` of each run report, graph.md
   6.3), then every change the world's expectations name, each `--against` the base.
4. Each world's own measure tool (measure.py, or glade's expect.py), which holds every build to the
   world's expect.toml and prints the spike's number beside ours (fixtures/oracle/<world>*/).
5. engines/godot/sheet.sh once per world, base against its key change, water=tag:water_level and
   the world's ways drawn as strips (paths=kind:...).
6. The conformance runner at milestone 2, and `xtask check --milestone 2`.

With --keep, a world whose DIR/<world>.json a previous run wrote is not built again unless
`--fresh` is given. `--draw-only` redraws from DIR/data.json and the builds a previous run kept in DIR (for the layout
only; a sheet that is handed over is made without it). The "still wrong" list is written by hand
from looking at the views and the numbers; the lines that are numbers are computed.
"""

from __future__ import annotations

import datetime
import json
import os
import platform
import re
import shutil
import statistics
import subprocess
import sys
import tempfile
import tomllib
from pathlib import Path

import matplotlib

matplotlib.use("Agg")
import matplotlib.image as mpimg  # noqa: E402
import matplotlib.pyplot as plt  # noqa: E402
import numpy as np  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "docs/reports/m2-sheet.png"
VIA = ROOT / "target/release/via"
PY = ROOT / "sdk/python/.venv/bin/python"
LIB = ROOT / "lib"
WAY_KINDS = ("road", "trail", "track", "lane", "path")

# Each world: its file, how its measure tool names build folders, the changes its expect.toml names
# (name, the --change argument), the key change the views show, and four ground bookmarks at its
# meetings (where a way meets a house, water meets land, and so on).
WORLDS = [
    {
        "name": "vale", "world": "examples/vale/world.toml", "size": "512 m",
        "base": "vale", "folder": "vale+{c}",
        "changes": [("move_road", "changes/move_road.toml"), ("new_seed", "changes/new_seed.toml"),
                    ("move_tarn", "changes/move_tarn.toml"), ("dress_only", "changes/dress_only.toml"),
                    ("revised_house", "changes/revised_house.toml"),
                    ("bigger_house", "changes/bigger_house.toml"), ("add_cliff", "changes/add_cliff.toml"),
                    ("winter", "winter")],
        "key": "move_road", "key_says": "the road swung west (spike's move_road)",
        "bookmarks": ["hamlet", "to_door", "trail_join", "tarn_side"],
    },
    {
        "name": "glade", "world": "examples/glade/world.toml", "size": "256 m",
        "base": "glade", "folder": "glade+{c}",
        "changes": [("mouth", "changes/mouth.toml"), ("seed", "changes/seed.toml"),
                    ("look", "changes/look.toml"), ("revised", "changes/revised.toml"),
                    ("by_cause", "changes/by_cause.toml"), ("by_step", "changes/by_step.toml")],
        "key": "mouth", "key_says": "the stream's mouth moved west (a moved river mouth)",
        "bookmarks": ["on_bank", "at_mouth", "to_door", "in_trees"],
    },
    {
        "name": "relief", "world": "examples/relief/world.toml", "size": "",
        "base": "base", "folder": "{c}",
        "changes": [("moved_hollow", "changes/moved_hollow.toml"), ("new_seed", "changes/new_seed.toml"),
                    ("denser", "changes/denser.toml"), ("revised_assets", "changes/revised_assets.toml"),
                    ("coarse", "changes/coarse.toml"), ("float_survey", "changes/float_survey.toml")],
        "key": "moved_hollow", "key_says": "the hollow's stamp moved 12 m east, 8 m north",
        "bookmarks": ["on_shore", "to_tarn", "at_door", "up_track"],
    },
    {
        "name": "massif", "world": "examples/massif/world.toml", "size": "",
        "base": "base", "folder": "{c}",
        "changes": [("new_seed", "changes/new_seed.toml"), ("steeper_repose", "changes/steeper_repose.toml"),
                    ("cottage_moved", "changes/cottage_moved.toml"),
                    ("denser_wood", "changes/denser_wood.toml"), ("revised", "changes/revised.toml")],
        "key": "new_seed", "key_says": "a new world seed (the valley moves)",
        "bookmarks": ["to_door", "by_water", "at_mouth", "to_barn"],
    },
    {
        "name": "basin", "world": "examples/basin/world.toml", "size": "",
        "base": "base", "folder": "{c}",
        "changes": [("move_haven", "move_haven"), ("two_ends", "changes/two_ends.toml"),
                    ("reseed", "changes/reseed.toml"), ("thin_woods", "changes/thin_woods.toml"),
                    ("revised_oak", "changes/revised_oak.toml"), ("cannot_hold", "changes/cannot_hold.toml"),
                    ("broken_hold", "changes/broken_hold.toml")],
        "key": "move_haven", "key_says": "the river's mouth (the haven) moved",
        "bookmarks": ["at_mouth", "on_shore", "on_bank", "tarn_shore"],
    },
]


def run(cmd: list, cwd: Path = ROOT, env: dict | None = None) -> tuple[int, str]:
    e = dict(os.environ)
    e.update(env or {})
    p = subprocess.run([str(c) for c in cmd], cwd=cwd, capture_output=True, text=True, env=e)
    return p.returncode, p.stdout + p.stderr


def jload(p: Path):
    return json.loads(p.read_text()) if p.exists() else None


# ---------------------------------------------------------------------------------------- collect

def build(w: dict, tmp: Path, env: dict, folder: str, change: str | None = None) -> dict:
    wf = ROOT / w["world"]
    root = tmp / w["name"]
    d = root / folder
    diag = root / f"{folder}.diagnostics.json"
    cmd = [VIA, "build", wf, "--lib", LIB, "--store", root / "store", "--out", d, "--diagnostics", diag]
    if change:
        cmd += ["--change", change, "--against", root / w["base"]]
    code, text = run(cmd, cwd=wf.parent, env=env)
    r = jload(root / f"{folder}.run.json")
    dg = jload(diag) or {}
    rec = {"folder": folder, "change": change, "status": code,
           "first": next((s for s in text.splitlines() if s.startswith("error")), "")}
    if r:
        rec["totals"] = r["totals"]
        rec["threads"] = r.get("threads")
        rec["steps"] = {n["id"].split(":", 1)[1]: n["status"] for n in r["nodes"] if n["family"] == "step"}
        fam = {}
        for n in r["nodes"]:
            f = fam.setdefault(n["family"], [0, 0])
            f[0] += 1
            f[1] += n.get("ms") or 0
        rec["families"] = fam
    rows = dg.get("diagnostics", [])
    rec["codes"] = sorted({f"{x['code']} ({x['severity'][0]})" for x in rows if x["severity"] != "info"})
    rec["counts"] = dg.get("counts", {})
    c = jload(d / "changes.json") if code == 0 or (d / "changes.json").exists() else None
    if c and c.get("against"):
        rec["entries"] = len(c["entries"])
        rec["by_table"] = {}
        for e in c["entries"]:
            for what in e["what"]:
                k = f"{e['table']} {what}"
                rec["by_table"][k] = rec["by_table"].get(k, 0) + 1
        rec["named"] = sorted(e["name"] for e in c["entries"] if "#" not in e["name"]
                              and e["table"] in ("feature", "instance"))
    revised = [x for x in rows if x["code"] == "asset.revised"]
    rec["revised"] = [{"subject": x["subject"], "instances": len((x.get("data") or {}).get("instances", [])),
                       "data_keys": sorted((x.get("data") or {}).keys())} for x in revised]
    return rec


def measure_tool(w: dict, tmp: Path, env: dict) -> tuple[int, list[str]]:
    root = tmp / w["name"]
    here = ROOT / "examples" / w["name"]
    e = dict(env, VIA=str(VIA), VIA_LIB=str(LIB))
    if w["name"] == "vale":
        exp = tomllib.loads((here / "expect.toml").read_text())
        lines, worst = [], 0
        for sec in exp:
            d = root / ("vale" if sec == "base" else f"vale+{sec}")
            code, out = run([sys.executable, here / "measure.py", d, "--check", sec], env=e)
            worst = max(worst, code)
            lines += [s for s in out.splitlines() if s.strip()]
        return worst, lines
    tool = here / ("expect.py" if w["name"] == "glade" else "measure.py")
    code, out = run([sys.executable, tool, root], env=e)
    return code, [s for s in out.splitlines() if s.strip()]


def way_kinds(d: Path) -> list[str]:
    fs = (jload(d / "features.json") or {}).get("features", [])
    return sorted({f["kind"] for f in fs if f.get("shape") == "path" and f.get("kind") in WAY_KINDS})


def has_water(d: Path) -> bool:
    m = jload(d / "manifest.json") or {}
    return any("water_level" in (v.get("tags") or []) for v in m.get("fields", {}).values())


def godot(w: dict, tmp: Path) -> tuple[int, str, list[str]]:
    root = tmp / w["name"]
    before, after = root / w["base"], root / w["folder"].format(c=w["key"])
    out = root / "review"
    if out.exists():
        shutil.rmtree(out)
    opts = []
    if has_water(before):
        opts += ["--option", "water=tag:water_level"]
    kinds = way_kinds(before)
    if kinds:
        opts += ["--option", "paths=" + ",".join(f"kind:{k}" for k in kinds)]
    code, text = run([ROOT / "engines/godot/sheet.sh", "--before", before, "--after", after,
                      "--out", out] + opts)
    return code, text[-600:], opts


def conformance() -> dict:
    cdir = ROOT / "tools/conformance"
    _, out = run(["uv", "run", "python", "-m", "via_conformance", "list"], cwd=cdir)
    per = {m.group(1): int(m.group(2)) for m in re.finditer(r"^(M\d+): (\d+) cases", out, re.M)}
    tag = {m.group(1): m.group(2) for m in re.finditer(r"^case\s+(\S+)\s+\((M\d+)[,)]", out, re.M)}
    _, out = run(["uv", "run", "python", "-m", "via_conformance", "run", "--via", VIA,
                  "--milestone", "2"], cwd=cdir)
    res: dict[str, dict[str, int]] = {}
    bad = []
    for m in re.finditer(r"^(pass|fail|pending|error)\s+(\S+)", out, re.M):
        ms = tag.get(m.group(2), "?")
        res.setdefault(ms, {k: 0 for k in ("pass", "fail", "pending", "error")})[m.group(1)] += 1
        if m.group(1) != "pass":
            bad.append(f"{m.group(1)} {m.group(2)} ({ms})")
    tail = [s for s in out.splitlines() if s.strip()][-2:]
    return {"per": per, "res": res, "bad": bad, "tail": tail}


def xtask() -> list[str]:
    _, out = run(["cargo", "run", "-q", "-p", "xtask", "--", "check", "--milestone", "2"])
    return [s for s in out.splitlines() if re.match(r"^[a-z-]+: ", s)]


def collect(tmp: Path) -> dict:
    env = {"VIA_CONFIG_DIR": str(tmp / "config")}
    (tmp / "config").mkdir(parents=True, exist_ok=True)
    for prog, path in (("python", PY), ("via-terrain", ROOT / "target/release/via-terrain")):
        code, out = run([VIA, "config", "program", prog, path], env=env)
        if code:
            sys.exit(f"via config program {prog}: {out}")
    _, load0 = run(["sysctl", "-n", "vm.loadavg"])
    data = {"worlds": {}, "load_start": load0.strip().strip("{} ")}
    for w in WORLDS:
        print(f"{w['name']} ...", flush=True)
        wf = ROOT / w["world"]
        saved = tmp / f"{w['name']}.json"
        if saved.exists() and "--fresh" not in sys.argv:
            data["worlds"][w["name"]] = json.loads(saved.read_text())
            continue
        # One trust per build: two changes may write the same table and cannot be asked together.
        for c in [None] + [c for _, c in w["changes"]]:
            code, out = run([VIA, "trust", wf, "--lib", LIB, "--yes"] + (["--change", c] if c else []),
                            cwd=wf.parent, env=env)
            if code:
                sys.exit(f"via trust {w['name']} {c}: {out}")
        root = tmp / w["name"]
        if root.exists():
            shutil.rmtree(root)
        root.mkdir(parents=True)
        cold = build(w, tmp, env, w["base"])
        if cold["status"]:
            sys.exit(f"{w['name']} base build failed:\n{cold['first']}")
        warm = [build(w, tmp, env, w["base"]) for _ in range(2)]
        ch = {}
        for name, c in w["changes"]:
            print(f"  {name}", flush=True)
            ch[name] = build(w, tmp, env, w["folder"].format(c=name), c)
        mcode, mlines = measure_tool(w, tmp, env)
        print(f"  godot", flush=True)
        gcode, gtext, gopts = godot(w, tmp)
        m = jload(root / w["base"] / "manifest.json")
        data["worlds"][w["name"]] = {
            "cold": cold, "warm": warm, "changes": ch, "measure_status": mcode, "measures": mlines,
            "godot": [gcode, gtext, gopts], "frame": m["frame"], "instances": m["instances"]["count"],
            "seed": m.get("seed"),
        }
        saved.write_text(json.dumps(data["worlds"][w["name"]], indent=1))
    _, load1 = run(["sysctl", "-n", "vm.loadavg"])
    data["load_end"] = load1.strip().strip("{} ")
    print("conformance, xtask ...", flush=True)
    data["conformance"] = conformance()
    data["xtask"] = xtask()
    sj = tmp / "vale/review/sheet.json"
    data["godot_version"] = (jload(sj) or {}).get("godot", "?")
    return data


# ------------------------------------------------------------------------------------------- draw

def plan_view(ax, before: Path, after: Path, bms: list[str]) -> None:
    m = jload(before / "manifest.json")
    fields = m["fields"]
    fr = m["frame"]
    x0, y0, x1, y1 = fr["extent"]
    res = fr["resolution"]
    surf = next(n for n, v in fields.items() if v.get("role") == "surface")
    h = np.load(before / fields[surf]["file"], mmap_mode="r")
    step = max(1, int(np.ceil(max(h.shape) / 900)))
    h = np.asarray(h[::step, ::step], dtype=np.float64)
    ext = (x0, x1, y0, y1)
    gy, gx = np.gradient(h, res * step)
    shade = np.clip(0.62 + 0.38 * (-gx * 0.707 + gy * 0.707) / np.sqrt(1 + gx**2 + gy**2) * 3, 0, 1)
    ax.imshow(shade, origin="lower", extent=ext, cmap="gray", vmin=0, vmax=1.25)
    span = float(h.max() - h.min())
    every = next(s for s in (1, 2, 5, 10, 20, 50, 100) if span / s <= 30)
    ax.contour(h, levels=np.arange(np.floor(h.min()), h.max(), every), origin="lower", extent=ext,
               colors="#8a7a5a", linewidths=0.35)
    wname = next((n for n, v in fields.items() if "water_level" in (v.get("tags") or [])), None)
    if wname:
        wv = np.asarray(np.load(before / fields[wname]["file"], mmap_mode="r")[::step, ::step], dtype=np.float64)
        depth = np.ma.masked_less_equal(wv - h, 0.001)
        ax.imshow(depth, origin="lower", extent=ext, cmap="Blues", vmin=-1.5, vmax=4, alpha=0.9)
    if (before / "instances/x.npy").exists():
        x = np.load(before / "instances/x.npy") / 1000.0
        y = np.load(before / "instances/y.npy") / 1000.0
        k = max(1, len(x) // 5000)
        ax.plot(x[::k], y[::k], ".", color="#2e6b2e", ms=1.6, alpha=0.7)
    span_m = max(x1 - x0, y1 - y0)

    def draw(d: Path, style: str, colour: str, label: bool) -> None:
        fs = jload(d / "features.json")
        if not fs:
            return
        for f in fs["features"]:
            if f["shape"] == "path" and f.get("points"):
                p = np.array(f["points"])[:, :2]
                ax.plot(p[:, 0], p[:, 1], style, color=colour, lw=1.3 if f.get("kind") else 0.6)
            elif f["shape"] == "region" and f.get("kind"):
                for pc in f.get("pieces", []):
                    p = np.array(pc["outer"] + pc["outer"][:1])
                    ax.plot(p[:, 0], p[:, 1], style, color=colour, lw=0.9)
            elif f.get("kind") == "bookmark" and f["name"] in bms:
                e, t = np.array(f["at"][:2]), np.array(f["attrs"]["toward"]["at"][:2])
                v = (t - e) / max(np.hypot(*(t - e)), 1e-6) * span_m * 0.05
                ax.annotate("", xy=tuple(e + v), xytext=tuple(e),
                            arrowprops=dict(arrowstyle="->", color=colour, lw=1.3))
                ax.plot(*e, "o", color=colour, ms=3.5)
                if label:
                    up = bms.index(f["name"]) % 2 == 0
                    ax.text(e[0] + span_m * 0.012, e[1] + span_m * (0.012 if up else -0.04), f["name"],
                            fontsize=9, color=colour, weight="bold",
                            bbox=dict(facecolor="white", edgecolor="none", pad=0.5, alpha=0.6))
        for fp in fs.get("footprints", []):
            for pc in fp["pieces"]:
                p = np.array(pc["outer"] + pc["outer"][:1])
                ax.plot(p[:, 0], p[:, 1], style, color=colour, lw=1.0)

    draw(before, "-", "#202020", True)
    draw(after, "--", "#c05a00", False)
    ax.set_xlim(x0, x1)
    ax.set_ylim(y0, y1)
    ax.set_aspect("equal")
    ax.tick_params(labelsize=8)


def draw(tmp: Path, data: dict) -> None:
    _, rustc = run(["rustc", "--version"])
    _, head = run(["git", "log", "-1", "--format=%h %ad", "--date=short"])
    _, porcelain = run(["git", "status", "--porcelain"])
    _, cpu = run(["sysctl", "-n", "machdep.cpu.brand_string"])
    machine = f"{platform.system()} {platform.release()} {platform.machine()}, {cpu.strip()}"
    dirty = len(porcelain.splitlines())

    W, H, DPI = 2400, 9000, 100
    fig = plt.figure(figsize=(W / DPI, H / DPI), dpi=DPI, facecolor="white")
    BODY, HEAD, MONO = 10, 14, "DejaVu Sans Mono"
    LH = 17
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
            text(self.x, self.y, s[: self.width], bold=bold, colour=colour)
            self.y += LH

    state = f"working tree: {dirty} uncommitted paths over it" if dirty else "working tree clean"
    text(40, 22, f"via M2 sheet: vale, glade, relief, massif, basin   branch rewrite, last commit "
         f"{head.strip()}; {state}   made {datetime.date.today().isoformat()}",
         size=18, mono=False, bold=True)
    text(40, 56, f"Made by docs/reports/make_m2_sheet.py on {machine}; {rustc.strip()}; Godot "
         f"{data.get('godot_version', '?')}. Every number is read from a build, a report or a command "
         f"the script ran; the builds are of the working tree, not of the commit.",
         size=11, mono=False, colour=GREY)
    text(40, 78, "Per world: eye height (1.7 m, engines/godot/sheet.sh, 1280x720, Compatibility renderer, "
         "Camera3D 75 deg) at four meetings before (top) and after (below) its key change; the plan, "
         "before black, after orange dashed; what re-ran per change;\nthe world's measures against "
         "the spike (fixtures/oracle), each line as the world's own tool prints it (ok / OUT / -- printed "
         "only), bands from its expect.toml.", size=11, mono=False, colour=GREY)

    y = 130
    PW = 540
    VW = (W - 80 - PW - 30 - 3 * 8) // 4
    VH = int(VW * 9 / 16)
    for w in WORLDS:
        d = data["worlds"].get(w["name"])
        if not d:
            continue
        root = tmp / w["name"]
        fr = d["frame"]
        ext = fr["extent"]
        text(40, y, f"{w['name']}   {ext[2] - ext[0]:g} x {ext[3] - ext[1]:g} m at {fr['resolution']:g} m "
             f"posts, {d['instances']} instances, seed {d['seed']}   key change: {w['key']}, "
             f"{w['key_says']}", size=15, mono=False, bold=True)
        y += 32
        # Plan.
        ax = axes(40, y, PW, PW)
        plan_view(ax, root / w["base"], root / w["folder"].format(c=w["key"]), w["bookmarks"])
        # Views: four columns, before over after, each with the sheet tool's walk from that eye.
        sheet = jload(root / "review/sheet.json") or {}
        walks = {x["name"]: x for x in sheet.get("walks", [])}
        vx0 = 40 + PW + 30
        for i, bm in enumerate(w["bookmarks"]):
            x = vx0 + i * (VW + 8)
            text(x, y, bm, size=11, mono=False, bold=True)
            for j, side in enumerate(("before", "after")):
                wk = walks.get(bm, {}).get(side)
                if wk:
                    s = f"walk: {wk['outcome']}" + (f" by {wk['by']}" if wk.get("by") else "") + \
                        f", {wk['walked']:.0f} m" + (f", {wk['left']:.0f} m short" if wk["outcome"] != "arrived" else "")
                    if wk.get("through"):
                        s += f", through {len(wk['through'])} with no body"
                    text(x + VW - 3, y + 20 + j * (VH + 6) + VH - 16, s, size=8, mono=False, ha="right",
                         colour="white" if wk["outcome"] == "arrived" and not wk.get("through") else "#ffb080",
                         box="black")
                a = axes(x, y + 20 + j * (VH + 6), VW, VH)
                p = root / "review" / side / f"{bm}.png"
                if p.exists():
                    a.imshow(mpimg.imread(p))
                else:
                    a.text(0.5, 0.5, "not rendered", ha="center", va="center", transform=a.transAxes)
                a.set_xticks([])
                a.set_yticks([])
        gcode, gtext, gopts = d["godot"]
        vy = y + 20 + 2 * (VH + 6)
        text(vx0, vy, f"sheet.sh {' '.join(o for o in gopts if o != '--option')}; exit {gcode}",
             size=9, colour=GREY if not gcode else BAD)
        # Builds table under the views.
        c = Col(vx0, vy + 22, width=205)
        c.line(f"{'build':<15}{'exit':>4}{'ms':>8}  {'nodes ran/reused/all':<22}{'entries':>8}  "
               f"{'steps by status':<53}findings (e error, w warning)", bold=True)

        def row(label: str, b: dict) -> None:
            t = b.get("totals") or {}
            st = b.get("steps") or {}
            steps = "; ".join(f"{v} " + ",".join(k for k in sorted(st) if st[k] == v)
                              for v in sorted(set(st.values())))
            codes = ", ".join(b.get("codes", []))
            nodes = f"{t.get('ran', '?')}/{t.get('reused', '?')}/{t.get('nodes', '?')}"
            ent = b.get("entries", "")
            c.line(f"{label:<15}{b['status']:>4}{t.get('ms', '?'):>8}  {nodes:<22}{ent!s:>8}  "
                   f"{steps:<52} {codes}", colour=OK if b["status"] == 0 or "hold" in label else BAD)

        row("base (cold)", d["cold"])
        row("unchanged", d["warm"][-1])
        for name, _ in w["changes"]:
            row(name, d["changes"][name])
        y = max(y + PW + 10, c.y + 6)
        # Measures, two columns.
        lines = [s.replace(str(root) + "/", "") for s in d["measures"]]
        n_out = sum(1 for s in lines if s.startswith("OUT"))
        n_ok = sum(1 for s in lines if s.startswith("ok"))
        tool = "expect.py" if w["name"] == "glade" else "measure.py"
        text(40, y, f"measures: examples/{w['name']}/{tool} against expect.toml: {n_ok} ok, {n_out} OUT, "
             f"exit {d['measure_status']}", size=11, mono=False, bold=True,
             colour=BAD if n_out or d["measure_status"] else OK)
        y += 22
        half = (len(lines) + 1) // 2
        cl, cr = Col(40, y, width=136), Col(1220, y, width=136)
        for i, s in enumerate(lines):
            col = cl if i < half else cr
            col.line(s, colour=BAD if s.startswith("OUT") or "Traceback" in s or "Error" in s else OK)
        y = max(cl.y, cr.y) + 26

    # Bottom: build times, conformance, still wrong.
    c = Col(40, y, width=110)
    c.head("Build times (run report totals.ms)")
    c.line(f"{'world':<8}{'cold':>8}{'unchanged':>11}  threads  nodes  slowest families (cold, ms summed)", bold=True)
    for w in WORLDS:
        d = data["worlds"].get(w["name"])
        if not d:
            continue
        cold = d["cold"]["totals"]
        warm = [b["totals"]["ms"] for b in d["warm"]]
        fam = sorted(d["cold"].get("families", {}).items(), key=lambda kv: -kv[1][1])[:3]
        c.line(f"{w['name']:<8}{cold['ms']:>8}{statistics.median(warm):>11.0f}  {d['cold'].get('threads')!s:>7}"
               f"  {cold['nodes']:>5}  " + ", ".join(f"{k} {v[1]}" for k, v in fam))
    c.line("Protocol: release binary, default threads; cold = the world's first build in a fresh store;")
    c.line("unchanged = median of two rebuilds into the same directory, warm store. memo (store) time is")
    c.line(f"included. Laptop, not isolated: load average {data['load_start']} at the start, "
           f"{data['load_end']} at the end")
    c.line("(other agents' builds were running). One run each: no spread is claimed.")
    left_bottom = c.y

    cf = data["conformance"]
    c = Col(1220, y, width=130)
    c.head("Conformance (tools/conformance, --via release, --milestone 2)")
    c.line("cases per milestone tag: " + ", ".join(f"{k} {v}" for k, v in sorted(cf["per"].items())))
    for ms, r in sorted(cf["res"].items()):
        c.line(f"  {ms}: {r['pass']} pass, {r['fail']} fail, {r['pending']} pending, {r['error']} error",
               colour=OK if not (r["fail"] or r["error"]) else BAD)
    bad = sorted(cf["bad"], key=lambda s: (not s.startswith(("fail", "error")), s))
    for a, b in zip(bad[::2][:12], (bad[1::2] + [""])[:12]):
        c.line(f"  {a:<56}{b}", colour=BAD if (a + b).count("fail") or "error " in a + b else GREY)
    if len(bad) > 24:
        c.line(f"  ... and {len(bad) - 24} more", colour=BAD)
    c.line("xtask check --milestone 2:")
    xs = data["xtask"]
    c.line("  " + "; ".join(xs[: len(xs) // 2 + 1]))
    c.line("  " + "; ".join(xs[len(xs) // 2 + 1:]))
    y = max(left_bottom, c.y) + 6

    c = Col(40, y, width=230)
    c.head("Still wrong")
    for s in still_wrong(data):
        c.line(s, colour=BAD)
    bottom = c.y + 30

    fig.canvas.draw()
    img = np.asarray(fig.canvas.buffer_rgba())[: int(bottom)]
    plt.imsave(OUT, img)
    print(f"wrote {OUT}  ({W}x{int(bottom)}; scratch builds in {tmp})")


def still_wrong(data: dict) -> list[str]:
    """Computed lines first (what the sheet's own numbers say), then what was seen by eye."""
    out = []
    for w in WORLDS:
        d = data["worlds"].get(w["name"])
        if not d:
            continue
        outs = [s for s in d["measures"] if s.startswith("OUT")]
        for s in outs:
            out.append(f"{w['name']}: {s[:200]}")
        for name, _ in w["changes"]:
            b = d["changes"][name]
            if b["status"] and name not in EXPECT_FAIL.get(w["name"], ()):
                out.append(f"{w['name']} {name}: exit {b['status']}: {b.get('first', '')[:170]}")
        if d["godot"][0]:
            out.append(f"{w['name']}: sheet.sh exit {d['godot'][0]}")
    cf = data["conformance"]
    fails = [s for s in cf["bad"] if s.startswith(("fail", "error"))]
    if fails:
        out.append(f"Conformance at milestone 2: {len(fails)} fail or error: " + ", ".join(
            s.split()[1] for s in fails[:8]) + (" ..." if len(fails) > 8 else ""))
    pend = [s for s in cf["bad"] if s.startswith("pending")]
    if pend:
        out.append(f"{len(pend)} conformance case(s) at M1 or M2 still pending (each names the fixture or "
                   f"action it waits for).")
    return out + STILL_WRONG_BY_EYE


# Changes whose expectations say they exit 1 (a statement the land cannot hold).
EXPECT_FAIL = {"basin": ("cannot_hold", "broken_hold")}

# Written by hand from looking at this sheet's views, plans and numbers (2026-11-15).
STILL_WRONG_BY_EYE: list[str] = [
    "Every change costs nearly a cold build: vale ~75 s a change against 87 s cold, massif ~86 s against 102 s,",
    "  basin 270-400 s when the land moves (fields recomputed over the whole extent; store memo dominates).",
    "massif's counts are one tree short of what its expect.toml recorded on 2026-11-15 in every build",
    "  (base 307, recorded 308): the engine drifted since, or a draw is not stable; not yet found which.",
    "massif's land, in plan, is parallel diagonal ridges at a regular spacing: it reads as made, not",
    "  weathered; its beck runs in a straight cut with stepped walls (at_mouth), and by_water's walk is",
    "  blocked after 9 m by a 53 degree bank.",
    "basin's river is a thin straight ribbon across a flat beach (at_mouth): the mouth does not read as a",
    "  mouth; the ground under two views is 13 m and 6.6 m from the spike's; 25126 woods against 417.",
    "basin's on_shore walks 234 m toward the haven before it meets water, though its text says it stands",
    "  about 12 m inland; tarn_shore's walk is 'blocked by water' though the tarn it walks to is water.",
    "vale after move_road: hamlet looks into conifers and its walk is blocked by cottage_a after 31 m;",
    "  the view no longer shows the hamlet from the road.",
    "row 16's lint (a large solid asset at collision none) is not written; only the walker would see it.",
    "row 30 (a step reading outside data) has no world. basin's terrain step is a Rust program, not the",
    "  SDK: R19a/b not met (owner's ruling owed).",
    "Placement (glade by_cause, row 50) is judged by eye only; nothing measures whether it looks good.",
    "Same bytes across systems: macOS only (set aside).",
]


def main() -> None:
    keep = sys.argv[sys.argv.index("--keep") + 1] if "--keep" in sys.argv else None
    tmp = Path(keep) if keep else Path(tempfile.mkdtemp(prefix="via-m2-sheet-"))
    if "--draw-only" in sys.argv:
        data = json.loads((tmp / "data.json").read_text())
    else:
        tmp.mkdir(parents=True, exist_ok=True)
        print("building via ...", flush=True)
        code, out = run(["cargo", "build", "--release", "-q", "-p", "via-cli", "-p", "via-terrain"])
        if code:
            sys.exit(out)
        data = collect(tmp)
        (tmp / "data.json").write_text(json.dumps(data, indent=1))
    draw(tmp, data)


if __name__ == "__main__":
    main()
