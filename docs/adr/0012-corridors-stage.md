# ADR 0012 — Corridors: currency, modes, network extraction, and gates

Status: accepted (2026-08-22, under the user's delegated authority;
proposed and amended the same day after the three-track design review
— corner-cut crossing rule, per-component trunk patching,
per-node-class hours rasters with recompute gates, k_Q degeneracy
declaration, u32 domain guard, contract additions (hours_to_trunk,
junction records, per-step modes), min-form energy envelope,
provenance and citation repairs — then amended again the same day
after the two-track implementation review: the watercourse
formulation of the straddle rule and its drainage-chain water
exemption, Knight-intermediate enterability, either-corner
admissibility, standing-water wading delay, reverse-settle-order
accumulation, physical-scale config bounds, sentinel-safe casts,
loud failure on malformed site records, and the measured behaviour
of the energy min form)
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
standard or heuristic in the stage summary (via-ecology's duty).

Constant regimes, stated once for the whole ADR: coefficients
interior to an adopted published formula (Tobler's 6, 3.5, 0.05; the
Herzog polynomial) are cited-and-fixed in source, ADR 0003's
"standard" reading; every via-chosen value — clamps, envelope
constants, caps, speeds, penalties, spacings, α — lives in config
with provenance and passes `validate()` (the climate.rs precedent;
ADR 0011 D1). All water-mode quantities inherit `k_Q` conditionality
through the terrain discharge chain (velocities, navigability), and
the summary restates `metric_values_conditional_on_k_q` wherever
they appear — with the degeneracy consequences declared in
Decision 3.

## Decision 2 — Currency: time for the solve, energy as a spectrum

0013's selection doctrine makes currency "an explicit declared
choice." Via declares **time (hours) as the corridor-solve
currency**: 0014 documents the absence of any boat-propulsion energy
cost commensurable with walking metabolic cost, so per the dossier a
joint currency must be time or a declared price vector — and time is
the only *citable* cross-mode currency (Livingood 2012 solves in
time; ORBIS solves its cheapest-route channel on the Edict price
vector and its fastest-route channel in time). Freight-price vectors
(Masschaele 8:4:1; the Edict's 1:5:10:52) are within-era cargo
economics, adopted later by the freight-catchment mechanism at the
settlement stage — not by the era-0 route skeleton.

- **Land time** — Tobler (1993): W = 6·e^(−3.5·|S + 0.05|) km/h, S =
  dh/dx (dimensionless gradient — Herzog's two misuse traps are
  honored: gradient units, and the function returns a speed that must
  be inverted to time). The base on-path function is used unmodified,
  which is Livingood's own precedent ("Foot = Tobler unmodified",
  0014) in a landscape that has no paths yet; the ×0.6 off-path
  multiplier is exposed as config `offpath_factor` (default 1.0,
  declared — note a global land factor is LCP-neutral on land but
  shifts the land:water mode share, which is exactly the ratio 0014
  records as outcome-deciding); the ×1.25 horseback multiplier is
  not applied (asserted without data; 0013). Label: standard, with
  the provenance caveat that NCGIA 93-1 is a technical report
  (empirical basis Imhof 1950).
- **Switchback envelope on time** — the Llobera & Sluckin (2007)
  Eq. 20 criterion, M(s) − s·M′(s) = 0, applied to Tobler's own time
  function has closed-form roots **s_crit = ±1/3.5 ≈ ±0.2857** (on
  the smooth branches, T ∝ e^(±3.5(s+0.05)) gives T − sT′ =
  T(1 ∓ 3.5s); the +0.05 offset cancels as a constant factor, and the
  kink at −0.05 lies between the roots): beyond ±0.2857 the cell is
  traversed at the critical gradient with horizontal length inflated
  by |s|/s_crit — tangent (C¹) at the seams by construction, and
  corroborated by Herzog's statement that Tobler's critical slope is
  in the range 25–30% (IA36 §5.1.4.2, verified in 0014). This keeps
  steep terrain finite and switchback-priced instead of exponentially
  divergent — the continuous alternative to a binary passability
  mask, the virtue 0013 records for the L&S envelope. Enveloping
  beats clamping here: clamped-Tobler descent of a 100% grade would
  cost 0.38 h per horizontal km against the envelope's 1.33 h,
  cheap enough to reroute scarp crossings past real passes. All
  per-distance costs in this ADR are applied per horizontal metre;
  the ≤ 4% along-slope length discrepancy inside the envelope range
  is absorbed as declared.
- **Energy spectrum (land only, not the solve currency)** — the
  Herzog IA36 §5.1.4.3 sixth-degree refit of Minetti 2002,
  Cost(s) = 1337.8·s⁶ + 278.19·s⁵ − 517.39·s⁴ − 78.199·s³ +
  93.419·s² + 19.825·s + 1.64, in J/kg/m — the primary prints
  "kilo-joule" per kg per metre, a label slip corrected by 0013's
  note (the constant term is Minetti's measured level-walking
  1.64 J/kg/m); no abs(); clamped to the ±0.45 fit range (via's
  inference, benign — the even-degree fit rises at both extremes).
  The envelope takes the **min form**: cost(s) = min(clamped
  polynomial, C(s_c)·|s|/s_c) with the **L&S published critical
  gradients +0.28/−0.22 m/m frozen as the ray constants** — min
  because the frozen constants come from a different polynomial
  (L&S's own quartic), so the ray is not guaranteed to be the cheaper
  strategy at every gradient and the min is what makes the
  construction mean "take the cheaper of straight-walking and
  switchbacking" regardless. Measured on the adopted pair
  (implementation review, 2026-08-22): the ray binds only
  **downhill** (−0.22 to −0.54) and in a thin uphill sliver
  (0.450–0.453); across the whole uphill limb 0.28–0.45 the sextic
  itself is cheaper (11.34 vs 11.51 J/kg/m at s = 0.30; 16.69 vs
  16.88 at 0.44), so the min silently keeps the polynomial there.
  Beyond the ray/polynomial crossover the field is *constant* at the
  clamp value (17.379 J/kg/m up, 3.521 down) — a flat asymptote for
  arbitrarily steep ground, recorded here as a known artifact of
  pairing a clamped fit with a frozen ray; it is bounded, positive,
  and never a solve currency (time, which does diverge with slope,
  decides every route). The constants are not recomputed from the
  sextic: 0014 records via's verification that Eq. 20 on the sextic
  is single-rooted uphill (+0.3814) but multi-rooted downhill
  (−0.41/−0.33/−0.14, fit oscillation), so recomputation would need
  an arbitrary root-selection rule. Envelope
  constants from their primary, traversal polynomial from its
  primary, the pairing declared: standard links, via assembly. This
  **freezes the deferred movement-cost values** (ADR 0011 D4/D7):
  L&S gradients as stated; Herzog refit as stated; Pandolf/Santee
  (load parameter, downhill correction) recorded as the load-bearing
  extension when a pack model is needed, with Santee's TN03-3
  validity carried from 0013 as pairs — derived at 1.34 m/s with
  loads to 18.1 kg, field-valid at 1.12 m/s to 27 kg, not acceptable
  at 0.89 m/s. Per 0013's caveat, the sextic's curve *shape* is the
  citable content (ten elite mountain athletes); absolute J/kg
  magnitudes carry that caveat in the summary. The energy spectrum
  ships per trunk edge (Decision 5); a time-vs-energy corridor
  divergence panel is a QA diagnostic (D10), never a gate.
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
The multimodal graph is node-split: each cell has a land node and/or
a water node, with switch edges between them. Movement rules, each
config-declared with provenance:

- **Foot** on land nodes per Decision 2. Land nodes exist on dry land
  (`receivers[i] ≠ i`, `water_depth = 0`) **and on standing-water
  cells passing the wading caps below** (crossability there equals
  depth by the ADR 0011 identity, and the wading delay below is
  charged for entering one) — otherwise an ankle-deep pond would be a
  harder barrier than a torrent. Grid geometry per Herzog:
  **16-neighbour land moves (Queen + Knight)**, Knight's moves
  subdivided into two submoves paying interpolated costs, bringing
  worst-case route displacement from 20% to 11% of path length
  (IA36; A/B-moves recorded as the upgrade if 11% ever shows in QA).
  A Knight move's segment traverses the interiors of exactly two
  intermediate cells, and **both must be enterable in their own
  right** — the strict reading of Herzog's "cannot skip barriers",
  adopted in place of a straddle-pair test for the submoves, since a
  Knight step passes *through* those cells rather than between them.
  Costs strictly positive; deterministic tie-breaks (cost, then node
  index).
- **No corner-cutting.** A diagonal land move *straddles* the two
  corner cells {(x1,y2), (x2,y1)}. Declared straddle rule: the corner
  pair **carries a watercourse** when either the two corners are
  consecutive on the drainage chain below a channel cell — the thread
  itself then runs diagonally through the shared corner, which is how
  a river reaches the sea at its mouth — or both corners are water of
  any kind (channel, standing, or ocean), so no dry ground separates
  them. A move over a watercourse pair is a **crossing**: admissible
  only if a corner passes the wading caps, and the wading delay is
  charged. A move whose corners are both un-enterable is inadmissible.
  Everything else passes free. Without this rule a diagonal between
  two dry cells jumps a diagonally-stepping channel while entering no
  river cell — the review probe counted 2,705 such sites on the
  reference run (m4c-research-s42), where they would have been the
  *only* crossings. This is the barrier half of Herzog's subdivision
  requirement ("cannot skip barriers"), stated as via's explicit
  rule. The admissibility test is *either* corner rather than a
  distinguished "cheaper" one: the crossing price is a flat delay, so
  the corners differ only in whether they are passable at all.
  Water moves mirror it: a diagonal water move whose straddled cells
  are both dry land is inadmissible (a boat does not squeeze between
  touching corners of a spit) — **except when the two water cells are
  themselves consecutive on the drainage chain**, where the channel
  demonstrably runs through the corner and the sub-cell geometry the
  raster cannot resolve is the channel's own. Without that exemption
  every diagonally-stepping channel would be unnavigable and the
  river-mode machinery would be structurally dead (2,699 such steps
  on the reference run at calibrated k_Q).
- **River crossings** — rivers are barriers pierced by fords (Herzog's
  rule; the ADR 0011 crossability spectrum is the piercing
  instrument). A land move entering — or, per the straddle rule,
  crossing — a river cell (strahler > 0) is admissible iff the
  cell's emitted crossability, depth, and velocity pass the Cox/AIDR
  people-stability caps — config, defaults `ford_max_dv = 0.8` m²/s
  (the recommended working limit for trained or well-equipped
  persons), `ford_max_depth_m = 1.2`, `ford_max_velocity_ms = 3.0`
  (independent caps; all Cox, Shand & Blacka 2010 / Guideline 7-3
  via 0013) — and charges `ford_delay_hours` (default 0.25; declared
  forcing, provenance Livingood 2012 Table 10.1, whose flow-banded
  crossing delays run 0–30 min). The delay is charged **per entering
  or crossing move**, so a multi-cell channel charges once per cell
  of width (cell-size-dependent by construction, declared) and
  walking *along* a channel pays it every step — intended, it prices
  riverbed travel out (the riverbed-LCP artifact Herzog warns of).
  Above the caps the cell is land-impassable — a cited stability
  limit, not an authored mask.
- **Small craft** on water nodes: admissible on the ADR 0011
  navigable-predicate cells (rivers and still water); water moves are
  **8-neighbour** (channels are one cell wide; 16-neighbour is a
  land-mode geometry and would only add corner-cut exposure); speed =
  `canoe_speed_kmh` (default 4.0; Livingood 2012: "probably any value
  between 3.5 and 5 km/hr could be defended") **plus the mean of the
  two cells' emitted ford_velocity downstream, minus it upstream**
  (his "plus or minus the speed of the current"; ORBIS's 65/15
  km-per-day civilian asymmetry is the corroborating anchor). An
  upstream edge whose net speed is ≤ 0 is inadmissible — towing is a
  later-era mode. Still water: base speed both directions. Label:
  standard values, via assembly (via gates admissibility on its own
  navigability predicate instead of Livingood's 100 cfs threshold —
  a declared substitution; a fixed-flow threshold would be
  k_Q-conditional anyway). The continuous navigability_ts spectrum
  is deliberately **not consumed at era 0** — the banded predicate
  suffices for small craft; the spectrum's recorded future consumer
  is era-graded admissibility for larger vessel classes.
- **Coastal water** (config `coastal_mode`, default on): admissible on
  the coastal-water ribbon (ocean cells with a land D8 neighbour —
  the fetch domain; per-landmass by construction, which Decision 4's
  component rule handles) at `coastal_speed_kmh` (default 4.0).
  **Declared forcing, flagged**: no citable era-0 coastal speed
  exists without a wind climatology (ORBIS's sea legs are wind-driven
  sail; the wind-weighted family is already the recorded harbour
  upgrade path). Cabotage parity with the canoe base is the
  declaration, not a finding. `coastal_exposure_cap_m` (default off)
  optionally closes high-fetch cells to era-0 craft, reading the
  emitted fetch spectrum.
- **Mode switches**: a land↔water edge between the two nodes of a
  cell, or between adjacent admissible nodes of opposite classes,
  costs `transship_hours` (default 0.25; declared forcing — 0014
  documents the literature gap: ORBIS prices transshipment at zero,
  Livingood's half-crossing table is the only quantified precedent,
  Page 2026 is paywalled). Affordance *value* stays emergent, per
  ADR 0011 D3: heads of navigation and river mouths acquire
  centrality because switches happen there, never by authored
  weight. Harbour-component-weighted switch penalties are recorded
  as the ship-era rule (the D5 harbour contract is consumed by this
  stage's later eras, not era 0 — small craft land anywhere).

**Declared degeneracy at the k_Q placeholder.** Under the shipped
`k_q_m3s_per_unit = 1.0` placeholder and the default caps, the
reference run (m4c-research-s42, suitability defaults) has **100% of
its 8,464 channel cells land-impassable** (median D·V 7.96 m²/s) and
**4 navigable channel cells — the mouths themselves**: the ford and
river-highway machinery is provably inert, and the emergent network
shape (headwater detours, coastal hugging) is in large part an
artifact of one uncalibrated forcing constant. This is honest
conditionality only when stated: the summary carries a mandatory
**degeneracy panel** (ford-passable channel fraction, navigable
channel fraction, per-mode admissible-edge counts), and **k_Q
calibration — or a declared reference k_Q with provenance — is a
prerequisite for any corridor-derived number being quoted as
evidence**, the same status the order ensemble has for trunk
numbers (Decision 4).

The ADR 0011 D5 contract is thereby consumed or explicitly
dispatched: heights/receivers (cost geometry), crossability + ford
fields (piercing), navigability predicate + ford_velocity (water
admissibility and asymmetry), the saddle and head-of-navigation site
sets (Decision 4 nodes), fetch (optional exposure cap) — while the
navigability spectrum (above), the confluence sites with their
symmetry ratio (rejected as endpoints, Decision 4 — their value
emerges in traffic), the harbour depth-window/sediment components
(ship eras), and slope/freshwater_dist/coast_dist (available to
consumers unchanged) are declared rather than silently dropped.

## Decision 4 — Network extraction: an endpoint-honest density spectrum and a site-anchored trunk graph

Two representations, both adopted from 0014, shipping side by side
with no composite:

- **Corridor density (FETE)** — White & Barber (2012): a uniform
  lattice of sources on dry-land nodes (config
  `lattice_spacing_cells`; default = grid side / 32, i.e. 16 on the
  512² reference — size-relative so the default never silently
  under-samples or explodes); least-cost paths between all directed
  lattice pairs over the full multimodal graph; **per-cell traversal
  counts** accumulated into a u32 raster. Counts are directed:
  A→B and B→A both count, the paper's own procedure — a consumer
  reading undirected flow halves it. Endpoint-honest (the lattice is
  the declared endpoint structure; spacing is the efficiency knob,
  with the paper's robustness finding — established on a land-only
  graph, flagged as such — that high-traffic routes persist across
  spacings). The raster ships raw — spectra over taxonomy; the
  Pareto 80/20 threshold is recorded as the cited rendering default
  (a tunable quantile, not a law — 0014 notes the power-law claim is
  asserted, not fitted). Water legs participate, so river highways
  and portages can appear in density mechanically — at calibrated
  k_Q, per Decision 3's degeneracy declaration. Label: standard
  mechanism (White & Barber 2012), via cost model and multimodal
  graph — declared adaptations: 16-neighbour land moves (Herzog),
  water participation (the paper flags water off-limits), the
  size-relative lattice default. Scaling is declared, not silent:
  the solve is Θ((N/spacing²) · N log N) plus O(N) accumulation per
  source over the predecessor DAG (subtree counts, the intended
  implementation — never per-target path walks; the DAG is walked in
  **reverse settle order**, which is children-before-parents by
  construction and therefore correct even where rounding makes a
  child's accumulated distance equal its parent's, a case a
  distance sort gets wrong), and the definition gate doubles it. The
  configuration is **rejected when the directed pair count would
  exceed u32::MAX** (the silent-wrap cliff sits ~16× above an
  8192²/spacing-16 lattice); the check lives with lattice resolution
  rather than in `validate()` because it needs the grid dimensions,
  and it runs before any artifact is written or gate evaluated.
  Path counting is integer, so parallel accumulation over sources is
  order-independent and the stage stays bit-deterministic given
  overflow-freedom (recorded deviation from the suitability ADR's
  "stays sequential" wording, with this justification). **Boundary bias is a known,
  undocumented-in-the-literature artifact**: density is suppressed
  near study-area edges; it ships as a rendering halo in the D10
  visual channel only — not a gate, not a summary field.
- **Gateway trunk network (Stahlberg)** — node set = the terrain's
  own gateways, all recomputable from shipped artifacts: **passes**
  (ADR 0011 saddle sites), **heads of navigation**, and **river
  mouths** (river cells whose receiver is ocean). Confluences are
  deliberately not trunk endpoints — they are water-water junctions
  whose value emerges in traffic, per ADR 0011 D3; on the reference
  run they would add 461 endpoints (m4c-research-s42, suitability
  defaults) and swamp K with river-parallel land edges. Pair set K =
  the **Gabriel graph in cost-distance space** (Groenhuijzen &
  Verhagen 2017, abstract-verified: empirically the best-performing
  archaeological site-graph in the one published comparison, run on
  cost-distances; symmetrized per Herzog, averaged over both
  directions). **Pairs with no admissible path are exempt from
  everything that follows**: the trunk graph is built per reachable
  component (the coastal ribbon is per-landmass, so multi-landmass
  maps legitimately partition), the summary records the component
  partition of the node set, and connecting landmasses is a future
  declared mode (open-water crossing), never a patch rule. Within a
  reachable component, if the Gabriel prune leaves sub-components
  (Herzog's fragmentation warning), a Kruskal-style patch applies:
  while more than one sub-component remains, add the globally
  cheapest cross-component pair by symmetrized cost-distance, ties
  by node-index pair. Insertion order = **ascending symmetrized
  cost-distance, ties by node-index pair** — Molinero & Hernando's
  decreasing-demand rule under uniform masses (no populations exist
  yet; gravity with equal masses degenerates to ascending distance),
  deterministic and authored-weight-free. Each connection is routed
  by least-cost path with **built land edges' time multiplied by
  `reuse_alpha`** (default 0.45, the midpoint of Stahlberg's fitted
  [0.4, 0.5]; single-region fit, portability flagged) — Stahlberg's
  multiplicative rule exactly; water and switch edges are never
  discounted (you do not build a river — a declared rule, no
  precedent either way per 0014). A trunk path may pass through a
  third site; the edge is left intact (junctions capture it,
  Decision 5). Label: standard links (Stahlberg mechanism, G&V
  pruning, Molinero ordering), via assembly — 0014's negative
  finding that no published pipeline combines them is restated in
  the summary provenance.

The order-sensitivity of sequential insertion is real (Stahlberg's
inferred-order machinery exists because order matters); the recorded
experiment is an **ensemble over insertion orders** (working rules:
≥ 5), reported as trunk-set stability, before any trunk-derived
number is quoted as evidence.

## Decision 5 — Representation, artifacts, and the contract onward

The stage writes, namespaced `corridors.<label>.*` (label rules and
manifest registration exactly as ADR 0011; one `StageRecord` keyed
`corridors.<label>`):

- `corridor_density.vrast` (u32) — FETE directed traversal counts.
- `trunk.vrast` (u32) — per cell, the number of trunk edges whose
  path traverses it.
- `hours_to_sea_land.vrast` and `hours_to_sea_water.vrast` (f32) —
  multimodal accumulated time from the tidewater source set
  (coastal-water nodes and ocean-adjacent land nodes at zero),
  **projected per node class** — a single min-projected raster would
  make any per-cell consistency check ill-posed across the
  transshipment seam; `f32::MAX` = no node or unreachable, the
  suitability sentinel, and every reachable value is held strictly
  below it so a saturating cast can never impersonate the sentinel.
  Settlement's market-access reading is the land raster.
- `hours_to_trunk.vrast` (f32) — land-node accumulated time to the
  nearest trunk-path cell (sources: every cell traversed by any
  trunk edge, in its traversal mode, at zero), so the settlement
  stage never reimplements the movement model to compute attachment
  cost.
- Summary JSON `corridors.<label>.json`: config echo; provenance
  labels per output; the degeneracy panel (Decision 3); the trunk
  **node records** (site class, cell, x/y, component id) and **edge
  records** in fixed order (endpoint node indices, undiscounted
  traversal time in each direction — the α-state solve costs are
  internal to construction and not artifact fields —, land-leg
  energy in J/kg per direction (water-leg energy has no citable
  form, 0014, and is null with the absence stated), path length in
  metres, and the path as a list of **(cell, mode) steps** — mode ∈
  {land, water}, switches implied by mode change — so consumers can
  site ports without re-deriving modes); **junction records**: build
  the union graph of trunk paths (cells as vertices, consecutive
  steps as edges); a junction is a vertex of degree ≥ 3 or a trunk
  endpoint — mechanically derived, no authored weight — recorded as
  (cell, x/y, degree, incident edge ids); the component partition;
  `metric_values_conditional_on_k_q: true`, declared once at the top
  level and governing every water-derived quantity in the document
  (times, ford admissibility, navigability) rather than repeated per
  field; `checks`; `artifact_blake3`.
- **No `.vgeo`.** Cell-path records serve every internal consumer;
  no external tool needs to read corridors; the deferred per-feature
  schema obligation stays with morphology (ADR 0009 D3, the ADR 0011
  precedent).

Contract onward (ADR 0009 D2 spirit — content fixed here, field-level
schema fixed with the implementation): `via-settlement` consumes the
trunk node/edge/junction records (a settlement attached to a corridor
derives its D2 attachment — bearing from the path geometry at the
attachment cell, class from the edge's era layer, route cost from the
recorded time — by construction), the corridor-density spectrum
(seed covariate), `hours_to_sea_land` (market-access proto-field),
and `hours_to_trunk` (attachment cost). Era-0 edges all carry class
`foot_trail`; the class enum grows with era layers (Decision 7).

## Decision 6 — The gates

All binary, all D1 internal-consistency, unevaluable is fail;
enforced as in-stage hard errors and as cargo tests (the repo's CI
channel, as ADR 0011). Every gate re-reads the stage's *inputs* from
disk — terrain rasters, suitability rasters, and the suitability
summary's site records, none of them carried over from the compute
pass — and compares emitted rasters byte-for-byte after reading them
back. The site-record structures are compared recompute-against-
memory rather than against the summary file, because the summary is
written after the gates; their inputs are disk-read all the same.
Malformed inputs are a hard error, never a silent default: a
suitability summary missing a site array, or carrying an entry
without a numeric cell, fails rather than yielding an empty node set
that every gate would then happily certify.

- **Determinism**: byte-identical artifacts and hashes on re-run.
  Parallel FETE accumulation is admissible only because counts are
  integers (commutative addition) and the config guard excludes
  overflow; everything else stays sequential. Hashes witness
  same-platform reruns (libm scope, as ADR 0011).
- **Grid agreement**: every raster matches heights_cm's geometry.
- **Definition gates by full recompute**: `corridor_density`, both
  `hours_to_sea_*` rasters, and `hours_to_trunk` each equal a full
  re-solve from the disk-read inputs (terrain + suitability rasters
  + config) through the same code path — byte-identical. The stage
  pays the 2× cost; a cheaper witness would not verify the
  definition, and edge-relaxation fixpoint checks are structurally
  false-positive-prone here (mode mixing across the transshipment
  seam; f64→f32 rounding at exact-equality relaxations), so the
  fixpoint formulation is demoted to a QA diagnostic with a stated
  ulp tolerance, never a gate.
- **Density bounds**: every cell ≤ the directed lattice-pair count;
  every lattice source cell that **shares a reachable component with
  at least one other source** carries ≥ 1 (an isolated source on a
  private component legitimately carries 0 — reachability is tested
  outward from the source, which is equivalent here because a source
  lies on every path it originates).
- **Trunk identities**: every edge's stored path is connected under
  the declared move set with every step admissible **including the
  straddle rule on diagonal and Knight submoves**; recomputed
  undiscounted direction costs along the stored path equal the
  stored costs (exact f32, same code path); endpoints equal the
  declared node set (recomputed from disk — saddle sites from the
  suitability summary, heads from the predicate raster, mouths from
  receivers/strahler); `trunk.vrast` equals the traversal counts
  derived from the stored paths; junction records equal the union-
  graph recomputation.
- **Preconditions**: config `validate()` (finiteness,
  `deny_unknown_fields`, declared domains, and **physical-scale
  bounds** — every cost-bearing knob within [1e-6, 1e6], not merely
  positive, so that no move's cost can vanish against an accumulated
  distance in f64 addition and break the strict ordering the solver
  and the density accumulation rest on), the directed-pair-count u32
  guard at lattice resolution, receiver range validation, and
  input-artifact validity all run before any artifact is written or
  gate evaluated; corrupt inputs fail loudly.

QA diagnostics (D10 visual channel, never in CI or the summary):
the boundary-bias halo on density; time-optimal vs energy-optimal
trunk divergence; density/trunk overlay against the affordance
panel; the demoted Bellman fixpoint check with ulp tolerance.

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
  every water-derived number, with the placeholder's degeneracy
  declared in Decision 3.

## Consequences

- Research 0014 joins the corpus as the corridor dossier; its
  retrieval targets (Filet & Rossi; Page 2026; Duncan-Jones p. 368;
  Llobera et al. 2011 full text) are the checklist for upgrading
  declared forcing to citation.
- The movement-cost deferral from ADR 0011 is discharged: L&S
  gradients, Herzog refit, and the Pandolf/Santee load extension are
  frozen with their primaries; the multi-rooted-sextic finding is on
  record in 0014.
- **The k_Q calibration experiment is promoted from "recorded idea"
  to prerequisite**: no corridor-derived number is evidence before a
  declared reference k_Q exists (Decision 3).
- `via-corridors` enters the workspace as a stage crate (serde +
  via-artifact only; via-viz gains a corridor panel; the CLI a
  `corridors` subcommand over an existing run dir).
- The stage introduces the workspace's first weighted-graph solver
  (16-neighbour land / 8-neighbour water, node-split,
  predecessor-tracked, straddle-checked); via-suitability's private
  distance Dijkstra is unrelated and stays private.
- Deferred by this ADR, recorded: A/B-moves (7×7 neighbourhood) if QA
  shows grid artifacts; MADO accumulations from affordance sites as
  an additional QA/render channel; circuit-theory redundancy audit
  (isotropic, symmetrized) as a possible future diagnostic; the
  Helbing self-consistent-field solve as a spike candidate for
  emergent bundling; wind climatology for sea legs; inter-landmass
  crossings as a declared open-water mode.
- Runtime honesty: the definition gates double the solve cost by
  design; the reference grid pays minutes, not hours, and the
  scaling law plus the size-relative lattice default are stated in
  Decision 4 — never silent sampling.

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
  gateway value emerges in traffic; and the endpoint count (461 on
  the reference run) would swamp K with river-parallel land edges.
- **`.vgeo` for corridor routes.** Wrong beneficiary of the deferred
  schema obligation (ADR 0011 precedent); cell-path records suffice.
- **A min-projected single hours raster.** Mode mixing across the
  transshipment seam makes every per-cell consistency statement
  ill-posed; per-node-class rasters cost one file and keep the
  artifact checkable.
- **An inter-landmass connectivity patch.** Forcing a trunk edge
  across open ocean would fabricate a mode the era doesn't have and
  brick the trunk-identity gate; partition honestly instead.
- **A hard-coded haul limit, walking-city radius, or era style.**
  0008 refusals; all such figures must emerge or remain config.
- **8-neighbour-only land movement.** Herzog's 20% worst-case route
  displacement is disqualifying for a stage whose output *is*
  routes.

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
