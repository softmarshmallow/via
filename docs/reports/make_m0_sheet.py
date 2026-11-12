# /// script
# requires-python = ">=3.11"
# dependencies = ["matplotlib>=3.9,<3.10"]
# ///
"""Draw the M0 sheet, docs/reports/m0-sheet.png, from the files and from fresh runs.

    uv run docs/reports/make_m0_sheet.py

Every number on the sheet is read from a file in the tree or from a command this script runs
(listed on the sheet with its protocol), so the sheet cannot drift from the tree it describes.
The one-line ruling summaries are written here by hand because the doctrine rows are paragraphs;
the script still checks that each ruling number exists in docs/doctrine.md.
"""

from __future__ import annotations

import datetime
import os
import platform
import re
import shutil
import subprocess
import sys
from pathlib import Path

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "docs/reports/m0-sheet.png"
PY = ROOT / "sdk/python/.venv/bin/python"

RULINGS = {
    "R20": "one binary `via`; spec/ normative; pure-Python SDK over JSON-RPC stdio",
    "R21": "a field's rows ordered by authority world-wide; layers break ties",
    "R22": "lifecycle points generate, lint, export; effect and place dropped",
    "R23": "engine calls from a step are always a batch: columns in, columns out",
    "R24": "geometry in integer mm in the core, decimal metres in text",
    "R25": "one surfaces model: a bounded field with the role `surface`",
    "R26": "same text, same bytes on macOS, Linux, Windows (one engine version)",
    "R27": "first cut: the spike rebuilt in v1 words; none of its core carries over",
    "R28": "formulas may call neighbourhood functions that declare halo and cap",
    "R29": "ties under R21: nesting, `order`, layer rank, id; never document order",
    "R30": "a pack swap changes the look and its facts, never the author's text",
    "R31": "terrain surface owned by the world; every meeting is a pair of mounts",
    "R32": "a step returns rows, features, new fields; never replaces a field",
    "R33": "pre-code placements: functions/vocabulary.toml, docs/reference, runner",
}

CRATES = [
    "via-num", "via-diag", "via-doc", "via-expr", "via-geom", "via-raster", "via-graph",
    "via-model", "via-proto", "via-step", "via-cli", "via-artifact", "via-terrain", "xtask",
]


def run(cmd: list[str], cwd: Path = ROOT, env: dict | None = None) -> tuple[int, str]:
    """Run a command, return (exit status, stdout+stderr); the sheet reports failures, not hides."""
    e = dict(os.environ)
    e.update(env or {})
    p = subprocess.run(cmd, cwd=cwd, capture_output=True, text=True, env=e)
    return p.returncode, p.stdout + p.stderr


def spec_chapters() -> list[tuple[str, int]]:
    return [(p.stem, len(p.read_text().splitlines())) for p in sorted((ROOT / "spec").glob("*.md"))]


def check_rulings() -> None:
    doctrine = (ROOT / "docs/doctrine.md").read_text()
    for r in RULINGS:
        if not re.search(rf"^\| {r} \|", doctrine, re.M):
            sys.exit(f"{r} not found as a row in docs/doctrine.md")


def limitation_counts() -> dict[str, int]:
    text = (ROOT / "docs/design/limitations.md").read_text()
    m = re.search(r"\| \*\*Total\*\* \| \*\*(\d+)\*\* \| \*\*(\d+)\*\* \| \*\*(\d+)\*\* \| \*\*(\d+)\*\* \| \*\*(\d+)\*\* \|", text)
    if not m:
        sys.exit("limitations.md: Counts total row not found")
    keys = ["bullets", "rows", "fix", "accept", "drop"]
    return dict(zip(keys, map(int, m.groups())))


def oracle() -> tuple[int, int]:
    names = [p.name for p in (ROOT / "fixtures/oracle").iterdir() if p.is_dir()]
    edits = sum("@" in n for n in names)
    return len(names), edits


def crate_tests() -> list[tuple[str, int, int, int]]:
    """Per crate: passed, failed, ignored, summed over its unit, integration and doc tests."""
    rows = []
    for c in CRATES:
        _, out = run(["cargo", "test", "-p", c])
        totals = [0, 0, 0]
        for m in re.finditer(r"test result: \w+\. (\d+) passed; (\d+) failed; (\d+) ignored", out):
            for i in range(3):
                totals[i] += int(m.group(i + 1))
        rows.append((c, *totals))
    return rows


def py_tests(folder: Path) -> str:
    code, out = run(["uv", "run", "python", "-m", "unittest", "discover", "-s", "tests"], cwd=folder)
    m = re.search(r"Ran (\d+) tests?", out)
    n = m.group(1) if m else "?"
    return f"{n} {'passed' if code == 0 else 'FAILED'}"


def conformance() -> tuple[dict[str, int], str, list[str]]:
    cdir = ROOT / "tools/conformance"
    _, out = run(["uv", "run", "python", "-m", "via_conformance", "list"], cwd=cdir)
    per = {m.group(1): int(m.group(2)) for m in re.finditer(r"^(M\d+): (\d+) cases", out, re.M)}
    _, out = run(["uv", "run", "python", "-m", "via_conformance", "run", "--milestone", "0"], cwd=cdir)
    summary = re.search(r"^\d+ passed.*$", out, re.M)
    lines = [l for l in out.splitlines() if re.match(r"^(pass|fail|pending|error)\s+build/c(01|39|40)", l)]
    return per, summary.group(0) if summary else "no summary", lines


def probe() -> dict[str, str]:
    _, out = run(["cargo", "run", "-q", "-p", "via-proto", "--release", "--example", "probe", "--",
                  "--python", str(PY), "--runs", "3"])
    want = {
        "start": r"start \+ initialize\s+([\d.]+ ms)",
        "field": r"6\.5 MB field out and back\s+([\d.]+ ms)",
        "call": r"empty batch call, round trip\s+([\d.]+ us)\s+\((p99 [\d.]+ us; n = \d+)",
        "map": r"build\.field \+ numpy map\s+([\d.]+ us)\s+\((p99 [\d.]+ us; n = \d+)",
        "bare": r"numpy\.load\(mmap\) alone\s+([\d.]+ us)",
        "engine": r"engine Store::map\s+([\d.]+ us)",
        "killed": r"(killed mid-call after [\d.]+ ms)",
        "code": r"error\[(protocol\.\w+)\]",
        "left": r"call folders left: (\d+)",
    }
    got = {}
    for k, rx in want.items():
        m = re.search(rx, out)
        if not m:
            got[k] = "not found"
        elif k in ("call", "map"):
            got[k] = f"{m.group(1)} ({m.group(2)})"
        else:
            got[k] = m.group(1)
    got["alive"] = "none" if out.count("alive after cleanup: false") == 2 else "CHECK"
    _, t = run(["cargo", "test", "-q", "-p", "via-proto", "--release", "--", "--include-ignored"])
    res = re.findall(r"test result: \w+\. (\d+) passed; (\d+) failed", t)
    got["tests"] = f"{sum(int(a) for a, _ in res)} passed, {sum(int(b) for _, b in res)} failed"
    _, v = run([str(PY), "-c", "import sys,numpy;print(sys.version.split()[0],numpy.__version__)"])
    got["py"] = v.strip()
    return got


def cross_system() -> dict[str, str]:
    text = (ROOT / "docs/reports/m0-cross-system.md").read_text()
    status = re.search(r"Status: \*\*(\w+)\*\*", text).group(1)
    digest = re.search(r"\| macOS arm64 \| `([0-9a-f]{64})` \| (\d+) \|", text)
    recorded = (ROOT / "spec/vectors/num/null_digest.txt").read_text()
    early = re.search(r"start \+ initialize ([\d.]+ ms); 6\.5 MB field out and back ([\d.]+ ms)", text)
    return {
        "status": status,
        "digest": digest.group(1),
        "lines": digest.group(2),
        "matches": "yes" if digest.group(1) in recorded else "NO",
        "early_start": early.group(1) if early else "?",
        "early_field": early.group(2) if early else "?",
    }


def gate() -> list[tuple[str, str]]:
    rows = []
    code, _ = run(["cargo", "fmt", "--all", "--", "--check"])
    rows.append(("cargo fmt --check", "pass" if code == 0 else "FAIL"))
    code, _ = run(["cargo", "clippy", "--workspace", "--all-targets", "--", "-D", "warnings"])
    rows.append(("cargo clippy -D warnings", "pass" if code == 0 else "FAIL"))
    code, out = run(["cargo", "test", "--workspace"])
    t = [0, 0, 0]
    for m in re.finditer(r"test result: \w+\. (\d+) passed; (\d+) failed; (\d+) ignored", out):
        for i in range(3):
            t[i] += int(m.group(i + 1))
    rows.append(("cargo test --workspace", f"{t[0]} passed, {t[1]} failed, {t[2]} ignored"))
    code, out = run(["cargo", "run", "-q", "-p", "xtask", "--", "check"])
    for m in re.finditer(r"^([a-z-]+): (ok|SKIPPED|FAIL\w*|failed.*)$", out, re.M):
        rows.append((f"xtask {m.group(1)}", m.group(2)))
    rows.append(("xtask check exit status", str(code)))
    return rows


def main() -> None:
    check_rulings()
    chapters = spec_chapters()
    lim = limitation_counts()
    n_oracle, n_edits = oracle()
    print("running crate tests ...", flush=True)
    crates = crate_tests()
    runner_self = py_tests(ROOT / "tools/conformance")
    sdk = py_tests(ROOT / "sdk/python")
    per_m, conf_summary, conf_lines = conformance()
    print("running the probe ...", flush=True)
    pr = probe()
    cs = cross_system()
    print("running the gate ...", flush=True)
    gt = gate()
    free_gib = shutil.disk_usage("/").free / 2**30
    _, rustc = run(["rustc", "--version"])
    _, head = run(["git", "rev-parse", "--short", "HEAD"])
    _, porcelain = run(["git", "status", "--porcelain"])
    today = datetime.date.today().isoformat()
    machine = f"{platform.system()} {platform.release()} {platform.machine()}"

    W, H, DPI = 2400, 1600, 100
    fig = plt.figure(figsize=(W / DPI, H / DPI), dpi=DPI, facecolor="white")
    BODY, HEAD, MONO = 12, 15, "DejaVu Sans Mono"  # 12 pt = 16.7 px, 15 pt = 20.8 px at 100 dpi
    LH = 27  # line height, px

    class Col:
        def __init__(self, x: int, y: int = 92):
            self.x, self.y = x, y

        def head(self, s: str) -> None:
            self.y += 18
            fig.text(self.x / W, 1 - self.y / H, s, fontsize=HEAD, weight="bold", va="top")
            self.y += 36

        def line(self, s: str, bold: bool = False, color: str = "black") -> None:
            fig.text(self.x / W, 1 - self.y / H, s, fontsize=BODY, family=MONO, va="top",
                     weight="bold" if bold else "normal", color=color)
            self.y += LH

        def gap(self, px: int = 10) -> None:
            self.y += px

    fig.text(40 / W, 1 - 28 / H,
             f"via M0 sheet   branch rewrite at {head.strip()} + uncommitted M0 work "
             f"({len(porcelain.splitlines())} paths); nothing committed   {today}",
             fontsize=18, weight="bold", va="top")
    fig.text(40 / W, 1 - 60 / H,
             f"Measured by docs/reports/make_m0_sheet.py on {machine}, {rustc.strip()}, "
             f"Python {pr['py'].split()[0]} numpy {pr['py'].split()[-1]}. Every number below is "
             "read from a file in the tree or from a command the script ran.",
             fontsize=BODY, va="top", color="#333333")

    # Column 1: what M0 delivered.
    c = Col(40)
    c.head(f"1  What M0 delivered: spec/ ({len(chapters)} chapters, "
           f"{sum(n for _, n in chapters)} lines)")
    half = (len(chapters) + 1) // 2
    for i in range(half):
        a = chapters[i]
        s = f"{a[0] + '.md':<16}{a[1]:>5}"
        if i + half < len(chapters):
            b = chapters[i + half]
            s += f"      {b[0] + '.md':<16}{b[1]:>5}"
        c.line(s)
    c.head("Rulings R20-R33 (docs/doctrine.md)")
    for r in sorted(RULINGS, key=lambda k: int(k[1:])):
        c.line(f"{r} {RULINGS[r]}")
    c.head("Spike limitations sorted (docs/design/limitations.md)")
    c.line(f"{lim['bullets']} source bullets, {lim['rows']} rows")
    c.line(f"fix {lim['fix']}    accept {lim['accept']}    drop {lim['drop']}", bold=True)
    c.head("Oracle builds exported (fixtures/oracle)")
    c.line(f"{n_oracle} folders: {n_oracle - n_edits} base worlds, {n_edits} edits/variants (name@edit)")

    # Column 2: the code, conformance cases and the probe.
    c = Col(830)
    c.head("2  The code: crates and their tests (cargo test -p <crate>)")
    c.line(f"{'crate':<14}{'passed':>8}{'failed':>8}{'ignored':>9}", bold=True)
    for name, p, f, i in crates:
        c.line(f"{name:<14}{p:>8}{f:>8}{i:>9}", color="black" if p or i else "#777777")
    c.line(f"{'total':<14}{sum(r[1] for r in crates):>8}{sum(r[2] for r in crates):>8}"
           f"{sum(r[3] for r in crates):>9}", bold=True)
    c.line("ignored = via-proto probe tests (need the Python venv)")
    c.line(f"Python: conformance runner self-tests {runner_self}; SDK {sdk}")
    c.head("Conformance cases per milestone tag (spec/cases)")
    c.line("   ".join(f"{k}: {v}" for k, v in sorted(per_m.items())) + f"   total {sum(per_m.values())}")
    c.line(f"run --milestone 0: {conf_summary.replace(' (milestone 0)', '')}")
    c.line("  (pending/skipped above include the M1 cases)")
    for l in conf_lines:
        c.line("  " + re.sub(r"\s+", " ", l).split(" (")[0])
    c.head("3  Protocol probe (protocol.md case 0), median of 3 runs")
    c.line(f"start + initialize           {pr['start']}")
    c.line(f"6.5 MB field out and back    {pr['field']}")
    c.line(f"empty batch call             {pr['call']}")
    c.line(f"build.field + numpy map      {pr['map']}")
    c.line(f"numpy.load(mmap) alone       {pr['bare']}")
    c.line(f"engine Store::map            {pr['engine']}")
    c.line(f"kill mid-call: {pr['code']}, pids left {pr['alive']}, call folders {pr['left']}")
    c.line(f"probe tests --include-ignored: {pr['tests']}")
    c.line(f"earlier run (m0-cross-system.md): start {cs['early_start']}, field {cs['early_field']}",
           color="#9a3a00")
    c.line("  same command; the spread is not explained yet", color="#9a3a00")
    c.line("protocol: --release, --runs 3, wall clock; per-call times taken")
    c.line("  in the worker (perf_counter_ns); laptop, machine not isolated")

    # Column 3: cross-system, gate, open questions.
    a, b = (float(x.split()[0]) for x in (pr["field"], cs["early_field"]))
    spread = f"{min(a, b):.0f} vs {max(a, b):.0f} ms" if a and b else "unmeasured"
    c = Col(1640)
    c.head("4  Cross-system (docs/reports/m0-cross-system.md)")
    c.line(f"status: {cs['status']}", bold=True, color="#9a3a00")
    c.line(f"macOS arm64 null digest ({cs['lines']} lines hashed):")
    c.line(f"  {cs['digest'][:32]}")
    c.line(f"  {cs['digest'][32:]}")
    c.line(f"matches spec/vectors/num/null_digest.txt: {cs['matches']}")
    c.line("Linux arm64 and amd64: not run. Docker Desktop")
    c.line("stopped when the host disk filled (140 MiB free).")
    c.line(f"Free on / now: {free_gib:.0f} GiB.")
    c.head("5  Gate (run by this script)")
    for k, v in gt:
        ok = v in ("pass", "ok", "0") or " 0 failed" in v
        c.line(f"{k:<26}{v}", color="black" if ok else "#9a3a00")
    c.line("private-names: no pattern file, nothing searched")
    c.head("6  Open questions for the owner")
    for q in [
        "Windows proof needs the branch pushed to hosted",
        "  runners (.github/workflows/ci.yml). Allow a push?",
        "Linux side of R26: rerun in the container now that",
        "  the disk has room; until then R26 is macOS only.",
        "private-names check skipped: write the patterns to",
        "  spike/local/private-patterns.txt or set",
        "  VIA_PRIVATE_PATTERNS before any commit.",
        "build/c01_null pending: its expected manifest and",
        "  id (the null vector, M0 step 6) are not recorded.",
        "Probe: a relative --python path fails with",
        f"  protocol.start; field round trip {spread}.",
        "Limitations W3, E3: M1 fixes proven by no case.",
    ]:
        c.line(q)

    fig.savefig(OUT, dpi=DPI, facecolor="white")
    print(f"wrote {OUT}")


if __name__ == "__main__":
    main()
