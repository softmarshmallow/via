#!/usr/bin/env python3
"""Drive the corpus acquisition: fetch every pre-registered town.

A thin, resumable driver over ``fetch_extract.py`` — one subprocess per
town, so the single-town script remains the only code path that touches
the manifest (ADR 0010 Decision 4). This driver adds only what a
132-town unattended run needs:

- the town list is parsed from ``reference/pre-registration.md`` (class
  from the ``###`` section heading, centre from the table row); a town
  count that does not match ``--expect`` aborts before any traffic;
- pilots and towns already present in the manifest are skipped, so the
  run is resumable at any point;
- Overpass etiquette: sequential fetches with a fixed pause between
  towns; on 429 the public status endpoint is polled until a slot
  frees; on 504 (or a partial-result remark) the fetch radius steps
  down 750 → 550 → 450 m — 435 m is the protocol floor (v1.1 P2), so
  450 is the last rung (the Kibera precedent);
- every attempt is logged with a timestamp; failures are collected and
  reported at the end, they never stop the run.

The attic date defaults to the pilot snapshot (2026-08-20T12:00:00Z) so
the whole corpus reads one OSM state.

Usage (from the repo root):

    uv run --project analysis python analysis/fetch_corpus.py \
        --archive-dir ~/Documents/via-reference-archive [--dry-run]
"""

import argparse
import datetime as dt
import json
import re
import subprocess
import sys
import time
from pathlib import Path

import requests

from fetch_extract import PILOT_TOWNS, REPO_ROOT, repo_path

PRE_REGISTRATION = "reference/pre-registration.md"
MANIFEST = "reference/extracts.jsonl"
ATTIC_DEFAULT = "2026-08-20T12:00:00Z"
RADIUS_LADDER = (750.0, 550.0, 450.0)
CLASSES = (
    "organic pre-modern core",
    "planted pre-modern grid",
    "19th-century survey plat",
    "mid-20th-century suburb",
    "contemporary informal",
)
STATUS_ENDPOINT = "https://overpass-api.de/api/status"


def parse_sample(text: str) -> list[dict]:
    """Rows of the five class tables, with the class from the heading.

    Only a ``###`` heading that IS one of the five class names starts a
    class table — the "… — origins and study centres" companions and
    the Limitations subsections must not match.
    """
    towns: list[dict] = []
    current: str | None = None
    cols: dict[str, int] | None = None
    for line in text.splitlines():
        m = re.match(r"^###\s+(.*?)\s*$", line)
        if m:
            heading = m.group(1)
            current = heading if heading in CLASSES else None
            cols = None
            continue
        if current is None or "|" not in line:
            cols = None if "|" not in line else cols
            continue
        cells = [c.strip() for c in line.strip().strip("|").split("|")]
        lowered = [c.lower() for c in cells]
        if "town" in lowered and "centre" in lowered:
            cols = {
                name: lowered.index(name)
                for name in ("town", "centre", "partition", "coverage")
            }
            continue
        if cols is None or all(set(c) <= set("-: ") for c in cells):
            continue
        town = cells[cols["town"]].lower()
        centre = cells[cols["centre"]]
        cm = re.match(r"^(-?\d+\.\d+),\s*(-?\d+\.\d+)$", centre)
        if not cm:
            raise ValueError(f"unparseable centre {centre!r} for town {town!r}")
        towns.append(
            {
                "town": town,
                "town_class": current,
                "lat": float(cm.group(1)),
                "lon": float(cm.group(2)),
                "partition": cells[cols["partition"]],
                "coverage": cells[cols["coverage"]],
            }
        )
    return towns


def manifest_towns(path: Path) -> set[str]:
    if not path.exists():
        return set()
    return {
        json.loads(line)["town"].lower()
        for line in path.read_text(encoding="utf-8").splitlines()
        if line.strip()
    }


def wait_for_slot(log, max_wait_s: float = 900.0) -> None:
    """Poll the Overpass status endpoint until a slot is available."""
    waited = 0.0
    while waited < max_wait_s:
        try:
            status = requests.get(
                STATUS_ENDPOINT,
                timeout=30,
                headers={"User-Agent": "via-benchmark/0.1 (reference corpus)"},
            ).text
            if "slots available" in status and not re.search(
                r"\b0 slots available", status
            ):
                return
        except requests.exceptions.RequestException:
            pass
        log("rate-limited: no slot; waiting 30 s")
        time.sleep(30.0)
        waited += 30.0


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--archive-dir", required=True)
    ap.add_argument("--date", default=ATTIC_DEFAULT)
    ap.add_argument("--pause", type=float, default=10.0, help="seconds between towns")
    ap.add_argument("--expect", type=int, default=138, help="registered town count")
    ap.add_argument("--limit", type=int, default=None, help="fetch at most N towns")
    ap.add_argument("--dry-run", action="store_true")
    ap.add_argument(
        "--log", default="runs/reference/fetch-corpus.log", help="append-only run log"
    )
    args = ap.parse_args()

    towns = parse_sample(repo_path(PRE_REGISTRATION).read_text(encoding="utf-8"))
    if len(towns) != args.expect:
        print(
            f"error: parsed {len(towns)} towns, expected {args.expect} — "
            "refusing to run on a partial parse",
            file=sys.stderr,
        )
        return 1

    done = manifest_towns(repo_path(MANIFEST))
    todo = [t for t in towns if t["town"] not in PILOT_TOWNS and t["town"] not in done]
    per_class = {c: sum(1 for t in todo if t["town_class"] == c) for c in CLASSES}
    print(
        f"{len(towns)} registered, {len(done)} in manifest, {len(todo)} to fetch: "
        + ", ".join(f"{v} {k.split()[0]}" for k, v in per_class.items())
    )
    if args.limit is not None:
        todo = todo[: args.limit]
    if args.dry_run:
        for t in todo:
            print(
                f"  {t['town']:24s} {t['town_class']:26s} "
                f"{t['lat']:.4f},{t['lon']:.4f}  {t['partition']}"
            )
        return 0

    log_path = repo_path(args.log)
    log_path.parent.mkdir(parents=True, exist_ok=True)

    def log(msg: str) -> None:
        stamp = dt.datetime.now(dt.timezone.utc).strftime("%H:%M:%SZ")
        line = f"[{stamp}] {msg}"
        print(line, flush=True)
        with open(log_path, "a", encoding="utf-8") as fh:
            fh.write(line + "\n")

    failures: list[tuple[str, str]] = []
    for i, t in enumerate(todo, 1):
        fetched = False
        for radius in RADIUS_LADDER:
            cmd = [
                sys.executable,
                str(REPO_ROOT / "analysis" / "fetch_extract.py"),
                "--town", t["town"],
                "--town-class", t["town_class"],
                f"--lat={t['lat']}",
                f"--lon={t['lon']}",
                f"--fetch-radius={radius:g}",
                "--date", args.date,
                "--archive-dir", args.archive_dir,
                "--partition", t["partition"],
            ]
            transient_retries = 0
            while True:
                proc = subprocess.run(cmd, capture_output=True, text=True)
                out = (proc.stdout + proc.stderr).strip().replace("\n", " | ")
                log(f"[{i}/{len(todo)}] {t['town']} r={radius:g}: exit {proc.returncode} — {out}")
                if proc.returncode == 4 and transient_retries < 8:
                    transient_retries += 1
                    wait_for_slot(log)
                    continue
                break
            if proc.returncode == 0:
                fetched = True
                break
            if proc.returncode == 3:
                continue  # too heavy — next rung of the ladder
            failures.append((t["town"], out))
            break
        if not fetched and (not failures or failures[-1][0] != t["town"]):
            failures.append((t["town"], "radius ladder exhausted (dense area)"))
        time.sleep(args.pause)

    log(f"done: {len(todo) - len(failures)}/{len(todo)} fetched, {len(failures)} failed")
    for town, why in failures:
        log(f"FAILED {town}: {why}")
    return 2 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
