#!/usr/bin/env python3
"""Measure every manifest extract with via-bench under one protocol id.

The corpus counterpart of ``fetch_corpus.py``: loops the provenance
manifest (``reference/extracts.jsonl``), and for each town

- verifies the extract file's blake3 against its manifest record
  before measuring — a hash mismatch is corruption and the town is
  reported, never measured (ADR 0008 D11);
- runs ``via-bench reference`` with the centre, name and class from
  the manifest record, so no coordinate is ever typed twice;
- renders panels on the class's operative street set (protocol v1.1
  P4: all_ways for contemporary informal, carriageway otherwise) —
  measurement always covers both sets regardless;
- skips towns whose ``<town>-fabric.json`` already exists in the
  output directory, so the run is resumable.

The six pilots are re-measured with everyone else: the corpus then
carries one code revision stamp end to end (the Phase B pilot record
in runs/reference/measured-v1.1/ stands unchanged as its own record).

Usage (from the repo root, after ``cargo build --release``):

    uv run --project analysis python analysis/measure_corpus.py \
        [--out runs/reference/corpus-v1.1] [--dry-run]
"""

import argparse
import json
import subprocess
import sys

from fetch_extract import REPO_ROOT, blake3_file, repo_path

MANIFEST = "reference/extracts.jsonl"
BENCH = REPO_ROOT / "target" / "release" / "via-bench"
INFORMAL_CLASS = "contemporary informal"


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--out", default="runs/reference/corpus-v1.1")
    ap.add_argument("--manifest", default=MANIFEST)
    ap.add_argument("--protocol-id", default="v1.1")
    ap.add_argument("--dry-run", action="store_true")
    args = ap.parse_args()

    if not BENCH.exists():
        print(f"error: {BENCH} not built — cargo build --release first", file=sys.stderr)
        return 1

    records = [
        json.loads(line)
        for line in repo_path(args.manifest).read_text(encoding="utf-8").splitlines()
        if line.strip()
    ]
    seen: dict[str, dict] = {}
    for r in records:
        if r["town"] in seen:
            print(
                f"error: manifest holds more than one record for {r['town']!r} — "
                "resolve which extract is the corpus one before measuring",
                file=sys.stderr,
            )
            return 1
        seen[r["town"]] = r

    out_dir = repo_path(args.out)
    todo = [r for r in records if not (out_dir / f"{r['town']}-fabric.json").exists()]
    print(f"{len(records)} manifest records, {len(todo)} to measure -> {args.out}")
    if args.dry_run:
        for r in todo:
            print(f"  {r['town']:24s} {r['class']}")
        return 0

    failures: list[tuple[str, str]] = []
    for i, r in enumerate(todo, 1):
        extract = repo_path(r["file"])
        if not extract.exists():
            failures.append((r["town"], f"extract missing: {r['file']}"))
            continue
        digest = blake3_file(extract)
        if digest != r["blake3"]:
            failures.append(
                (r["town"], f"blake3 mismatch: file {digest[:16]}… vs manifest "
                 f"{r['blake3'][:16]}… — refusing to measure a corrupted extract")
            )
            continue
        render_set = "all_ways" if r["class"] == INFORMAL_CLASS else "carriageway"
        lat, lon = r["centre"]
        cmd = [
            str(BENCH), "reference", str(extract),
            f"--centre={lat},{lon}",
            "--name", r["town"],
            "--town-class", r["class"],
            "--protocol-id", args.protocol_id,
            "--render-set", render_set,
            "--out", str(out_dir),
        ]
        proc = subprocess.run(cmd, capture_output=True, text=True)
        head = proc.stdout.strip().splitlines()
        print(f"[{i}/{len(todo)}] {head[0] if head else r['town']}", flush=True)
        if proc.returncode != 0:
            failures.append((r["town"], (proc.stderr or proc.stdout).strip()[-300:]))

    print(f"done: {len(todo) - len(failures)}/{len(todo)} measured, {len(failures)} failed")
    for town, why in failures:
        print(f"FAILED {town}: {why}")
    return 2 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
