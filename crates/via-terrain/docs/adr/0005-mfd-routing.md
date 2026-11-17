# ADR 0005 — Multiple-flow-direction routing (M3.5)

Status: accepted (2026-08-13)
Scope: replaces single-receiver D8 flow with weighted multiple flow
directions. A discretization upgrade, not a physics change: the governing
equations, gates, and artifact contracts are untouched except where noted.

## Context

D8 is a discretization convention, not a law: real flow spreads on
divergent ground and takes continuous directions. Single-receiver routing
was structural to the Braun & Willett (2013) implicit solve — the
closed-form update needs every receiver solved before its donors — but
that requirement generalizes: with flow split among several strictly
lower neighbors, the receiver graph is a DAG, a DAG has a topological
order, and the same implicit trick works cell by cell against the
weighted sum of already-updated receivers (the direction taken by the
MFD-capable implicit schemes descending from Yuan et al. 2019).

The visible D8 pathologies this removes or softens (recorded from the M3
maps): channels locked to 0°/45°/90°, parallel fall-line fans on smooth
flanks, and the single-thread "comb" delta in lake fills. What it does
NOT produce: river meandering and braiding — separate physics (bank
erosion, secondary flow, bedload sorting), out of scope for any model at
this layer; declared again to prevent the upgrade from being oversold.

## Decision 1 — Freeman weights on the routed surface, converging in channels

Each cell sends flow to every strictly lower D8 neighbor of the routing
surface, weighted ∝ S^p (Freeman 1991), p = `mfd_exponent` in config
(default 1.1, Freeman's value). Base-level cells have no out-edges.
Priority-flood+ε guarantees every non-base cell at least one strictly
lower neighbor, so the graph is total. Edges, weights, donor lists, and
the channel tree live in one CSR structure (`flow::MfdGraph`).

**Amendment — hybrid convergence (measured, same day).** Pure MFD
everywhere braids valley floors into flat multi-thread sheets: at
island8k the ε-flat channel-cell count went 21 → 1228, Horton Rb
3.2 → 5.9, the hypsometric integral collapsed, and the lake never
deepened past the merge threshold. Channelized flow converges — the
principle behind Holmgren (1994)'s area-dependent exponent — so routing
is **hybrid**, taking that principle to its binary limit at the fluvial
threshold the project already declares: dry cells with discharge ≥
`fluvial_min_area_km2` route their single steepest edge; hillslopes and
flooded cells spread. Built in two passes per step (a pure-MFD pass
locates the channels, the hybrid pass converges them).

Extraction recalibration under the smoother flux field: the research
preset's `river_min_area_km2` moved 1.0 → 1.5 km² — the hybrid discharge
crosses a given threshold at more marginal heads, inflating first-order
stream counts (order-1 streams 70 → 136 at the old threshold, Rb 5.04);
the raised threshold selects a network comparable to the D8 one
(Rb 4.69). A threshold change, not a physics change; island8k's explicit
config needed no adjustment.

## Decision 2 — Topological order by routed height

Every edge points to a strictly lower routed height, so sorting cells by
(routed height, cell index) yields a valid topological order — receivers
before donors — with bitwise-deterministic ties. Ascending order drives
the implicit detachment solve (receivers already updated); descending
order drives discharge accumulation and the sediment sweep (donors
complete before their receivers). No Kahn bookkeeping needed.

## Decision 3 — The implicit solve over the DAG

    h⁺ = (h + Σᵢ cᵢ·hrᵢ_eff⁺) / (1 + Σᵢ cᵢ),   cᵢ = K·√Q·dt·wᵢ / distᵢ

summed over out-edges whose effective receiver height (water level for
flooded receivers, max(h, sea) at base) lies below the cell. Reduces
exactly to the M3 update when a cell has one edge. Flooded cells still
neither incise nor detach; the frozen-mask rule of ADR 0004 stands.
Sediment routing splits each cell's outflux by the same weights;
deposition rules (G-fraction on the fluvial domain, trap-to-level when
flooded, donor-floor cap) are unchanged, with the donor floor taken over
MFD donors.

## Decision 4 — The channel tree and the artifact contract

Statistics and extraction need a tree; the field standard is to extract
channels from the flux field along dominant directions. The **channel
tree** is each cell's maximum-weight receiver (= steepest, since w is
monotone in S; ties break to the lower index — identical tie policy to
M1 D8). Strahler orders, basins, flow distance, mainstream length, Hack
and Horton gates, and the `receivers` artifact all use the tree, so the
downstream water contract (self-receiver = ocean) is unchanged and
ecology/suitability/viz need no changes. `discharge` becomes the MFD
flux field (TWI over MFD flux is the index's classic form); `area_cells`
stays the tree's integer accumulation (Hack remains planimetric).

## Decision 5 — Gates under MFD

The slope–area and residual regressions keep measuring slope toward the
tree receiver (dominant-direction channel slope, as on real DEMs), but
the residual's fluvial term uses what erosion actually applied — the
weighted sum K·√Q·Σ wᵢ·Sᵢ over out-edges. Bands are unchanged;
calibration is re-verified at both scales rather than assumed.

## Rejected / deferred

- **Irregular (Voronoi) mesh** — the complete cure for lattice bias;
  rewrites every grid operation and the raster artifact format. Stays on
  the ledger for the M7 era.
- **D∞** — subsumed: Freeman MFD already interpolates direction and
  handles divergence better on fans and deltas.
- **Sub-8 m cells** — rejected on validity, not cost: stream power on
  drainage area and κ∇²h hillslopes are statistical laws that break near
  channel width; finer detail is M7's gated amplification.
