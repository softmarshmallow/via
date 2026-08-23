# Measurement protocol v1.1

Identifier: **v1.1** — the protocol identifier every number measured
under this document carries (ADR 0008 D4/D11; ADR 0010 Decision 3).
Frozen 2026-08-20. Supersedes **v1**, which stands as record. **This
file is immutable: any amendment is a new identifier.**

## Why v1.1 exists — the amendment record

The adversarial review of the Phase B build re-ran v1's own checks
and found them not clean. Every change below is on instrument grounds
(ADR 0010 Decision 3); none references any generated model's
performance. Measurements: code revision 931cc18bb261, clean tree.

1. **Betweenness tie handling was a formula defect.** v1's kernel
   inherited three spike conventions that diverge from the published
   implementation the formula check compares against: a
   millimetre-quantized Dijkstra heap key (which could settle a node
   twice and double-count betweenness — the review measured an
   impossible bc_max 1.33 on sub-millimetre parallel chains), a 1e-9
   tie epsilon (which counts float-residue near-ties networkx does
   not — the signature: Abilene's bc_gini off 1.3e-5 with bc_max
   exact), and multigraph adjacency where the check uses a simple
   graph. All three now match the published conventions, and the
   formula-level check passes ≤ 1e-6 on all six pilot towns, both
   street sets, via a committed tool (`analysis/tier_a_check.py`).
2. **The self-loop degree convention was wrong.** A self-loop chain
   contributed +1 to its junction's simplified degree; the
   networkx/osmnx multigraph convention — and this graph's own raw
   edges — count both ends (+2). Found by the checking tool on
   Alnwick's deg3_share. Fixed.
3. **v1's construction-level envelopes were derived from
   carriageway-only cross-validation** — including for Kibera, whose
   operative set is all-ways. The envelopes below are re-derived from
   all six towns × both street sets, and two of v1's pilot-basis
   attributions were wrong (street_km's "4.3% observed" appears in no
   saved run; the 0.125 degree-share observation was Kibera's, not
   Monpazier's). The new table replaces both.
4. **v1's pilot measurements carried a dirty-tree revision stamp**
   shared by two different code states — the provenance failure D11
   exists to prevent. The pilot is re-measured from a clean build,
   and the T-touch construction fix (a review-found defect where the
   pre-pass could still leave a disconnected ghost) is in it.
5. **Enforcement now exists where v1 only declared**: the
   30-footprint floor is in code (`below_footprint_floor` flag; NaN
   building characters below it), the envelopes are machine-checkable
   (`cross_validate.py --envelopes reference/envelopes-v1.1.json`,
   exit 2 on drift), ensembles report per-field valid-run counts, and
   the partition field of the extract manifest is derived, never
   authored.
6. **The exclusion list is completed** (ADR 0010 Decision 3 requires
   every exclusion from the full published sets to be named): see
   Characters below.
7. **Terminology**: the two cross-validation checks are now **Level 1
   (formula)** and **Level 2 (construction)** — v1 called them Tier
   A/B, colliding with 0012's Tier A/B (settlement systems / urban
   fabric).

## P1 — Study area

Statistics describe the disc of radius **300 m** about the declared
study centre (0012 P1: the radius is a protocol parameter, bound per
identifier). Rationale as v1: pilot-validated at town-core scale
across all six classes, and the §9.4 resolution below makes the
project's own corpus the primary reference, so the radius need not
match any external dataset's support.

## P2 — Participation buffer

Fabric is imported and participates in the graph out to **1.45 × r =
435 m**; statistics are tallied inside r. Acquisition: fetch radius
≥ 435 m — default 750 m, 450 m acceptable where density forces it
(the Kibera precedent; recorded per extract in the manifest).

## P3 — Graph construction and simplification

**via-bench's snap-and-split planar insertion**: node snap radius
**6.0 m**; segments split at proper crossings; T-touches within the
snap radius resolved (a segment ending on an edge's interior splits
it — exactly at the projection when no reachable node exists — and a
segment passing over an existing node routes through it). This is
the protocol's declared construction on both sides of every
comparison; it deliberately diverges from shared-node topology, and
the divergence is characterized by the Level 2 envelopes below.

Topological characters are computed with degree-2 vertices dissolved.
A self-loop chain counts twice toward its junction's degree (the
multigraph convention). Meshedness assumes a connected interior:
`interior_components` — components of the interior subgraph, where an
interior node isolated by the boundary clip counts as its own
component, the same subgraph the meshedness formula counts — is
reported beside it, and a meshedness whose interior has more than one
component is quoted only with that count (Kibera's carriageway disc:
16 nodes, 3 components, meshedness −0.037).

## P4 — Street sets

Two sets, both always measured and reported (0012 P4, verbatim):

- **Carriageway**: highway ∈ {motorway, trunk, primary, secondary,
  tertiary, unclassified, residential, living_street, service,
  pedestrian} plus `_link` variants, excluding
  `service=driveway|parking_aisle`.
- **All-ways**: the above plus {footway, path, steps, cycleway}.

Resolved spike drifts (v1, unchanged): drive-through exclusion
dropped; `service=alley` deliberately in the carriageway set.

**Per-class operative set** (v1, unchanged): carriageway for organic,
planted, plat and suburb; **all-ways for contemporary informal** —
Kibera's disc holds 1,199 buildings against 16 carriageway nodes;
informal circulation is footpath fabric.

## P5 — Projection

Local equirectangular about the study centre: x = (lon − lon₀) ·
111,320 · cos(lat₀), y = (lat − lat₀) · 110,540, metres.

## P6 — Provenance

Every reported number carries: this identifier, the via-bench code
revision (embedded at build; the build script watches the git refs
that actually move on commit), the extract's blake3 (keying its
manifest record), and — for seeded models — the seed set. Sidecar
versions pinned by `analysis/uv.lock`.

## Elements

- Block: bounded face of the street graph, area **[300, 90,000] m²**,
  centroid inside the buffer. One range, both sides.
- Building: a **closed** way tagged `building` (and not
  `building=no`), area ≥ **8 m²**, centroid inside the buffer;
  counted by centroid inside the disc. Storey statistics over tagged
  buildings only, tagged share reported.
- Building characters require ≥ **30 footprints** in the disc
  (declared convention, ADR 0008 D8) — enforced in code: below the
  floor they are NaN with `below_footprint_floor` set, and only the
  count is reported. Coverage above the floor is still not proof of
  completeness (0012 §9.1); the pre-registration document must flag
  unverified towns.
- Quantiles are **nearest-rank**: sort ascending, index
  `round((n−1)·q)`. Interpolation methods move p90 by up to ~9% at
  n < 100.

## Characters

Adopted by name (ADR 0008 D2), computed by via-bench — the same
battery as v1 (meshedness with `interior_components`; k_avg,
edge_node_ratio, degree shares, self_loop_proportion, circuity_avg,
orientation entropy and order, bc_gini, bc_max; segment/block/
footprint distributional characters; block compactness, corners,
elongation; the labelled inventions street_wall_share ≤ 6 m and
street_fronting_share ≤ 25 m; gsi; storeys over tagged), with every
exclusion from the full published sets now named:

- **Boeing indicators not carried, with grounds**: `grade_mean` /
  `grade_median` and `built_up_area_percap` — data-coverage limits
  (extracts carry no elevation; per-capita needs GHS population);
  `cc_avg_undir` and `length_mean` — unimplemented (median/p90 are
  carried for lengths); `intersect_count_clean` — Boeing's 10 m
  consolidation is a different graph construction, and this
  protocol's snap-and-split node count is the declared counterpart.
- **momepy**: the whole taxonomy is absent, not only its
  tessellation-based half — an implementation gap (the sidecar has
  no tessellation tooling), named so the corpus phase closes or
  re-justifies it. Block elongation is the one momepy character the
  battery carries.
- Consequence: block and building characters currently have **no
  cross-validation at either level**; their tolerance row below says
  so rather than pretending.

## The §9.4 resolution (scale mismatch)

As v1, with one wording fix. **The primary urban-fabric reference is
the project's own pre-registered corpus, measured under this protocol
at this scale.** Disc-scale bc_gini/bc_max are protocol-bound and are
not compared against Boeing's whole-network values, and the Boeing
population appears in reports as qualitative context only — carrying
no statistical comparison and no evidential weight — **until a
per-character scale-robustness study exists**. The upgrade path
(recomputing indicators from Boeing's published models at this
scale) remains open.

## Cross-validation

Two levels, both with committed tooling; disagreement beyond a bound
is a defect, not a tolerance to relax.

**Level 1 — formula, identical exported graph: exact (≤ 10⁻⁶).**
Tool: `analysis/tier_a_check.py` on the `<town>-graph.json` exports.
Pilot record: **all six towns, both street sets, pass** (bc_gini,
bc_max, meshedness, k_avg, edge_node_ratio, degree shares).

**Level 2 — independent construction from the raw extract** (osmnx
shared-node topology vs P3's snap-and-split). Systematic construction
differences are expected; the envelopes — from all six towns × both
street sets — say when divergence signals a bug. Machine-checkable
via `cross_validate.py --envelopes reference/envelopes-v1.1.json`.

| Character | Envelope | Pilot basis (worst observed) |
| --- | --- | --- |
| buildings | exact | 12/12 runs exact |
| footprint_area_median_m2 | ≤ 1% rel | 0.8% |
| footprint_area_p90_m2 | ≤ 10% rel | 9.3% (quantile methods) |
| orientation_entropy | ≤ 8% rel | 7.2% (Kibera carriageway, 14 chains) |
| circuity_avg | ≤ 5% rel | 4.2% |
| street_km | ≤ 7% rel | 5.9% (Kibera all-ways) |
| segment_len_median_m | ≤ 17% rel | 16.9% |
| segment_len_p90_m | ≤ 10% rel | 6.3% |
| nodes, edges | ≤ 15% rel | 11.7% (snap merges) |
| k_avg, edge_node_ratio | ≤ 10% rel | 7.1% |
| dead_end/deg3/deg4 shares | ≤ 0.13 abs | 0.125 (Kibera carriageway) |
| self_loop_proportion | ≤ 0.02 abs | 0.006 |
| meshedness, bc_gini, bc_max, orientation_order | Level 1 only | construction-hypersensitive at pilot sizes |
| block & building shape characters | none yet | no independent implementation in the sidecar — named gap |

## Fetch acquisition

As v1: attic-dated queries; endpoint, generator, `osm_base`, fetch
date, radius, blake3, licence per extract in
`reference/extracts.jsonl`; two locations before publication; the
partition field derived (pilot towns from ADR 0010 Decision 2, other
towns from the pre-registration document, anything else refused).

## Pilot record (measured under v1.1, extracts of 2026-08-20)

Contains information from OpenStreetMap, © OpenStreetMap
contributors, ODbL. Operative street set per class (P4); `comp` =
interior_components. Single extracts, not ensembles — reference towns
are data, not seeded models. New measurements on new extracts; they
supersede the historical record (ADR 0010 Decision 2) and v1's
tables. All six towns are calibration-partition, never held out.

| Town | class | set | nodes | meshedness | comp | dead ends | blocks | buildings | H_o | bc_gini |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Alnwick | organic | carriageway | 77 | 0.060 | 11 | 33.8% | 13 | 303 | 3.503 | 0.636 |
| Lavenham | organic | carriageway | 57 | 0.073 | 1 | 28.1% | 11 | 257 | 3.366 | 0.560 |
| Monpazier | planted | carriageway | 55 | 0.181 | 1 | 12.7% | 26 | 393 | 2.382 | 0.413 |
| Abilene | plat | carriageway | 42 | 0.215 | 1 | 0.0% | 28 | 116 | 1.641 | 0.343 |
| Levittown | suburb | carriageway | 46 | 0.092 | 4 | 23.9% | 12 | 144 | 2.314 | 0.554 |
| Kibera | informal | all-ways | 51 | 0.082 | 3 | 21.6% | 19 | 1,199 | 3.265 | 0.514 |

Alnwick's 11 components are boundary-isolated interior nodes, not
fragmented fabric — the count reads against the same clipped subgraph
meshedness reads against, which is exactly why it travels with it.
