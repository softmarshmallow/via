# ADR 0011 — Suitability affordances: tier, representation, and gates

Status: accepted (2026-08-22, under the user's delegated authority —
"you know what to do"; proposed 2026-08-21 and amended the same day
after the implementation chunk's adversarial review — Finnegan α
provenance, Langbein's f, stability-band placement, depth-window
ramp, D6 clause enumeration, recorded deferrals; amended again
2026-08-22 on research 0015 — k_Q derived from the terrain's own
hydrology with C = 0.35 the single cited coefficient, the ford chain
split into a channel-forming and a crossing flow, and Langbein's f
closed as a digitized curve with an enforced validity domain)
Scope: the stage decisions ADR 0009 left open for `via-suitability` —
what epistemic tier the stage occupies, how the named affordances
(harbours, fords, confluences, passes) are represented, and what its
gates may check — fixed before any affordance code is written.
Detection schemes are adopted from research 0013, the dossier
commissioned for this decision.

## Context

The causal chain's first humanity level is "suitability &
affordances"; ADR 0009 D1 assigns it to `via-suitability` with unit
of work = cell, and records that the settlement spike failed partly
because the named affordances were never implemented — its towns sat
on generic good farmland instead of at crossings, confluences and
harbours. What exists today is the generic first slice: four derived
fields, config-thresholded patches, and a boolean "gate" that reports
whether any patch exists.

Extending it forces three questions the record leaves open:

1. **Tier.** ADR 0003 names "suitability scoring" as a Tier-3
   example and says gates exist for Tier 1 "and only this tier" —
   while ADR 0008 D1 defines gates as internal-consistency
   instruments with scope "every stage", and ADR 0009 D4 says "stage
   crates keep their gates". The documents never state whether a
   Tier-3 stage may carry D1-style gates.
2. **Representation.** No document assigns affordances an artifact
   form. The signals cut both ways: the cell unit of work and the
   spectra-over-taxonomy corollary point raster; named point features
   resemble the geometry case ADR 0009 D3 built `.vgeo` for — but D3
   defers the `.vgeo` per-feature schema to "before the first crate
   emits a .vgeo", so the choice decides who inherits that
   obligation.
3. **Validation.** Research 0012 defines no suitability or corridor
   character at any tier — there is no benchmark channel for this
   stage's outputs. What may the stage claim, and what checks it?

A corpus sweep also found that no dossier records detection criteria
for any of the four affordances; research 0013 now supplies the
citable routes, each constrained to the shipped terrain artifacts.
Finally, research 0011's header rule binds: an adopting ADR must
resolve the bearing open questions or declare them as limitations
(Decision 7).

## Decision 1 — via-suitability is a Tier-3 interpretation stage

The stage derives description over tier-1/2 fields; it claims no
mechanism and solves no equations. ADR 0003 already names
"suitability scoring" as the canonical Tier-3 example, and this ADR
confirms that reading for the affordance extension:

- **Every output is labeled** `standard` (published scheme, cited and
  fixed) or `heuristic` (project invention, named as such) in the
  stage's docs and JSON summary — the labeling duty via-ecology
  already discharges and the current suitability.json omits
  (implementation debt, Consequences below).
- **Being consumed by future Tier-1 mechanisms does not raise the
  tier.** The doctrine screen has suitability replacing fiat inputs
  inside process mechanisms (Harris–Wilson O_i, street-growth
  potentials); a description's epistemic status is set by how it is
  derived, not by who reads it.
- **Every threshold and coefficient is config-declared** with
  provenance (the climate.rs undeclared-forcing precedent). One
  parameter is singled out as Tier-2 forcing proper: `k_Q`, the
  single relative-to-absolute discharge scale (m³/s per discharge
  unit) — the lumped-calibration practice of the stream-power
  literature (Whipple & Tucker 1999, via 0013). It is the one "how
  big are rivers in this world" knob, and everything metric
  downstream of it inherits its status.

  **Amended 2026-08-22 (research 0015): k_Q is derived, not free.**
  The terrain stage weights flow accumulation by
  `max(precip/precip_mean_m_per_yr, 0.05)`, so `discharge` counts
  mean-precipitation-equivalent upslope cells. One unit therefore
  carries one cell's area of the land-mean annual precipitation, and

      k_Q = A_cell × P_mean × C / seconds_per_year

  where every term but **C, the runoff ratio**, is already declared in
  the terrain config. C is adopted as **0.35**, cited to Dai &
  Trenberth (2002) *J. Hydrometeorology* 3(6):660–687, who state it as
  a ratio ("~35% of terrestrial precipitation"); the published spread
  across fourteen global water budgets is 0.33–0.42, clustering there.
  The pairing is exact rather than approximate: 0.35 is a
  *precipitation-weighted* mean of local runoff ratio, and via's
  accumulation is precipitation-weighted by the same construction, so
  Q = Σ_i (A_cell·P_i·C)/T is mass-consistent by inspection.

  This demotes k_Q from an unverifiable knob to **one cited
  coefficient plus one declared mapping**, and the mapping is the part
  that remains Tier-2: the claim that a generated dimensionless
  climate lands on absolute mm/yr and m² is a declaration, not a
  derivation, as is the assumption that the generated world's
  precipitation distribution is Earth-like enough for Earth's global
  statistic to transfer. Two consequences are declared with it: a
  uniform C runs arid basins wet and hyper-humid basins dry (true
  local C spans ~0.8 to ~0.04 across the Budyko curve), and it forces
  Q ∝ A exactly — which is the *correct* exponent for mean annual flow
  (USGS regional regressions give b = 0.96–1.02 across six humid
  states), so any sub-linearity via shows comes from its own
  orographic gradients. The 0.05 rain-shadow floor is recorded as a
  small positive bias (measured: 0.04% on the reference run). An
  explicit k_Q override stays available for worlds that want a
  different scale, and remains declared forcing when used.

  Per-cell climate-derived C (Budyko) is **deferred, with reasons on
  record in 0015 §1**: it would require via to invent a PET field, and
  every temperature-only route either needs a latitude the world does
  not declare (Oudin), misbehaves on isothermal per-cell means
  (Thornthwaite), or is a declared convention rather than a citation
  (Hamon with a fixed day length) — replacing one declared number with
  four, evaluated at 200 m cells against a framework its authors
  verify only above 1,000 km². The recorded upgrade path is Zhang,
  Dawes & Walker's reduced form, which needs precipitation and a
  forest fraction and no PET, and which belongs inside the terrain
  stage's `weights_of` — where it changes the stream-power field and
  therefore the landscape, so it is a gated change, never a
  display-layer conversion.
- The stage remains optional and ignorable by a research consumer;
  patch/affordance semantics continue to live in the experiment
  config that names them, not in the crate.

## Decision 2 — What a gate means here

The recorded tension resolves by reading the two documents as
governing different instruments:

- ADR 0003's "Gates exist for this tier [Tier 1] and only this
  tier" governs **statistical gates** — checks whose failure impeaches a claimed
  mechanism or its calibration. A Tier-3 stage claims no mechanism,
  so it can never carry one. This rule stands unchanged.
- ADR 0008 D1's internal-consistency checks — bitwise determinism,
  grid agreement, definitional invariants — are **artifact-contract
  checks**: they ask whether the stage computed what it defined, not
  whether the world is real. Every stage carries these (ADR 0009 D4:
  "stage crates keep their gates … deterministic and hashed, part of
  the artifact contract"). via-suitability carries only these
  (Decision 6 lists them).

Two reclassifications follow:

- The existing patches-nonempty boolean is **not a gate**. It checks
  no definition and no equation; its recorded role (island8k) is a
  seed-**selection criterion** under ADR 0003 corollary 4 — curation,
  which is legitimate and stays — and the output schema will say so
  instead of `gate` (Consequences).
- **No band-scored checks, ever, for this stage.** The θ/Hack/R_b
  arrangement is a recorded exception ADR 0008 calls the weakest form
  under the doctrine, not a template. With 0012 defining no
  affordance characters, there is no reference population to score
  against, and none will be invented.

The stage therefore **makes no realism claim**. Its outputs are
validated indirectly, where the doctrine already looks: downstream
settlement-system statistics (0012 Tier A) once settlements exist,
with the Kvamme-gain advisory (0009-gate-candidates) scoring emergent
placement against these very covariates. Any future *direct* claim
("via's harbours are where real harbours are") requires new
characters under ADR 0008 D2 and a measured reference population
under D3 — there is no shortcut through a band.

## Decision 3 — Representation: spectra in rasters, sites as ordered records

Affordances ship in the stage's existing dual form (the patch
precedent):

- **Spectra as `.vrast` rasters** (f32; the sealed dtype set):
  continuous per-cell fields — crossability, navigability, fetch,
  and the generic fields already shipped. Cell unit of work,
  spectra-over-taxonomy default.
- **Sites as JSON records** in the stage summary: where an adopted
  scheme yields discrete objects (saddles, confluences,
  head-of-navigation nodes), each site is a cell-anchored record
  carrying its measured spectra (persistence, symmetry ratio, …),
  written in a fixed, documented order — deterministic and hashed
  like everything else.
- **No `.vgeo` from this stage.** D3's justification for a vector
  format — osmnx/momepy must read the output — does not apply to
  affordance sites, and emitting one here would trigger the deferred
  schema-fixing obligation prematurely and for the wrong feature
  types. That obligation stays with morphology, as D3 intended.
- **No authored value.** Sites carry measured quantities only. Any
  settlement-desirability weighting of a confluence or harbour is
  declared config in the consuming stage, and affordance *value*
  arises mechanically there (mode-change penalties, corridor
  traffic) — the transshipment precedent in the doctrine screen. The
  one adopted intrinsic ranking is persistence, which is a measured
  landform property, not a desirability score.
- **Era-free.** Affordance spectra are physical; era enters
  downstream as corridor cost vectors and edge-type availability
  (0005). Nothing in via-suitability is era-indexed, which keeps the
  stage a single fixed description per world.

## Decision 4 — The affordance set and its adopted schemes

From research 0013, each with its label and its config surface
(defaults with provenance; all values live in config, none in
source):

- **Passes** — Morse-theoretic saddle extraction: Peucker–Douglas
  (1975) 8-ring operator, topological consistency per Takahashi et
  al. (1995), persistence attached by a height-sorted union-find
  sweep (Edelsbrunner et al. 2002; instantiation Kirmse & de
  Ferranti 2017). The sweep decides the site set; the ring operator
  is recorded per site as the Takahashi-consistency diagnostic (the
  raw operator is inconsistent on grids — that is the correction
  Takahashi supplies). Label: standard. Config: minimum persistence
  (default 30 m, Kirmse & de Ferranti), declared deterministic
  tie-break for equal integer-cm heights; domain is dry land (ocean
  and standing-water bathymetry excluded — a submerged col is not a
  land-movement pass). The basin-boundary-minima
  cross-check is a QA diagnostic, not a gate (Decision 6). The per-cell fuzzy "passness"
  field (Fisher–Wood–Cheng 2004) is admitted as standard but
  deferred until a consumer needs a field rather than sites.
  Geomorphons are rejected for this purpose — 0013's negative
  finding: the published 10-class reduction has no saddle class.
  Pass *value as a crossing* is left emergent in the corridor solve.
- **Fords** — the hydraulic chain: `k_Q` (Decision 1) → channel
  width per Finnegan et al. (2005), W ∝ Q^(3/8)·S^(−3/16)·n^(3/8)
  (width-to-depth ratio α is config: the paper fits α by substrate —
  5 bedrock to 59 gravel, its Fig. 1 — rather than recommending one
  value; via defaults 20, a declared choice near the cobble-bed 21) →
  depth
  and velocity per Manning (1891; n from the Chow 1959 tables; the
  lithology→n lookup is labeled heuristic) on reach-averaged slope →
  **crossability spectrum D·V** along river cells, still-water depth
  alone on standing water. Label: standard (each link cited; the
  chain's assembly is via's, stated as such). The people-stability
  bands (Cox, Shand & Blacka 2010; AIDR Guideline 7-3) are config
  constants with provenance in the consuming stage (Decision 5's
  contract carries the spectrum, not classes) — never baked, and this
  stage emits the continuous product alone.

  **Amended 2026-08-22 (research 0015): the chain carries two flows.**
  The sweep established that Finnegan *derives* his relation in
  bankfull terms (W and D are defined bank-full; Q = UA must fill that
  section) but *calibrates* it against mean annual discharge, so the
  fitted constant in his regression absorbs both the geometry factors
  and the bankfull-to-mean ratio. Using the closed α-and-n form to
  produce an absolute width — which is what via does — is therefore a
  mode the paper never tested, and feeding mean annual Q into it
  under-predicts. The chain is corrected to state its flows
  explicitly:
  - **Channel geometry** (width, and the depth and velocity that
    define the section) is computed at a **channel-forming discharge**,
    Q_bf = `bankfull_ratio` × Q_mean, with the ratio config-declared,
    default **3.0**. No published Qbf/Qmean exists; two independent
    derivations bracket it at 1.6–4.6 (pairing Wolman & Leopold 1957
    Table 1 against Leopold & Maddock 1953 Appendix A at four
    matching gauges: median 3.0; and a UK flow-duration curve: 3–6).
    Emmett (1975) USGS PP 870-A states mean annual ≈ 25% of bankfull
    for snowmelt Idaho basins, with his own caution that the ratio
    "cannot be used indiscriminately". Bankfull recurrence is 1–2 yr
    on the annual maximum series, "closer to 1 than 2" (Wolman &
    Leopold 1957), median 1.4–1.64 yr in temperate settings (Liu et
    al. 2026; Ahilan et al. 2013) — but Williams (1978) establishes
    that no common recurrence exists, so the ADR adopts the *ratio*
    and not a recurrence interval.
  - **Crossing conditions** (the depth and velocity a traveller
    actually meets, hence the emitted crossability spectrum) are
    computed at a **declared exceedance percentile of the
    flow-duration curve**, not at bankfull and not at mean annual.
    This is the settled convention of the low-water-crossing
    literature, where the passability flow is Q_e with e the
    acceptable percentage of the year the crossing is closed
    (Rossmiller et al. 1983; Ring 1987; Lohnes et al. 2001, Iowa's
    default Q₂% ≈ seven days a year). Config `ford_flow_fraction`
    expresses the chosen percentile as a fraction of mean annual
    flow, default **0.62** — the median-flow ratio for a UK-average
    river from the one-parameter flow-duration family of Gustard,
    Bullock & Dixon (1992), IH Report 108 Table 5.2. Mean annual flow
    is itself exceeded only ~25–30% of the time (Leopold & Maddock
    1953; Langbein 1962), so judging fords at it would model a
    wetter-than-typical world three-quarters of the year — the
    limitation this correction removes.

  Both factors are declared forcing with the provenance above, and
  both are restated wherever a metric number is reported. **The α
  caveat is recorded rather than resolved:** Finnegan's Figure 1 has
  no discharge axis at all, so none of his α values carries a stated
  flow convention; the gravel α = 59 is traceable to Leopold &
  Maddock's Appendix A, explicitly "at stage corresponding to mean
  annual discharge" (refitting that appendix gives 66.3 against the 59
  plotted), but the bedrock/boulder/cobble values 5/9/21 come from
  unattributed Cascades field surveys — and via's default α = 20 sits
  with those, not with gravel. This is the chain's largest
  unverifiable exposure, larger than the discharge convention itself,
  and is declared as such.
- **Confluences** — definitional on the inverted receivers tree: a
  strahler > 0 cell with ≥ 2 river donors (Strahler 1957; Shreve
  1966/1967 frame). Zero new parameters; the already-declared
  channelization threshold parameterizes the count and is restated
  with any reported number. Importance: the Benda et al. (2004)
  symmetry ratio (smaller/larger contributing drainage, exact from
  area_cells or as a discharge ratio where relative units cancel) —
  a standard measured spectrum, kept continuous.
- **Harbours** — three component spectra, shipped separately: wave
  fetch index F (Burrows et al. 2008; config: 16 sectors and the
  200 km cap with the paper's provenance, its smoothing and
  hierarchical search; standard, with the coastal-water focal-cell
  adaptation declared — a higher sector count for narrow mouths at
  16 m cells would be a declared adaptation, as the paper uses 16),
  a depth-window ramp (config anchors: dead at ~1 m of water column
  rising to saturation at the large-ship draught ~4.5 m — Boetto
  2010, Salomon et al. 2016; the two-knot linear ramp joining them is
  a declared interpolation, with the ordinary merchantman band
  ~1–3.5 m lying on the rising limb), and a sediment-supply
  penalty near river outlets ranked on
  relative discharge (heuristic; motivated by Marriner & Morhange
  2007). **No baked composite**: no published harbour index exists,
  so any combination is declared config in the consumer. The
  wind-weighted exposure family is recorded as the upgrade path if
  via ever grows a wind climatology.
- **Navigability and head of navigation** — the water-mode
  admissibility the corridor stage needs: Langbein (1962) specific
  tractive force as the continuous spectrum (published anchor
  Ts > 0.002 unnavigable; the formula's constants are
  imperial-unit-bearing — 0013 carries the conversion note; Langbein's
  f is the shallow-water vessel-resistance ratio of his Fig. 8 at the
  paper's draft = 0.7·D convention, shipped as a declared config
  constant flagged pending exact digitization — it is not a
  bed-friction factor, per the dossier's correction note), the
  Magirl & Olsen (2009) slope bands applied as-is (dimensionless),
  depth via the `k_Q` chain against the pre-modern anchor (Eckoldt
  0.3–0.7 m via Appel et al. 2024). Head-of-navigation sites are
  compositional: the most upstream cells of the navigable set
  continuously connected downstream to the river mouth — an
  isolated navigable pocket above an unnavigable reach is not a
  head of navigation. The Filet et al.
  (2025) change-point instrument — the one calibration-free method
  0013 found — is adopted as a consistency cross-check, not a second
  authority.

  **Amended 2026-08-22 (research 0015): the f flag is closed, and the
  criterion gains a validity domain.** Figure 8 has been digitized and
  validated end-to-end against the paper's own Figure 11 (two named
  cases reproduce Ts to ~15%, inside a 1962 log-scale figure's reading
  precision). At Langbein's draft convention d/D = 0.7, f is 1.21,
  2.48, 4.71 and ≈7.8 at F = 0.25, 0.50, 0.75 and 0.90, where
  F = V/√(gD) — so f **stops being a declared constant** and becomes a
  lookup on the primary's own curve, interpolated per cell from the
  emitted depth and velocity. The shipped constant 2.5 corresponded to
  a fast vessel; typical navigable rivers sit at F ≈ 0.17, hence
  f ≈ 1.1–1.4.

  Figure 8 stops at **F = 0.90**, and that bound is now enforced
  rather than extrapolated: a reach whose Froude number exceeds the
  figure's domain is **declared unnavigable by domain**, with the
  reason recorded, instead of being assigned an extrapolated f. This
  is not a corner case for via — measured on the reference world the
  median channel Froude number is 1.25 (supercritical), which is
  structural rather than a k_Q artifact (F ∝ Q^(1/16)) and follows
  from steepness: the Manning algebra puts the transition near S ≈ 3%
  and the reference island's channels have median slope 5.4%, 73%
  steeper than that. The consequence is that these worlds correctly
  report no navigable water — they are mountain torrents — and that
  exercising the water modes needs a larger, flatter domain with
  lowland reaches. Langbein's own reading corrections are recorded for
  a future refinement: his criterion belongs at the **shallow
  controlling section** (riffle or crossover), not the gaged mean
  section — a 13× swing on his Mississippi example — with channel
  depth ≈ 1.25× and channel velocity ≈ 1.15× the section means.

Movement-cost values (the Llobera & Sluckin critical gradients, the
Herzog refit coefficients, Santee's downhill correction) are **not**
frozen by this ADR; they belong to the corridor stage's cost
functions, whose adoption is that stage's ADR (Decision 5). Their
0013 pending-verification flags were closed against primary sources
on 2026-08-21 — see the correction notes in place there.

## Decision 5 — The suitability → corridors contract

`via-corridors` consumes, in the ADR 0009 D2 spirit (content fixed
here, field-level schema fixed with the implementation):

- the terrain artifacts directly (heights, receivers — suitability
  does not proxy terrain);
- the crossability spectrum (ford penalties along river cells);
- the navigability spectrum and its banded predicate (water-mode
  edge admissibility);
- the site sets with their spectra: confluences (symmetry ratio),
  saddles (persistence), head-of-navigation nodes;
- the harbour component spectra (transshipment-penalty inputs at the
  land–water interface);
- the generic fields (slope, freshwater_dist, and coast_dist — now
  exported; Consequences).

Movement-cost functions, mode sets, transshipment penalties and era
cost vectors are corridor config — Tier-2 forcing under ADR 0003's
"economic cost weights (later)" — surveyed in 0013 §Movement cost
but adopted by the corridors ADR, not here. Nothing this stage emits
is era-indexed.

## Decision 6 — The gates

All binary, all in CI, all D1 internal-consistency; unevaluable is
fail (ADR 0001 QA contract):

- **Determinism**: byte-identical artifacts and hashes on re-run;
  the stage stays sequential, and the stage summary's recorded
  hashes are the witness (the run manifest takes over once the
  multi-stage schema evolution recorded in Consequences lands).
- **Grid agreement**: every input and output raster matches
  heights_cm's geometry (the existing ensure_grid check, kept).
- **Confluence definition**: every emitted confluence site has ≥ 2
  river donors and no non-emitted river cell does — donor counts
  independently recomputed from the receivers artifact, river
  membership read from the emitted strahler artifact.
- **Spectrum identities**: on river cells the emitted crossability
  equals the f32 product of the emitted f32 depth and velocity
  rasters, and on standing-water cells it equals the emitted depth
  field — the stage computes the composite from the rounded factors
  so the identity is exact, which also pins the composite to the
  factor artifacts shipped beside it; on standing water the emitted
  velocity is zero and the emitted depth equals the terrain
  water-depth artifact; all four ford fields are zero outside the
  water domains; fetch values lie in [0, cap];
  head-of-navigation sites are exactly the cells of the
  mouth-connected navigable set that have no donor in that set,
  recomputed from the emitted predicate and the receivers artifact.
- **Site-record integrity**: every payload field an emitted site
  carries (coordinates, donor counts and drainage areas, symmetry
  ratios, strahler orders, depths, velocities, Ts) equals the value
  recomputed from the artifacts on disk — sites carry only
  recomputable measurements, so the whole record is checkable.
- **Artifact validity preconditions**: receiver values are validated
  against the grid and the config against its declared domains before
  any gate runs; a corrupt artifact or degenerate config fails
  loudly rather than panicking, hanging, or emitting NaN (unevaluable
  is fail).

No advisory checks at introduction; none of the above says anything
about realism, by construction (Decision 2). One check is
deliberately **not** a gate: the basin-boundary col cross-check for
saddles (0013). Via's basins are labeled by ocean outlet — coastal
adjacencies bottom out at the shoreline, not at cols — and D8
divides derive from the epsilon-filled routed surface, so they can
wander off heights_cm ridges across filled flats; col
correspondence is expected but not invariant, and a binary form
would fail legitimate maps. It ships as a QA diagnostic in the
visual-inspection channel (ADR 0008 D10), never in CI.

## Decision 7 — Dispatch of the bearing open questions (research 0011)

- **Pack/foot maximum grade** — resolved by citation, adoption
  deferred: the "maximum practical grade" is not an engineering
  constant but a metabolic phase transition (Llobera & Sluckin 2007
  switchback onset; critical gradients +0.28/−0.22 m/m,
  primary-confirmed 2026-08-21 per 0013's correction note), with
  federal trail-engineering standards (USFS
  1935; FSH 2309.18; Hancock et al. 2007) as declared-forcing
  corroboration — 0013 §Movement cost. The corridors ADR freezes
  the values.
- **Cross-era cost normalization** — not needed by this stage: its
  outputs are era-free (Decision 3). Deferred to the corridors ADR,
  which must resolve it or declare it.
- **Wells/springs → population-capacity link** — declared
  limitation: affordances here carry *access* semantics only.
  Nothing in this ADR claims carrying capacity, and no output may be
  read as one.

## Consequences

- Research 0013 joins the corpus as the affordance dossier; its
  pending-verification flags are the checklist for closing any value
  this ADR's config defaults cite.
- Implementation debts recorded against the existing slice, to be
  paid in the affordance implementation chunk: per-output
  standard/heuristic labels in suitability.json (via-ecology's
  precedent); the `gate:` field renamed to selection-criterion
  vocabulary (Decision 2); coast_dist exported as an artifact
  (computed today, dropped); output filenames namespaced per config
  label so multiple affordance configs can share a run directory;
  suitability outputs registered in the run manifest — which entails
  a manifest schema evolution, since RunManifest is single-stage
  today (one `stage`, one `config`); until it lands, the stage's own
  hashed summary is the determinism witness (Decision 6).
- Deferrals recorded by the implementation chunk, to be paid in its
  QA/visualization slice: the Filet et al. (2025) change-point
  consistency cross-check and the basin-boundary col diagnostic (both
  QA-channel items under ADR 0008 D10; neither is a gate). The
  per-lithology Manning n lookup remains a recorded extension
  (Decision 4 labels it heuristic). Fetch rays are unit-step
  point-sampled — a declared substitution for the paper's
  three-scale hierarchical search. Artifact hashes witness
  same-platform reruns only (libm transcendentals), the same scope
  as the terrain stage's snapshot channel.
- The corridor stage inherits a written consumption contract
  (Decision 5) and two deferred adoptions (cost functions; cross-era
  normalization) with their literature already surveyed.
- The one absolute scale in the humanity side so far is `k_Q`; every
  metric affordance number downstream of it is conditional on a
  declared, unverifiable forcing constant, and reports quoting such
  numbers restate that.

## Rejected

- **Tier-1 status for affordance derivation.** No equations are
  claimed, so a statistical gate would have nothing to check; and
  with 0012 defining no affordance characters, a realism claim would
  owe a benchmark apparatus it cannot pay. The tier-1 slot on the
  humanity side stays reserved for the settlement economics (ADR
  0003's goals audit).
- **`.vgeo` for affordance sites.** Wrong beneficiary of the
  deferred schema obligation; no external tool needs to read these.
- **Geomorphons as the pass detector.** The published reduction has
  no saddle class; using the raw patterns would smuggle in a
  heuristic under a standard's name (0013 negative finding).
- **A baked composite harbour score.** No published index exists;
  components ship separately and the combination is declared
  downstream config — no invented composite under a stage's name.
- **Authored affordance labels or weights.** Value arises
  mechanically downstream (doctrine screen's transshipment
  precedent); the stage emits measured properties only.
- **Wind-weighted exposure now.** No wind field exists; every
  wind-weighted index degenerates to the fetch index under a uniform
  rose, so shipping F is the honest form and the family is the
  recorded upgrade path.

## References

- Research 0013 (affordance detection and movement cost — the
  adopted schemes' citations live there); research 0005 (corridor
  cost structure), 0008 (doctrine screen), 0009 (gate candidates),
  0011 (open questions).
- ADR 0001 (artifact format, QA contract); ADR 0003 (epistemic
  tiers); ADR 0008 (validation doctrine); ADR 0009 (stage
  architecture); ADR 0010 (benchmark instrument).
- experiments/island8k/README.md (selection-by-measurement record).
