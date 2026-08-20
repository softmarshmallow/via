# Measurement protocol v1

Identifier: **v1** — the protocol identifier every number measured
under this document carries (ADR 0008 D4/D11; ADR 0010 Decision 3).
Frozen 2026-08-20, after the six-town pilot. **This file is immutable:
any amendment is a new identifier.** The pilot ran under the working
identifier `pilot-0`, whose parameter values are identical to v1;
pilot measurements are v1 measurements by identity.

Every change this document makes relative to the spike's protocol is
justified on instrument grounds — a measurement defect, a
cross-validation finding, or a data-coverage limit — never by
reference to any generated model's performance (ADR 0010 Decision 3;
the protocol-level analogue of ADR 0008 D6).

## P1 — Study area

Statistics describe the disc of radius **300 m** about the declared
study centre. The radius is bound here, per 0012 P1's rule that it is
a protocol parameter quoted with every number. Rationale: the pilot
validated it at town-core scale across all six classes, and the §9.4
resolution below makes the project's own corpus the primary
reference, so the radius no longer needs to match any external
dataset's support.

## P2 — Participation buffer

Fabric is imported and participates in the graph out to **1.45 × r =
435 m**; statistics are tallied inside r. A street leaving the disc
is not a dead end; routes through the edge exist. Acquisition: the
Overpass fetch radius must be ≥ 435 m — 750 m is the default, 450 m
is acceptable where element density forces it (the Kibera precedent;
the fetch radius is recorded per extract in the manifest).

## P3 — Graph construction and simplification

The graph is built by **via-bench's snap-and-split planar insertion**:
node snap radius **6.0 m**, segments split at proper crossings, and
T-touches within the snap radius resolved (a segment ending on an
edge's interior splits it; a segment passing over an existing node
routes through it). This is the protocol's declared construction, on
both sides of every comparison — junction counts are comparable by
construction, not inherited from OSM's node placement. It is a
deliberate divergence from shared-node topology (which cannot see an
unshared crossing and preserves mapping accidents as topology).

All topological characters are computed on the graph with degree-2
vertices dissolved (nodes are junctions and dead ends; edges are
whole streets), the graph every published figure refers to.

Known caveat, measured in the pilot: **meshedness assumes a connected
graph** and goes negative on a fragmented interior (Kibera's
carriageway disc: 16 nodes in 3 fragments, meshedness −0.037). The
battery therefore reports `interior_components` beside it, and a
meshedness whose interior has more than one component is quoted only
with that count.

## P4 — Street sets

Two sets, both always measured and reported (research 0012 P4,
verbatim):

- **Carriageway**: highway ∈ {motorway, trunk, primary, secondary,
  tertiary, unclassified, residential, living_street, service,
  pedestrian} plus `_link` variants, excluding
  `service=driveway|parking_aisle`.
- **All-ways**: the above plus {footway, path, steps, cycleway}.

Resolved spike drifts, on instrument grounds: the spike's extra
`service=drive-through` exclusion is dropped (not in P4);
`service=alley` is deliberately in the carriageway set (alleys are
fabric; the spike excluded them in prose while including them in
code).

**Per-class operative set.** The carriageway set is the operative
(headline) set for the organic, planted, plat and suburb classes. For
the **contemporary informal class the all-ways set is operative**:
pilot evidence is Kibera, where the 300 m disc holds 1,199 buildings
against 16 carriageway nodes — informal circulation is footpath
fabric, and measuring it by carriageway alone measures the mapping
taxonomy, not the settlement. Both sets are still reported for every
town in every class.

## P5 — Projection

Local equirectangular about the study centre: x = (lon − lon₀) ·
111,320 · cos(lat₀), y = (lat − lat₀) · 110,540, metres. Distortion
over the disc is far below input resolution.

## P6 — Provenance

Every reported number carries: this protocol identifier, the
via-bench code revision (embedded at build), the extract's blake3
hash (which keys its manifest record: fetch date, attic timestamp,
endpoint, generator), and — for seeded models — the seed set.
Software versions on the sidecar side are pinned by `analysis/uv.lock`.

## Elements

- Block: a bounded face of the street graph with area in
  **[300, 90,000] m²**, centroid inside the participation buffer.
  One range, both sides of every comparison (the spike's 90k/200k
  reference/synthetic asymmetry is removed).
- Building: a closed way tagged `building`, area ≥ **8 m²**, centroid
  inside the buffer; counted in statistics by centroid inside the
  disc. Storey statistics are computed over buildings carrying
  `building:levels`, with the tagged share reported beside them.
- Building characters are computed only where the disc holds at least
  **30 footprints** — a declared convention (ADR 0008 D8): below
  that, medians are noise (the spike's abilene-res disc returned 7
  footprints and was unusable). Coverage above the floor is still not
  proof of completeness; the pre-registration document must flag
  towns whose OSM building coverage is unverified (0012 §9.1).
- Quantiles are **nearest-rank**: sort ascending, take index
  `round((n−1)·q)`. Pinned because the pilot showed interpolation
  methods move p90 by up to ~9% at n < 100 (the Levittown footprint
  p90 divergence).

## Characters

Adopted by name (ADR 0008 D2), computed by via-bench:

| Character | Source |
| --- | --- |
| meshedness (e−v+1)/(2v−5), with `interior_components` | Cardillo et al. 2006 |
| k_avg, edge_node_ratio, dead_end/deg3/deg4 shares, self_loop_proportion | Boeing 2021 indicator set |
| circuity_avg (self-loops excluded) | Boeing 2021 |
| orientation_entropy — one bearing per chain endpoint pair plus reciprocal, 36 bins of 10° with bin 1 spanning [−5°, 5°), unweighted, natural log | Boeing 2019 |
| orientation_order φ = 1 − ((H − ln 4)/(ln 36 − ln 4))² | Boeing 2019 |
| bc_gini, bc_max — node betweenness, length-weighted, networkx normalization, whole buffered graph, interior tally | Boeing 2021 |
| segment length median/p90; block area median/p90; footprint area median/p90 | distributional (0012 §6) |
| block compactness 4πA/P², corner count (>25° turns) | shape standards; corners added under D10 |
| block elongation (minor/major of minimum bounding rectangle) | momepy `Elongation`, Fleischmann et al. 2022 |
| street_wall_share (≤6 m), street_fronting_share (≤25 m) | **labelled inventions** (D2): no published character covers building-line-on-street; thresholds declared, not sourced |
| gsi (footprint/block area), storeys_mean_tagged + tagged share | standard density measures |

Exclusion, named on instrument grounds: **momepy's tessellation-based
characters are not yet in the battery** — the sidecar tooling for
morphological tessellation is not built; this is an implementation
gap, not a model-motivated choice, and the corpus phase should close
it.

## The §9.4 resolution (scale mismatch)

**The primary Tier B reference is the project's own pre-registered
corpus, measured under this protocol at this scale.** Boeing's global
indicator population is computed over whole urban areas; a 300 m disc
is not that object, and by the project's own doctrine (README, "The
causal chain") centrality on a clipped graph is a different quantity
— so disc-scale bc_gini/bc_max are protocol-bound quantities, **never
compared against Boeing's whole-network values**.

The Boeing population may appear in reports as qualitative context
only, clearly labelled, carrying no statistical comparison and no
evidential weight, until a per-character scale-robustness study
exists. The upgrade path 0012 names — recomputing indicators from
Boeing's published street-network models at this protocol's scale —
remains open and is the corpus phase's option to take.

## Cross-validation tolerances

Two tiers of check against `osmnx`/`networkx` (ADR 0009 Decision 4),
with envelopes from the six-town pilot; disagreement beyond an
envelope is a defect, not a tolerance to relax:

**Tier A — formula level, identical exported graph: exact (≤ 10⁻⁶).**
Verified in the pilot for betweenness (bc_gini and bc_max to six
decimals on Alnwick, Monpazier, Levittown). Any character computable
on the exported graph must match at this tier.

**Tier B — independent construction from the raw extract** (osmnx
shared-node topology vs P3's snap-and-split): systematic construction
differences are expected and characterized; the envelopes say when
divergence signals a bug instead:

| Character family | Envelope | Pilot basis |
| --- | --- | --- |
| building count | exact | 6/6 towns exact |
| footprint area median | ≤ 1% | ≤ 0.8% observed |
| footprint area p90 | ≤ 10% | quantile-method difference, ≤ 9.3% observed |
| orientation_entropy | ≤ 4% | ≤ 3.2% observed |
| circuity_avg | ≤ 2% | ≤ 1.8% observed |
| street_km | ≤ 5% | ≤ 4.3% observed |
| segment length median/p90 | ≤ 17% | quantile + construction, ≤ 16.9% observed |
| node/edge counts | ≤ 15% | snap merges, ≤ 12% observed |
| degree shares | ≤ 0.13 absolute | Monpazier offset-crossroads merges, ≤ 0.125 observed |
| meshedness, bc_gini, bc_max | no Tier B envelope — checked at Tier A only | hypersensitive to construction at pilot graph sizes |

On a clean shared-node town (Abilene) everything, including bc_gini,
agreed within 1.5% at Tier B — the construction difference, not the
formulas, is the divergence source.

## Fetch acquisition

Overpass, attic-dated (`[date:"…"]`) so extracts are
content-reproducible; endpoint, generator, `osm_base`, fetch date,
radius, blake3 and licence recorded per extract in
`reference/extracts.jsonl`; extract in two locations before any
number measured on it is published (ADR 0010 Decision 4).

## Pilot record (measured under this protocol, extracts of 2026-08-20)

Contains information from OpenStreetMap, © OpenStreetMap
contributors, ODbL. Carriageway set except Kibera, whose operative
set is all-ways (P4); single extracts, not ensembles — reference
towns are data, not seeded models.

| Town | class | nodes | meshedness | dead ends | blocks | buildings | H_o | bc_gini |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Alnwick | organic | 77 | 0.060 | 33.8% | 13 | 303 | 3.513 | 0.630 |
| Lavenham | organic | 57 | 0.073 | 28.1% | 11 | 257 | 3.366 | 0.560 |
| Monpazier | planted | 55 | 0.181 | 12.7% | 26 | 393 | 2.382 | 0.413 |
| Abilene | plat | 42 | 0.215 | 0.0% | 28 | 116 | 1.641 | 0.343 |
| Levittown | suburb | 46 | 0.092 | 23.9% | 12 | 144 | 2.314 | 0.554 |
| Kibera (all-ways) | informal | 51 | 0.082 | 21.6% | 19 | 1,199 | 3.262 | 0.514 |

These are new measurements on new extracts. They supersede the
historical record in spikes/townfabric/VALIDATION.md and are not
reproductions of it (ADR 0010 Decision 2). All six towns are
calibration-partition pilot cases and can never be held out.
