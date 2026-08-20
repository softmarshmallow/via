# ADR 0009 — Where settlement and road-network solving lives

Status: accepted (2026-08-20)
Scope: answers a structural question the spikes exposed — the human
side had no home, so it grew as two monolithic throwaway binaries that
violated the project's own declared chain. Nothing here is implemented
yet; this ADR fixes where each piece lives when it is.

## Context

The README declares the causal chain in detail:

```
suitability & affordances (harbours, fords, confluences, passes)
  └─ corridors (least-cost, multi-modal: at-grade | cut | fill | viaduct | tunnel)
       └─ settlement seeds at network nodes ── population (rank-size)
            └─ demand (gravity) ── flows (assignment) ── network class
                 └─ growth epochs (promotion, bypass, demotion, fringe belts)
                      └─ morphology (blocks, plots) ── placement (buildings)
```

ADR 0001 anticipated this: "Stages get one crate each as they arrive
(`via-suitability`, `via-corridors`, …)". Only `via-suitability`
exists, and only as a generic affordance-threshold slice.

Two spikes were written outside that structure, and both contradicted
the chain above:

1. **Settlements were placed before corridors existed**, then roads
   were drawn between them. The chain says the opposite: corridors
   first, *settlement seeds at network nodes*. Towns form on routes;
   the spike made routes serve towns.
2. **The named affordances were never implemented.** The chain names
   harbours, fords, confluences and passes. The spike scored sites on
   slope and distance-to-water only — which is why its towns sit on
   generic good farmland rather than at crossings and confluences,
   where towns actually are.

Neither failure is a bug in the spike's code; both are consequences of
having no stage structure to write the code into.

## Decision 1 — One crate per stage, following the chain

| Crate | Chain level | Unit of work |
| --- | --- | --- |
| `via-suitability` *(exists, to extend)* | suitability & affordances | cell |
| `via-corridors` *(new)* | multimodal least-cost network | route, cost surface |
| `via-settlement` *(new)* | seeds at nodes, population, demand, flows, network class, growth epochs | settlement, corridor link |
| `via-morphology` *(new)* | streets within settlements, blocks, plots, buildings | street, block, plot, building |

`via-settlement` keeps population, demand, flows and epochs together
because they form one tight fixed-point loop; putting an artifact
boundary inside that loop would buy nothing and cost a serialisation
round-trip per iteration. Everything else gets its own boundary,
because everything else is a genuine one-directional hand-off.

The split between `via-settlement` and `via-morphology` falls exactly
where three independent things break at once: the unit changes
(settlement → plot), the literature changes (spatial economics → urban
morphology), and the benchmark reference changes (GHS-UCDB → OSM
morphometrics). A boundary that three separate considerations agree on
is the right boundary.

One further crate sits outside this table because it is not a chain
level: `via-bench`, the benchmark's measurement half, defined in
Decision 4.

## Decision 2 — The two-scale coupling contract

docs/research/humanity/0011 records "no literature template exists for
how settlement-layer mass maps onto intra-city demand fields" as via's
own design problem. It is an artifact schema, and it is specified here
so that the morphology stage can be built and benchmarked against one
settlement without the settlement stage existing yet.

`via-settlement` emits, per settlement:

- **site**: position, and the terrain cell it occupies;
- **population**: resident count, and the epoch it belongs to;
- **attachments**: for each corridor meeting this settlement, its
  bearing, class, and the cost of the route it carries;
- **role**: through-traffic volume and market-catchment mass, as
  scalars — the quantities the fabric scale needs in order to know
  whether this is a market town or a hamlet;
- **regime**: the declared era/culture parameter bundle in force.

`via-morphology` consumes exactly that record plus the terrain patch,
and nothing else. This is what makes the fabric stage independently
testable: a hand-written settlement record is a valid input.

## Decision 3 — Vector artifacts

ADR 0001 defined `.vrast` for rasters and rejected GeoTIFF. Morphology
emits geometry, not fields: a street graph, block polygons, plot
polygons, building footprints. A raster cannot carry them.

The format: a **line-delimited GeoJSON** artifact (`.vgeo`), one
feature per line, with:

- coordinates quantised to **integer centimetres** in the local metric
  frame before serialisation, satisfying the fixed-point rule and
  making byte-level hashing meaningful;
- features written in a fixed, documented order;
- the same blake3 manifest treatment every other artifact gets.

The reason to accept a text format here, having rejected one for
rasters, is the benchmark: ADR 0008 requires comparison against
reference populations measured with `osmnx` and `momepy`, both of
which read GeoJSON natively. A bespoke binary would force us to write
and maintain a bridge whose only purpose is to be read by the
instrument that judges us.

The per-feature schema — field names by feature type, the feature
ordering rule, the local metric frame's definition — is fixed before
the first crate emits a `.vgeo`, not here.

## Decision 4 — Where the benchmark lives

Three parts, deliberately separated:

- **Benchmark measurement lives in one dedicated crate, `via-bench`.**
  It computes the benchmark characters on *both* sides of every
  comparison it mediates — reference towns imported from OSM
  extracts, and generated fabric — through one code path;
  comparisons against published populations (GHS-UCDB, Boeing) are a
  different channel, governed by 0012 §9.4. That is the same-code
  rule the spike's validation record established: when real and
  synthetic fabric are measured by different implementations, the
  comparison measures the implementations, not the towns, which is
  how the spike's first retraction happened. `via-bench` is not a
  chain stage; like `via-viz`, it is never a dependency of a stage
  crate. It exists before any stage does, because the benchmark is
  built first and reference towns belong to no stage.
- **Stage crates keep their gates.** Internal consistency checks (ADR
  0008 D1) stay inside the stage they check, deterministic and
  hashed, part of the artifact contract. Stages do not compute
  benchmark characters.
- **Population statistics and reporting** live in an analysis sidecar
  under `analysis/`, in Python, because that is where `osmnx`,
  `momepy` and the GHSL/Dataverse readers are. The sidecar consumes
  the per-town character tables `via-bench` emits, and the published
  populations (GHS-UCDB, Boeing), and produces the validation
  report's statistics and figures. It is also the independent check
  on `via-bench` itself: on a pilot set of towns, characters computed
  by `via-bench` are cross-validated against `osmnx`/`momepy` at a
  declared tolerance, so definition drift between our implementation
  and the published ones is caught rather than shipped. The sidecar
  is a standing, version-pinned tool — CONTRIBUTING's Python rule, as
  amended, permits exactly this — and the quarantine is absolute: it
  may never be imported by a crate, and no gate may depend on it.

Reference extracts are stored outside version control, with licence
and snapshot date recorded (OpenStreetMap data is ODbL; GHSL and the
Boeing dataset carry their own terms). The store's layout and
manifest, and the rulings on the towns measured before the store
existed, are fixed in ADR 0010.

## Decision 5 — Spikes are not stages

`spikes/` stays outside `crates/`, is never a dependency, and is never
gated. Its purpose is to be thrown away; the two current spikes have
already produced their return — a measurement harness, a validated
protocol, and a documented list of what fails — and their generator
code is superseded by this ADR rather than promoted by it.

## What this fixes

- Corridors precede settlements, as the chain always said.
- Affordances (harbours, fords, confluences, passes) become a named
  stage responsibility with a place to live, instead of being absent.
- The fabric scale becomes independently testable through a written
  input contract.
- Benchmark tooling has a home that cannot contaminate the engine.

## Rejected

- **One `via-humanity` crate.** It would put a regional fixed-point
  solver and a metre-scale geometry kernel in one compilation unit
  with one config, and would hide the scale boundary that the
  benchmark, the literature and the unit of work all agree on.
- **Four crates split further** (`via-flows`, `via-epochs`): an
  artifact boundary inside a fixed-point loop.
- **Promoting either spike's generator into `crates/`.** The
  generators are known-invalid (spikes/townfabric/VALIDATION.md).
  This rejection does not extend to the townfabric measurement kernel
  — OSM import, planar graph, statistic battery: that code measured
  every defect the validation record reports — including, once
  characters were added under ADR 0008 D10, the two first caught by
  eye — and reimplementing it from prose would re-open exactly the
  definition-drift failures it exists to prevent. It is ported into
  `via-bench` with tests added, against the protocol and defect list
  VALIDATION.md records and research 0012 as the specification it
  must satisfy — never against VALIDATION.md's numbers, which
  ADR 0010 rules historical — and cross-validated against
  `osmnx`/`momepy` (Decision 4). Ported, not depended on in place:
  `spikes/` remains outside the dependency graph (Decision 5).
- **A bespoke binary vector format.** It would be marginally smaller
  and would isolate us from the tools that must read our output.

## References

- ADR 0001 (crate layout and artifact format); ADR 0003 (epistemic
  tiers); ADR 0008 (validation doctrine); ADR 0010 (benchmark
  instrument adoption and the reference corpus).
- docs/research/humanity/0011 (open questions — the two-scale coupling
  contract is Decision 2 here); 0012 (benchmark specification).
- README, "The causal chain".
