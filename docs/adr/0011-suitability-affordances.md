# ADR 0011 — Suitability affordances: tier, representation, and gates

Status: proposed (2026-08-21; amended the same day after the
implementation chunk's adversarial review — Finnegan α provenance,
Langbein's f, stability-band placement, depth-window ramp, D6 clause
enumeration, recorded deferrals)
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
  big are rivers in this world" knob; it is unverifiable from inside
  via, and everything metric downstream of it inherits that.
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
