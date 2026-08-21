#!/usr/bin/env python3
"""Run both cross-validation levels over the measured corpus.

For every town in the provenance manifest and both P4 street sets:

- **Level 1** (``tier_a_check.py``): networkx re-measurement of
  via-bench's exported graph; agreement must be exact (≤ 1e-6). A
  failure is a formula defect, full stop.
- **Level 2** (``cross_validate.py --envelopes``): independent osmnx
  construction from the raw extract; divergence beyond a protocol
  v1.1 envelope is reported as a violation. The envelopes were fitted
  on six pilot towns, so a corpus violation is an investigation
  trigger — the protocol calls it a defect until shown otherwise, and
  this driver's job is to surface every one, never to relax one.

Failures and violations are collected and printed at the end; the run
never stops early. Towns are processed in parallel subprocesses.

Usage (from the repo root, after measure_corpus.py):

    uv run --project analysis python analysis/validate_corpus.py \
        [--measured runs/reference/corpus-v1.1] [--jobs 4] [--level 1|2|both]
"""

import argparse
import json
import subprocess
import sys
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

from fetch_extract import REPO_ROOT, repo_path

MANIFEST = "reference/extracts.jsonl"
ENVELOPES = "reference/envelopes-v1.1.json"
SETS = ("carriageway", "all_ways")


def run_town(r: dict, measured: Path, out: Path, level: str) -> list[str]:
    """Both checks for one town; returns problem descriptions."""
    town = r["town"]
    lat, lon = r["centre"]
    fabric = measured / f"{town}-fabric.json"
    graph = measured / f"{town}-graph.json"
    problems: list[str] = []
    if not fabric.exists():
        return [f"{town}: not measured ({fabric.name} missing)"]

    if level in ("1", "both"):
        p = subprocess.run(
            [
                sys.executable, str(REPO_ROOT / "analysis" / "tier_a_check.py"),
                "--graph-json", str(graph),
                "--bench-json", str(fabric),
                "--street-set", "both",
            ],
            capture_output=True, text=True,
        )
        if p.returncode != 0:
            tail = (p.stdout + p.stderr).strip().splitlines()[-6:]
            problems.append(f"{town} LEVEL1: " + " | ".join(tail))

    if level in ("2", "both"):
        for street_set in SETS:
            cmp_out = out / f"{town}-crossval-{street_set}.json"
            p = subprocess.run(
                [
                    sys.executable, str(REPO_ROOT / "analysis" / "cross_validate.py"),
                    "--extract", str(repo_path(r["file"])),
                    f"--centre={lat},{lon}",
                    "--bench-json", str(fabric),
                    "--street-set", street_set,
                    "--envelopes", str(repo_path(ENVELOPES)),
                    "--out", str(cmp_out),
                ],
                capture_output=True, text=True,
            )
            if p.returncode == 2:
                lines = [
                    ln for ln in p.stdout.splitlines() if "VIOLATION" in ln.upper()
                ] or (p.stdout + p.stderr).strip().splitlines()[-4:]
                problems.append(
                    f"{town} LEVEL2 {street_set}: " + " | ".join(ln.strip() for ln in lines)
                )
            elif p.returncode != 0:
                tail = (p.stdout + p.stderr).strip().splitlines()[-4:]
                problems.append(
                    f"{town} LEVEL2 {street_set} ERROR: " + " | ".join(tail)
                )
    return problems


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--measured", default="runs/reference/corpus-v1.1")
    ap.add_argument("--manifest", default=MANIFEST)
    ap.add_argument("--level", choices=("1", "2", "both"), default="both")
    ap.add_argument("--jobs", type=int, default=4)
    args = ap.parse_args()

    records = [
        json.loads(line)
        for line in repo_path(args.manifest).read_text(encoding="utf-8").splitlines()
        if line.strip()
    ]
    measured = repo_path(args.measured)
    out = measured  # crossval JSONs live beside the fabric JSONs

    all_problems: list[str] = []
    done = 0
    with ThreadPoolExecutor(max_workers=args.jobs) as pool:
        for problems in pool.map(
            lambda r: run_town(r, measured, out, args.level), records
        ):
            done += 1
            print(f"[{done}/{len(records)}] {'OK' if not problems else 'PROBLEMS'}", flush=True)
            all_problems.extend(problems)

    print(f"\n{len(records)} towns checked, {len(all_problems)} problems")
    for p in all_problems:
        print(f"  {p}")
    return 2 if all_problems else 0


if __name__ == "__main__":
    sys.exit(main())
