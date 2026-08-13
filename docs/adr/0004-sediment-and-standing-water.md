# ADR 0004 — Sediment and standing water (M3)

Status: accepted (2026-08-13); numerics amended by ADR 0006 (the
explicit deposition sweep and its dt limit and donor-floor cap are
replaced by the implicit Gauss–Seidel fixed point — the physics
contract here is unchanged)
Scope: replaces the two known fakes in the terrain substrate — ε-fill
plains (detachment-limited SPL never deposits) and the absence of lakes
(depressions were erased into flat fills). After M3 the surface is
h = bedrock + sediment, depressions hold water, and sediment mass is
conserved to numerical precision.

## Decision 1 — Erosion–deposition via the G-coefficient form

Model: Davy & Lague (2009) ξ–q erosion–deposition, in the discrete
G-coefficient form of Yuan et al. (2019):

    ∂h/∂t = U − K·√Q·S + G · q_s / Q̃

where q_s is the incoming sediment flux and Q̃ the discharge. Steady state
with uniform U gives S = U(1+G)/(K√Q) on channels: channel slopes scale by
(1+G) and concavity θ is untouched, so the gate bands survive. **K keeps
its detachment-limited (M2) calibration**: deposition acts on the fluvial
domain only, so hillslopes — which set most of the relief — keep their M2
steady slopes, while channels carry the (1+G) steepening. Rescaling K
globally for (1+G) was tried and rejected: it halves hillslope slopes and
the island's relief fell to a third.

Discrete scheme per step, after the implicit detachment solve:

1. The implicit Braun–Willett pass runs as before (unconditionally stable)
   and yields the *detached* volume per cell.
2. A single sweep in upstream→downstream (reverse stack) order routes
   sediment flux Φ down the receiver tree. At each subaerial cell,
   deposition d·dt = min(Φ/A_cell, G·Φ/(A_cell·Q̃)·dt-normalized) — i.e.
   deposit the G-fraction, capped by what arrived. Deposition is further
   capped so a cell never aggrades to or above its lowest donor
   (min-donor − ε): aggradation cannot invert the local receiver graph
   within a step. Physical dams therefore only form through lake filling,
   not through single-step spikes; the cap is a stability bound, recorded
   here as a deliberate M3 simplification.
3. Remaining flux continues to the receiver. Flux reaching base level
   (ocean) either progrades the coast (Decision 2) or is counted as export.

**Deposition acts on the fluvial domain only** (discharge ≥ the same
threshold the slope–area gate uses for channel membership). The ξ–q model
is channel physics: on Q̃ ≈ 1 hillslope cells the G/Q̃ fraction saturates
at 1, every hillslope cell becomes a perfect trap, sediment creeps
downhill one cell per step and the landscape smears into a smooth dome —
observed directly at island8k (valleys buried, network statistics
destroyed, 13 m mean sediment blanket). Hillslope transport is the
diffusion term's job (the standard channel/hillslope process
decomposition); hillslope cells pass flux through. Flooded cells trap
regardless of discharge.

Single erodibility K for both bedrock and sediment in M3. The K_sed > K_br
contrast of the full Davy–Lague model is deferred to M4 (lithology), where
per-material erodibility becomes a first-class field anyway — together
with the Yuan et al. (2019) *implicit* erosion–deposition solve
(Gauss–Seidel over the stack), which removes the explicit-deposition dt
limit recorded below and is the right vehicle for K contrast.

## Decision 2 — Standing water: route on the filled surface, evolve the true one

Priority-flood+ε now fills a **routing copy** h_route; the true surface h
keeps its depressions. Semantics:

- **Receivers, stack, discharge** are computed on h_route: water crosses
  lake surfaces to their spillways (the grid-scale equivalent of
  Cordonnier et al. 2019 depression routing; revisit that algorithm only
  if the fill copy becomes a performance problem).
- **Shallow ponding is sub-grid noise, not lakes**: after each flood, any
  connected ponded component whose *maximum* depth is below 0.5 m merges
  back into the true surface (h adopts h_route there) — these are
  transient pits recut each step by erosion and diffusion, and within a
  multi-century step such ponding sediments instantly in reality. Left
  standing, every centimetre pit stops incision and traps flux, and the
  landscape smears (observed). This is the M2 ε-fill retained at the
  scale where it was always honest; only deep depressions persist as
  lakes.
- **Flooded cells** (h_route − h > 0): no fluvial incision (they are under
  water; the implicit solve would otherwise *raise* them toward their
  across-lake receiver, which is deposition by the wrong mechanism) and no
  incision means bedrock under lakes is preserved. Hillslope diffusion
  still acts (subaqueous creep).
- **Rivers grade to the water surface**: the detachment solve's effective
  receiver height is max(h_route[rcv], sea_level) — a stream entering a
  lake grades to the lake level, and at the coast to sea level, never to
  the submerged bottom.
- **Lakes trap sediment**: flux entering a flooded cell deposits up to the
  water level (delta progradation — the first flooded cell along the path
  aggrades to level and becomes land, then the next). Remaining flux
  continues along the routed path.
- **The ocean exports**: flux reaching base level is counted as export,
  not deposited. At steady state the landmass exports its entire uplift
  flux — parking that at the mouth without subaqueous dispersal (repose
  avalanching, wave reworking) would prograde a one-cell-wide pier of
  land indefinitely, a worse fake than no delta. Coastal deltas arrive
  with M6 marine transport; M3 deltas are lacustrine (lakes trap, fill,
  and turn into depositional plains — including the enclosed sub-sea
  pockets that used to be ε-fill fakes, which now flood as real lakes
  and fill with real sediment).

## Decision 3 — Two surfaces: h = bedrock + sediment

`sediment_m ≥ 0` is tracked per cell; `bedrock = h − sediment_m`.
Bookkeeping rules: uplift moves both surfaces together; fluvial detachment
removes sediment first, then bedrock; deposition adds sediment; diffusion
that lowers a cell takes sediment first then bedrock, diffusion that
raises a cell adds sediment (hillslope flux *is* colluvium). This is
bookkeeping for mass and material — sediment has no distinct mechanical
behavior in M3 (see Decision 1).

### Operator-splitting limit (extends ADR 0002 correction 2)

The deposition pass is explicit within the step: at steady state it moves
G·U·dt per channel cell against (1+G)·U·dt of implicit detachment. When
G·U·dt is no longer small against valley relief the erode→deposit
splitting oscillates and the oscillation feeds back through the flux —
observed as *growing* max|Δh| and runaway peaks at the old research
dt = 1e5 yr (G·U·dt = 50 m/step). The research preset moved to
dt = 1e4 yr / 3000 steps; island8k's dt = 2e3 yr (1 m/step) was already
inside the limit. The convergence trace and the residual gate are the
tripwires.

## Decision 4 — Gates

- **Sediment mass closure (core)**: over the whole run,
  |Σdetached − Σdeposited − Σexported| / Σdetached ≤ 1e-9. Pure
  consistency, but it is the tripwire for every bookkeeping bug in the
  routing sweep.
- **Slope–area θ excludes flooded cells only.** Under the single-K G-model
  every steady channel obeys S = U(1+G)/(K√Q) regardless of sediment
  cover (channel cells carry an in-transit layer of order G·U·dt every
  step — ~1 m at island8k's dt, ~50 m at the research dt), so excluding
  sediment-covered channels would measure the model wrong. Flooded cells
  stay out: lake-bed slopes are not stream-power statements. The
  bedrock/alluvial regression split becomes physical when M4 introduces
  the K contrast. "Persistent floodplain" for the advisory gate means
  sediment well above the in-transit layer (2·G·U_max·dt, floored at
  0.5 m).
- **Lake consistency (core)**: every lake's water level equals its spill
  elevation on the routed surface (within ε-fill tolerance) — by
  construction, so a violation means the extraction is wrong.
- **Floodplain slope (advisory)**: median slope of high-sediment cells
  *within the fluvial domain* should sit below the median channel slope.
  The fluvial-domain restriction matters: colluvial hillslope piles (the
  diffusion bookkeeping) also exceed the thickness threshold and would
  dominate the median at coarse dt.
- **Pond-merge metering (report field, not a gate)**: the shallow-pond
  merge is a declared mass source; its cumulative volume is reported as
  `pond_merge_m3` so the fake stays quantified against the fluvial
  budget rather than silent.
- Existing gates unchanged; pits gate still applies to the routed surface.

## Decision 5 — Artifacts and contracts

New f32 rasters: `sediment_m`, `water_depth_m` (h_route − h; 0 where dry).
`fill_depth_m` is superseded by `water_depth_m` and removed (M3 is
uncommitted work; no migration needed).

**Water contract (updates ADR 0002)**: ocean = self-receiver, as before.
Standing fresh water ships as the `water_depth_m` **spectrum** — lake
cells route to their spillway and are not self-receivers, so `receivers`
alone no longer identifies all water. Each consumer thresholds the
spectrum with constants declared in its own config (per ADR 0003:
spectra over taxonomy; the discretization belongs to the interpreter).
Reference thresholds: physics treats a cell as flooded above 1 mm
(FLOOD_EPS, an internal numerical guard against ε-fill noise);
via-ecology classifies ≥ 0.3 m as `lake` (a measurement, not authored
taxonomy) and 0.05–0.3 m ponding or high-TWI wet cells as `wetland`;
via-suitability excludes ponded cells from sites and counts lakes as
freshwater sources.
