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

**The partition is derived, never authored** (ADR 0010 Decision 4). The
six pilot towns — alnwick, lavenham, monpazier, abilene, levittown,
kibera — are always `fitted` (ADR 0010 Decision 2 + addendum). Any
other town is looked up, case-insensitively, in the
`Town | Class | Centre | Partition` table of
`reference/pre-registration.md`; a town in neither is an error before
any network traffic — pre-register it first. `--partition` survives
only as an optional cross-check: when given it must match the derived
value, and a mismatch is an error (so a typo cannot author a
partition).

## Cross-validating via-bench (Tier B)

Re-measures the identical extract independently (osmnx / networkx /
shapely; research 0012 §3 P1–P6) and compares against via-bench's
character table. By default it reports positions and distances only —
no pass/fail: per-character tolerances are declared in the protocol
freeze document (ADR 0010 Decision 3).

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

**Enforcing the Tier B envelopes**: `--envelopes <file>` applies the
per-character envelopes of `reference/protocol-v1.md` ("Cross-validation
tolerances"). The file is a JSON object mapping character name to
`{"kind": "rel"|"abs", "max": number}` — `rel` bounds the relative
difference, `abs` the absolute one (the degree-share envelopes are
absolute). Every mapped character is checked (an enveloped character
that is missing or not comparable is a violation); characters not in
the map stay informational. Violations are printed and the exit code is
2 if any envelope is exceeded; the envelope results are also recorded
in the `--out` JSON.

## Tier A: formula-level check on the identical exported graph

The protocol's Tier A ("formula level, identical exported graph: exact,
≤ 10⁻⁶") is performed by `tier_a_check.py`. It rebuilds a networkx
graph from via-bench's own `<town>-graph.json` export — no independent
construction, the identical nodes and chains — recomputes everything
the exported graph determines (node/edge counts, k_avg,
edge_node_ratio, meshedness, dead-end/deg3/deg4 shares, and
length-weighted betweenness → bc_gini, bc_max), and compares against
the `<town>-fabric.json` character block:

```bash
uv run --project analysis python analysis/tier_a_check.py \
    --graph-json runs/reference/measured-v1/alnwick-graph.json \
    --bench-json runs/reference/measured-v1/alnwick-fabric.json \
    --street-set both --tolerance 1e-6
```

It prints a per-character table (ours / bench / abs_diff / status) for
each requested street set and exits nonzero if any character differs by
more than `--tolerance` (default 1e-6, the Tier A bound). Because both
sides read the same graph, any disagreement here is a formula defect,
never a construction difference.

Relative paths given to either script are resolved against the repo
root, not the current working directory.
