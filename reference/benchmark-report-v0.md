# Benchmark Report v0 — the reference corpus, measured

First corpus-scale report of the benchmark instrument (ADR 0008/0009/
0010; research 0012). **Threshold-free by design**: this document
reports positions and distances, never pass/fail — no generated model
is measured here, no composite score exists (ADR 0008 D7), and no
acceptance band is declared (bands, when they come, are declared
conventions under D8 at their point of use).

Contains information from OpenStreetMap, © OpenStreetMap
contributors, ODbL.

## Provenance (D11)

- **Protocol**: v1.1, frozen ([protocol-v1.1.md](protocol-v1.1.md));
  every number below carries it.
- **Code revision**: `eb16ae178af3` (via-bench, embedded at build) —
  one clean stamp across all 138 measurements.
- **Corpus**: the 138 towns of
  [pre-registration.md](pre-registration.md) — 132 fresh fetches plus
  the six pilots — all under one attic snapshot
  **2026-08-20T12:00:00Z**, so the corpus reads a single OSM state.
  Extract hashes, endpoints, and per-town fetch radii:
  [extracts.jsonl](extracts.jsonl).
- **Partition**: fitted n=94, held-out n=44 — the mechanical blake3
  split, registered before any fresh town was fetched.
- **Store**: measured outputs live in `runs/reference/corpus-v1.1`
  (plus the off-repo archive), never redistributed (ODbL; ADR 0010
  Decision 5).

## What this report publishes, and what it withholds

**Published**: per-class aggregate populations over fitted towns
([populations-v1.1.json](populations-v1.1.json)), one strip figure
per battery character ([report-v0/figures/](report-v0/figures/)),
fitted-town contact sheets ([report-v0/sheets/](report-v0/sheets/)),
null-model context tables
([report-v0/null-tables.md](report-v0/null-tables.md)), coverage
verification, and one named case study.

**Withheld**: every per-town corpus-scale character table (a cautious
ODbL reading treats a systematic hundred-town character database as a
derivative database — ADR 0010 Decision 5), and every held-out
aggregate (ADR 0008 D6: the held-out partition fits nothing and is
spent only at a validation event; its towns are measured and stored,
and appear here only as counts).

## Acquisition

132 fresh fetches (one verification fetch plus a 131-town batch),
**zero failures**. 131 of the 138 extracts sit at the default 750 m
fetch radius (126 fresh + 5 pilots); six dense towns stepped down the
declared ladder to 550 m (siena, stein-am-rhein, tetouan, richelieu,
broadmeadows-dallas, vidigal-rio) and the pilot kibera stands at
450 m — every radius ≥ the 435 m participation floor (v1.1 P2), every
radius recorded per extract. Each extract was hash-verified in two storage locations
before its manifest line was written (ADR 0010 Decision 4). All 138
towns measured, zero failures, both P4 street sets each.

## Coverage verification (0012 §9.1)

The registration-time coverage flag is confronted with the measured
30-footprint floor — over the **fitted partition** (94 towns
including pilots; held-out outcomes stay in the store per D6 and
appear only as a count). Medians are nearest-rank, the protocol's
pinned convention:

| coverage flag | towns | below floor | footprints median | min |
| --- | --- | --- | --- | --- |
| community-mapped | 7 | 0 | 1,726 | 158 |
| expected-good | 31 | 0 | 569 | 98 |
| pilot (measured) | 6 | 0 | 303 | 116 |
| unverified | 50 | 10 | 186 | 0 |

**All ten fitted floor-tripped towns were flagged `unverified` at
registration — zero contradictions**, and the flag ordering is
monotone in measured coverage. The fitted floor list: jamestown-sa,
lindsborg-ks (plat); heliopolis-sp, jalousie-pap, manshiyat-naser
(informal); elizabeth-sa, green-valley-nsw, levittown-pa,
medina-kwinana (suburb); safranbolu (organic). Their building
characters are suppressed to NaN by rule (D8 declared convention);
their street characters stand. Of the 44 held-out towns, measured and
recorded in the store, 1 is below the floor. Class impact on the
building-character populations (per-character `n_valid` in the
populations file): organic 19 of 20 fitted towns, plat 17 of 19,
suburb 14 of 18, informal 16 of 19 — except informal `gsi` at 15,
where dworzark-freetown's all-ways disc holds 1,640 footprints but
zero protocol-range block faces, leaving gsi undefined (the same
degeneracy family as the Fes el-Bali gsi artifact; v1.2 agenda).

## Cross-validation of the corpus

**Level 1 — formula exactness.** networkx re-measurement of
via-bench's exported graph, both street sets, all 138 towns: **276 of
276 runs exact (≤ 10⁻⁶), zero failures.** Every formula in the
battery computes what its source says it computes, corpus-wide.

**Level 2 — independent construction.** The first corpus-wide pass
flagged 143 of 276 runs. Every flagged family was investigated and
each diagnosis independently adversarially verified before anything
was concluded or changed. The investigation found **three defects in
the cross-validator, one defect in via-bench, one protocol
underspecification, and one protocol gap** — and confirmed the
remainder as the declared construction divergence:

1. **Cross-validator defects (fixed).** (a) It computed quantiles by
   linear interpolation where the protocol pins nearest-rank — pinned
   precisely because methods move p90 by ~9% — and this convention
   gap, not construction, accounted for *every* footprint-median
   envelope violation in the corpus (switching conventions reproduces
   via-bench to 10⁻⁶). (b) osmnx's `simplify_graph` only dissolves
   chains that start at an endpoint, so endpoint-free ring components
   escaped the declared P3 dissolution: 135 of chefchaouen's 161
   interior "simplified" nodes were raw vertices of nine closed
   `highway=pedestrian area=yes` plaza outlines, manufacturing a
   4.9× edge-count divergence. Junction-free cycles are now dropped,
   matching P3's "nodes are junctions and dead ends"; chefchaouen's
   node comparison fell from 161-vs-34 to 26-vs-34. (c) The
   fail-closed envelope checker counted not-computable diffs as
   violations even where the null *is* the protocol working (the
   30-footprint floor) or both sides agree a character is undefined
   (west-point-monrovia's edgeless 1-node carriageway interior, itself
   cross-validated); those are now not-evaluable and touch nothing.
2. **A via-bench defect (fixed, corpus re-measured).** The extract
   loader's else-branch let any way carrying a highway tag bypass the
   building test, so a closed way tagged both `building=yes` and
   `highway=elevator` — dinan way 682988676, the only such way in all
   138 extracts — was silently lost. The Elements text qualifies a
   building only by its building tag, closure, area, and centroid;
   the branches are now independent, with a regression test, and the
   corpus was re-measured under the fixed revision.
3. **A protocol underspecification (v1.2 agenda).** Two chefchaouen
   footprints are self-intersecting figure-eight rings, where the
   area and centroid of an invalid ring are undefined by the protocol
   text: three defensible readings put one way at 7.997 / 8.616 /
   9.235 m² astride the 8 m² floor, and near-cancelling lobes drive
   via-bench's centroid arithmetic to a meaningless point 7.5 km out.
   Neither implementation violates the stated rule; the exact-match
   buildings envelope simply cannot hold until the text picks a
   canonical treatment. These two ways are the only remaining
   buildings-envelope breaks in the corpus.
4. **A P4 gap (v1.2 agenda).** Closed `area=yes` pedestrian polygons
   — plaza outlines — are admitted as street fabric by the P4 filter
   on both sides (osmnx's own network loader excludes `area=yes` for
   exactly this reason). Both implementations now treat them
   consistently, but a plaza perimeter measured as street centreline
   is a taxonomy accident P4 should name.

**After the fixes: 183 of 276 runs sit within every pilot envelope**
(19 of them carrying not-evaluable entries by rule), Level 1 remains
exact everywhere, and the buildings envelope holds everywhere except
chefchaouen's two undefined rings. The 93 still-violating runs are
the declared P3 construction divergence, now statistically
characterized rather than assumed (statistics recomputed on the
post-fix corpus): graph-topology divergence correlates negatively
with the independent construction's median segment length (Spearman
−0.47 to −0.57 for node/edge counts and street_km), violation rates
fall monotonically from 71% of runs where median segments are under
20 m — fabric approaching the 6 m snap radius — through 51% and 31%
to 14% at ≥ 50 m, and the class ranking (organic worst at 57% of
runs, informal best at 5%) tracks segment length and fragmentation.
Two
hand-verified mechanisms bound the family: stein-am-rhein loses
~2.6 km of street_km to the snap merging parallel node-disjoint ways,
and sharpstown's divergence is one 272 m chain toggled across the
hard 300 m boundary by a 5.7 m snap displacement of one junction.
**The pilot-fitted envelope table therefore does not generalize to
dense-fabric constructions, and re-characterizing it on the corpus's
divergence distribution is v1.2 material** (agenda below). No formula
defect exists on either side of any of it.

## The five populations

Per-class aggregates over **fitted towns only**, operative street set
(carriageway; all_ways for informal), nearest-rank medians. Full
quantile tables for both street sets:
[populations-v1.1.json](populations-v1.1.json). One figure per
character in [report-v0/figures/](report-v0/figures/).

| class | n | meshedness | dead-ends | φ | circuity | seg med m | bc gini | blocks | gsi |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| organic | 20 | 0.155 | 0.212 | 0.112 | 1.071 | 31.8 | 0.560 | 35 | 0.573 |
| planted | 18 | 0.228 | 0.116 | 0.751 | 1.050 | 48.5 | 0.413 | 27 | 0.386 |
| plat | 19 | 0.236 | 0.049 | 0.959 | 1.017 | 66.1 | 0.358 | 24 | 0.196 |
| suburb | 18 | 0.133 | 0.100 | 0.593 | 1.084 | 74.6 | 0.432 | 9 | 0.299 |
| informal (AW) | 19 | 0.080 | 0.269 | 0.245 | 1.061 | 37.9 | 0.524 | 19 | 0.512 |

The separations are the literature's, measured at home: orientation
order φ runs 0.11 (organic) → 0.96 (plat) with suburb smeared across
the middle; circuity peaks in the suburb class (curvilinear design,
1.084) and bottoms out on the plats (1.017); betweenness
concentration is highest in organic fabric (spine streets, gini
0.560) and lowest on the grids (0.358); the organic class has the
shortest segments outright (31.8 m median), and the informal class
pairs short segments (37.9 m) with the highest dead-end share
(0.269). Ground-space index spans 0.20 (plat) to 0.57 (organic).

Notable inside the classes:

- **Suburb dead-ends are a spread, not a peak.** The cul-de-sac
  reputation shows as a wide distribution (p10 0.04 to p90 0.24, with
  outliers to 0.42 — see the figure) around a low median (0.100) at
  the 300 m disc scale — many mid-century tracts read
  through-connected at their core.
- **Suburb blocks are scarce**: median 9 faces per disc (organic: 35)
  — superblocks and loops leave few closable faces at this radius.
- **The informal class is not one shape**: the sheets show everything
  from Karachi's near-grid Machar Colony to Rocinha's dendritic
  hillside; φ spans 0.04–0.72 within the fitted partition.

## The medina finding

Four organic-class towns — fes-el-bali, moulay-idriss, tetouan,
chefchaouen — have carriageway graphs of 23–34 interior nodes in 4–10
fragments, while their all-ways graphs hold 257–304 nodes — an order
of magnitude more circulation, though themselves fragmented (2–12
interior components). Their circulation is alley fabric that OSM maps as
footway/path/pedestrian mixtures, so the **organic class's operative
set (carriageway) reads mapping taxonomy, not settlement, on exactly
these towns** — the Kibera precedent (v1.1 P4) reproduced inside a
class whose operative set the protocol froze as carriageway.

v1.1 stays in force: both sets are measured and published for every
town, the meshedness-with-`interior_components` rule already quotes
the fragmentation honestly, and the populations file carries the
all-ways quantiles beside the carriageway ones. The v1.2 agenda
(below) takes the question of whether the operative set should be a
per-town data property rather than a per-class constant.

### Named case study: Fes el-Bali (ODbL attribution above)

| character | carriageway | all_ways |
| --- | --- | --- |
| interior nodes / components | 32 / 4 | 304 / 12 |
| meshedness | 0.085 | 0.119 |
| dead-end share | 0.250 | 0.257 |
| orientation entropy / φ | 3.203 / 0.317 | 3.457 / 0.112 |
| circuity | 1.105 | 1.073 |
| segment median | 34.7 m | 21.5 m |
| bc gini | 0.659 | 0.575 |
| blocks | 9 | 66 |
| buildings in disc | 61 | 61 |
| gsi | **1.06** | 0.065 |

Two instrument lessons in one town: the carriageway gsi exceeds 1 —
61 footprints rated against 9 faces of a fragmented graph — so gsi is
meaningless where the block set is degenerate (v1.2 agenda: a block
floor for gsi, mirroring the footprint floor); and 61 mapped
buildings for the densest medina in the world is a coverage floor the
flag system correctly anticipated (`unverified`).

## Null-model context (ADR 0008 D9)

Three nulls per class, matched to the fitted population's operative
medians (grid spacing s = r·√(π/n); scatter and particle counts with
one recorded rescale; derivations in `runs/reference/nulls-v1.1/
<class>/matching.json`), 8 seeds each (D5) — global seed 42, seed i
derived as `via_artifact::seed::derive(42, "bench-null-<model>", i)`
for i = 0..7, recorded per ensemble (D11) — protocol v1.1 stamped:

| class | target nodes | grid | random | dla |
| --- | --- | --- | --- | --- |
| organic | 101 | 52.9 m → 101 | 306 @ enr 1.29 → 106 | 235 → 61 |
| planted | 71 | 63.1 m → 69 | 185 @ enr 1.38 → 66 | 145 → 53 |
| plat | 40 | 84.1 m → 37 | 103 @ enr 1.40 → 41 | 76 → 36 |
| suburb | 25 | 106.3 m → 21 | 75 @ enr 1.20 → 25 | 46 → 24 |
| informal | 78 | 60.2 m → 69 | 274 @ enr 1.14 → 82 | 162 → 56 |

DLA saturates against dense targets (attachment-radius merging;
achieved counts recorded) — a known limit of the dendritic null, not
of the matching.

The point of the exercise
([report-v0/null-tables.md](report-v0/null-tables.md)): **no null
reproduces any class across the battery.** The matched grid's φ of
1.000 sits just above the plat p90 (0.998, within the population's
full range) while falling below the class's dead-end band (0 vs p10
0.023) and its betweenness concentration (gini 0.18 vs p10 0.28);
matched
random planar graphs reach organic meshedness but not its orientation
structure (φ 0.08 vs a population whose p90 is 0.32) and overshoot
bc_gini badly (0.80 vs 0.56); DLA reproduces dead-end-heavy shares
but nothing else. A future generator matching a class on one character
has, by these tables, matched nothing a null could not.

## Held-out policy

The 44 held-out towns are fetched, measured, and stored with full
provenance. Nothing here aggregates them; no figure draws them; the
contact sheets exclude them. They fit nothing (D6) and are spent only
at a declared validation event against a candidate generator.

## Limitations and the v1.2 agenda

Recorded here as instrument grounds for the next amendment; none of
these changes any v1.1 number:

1. **Level 2 envelopes were pilot-fitted and do not generalize** —
   the corpus-wide divergence distribution (above) is the evidence
   base for re-characterized envelopes, conditioned on construction
   sensitivity (node spacing vs snap radius, fragmentation).
2. **Operative street set as a per-town data property** — the medina
   finding: four organic towns whose carriageway set reads the
   mapping, not the town.
3. **A canonical treatment of self-intersecting building rings** —
   the Elements text leaves the area and centroid of an invalid ring
   undefined (the two chefchaouen figure-eights); pick one reading
   (reject invalid rings outright, or define both via
   orientation-consistent repair) so the buildings envelope can be
   exact again.
4. **A P4 `area=yes` exclusion** — closed pedestrian-area polygons
   (plazas) are not street centrelines; osmnx's own loader already
   draws this line.
5. **A block floor for gsi** (and any per-block-area statistic),
   mirroring the footprint floor — the Fes el-Bali gsi > 1 artifact,
   and dworzark-freetown's undefined gsi (1,640 footprints over zero
   protocol-range faces).
6. **Block characters now have an informational Level 2 comparison**
   (independent polygonization + momepy elongation, exact corner-
   convention port; machine-precision agreement on the clean-grid
   case) — promotable to enveloped status once corpus-characterized.
7. **Tessellation-based characters** (momepy) remain outside the
   battery — the sidecar can now carry them; adding any is a v1.2
   battery amendment with its own sources.
8. **Boeing indicators stay context-only** (§9.4 resolution); the
   recompute-at-protocol-scale upgrade path remains open and untaken.
9. **storeys_mean_tagged is thin everywhere** — tagged share is low
   across the corpus; treat storey statistics as indicative only.

## Figure index

One strip figure per character (fitted towns as dots, nearest-rank
median ticked, informal row on all_ways tagged "AW"):
[report-v0/figures/](report-v0/figures/). Contact sheets per class:
[report-v0/sheets/](report-v0/sheets/).
