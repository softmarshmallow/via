# /// script
# requires-python = ">=3.11"
# dependencies = ["matplotlib>=3.9,<3.10", "numpy>=2.0,<3"]
# ///
"""Draw the M1 sheet, docs/reports/m1-sheet.png, from fresh builds of examples/ford.

    uv run docs/reports/make_m1_sheet.py [--keep DIR] [--slow FILE]

What it runs, in order (every number on the sheet comes from one of these):

1. `cargo build --release -p via-cli`; the binary is target/release/via.
2. A scratch user config (VIA_CONFIG_DIR, protocol.md 6.3) with `python` set to the SDK's venv
   Python and ford's lint step trusted with `via trust --yes`; the user's own config is not touched.
3. ford built cold three times (a fresh store and directory each), then unchanged three times; the
   build times are `totals.ms` of each run report (graph.md 6.3), never a lint's.
4. The six change rows of docs/plan.md's acceptance table, each against its baseline: nothing,
   downstream, changes/vertex.toml, changes/bigger.toml, hand then hand+denser, the four bad_* files.
5. The lint's measures (lib/review/checks.py, `data.value` of each `step.checks.*` record).
6. engines/godot/sheet.sh twice (before/downstream, before/vertex), water=tag:water_level.
7. The conformance runner at milestone 1, `xtask check --milestone 1`'s case coverage, and the
   via-geom tests' MEASURE lines (track A's gate).

The "still wrong" list is written by hand from looking at the views and the numbers; the lines
that are numbers are computed.
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
OUT = ROOT / "docs/reports/m1-sheet.png"
FORD = ROOT / "examples/ford/world.toml"
VIA = ROOT / "target/release/via"
PY = ROOT / "sdk/python/.venv/bin/python"
BOOKMARKS = ["at_ford", "on_bank", "to_door", "in_trees"]
WHERE = {
    "at_ford": "on the lane short of the ford",
    "on_bank": "along the water's edge",
    "to_door": "from the lane to the door",
    "in_trees": "along the lane through the trees",
}


def run(cmd: list, cwd: Path = ROOT, env: dict | None = None) -> tuple[int, str]:
    e = dict(os.environ)
    e.update(env or {})
    p = subprocess.run([str(c) for c in cmd], cwd=cwd, capture_output=True, text=True, env=e)
    return p.returncode, p.stdout + p.stderr


class Ford:
    """Builds of ford in one scratch folder, with a scratch user config."""

    def __init__(self, tmp: Path):
        self.tmp = tmp
        self.env = {"VIA_CONFIG_DIR": str(tmp / "config")}
        (tmp / "config").mkdir(parents=True, exist_ok=True)
        code, out = run([VIA, "config", "program", "python", PY], env=self.env)
        if code:
            sys.exit(f"via config program: {out}")
        code, out = run([VIA, "trust", FORD, "--lib", ROOT / "lib", "--yes"], env=self.env)
        if code:
            sys.exit(f"via trust: {out}")

    def build(self, out: str, store: str, changes=(), against: str | None = None,
              threads: int | None = None) -> dict:
        d = self.tmp / out
        diag = self.tmp / f"{out}.diagnostics.json"
        # --lib is absolute so the scratch folders can live anywhere (a relative one works too).
        # No --sdk: the SDK carried in the binary runs the lint (an --sdk working copy would enter
        # the trust digest, protocol.md 8.1, and `via trust` would need the same flag).
        cmd = [VIA, "build", FORD, "--lib", ROOT / "lib",
               "--store", self.tmp / store, "--out", d, "--diagnostics", diag]
        for c in changes:
            cmd += ["--change", c]
        if against:
            cmd += ["--against", self.tmp / against]
        if threads:
            cmd += ["--threads", threads]
        code, text = run(cmd, cwd=FORD.parent, env=self.env)
        r = {"dir": d, "status": code, "text": text}
        rj = self.tmp / f"{out}.run.json"
        r["run"] = json.loads(rj.read_text()) if rj.exists() else None
        r["diag"] = json.loads(diag.read_text()) if diag.exists() else None
        lints = (r["diag"] or {}).get("lints", [])
        if code == 0 and not all(x["ran"] for x in lints):
            sys.exit(f"{out}: the lint step did not run:\n{text}")
        cj = d / "changes.json"
        r["changes"] = json.loads(cj.read_text()) if code == 0 and cj.exists() else None
        return r


def tree_bytes(d: Path) -> dict[str, bytes]:
    return {str(p.relative_to(d)): p.read_bytes() for p in sorted(d.rglob("*")) if p.is_file()}


def measures(b: dict) -> dict[tuple[str, str], float]:
    got = {}
    for x in (b["diag"] or {}).get("diagnostics", []):
        if x["code"].startswith("step.checks."):
            got[(x["code"][12:], x["subject"])] = x["data"]["value"]
    return got


def summary_lines(c: dict) -> list[str]:
    """The report's summary rows that are not all zero, and a count of those that are."""
    rows, zero = [], 0
    for s in c["summary"]:
        if any(s[k] for k in ("added", "removed", "moved", "swapped", "changed")):
            rows.append(s["line"])
        else:
            zero += 1
    if zero:
        rows.append(f"({zero} more row(s), all zero)")
    return rows


def names(c: dict, word: str, prefix: str = "") -> list[str]:
    return [e["name"] for e in c["entries"] if word in e["what"] and e["name"].startswith(prefix)]


def nodes_line(b: dict) -> str:
    t = b["run"]["totals"]
    return f"nodes ran {t['ran']}, reused {t['reused']}, skipped {t['skipped']} of {t['nodes']}"


def pad_area(d: Path) -> float:
    """The cottage's footprint area in features.json, outer rings less holes (shoelace)."""
    def ring(r: list) -> float:
        p = np.array(r)[:, :2]
        return 0.5 * abs(float(np.dot(p[:, 0], np.roll(p[:, 1], -1)) - np.dot(np.roll(p[:, 0], -1), p[:, 1])))
    fs = json.loads((d / "features.json").read_text())
    return sum(ring(pc["outer"]) - sum(ring(h) for h in pc.get("holes", []))
               for fp in fs["footprints"] if fp.get("instance") == "cottage" for pc in fp["pieces"])


WORLD_TEXT = FORD.read_text()
WORLD = tomllib.loads(WORLD_TEXT)
PIN = WORLD["change"]["hand"]["pin"]["oak"]["at"]
CLEAR = WORLD["change"]["hand"]["clear"]["gap"]


def change_rows(f: Ford, base: dict) -> list[dict]:
    rows = []
    # 1. Nothing: rebuild unchanged against itself; and bytes at 1 and 8 threads, fresh stores.
    again = f.build("ford", "store", against=None)
    t1 = f.build("t1", "store_t1", threads=1)
    t8 = f.build("t8", "store_t8", threads=8)
    tot = again["run"]["totals"]
    same = tree_bytes(t1["dir"]) == tree_bytes(t8["dir"])
    c = again["changes"]
    rows.append({
        "name": "Nothing (rebuild into build/ford)",
        "lines": [f"{len(c['entries'])} entries; nodes ran {tot['ran']}, reused {tot['reused']} "
                  f"of {tot['nodes']}"],
        "checks": [("no node runs", tot["ran"] == 0),
                   ("directory bytes equal at --threads 1 and 8 (fresh stores)", same)],
    })
    # 2. downstream.
    ds = f.build("ford+downstream", "store", ["downstream"], against="ford")
    c = ds["changes"]
    want = {"ford", "lane_west", "lane_east", "at_ford", "on_bank", "to_door", "in_trees"}
    feats = {e["name"] for e in c["entries"] if e["table"] == "feature"}
    fields = {e["name"]: e["region"] for e in c["entries"] if e["table"] == "field"}
    t_add, t_rem = len(names(c, "added", "trees#")), len(names(c, "removed", "trees#"))
    t_mov = len(names(c, "moved", "trees#"))
    caused = all(e["causes"] for e in c["entries"])
    rows.append({
        "name": "downstream (ford moved to t = 0.8 on the beck)",
        "lines": [nodes_line(ds)] + summary_lines(c) + [
            "fields: " + ", ".join(f"{k} {v['posts']} posts" for k, v in sorted(fields.items()))],
        "checks": [("the seven features named", want <= feats),
                   ("cottage and its footprint named",
                    {("instance", "cottage"), ("footprint", "cottage")}
                    <= {(e["table"], e["name"]) for e in c["entries"]}),
                   ("height, clearing, water regions", set(fields) == {"height", "clearing", "water"}),
                   ("every entry has its cause", caused),
                   (f"trees added {t_add}, removed {t_rem}, moved {t_mov} (ground under them)",
                    t_add > 0 and t_rem > 0 and t_mov > 0)],
        "build": ds,
    })
    # 3. vertex.
    vx = f.build("ford+vertex", "store", ["changes/vertex.toml"], against="ford")
    c = vx["changes"]
    feats = {e["name"] for e in c["entries"] if e["table"] == "feature"}
    fields = {e["name"] for e in c["entries"] if e["table"] == "field"}
    t_mov = len(names(c, "moved", "trees#"))
    rows.append({
        "name": "changes/vertex.toml (beck vertex [160, 90] 4 m east)",
        "lines": [nodes_line(vx)] + summary_lines(c),
        "checks": [("beck named", "beck" in feats),
                   ("its height, clearing, water regions", fields == {"height", "clearing", "water"}),
                   (f"trees moved where the ground changed: {t_mov}", t_mov > 0),
                   ("downstream's features follow", want <= feats),
                   ("every entry has its cause", all(e["causes"] for e in c["entries"]))],
        "build": vx,
    })
    # 4. bigger.
    bg = f.build("ford+bigger", "store", ["changes/bigger.toml"], against="ford")
    c = bg["changes"]
    cot = [e for e in c["entries"] if e["name"] == "cottage"]
    facts = sorted({x["fact"] for e in cot for x in e["facts"]})
    rows.append({
        "name": "changes/bigger.toml (pack swapped for primitive_large)",
        "lines": [nodes_line(bg)] + summary_lines(c)
        + [f"cottage facts named: {', '.join(facts) or 'none'}"],
        "checks": [("cottage named with old and new facts", bool(facts)),
                   (f"the pad grows: {pad_area(base['dir']):.1f} to {pad_area(bg['dir']):.1f} m2",
                    pad_area(bg["dir"]) > pad_area(base["dir"])),
                   ("world text untouched (world.toml bytes the same)", FORD.read_text() == WORLD_TEXT)],
    })
    # 5. hand, then hand + denser.
    hd = f.build("ford+hand", "store", ["hand"], against="ford")
    dn = f.build("ford+hand+denser", "store", ["hand", "denser"], against="ford+hand")
    c = dn["changes"]
    before = (hd["dir"] / "instances/names.txt").read_text().split()
    after = (dn["dir"] / "instances/names.txt").read_text().split()
    s7 = [n for n in before if n.startswith("trees#s7000_")]
    s6 = [n for n in after if n.startswith("trees#s6000_")]
    removed = set(names(c, "removed", "trees#s7000_"))
    added = set(names(c, "added", "trees#s6000_"))
    x = np.load(dn["dir"] / "instances/x.npy") / 1000.0
    y = np.load(dn["dir"] / "instances/y.npy") / 1000.0
    (cx, cy), cr = CLEAR["at"], CLEAR["radius"]
    inside = int(np.sum((x - cx) ** 2 + (y - cy) ** 2 < cr ** 2))
    pin_at = [(float(x[i]), float(y[i])) for i, n in enumerate(after) if n == "oak"]
    hand_entries = [e for e in c["entries"] if e["name"] in ("oak", "gap")]
    hd_lines = [s for s in summary_lines(hd["changes"]) if not s.startswith("(")]
    rows.append({
        "name": "hand, then hand + denser (spacing 7 to 6)",
        "lines": ["hand vs ford: " + s for s in hd_lines] + [
                  f"denser: s7000_ removed {len(removed)} of {len(s7)}, "
                  f"s6000_ added {len(added)} of {len(s6)}; {nodes_line(dn)}"],
        "checks": [("every s7000_ removed, every s6000_ added",
                    removed == set(s7) and added == set(s6)),
                   (f"the pin oak stands at {pin_at[0] if pin_at else 'nowhere'} (world says {PIN})",
                    pin_at == [tuple(float(v) for v in PIN)]),
                   (f"nothing inside the clear gap, r {cr} m at {CLEAR['at']} ({inside} instances)",
                    inside == 0),
                   ("both hand edits unchanged (no entry for oak or gap)", not hand_entries)],
        "build": hd,
    })
    # 6. the four bad files together.
    bad = f.build("ford+bad", "store", [f"changes/bad_{n}.toml" for n in
                                        ("anchor", "typo", "cycle", "asset")])
    ds_ = (bad["diag"] or {}).get("diagnostics", [])
    errs = [d for d in ds_ if d["severity"] == "error"]
    warns = [d for d in ds_ if d["severity"] == "warning"]
    skips = {d["subject"]: d for d in ds_ if d["code"] == "diag.skipped"}
    want_codes = ["authority.conflict", "formula.unknown_name", "graph.cycle", "asset.unknown"]
    got_codes = sorted(d["code"] for d in errs)
    by = {d["code"]: d for d in errs}
    lines = []
    for d in errs + warns:
        near = (d.get("data") or {}).get("nearest")
        lines.append(f"{d['code']} {d['subject']} {d.get('file')}:{d.get('line')}"
                     + (" pos" if d.get("position") else "") + (f" nearest {near}" if near else ""))
    lines.append("diag.skipped under " + ", ".join(f"{k} ({v.get('count')})" for k, v in sorted(skips.items())))
    sq = [d for d in warns if d["code"] == "authority.squeezed"]
    shed = skips.get("instance.shed", {})
    rows.append({
        "name": "the four bad_* files together",
        "lines": lines,
        "checks": [("exit status 1", bad["status"] == 1),
                   ("the four errors, one each, each with a hint",
                    got_codes == sorted(want_codes) and all(d.get("hint") for d in errs)),
                   ("nearest x and shed_gable",
                    (by.get("formula.unknown_name", {}).get("data") or {}).get("nearest") == "x"
                    and (by.get("asset.unknown", {}).get("data") or {}).get("nearest") == "shed_gable"),
                   ("conflict and unknown asset have a position",
                    all(by.get(k, {}).get("position") for k in ("authority.conflict", "asset.unknown"))),
                   ("one squeezed warning: cottage, related beck, on clearing",
                    len(sq) == 1 and sq[0]["subject"] == "instance.cottage"
                    and any(r["subject"] == "feature.beck" for r in sq[0].get("related") or [])
                    and (sq[0].get("data") or {}).get("field") == "clearing"),
                   ("diag.skipped under shed (count 3: cottage, trees, checks), loop_a, cottage, trees",
                    set(skips) == {"instance.shed", "field.loop_a", "instance.cottage", "scatter.trees"}
                    and shed.get("count") == 3
                    and (shed.get("data") or {}).get("first")
                    == ["instance.cottage", "scatter.trees", "step.checks"]),
                   ("no other error or warning", len(errs) == 4 and len(warns) == 1)],
    })
    return rows


def godot(f: Ford, before: Path, after: Path, out: Path) -> tuple[int, str]:
    if out.exists():
        shutil.rmtree(out)
    return run([ROOT / "engines/godot/sheet.sh", "--before", before, "--after", after,
                "--out", out, "--option", "water=tag:water_level", "--option", "paths=kind:lane"])


def query(f: Ford, base: dict) -> list[str]:
    """`via query` (surfaces.md 7): surfaces_at under each bookmark's eye, against height.npy."""
    fs = json.loads((base["dir"] / "features.json").read_text())["features"]
    pts = [next(x for x in fs if x["name"] == bm)["at"][:2] for bm in BOOKMARKS]
    q_in, q_out = f.tmp / "query.in.json", f.tmp / "query.out.json"
    q_in.write_text(json.dumps({"x": [round(p[0] * 1000) for p in pts],
                                "y": [round(p[1] * 1000) for p in pts]}))
    code, text = run([VIA, "query", FORD, "surfaces_at", "--in", q_in, "--out", q_out,
                      "--lib", ROOT / "lib", "--store", f.tmp / "store"], cwd=FORD.parent, env=f.env)
    if code or not q_out.exists():
        return [f"via query surfaces_at: exit {code}: {text.strip().splitlines()[:1]}"]
    a = json.loads(q_out.read_text())
    h = np.load(base["dir"] / "fields/height.npy").astype(np.float64)
    out = [f"via query surfaces_at, the four bookmarks (exit {code}; columns {', '.join(sorted(a))}):"]
    zs = a.get("z", [])
    for i, bm in enumerate(BOOKMARKS):
        surf = [a["surface"][k] for k in range(len(zs)) if a["point"][k] == i] if "point" in a else []
        z = [zs[k] for k in range(len(zs)) if a.get("point", list(range(len(zs))))[k] == i]
        px, py = pts[i]
        hv = h[int(round(py / 0.5)), int(round(px / 0.5))]
        out.append(f"  {bm:<9} {', '.join(f'{s} {v:.3f}' for s, v in zip(surf or ['?'] * len(z), z))}"
                   f"  (height.npy at the nearest post {hv:.3f})")
    return out


def conformance() -> tuple[dict[str, int], dict[str, int], list[str]]:
    cdir = ROOT / "tools/conformance"
    _, out = run(["uv", "run", "python", "-m", "via_conformance", "list"], cwd=cdir)
    per = {m.group(1): int(m.group(2)) for m in re.finditer(r"^(M\d+): (\d+) cases", out, re.M)}
    _, out = run(["uv", "run", "python", "-m", "via_conformance", "run", "--via", VIA,
                  "--milestone", "1"], cwd=cdir)
    res = {k: 0 for k in ("pass", "fail", "pending", "error")}
    bad = []
    for m in re.finditer(r"^(pass|fail|pending|error)\s+(\S+)", out, re.M):
        res[m.group(1)] += 1
        if m.group(1) != "pass":
            bad.append(f"{m.group(1)} {m.group(2)}")
    return per, res, bad


def xtask_cases() -> tuple[str, str, int]:
    _, out = run(["cargo", "run", "-q", "-p", "xtask", "--", "check", "--milestone", "1"])
    status = re.search(r"^cases: (.*)$", out, re.M)
    note = re.search(r"note: (\d+ cases read at milestone 1; .*)$", out, re.M)
    others = len(re.findall(r"^[a-z-]+: ok$", out, re.M))
    return (status.group(1) if status else "?"), (note.group(1) if note else "?"), others


def track_a() -> tuple[list[str], str]:
    _, out = run(["cargo", "test", "-p", "via-geom", "--release", "--", "--nocapture"])
    meas = [m.group(1) for m in re.finditer(r"MEASURE (.*)$", out, re.M)]
    t = [0, 0]
    for m in re.finditer(r"test result: \w+\. (\d+) passed; (\d+) failed", out):
        t[0] += int(m.group(1))
        t[1] += int(m.group(2))
    named = {n: ("ok" if re.search(rf"test {n} \.\.\. ok", out) else "?")
             for n in ("clear_one_cell", "pin_in_cell", "distant_vertex_keeps_names")}
    return meas, f"{t[0]} passed, {t[1]} failed; " + ", ".join(f"{k} {v}" for k, v in named.items())


def gate_lines(meas: list[str]) -> list[str]:
    """Track A's eight gate clauses, each with its number from the via-geom tests."""
    def find(rx: str) -> re.Match | None:
        for s in meas:
            m = re.search(rx, s)
            if m:
                return m
        return None

    out = []
    m = find(r"linework_a: areas (\d+) \(reference faces (\d+)\), total ([\d.]+) m2 \(reference ([\d.]+)\)")
    out.append(f"faces close and count: {m.group(1)} areas (spike {m.group(2)}), "
               f"{m.group(3)} m2 (spike {m.group(4)})" if m else "faces: no measure")
    m = find(r"noding bound on linework_a: ([\d.]+) mm")
    out.append(f"  noding moves a vertex at most {m.group(1)} mm" if m else "  noding: no measure")
    m = find(r"names: (\d+) of (\d+) moved vertices kept every .*\((\d+) junctions, (\d+) edges, (\d+) areas\)")
    out.append(f"names hold under a distant move: {m.group(1)} of {m.group(2)} "
               f"({m.group(3)} junctions, {m.group(4)} edges, {m.group(5)} areas)" if m
               else "names: no measure")
    for d in ("8500", "17000"):
        m = find(rf"course_a stroke {d} mm both sides: pieces (\d+), holes (\d+), .*area ([\d.]+) m2 "
                 rf"\(reference ([\d.]+), diff ([-\d.]+) m2, ([-\d.]+) %\)")
        out.append(f"bank offset {int(d) / 1000:g} m: {m.group(1)} piece, {m.group(2)} holes, "
                   f"{m.group(3)} m2 (spike {m.group(4)}, {m.group(6)} %)" if m else
                   f"bank {d}: no measure")
    m = find(r"linework cells: areas (\d+)/(\d+), cells (\d+), cell_small (\d+), cell_none (\d+), "
             r"closure exact (\d+)")
    if m:
        out.append(f"every cell touches a street: {m.group(3)} cells over {m.group(1)}/{m.group(2)} areas,")
        out.append(f"  {m.group(4)} cell_small, {m.group(5)} cell_none, closure exact in {m.group(6)}")
    else:
        out.append("cells: no measure")
    return out


def plan_view(ax, base: dict, ds: dict) -> None:
    d = base["dir"]
    h = np.load(d / "fields/height.npy").astype(np.float64)
    w = np.load(d / "fields/water.npy").astype(np.float64)
    k = np.load(d / "fields/clearing.npy").astype(np.float64)
    ext = (0, 256, 0, 256)
    # Hillshade from the north-west, then contours every metre.
    gy, gx = np.gradient(h, 0.5)
    shade = np.clip(0.6 + 0.35 * (-gx * 0.707 + gy * 0.707) / np.sqrt(1 + gx**2 + gy**2) * 4, 0, 1)
    ax.imshow(shade, origin="lower", extent=ext, cmap="gray", vmin=0, vmax=1.2)
    ax.contour(h, levels=np.arange(18, 37, 1.0), origin="lower", extent=ext, colors="#8a7a5a",
               linewidths=0.4)
    depth = np.ma.masked_less_equal(w - h, 0.001)
    ax.imshow(depth, origin="lower", extent=ext, cmap="Blues", vmin=-0.4, vmax=1.4, alpha=0.85)
    ax.contour(k, levels=[0.5], origin="lower", extent=ext, colors=["#d9534f"], linewidths=0.8)
    feats = json.loads((d / "features.json").read_text())
    after = json.loads((ds["dir"] / "features.json").read_text())

    def draw(fs: dict, style: str, colour: str, label: bool) -> None:
        for x in fs["features"]:
            if x["shape"] == "path":
                p = np.array(x["points"])[:, :2]
                ax.plot(p[:, 0], p[:, 1], style, color=colour, lw=1.6)
            elif x["kind"] == "ford":
                ax.plot(*x["at"][:2], "D", color=colour, ms=7, mfc="none" if style != "-" else colour)
            elif x["kind"] == "bookmark":
                e, t = x["at"][:2], x["attrs"]["toward"]["at"][:2]
                v = np.array(t) - np.array(e)
                v = v / np.hypot(*v) * 12
                ax.annotate("", xy=(e[0] + v[0], e[1] + v[1]), xytext=e,
                            arrowprops=dict(arrowstyle="->", color=colour, lw=1.4))
                ax.plot(*e, "o", color=colour, ms=4)
                if label:
                    ax.text(e[0] + 2, e[1] - 6, x["name"], fontsize=10, color=colour, weight="bold")
        for fp in fs["footprints"]:
            for piece in fp["pieces"]:
                p = np.array(piece["outer"] + piece["outer"][:1])
                ax.plot(p[:, 0], p[:, 1], style, color=colour, lw=1.2)

    x = np.load(d / "instances/x.npy") / 1000.0
    y = np.load(d / "instances/y.npy") / 1000.0
    ax.plot(x, y, "o", color="#2e6b2e", ms=3.5)
    draw(feats, "-", "#202020", True)
    draw(after, "--", "#c05a00", False)
    ax.set_xlim(0, 256)
    ax.set_ylim(0, 256)
    ax.set_aspect("equal")
    ax.set_xticks(range(0, 257, 64))
    ax.set_yticks(range(0, 257, 64))
    ax.tick_params(labelsize=9)


def main() -> None:
    keep = sys.argv[sys.argv.index("--keep") + 1] if "--keep" in sys.argv else None
    tmp = Path(keep) if keep else Path(tempfile.mkdtemp(prefix="via-m1-sheet-"))
    if tmp.exists() and keep:
        shutil.rmtree(tmp)
    tmp.mkdir(parents=True, exist_ok=True)
    print("building via ...", flush=True)
    code, out = run(["cargo", "build", "--release", "-q", "-p", "via-cli"])
    if code:
        sys.exit(out)
    f = Ford(tmp)

    print("cold and unchanged builds ...", flush=True)
    _, load = run(["sysctl", "-n", "vm.loadavg"])
    cold, warm = [], []
    for i in range(3):
        b = f.build(f"cold{i}", f"store_cold{i}")
        cold.append(b["run"]["totals"]["ms"])
    base = f.build("ford", "store")
    for _ in range(3):
        b = f.build("ford", "store")
        warm.append(b["run"]["totals"]["ms"])
    threads = base["run"]["threads"]
    nodes = base["run"]["totals"]["nodes"]
    fam_n, fam_ms = {}, {}
    for n in json.loads((tmp / "cold0.run.json").read_text()).get("nodes", []):
        fam_n[n["family"]] = fam_n.get(n["family"], 0) + 1
        fam_ms[n["family"]] = fam_ms.get(n["family"], 0) + (n.get("ms") or 0)

    print("change rows ...", flush=True)
    rows = change_rows(f, base)
    ds, vx, hd = rows[1]["build"], rows[2]["build"], rows[4]["build"]
    m0, m1, m2, m3 = measures(base), measures(ds), measures(vx), measures(hd)
    q_lines = query(f, base)

    print("Godot views ...", flush=True)
    gs = [godot(f, base["dir"], ds["dir"], tmp / "review_downstream"),
          godot(f, base["dir"], vx["dir"], tmp / "review_vertex"),
          godot(f, base["dir"], hd["dir"], tmp / "review_hand")]
    gver = json.loads((tmp / "review_downstream/sheet.json").read_text())["godot"] \
        if (tmp / "review_downstream/sheet.json").exists() else "?"

    # --slow FILE: reuse the conformance, xtask and track A results a previous run saved there
    # (for redrawing the layout only; a sheet that is handed over is made without it).
    slow = Path(sys.argv[sys.argv.index("--slow") + 1]) if "--slow" in sys.argv else None
    if slow and slow.exists():
        per_m, conf, conf_bad, cases_status, cases_note, xtask_ok, meas, geom_tests = \
            json.loads(slow.read_text())
    else:
        print("conformance, xtask, track A ...", flush=True)
        per_m, conf, conf_bad = conformance()
        cases_status, cases_note, xtask_ok = xtask_cases()
        meas, geom_tests = track_a()
        if slow:
            slow.write_text(json.dumps([per_m, conf, conf_bad, cases_status, cases_note, xtask_ok,
                                        meas, geom_tests]))

    _, rustc = run(["rustc", "--version"])
    _, head = run(["git", "log", "-1", "--format=%h %ad", "--date=short"])
    _, porcelain = run(["git", "status", "--porcelain"])
    _, cpu = run(["sysctl", "-n", "machdep.cpu.brand_string"])
    machine = f"{platform.system()} {platform.release()} {platform.machine()}, {cpu.strip()}"
    today = datetime.date.today().isoformat()
    dirty = len(porcelain.splitlines())

    # ---- the sheet ----
    W, H, DPI = 2400, 3200, 100
    fig = plt.figure(figsize=(W / DPI, H / DPI), dpi=DPI, facecolor="white")
    BODY, HEAD, MONO = 11, 14, "DejaVu Sans Mono"
    LH = 21
    BAD = "#b03a00"
    GREY = "#333333"

    def axes(x, y, w, h):
        return fig.add_axes([x / W, 1 - (y + h) / H, w / W, h / H])

    def text(x, y, s, size=BODY, mono=True, bold=False, colour="black"):
        fig.text(x / W, 1 - y / H, s, fontsize=size, family=MONO if mono else None, va="top",
                 weight="bold" if bold else "normal", color=colour)

    class Col:
        def __init__(self, x: int, y: int):
            self.x, self.y = x, y

        def head(self, s: str) -> None:
            self.y += 10
            text(self.x, self.y, s, size=HEAD, mono=False, bold=True)
            self.y += 30

        def line(self, s: str, bold: bool = False, colour: str = "black") -> None:
            text(self.x, self.y, s, bold=bold, colour=colour)
            self.y += LH

        def check(self, s: str, ok: bool) -> None:
            self.line(("  held   " if ok else "  NOT    ") + s, colour="black" if ok else BAD)

    state = (f"working tree: {dirty} uncommitted paths over it" if dirty
             else "working tree clean")
    text(40, 22, f"via M1 sheet: ford   branch rewrite, last commit {head.strip()}; {state}   "
         f"made {today}", size=18, mono=False, bold=True)
    text(40, 56, f"Made by docs/reports/make_m1_sheet.py on {machine}; {rustc.strip()}; "
         f"Godot {gver}. Every number is read from a build, a report or a command the script ran; "
         f"the builds are of the working tree, not of the commit.",
         size=BODY, mono=False, colour=GREY)

    # 1. Eye-height views: four bookmarks by before / downstream / vertex / hand.
    VX, VY, VW = 40, 100, 350
    VH = int(VW * 9 / 16)
    text(VX, VY, "1  Eye height (1.7 m), the four bookmarks, rendered by engines/godot/sheet.sh",
         size=HEAD, mono=False, bold=True)
    cols = [("before: ford", tmp / "review_downstream/before"),
            ("after downstream (ford to t = 0.8)", tmp / "review_downstream/after"),
            ("after vertex.toml (beck vertex 4 m east)", tmp / "review_vertex/after"),
            ("after hand (pin oak, clear gap)", tmp / "review_hand/after")]
    y = VY + 34
    for i, (label, _) in enumerate(cols):
        text(VX + i * (VW + 10), y, label, size=11, mono=False, bold=True)
    y += 24
    feats0 = json.loads((base["dir"] / "features.json").read_text())["features"]
    for bm in BOOKMARKS:
        eye = next((x for x in feats0 if x["name"] == bm), None)
        tow = eye["attrs"]["toward"]["name"] if eye else "?"
        text(VX, y, f"{bm}: {WHERE[bm]}, toward {tow}", size=11, mono=False)
        y += 20
        for i, (_, folder) in enumerate(cols):
            ax = axes(VX + i * (VW + 10), y, VW, VH)
            p = folder / f"{bm}.png"
            if p.exists():
                ax.imshow(mpimg.imread(p))
            else:
                ax.text(0.5, 0.5, "not rendered", ha="center", va="center", transform=ax.transAxes)
            ax.set_xticks([])
            ax.set_yticks([])
        y += VH + 8
    text(VX, y, "Protocol: 1280x720 per view (spec/build.md 15.1), Compatibility renderer, Camera3D "
         "defaults (75 deg), one sun from 225 deg at 45 deg;\nthe eye from the after build's bookmark; "
         "options water=tag:water_level, paths=kind:lane (lanes drawn as strips).",
         size=10, mono=False, colour=GREY)
    y += 34
    codes = [g[0] for g in gs]
    if any(codes):
        text(VX, y, f"sheet.sh exit {codes}", colour=BAD)
        y += LH
    views_bottom = y

    # 2. Plan view.
    PX, PY0, PW = 1500, 100, 860
    text(PX, PY0, "2  Plan, 256 m square: ford before (black), downstream (orange, dashed)",
         size=HEAD, mono=False, bold=True)
    ax = axes(PX + 30, PY0 + 40, PW - 40, PW - 40)
    plan_view(ax, base, ds)
    text(PX, PY0 + PW + 30, "Hillshade of height, contours every 1 m; blue: water less height > 1 mm; "
         "red line: clearing = 0.5;\ndots: trees (before); arrows: bookmarks, 12 m toward their target; "
         "boxes: the cottage's pad footprint.", size=10, mono=False, colour=GREY)

    # 3. Measures table under the plan.
    c = Col(PX, PY0 + PW + 74)
    c.head("3  Measures (lib/review/checks.py, the lint step through the SDK)")
    c.line(f"{'measure':<18}{'subject':<11}{'before':>9}{'downstr.':>10}{'vertex':>9}{'hand':>9}",
           bold=True)
    for key in sorted(m0):
        name, subj = key
        vals = [m.get(key) for m in (m0, m1, m2, m3)]
        c.line(f"{name:<18}{subj.split('.', 1)[1]:<11}"
               + "".join(f"{('-' if v is None else f'{v:g}'):>{w}}"
                         for v, w in zip(vals, (9, 10, 9, 9))))
    c.line("Protocol: data.value of each step.checks.* info in diagnostics.json;", colour=GREY)
    c.line("units in checks.py; a measure informs, never a target (R15).", colour=GREY)
    c.line("grade = rise over run.", colour=GREY)
    measures_bottom = c.y

    # 5 and 6 under the views, left of the measures.
    c = Col(40, views_bottom)
    c.head("5  Build times (run report totals.ms)")
    c.line(f"cold      {statistics.median(cold):>5} ms  median of {cold}")
    c.line(f"unchanged {statistics.median(warm):>5} ms  median of {warm}")
    c.line(f"Protocol: {threads} threads (default), release binary; cold = fresh")
    c.line("store and directory each run; unchanged = rebuild into the same")
    c.line("directory, warm store. The run report's own wall time; lint step")
    c.line(f"included. Laptop, not isolated: load average {load.strip().strip('{} ')}")
    c.line("  at the start (other builds were running).")
    top = sorted(fam_n, key=lambda k: -fam_ms[k])
    c.line(f"Graph: {nodes} nodes; first cold run, ms summed by family:")
    c.line("  " + ", ".join(f"{k} {fam_ms[k]}" for k in top[:4]))
    c.line("  nodes by family: " + ", ".join(f"{k} {fam_n[k]}" for k in top[:4]) + ", ...")
    left_bottom = c.y

    c = Col(760, views_bottom)
    c.head("6  Conformance (tools/conformance, --via release)")
    c.line("cases per milestone tag: " + ", ".join(f"{k} {v}" for k, v in sorted(per_m.items())))
    c.line(f"run --milestone 1: {conf['pass']} pass, {conf['fail']} fail, "
           f"{conf['pending']} pending, {conf['error']} error",
           colour="black" if not conf_bad else BAD)
    conf_bad = sorted(conf_bad, key=lambda s: (not s.startswith("fail"), s))
    for a, b in zip(conf_bad[::2], conf_bad[1::2] + [""]):
        c.line(f"  {a:<36}{b}", colour=BAD)
    c.line(f"xtask cases (owed rows/codes): {cases_status}",
           colour=BAD if "FAIL" in cases_status else "black")
    for part in cases_note.split("; "):
        c.line("  " + part)
    c.line(f"xtask: {xtask_ok} other checks ok")
    for s in q_lines:
        c.line(s)
    mid_bottom = c.y

    # 4. Change rows, full width.
    c = Col(40, max(left_bottom, mid_bottom, measures_bottom) + 6)
    c.head("4  The six change rows (docs/plan.md acceptance table): changes.json summary, then the checks")
    left = Col(40, c.y)
    right = Col(1240, c.y)
    for i, r in enumerate(rows):
        col = left if i < 3 else right
        col.line(f"{i + 1}. {r['name']}", bold=True)
        for s in r["lines"]:
            col.line("  " + s)
        for s, ok in r["checks"]:
            col.check(s, ok)
        col.y += 8
    y = max(left.y, right.y) + 4

    # 7. Track A; 8. still wrong.
    c = Col(40, y)
    c.head("7  Track A gate (cargo test -p via-geom --release)")
    for s in gate_lines(meas):
        c.line(s)
    c.line("clear removes one cell's building; a pin inside one is reported;")
    c.line("every masked piece kept: unit tests")
    c.line(f"  {geom_tests.split('; ')[1]}")
    c.line(f"via-geom tests: {geom_tests.split('; ')[0]}")
    c.line("same bytes on every system: macOS only (Linux and Windows testing", colour=BAD)
    c.line("  set aside, doctrine 2026-11-13)", colour=BAD)
    c.line("Spike numbers: fixtures/kernel/townland/reference.json")
    c.line("  (Shapely 2.2 / GEOS 3.14, floating point).")
    bottom = c.y

    c = Col(1240, y)
    c.head("8  Still wrong")
    for s in STILL_WRONG(rows, base, conf_bad, cases_status):
        c.line(s, colour=BAD)
    bottom = max(bottom, c.y) + 30

    fig.canvas.draw()
    img = np.asarray(fig.canvas.buffer_rgba())[: int(bottom)]
    plt.imsave(OUT, img)
    print(f"wrote {OUT}  ({W}x{int(bottom)}; scratch builds in {tmp})")


def STILL_WRONG(rows: list[dict], base: dict, conf_bad: list[str], cases_status: str) -> list[str]:
    d = base["dir"]
    h = np.load(d / "fields/height.npy").astype(np.float64)
    w = np.load(d / "fields/water.npy").astype(np.float64)
    wet_m2 = float(np.sum(w - h > 0.001)) * 0.25
    failed = [f"{r['name'].split(' (')[0]}: {s}" for r in rows for s, ok in r["checks"] if not ok]
    out = [f"{len(failed)} acceptance check(s) not held above" + (":" if failed else ".")]
    out += ["  " + s for s in failed]
    fails = [s.split()[1] for s in conf_bad if s.startswith("fail")]
    if fails:
        out.append(f"Conformance at milestone 1 fails: {', '.join(fails)} (section 6).")
    pend = [s for s in conf_bad if s.startswith("pending")]
    if pend:
        out.append(f"{len(pend)} conformance case(s) at milestone 1 still pending (section 6).")
    if "FAIL" in cases_status:
        out.append(f"xtask cases: {cases_status}: owed case folders (section 6).")
    out += STILL_WRONG_BY_EYE(wet_m2)
    return out


def STILL_WRONG_BY_EYE(wet_m2: float) -> list[str]:
    """Written by hand from looking at this sheet's views (2026-11-14); the one number is computed."""
    out = []
    if wet_m2 > 2000:
        out += [f"Water covers {wet_m2:.0f} m2 (the 6 m beck over ~270 m is ~1600 m2): the wet",
                "  area spreads past the channel."]
    return out + [
        "The ground is one grey: ford paints no material field, so the pack's grass",
        "  and rock are never drawn (lib/land; a `weights` field, role material).",
        "Lanes show only because the sheet asks `paths=kind:lane`: a drawing option",
        "  (build.md 15.1), not the world's ground.",
        "Trees are spheres on no trunk; the cottage a box with no door, so to_door",
        "  looks at a blank wall: primitive pack only (asset facts arrive in M2).",
        "In in_trees after hand, the pin oak and the clear gap cannot be told from",
        "  the scatter by eye: the oak is the same sphere; the views do not mark them.",
        "Nobody walked ford: the loader makes terrain collision, but no walker was",
        "  run; the views are cameras only. 'On foot' is not shown.",
        "Same bytes across systems: macOS only (set aside).",
    ]


if __name__ == "__main__":
    main()
