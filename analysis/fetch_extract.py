#!/usr/bin/env python3
"""Fetch one town's Overpass extract with full provenance (ADR 0010 Decision 4).

Why this exists: every reference number in spikes/townfabric/VALIDATION.md
was measured on extracts that lived only in the gitignored runs/ tree, with
no fetch date recorded. The extracts are gone and not one number can be
re-measured. ADR 0010 Decision 4 is the rule that prevents a recurrence,
and this script is its fetch half:

- the query pins the data timestamp with an attic ``[date:"..."]`` clause,
  so a lost extract is content-reproducible from its manifest record;
- one JSON line per fetch is appended to the manifest
  (``reference/extracts.jsonl``): town, class, centre, radius, the exact
  query, endpoint, generator, ``osm_base``, fetch date, blake3 of the file
  bytes, licence, partition;
- the extract is stored in TWO locations before success is reported — the
  runs/ tree and ``--archive-dir`` — verified by hash, because "loss is
  designed against, not just detected";
- an existing extract file is never overwritten: a re-fetch is a new
  extract with its own date, filename, and manifest record.

The query shape matches the spike precedent (spikes/townfabric/VALIDATION.md):
highway and building ways within ``around:<radius>`` of the study centre,
with node completion, POSTed to the Overpass API.

Data is © OpenStreetMap contributors, ODbL. Extracts are never
redistributed from this repository (ADR 0010 Decision 5).

Usage (from the repo root):

    uv run --project analysis python analysis/fetch_extract.py \
        --town alnwick --town-class "organic pre-modern core" \
        --lat 55.4147 --lon -1.7061 \
        --archive-dir /path/to/off-repo/archive

Relative paths are resolved against the repository root (the parent of
``analysis/``), never against the current working directory.
"""

import argparse
import datetime as dt
import json
import shutil
import sys
from pathlib import Path

import blake3
import requests

REPO_ROOT = Path(__file__).resolve().parent.parent

QUERY_TEMPLATE = (
    '[out:json][timeout:180][date:"{date}"];'
    '(way["highway"](around:{radius},{lat},{lon});'
    'way["building"](around:{radius},{lat},{lon}););'
    "(._;>;);out body;"
)


def repo_path(p: str) -> Path:
    """Resolve a possibly-relative path against the repo root."""
    path = Path(p)
    return path if path.is_absolute() else REPO_ROOT / path


def parse_utc_date(text: str) -> dt.datetime:
    """Parse an ISO8601 date and normalise it to UTC."""
    parsed = dt.datetime.fromisoformat(text.replace("Z", "+00:00"))
    if parsed.tzinfo is None:
        parsed = parsed.replace(tzinfo=dt.timezone.utc)
    return parsed.astimezone(dt.timezone.utc)


def blake3_file(path: Path) -> str:
    hasher = blake3.blake3()
    with open(path, "rb") as fh:
        while chunk := fh.read(1 << 20):
            hasher.update(chunk)
    return hasher.hexdigest()


def main() -> int:
    ap = argparse.ArgumentParser(
        description="Fetch one town's Overpass extract with ADR 0010 provenance."
    )
    ap.add_argument("--town", required=True, help="short town name, used in the filename")
    ap.add_argument(
        "--town-class", required=True, help="reference class per research 0012 §5.4"
    )
    ap.add_argument("--lat", type=float, required=True, help="study centre latitude")
    ap.add_argument("--lon", type=float, required=True, help="study centre longitude")
    ap.add_argument(
        "--fetch-radius", type=float, default=750.0,
        help="Overpass around: radius in metres (default 750, the spike precedent)",
    )
    ap.add_argument(
        "--endpoint", default="https://overpass-api.de/api/interpreter",
        help="Overpass API endpoint",
    )
    ap.add_argument(
        "--date", default=None,
        help="ISO8601 UTC attic date for the [date:...] clause "
        "(default: the current UTC hour). This is what makes a lost "
        "extract content-reproducible.",
    )
    ap.add_argument(
        "--out-dir", default="runs/reference/osm",
        help="where the extract file is written (default runs/reference/osm, "
        "relative to the repo root)",
    )
    ap.add_argument(
        "--manifest", default="reference/extracts.jsonl",
        help="append-only provenance manifest (default reference/extracts.jsonl)",
    )
    ap.add_argument(
        "--archive-dir", required=True,
        help="REQUIRED second storage location (ADR 0010 Decision 4: an "
        "extract exists in at least two places before a number measured "
        "on it is published)",
    )
    ap.add_argument(
        "--partition", default="fitted",
        help='partition per the pre-registration document (default "fitted", '
        "the pilot towns' permanent assignment under ADR 0010 Decision 2)",
    )
    args = ap.parse_args()

    if args.date is None:
        attic = dt.datetime.now(dt.timezone.utc).replace(
            minute=0, second=0, microsecond=0
        )
    else:
        attic = parse_utc_date(args.date)
    attic_iso = attic.strftime("%Y-%m-%dT%H:%M:%SZ")
    attic_compact = attic.strftime("%Y%m%dT%H%M%SZ")

    query = QUERY_TEMPLATE.format(
        date=attic_iso, radius=f"{args.fetch_radius:g}", lat=args.lat, lon=args.lon
    )

    out_dir = repo_path(args.out_dir)
    out_file = out_dir / f"{args.town}-{attic_compact}.json"
    if out_file.exists():
        print(
            f"error: {out_file} already exists — a re-fetch is a new extract "
            "with a new date, never an overwrite (ADR 0010 Decision 4)",
            file=sys.stderr,
        )
        return 1

    # overpass-api.de returns 406 for the default python-requests
    # User-Agent; identify the tool per the endpoint's usage policy.
    resp = requests.post(
        args.endpoint,
        data={"data": query},
        timeout=240,
        headers={"User-Agent": "via-benchmark/0.1 (reference corpus pilot)"},
    )
    resp.raise_for_status()
    body = resp.content
    doc = json.loads(body)

    osm3s = doc.get("osm3s", {})
    generator = doc.get("generator", osm3s.get("generator", ""))
    osm_base = osm3s.get("timestamp_osm_base", "")
    n_elements = len(doc.get("elements", []))
    if not osm_base:
        print(
            "error: response carries no osm3s.timestamp_osm_base — refusing to "
            "record an extract without its data timestamp",
            file=sys.stderr,
        )
        return 1

    out_dir.mkdir(parents=True, exist_ok=True)
    out_file.write_bytes(body)
    digest = blake3_file(out_file)

    # Second location, verified by hash, BEFORE the manifest line exists:
    # a manifest record must never describe an extract that exists in only
    # one place.
    archive_dir = repo_path(args.archive_dir)
    archive_dir.mkdir(parents=True, exist_ok=True)
    archive_file = archive_dir / out_file.name
    if archive_file.exists():
        if blake3_file(archive_file) != digest:
            print(
                f"error: {archive_file} exists with different content — refusing "
                "to overwrite",
                file=sys.stderr,
            )
            return 1
    else:
        shutil.copy2(out_file, archive_file)
        if blake3_file(archive_file) != digest:
            print(
                f"error: archive copy {archive_file} does not hash-match the "
                "extract — archive not verified, manifest NOT written",
                file=sys.stderr,
            )
            return 1

    record = {
        "town": args.town,
        "class": args.town_class,
        "centre": [args.lat, args.lon],
        "fetch_radius_m": args.fetch_radius,
        "query": query,
        "endpoint": args.endpoint,
        "generator": generator,
        "osm_base": osm_base,
        "fetch_date": attic_iso,
        "file": str(out_file.relative_to(REPO_ROOT))
        if out_file.is_relative_to(REPO_ROOT)
        else str(out_file),
        "blake3": digest,
        "licence": "ODbL",
        "partition": args.partition,
    }
    manifest = repo_path(args.manifest)
    manifest.parent.mkdir(parents=True, exist_ok=True)
    with open(manifest, "a", encoding="utf-8") as fh:
        fh.write(json.dumps(record, sort_keys=True) + "\n")

    print(
        f"{args.town}: {n_elements} elements, osm_base {osm_base}, "
        f"blake3 {digest[:16]}…, saved {record['file']}, archived "
        f"{archive_file}, manifest {manifest.name} +1 line"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
