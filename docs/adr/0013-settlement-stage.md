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
D1 both defer their realism claim" was wrong on both counts. Amended a
third time after an end-to-end internal-consistency read: `α` and `β`
were wrongly filed as Tier-2 budget forcing, which would have declared
the pattern the Tier-1 gate is meant to test and collapsed Decision 1;
they are calibrated parameters of the mechanism. Also: `α > 1` is no
longer asserted, and the `δ/κ` size-floor gate is scoped to
convergence, since it is an equilibrium floor and would have fired on
correct runs mid-solve. The same read found the stage's biggest
unclosed hole: nothing specified how node `W_j` becomes the population
*grid* GHSL consumes, and that rule governs whether anything
delineates at all — now named in Decision 5 and in Decision 10 as a
swept declared choice. A fourth amendment finished the read over
Decisions 6, 8 and 9: the rank-turbulence character was demoted to an
internal diagnostic (its published value is computed over 500 cities
and the metric is bounded by rank count, so via's tens cannot be
scored against it); the flow-conservation gate became a tolerance
rather than a float equality; the stability-bound gate was split into
a runtime assertion plus a recorded per-run maximum, since a per-step
quantity cannot be re-read from disk; and the delineation-determinism
gate moved to `via-bench`, which is the crate that actually runs it.

**Amended a fifth time on the five-track adversarial review (26
agents; 40 raw findings, 10 verified against two adversarial lenses
each, 4 confirmed, 6 refuted, 30 below the severity cut and NOT
examined). The review's verdict on the text as it then stood was
DO-NOT-RATIFY, and all three blocking findings are fixed here:**
(1) the adopted dynamic had lost its `W_j` prefactor, making it a
linear relaxation that contradicted both the gradient-flow claim and
the logistic reduction Decision 2 derives from it — the multiplicative
form is restored and `x = ln W` named as the integrated coordinate;
(2) the stability bound omitted `dt`, was stated on unnormalised
population units under which it would hard-error on every run, and had
been loosened from research 0016's recommended `0.5` margin to the
marginal-stability value `2` by an earlier amendment of mine — the
normalisation is now declared, the bound is on `ε·dt`, and the margin
is restored; (3) `c_ij` pointed at two artifacts that cannot yield a
pairwise cost — it is now a declared cost field, junction seeding is
deferred behind a named ADR 0012 amendment, and `c_ii` and
cross-component pairs have stated rules. **The 30 unexamined findings
are not cleared, and this ADR should not be ratified as if they
were.**

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
  forcing** — `ΣO_i`, κ, δ, the shock magnitudes and the epoch
  schedule. Decision 3 states why this is not a shortcut.
- **`α` and `β` are not in that bucket, and the distinction is what
  keeps the Tier-1 claim honest.** Osawa's gloss is precise: "O and κ
  change only the **scale** of h". `α` (returns to scale) and `β`
  (distance decay) set the *pattern* — they are parameters of the
  Tier-1 mechanism, and under ADR 0008 D6 they are **calibrated by
  measured sweep against reference data, with calibration separated
  from validation**, never declared and never hand-tuned. Filing them
  as forcing would declare the very shape the statistical gate is
  supposed to test, and the Tier-1 ruling above would collapse into
  decoration.
- **`ε` and `dt` are numerical, not physical.** They are integration
  parameters, jointly constrained by Decision 2's stability bound —
  which is stated on the product `ε·dt`, since constraining either
  alone leaves the other free to diverge — and by convergence. They
  carry no claim about the world, and a result that moves when `ε`
  changes within the stable range is a bug.
- **`K`, the epoch's total settled mass, is Tier-2 forcing and is the
  normalisation the solve runs in** (Decision 2). It is the same
  quantity as `ΣO_i/κ`; naming it separately matters because every
  published `ε` and convergence tolerance assumes shares, not people.
- **The *shape* of `O_i` across nodes is Tier-3, and only its total is
  Tier-2.** The bullets above cover `ΣO_i`; the per-node weighting
  that distributes it (Decision 4) is a suitability composite, and
  ADR 0003 names suitability scoring as the canonical Tier-3 example.
  A Tier-1 mechanism reading Tier-3 inputs is legitimate — corridors'
  `c_ij` enters the same way — but it has to be said out loud,
  because it is exactly why Decision 10's first limitation bites:
  when Osawa shows the engine yields equal-sized centres on
  homogeneous space, the dispersion via actually produces is coming
  through *this* input, at Tier-3, under a declared weighting.
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
    dW_j/dt = ε · W_j · (D_j − κ W_j + δ)

**The `W_j` prefactor on the last line is load-bearing and an earlier
draft of this ADR omitted it.** Without it the dynamic is a linear
relaxation, not the Boltzmann–Lotka–Volterra dynamic the name refers
to, and three things this ADR asserts become false at once: Ellam's
gradient flow in `x = ln W` requires it (`d(ln W_j)/dt = ε(D_j −
κW_j + δ)` is exactly the multiplicative form divided by `W_j`);
Wilson's quadratic difference reduction cannot be obtained from an
equation linear in `W`; and `W = 0` stops being a fixed point, which
removes the very absorbing-state that Decision 2's `δ > 0` argument
exists to defeat. The additive form is what research 0001 line 129
records, and that rendering is wrong; it is corrected here rather
than inherited.

**The state is integrated in `x = ln W`, not in `W`.** This is the
coordinate Ellam's gradient flow is stated in, and it is the reason
the floor works: as `W → 0` the log-space drift tends to
`ε(D_j + δ) > 0`, so a shrinking centre is pushed back up instead of
crossing zero. Integrating in `W` would need an explicit positivity
clamp, and a negative `W_j` makes `W_j^α` NaN for non-integer `α`,
which Decision 8's "unevaluable is fail" preamble turns into a gate
failure. Emitted populations are `exp(x_j)`.

`O_i` is the origin mass at node `i` (Decision 4). `α` is the
returns-to-scale term and `β` the
distance decay; **their values are calibrated, not asserted here**
(Decision 1). The regime note matters for reading results but is not
a constraint the ADR imposes: `α > 1` drives agglomeration, `α ≤ 1`
disperses, and which side the calibration lands on is a finding
rather than a setting.

**`δ > 0` is required, not optional.** Research 0016 records both
reasons: at `δ = 0` the Gibbs measure is unnormalisable, and dead
zones become absorbing states — Osawa's "once abandoned, a zone will
never obtain a new retailer regardless of the extent of transport
costs". The minimum settlement size is `δ/κ`, read off the interior
equilibrium `D_j − κW_j + δ = 0` as `D_j → 0`; it is a derived
quantity, not a separate threshold to tune.

**Units and normalisation, declared — because nothing downstream is
well-posed without them.** The stage solves on **shares**: `Σ_j W_j =
K` with `K` the epoch's declared total settled mass, and `κ =
(Σ_i O_i + δM)/K` (Ellam Eq. 25, where `M` is the number of seed
sites). `O_i` is expressed in the same share units, so `Σ_i O_i = K`
by construction and `D_j` is dimensionless-per-unit-mass. This is
what makes `ε` a pure rate and the stability bound below dimensionally
meaningful; solving on absolute head-counts instead would leave `ε·D_j`
carrying population units and every published `ε` value inapplicable.
Absolute populations are recovered once, at emission, by multiplying
the converged shares by `K`.

**IPF/Furness balancing is not needed** for a singly-constrained
model — `A_i` closes in one pass. It is recorded here only because
the corpus mentions it, and adopting it would be cargo-culting a
doubly-constrained model's machinery.

**Integrator discipline, and a corpus misreading corrected.** The
continuous Harris–Wilson dynamic is a gradient flow in `x = ln W`
(Ellam et al.) and therefore **cannot be chaotic**. Any chaos
observed in via would be a discretisation artefact, and is treated as
a bug rather than a finding.

**The stability bound, derived in the coordinate actually
integrated.** Explicit Euler on `x = ln W` gives `x_{n+1} = x_n +
ε·dt·(D_j − κ e^{x_n} + δ)`. Linearising at the interior fixed point,
where `κW* = D_j + δ`, the multiplier is `1 − ε·dt·(D_j + δ)`, so the
fixed point is stable exactly when

    0 < ε · dt · (D_j + δ) < 2

and **`ε · dt · max_j (D_j + δ) < 0.5` is asserted at runtime** — the
margin research 0016 recommends, not the marginal-stability point.
Three things about this bound were wrong in an earlier draft and are
corrected here.

*`dt` belongs in it.* In any explicit scheme the step multiplier is
`ε·dt`. A bound on `ε` alone leaves `dt` free, so `dt = 100` would
pass an assertion on `ε·max_j D_j` and diverge — while Decision 1
claims both are "constrained by Decision 2's stability bound".

*The bound is on the shares of the declared normalisation above*, not
on absolute head-counts. With `D_j` in people, `ε·D_j` carries
population units, every published `ε` is inapplicable, and at
`D_j ~ 1e3–1e5` the assertion would hard-error on every run.

*2 is not a safe bound, and 0.5 was not an invention.* Research 0016
derives `ε·D_j < 2` (fixed point) and `< 2.57` (chaos onset) from
Wilson's difference equation, then separately recommends "Assert
ε·max(D_j) < 0.5 at runtime". A previous amendment to this ADR read
the derived marginal value as superseding the recommended margin and
raised the assertion to 2 — which places the run arbitrarily close to
the period-doubling boundary that this decision elsewhere calls "a
bug rather than a finding". The margin is restored.

**Wilson's logistic reduction is retained only as what it is.**
`ΔZ_j = ε(D_j − Z_j)Z_j` is the logistic map with `r = 1 + εD_j`, and
research 0016 pairs it with May (1976) Table I (period-2 at `a = 3.0`,
chaos at `a_c = 3.5700`) — **via's own algebra on Wilson's equation
and May's thresholds, declared as such, and valid only in Wilson's
normalised-share case `κ = 1, δ = 0`.** Via does not integrate that
map: the log-coordinate Euler step above is of the exponential
(Ricker) family, whose bifurcation constants differ and **are not
imported here**, which is the second reason the assertion carries a
margin rather than sitting on a named threshold.

Published settings, for orientation rather than adoption: `ε = 1`
with `dt = 0.01` under Euler–Maruyama (Zachos et al.), or `ε = 0.01`
"so that the model does not converge too rapidly" (Peeples &
Brughmans). Their convergence test `Σ(D_j − W_j)² < 1e-5` is an
equilibrium residual **only when `κ = 1` and `δ = 0`**; via's residual
is stated on the actual equilibrium condition, `max_j |D_j − κW_j + δ|`
in the declared share units, with a 10,000-iteration cap.

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

Seed sites are the trunk **node** records of ADR 0012 D5 — passes,
heads of navigation and river mouths. This is the causal chain of ADR
0009 taken literally: corridors exist before settlements, so
settlements attach to the network rather than the network being drawn
between settlements.

**Junctions are excluded at era 0, and the reason is a contract gap,
not a design preference.** An earlier draft seeded them too. But ADR
0012 D5 emits junction records as `(cell, x/y, degree, incident edge
ids)` with **no traversal time of any kind** — verified against
`runs/big-s102/corridors.site.json`, whose 1465 junction records carry
exactly `cell, x, y, degree, edges`. A junction sits in the *interior*
of an edge path, and edge records store their path as untimed
`(cell, mode)` steps, so no cost from a junction to anywhere is
recoverable without re-deriving the movement model — which Decision 2
forbids. **Seeding junctions therefore requires an upstream amendment:
ADR 0012 D5 must emit cumulative time along each edge path (per step,
or at minimum per junction cell).** That amendment is named here as a
prerequisite for junction seeding and is not assumed by this ADR.
Until it lands, the seed set is the 1198 trunk nodes, not the 2663
nodes-plus-junctions.

### The cost field `c_ij`, declared explicitly

Decision 2 says `c_ij` is never re-derived here. That obligation is
only dischargeable if the cost field is *stated*, because neither
artifact ADR 0012 names supplies a pairwise cost on its own:
`hours_to_trunk` is identically zero at every seed by construction —
its sources are "every cell traversed by any trunk edge … at zero"
and seeds are trunk cells — and edge records give end-to-end hours
only for Gabriel-graph adjacencies. So:

- **`c_ij` (i ≠ j) is the shortest-path time over the trunk graph**,
  vertices = trunk nodes, arc weights = the recorded directional
  `hours_ab` / `hours_ba`. Directionality is preserved; `c_ij` is not
  assumed symmetric, and the asymmetry is real (water legs and slope).
  This composes recorded times only — it re-derives nothing.
- **`c_ii` is an intrazonal convention, declared Tier-2 forcing with
  no literature template in the corpus.** It is `c_ii = γ · min_{k≠i}
  c_ik` with `γ` in config and swept, because `c_ii` controls
  self-containment: at `c_ii = 0`, `exp(−β·0) = 1` makes every origin
  its own strongest destination and manufactures precisely the size
  dispersion Decision 10 forbids the stage from claiming. The sweep's
  effect on the size distribution is reported, never a single value.
- **Cross-component pairs have no path, and are not given a large
  finite cost.** The solve is **per component**: an unreachable pair
  would otherwise leave `A_i = 1 / Σ_k W_k^α exp(−β c_ik)` dividing by
  zero and propagating NaN into every gate. Nodes with component
  `−1` (unanchored) are excluded from the solve, and both the
  component partition and the excluded count are recorded in the
  summary. On `runs/big-s102` that is 1188 / 6 / 4 — a dominant
  component, a small second one, and four unanchored nodes.

Because `c_ij` is now a defined field rather than a pointer, Decision
8 gate 5 checks route cost against **that** field, not against a
single edge's recorded time.

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

**The step before the algorithm is the one that decides the result,
and this ADR cannot close it.** GHSL consumes a population *grid*;
Harris–Wilson produces `W_j` at *nodes*. Something must spread node
mass onto 1 km² cells, and the delineation is almost entirely at that
rule's mercy: a node carrying `W_j = 60,000` in a single cell clears
both the 1500/km² density threshold and the 50,000 centre threshold
immediately, while the same population spread over 40 km² clears
neither. Since Decision 5's fallback (below) triggers on exactly those
thresholds, **the spreading rule silently controls which benchmark
tier the run is measured against.**

The obvious anchor is a settled-area scaling `A ~ N^a`, and research
0012 records Ortman et al. (2014) reporting `a ∈ [2/3, 5/6]` and
era-invariant — but research 0016's open gaps list it as **outside
the sweep and unverified here**, so this ADR does not adopt it. The
rule is therefore **a declared Tier-2 choice fixed at implementation,
not in this ADR**, subject to three requirements: it is exposed in
config; its exponent (or equivalent) is swept, with the resulting
movement in delineated-centre count and ζ reported as a sensitivity,
not a single number; and the summary names it as the largest declared
choice standing between `W_j` and every benchmark comparison. **If
Ortman is verified before implementation, it supersedes this and the
ADR should be amended.**

With that input assumed, the algorithm adopted is the **GHSL Degree
of Urbanisation**, because it is the delineation behind GHS-UCDB and
therefore the only one that makes the benchmark like-for-like.
Parameters fixed here:

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
  year) is adopted, but **as an internal diagnostic only, not as a
  benchmark character** — and the reason is the same `n` argument
  Decision 7 applies to ζ. Their published values (France 1876–2015:
  6.0; their model 6.1; Gabaix 8.0) are computed **over 500 cities**.
  `d` is mechanically bounded by how many ranks exist to move
  through, so a `d` computed over via's tens of settlements is not
  the same quantity and cannot be scored against 6.0. It would be the
  precise error this ADR refuses elsewhere: comparing two numbers that
  share a name and not a definition. **The metric is therefore used
  via-against-via — across parameter settings and seeds — and becomes
  an external character only if the reference is recomputed on a
  matched-`n` top-`n` subsample**, which needs the underlying French
  series and is recorded as a retrieval target, not assumed.
  It also requires dated epochs to have a time base at all; ADR 0012
  D7's declared availability dates supply one.
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
  will not have, so **a fixed threshold is declared instead**.
  **A minimum `n` for reporting ζ at all is declared here, because
  Decision 5's fallback exists precisely because `n` can collapse.**
  Since `SE = √(2/n)·ζ̂`, the relative width is fixed by `n` alone:
  ±63.2% at n = 5, ±44.7% at n = 10, ±40.8% at n = 12, ±36.5% at
  n = 15, ±31.6% at n = 20. **The floor is n = 15 after the largest
  settlement is excluded**; below it the character is *not reported*,
  and the run says it was withheld and why rather than quietly
  printing a number. Publishing an exponent at the ±44.7% of `n = 10`
  would be the same error this ADR rejects Clauset's `x_min` for.
  Note that even at the
  floor the interval is wide enough that ζ will rarely separate two
  candidate mechanisms — which is a fact about via's `n`, not about
  the estimator, and belongs in the summary next to the number.
  Two further conventions are fixed here rather than left to the
  implementation:
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
   condition; if it fails, the engine is not Harris–Wilson. It is
   unaffected by the multiplicative correction to the dynamic: summing
   the interior equilibrium `D_j − κW_j + δ = 0` over `j` and using
   `Σ_j D_j = Σ_i O_i` returns the same identity.
2. **Flow conservation** — `|Σ_j T_ij − O_i| / O_i` below tolerance
   for every `i`. Stated as a tolerance, not an equality: `A_i`
   normalises a sum of floats, so exact equality is unachievable and
   a gate demanding it would be unevaluable — which this preamble
   counts as failure.
3. **Size floor** — **at convergence**, every surviving settlement has
   `W_j ≥ δ/κ`. The scope matters: `δ/κ` is the *equilibrium* floor,
   so a transient `W_j` below it during the solve is expected, not a
   violation, and a gate that checked every step would fire on
   correct runs.
4. **Convergence** — the residual `max_j |D_j − κW_j + δ|` below
   tolerance in the declared share units, and the Decision 2 stability
   bound respected. **The bound needs splitting to be checkable at
   all**, because a per-step quantity cannot be re-read from disk
   after the run: the per-step `ε · dt · max_j (D_j + δ) < 0.5` is a
   **runtime assertion** (hard error on violation), and the stage
   records `max` of that quantity **over all steps** in the summary;
   the *gate* re-reads that recorded maximum from disk and checks it.
   Without the recorded maximum the gate would be unevaluable, and the
   preamble makes unevaluable a failure.
5. **Attachment integrity** — every emitted attachment references an
   existing ADR 0012 trunk edge; its bearing is re-derivable from the
   recorded path geometry; its route cost equals the Decision 4 cost
   field `c_ij` evaluated on that attachment, which for a single-edge
   attachment reduces to the edge's recorded directional time.
6. **Population-raster determinism** — the emitted population field
   reproduces bit-identically from the settlement records and the
   declared spreading-rule parameters. This is the stage's gate; note
   that **delineation itself is not gated here, because the stage
   does not run it.** Per Decision 5 and ADR 0009 D4 the delineation
   executes in `via-bench`, so its determinism gate — the majority
   pass idempotent and bit-identical under the declared tie-break —
   is a `via-bench` gate, listed in Decision 5 only because this ADR
   fixes the algorithm. Putting it in the stage's list would have
   gated a computation the stage never performs.
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

Rasters: a population field at the delineation cell size — produced
by Decision 5's declared node-to-grid spreading rule, whose parameters
and sweep are recorded in the summary alongside it, because that
raster is not an observation but the output of the stage's largest
declared choice — and the market-access field read from ADR 0012's
`hours_to_sea_land`.

## Decision 10 — Declared limitations

- The budget magnitude has no published series (Decision 3). This is
  the largest declared gap in the stage and belongs in the summary,
  not only in this ADR.
- **The node-to-grid spreading rule (Decision 5) is the largest
  declared gap in the *measurement* path**, and it is a different kind
  of gap from the budget: the budget sets scale only, whereas the
  spreading rule can change which benchmark tier the run is compared
  against and therefore which reference population every character is
  scored on. Until Ortman et al. (2014) is verified it stays a swept
  declared choice, reported as a sensitivity rather than a value.
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
- **The multi-good extension** (Hsu 2012; Mori, Akamatsu, Takayama &
  Osawa 2023), even though Context finding 3 names it as the *only*
  published route to an endogenous size hierarchy. Two reasons, and
  the second is the real one: it is a substantially different model
  from the one the three balancing-condition primaries describe, so
  adopting it would forfeit the agreement Decision 2 rests on; and
  **it would not deliver what it appears to promise** — neither paper
  predicts the exponent, which inherits the tail index of an assumed
  input distribution. Via would be choosing that input, so the
  hierarchy would still be forced, just less visibly. Recorded as the
  extension to revisit if the single-good stage is judged too flat.
