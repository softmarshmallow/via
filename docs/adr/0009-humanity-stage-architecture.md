# ADR 0009 — Where settlement and road-network solving lives

Status: **proposed**. Nothing here is implemented. This ADR answers a
structural question that the spikes exposed: the human side has no
home, so it grew as two monolithic throwaway binaries that violated
the project's own declared chain.

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

Proposal: a **line-delimited GeoJSON** artifact (`.vgeo`), one feature
per line, with:

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

## Decision 4 — Where the benchmark lives

Two halves, deliberately separated:

- **Measurement of our own output** stays in Rust, inside the stage
  crates, deterministic and hashed. It is part of the artifact
  contract.
- **Comparison against reference populations** lives in an analysis
  sidecar under `analysis/`, in Python, because that is where `osmnx`,
  `momepy` and the GHSL/Dataverse readers are. CONTRIBUTING permits
  exactly this: "a throwaway script run *against* an exported
  artifact, never a dependency of this project". The sidecar may never
  be imported by a crate, and no gate may depend on it.

Reference extracts are stored outside version control, with licence
and snapshot date recorded (OpenStreetMap data is ODbL; GHSL and the
Boeing dataset carry their own terms).

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
- **Promoting either spike into `crates/`.** Their generators are
  known-invalid (spikes/townfabric/VALIDATION.md); the parts worth
  keeping are the measurement protocol and the reference corpus, both
  of which are specifications, not code.
- **A bespoke binary vector format.** It would be marginally smaller
  and would isolate us from the tools that must read our output.

## References

- ADR 0001 (crate layout and artifact format); ADR 0003 (epistemic
  tiers); ADR 0008 (validation doctrine).
- docs/research/humanity/0011 (open questions — the two-scale coupling
  contract is Decision 2 here); 0012 (benchmark specification).
- README, "The causal chain".
