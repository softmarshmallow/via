# ADR 0013 — Settlement: the allocation engine, its budget, and what it may claim

Status: **proposed** (2026-08-22). Written under the user's delegated
authority for the spike branch, but held at *proposed* rather than
accepted because ADR 0011 and ADR 0012 were each ratified only after
a multi-track adversarial design review, and no such review has been
run against this text. The review is the ratification precondition,
not the delegation. Amended the same day after a self-check of the
ADR's cross-references against the corpus: research 0006 added to
scope, the founding/survival separation stated (survival is the
engine's `δ/κ` floor, not a second mechanism), realized spacing added
as a character with Bracton's 6⅔ miles held as forcing rather than a
target, the Black Death market-thinning added as a sign-and-shape
expectation, and the ADR 0008 D9 attribution tightened. Amended again
after checking the upstream cross-references: only ADR 0012 D1
deferred anything to this stage, and what it deferred is validation,
not a realism claim — the earlier draft's "ADR 0011 D2 and ADR 0012
D1 both defer their realism claim" was wrong on both counts.

Scope: the stage decisions for `via-settlement` (ADR 0009 D1: unit of
work "settlement, corridor link") — which allocation engine, what
supplies its population budget, where settlements may sit, how they
are cut out for comparison against GHS-UCDB, the shock and epoch
structure, the artifacts, the gates, and — the part this stage owes
most carefully — an explicit statement of what it is *not* entitled
to claim. Mechanisms are adopted from research 0006 (founding and
planning), 0012 (benchmark specification), and 0016 (the dossier
commissioned for this decision).

**Research 0001 (settlement systems), 0004 (land-use economics) and
0005 (transport eras) are deliberately *not* drawn on here**, though
an earlier draft of this scope line claimed they were. Their
mechanisms — von Thünen hinterland rings, freight catchments with the
Masschaele price vectors, Louf's `R = B − C` — all require
settlements that already have sizes and markets, so they activate in
the demand-driven-growth chunk that follows this one, not at
introduction. Listing them here would have been a decorative
citation.

## Context

This is the stage the project's thesis actually rests on. Terrain,
suitability and corridors are all upstream instruments, and **exactly
one of them has deferred anything to here.** ADR 0012 D1 states it
outright: "Validation is indirect and downstream: settlement-system
statistics once settlements exist." ADR 0011 is a different case and
should not be described as a deferral — a Tier-3 stage that "claims
no mechanism and solves no equations" never had a realism claim to
postpone. So one debt falls due here, not two, and it is a *validation*
debt rather than a realism one.

That distinction matters for what follows: if this stage's
settlement-system statistics come out badly, they impeach the
corridor model, which asked to be judged this way. They say nothing
either way about suitability, which never made a claim to impeach.

It is also the first humanity stage with an external benchmark
channel: research 0012 defines settlement-system characters (its
Tier A), and GHS-UCDB exists to be compared against.

Research 0016 returns four findings that constrain this ADR before it
starts, and three of them are negative:

1. **Harris–Wilson cannot supply its own budget.** Three independent
   primaries state the same balancing condition — Osawa, Akamatsu &
   Takayama (2017) Eq. 6, Ellam et al. (2018) Eq. 25, and Zachos,
   Girolami & Damoulas (2024). Osawa's gloss is decisive: "O and κ
   change only the scale of h". The total settled mass is `ΣO_i/κ`,
   exogenous by construction. There is no endogenous option inside
   this model family.
2. **Single-good Harris–Wilson on homogeneous space yields
   equal-sized centres** (Osawa, concluding remarks). Every unit of
   size dispersion via produces comes from via's own terrain, its
   `c_ij`, and its `O_i` — not from the engine.
3. **No published mechanism derives a Zipf exponent from spatial
   interaction alone.** Wilson (2008) states it as an open
   problem in his own words — "a mathematical challenge: to find a
   way of explicitly connecting the {Z_j} size distributions that
   arise in the BLV models to the statistical distributions used as
   measures of network structure in the scale-free literature".
   Nothing since closes it. The rank-size gate is therefore
   non-circular, but the engine is not entitled to be graded on
   passing it. The only candidates that produce a power law
   non-circularly — Hsu (2012), Mori, Akamatsu, Takayama & Osawa
   (2023) — derive it from heterogeneity in scale economies **across
   goods**, and neither predicts the exponent; it inherits the tail
   index of an assumed input distribution. That is the only route to
   an endogenous hierarchy, and it is a multi-good extension this
   stage does not adopt.
4. **No standard battery exists** for settlement-pattern validation;
   via's frozen protocol would be ahead of surveyed practice.

And one finding arrived after the dossier's first draft, from sources
re-read directly on 2026-08-22 (0016 §6, "Refilled"): **the sign of
pre-industrial urban natural increase is contested, and the
disagreement is itself the mechanism.** Jedwab & Vollrath (2019)
measure +2.0 per 1,000/yr for the largest cities before 1800; Jedwab,
Johnson & Koyama (WP p. 23) call it "typically negative … until the
19th century" while citing that very paper. Jedwab & Vollrath's
footnote 9 (p. 233) reconciles them: their panel covers "normal"
periods only, and "during the Black Death, cities had death rates of
250–750". There is no single rate to force.

## Decision 1 — The tier ruling is per-component, not per-stage

ADR 0011 ruled its whole stage Tier-3 and ADR 0012 ruled its whole
stage Tier-3 over declared Tier-2 forcing. Neither shape fits here,
and forcing one would misdescribe the stage. The ruling is split:

- **The allocation dynamic is Tier-1 process.** Harris–Wilson with
  the BLV update is a falsifiable mechanism with a published
  equilibrium structure and a conservation law that can be checked
  numerically. It may carry statistical gates (ADR 0003's
  "gates measure tier-1 statistics only" is satisfied).
- **The budget, and every parameter that scales it, is Tier-2
  forcing** — `ΣO_i`, κ, δ, ε, α, β, the shock magnitudes and the
  epoch schedule. Decision 3 states why this is not a shortcut.
- **Delineation and the derived role scalars are Tier-3
  interpretation**, carrying `standard | heuristic` labels exactly as
  ADR 0011 requires.

This makes `via-settlement` the first humanity stage able to hold a
statistical gate, and the first obliged to say, in the same breath,
which of its numbers are forced.

## Decision 2 — The engine: singly-constrained Harris–Wilson with a BLV update

Adopted, in the form the three primaries agree on:

    T_ij = A_i · O_i · W_j^α · exp(−β c_ij)
    A_i  = 1 / Σ_k W_k^α · exp(−β c_ik)
    D_j  = Σ_i T_ij
    dW_j/dt = ε · (D_j − κ W_j + δ)

`c_ij` is the corridor time in hours (ADR 0012 D5's edge records and
`hours_to_trunk`), never re-derived here — the settlement stage does
not reimplement the movement model. `O_i` is the origin mass at node
`i` (Decision 4). `α > 1` is the returns-to-scale term, `β` the
distance-decay.

**`δ > 0` is required, not optional.** Research 0016 records both
reasons: at `δ = 0` the Gibbs measure is unnormalisable, and dead
zones become absorbing states. The minimum settlement size is `δ/κ`,
and it is a derived quantity, not a separate threshold to tune.

**IPF/Furness balancing is not needed** for a singly-constrained
model — `A_i` closes in one pass. It is recorded here only because
the corpus mentions it, and adopting it would be cargo-culting a
doubly-constrained model's machinery.

**Integrator discipline, and a corpus misreading corrected.** The
continuous Harris–Wilson dynamic is a gradient flow in `x = ln W`
(Ellam et al.) and therefore **cannot be chaotic**. Any chaos
observed in via would be a discretisation artefact, and is treated as
a bug rather than a finding.

The bound comes from the scalar reduction `ΔZ_j = ε(D_j − Z_j)Z_j`,
which is the logistic map with `r = 1 + ε D_j` — **the mapping is
via's, not a published result**, and is declared as such. Using May
(1976) Table I (period-2 at `a = 3.0`, chaos at `a_c = 3.5700`), the
stability condition is **`ε · D_j < 2` for the fixed point and
`< 2.57` for chaos onset**. `ε · max_j D_j < 2` is asserted at
runtime. Published settings for a defensible default: `ε = 1` with
`dt = 0.01` under Euler–Maruyama (Zachos et al.), or `ε = 0.01`
"so that the model does not converge too rapidly" with convergence at
`Σ(D_j − W_j)² < 1e-5` and a 10,000-iteration cap (Peeples &
Brughmans).

Osawa's "period-doubling" is **spatial and in parameter space, not
temporal** — the corpus previously misread it, and the misreading is
corrected on the record here.

## Decision 3 — The budget is declared Tier-2 forcing, and its magnitude has no published series behind it

Given Decision 2's conservation law, `Σκ_j W_j = ΣO_i + δM` fixes the
total; the engine distributes it and nothing more. The budget is
therefore declared forcing. That much is forced by the literature.

What must be said alongside it is that **the magnitude is not
currently backed by a published series.** Research 0016 §6's original
demographic table was fabricated by a research subagent and retracted
(2026-08-22); the direct re-run recovered the *structure* of the
constraint but not an aggregate growth series, which is in neither
re-read paper and needs the Maddison Project, McEvedy & Jones (1978),
or Broadberry et al. — none of them opened. **Until one is, the
budget's magnitude is a declared number with a stated absence behind
it, and no settlement-size claim may rest on its level.** Claims that
rest on the *pattern* — spacing, rank-size shape, attachment
structure — are unaffected, because the budget sets only the scale.

Four structural constraints on the budget's *shape* do now have
citations, and they bind:

- **Urban natural increase is a normal-period rate plus a shock
  process, never a single mean.** Jedwab & Vollrath (2019) Fig. 3,
  p. 232 (392 city-period observations, per 1,000): pre-1800s CBR
  38.1 / CDR 36.1 / CRNI 2.0; 1820s–50s 5.2; 1880s 5.9; 1900s 5.9.
  Their footnote 9, p. 233, supplies the shock arm. Forcing one mean
  would either erase the shocks or count them twice.
- **Growth of the urban *share* is a migration flow.** Jedwab &
  Vollrath p. 231 ("in-migration was the dominant source of new city
  dwellers") and p. 233 (nineteenth-century city growth "averaged 3
  percent per year … mostly occurred through in-migration"); Jedwab,
  Johnson & Koyama WP p. 22 independently attribute post-plague urban
  recovery to migration. In via this means the corridor-mediated `T_ij`
  flow is the right carrier of share growth, which the engine already
  provides.
- **Mortality shocks are not conditioned on site quality.** For 165
  cities, Black Death mortality was "uncorrelated with various city
  characteristics proxying for physical geography, economic
  geography, human capital and institutions" (JJK WP p. 49). **A
  shock in via must therefore not read the suitability field.** Any
  correlation between shock magnitude and site quality would be an
  artefact with no support in the record — and, worse, would
  contaminate the inhomogeneous-Poisson null of Decision 7, which is
  fitted on that same field.
- **Recovery is path-dependent.** Cities recovered to pre-plague
  levels on average by 1500 (JJK WP p. 21), but Barcelona, Florence,
  Lübeck and Venice took 5, 30, 10 and 25 years while Narbonne and
  Winchester "shrank to insignificance" (WP pp. 21–22). The stage
  needs hysteresis, not a return to a fixed point.

All JJK citations above are to the open working paper IIEP-WP-2020-14
(4 August 2020). **The published *JEL* 60(1):132–178 is paywalled and
was not read; no page number here may be attributed to the journal.**

## Decision 4 — Settlements sit at corridor nodes; `O_i` is seeded from suitability

Seed sites are the trunk **node** and **junction** records of ADR 0012
D5 — passes, heads of navigation, river mouths, and the mechanically
derived degree-≥3 junctions. This is the causal chain of ADR 0009
taken literally: corridors exist before settlements, so settlements
attach to the network rather than the network being drawn between
settlements.

`O_i` is seeded from the suitability spectra at the node's cell —
patch rank, freshwater distance, crossability, harbour components —
combined by a **declared** weighting, labelled `heuristic`, with **no
composite score emitted** (ADR 0008 D7, and the ADR 0011 precedent of
refusing to bake one). The weighting is config, exposed, and swept.

**Founding is a forcing event; survival is the engine's own.**
Research 0006 separates these cleanly and this ADR adopts the
separation. Britnell (1981) documents the market charter as the
economic founding act — crown-licensed for a fee, distinct from and
often preceding physical planning, proliferating 1200–1349 and
thinning after the Black Death as unviable markets failed. So *which*
sites are founded and *when* is a declared event stream (Tier-2), and
Beresford's plantation package is not a growth mechanism and is not
used as one.

What is **not** separately modelled is survival, because the engine
already supplies it. Research 0006's rule — a chartered site survives
iff its catchment captures demand above a viability threshold, and
failures revert to village status — is exactly Decision 2's dynamic:
`D_j` *is* the catchment demand over the corridor-cost field, and
the `δ/κ` floor *is* the viability threshold. Adding a second
survival test would double-count. This is recorded because the
coincidence is easy to miss and expensive to discover later.

**Era enters through the cost field, not through a re-tuned spacing
constant.** Research 0006 notes the spacing parameter "scales with
transport speed" (~10 km medieval foot/cart half-day; 8–11 miles for
Plains rail depots). In via that scaling is automatic, because `c_ij`
is corridor time in hours and the era's speeds already live in ADR
0012's mode set. No era-indexed spacing parameter is introduced.

## Decision 5 — Delineation is fixed here, and executed in `via-bench`

Via's settlements are node populations; GHS-UCDB's are polygons cut
from a 1 km population grid. Comparing them directly is not
like-for-like, and research 0016 §1 shows the cost of getting this
wrong: **the rank-size exponent moves 0.90 → 1.17 on the delineation
choice alone**, which is the whole span the literature argues about.

Adopted: the **GHSL Degree of Urbanisation** algorithm, because it is
the delineation behind GHS-UCDB and therefore the only one that makes
the benchmark like-for-like. Parameters fixed here:

- 1 km² cells; density threshold ≥ 1500 per km² **of land**, with the
  land mask from terrain (Decision 7's window rule);
- **4-point contiguity for clustering** — this is the specification's
  own rule ("to avoid over-aggregation"), not a deviation from it;
- **but the gap-filling majority rule counts the 8-neighbourhood**:
  "if five or more of the (eight) cells surrounding a particular cell
  belong to the same unique urban centre, then that cell is also
  considered to belong", iterated to idempotence. **The asymmetry is
  deliberate and load-bearing — an implementation using one
  convention throughout will not reproduce GHSL.**
- population threshold ≥ 50,000; holes < 15 km² filled;
- **a declared tie-break**, because the published majority rule is not
  deterministic — the manual's own footnote admits "the outcome of the
  majority rule may lead to different results depending on which urban
  centre is treated first". The tie-break is ascending row-major cell
  index, stated in the summary as a declared adaptation.

**The tier question must be decided before the first run, not after
it.** All three GHSL thresholds are defined on 1 km² cells and none
transfers to another cell size without restatement; and the 50,000
urban-centre threshold **may select nothing at all** in a pre-modern
world, which would push the comparison down to the "urban cluster"
tier (≥ 300/km², ≥ 5,000) and change the reference set away from
UCDB. This ADR fixes the rule rather than the outcome: **the
urban-centre tier is attempted first, and if it yields fewer than
five centres the run falls back to the urban-cluster tier and says so
in the summary — a fallback that also changes the benchmark
reference, which the summary must name.** Note that the disagreement
between spec and reference implementation lives precisely in that
secondary tier: the DUG tool applies 4-connectivity where the papers
specify 8. If the fallback fires, 4 is used, matching the tool that
produced the reference data.

Applying a census delineation algorithm to a simulated field is
itself a declared adaptation. Its one close precedent is **Rybski et
al. (2013)**, who applied the *CCA* — not GHSL — to a 630 × 630
simulated lattice, almost exactly via's raster scale. So the
precedent covers the *move*, not the algorithm, and the summary says
both. Rybski also supplies two warnings that go next to every
rank-size number: **ζ(p) = a + b·ln(p) + c·ln(1−p)** — the exponent on
a generated field is confounded by the occupied fraction `p`, so a
gate that does not control for it measures world-fill rather than
settlement structure — and **the largest cluster is a "Dragon King"**,
markedly larger than Zipf predicts, which is why Decision 7 must
state explicitly whether it is inside the fit.

Per ADR 0009 D4, the *measurement* lives in `via-bench`; this ADR
fixes the algorithm so the stage and the benchmark cannot drift.

## Decision 6 — Shocks, epochs, and the churn law

- **Epochs** are declared switches (ADR 0012 D7's era structure,
  inherited). Each epoch carries its own budget, `O_i` weighting, and
  mode availability.
- **Shocks** are a declared Tier-2 process: magnitude, arrival, and
  spatial extent are forcing. Magnitude has a citable anchor —
  JJK Table 1 (WP p. 48): Western Europe 72.8 m in 1300, mortality
  38.75%; 274 localities population-weighted 38.90%; country values
  from 20% (Austria/Czechia/Hungary) to 55% (England & Scotland,
  Scandinavia). **The spatial field is uniform-random by
  construction**, per Decision 3's third constraint.
- **Churn between epochs** uses Verbavatz & Barthélemy (2020)'s
  fluctuation law as the citable form: `∂_t S_i = η_i S_i + D·S_i^β·ζ_i`
  with `ζ_i` **Lévy-stable** (fitted α: France 1.43 ± 0.07, US 1.76 ±
  0.07, UK 1.32 ± 0.26, Canada 1.69 ± 0.12), Itô convention. Two
  import hazards, both recorded in 0016 and repeated here because
  they are silent failures: **their α and β mean the opposite kind of
  thing from Harris–Wilson's α and β — rename on sight**, and the
  time scaling is **dt^(1/α), not dt^(1/2)**.
- Their **rank-turbulence metric `d`** (mean absolute rank shift per
  year) is adopted as a dynamics character: France 1876–2015 over 500
  cities gives 6.0, their model 6.1, Gabaix's 8.0.
- **The shock has a qualitative validation target that costs
  nothing to check.** Research 0006 records English market charters
  proliferating through 1200–1349 and then thinning after the Black
  Death "as unviable markets failed" (Kent: under 20 markets in 1200
  to over 80 by 1350). A shock applied to via should therefore
  produce **net loss of small centres below `δ/κ` while the surviving
  hierarchy re-concentrates** — the marginal-site die-off, not a
  uniform rescaling of every settlement. This is a *sign-and-shape*
  expectation, not a band: no count is claimed, and it is reported in
  the D10 visual channel rather than gated.
- Verbavatz's anti-Zipf result is absorbed into the validation
  doctrine rather than argued with: "Zipf's law does not hold in
  general due to finite-time effects", so an upper-tail power-law fit
  "may be mistaken for a Pareto-tail with a spurious exponent that
  changes with the definition of the upper-tail."

## Decision 7 — The benchmark channel, the nulls, and the window

- **Rank-size**: Gabaix–Ibragimov `log(Rank − ½)`, with **`SE =
  √(2/n)·ζ̂`, not the OLS standard error**. At n = 20 that is ±31.6%,
  and the summary states it — the character may simply not
  discriminate at via's n, and that has to be visible rather than
  discovered later. Clauset's `x_min` needs `n_tail ≳ 1000`, which via
  will not have, so **a fixed threshold is declared instead**. Two
  conventions are fixed here rather than left to the implementation:
  **the largest settlement is excluded from the fit** (Rybski's
  Dragon King handling, and via's primacy character reports it
  separately), and **every reported ζ is accompanied by the occupied
  fraction `p`** so the Rybski confound is visible rather than
  absorbed. The exponent convention is stated with every number.
- **Spacing**: Clark–Evans is **demoted, not corrected**. Its edge
  bias runs toward *spurious regularity*; Baddeley's own 230-page
  notes never mention it; spatstat calls it "crude"; and Donnelly's
  correction is rectangles-only, so it cannot be applied to via's
  coastline window at all. Use G / K / pcf with Kaplan–Meier `cdf`
  edge correction. **The 2.15 ceiling is removed** — Philo & Philo
  (2022) measure 2.23, 2.27 and 2.97, approaching it from above.
- **Realized spacing is a character; the legal rule is not a target.**
  Britnell (1981) records Bracton's doctrine that a new market was
  injurious within 6⅔ miles of an existing one — one-third of a
  20-mile day's round trip — which is an explicit, citable spacing
  rule and enters via as **declared forcing**. But Britnell is equally
  explicit that "in practice spacing was set by competitive failure
  rather than the rule alone", so **the realized spacing distribution
  is the emergent quantity that gets gated, and hitting 6⅔ miles is
  not the pass condition.** Confusing the two would turn a forcing
  parameter into a fake validation.
- **Envelopes are global, not pointwise**, with M = 19 for an exact
  5% level, declared per character.
- **The null ladder has three rungs**: CSR → **inhomogeneous Poisson
  fitted as the single-parameter offset model `log λ(u) = θ + log Z(u)`**
  ("suitability alone, no interaction" — this is the load-bearing
  rung) → the generator itself. ADR 0008 D9 requires "at least one
  character that a null model demonstrably fails"; **this ADR
  discharges that by naming rung 2 as the null that must fail**, which
  is stronger than D9 asks, because rung 2 consumes the same
  suitability field the generator does and is therefore the only
  adversary that isolates the interaction mechanism from the terrain.
- **The window** is defined on the land mask, with inland water, the
  coastal ribbon and unusable terrain **inside** it. No source
  addresses this and it changes every CSR benchmark, so it is a
  declared protocol clause rather than an implementation detail.

## Decision 8 — Gates

All binary, all ADR 0008 D1 internal consistency, unevaluable is
fail, enforced as in-stage hard errors and as cargo tests. Every gate
re-reads the stage's inputs and outputs **from disk**, never from the
compute's in-memory state — the ADR 0011/0012 precedent.

1. **Mass conservation** — `|Σκ_j W_j − (ΣO_i + δM)| / (ΣO_i + δM)`
   below tolerance at convergence. This is Decision 2's balancing
   condition; if it fails, the engine is not Harris–Wilson.
2. **Flow conservation** — `Σ_j T_ij = O_i` for every `i`.
3. **Size floor** — every surviving settlement has `W_j ≥ δ/κ`.
4. **Convergence** — fixed-point residual below tolerance, and the
   `ε · max_j D_j < 2` bound (Decision 2) held at every step.
5. **Attachment integrity** — every emitted attachment references an
   existing ADR 0012 trunk edge; its bearing is re-derivable from the
   recorded path geometry; its route cost equals the edge's recorded
   time.
6. **Delineation determinism** — the Decision 5 majority pass is
   idempotent and reproduces bit-identically under the declared
   tie-break.
7. **Artifact contract** — every emitted record recomputed from disk.

No composite score, in any gate or acceptance claim (ADR 0008 D7).

## Decision 9 — Representation and the contract onward

Namespaced `settlement.<label>.*`, one `StageRecord`, label rules and
manifest registration exactly as ADR 0011/0012. **No `.vgeo`** — the
deferred per-feature schema obligation stays with morphology (ADR
0009 D3).

The stage emits the **ADR 0009 D2 record** in full, per settlement:
site (position, terrain cell); population (count, epoch);
attachments (per corridor: bearing, class, route cost); role
(through-traffic volume, market-catchment mass); regime (the declared
era/culture bundle). Role's two scalars are **via's own inventions
with no literature template** (ADR 0009 D2 says so), and carry
`heuristic` labels naming that absence.

Rasters: a population field at the delineation cell size (the input
to Decision 5), and the market-access field read from ADR 0012's
`hours_to_sea_land`.

## Decision 10 — Declared limitations

- The budget magnitude has no published series (Decision 3). This is
  the largest declared gap in the stage and belongs in the summary,
  not only in this ADR.
- Size dispersion is inherited from terrain, not generated by the
  engine (Osawa). The stage may never claim its hierarchy as an
  emergent result of spatial interaction.
- The rank-size gate is not something the engine is entitled to pass
  (Wilson 2008); it is reported, and a pass is not evidence
  for the mechanism.
- Era-transition trajectory shape, wells→capacity, wall cost `c_w`,
  the movement-to-land-use multiplier, and non-Western scope are all
  **declared** rather than derived (research 0016's own list).
- Every water-derived input inherits `metric_values_conditional_on_k_q`
  from ADR 0012 D5.

## Consequences

- `via-settlement` enters the workspace as the fourth stage crate,
  and the first that can hold a statistical gate.
- The validation debt deferred by ADR 0012 D1 becomes collectable —
  and Decision 10 states in advance which parts of it this stage
  cannot pay. ADR 0011 is not a creditor here (Context).
- Research 0016's retrieval targets become the checklist for
  upgrading the budget from declared forcing to citation: the
  Maddison Project, McEvedy & Jones (1978), Broadberry et al.,
  Wrigley (1967), Woods (2003), Davenport, and any Bairoch/de Vries
  transcription.
- The GHSL parameter set fixed in Decision 5 must match whatever
  `via-bench` uses against GHS-UCDB, or the comparison silently stops
  being like-for-like.

## Rejected

- **Deriving the population budget inside the engine.** Three
  primaries agree it is impossible in this model family (Decision 3).
  Any "endogenous" budget would be a hidden forcing wearing a
  mechanism's clothes.
- **Clark–Evans with an edge correction.** Donnelly is
  rectangles-only and via's window is a coastline. Retaining a
  statistic whose bias points at *spurious regularity* while unable
  to correct it would manufacture the result the stage most wants.
- **Clauset `x_min` selection.** Requires `n_tail ≳ 1000`; via will
  have tens. A declared fixed threshold is honest; a fitted one at
  n = 20 is noise with a citation attached.
- **A composite settlement-quality score** to seed `O_i`. ADR 0008 D7,
  and the ADR 0011 precedent of emitting component spectra instead.
- **Conditioning mortality shocks on suitability.** Contradicted
  directly by JJK WP p. 49, and it would contaminate the Decision 7
  null that is fitted on the same field.
- **Doubly-constrained IPF balancing.** Unnecessary for a
  singly-constrained model (Decision 2).
