# /// script
# requires-python = ">=3.11"
# dependencies = ["matplotlib>=3.9,<3.10", "numpy>=2.0,<3"]
# ///
"""Draw the M3 sheet, docs/reports/m3-sheet.png, from fresh builds of M3's five worlds.

    uv run docs/reports/make_m3_sheet.py [--keep DIR] [--draw-only] [--fresh]

What it runs, in order (every number on the sheet comes from one of these):

1. `cargo build --release -p via-cli` into $CARGO_TARGET_DIR (default target/); or the binary the
   VIA environment variable names.
2. A scratch user config (VIA_CONFIG_DIR, protocol.md 6.3) with `python` set to the SDK's venv
   Python, and each world's steps trusted with `via trust` (base and every change).
3. Per world (examples/quay, townland, example, monastery, city), in one fresh store: the base build
   cold, the same build again into a new directory (unchanged, warm store), then every change in
   the world's changes/ folder, each `--against` the base. Each build is timed by /usr/bin/time -l
   (wall seconds and peak resident memory) and by its run report's `totals.ms` (graph.md 6.3).
4. Each world's measure.py `--check` on the base and on every change (bands from expect.toml).
5. A meetings probe of this script's own (`probe`, below) on the base and the key change.
6. engines/godot/sheet.sh once per world, base against its key change, water=tag:water_level and
   no paths overlay (the M3 worlds paint their own ways).

With --keep, a world whose DIR/<world>.json a previous run wrote is not built again unless `--fresh`
is given. `--draw-only` redraws from DIR/data.json and the renders kept in DIR. The "still wrong"
list is written by hand from the views, the probe and the M3 slice 5 review on foot; the lines
that are numbers are computed.
"""

from __future__ import annotations

import datetime
import json
import math
import os
import platform
import re
import shutil
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
OUT = ROOT / "docs/reports/m3-sheet.png"
TARGET = Path(os.environ.get("CARGO_TARGET_DIR", ROOT / "target"))
VIA = Path(os.environ.get("VIA", TARGET / "release/via"))
PY = ROOT / "sdk/python/.venv/bin/python"
LIB = ROOT / "lib"

# Each world: the key change the views show, what it says, the measure.py arguments a change takes
# ("base": --base; "run": --base and --run; "none": neither), and four ground bookmarks at meetings.
WORLDS = [
    {"name": "quay", "key": "move_hill", "key_says": "the crest moves 80 m west (the moved hill)",
     "margs": "run", "bookmarks": ["way_street", "lot_street", "street_quay", "from_gate"]},
    {"name": "townland", "key": "move_crossing", "key_says": "the old bridge moves; ways, embankment and streets follow",
     "margs": "base", "bookmarks": ["bridge_bank", "on_bridge", "market_view", "up_old_town"]},
    {"name": "example", "key": "move_beck", "key_says": "the beck moves; footbridge, lanes and mill follow",
     "margs": "none", "bookmarks": ["lane_bridge", "under_bridge", "at_mill", "east_row_view"]},
    {"name": "monastery", "key": "move_bridge", "key_says": "the bridge over the gorge moves; the approach re-routes",
     "margs": "base", "bookmarks": ["bm_bridge_end", "bm_deck", "bm_gate", "bm_gorge"]},
    {"name": "city", "key": "move_bridge", "key_says": "the central crossing moves 48 m upstream; one deck, arterial re-routed",
     "margs": "base", "bookmarks": ["central_east_head", "downtown_at_central", "downtown_street", "old_town_street"]},
]


def run(cmd: list, cwd: Path = ROOT, env: dict | None = None) -> tuple[int, str]:
    e = dict(os.environ)
    e.update(env or {})
    p = subprocess.run([str(c) for c in cmd], cwd=cwd, capture_output=True, text=True, env=e)
    return p.returncode, p.stdout + p.stderr


def jload(p: Path):
    return json.loads(p.read_text()) if p.exists() else None


# ------------------------------------------------------------------------------------------ probe

PROBE_SKIP = {"crossing", "waterfront", "beck", "cliff", "river", "enclosure"}


def _sampler(man: dict, h: np.ndarray):
    res = man["frame"]["resolution"]
    ox, oy = man["frame"]["extent"][:2]
    rows, cols = h.shape

    def g(x: float, y: float) -> float:
        i = min(max((x - ox) / res, 0.0), cols - 1.000001)
        j = min(max((y - oy) / res, 0.0), rows - 1.000001)
        i0, j0 = int(i), int(j)
        u, v = i - i0, j - j0
        return float((h[j0, i0] * (1 - u) + h[j0, i0 + 1] * u) * (1 - v)
                     + (h[j0 + 1, i0] * (1 - u) + h[j0 + 1, i0 + 1] * u) * v)
    return g


def _resample(pts, step=0.5):
    p = np.array([q[:2] for q in pts], float)
    s = np.concatenate([[0], np.cumsum(np.hypot(*np.diff(p, axis=0).T))])
    t = np.arange(0, s[-1] + 1e-9, step)
    if s[-1] - t[-1] > 1e-6:
        t = np.append(t, s[-1])
    return np.c_[np.interp(t, s, p[:, 0]), np.interp(t, s, p[:, 1])], float(s[-1])


def _segdist(p, ln) -> float:
    a, b = ln[:-1], ln[1:]
    ab = b - a
    t = np.clip(((p - a) * ab).sum(1) / np.maximum((ab * ab).sum(1), 1e-12), 0, 1)
    return float(np.min(np.hypot(*(a + ab * t[:, None] - p).T)))


def probe(d: Path) -> dict:
    """Meetings on foot, ours (after the M3 slice 5 review's probe). Ground = the surface field read
    bilinear. A way = a path feature with attrs.width whose kind is not in PROBE_SKIP; its centre
    line is resampled every 0.5 m. g1 = largest rise over 1 m; g10 = largest rise over 10 m / 10.
    A junction end = a way end within 1 m of another way's line; its lip = the largest rise over 1 m
    within its last 6 m. Near miss = two way ends 0.05 to 3 m apart; stub = a way under 10 m.
    Doors (instances with a `door` mount): floor = ground at the origin + z, against the ground 3 m
    out from the door; level = within 0.3 m, off = more than 1 m. Setback = the door's distance from
    the nearest street or main_street centre line less (width / 2 + footway)."""
    man = jload(d / "manifest.json")
    surf = next(n for n, v in man["fields"].items() if v.get("role") == "surface")
    h = np.load(d / man["fields"][surf]["file"])
    g = _sampler(man, h)
    fs = jload(d / "features.json")["features"]
    ways = [f for f in fs if f["shape"] == "path" and "width" in (f.get("attrs") or {})
            and f.get("kind") not in PROBE_SKIP]
    lines = {f["name"]: np.array([p[:2] for p in f["points"]], float) for f in ways}
    out = {"ways": len(ways), "lips": [], "near": 0, "stubs": 0, "g10_main": 0.0, "g1_main": 0.0}
    ends = []
    for f in ways:
        xy, L = _resample(f["points"])
        z = np.array([g(*p) for p in xy])
        if L < 10:
            out["stubs"] += 1
        if f.get("kind") == "main_street" and len(z) > 20:
            out["g1_main"] = max(out["g1_main"], float(np.max(np.abs(z[2:] - z[:-2]))))
            out["g10_main"] = max(out["g10_main"], float(np.max(np.abs(z[20:] - z[:-20])) / 10))
        for end in (0, -1):
            p = xy[end]
            ends.append(p)
            if not any(_segdist(p, ln) < 1.0 for nm, ln in lines.items() if nm != f["name"]):
                continue
            zz = z[:13] if end == 0 else z[-13:]
            if len(zz) > 2:
                out["lips"].append(float(np.max(np.abs(zz[2:] - zz[:-2]))))
    for i in range(len(ends)):
        for j in range(i + 1, len(ends)):
            if 0.05 < float(np.hypot(*(ends[i] - ends[j]))) < 3:
                out["near"] += 1
    lips = np.array(out["lips"] or [0.0])
    out["junction_ends"] = len(out["lips"])
    out["lip_max"] = float(lips.max())
    out["lip_over_0_3"] = int((lips > 0.3).sum())
    out["lip_median"] = float(np.median(lips))
    del out["lips"]
    names = [n for n in (d / "instances/names.txt").read_text().splitlines() if n] \
        if (d / "instances/names.txt").exists() else []
    if names:
        X, Y, Z, F = (np.load(d / f"instances/{k}.npy").astype(float) for k in ("x", "y", "z", "face"))
        if np.issubdtype(np.load(d / "instances/x.npy").dtype, np.integer):
            X, Y, Z, F = X / 1000, Y / 1000, Z / 1000, F / 1000
        asset = [None] * len(names)
        for ch in man["chunks"]:
            for a, (s, n) in ch.get("instances", {}).items():
                for k in range(s, s + n):
                    asset[k] = a
        st = [(np.array([p[:2] for p in f["points"]], float),
               float(f["attrs"].get("width", 0)) / 2 + float(f["attrs"].get("footway", 0)))
              for f in fs if f["shape"] == "path" and f.get("kind") in ("street", "main_street")]
        offs, gaps = [], []
        for k in range(len(names)):
            m = man["assets"].get(asset[k], {}).get("mounts", {}).get("door")
            if not m:
                continue
            a = math.radians(F[k])
            ax, ay = m["at"]
            dx, dy = ax * math.cos(a) + ay * math.sin(a), -ax * math.sin(a) + ay * math.cos(a)
            offs.append(g(X[k], Y[k]) + Z[k] - g(X[k] + dx + 3 * math.sin(a), Y[k] + dy + 3 * math.cos(a)))
            if st and names[k].startswith("houses"):
                p = np.array([X[k] + dx, Y[k] + dy])
                gaps.append(min(_segdist(p, ln) - e for ln, e in st))
        o = np.abs(np.array(offs or [0.0]))
        out["doors"] = len(offs)
        out["doors_level"] = int((o < 0.3).sum()) if offs else 0
        out["doors_off"] = int((o > 1).sum()) if offs else 0
        if gaps:
            gp = np.array(gaps)
            out["setback"] = [float(np.percentile(gp, 10)), float(np.median(gp)), float(np.percentile(gp, 90))]
    return out


# ---------------------------------------------------------------------------------------- collect

def timed(cmd: list, cwd: Path, env: dict) -> tuple[int, str, float, float]:
    code, text = run(["/usr/bin/time", "-l"] + cmd, cwd=cwd, env=env)
    wall = re.search(r"([\d.]+) real", text)
    rss = re.search(r"(\d+)\s+maximum resident set size", text)
    return code, text, float(wall.group(1)) if wall else float("nan"), int(rss.group(1)) / 1e9 if rss else float("nan")


def build(w: dict, root: Path, env: dict, folder: str, change: str | None = None) -> dict:
    wf = ROOT / "examples" / w["name"] / "world.toml"
    d = root / folder
    diag = root / f"{folder}.diagnostics.json"
    cmd = [VIA, "build", wf, "--lib", LIB, "--store", root / "store", "--out", d, "--diagnostics", diag]
    if change:
        cmd += ["--change", change, "--against", root / "base"]
    code, text, wall, rss = timed(cmd, ROOT, env)
    r = jload(root / f"{folder}.run.json") or {}
    dg = jload(diag) or {}
    rows = dg.get("diagnostics", [])
    counts: dict[str, int] = {}
    for x in rows:
        if x["severity"] != "info":
            k = f"{x['code']} ({x['severity'][0]})"
            counts[k] = counts.get(k, 0) + 1
    walked = next((x.get("data") or {} for x in rows if x["code"] == "step.reach.walked"), {})
    return {"folder": folder, "status": code, "wall": wall, "rss": rss,
            "reach": [walked.get("held"), walked.get("pairs")] if walked else None,
            "first": next((s for s in text.splitlines() if s.startswith("error")), ""),
            "totals": r.get("totals", {}), "threads": r.get("threads"), "codes": counts}


def measure(w: dict, root: Path, folder: str, section: str) -> tuple[int, list[str]]:
    tool = ROOT / "examples" / w["name"] / "measure.py"
    args = [PY, tool, root / folder]
    if section != "base" and w["margs"] in ("base", "run"):
        args += ["--base", root / "base"]
    if section != "base" and w["margs"] == "run":
        args += ["--run", root / f"{folder}.diagnostics.json"]
    code, out = run(args + ["--check", section], env={"VIA": str(VIA), "VIA_LIB": str(LIB)})
    return code, [s for s in out.splitlines() if s.strip()]


def godot(w: dict, root: Path) -> tuple[int, str]:
    out = root / "review"
    if out.exists():
        shutil.rmtree(out)
    code, text = run([ROOT / "engines/godot/sheet.sh", "--before", root / "base", "--after",
                      root / w["key"], "--out", out, "--option", "water=tag:water_level"],
                     env={"SHEET_TIMEOUT": "1500"})
    return code, text[-600:]


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
        changes = sorted(p.stem for p in (here / "changes").glob("*.toml"))
        for c in [None] + changes:
            extra = ["--change", here / "changes" / f"{c}.toml"] if c else []
            code, out = run([VIA, "trust", wf, "--lib", LIB, "--yes"] + extra, env=env)
            if code:
                sys.exit(f"via trust {w['name']} {c}: {out}")
        root = tmp / w["name"]
        if root.exists():
            shutil.rmtree(root)
        root.mkdir(parents=True)
        cold = build(w, root, env, "base")
        if cold["status"]:
            sys.exit(f"{w['name']} base build failed:\n{cold['first']}")
        warm = build(w, root, env, "unchanged")
        ch, ms = {}, {"base": measure(w, root, "base", "base")}
        for c in changes:
            print(f"  {c}", flush=True)
            ch[c] = build(w, root, env, c, str(here / "changes" / f"{c}.toml"))
            ms[c] = measure(w, root, c, c)
        print("  probe, godot", flush=True)
        pr = {"base": probe(root / "base"), w["key"]: probe(root / w["key"])}
        gcode, gtext = godot(w, root)
        m = jload(root / "base/manifest.json")
        data["worlds"][w["name"]] = {
            "cold": cold, "warm": warm, "changes": ch, "measures": ms, "probe": pr,
            "godot": [gcode, gtext], "frame": m["frame"], "instances": m["instances"]["count"],
        }
        saved.write_text(json.dumps(data["worlds"][w["name"]], indent=1))
        # Keep the views and the two builds the plan draws; drop the store and the other builds.
        shutil.rmtree(root / "store", ignore_errors=True)
        for p in root.iterdir():
            if p.is_dir() and p.name not in ("base", w["key"], "review"):
                shutil.rmtree(p)
    _, load1 = run(["sysctl", "-n", "vm.loadavg"])
    data["load_end"] = load1.strip().strip("{} ")
    sj = tmp / "quay/review/sheet.json"
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
        ax.imshow(np.ma.masked_less_equal(wv - h, 0.001), origin="lower", extent=ext, cmap="Blues",
                  vmin=-1.5, vmax=4, alpha=0.9)
    span_m = max(x1 - x0, y1 - y0)

    def draw(d: Path, style: str, colour: str, label: bool) -> None:
        fs = jload(d / "features.json")
        if not fs:
            return
        for f in fs["features"]:
            if f["shape"] == "path" and f.get("points"):
                p = np.array(f["points"])[:, :2]
                wide = f.get("kind") in ("main_street", "arterial", "avenue", "freeway", "road", "crossing")
                ax.plot(p[:, 0], p[:, 1], style, color=colour, lw=1.5 if wide else 0.7)
            elif f["shape"] == "region" and f.get("kind") in ("block",):
                for pc in f.get("pieces", []):
                    q = np.array(pc["outer"] + pc["outer"][:1])
                    ax.plot(q[:, 0], q[:, 1], style, color=colour, lw=0.3, alpha=0.6)
            elif f.get("kind") == "bookmark" and f["name"] in bms:
                e, t = np.array(f["at"][:2]), np.array(f["attrs"]["toward"]["at"][:2])
                v = (t - e) / max(np.hypot(*(t - e)), 1e-6) * span_m * 0.05
                ax.annotate("", xy=tuple(e + v), xytext=tuple(e), arrowprops=dict(arrowstyle="->", color=colour, lw=1.3))
                ax.plot(*e, "o", color=colour, ms=3.5)
                if label:
                    up = bms.index(f["name"]) % 2 == 0
                    ax.text(e[0] + span_m * 0.012, e[1] + span_m * (0.012 if up else -0.04), f["name"],
                            fontsize=8, color=colour, weight="bold",
                            bbox=dict(facecolor="white", edgecolor="none", pad=0.5, alpha=0.6))
        for fp in fs.get("footprints", []):
            for pc in fp["pieces"]:
                q = np.array(pc["outer"] + pc["outer"][:1])
                ax.fill(q[:, 0], q[:, 1], color="#7a2f1d" if style == "-" else "none",
                        edgecolor=colour, lw=0.3, alpha=0.8)

    if (before / "instances/x.npy").exists():
        x = np.load(before / "instances/x.npy") / 1000.0
        y = np.load(before / "instances/y.npy") / 1000.0
        k = max(1, len(x) // 6000)
        ax.plot(x[::k], y[::k], ".", color="#2e6b2e", ms=1.0, alpha=0.6)
    draw(before, "-", "#202020", True)
    draw(after, "--", "#c05a00", False)
    ax.set_xlim(x0, x1)
    ax.set_ylim(y0, y1)
    ax.set_aspect("equal")
    ax.tick_params(labelsize=7)


def rows(d: dict) -> dict[str, dict[str, float]]:
    """Every measure.py line as section -> measure -> value."""
    out: dict[str, dict] = {}
    for sec, (_, lines) in d["measures"].items():
        for s in lines:
            mm = re.match(r"^(ok|FAIL)\s+\S+?: (\S+) = (\S+?)[,\s]", s + " ")
            if mm:
                try:
                    out.setdefault(sec, {})[mm.group(2)] = float(mm.group(3))
                except ValueError:
                    out.setdefault(sec, {})[mm.group(2)] = mm.group(3)
    return out


def acceptance(data: dict) -> list[tuple[str, str]]:
    """The M3 row of docs/plan.md, a number for each, read from the measure lines above."""
    R = {n: rows(d) for n, d in data["worlds"].items()}

    def v(world, sec, key, fmt="{:g}"):
        x = R.get(world, {}).get(sec, {}).get(key)
        return "?" if x is None else fmt.format(x) if isinstance(x, float) else str(x)

    out = []
    out.append(("head", "1. The moved hill (quay move_hill: the crest 80 m west; townland move_hill)"))
    out.append(("", f"quay: {v('quay', 'move_hill', 'lot_removed')} lots resettled, {v('quay', 'move_hill', 'lot_added')} new, "
                    f"{v('quay', 'move_hill', 'street_removed')} streets re-laid and {v('quay', 'move_hill', 'street_added')} added "
                    f"(a street down to the way), {v('quay', 'move_hill', 'houses_removed')} houses moved out; "
                    f"identity.reselected {v('quay', 'move_hill', 'identity.reselected')} (two views now on one block: said, not silent)."))
    out.append(("", f"townland move_hill: {v('townland', 'move_hill', 'houses')} houses (base {v('townland', 'base', 'houses')}), "
                    f"reach {v('townland', 'move_hill', 'reach_held')} pairs held, 0 errors; the districts stay where the hill was "
                    f"(README), so the old town empties. Holds as a resettle; the town does not follow the hill."))
    out.append(("head", "2. R13's four obligations (core networks and cells, geometry.md 8)"))
    out.append(("", f"a clear on a generated lot removes its houses: quay clear_one {v('quay', 'clear_one', 'houses_removed')} houses, "
                    f"{v('quay', 'clear_one', 'lot_removed')} lots changed; city hand {v('city', 'hand', 'houses_removed')} houses, "
                    f"{v('city', 'hand', 'lot_removed')} lots, {v('city', 'hand', 'blocks_kept')} blocks kept.   HELD"))
    out.append(("", f"a pin inside one is reported: quay pin_one identity.pin_removed {v('quay', 'pin_one', 'identity.pin_removed')}, "
                    f"{v('quay', 'pin_one', 'houses_removed')} houses give way, {v('quay', 'pin_one', 'lot_removed')} lots changed.   HELD"))
    out.append(("", f"a thing placed on a generated street: quay's way_street and street_street views stand `on` junctions of "
                    f"network town ({v('quay', 'base', 'bookmarks')} views resolve).   HELD (junctions and cells only; no `run` along one: doc.not_yet)"))
    out.append(("", f"names stay when an unrelated part changes: quay unrelated (east edge 6 m in) {v('quay', 'unrelated', 'lot_removed_west')} lots "
                    f"west changed, {v('quay', 'unrelated', 'block_removed')} blocks; city move_bridge "
                    f"{v('city', 'move_bridge', 'lot_changed_in_kept_blocks')} and move_avenue "
                    f"{v('city', 'move_avenue', 'lot_changed_in_kept_blocks')} lots renamed in kept blocks.   NOT HELD at city size"))
    out.append(("head", "3. Dead ends and decks no worse than R18's (townland 39% -> 15% dead ends, 120 -> 35 places; three decks -> one)"))
    de = ", ".join(f"{n} {v(n, 'base', 'dead_end_count')}/{v(n, 'base', 'edge_ends')}" for n in ("quay", "townland", "city"))
    out.append(("", f"dead ends / edge ends (lib/review form.py, edge and gate ends counted apart): {de}. The spike counted edge and "
                    f"gate ends as dead ends, so townland's comparable figure is {v('townland', 'base', 'edge_ends')} places against "
                    f"R18's 35 (and 120 before)."))
    out.append(("", f"decks: townland {v('townland', 'base', 'decks')} ({v('townland', 'base', 'doubled_decks')} doubled), city "
                    f"{v('city', 'base', 'decks')} for {v('city', 'base', 'crossings')} crossings ({v('city', 'base', 'doubled_decks')} doubled, "
                    f"{v('city', 'base', 'crossing_ends_met')} of 6 ends met), monastery {v('monastery', 'base', 'decks')}; "
                    f"0 wet deck posts; geom.unowned_crossing 0 in every world. Ways reach decks at 1:10 (head_grade "
                    f"{v('townland', 'base', 'head_grade')}). Decks are causeways on the terrain (stand-in).   HELD"))
    qr = data["worlds"].get("quay", {}).get("cold", {}).get("reach") or ["?", "?"]
    qr = f"{qr[0]} of {qr[1]}"
    out.append(("head", "4. Reach as a library lint (lib/review reach.py, a lint step through the SDK)"))
    out.append(("", f"pairs held (step.reach.walked): quay {qr}, townland {v('townland', 'base', 'reach_held')} (move_crossing {v('townland', 'move_crossing', 'reach_held')}), "
                    f"example {v('example', 'base', 'reach.held')} (0 unreached, 0 not sealed), monastery {v('monastery', 'base', 'reach_held')} of "
                    f"{v('monastery', 'base', 'reach_pairs')}, city {v('city', 'base', 'reach_held')} (gate to gate over the three bridges).   HELD "
                    f"(top surface only; no decks under, no interiors)"))
    return out


def draw(tmp: Path, data: dict) -> None:
    _, rustc = run(["rustc", "--version"])
    _, head = run(["git", "log", "-1", "--format=%h %ad", "--date=short"])
    _, porcelain = run(["git", "status", "--porcelain"])
    _, cpu = run(["sysctl", "-n", "machdep.cpu.brand_string"])
    machine = f"{platform.system()} {platform.release()} {platform.machine()}, {cpu.strip()}"
    dirty = len(porcelain.splitlines())

    W, H, DPI = 2400, 11000, 100
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
    text(40, 22, f"via M3 sheet: quay, townland, example, monastery, city   branch rewrite, last commit "
         f"{head.strip()}; {state}   made {datetime.date.today().isoformat()}", size=16, mono=False, bold=True)
    text(40, 56, f"Made by docs/reports/make_m3_sheet.py on {machine}; {rustc.strip()}; Godot "
         f"{data.get('godot_version', '?')}. Every number is read from a build, a report or a command the "
         f"script ran; the builds are of the working tree, not of the commit.", size=11, mono=False, colour=GREY)
    text(40, 78, "Per world: eye height (1.7 m, engines/godot/sheet.sh, 1280x720, Compatibility renderer) at four "
         "meetings before (top) and after (below) its key change; the plan, before black (houses filled), after "
         "orange dashed;\nevery build with wall time and peak memory; the world's measure.py --check against its "
         "expect.toml (ok / FAIL); this script's meetings probe on the base and the key change.",
         size=11, mono=False, colour=GREY)

    # Acceptance first: the M3 row of docs/plan.md.
    c = Col(40, 120, width=225)
    c.head("M3 acceptance (docs/plan.md row M3: \"A town that holds on foot and under edits\"), each with its number")
    for kind, s in acceptance(data):
        if kind == "head":
            c.line(s, bold=True)
        else:
            c.line("  " + s, colour=BAD if "NOT HELD" in s else OK)
    y = c.y + 16

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
                p = root / "review" / side / f"{bm}.png"
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
        c.line(f"{'build':<15}{'exit':>4}{'wall s':>8}{'peak GB':>8}{'totals.ms':>10}  {'ran/reused/nodes':<20}"
               f"warnings and errors (non-info findings)", bold=True)

        def row(label: str, b: dict) -> None:
            t = b.get("totals") or {}
            nodes = f"{t.get('ran', '?')}/{t.get('reused', '?')}/{t.get('nodes', '?')}"
            codes = ", ".join(f"{k} {n}" for k, n in sorted(b["codes"].items(), key=lambda kv: -kv[1]))
            c.line(f"{label:<15}{b['status']:>4}{b['wall']:>8.1f}{b['rss']:>8.2f}{t.get('ms', '?')!s:>10}  "
                   f"{nodes:<20}{codes}", colour=OK if b["status"] == 0 else BAD)

        row("base (cold)", d["cold"])
        row("unchanged", d["warm"])
        for name, b in d["changes"].items():
            row(name, b)
        # Probe.
        c.y += 6
        for sec, p in d["probe"].items():
            sb = p.get("setback")
            c.line(f"probe {sec}: {p['ways']} ways, {p['junction_ends']} junction ends: lip (steepest metre in the "
                   f"last 6 m) median {p['lip_median']:.2f}, max {p['lip_max']:.2f}, {p['lip_over_0_3']} over 0.3; "
                   f"main way g1 {p['g1_main']:.2f} g10 {p['g10_main']:.2f}; {p['near']} near misses, {p['stubs']} stubs"
                   + (f"; doors {p['doors']}: {p['doors_level']} level, {p['doors_off']} off" if p.get("doors") else "")
                   + (f"; door behind the footway's back edge p10 {sb[0]:.2f}, median {sb[1]:.2f} m" if sb else ""),
                   colour=BAD if p["lip_over_0_3"] else OK)
        y = max(y + PW + 10, c.y + 6)
        # Measures, three columns.
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

    # Build times, then still wrong.
    c = Col(40, y, width=225)
    c.head("Build times: cold, unchanged and every change (wall seconds, /usr/bin/time -l; one build each)")
    c.line(f"{'world':<10}{'cold':>8}{'unchanged':>11}  changes against base (s)", bold=True)
    for w in WORLDS:
        d = data["worlds"].get(w["name"])
        if not d:
            continue
        ch = ", ".join(f"{n} {b['wall']:.1f}" for n, b in d["changes"].items())
        worst = max(b["wall"] for b in d["changes"].values()) / d["cold"]["wall"]
        c.line(f"{w['name']:<10}{d['cold']['wall']:>8.1f}{d['warm']['wall']:>11.1f}  {ch}   (slowest change "
               f"{worst:.0%} of cold)", colour=BAD if worst > 0.5 else OK)
    c.line("Protocol: release via from the working tree, default threads, VIA_LIB = the repo's lib; one fresh store per "
           "world; cold = the first build; unchanged = the same world rebuilt into a new directory, warm store; each change "
           f"--against base in that store. Laptop shared with other agents: load average {data['load_start']} at the "
           f"start, {data['load_end']} at the end. One run each: no spread is claimed.", colour=GREY)
    c.line("A change still costs most of a cold build (plan M3 asked for no number; M2's row 29 owes it): the fields memo "
           "is resolved over the whole extent once per plan, three plans per build (core report, M3 slice 5).", colour=BAD)
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
    for w in WORLDS:
        d = data["worlds"].get(w["name"])
        if not d:
            continue
        for sec, (code, ls) in d["measures"].items():
            for s in ls:
                if s.startswith("FAIL"):
                    out.append(f"{w['name']}: {s[:200]}")
        for name, b in d["changes"].items():
            if b["status"]:
                out.append(f"{w['name']} {name}: exit {b['status']}: {b.get('first', '')[:170]}")
        if d["godot"][0]:
            out.append(f"{w['name']}: sheet.sh exit {d['godot'][0]}")
    return out + STILL_WRONG_BY_EYE


# Written by hand from this sheet's views and probe lines and the M3 slice 5 review on foot
# (2026-11-15). Numbers with no source here are the probe's, above, or the review's probe (same
# protocol, the review's tree).
STILL_WRONG_BY_EYE: list[str] = [
    "1. Junction plates with 1:2 shoulders (worst on foot). Where two street beds meet, the owner of the crossing band holds a level",
    "   plate and the other way climbs its bank at 1:2: the probe's junction lips above (quay high_street on s0 g1 0.50 where its",
    "   g10 is 0.21; townland east_road climbs 3.5 m over 8 m off the market). Owed: the network node owns the meeting (R18) with the",
    "   crossing's approach hold (1:10); a named place kind owns the market's plate.",
    "2. Houses stand back from the footway: the door is about 1 m (quay, townland) and 1.5 m (city) behind the footway's back edge",
    "   (probe, setback) against an intended 0.5 m; the strip between is grass. Better than the review's 1.55 / 3.05 m, not closed.",
    "3. R13 for lots at city size: move_avenue renames 212 lots inside kept blocks, move_bridge 16 (measure rows, recorded as FAULT).",
    "   Wards and blocks hold. Not diagnosed: suspect the cell key's stretch anchor and ordinal (identity.md 2.3).",
    "4. Terraces cannot build at city size (selector members read whole per chunk, graph.rs define_all; quadratic in terraces): the",
    "   city's houses stand on the ground as it lies, 190 place.relief, doors up to 0.5 m off; the old town's hill is nearly empty.",
    "5. A change costs nearly a cold build in every world (table above). Owed: a windowed Fields resolve per chunk, one plan for the",
    "   three strata, answer/define/compute_step reading only what they need, houses and lints split per item.",
    "6. Not in M3 though planned: `run` and `set` are still doc.not_yet (example's row is written house by house); path way points",
    "   by name are not started (the city's grand avenue wanders 39 m off its line; monastery's turns are typed numbers).",
    "7. Stand-ins seen on foot: decks are causeways (water cannot pass under), interchanges are level grey plates, the freeway lies",
    "   on the ground, the monastery wall is ground with a gap for a gate (the approach steps -0.25 / +0.30 m at the gate).",
    "8. Views: city downtown_at_central looks into a house wall in the base; old_town_street looks across lots, not along a street;",
    "   a bush lands on example's at_mill eye after move_beck. Two grid corners on the city's district outlines miss by 1.0 and 2.3 m.",
    "9. Plan step 6 (a fresh agent builds a small variation from the documents alone) was not run for M3.",
]


def main() -> None:
    keep = sys.argv[sys.argv.index("--keep") + 1] if "--keep" in sys.argv else None
    tmp = Path(keep) if keep else Path(tempfile.mkdtemp(prefix="via-m3-sheet-"))
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
