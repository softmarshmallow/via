# analysis — the Python sidecar

The standing, version-pinned analysis sidecar defined by ADR 0009
Decision 4 and adopted into CONTRIBUTING by ADR 0010: the half of the
benchmark that lives where `osmnx`, `momepy` and the reference-data
readers are. It fetches reference extracts with full provenance, and it
is the independent check on the Rust measurement kernel (`via-bench`) —
the frozen protocol reimplemented from the raw extract with the
published toolchain, so definition drift between our implementation and
the published ones is caught rather than shipped.

**The quarantine is absolute** (ADR 0009 Decision 4):

> The sidecar is a standing, version-pinned tool — CONTRIBUTING's
> Python rule, as amended, permits exactly this — and the quarantine is
> absolute: it may never be imported by a crate, and no gate may depend
> on it.

The engine cannot tell whether this directory exists.

## Setup

The project is managed by [uv](https://docs.astral.sh/uv/); Python 3.12
and every dependency are pinned in `uv.lock` — this pin is the software
half of protocol provenance (research 0012 §3 P6: every result carries
code revision and software versions). From the repo root:

```bash
uv sync --project analysis
```

## Fetching a reference extract

One town, full ADR 0010 Decision 4 provenance: attic-dated query (so a
lost extract is content-reproducible), blake3 of the file bytes, a JSON
line appended to `reference/extracts.jsonl`, and a hash-verified copy
in a second storage location before success is reported.

```bash
uv run --project analysis python analysis/fetch_extract.py \
    --town alnwick --town-class "organic pre-modern core" \
    --lat 55.4147 --lon -1.7061 \
    --archive-dir /Volumes/archive/via-reference
```

Extract files land in the gitignored `runs/reference/osm/` and are never
redistributed from this repository (© OpenStreetMap contributors, ODbL).
An existing extract file is never overwritten: a re-fetch is a new
extract with its own date and manifest record.

## Cross-validating via-bench

Re-measures the identical extract independently (osmnx / networkx /
shapely; research 0012 §3 P1–P6) and compares against via-bench's
character table. It reports positions and distances only — no pass/fail:
per-character tolerances are declared in the protocol freeze document
(ADR 0010 Decision 3).

```bash
uv run --project analysis python analysis/cross_validate.py \
    --extract runs/reference/osm/alnwick-20260820T100000Z.json \
    --centre 55.4147,-1.7061 \
    --bench-json runs/reference/alnwick-fabric.json \
    --street-set carriageway \
    --out runs/reference/alnwick-crossval.json
```

Both street sets of protocol P4 (`carriageway`, `all_ways`) are
measured by separate invocations, against the matching block of the
via-bench output.

Relative paths given to either script are resolved against the repo
root, not the current working directory.
