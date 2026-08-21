# Research 0016 — Settlement extraction, measurement, and null models

Status: research corpus — pre-decision (2026-08-22). Nothing here is
adopted; adoption requires an ADR. Commissioned for the via-settlement
ADR after a corpus sweep established that the settlement engine itself
(Harris–Wilson, von Thünen, founding-as-forcing) can be adopted from
citations already held in 0001/0004/0005/0006/0007/0008, while five
things could not: how to cut discrete settlements out of a generated
population field, what supplies the population budget the engine
allocates, how to measure a settlement point pattern on a bounded
region, what to null-test it against, and how to integrate a dynamic
with documented bifurcations. This dossier covers the first, third and
fourth; the second is a sweep still outstanding at time of writing.

Verification status is marked per item: *primary* (full text read),
*primary-part* (the relevant sections read verbatim from the primary),
*proxy* (primary inaccessible; formulas verified from two or more
independent authoritative renderings that agree), *abstract*, or
*cite-only* (citation confirmed, text not obtained). Where a sweep
could not read a source, that is stated rather than papered over.

**A methodological note that belongs at the top.** During the
delineation sweep, one automated retrieval returned a fluent and
entirely fabricated account of the City Clustering Algorithm — it
described the parameter ℓ as a population density in persons/km² with
a preferred value of 1,000, all of which is wrong. Every parameter in
this dossier was subsequently read out of an extracted document text.
That failure is recorded because it is precisely the failure this
corpus exists to prevent.

## 1 — Cutting settlements out of a continuous field

Every settlement-system statistic the project intends to report —
rank-size exponent, settled-area-vs-population scaling, spacing,
primacy — is computed on discrete settlements, but the generator
produces a continuous field. The delineation step is therefore not a
detail; it sets the answer.

### The scale of the problem, quantified twice

Rozenfeld, Rybski, Gabaix & Makse (2011), "The Area and Population of
Cities", *American Economic Review* 101(5):2205–2225 [primary,
published version] sweep the CCA's length parameter ℓ from 1 to 8 km
on US data and report the rank-size exponent across it: **ζ ≈ 1.17 at
ℓ = 1 km, falling monotonically to ≈ 0.90 at ℓ ≈ 4–6 km**, recovering
to ≈ 0.95 by 8 km. That is the *same 0.90–1.17 span* the corpus
already records from Soo (2005) for cities-proper versus
agglomerations — reproduced here from the delineation parameter alone,
within one algorithm on one dataset. The concern is citable from two
independent directions.

Arcaute et al. (2015), "Constructing cities, deconstructing scaling
laws", *J. R. Soc. Interface* 12:20140745 [primary] show the effect
crossing regime boundaries. Their headline: "most urban indicators
scale linearly with city size regardless of the definition of the
urban boundaries. However, when non-linear correlations are present,
**the exponent fluctuates considerably**." Their sharpest single case:
area of greenspace moves from β = 0.95 [0.92, 0.99] to **β = 1.23
[1.19, 1.28]** — sublinear to superlinear, non-overlapping confidence
intervals — from a commuting-threshold change alone. A population
cutoff alone flips a regime too: "For the cutoff of 10⁴ individuals,
the exponent lies within the superlinear regime (at a confidence level
of 95%), while for the cutoff of 5×10⁴ individuals, linearity cannot
be rejected."

Their recommendation is procedural rather than algorithmic: build the
whole ensemble of boundary definitions and treat it as "a laboratory
for the scaling analysis", reporting sensitivity rather than a point
estimate. That aligns with the project's existing ensemble rule.

### The candidates

**City Clustering Algorithm** — Rozenfeld et al. (2008), *PNAS*
105(48):18702–18707 [primary, preprint] and Rozenfeld et al. (2011)
[primary]. The rule, verbatim from 2008: "we then grow a cluster by
adding nearest neighbors of the boundary cells with a population
strictly greater than zero, until all neighbors of the boundary are
unpopulated… termed the 'burning algorithm'". In 2011 it gains a
density threshold that they then set to zero: "to minimize the number
of free parameters, **we set the threshold D∗ = 0**". **ℓ is a length,
not a density.** Determinism is explicit and is a genuine virtue:
"the outcome of the CCA is independent of the initial condition",
unlike commuting-based definitions.

Its weakness is that ℓ has no principled value. The only selection
rule in the literature is fitting to US metropolitan areas — maximise
the correlation with MSAs, giving "ℓ = 3 km as the optimal value" —
which is unavailable for a generated world, and whose two criteria
disagree anyway (3 km by correlation, 5 km by log-distance). It also
percolates: at 8 km cells "all cities in the northeastern the USA
spanning from Boston to Washington D.C. form a single cluster."

**GHSL Degree of Urbanisation** — Dijkstra & Poelman (2014), European
Commission Regional Working Paper WP 01/2014 [primary]; the 2021
UN-endorsed methodological manual [primary]; Florczyk et al. (2019),
JRC115586 [primary]; and the GHS-DUG User Guide v3.0, JRC118444
[primary]. The urban-centre rule, verbatim from the manual §7.2.2.1:
grid cells of 1 km² with "a density of at least 1 500 inhabitants per
km² **of land**"; "contiguous high-density cells are then clustered.
Only those clusters with at least 50 000 inhabitants are kept. **To
avoid over-aggregation, four-point contiguity is used**"; then "gaps
in each cluster are filled separately and its edges smoothed" by an
iterative majority rule — "If five or more of the (eight) cells
surrounding a particular cell belong to the same unique urban centre,
then that cell is also considered to belong" — repeated to
idempotence, with holes below 15 km² filled.

Note the deliberate asymmetry: **clustering uses 4-connectivity, but
the gap-filling majority rule counts the 8-neighbourhood.** An
implementation using one convention throughout will not reproduce
GHSL.

**Percolation** — Arcaute et al. (2016), *R. Soc. Open Sci.* 3:150691
[primary] and Fluschnik et al. (2016), *ISPRS IJGI* 5(7):110
[primary]. Both seek a principled threshold: Arcaute at the maximum of
the fractal-dimension spectrum (d = 180 m on intersections, 300 m on
the network), Fluschnik at the peak of mean cluster size excluding the
largest. Both disown its transferability in their own words. Arcaute:
the distance "is **not universal nor uniquely characterised**… It is
not uniquely defined, because the maximum corresponds to some sort of
plateau." Fluschnik: "for many countries l_c cannot be identified
unambiguously, as in the presence of multiple peaks".

### Three findings that settle the choice

1. **GHSL is the delineation behind the project's own reference
   population.** GHS-UCDB is built with it. Adopting anything else
   makes every benchmark a cross-algorithm comparison rather than a
   like-for-like one.
2. **Percolation is *less* reliable on generated fields than on real
   ones.** Rybski, García Cantú Ros & Kropp (2013), *Phys. Rev. E*
   87:042114 [primary] applied percolation to a simulated lattice and
   found p_c ≈ 0.2 against the uncorrelated-lattice value 0.593, could
   not determine it at all for part of the parameter range, and showed
   that "scaling in the form of Zipf's law and fractality are
   reproduced even away from criticality." This effectively rules
   percolation out as a threshold selector here.
3. **The published GHSL rule is not deterministic, and the spec and
   the reference implementation disagree.** The manual's own footnote:
   "the outcome of the majority rule **may lead to different results
   depending on which urban centre is treated first**." And the DUG
   tool applies 4-connectivity to the secondary urban-cluster tier
   where the papers specify 8. "Following GHSL" is ambiguous until the
   adopter picks one and declares it.

### Applying a census algorithm to a simulated field

There is **no published algorithm designed for delineating settlements
from simulated output**, and no published protocol for benchmarking a
generated settlement system against GHS-UCDB. The 2026 *Physics
Reports* review of city-growth modelling (Marquis & Barthelemy,
1180:1–105) [primary] says so directly: "**Only a few models… discuss
the formation of these clusters and the resulting size distribution**
… and further studies are clearly needed."

There is one close precedent, and it is close: **Rybski et al. (2013)
applied the CCA to a 630 × 630 simulated lattice** — almost exactly
via's raster scale — stating "We employ the City Clustering Algorithm
(CCA) and find that the largest cluster is markedly larger than the
remaining ones". So adoption is a *declared adaptation with a
precedent*, not an invention, and the honest framing names both.

That paper also supplies two warnings the project would otherwise
discover the hard way:

- **The rank-size exponent on a generated field depends on how full
  the world is.** They fit ζ(p) = a + b·ln(p) + c·ln(1 − p), with p
  the occupied fraction, and find it "depends strongly on both, the
  model exponent γ and the iteration of the model". **A rank-size gate
  that does not control for the settled fraction measures world-fill,
  not settlement structure.**
- **The largest cluster is a "Dragon King"** — markedly larger than
  Zipf predicts — and they handle it by excluding it from the fit. The
  project's primacy character must decide explicitly whether the
  largest settlement is in or out of the rank-size fit.

A further practical hazard: the 50,000 urban-centre threshold may
select **nothing** in a pre-modern or sparsely settled world, which
would push the comparison to the "urban cluster" tier (≥300/km²,
≥5,000) and change the reference set away from UCDB. And all GHSL
thresholds — 1,500/km², 50,000, the 15 km² hole fill — are defined on
**1 km² cells**; none transfers to another cell size without
restatement.

## 2 — Measuring the pattern: what to use instead of Clark–Evans

The project's benchmark spec currently lists Clark & Evans (1954) R as
a settlement-system character, marked advisory. This sweep establishes
that the demotion should go further, and for a reason that matters:
**the bias runs toward spurious regularity** — exactly the artefact
that would falsely flatter a settlement generator into looking
central-place-like.

### The statistic and its bias

Clark & Evans R = observed mean nearest-neighbour distance divided by
its CSR expectation, E[W] = 1/(2√λ) [proxy: the 1954 original could
not be obtained; formulas verified against Krebs (2017), Dixon (2001)
*Encyclopedia of Environmetrics*, and the spatstat source, which
agree]. Dixon states the bias direction plainly: "Edge effects lead to
overestimation (positive bias) of the mean NN distance." Krebs, after
Sinclair (1985), states the consequence: the uncorrected test "is
biased in favor of regular patterns, so that many aggregated patterns
are judged to be random and many random patterns are judged to be
uniform. **This bias is enormous with small sample sizes (n < 100)**."

### The correction the project cannot use

Donnelly (1978), in Hodder (ed.), *Simulation Methods in Archaeology*,
pp. 91–95 [proxy: verified from Dixon, Krebs and the spatstat source,
which agree to rounding] gives the edge-corrected expectation

    E[W] ≈ 0.5·√(A/N) + 0.0514·(P/N) + 0.0412·(P/N^(3/2))

with A the window area and P its perimeter. **It is restricted to
rectangles.** spatstat refuses it outside one — "Donnelly correction
only available for rectangular windows" — and Donnelly's own stated
validity condition, via Krebs, is a "smooth boundary like a square or
circle… not recommended if the study zone is a long, thin rectangle
because the edge effects are overwhelming." via's window is a
coastline-bounded irregular land mask. The correction is unavailable.

Two further details worth recording: spatstat implements Donnelly's
*expectation* but **not** his *variance*, substituting the uncorrected
Poisson standard error — so "the Donnelly test" as run by the standard
tool is not the test Donnelly specified. And for non-rectangular
windows spatstat's default is the `cdf` correction: estimate G by
Kaplan–Meier and take its mean, which **is** valid on an irregular
window.

### The 2.15 ceiling is false

Philo, C. & Philo, P. (2022), "2.15 or Not 2.15?", *Geographical
Analysis* 54(2):333–356 [primary, open access] prove it analytically
and demonstrate it empirically. The value 2.1491 is the infinite
triangular-lattice limit; in bounded space R exceeds it. Their own
measurements on a "perfectly regular" lattice: **2.23, 2.27 and 2.97**
as the window is narrowed, with Ebdon's hexagonal figure at **3.2829**.
And counter-intuitively, "**Rn\* approaches 2.15 'from above,' not from
below**". Their conclusion: "Countless statements ever since Clark and
Evans (1954) about Rn having an upper threshold of 2.15 are hence
shown to be mistaken."

This independently corroborates Donnelly's simulation-derived warning
about elongated windows, from a different discipline twenty years
later: **R is a function of the window's shape as much as of the
pattern.** The project's spec calls the ceiling "contested"; it is
disproven, and any code or documentation treating 2.15 as a bound
should be removed.

Note also that the corpus's attribution of "hexagonal regularity is
essentially never observed" to Dacey (1962) is **unverified** — that
volume is not digitised. The supportable weaker claim is that complete
regularity, and hence the 2.1491 interpretation, applies only to an
unbounded plane.

### What the standard reference actually recommends

Baddeley, Rubak & Turner (2015), *Spatial Point Patterns* is the
modern reference. Two pieces of evidence about its authors' position:
their own 230-page course notes, *Analysing Spatial Point Patterns in
R* v4.1 [primary-part], **never mention Clark–Evans at all** — zero
occurrences across the whole text, which goes straight from intensity
to F, G, K and J with edge corrections and Monte Carlo envelopes. And
the package documentation, authored by Baddeley, calls R "a **crude**
measure" and warns: "**It is strongly recommended to avoid using
`correction="none"` which would lead to a severely biased test.**"

Their guidance on which correction to pick, for K: "**The choice of
estimator does not seem to be very important, as long as some edge
correction is applied.**"

### The envelope discipline

Baddeley's course notes [primary-part] state the requirement that most
applied papers violate. A Monte Carlo envelope has exact level
α = 2k/(M+1) **only for r fixed in advance**: "If we plot the envelope
and check whether the empirical K function ever wanders outside the
envelope, this is equivalent to choosing the value of r in a
data-dependent way, and **the true significance level is higher**."
The fix is the simultaneous band built from the maximum deviation
D = max_r |K̂(r) − K_pois(r)|, giving "a test of size 5% … by taking
M = 19."

## 3 — Null models: the ladder that gives a result evidential weight

The doctrine requires at least one null the mechanism demonstrably
beats. Complete spatial randomness is necessary but nearly worthless
here: any generator that respects terrain beats uniform-random
placement trivially, so passing it demonstrates almost nothing.

The load-bearing null is the **inhomogeneous Poisson process with
intensity proportional to a covariate** — settlements placed by
suitability alone, with no interaction. Baddeley et al. (2015)
Chapter 9 [primary-part, from the publisher's free sample chapter]
gives it exactly, eqs. 9.18–9.19:

    λ(u) = κ·Z(u)          equivalently    log λ(u) = θ + log Z(u)

with the note that matters: "**there is no coefficient in front of the
term log Z(u) in (9.19), so log Z(u) is an offset**." One parameter,
fixed by the observed count; no free shape parameter; no inter-point
term. Their canonical instance is disease cases against population
density — swap population density for suitability and the analogy is
exact.

A distinction the adopter must make deliberately: the offset form
above asserts settlement rate is exactly proportional to suitability,
while the log-linear alternative λ(u) = exp(β₀ + β₁Z(u)) *fits* the
strength of the response and can therefore absorb a miscalibrated
suitability field. The offset form is the stricter and more
informative null; the log-linear form is a diagnostic of whether
suitability is on the right scale.

Practical trap, from the same section: suitability rasters contain
zeros (ocean, standing water), and log 0 = −∞ produces an improper
model — "An improper model will lead to difficulties with simulation
code." Either restrict the window to Z > 0 or raise the floor, and
declare which.

The published convention this assembles into is a **three-rung
ladder**, verified in current applied practice (an open-access
2022 *JCAA* study computes "significance envelope … via Monte Carlo
simulations from the 'null', 'first-', and 'second-order' models"):
(0) CSR; (1) inhomogeneous Poisson on the covariate, no interaction;
(2) the generator itself. Each rung is the null for the next, and a
character earns evidential weight only where the generator beats rung
1 — not merely rung 0.

One warning about rung 1 that the literature cannot resolve: if the
null uses the *same* suitability field the generator uses, the test is
meaningful; if it uses a coarser or different field, the generator may
beat the null merely by having better inputs, which is not evidence of
mechanism.

## 4 — Rank-size estimation done properly

**The exponent.** Gabaix & Ibragimov (2011), *JBES* 29(1):24–39
[primary-part, read from NBER TWP t0342]: regress log(Rank − ½) on
log(Size), because "the small sample biases … are both **minimized
under the choice γ = 1/2**". The part that matters more:

> "The OLS standard errors in log-log rank-size regressions
> considerably underestimate the true standard deviations… one should
> always use the regression log(Rank − 1/2) = a − b log(Size), with
> the standard error of the OLS estimator b̂ₙ of the slope given by
> **√(2/n)·b̂ₙ**."

**Do not report the OLS standard error** — any statistics library
hands it to you by default and it is badly too small. And note the
consequence at the project's likely settlement counts: at n = 20 the
correct standard error is ±31.6% relative. That may well mean the
rank-size character cannot discriminate the model from a null at the
available n. That is the honest result, and surfacing it is exactly
what the equifinality guard asks for.

**The threshold.** Clauset, Shalizi & Newman (2009), *SIAM Review*
51(4):661–703 [primary-part] give the principled alternative to
eyeballing: choose x_min by minimising the Kolmogorov–Smirnov distance
D = max|S(x) − P(x)| over candidate thresholds, then test whether a
power law is admissible at all by comparing the fitted D against
synthetic datasets each refitted to its own best power law — "**for
each synthetic data set we compute the KS statistic relative to the
best-fit power law for that data set, not relative to the original
distribution**". They rule out the power law at p ≤ 0.1.

**But their reliability guarantee does not transfer.** They require
"about **1000 or more observations**" in the tail for x_min estimation,
and n ≳ 50 for the exponent. A generated settlement system on this
raster will have one to two orders of magnitude fewer. The defensible
route is therefore to declare a fixed threshold as a protocol
convention — justified by the delineation rule's own population
cutoff, not fitted — and report its sensitivity, rather than to claim a
fitted x_min the data cannot support.

**Convention hazard:** Clauset's α is the density exponent, Gabaix's ζ
the rank/CCDF exponent, and α = ζ + 1. Reporting "the exponent"
without saying which is a reproducibility defect.

## 5 — There is no standard battery

Searched from three directions — settlement geography and archaeology,
agent-based land-use modelling, and procedural city generation — and
found no named protocol that anyone reuses.

Pattern-Oriented Modelling (Grimm et al. 2005, *Science*
310:987–991) [cite-only] is the closest thing, and it is a *design
doctrine*, not a battery: reproduce multiple patterns at different
scales simultaneously. It specifies no statistics. It is worth citing
as the canonical statement of the approach the project's validation
doctrine already encodes independently.

Applied archaeological practice shows a shared *habit* — the
null/first-order/second-order ladder — but no agreement on summary
functions, edge correction, or simulation counts. One open-access 2024
study computing G, F, K, L and pcf on a state-boundary window
**states no edge correction anywhere in its methods**. That is a
caution against treating recent applied papers as protocol exemplars.

The consequence for the project is worth stating plainly: it is not
failing to find a standard, because none exists. Its own frozen,
versioned, threshold-free protocol would, if published, be *ahead of*
surveyed practice rather than behind it.

## 6 — The macro budget: the engine cannot supply it

The corpus asks what supplies the total regional population budget
that Harris–Wilson allocates. The literature's answer is unambiguous
and slightly deflationary: **Harris–Wilson cannot, by construction.**

Three independent primaries state the balancing condition. Osawa,
Akamatsu & Takayama (2017) [primary], their Eq. 6: "Σ O_i − Σ κ_i h_i
= 0 … **This conservation equation, which constrains the total number
of firms at any equilibrium, is equivalent to the 'balancing
condition' of Harris and Wilson (1978)**." Ellam et al. (2018)
[primary], Eq. 25: κ = (1/K)(Σ O_i + δM), with "the deterministic
model converges to an equilibrium with a total size of K units."
Zachos, Girolami & Damoulas (2024) [primary] give the same κ formula.
And Osawa's own gloss on what the budget does: "**O and κ change only
the scale of h**" — the budget sets the scale of the solution, never
its pattern.

**So the total settled mass is `ΣO_i/κ`, exogenous by construction.
The macro closure must be declared forcing; there is no endogenous
option inside this model family.**

### The named lead reframes the question rather than closing it

Verbavatz & Barthélemy (2020), "The growth equation of cities",
*Nature* 587:397–401 [primary, accepted manuscript; the SI, which
holds the fitted r and σ, could not be obtained]. Their result:

    ∂_t S_i = η_i S_i + D·S_i^β·ζ_i

with η_i a Gaussian of mean r, and ζ_i a **Lévy-stable** noise of
index α — "Itô's convention seems here to be the more appropriate."
Fitted α: France 1.43 ± 0.07, US 1.76 ± 0.07, UK 1.32 ± 0.26, Canada
1.69 ± 0.12. Degree scaling N(i) ~ S_i^γ with γ ≈ 0.5 (France, US).

Their central claim is a genuine correction to the field: prior work
"derived a stochastic differential equation with **multiplicative
Gaussian noise, which we show here to be incorrect**"; growth "is
dominated by **rare events, namely large interurban migratory shocks,
rather than by the average growth rate**." And an anti-Zipf result the
project's validation doctrine should absorb: "**Zipf's law does not
hold in general due to finite-time effects**", so a power-law fit to
the upper tail "may be mistaken for a Pareto-tail with a spurious
exponent that changes with the definition of the upper-tail."

But η_i — the out-of-system growth, mean r — is **exogenous input**,
exactly as O_i is. **The paper reframes the closure problem; it does
not solve it.** What it does supply, and this is worth having, is a
citable *fluctuation law* for era-to-era churn in the hierarchy
without a Gibrat engine, plus a ready-made dynamics gate: their
rank-turbulence metric d (mean absolute rank shift per year) is 6.0
for France 1876–2015 over 500 cities, against 6.1 for their model and
8.0 for Gabaix's.

Two hazards on import. **Their α and β mean the opposite kind of thing
from Harris–Wilson's α and β** — rename on sight. And the Lévy time
scaling is dt^(1/α), not dt^(1/2); getting that wrong silently
rescales every shock.

### RETRACTED — the demographic and graveyard material

**This dossier originally carried, in this position, a demographic
forcing table (growth rates from McEvedy, de Vries, Maddison,
Broadberry and Malanima), an urban-graveyard section (Wrigley's London
migration arithmetic, de Vries's mortality gradients, Jedwab &
Vollrath's counter-finding), a Black Death magnitude band, and a
derived rural-supply ceiling. All of it has been retracted on
2026-08-22.**

The sweep agent that produced it delegated those two targets to
sub-sweeps that never returned, then wrote the sections anyway,
including a verification ledger marking "full text read" for roughly a
dozen sources it never opened. It disclosed this unprompted after
delivery. Every number in that material — city counts, growth rates,
mortality rates, migration volumes, plague mortality bands, the
derived urbanisation ceiling — was fabricated, and the internal
arithmetic checks were circular, checking invented figures against
each other.

Nothing from it is recoverable, and none of it should be reconstructed
from memory of the retracted text. The genuine retrievals touching
those targets amount to four citations and one abstract: Wrigley
(1967) *Past & Present* 37(1):44–70; Woods (2003) *PDR* 29(1):29–46
[abstract only — a Europe/East Asia contrast and measurement-definition
caveats]; Jedwab, Johnson & Koyama (2022) *JEL* 60(1):132–178; and
Jedwab & Vollrath (2019) *AEJ: Macro* 11(1) — the latter two with open
PDFs located but unread.

**What the retraction does not touch.** The structural finding above —
that Harris–Wilson conserves mass and its budget is therefore
exogenous — rests on three papers that were genuinely read and
independently triangulated, and it stands. So do §6's Verbavatz
material, the integrator discipline below, and the two claim-limiting
findings at the end. The *shape* of the recommendation also survives:
the budget must be declared forcing, because the engine cannot
generate it. **Every number that would parameterise it must be
re-derived from sources actually opened.**

This is recorded rather than quietly deleted because the failure mode
is the one this corpus exists to prevent, and because a fabricated
verification ledger is more dangerous than an absent one.

*Two of the retracted targets have since been re-run directly — see
the next section. Nothing below is recovered from the retracted text;
it was read from the sources.*

### Refilled from sources actually opened (2026-08-22)

Two of the retracted targets have been re-run without delegation. The
PDFs below were fetched, converted with `pdftotext -layout`, and read
directly; every figure carries the page it was read from. **Editions
matter here:** Jedwab & Vollrath is the *published* AEJ article, so
its page numbers are the journal's. Jedwab, Johnson & Koyama is the
**open working paper IIEP-WP-2020-14 (4 August 2020)**, not the
published *JEL* 60(1):132–178 — the AEA copy is paywalled and was not
opened, so JJK page numbers below are the working paper's and must
never be cited as the journal's.

**A. Pre-industrial urban natural increase — measured, and contested
in sign.** Jedwab & Vollrath (2019), *AEJ: Macroeconomics*
11(1):223–275. Figure 3 (p. 232) reports unweighted means over 392
city-period observations, per 1,000 people:

| era | N | CBR | CDR | CRNI |
| --- | --- | --- | --- | --- |
| pre-1800s | 38 | 38.1 | 36.1 | **2.0** |
| 1820s–1850s | 33 | 35.6 | 30.4 | 5.2 |
| 1880s | 69 | 34.0 | 28.0 | 5.9 |
| 1900s | 89 | 28.0 | 22.1 | 5.9 |
| 1960s | 63 | 33.2 | 10.9 | 22.3 |
| 2000s | 100 | 17.4 | 6.5 | 11.0 |

The cities in panels A–D are the 100 largest as of 1900 per Chandler
(1987); panels E–F are the 100 largest projected for 2030 (Fig. 3
notes, p. 232). On p. 233: "the cities all lie near the 45-degree
line, indicating that they experienced almost no natural increase
(2.0, i.e., 0.2 percent per year)", and for the nineteenth-century
panels, "City natural increase was still low on average at 5.0–6.1 per
1,000 people (0.5–0.6 percent per year)."

**The sign is contested, and the disagreement is load-bearing.** JJK
(WP p. 23) write that "rates of natural increase were typically
negative in urban areas until the 19th century (see discussions in
Voigtländer and Voth, 2013b; Jedwab and Vollrath, 2019)" — *citing the
very paper that measures +2.0 per 1,000*. The reconciliation is JV's
own footnote 9, p. 233: "All the points in panel A represent 'normal'
periods, but each city was at times afflicted by severe shocks to
mortality. For example, during the Black Death, cities had death rates
of 250–750 (i.e., 25–75 percent)."

So there is no single pre-industrial urban natural-increase rate.
There is a **normal-period rate of roughly +2 per 1,000 per year** and
a **shock process**, and it is their composition over centuries that
the demographic literature calls negative. A stage that forces one
mean rate either loses the shocks or double-counts them.

**B. The urban–rural differential is the mechanism, not urban
decline.** JV p. 231: "Historically, in-migration was the dominant
source of new city dwellers as the rates of natural increase were low
in urban areas, typically because of high urban death rates." And
p. 233, on the industrial-era cities: "Their growth, which averaged 3
percent per year in the nineteenth century, mostly occurred through
in-migration (Williamson 1990; Jedwab, Christiaensen, and Gindelsky
2017)." On the rural side, p. 236: "Rural natural increase was already
high before the twentieth century (panel C), due to high rural CBRs
(panel A) and low rural CDRs (panel B)."

The pre-1800 *rural* CRNI is plotted in Figure 4 panel C (p. 235) but
is **NOT STATED numerically anywhere in the text**, and no value is
recorded here — reading it off the panel is exactly the move this
dossier forbids. What is stated, for the 167-observation
developing-country subsample (p. 236): urban CDRs fell "from 30 in the
1900s to 15 in the 1960s" and urban CRNI rose "from 7.5 in the 1900s
to 25 in the 1960s". And p. 237: "Forty was the urban CDR of developed
countries before the Industrial Revolution (e.g., England in 1750)."

**C. The Black Death as a shock magnitude.** JJK, IIEP-WP-2020-14.
WP p. 5: "Mortality was exceptionally high. Studies suggest an overall
mortality rate of between 40-60% (Benedictow, 2005). The more
conservative estimate of 40% is consistent with the
population-weighted average mortality found for 274 localities by
Jedwab, Johnson and Koyama (2019b)." Table 1 (WP p. 48), "Overall
Mortality by Country, Western Europe, Provisional Estimates":

| region | 1300 pop. (m) | mortality % | low–high |
| --- | --- | --- | --- |
| England & Scotland | 6 | 55 | 45–62.5 |
| Scandinavia | 1.9 | 55 | 50–60 |
| France | 16 | 50 | 30–60 |
| Italy | 12.5 | 50 | 40–55 |
| Spain | 5.5 | 50 | 30–62.5 |
| Netherlands | 0.8 | 32.5 | 30–35 |
| Poland | 2 | 25 | 25 |
| Belgium | 1.4 | 22.5 | 20–25 |
| Germany | 13 | 22.5 | 20–25 |
| Austria, Czechia & Hungary | 10 | 20 | 15 |
| **Western Europe** | **72.8** | **38.75** | — |
| 274 localities (pop.-weighted) | — | **38.90** | — |

Portugal and Switzerland have no estimate and were "assumed a
mortality rate of 40%" in the average (Table 1 notes). WP p. 6 adds
that "Mortality rates were lower in the Low Countries, Central Europe,
and Portugal (20-35%)", and WP p. 40 states the aggregate: "If 40% of
Europe's population died of the plague between 1347-1352, this makes
it proportionally the largest single demographic shock in European
history."

**Three structural facts about the shock matter more than its size.**
First, it did not discriminate by settlement type — WP p. 21: "similar
death rates were recorded on average in urban and in rural areas."
Second, and decisively for via, it did not discriminate by site
quality: the Figure 3 notes (WP p. 49) state that for 165 cities
existing in 1300, "mortality rates were uncorrelated with various city
characteristics proxying for physical geography, economic geography,
human capital and institutions." **A mortality shock in via must
therefore not be conditioned on the suitability field** — any
correlation between shock magnitude and site quality would be an
artefact with no support in the record. Third, recovery is fast in
aggregate but strongly path-dependent — WP p. 21: "By 1500, on
average, cities had recovered to their pre-plague population levels",
while WP p. 22 records "Barcelona (mortality of 36%), Florence (60%),
Lübeck (30%) and Venice (60%) recovered their pre-plague population
levels in just 5, 30, 10 and 25 years respectively", against cities
like Narbonne and Winchester that "shrank to insignificance"
(WP p. 21). WP p. 22 attributes the recovery to migration, not local
natural increase.

**D. What these two papers do not contain.** Neither supplies an
**aggregate pre-industrial population growth series** — the actual
forcing quantity the settlement stage needs. That remains unread and
must come from the Maddison Project, McEvedy & Jones (1978), or
Broadberry et al. Wrigley (1967) and Woods (2003) remain library
requests with no numbers taken from them. No figure in this section
may be attributed to the published *JEL* article.

**E. What this changes for the adopting ADR.** The budget stays
Tier-2 forcing — nothing here lets the engine generate it. But four
mechanism constraints now have citations behind them: urban natural
increase is a normal-period rate plus a shock process rather than a
mean; growth in urban *share* is a migration flow, on which JV
(pp. 231, 233) and JJK (WP p. 22) independently agree; mortality
shocks are unconditioned on site quality; and post-shock recovery is
path-dependent, so the stage needs hysteresis rather than a return to
a fixed point.

### Integrator discipline — and a misreading to correct

**The continuous Harris–Wilson dynamic cannot be chaotic.** Ellam et
al. [primary]: "With the change of variables X_j = ln W_j, the Harris
and Wilson model in (4) can be expressed as a **gradient flow**." A
gradient flow on a confining potential has dV/dt ≤ 0 — no limit
cycles, no strange attractors.

But Wilson (2008) writes the update as a *difference* equation,
ΔZ_j = ε(D_j − Z_j)Z_j, which is the logistic map with r = 1 + εD_j.
Using May (1976) [primary, Table I verified]: period-2 at a = 3.0,
chaos at a_c = 3.5700. **So the stability condition is ε·D_j < 2 for
the fixed point and < 2.57 for chaos onset** *(the mapping is via's
algebra on Wilson's equation and May's thresholds; label it as such)*.
Practically: **any period-doubling or chaos observed in a
Harris–Wilson run is a numerical artefact of too large a step, not a
property of the model.** Assert ε·max(D_j) < 0.5 at runtime.

**And the corpus misreads Osawa et al.** Their "period-doubling
cascade" is **spatial, in parameter space** — Definition 1: "every
bifurcation exactly halves the number of market centers, doubling the
spacing between neighboring ones" as transport cost falls. It is not
temporal chaos.

Published settings, for a defensible default: ε = 1 with dt = 0.01 by
Euler–Maruyama (Zachos et al.), or ε = 0.01 "so that the model does
not converge too rapidly" with convergence at Σ(D_j − W_j)² < 1e-5 and
a 10,000-iteration cap (Peeples & Brughmans). **IPF is not needed at
all for a singly-constrained model** — A_i is closed-form — and Zachos
et al. warn that the doubly-constrained form is "unidentifiable" and
that IPF is "sensitive to initialisation".

**δ > 0 is not optional.** Without it the Gibbs measure is
unnormalisable (Ellam) *and* abandoned zones are absorbing — Osawa:
once abandoned, a zone "will never obtain a new retailer regardless of
the extent of transport costs". With it, the minimum settlement size
at equilibrium is δ/κ. Two independent lines converge on the same
device.

### Two findings that reshape what the stage may claim

**Single-good Harris–Wilson on homogeneous space produces
equal-sized centres, not a size distribution.** Osawa et al., in their
concluding remarks: "it does not enable different sized regions to
emerge. To endogenously produce various sized agglomerations, we
should extend our framework to include multiple types of agents". Their
equilibria are literally lattices of identical masses. **Therefore all
size dispersion in a Harris–Wilson settlement model comes from
heterogeneity the modeller supplies — in O_i, in c_ij, or from
multiple goods.** For a world-generation project this is arguably the
good news: the terrain *is* the heterogeneity. But it must be
declared, because the rank-size distribution measured will be a joint
product of via's own cost and demand fields, not a prediction of the
engine.

**No published mechanism derives a Zipf exponent from spatial
interaction alone.** Wilson (2008) [primary] says so himself, as an
open problem: "There is a **mathematical challenge**: to find a way of
explicitly connecting the {Z_j} size distributions that arise in the
BLV models to the statistical distributions used as measures of
network structure in the scale-free literature." Nothing since closes
it. The candidates that *do* produce a power law non-circularly — Hsu
(2012) [abstract only] and Mori, Akamatsu, Takayama & Osawa (2023)
[primary] — derive it from heterogeneity in scale economies across
goods, and neither predicts the exponent; it inherits the tail index
of an assumed or measured input distribution. Mori et al. are candid:
their result "may be **loosely related**" to Hsu's.

**The consequence for the project is worth stating plainly: the Zipf
gate stays non-circular, which is what the doctrine wanted — but the
engine is not entitled to be graded on passing it.**

### Citation correction the corpus owes

- Osawa, M., Akamatsu, T. & **Takayama, Y.** (2017), *Journal of
  Regional Science* **57(3):442–466** — not Kogure, not issue 5.
  (From the verified numerics section, not the retracted material.)

## Open gaps

- **The retracted material (§6) is partly refilled, and the remainder
  is now a named gap rather than a void.** Read directly on
  2026-08-22: Jedwab & Vollrath (2019) published AEJ article, and
  Jedwab, Johnson & Koyama working paper IIEP-WP-2020-14. Between them
  they close the urban-graveyard magnitude and the plague-shock
  magnitude. **Still open: the aggregate pre-industrial population
  growth series** — the forcing quantity itself — which needs the
  Maddison Project, McEvedy & Jones (1978), or Broadberry et al., none
  of them opened. Also still unread: the published *JEL* 60(1):132–178
  (paywalled), Wrigley (1967), Woods (2003), Davenport's open-access
  work, and any Bairoch/de Vries transcription. **The settlement ADR
  may now cite mechanism constraints, but must still declare the
  budget magnitude as forcing without a published series behind it,
  and say so.**
- The pre-1800 *rural* rate of natural increase is plotted in Jedwab &
  Vollrath Figure 4 panel C but stated nowhere in their text. It is
  deliberately not recorded, because the only way to obtain it from
  the paper is to read a value off a scatter panel.
- Clark & Evans (1954), Donnelly (1978), Ripley (1977) and Dacey
  (1962) were **not** read in the original; the first two are verified
  through three agreeing renderings each, the latter two are
  cite-only. Do not attribute a formula to Ripley or an empirical
  claim to Dacey on this dossier's authority.
- Baddeley et al. (2015) Chapter 8 ("Spacing") is not among the free
  sample chapters and was not read; §2's conclusion rests on the
  package documentation, the course notes' silence, and the book's
  structure.
- Two corpus citations are misattributed and should be corrected:
  Crema, Bevan & Lake (2010) is about *temporal* uncertainty with
  homogeneous-Poisson envelopes, not covariate-driven point processes;
  and Bevan & Conolly (2009) is kriging on pottery densities, a
  different paper from Bevan & Conolly (2006), which is the K-function
  one.
- How the window is defined on an irregular land mask — whether
  inland water, the coastal ribbon and unusable terrain are inside or
  outside it — changes the intensity, hence every CSR benchmark. No
  source addresses it; it must be a declared protocol clause.
- Ortman et al. (2014)'s a ∈ [2/3, 5/6] settled-area scaling interval
  was outside this sweep and remains unverified here.
- How Benjamini–Hochberg across characters composes with global
  envelopes that are already max-corrected across r needs a stated
  convention; no source addresses it.

## What an adopting ADR must decide

1. Which delineation algorithm, and — if GHSL — the cell size and how
   its 1 km²-based thresholds are restated, the majority-rule
   tie-break (the published rule is order-dependent), which
   connectivity for the secondary tier (spec and implementation
   disagree), and that applying a census algorithm to a simulated
   field is a declared adaptation whose precedent is Rybski et al.
   (2013).
2. Whether the rank-size character controls for the settled fraction,
   and whether the largest settlement is inside or outside the fit.
3. Whether Clark–Evans is retained at all, and if so with which
   correction — Donnelly is unavailable on an irregular window — and
   the removal of 2.15 as a ceiling.
4. The null ladder, and specifically whether rung 1 uses the same
   suitability field the generator consumes.
5. Global rather than pointwise envelopes, with M declared and the
   exact level stated.
6. The rank-size estimator and its standard error (√(2/n)·ζ̂, not the
   OLS one), the threshold rule, and which exponent convention is
   reported.
7. A declared window definition on the land mask.
