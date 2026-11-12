"""Regression check: compile every example world and compare what comes out with a recorded baseline.

    uv run python check.py            compare with golden.json
    uv run python check.py --record   record a new baseline

Only the committed primitive pack is used, so the check runs anywhere.
"""
import hashlib
import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).parent
CASES = [
    ("vale", ["worlds/vale/world.toml"]),
    ("vale_winter", ["worlds/vale/world.toml", "--variant", "winter"]),
    ("vale_invalid", ["worlds/vale/world.toml", "--overlay", "worlds/vale/changes/invalid.toml"]),
    ("station", ["worlds/station/world.toml"]),
    ("college", ["worlds/college/world.toml", "--lint", "reach"]),
    ("coast", ["worlds/coast/world.toml", "--lint", "reach", "--map"]),
    ("relief", ["worlds/relief/world.toml"]),
    ("example", ["worlds/example/world.toml", "--lint", "reach"]),      # the canonical project: one of everything          # fields read from pictures; run make_inputs.py first
    ("pistol", ["worlds/pistol/world.toml"]),          # formulas as attributes, a free solid, a measuring lint
    ("glade", ["worlds/glade/world.toml", "--variant", "by_cause"]),     # clumps and growth by cause
    ("monastery", ["worlds/monastery/world.toml", "--lint", "reach"]),  # written by another agent from the documents alone
    ("strait", ["worlds/strait/world.toml", "--overlay", "worlds/strait/changes/move_b.toml"]),  # two worlds joined; the master owns the bridge
    ("border", ["worlds/border/world.toml", "--lint", "reach", "--overlay", "worlds/border/changes/move_college.toml"]),  # joined by ground; the master's join layer owns the ridge and its openings
    ("basin", ["worlds/basin/world.toml", "--overlay", "worlds/basin/changes/move_haven.toml"]),  # land simulated under stated facts; needs the Rust engine: cargo build --release -p via
    ("townland", ["worlds/townland/world.toml"]),  # a town on simulated land, main ways found by their ends; needs the Rust engine
]
SKIP = {"minimap.png", "reach.png", "changes.json"}          # pictures whose bytes depend on the imaging library's version


def digest(case: str) -> dict:
    out = {}
    for p in sorted((ROOT / "build" / f"check_{case}").rglob("*")):
        if p.is_file() and p.name not in SKIP and p.suffix != ".png":
            out[str(p.relative_to(ROOT / "build" / f"check_{case}"))] = hashlib.sha1(p.read_bytes()).hexdigest()[:16]
    return out


def main() -> int:
    record = "--record" in sys.argv
    golden_path = ROOT / "golden.json"
    golden = {} if record or not golden_path.exists() else json.loads(golden_path.read_text())
    now, bad = {}, 0
    for case, args in CASES:
        run = subprocess.run([sys.executable, "-m", "worldc", *args, "--out", f"check_{case}"], cwd=ROOT, capture_output=True, text=True)
        summary = [l for l in run.stdout.splitlines() if "errors," in l]
        now[case] = {"exit": run.returncode, "summary": summary[-1].rsplit(",", 1)[0] if summary else run.stderr[-200:], "files": digest(case)}
        if record:
            print(f"recorded {case}: {now[case]['summary']}, {len(now[case]['files'])} files")
            continue
        want = golden.get(case)
        if want is None:
            print(f"NEW      {case}: no baseline")
            bad += 1
            continue
        diff = sorted(k for k in set(want["files"]) | set(now[case]["files"]) if want["files"].get(k) != now[case]["files"].get(k))
        same = not diff and want["exit"] == now[case]["exit"] and want["summary"] == now[case]["summary"]
        print(f"{'same    ' if same else 'DIFFERS '} {case}: {now[case]['summary']}" + ("" if same else f"   was: {want['summary']}   files: {', '.join(diff[:8])}"))
        bad += 0 if same else 1
    if record:
        golden_path.write_text(json.dumps(now, indent=1))
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
