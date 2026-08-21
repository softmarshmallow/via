# ADR 0012 — Corridors: currency, modes, network extraction, and gates

Status: proposed (2026-08-22)
Scope: the stage decisions for `via-corridors` (ADR 0009 D1: unit of
work "route, cost surface") — the movement-cost adoption ADR 0011
deferred here, the multimodal mode set for the first (pre-modern)
era, how a corridor network is extracted without settlements, the
artifact forms, and the gates. Mechanisms are adopted from research
0005 (freight structure and era vectors), 0013 (§Movement cost, the
Cox stability bands), and 0014 (network extraction and multimodal
precedents — the dossier commissioned for this decision).

## Context

The causal chain (ADR 0009) runs suitability → **corridors** →
settlement seeds at network nodes: corridors must exist before any
settlement does, which rules out every demand-driven network model at
introduction (Louf's R = B − C needs city masses; freight catchments
need markets). What the stage can honestly compute pre-settlement is
the structure of least-cost movement over the terrain given a
declared movement model: where routes concentrate, and how the
terrain's own gateways (passes, heads of navigation, river mouths)
interconnect.

Research 0014's two negative findings frame everything: **no
endpoint-free network method exists** (endpoint-honesty — uniform
lattices, terrain-derived sites — is the published frontier), and
**no published model grows networks between natural affordance
sites** (the pieces exist separately and are individually citable;
their combination is via's, and is declared as such). ADR 0011
hands this stage a written consumption contract (its D5) and two
deferred adoptions: the movement-cost values and cross-era cost
normalization. Research 0011's header rule binds: bearing open
questions are resolved here or declared.

## Decision 1 — via-corridors is Tier-3 interpretation over declared Tier-2 cost forcing

The stage derives description — accumulated times, path densities,
least-cost trunk routes — over tier-1/2 fields under a **declared
movement model**. The cost functions are standard (cited below); the
mode rates and penalties are Tier-2 forcing under ADR 0003's
"economic cost weights (later)"; the assembly is via's. The stage
claims no realism: research 0012 defines no corridor characters, so
there is no benchmark channel, no band, and no statistical gate —
only D1 artifact-contract checks (Decision 6), exactly the ADR 0011
D2 reading. Validation is indirect and downstream: settlement-system
statistics once settlements exist, and the Strano
densification/exploration battery is the recorded future instrument
if a direct street/route-network claim is ever made (it would owe
new characters under ADR 0008 D2/D3 first). Every output is labeled
standard or heuristic in the stage summary (via-ecology's duty). All
water-mode times inherit `k_Q` conditionality through the terrain
discharge chain (velocities, navigability), and the summary restates
`metric_values_conditional_on_k_q` wherever they appear.

## Decision 2 — Currency: time for the solve, energy as a spectrum

0013's selection doctrine makes currency "an explicit declared
choice." Via declares **time (hours) as the corridor-solve currency**,
because it is the only citable cross-mode currency: 0014 documents
the absence of any boat-propulsion energy cost commensurable with
walking metabolic cost, and every multimodal precedent (ORBIS,
Livingood 2012, Filet & Rossi 2025) solves in time. Freight-price
vectors (Masschaele 8:4:1; the Edict's 1:5:10:52) are *within-era
cargo economics*, adopted later by the freight-catchment mechanism at
the settlement stage — not by the era-0 route skeleton.

- **Land time** — Tobler (1993): W = 6·e^(−3.5·|S + 0.05|) km/h, S =
  dh/dx (dimensionless gradient — Herzog's two misuse traps are
  honored: gradient units, and the function returns a speed that must
  be inverted to time). Base function, on-path form, unmodified — the
  ×0.6 off-path and ×1.25 horseback multipliers are not applied (the
  latter is asserted without data; 0013). Label: standard, with the
  provenance caveat that NCGIA 93-1 is a technical report (empirical
  basis Imhof 1950).
- **Switchback envelope on time** — the Llobera & Sluckin (2007)
  Eq. 20 criterion, M(s) − s·M′(s) = 0, applied to Tobler's own time
  function has closed-form roots **s_crit = ±1/3.5 ≈ ±0.2857** (on
  the smooth branches, T ∝ e^(±3.5(s+0.05)) gives T − sT′ =
  T(1 ∓ 3.5s)): beyond ±0.2857 the cell is traversed at the critical
  gradient with horizontal length inflated by |s|/s_crit — continuous
  at the seam by construction, anisotropy-free by symmetry of the
  roots, and corroborated by Herzog's statement that Tobler's
  critical slope is 25–30% (IA36). This keeps steep terrain finite
  and switchback-priced instead of exponentially divergent — no
  binary passability mask, per the 0008 refusal. Label: standard
  criterion (L&S) on a standard function (Tobler); the closed-form
  derivation is recorded above and is checkable by hand.
- **Energy spectrum (land only, not the solve currency)** — the
  Herzog IA36 §5.1.4.3 sixth-degree refit of Minetti 2002,
  Cost(s) = 1337.8·s⁶ + 278.19·s⁵ − 517.39·s⁴ − 78.199·s³ +
  93.419·s² + 19.825·s + 1.64 J/kg/m, no abs(), clamped to the
  ±0.45 fit range (via's inference, benign — the even-degree fit
  rises at both extremes), with the **L&S published critical
  gradients +0.28/−0.22 m/m frozen as its envelope constants**. The
  values are not recomputed from the sextic: 0014 records via's
  verification that Eq. 20 is multi-rooted on the sextic's downhill
  limb (fit oscillation at −0.41/−0.33/−0.14), so recomputation would
  require an arbitrary root-selection rule. Envelope constants from
  the primary, traversal polynomial from its primary, the pairing
  declared: standard links, via assembly. This **freezes the deferred
  movement-cost values** (ADR 0011 D4/D7): L&S gradients as stated;
  Herzog refit as stated; Pandolf/Santee (load parameter, downhill
  correction) recorded as the load-bearing extension when a pack
  model is needed, with Santee's TN03-3 validity limits (1.12–1.34
  m/s, ≤ 27 kg) carried from 0013. The energy spectrum ships per
  trunk edge (Decision 5); a time-vs-energy corridor divergence
  panel is a QA diagnostic (D10), never a gate.
- **Maximum practical grade** (research 0011's open question) —
  resolved: for foot/pack it is not a cap but the metabolic
  switchback transition above (L&S); the USFS 1935 ruling grade
  (15% for loaded pack animals, graded exceedances to 40% on way
  trails), FSH 2309.18 (targets 2–20% by class), and Hancock et al.
  2007 ("the caps are erosion control, not animal capability") are
  recorded as declared-forcing corroboration for *engineered* trails,
  not applied to era-0 movement. The wheeled regime is a separate,
  gentler, citable family for later eras: Herzog's quadratic
  Cost(s) = 1 + (s/ŝ)², ŝ = 8–15% (IA36, cart-road reconstruction),
  consistent with turnpike ~5% and rail 1–2.2% (0010).

## Decision 3 — Era-0 modes: foot and small craft, joined by declared switches

The first corridor era is pre-wheel land movement plus small-craft
water movement — the base layer every later era palimpsests onto.
The multimodal graph is node-split: each cell has a land node (dry
land: `receivers[i] ≠ i` and `water_depth = 0`) and/or a water node
(admissible water), with switch edges between them. Movement rules,
each config-declared with provenance:

- **Foot** on land nodes per Decision 2. Grid geometry per Herzog:
  **16-neighbour moves (Queen + Knight)**, Knight's moves subdivided
  into two submoves paying interpolated costs, bringing worst-case
  route displacement from 20% to 11% of path length (IA36; A/B-moves
  recorded as the upgrade if 11% ever shows in QA). Costs strictly
  positive; deterministic tie-breaks (cost, then node index).
- **River crossings** — rivers are barriers pierced by fords (Herzog's
  rule; the ADR 0011 crossability spectrum is the piercing
  instrument). A land move entering a river cell (strahler > 0) is
  admissible iff the cell's emitted crossability, depth, and velocity
  pass the Cox/AIDR people-stability caps — config, defaults
  `ford_max_dv = 0.8` m²/s (the recommended working limit for trained
  or well-equipped persons), `ford_max_depth_m = 1.2`,
  `ford_max_velocity_ms = 3.0` (independent caps; all Cox, Shand &
  Blacka 2010 / Guideline 7-3 via 0013) — and charges a wading delay
  `ford_delay_hours` (default 0.25; declared forcing, provenance
  Livingood 2012 Table 10.1, whose flow-banded crossing delays run
  0–30 min). Above the caps the cell is land-impassable — a cited
  stability limit, not an authored mask. All three caps read emitted
  spectra, so ford admissibility is k_Q-conditional; the summary says
  so.
- **Small craft** on water nodes: admissible on the ADR 0011
  navigable-predicate cells (rivers and still water); speed =
  `canoe_speed_kmh` (default 4.0; Livingood 2012: "probably any value
  between 3.5 and 5 km/hr could be defended") **plus the cell's
  emitted ford_velocity downstream, minus it upstream** (his "plus or
  minus the speed of the current"; ORBIS's 65/15 km-per-day civilian
  asymmetry is the corroborating anchor). An upstream edge whose net
  speed is ≤ 0 is inadmissible — towing is a later-era mode. Still
  water: base speed both directions. Label: standard values, via
  assembly (via gates admissibility on its own navigability predicate
  instead of Livingood's 100 cfs threshold, which is a declared
  substitution — a fixed-flow threshold would be k_Q-conditional
  anyway).
- **Coastal water** (config `coastal_mode`, default on): admissible on
  the coastal-water ribbon (ocean cells with a land D8 neighbour —
  the fetch domain) at `coastal_speed_kmh` (default 4.0). **Declared
  forcing, flagged**: no citable era-0 coastal speed exists without a
  wind climatology (ORBIS's sea legs are wind-driven sail; the
  wind-weighted family is already the recorded harbour upgrade path).
  Cabotage parity with the canoe base is the declaration, not a
  finding. `coastal_exposure_cap_m` (default off) optionally closes
  high-fetch cells to era-0 craft, reading the emitted fetch
  spectrum.
- **Mode switches**: a land↔water edge between adjacent (or
  co-located) admissible nodes costs `transship_hours` (default 0.25;
  declared forcing — 0014 documents the literature gap: ORBIS prices
  transshipment at zero, Livingood's half-crossing table is the only
  quantified precedent, Page 2026 is paywalled). Affordance *value*
  stays emergent, per ADR 0011 D3: heads of navigation and river
  mouths acquire centrality because switches happen there, never by
  authored weight. Harbour-component-weighted switch penalties are
  recorded as the ship-era rule (the D5 harbour contract is consumed
  by this stage's later eras, not era 0 — small craft land anywhere).

The ADR 0011 D5 contract is thereby consumed: heights/receivers
(cost geometry), crossability + ford fields (piercing), navigability
predicate + ford_velocity (water admissibility and asymmetry), site
sets (Decision 4 nodes), fetch (optional exposure cap), with
harbour depth-window/sediment deferred to ship eras and slope/
freshwater_dist/coast_dist available to consumers unchanged.

## Decision 4 — Network extraction: an endpoint-honest density spectrum and a site-anchored trunk graph

Two representations, both adopted from 0014, shipping side by side
with no composite:

- **Corridor density (FETE)** — White & Barber (2012): a uniform
  lattice (config `lattice_spacing_cells`, default 16) of sources on
  dry-land nodes; least-cost paths between all directed lattice pairs
  over the full multimodal graph; **per-cell traversal counts**
  accumulated into a u32 raster. Endpoint-honest (the lattice is the
  declared endpoint structure; spacing is the efficiency knob, with
  the paper's robustness finding that high-traffic routes persist
  across spacings). The raster ships raw — spectra over taxonomy; the
  Pareto 80/20 threshold is recorded as the cited rendering default
  (a tunable quantile, not a law — 0014 notes the power-law claim is
  asserted, not fitted). Water legs participate, so river highways
  and portages appear in density mechanically. Path counting is
  integer, so parallel accumulation over sources is order-independent
  and the stage stays bit-deterministic (recorded deviation from the
  suitability ADR's "stays sequential" wording, with this
  justification). **Boundary bias is a known, undocumented-in-the-
  literature artifact**: density is suppressed near study-area edges;
  it ships as a QA note and rendering halo, never a gate (0014).
- **Gateway trunk network (Stahlberg)** — node set = the terrain's
  own gateways, all recomputable from shipped artifacts: **passes**
  (ADR 0011 saddle sites), **heads of navigation**, and **river
  mouths** (river cells whose receiver is ocean). Confluences are
  deliberately not trunk endpoints — they are water-water junctions
  whose value emerges in traffic (density; water legs), per ADR 0011
  D3. Pair set K = the **Gabriel graph in cost-distance space**
  (Groenhuijzen & Verhagen 2017: empirically the best archaeological
  site-graph; symmetrized cost-distances per Herzog, averaged over
  both directions), followed by a connectivity check (Herzog's
  warning on fragmenting prunes; if K leaves components, the
  cheapest inter-component pairs are added — declared rule). Insertion
  order = **ascending symmetrized cost-distance** — Molinero &
  Hernando's decreasing-demand rule under uniform masses (no
  populations exist yet; gravity with equal masses degenerates to
  ascending distance), which keeps the order deterministic and
  authored-weight-free. Each connection is routed by least-cost path
  with **built edges discounted by `reuse_alpha`** (default 0.45, the
  midpoint of Stahlberg's fitted [0.4, 0.5]; single-region fit,
  portability flagged) — the path-dependence mechanism, and the
  archaeology-citable form of corridor consolidation. Label: standard
  links (Stahlberg mechanism, G&V pruning, Molinero ordering), via
  assembly — 0014's negative finding that no published pipeline
  combines them is restated in the summary provenance.

The order-sensitivity of sequential insertion is real (Stahlberg's
inferred-order machinery exists because order matters); the recorded
experiment is an **ensemble over insertion orders** (working rules:
≥ 5), reported as trunk-set stability, before any trunk-derived
number is quoted as evidence.

## Decision 5 — Representation, artifacts, and the contract onward

The stage writes, namespaced `corridors.<label>.*` (label rules and
manifest registration exactly as ADR 0011; one `StageRecord` keyed
`corridors.<label>`):

- `corridor_density.vrast` (u32) — FETE traversal counts.
- `trunk.vrast` (u32) — per cell, the number of trunk edges whose
  path traverses it.
- `hours_to_sea.vrast` (f32) — multimodal accumulated time from the
  tidewater source set (coastal-water nodes and ocean-adjacent land
  nodes at zero); `f32::MAX` = unreachable, the suitability sentinel.
- Summary JSON `corridors.<label>.json`: config echo; provenance
  labels per output; the trunk **node records** (site class, cell,
  x/y) and **edge records** in fixed order (endpoint node indices,
  time cost in each direction, land-leg energy in J/kg — water-leg
  energy has no citable form, 0014, and is null with the absence
  stated —, path length in metres, and the full path as a cell-index
  list); `metric_values_conditional_on_k_q: true` on every water-
  derived quantity; `checks`; `artifact_blake3`.
- **No `.vgeo`.** Cell-path records serve every internal consumer;
  no external tool needs to read corridors; the deferred per-feature
  schema obligation stays with morphology (ADR 0009 D3, the ADR 0011
  precedent).

Contract onward (ADR 0009 D2 spirit — content fixed here, field-level
schema fixed with the implementation): `via-settlement` consumes the
trunk node/edge records (a settlement attached to a corridor derives
its D2 attachment — bearing from the path geometry at the attachment
cell, class from the edge's era layer, route cost from the recorded
time — by construction), the corridor-density spectrum (seed
covariate), and `hours_to_sea` (market-access proto-field). Era-0
edges all carry class `foot_trail`; the class enum grows with era
layers (Decision 7).

## Decision 6 — The gates

All binary, all D1 internal-consistency, unevaluable is fail; every
recomputation reads the artifacts back from disk (the ADR 0011
discipline):

- **Determinism**: byte-identical artifacts and hashes on re-run.
  Parallel FETE accumulation is admissible only because counts are
  integers summed per source raster in fixed order; everything else
  stays sequential. Hashes witness same-platform reruns (libm scope,
  as ADR 0011).
- **Grid agreement**: every raster matches heights_cm's geometry.
- **Density definition**: `corridor_density` equals a full recompute
  from the disk-read inputs (terrain + suitability rasters + config)
  — the stage pays the 2× cost; a cheaper witness would not verify
  the definition.
- **Density bounds**: every cell ≤ the directed lattice-pair count;
  lattice source cells carry ≥ 1.
- **Trunk identities**: every edge's stored path is connected under
  the declared move set with every step admissible; recomputed
  direction costs along the stored path equal the stored costs
  (exact f32, same code path); endpoints equal the declared node set
  (recomputed from disk artifacts — saddle sites from the summary,
  heads from the predicate raster, mouths from receivers/strahler);
  `trunk.vrast` equals the traversal counts derived from the stored
  paths.
- **Bellman optimality**: `hours_to_sea` is a fixpoint — no admissible
  edge (j → i) has hours[j] + cost(j,i) < hours[i], recomputed from
  disk; source cells are exactly the declared tidewater set at 0.
- **Preconditions**: config `validate()` (positivity, finiteness,
  domains, `deny_unknown_fields`), receiver range validation, and
  input-artifact validity run before any gate; corrupt inputs fail
  loudly.

QA diagnostics (D10 visual channel, never in CI or the summary):
the boundary-bias halo on density; time-optimal vs energy-optimal
trunk divergence; density/trunk overlay against the affordance panel.

## Decision 7 — Era structure and dispatch of the bearing open questions

- **This chunk is era 0 only.** Later eras are new mode layers and
  edge classes — wagon (Herzog quadratic, ŝ = 8–15%), tow/barge,
  canal, rail — whose availability dates are declared forcing (0005:
  "which edge types exist" is forcing), acting on **new accretion
  only** while the α-discounted trunk carries memory between eras
  (the 0008 palimpsest refusal, mechanized by the reuse discount).
  Demand-driven growth (Louf R = B − C; freight LCP catchments with
  the Masschaele/Edict price vectors) activates once settlements
  with sizes exist — recorded, not adopted here.
- **Cross-era cost normalization** (research 0011; deferred here by
  ADR 0011 D7) — **resolved by declaration**: cost vectors are
  within-era relative structures (0010's rule elevated to stage law);
  the time currency is dimensionally cross-era but *speeds are
  era forcing*, and no cross-era cost comparison is ever claimed or
  emitted. Nominal price series are never used.
- **Era transitions** (research 0011) — declared: availability dates
  are dated switches on new accretion; the trajectory-shape question
  (continuous paths vs switches inside demand mechanisms with
  bifurcations) is deferred to the settlement-coupling ADR, which
  owns those mechanisms.
- **Equilibrium selection / path dependence** — the insertion order
  is the declared selection rule; its sensitivity is quantified by
  the order-ensemble experiment (Decision 4) before trunk numbers
  are evidence.
- **Declared limitations** (0013/0014 gaps): no pack-equid cost
  function exists (Tobler ×1.25 unsourced — not applied); no
  seasonality (pass closure, low water — uncoverable from via's
  inputs); no substrate/footing or bank-condition scheme for fords;
  transshipment and coastal speeds are forcing with documented
  literature absences (Filet & Rossi 2025 and Page 2026 are the
  flagged retrievals that could replace them); FETE boundary bias is
  characterized visually, not corrected; k_Q conditionality pervades
  every water-derived number.

## Consequences

- Research 0014 joins the corpus as the corridor dossier; its
  retrieval targets (Filet & Rossi; Page 2026; Duncan-Jones p. 368;
  Llobera et al. 2011 full text) are the checklist for upgrading
  declared forcing to citation.
- The movement-cost deferral from ADR 0011 is discharged: L&S
  gradients, Herzog refit, and the Pandolf/Santee load extension are
  frozen with their primaries; the multi-rooted-sextic finding is on
  record in 0014.
- `via-corridors` enters the workspace as a stage crate (serde +
  via-artifact only; via-viz gains a corridor panel; the CLI a
  `corridors` subcommand over an existing run dir).
- The stage introduces the workspace's first weighted-graph solver
  (16-neighbour, node-split, predecessor-tracked); via-suitability's
  private distance Dijkstra is unrelated and stays private.
- Deferred by this ADR, recorded: A/B-moves (7×7 neighbourhood) if QA
  shows grid artifacts; MADO accumulations from affordance sites as
  an additional QA/render channel; circuit-theory redundancy audit
  (isotropic, symmetrized) as a possible future diagnostic; the
  Helbing self-consistent-field solve as a spike candidate for
  emergent bundling; wind climatology for sea legs.
- Runtime honesty: the density definition gate doubles the FETE cost
  by design; the reference grid pays minutes, not hours. If grid
  sizes grow past feasibility, the lattice spacing is the declared
  knob — never silent sampling.

## Rejected

- **Energy as the multimodal solve currency.** 0014 documents the
  absence of any citable boat energy cost; a declared land↔water
  energy conversion would be an invented constant wearing a
  standard's clothes. Time is citable on both legs.
- **Freight-price vectors as the era-0 currency.** Masschaele/Edict
  ratios are cargo economics for the freight-catchment mechanism
  (settlement stage); using them to route era-0 foot movement would
  claim more than the sources say.
- **Circuit theory as the skeleton.** Resistors are isotropic (McRae
  2008, verbatim restriction); via's movement cost is slope-
  anisotropic. Recorded as a symmetrized audit instrument only.
- **Helbing active walkers as the workhorse.** Deterministic solve
  exists but needs one field per OD pair and has no published κ, λ
  calibration. Spike candidate.
- **Demand-inferred insertion order** (Stahlberg's EDA). It fits
  order to archaeological evidence; via has none. The uniform-mass
  Molinero order is deterministic and assumption-minimal.
- **Authored node weights or a corridor-importance composite.**
  Density and trunk ship separately; any combination is consumer
  config (the ADR 0011 no-composite precedent).
- **Confluences as trunk endpoints.** Water-water junctions; their
  gateway value emerges in traffic. Adding 461 endpoints would also
  swamp K with river-parallel land edges.
- **`.vgeo` for corridor routes.** Wrong beneficiary of the deferred
  schema obligation (ADR 0011 precedent); cell-path records suffice.
- **A hard-coded haul limit, walking-city radius, or era style.**
  0008 refusals; all such figures must emerge or remain config.
- **8-neighbour-only movement.** Herzog's 20% worst-case route
  displacement is disqualifying for a stage whose output *is* routes.

## References

- Research 0014 (corridor network extraction and multimodal movement
  — adopted schemes' citations live there); 0013 (§Movement cost,
  fords/Cox bands, navigability); 0005 (freight structure, era
  vectors); 0010 (era parameters); 0011 (open questions); 0008
  (doctrine screen).
- ADR 0003 (tiers; "economic cost weights (later)"); ADR 0008
  (validation doctrine); ADR 0009 (stage architecture, D2 contract);
  ADR 0011 (suitability contract, representation and gate
  precedents).
- Key primaries via the dossiers: White & Barber 2012; Stahlberg et
  al. 2023; Molinero & Hernando 2020 (preprint); Groenhuijzen &
  Verhagen 2017; Herzog IA36 & A&C 25; Llobera & Sluckin 2007;
  Tobler 1993; Livingood 2012; Scheidel & Meeks 2012/2014; Masschaele
  1993 (via Langdon & Claridge 2011); Cox, Shand & Blacka 2010.
