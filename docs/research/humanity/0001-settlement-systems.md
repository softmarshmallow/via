# Research 0001 — Settlement systems (regional scale)

Status: research corpus — pre-decision. Nothing in this document is
adopted; adoption of any mechanism requires its own ADR. Part of
docs/research/humanity (see README there).

## The question this literature answers for via

Given a suitability field and a travel-cost surface from the corridor
layer, what makes settlements form, grow, die, space themselves, and
stack into a hierarchy — and which statistics of the result are
empirically constrained enough to gate on. The literature spans
premodern periodic markets through 19th-century frontier growth to
modern metropolitan systems; era enters almost everywhere as
parameters (travel cost, demand density, agglomeration returns), not
as structural change.

## Models and mechanisms

### Central Place Theory (classical)

Christaller, W. (1933) Die zentralen Orte in Sueddeutschland, Gustav
Fischer, Jena (transl. Baskin 1966, Prentice-Hall); Loesch, A. (1940)
Die raeumliche Ordnung der Wirtschaft (transl. The Economics of
Location, Yale UP 1954). Verified as standard citations via secondary
literature (Parr 2017, Rev. Urban & Regional Dev. Studies).

Each good has a *range* (max distance consumers travel for it) and a
*threshold* (min demand to sustain a supplier). On an isotropic
plain, competition packs suppliers into hexagonal market areas;
larger-threshold goods come from fewer, more widely spaced centers,
producing a nested discrete hierarchy (Christaller K=3 marketing,
K=4 transport, K=7 administrative). Loesch generalizes to overlapping
hexagonal nets of different mesh per good — sector-structured
"economic landscapes" with a more continuous size distribution.

As code: descriptive-only in original form — a static equilibrium
geometry with no update rule on an isotropic featureless plain, which
via explicitly does not have. The threshold/range primitives are
simulable: state = per-cell purchasing power (suitability ×
population); a candidate center survives iff demand integrated within
a travel-cost radius on the actual terrain/corridor cost surface
exceeds threshold; iterate entry/exit to a fixed point. Used that way
it becomes a component of a forward model (see Harris–Wilson,
Fujita–Krugman–Mori), and its geometric predictions (spacing
regularity, hierarchy nesting) become gates rather than mechanisms.

Era dependence: threshold and range are directly era-parameterizable
(range = travel speed × acceptable travel time; threshold =
per-capita demand × density). Era enters as parameters, not
structural change — except that premodern systems share central
functions in time via periodic markets (see Skinner), which classical
CPT with permanent centers cannot represent.

### Dynamic self-organizing central place models (Allen–Sanglier)

Allen, P.M. & Sanglier, M. (1979) 'A Dynamic Model of Growth in a
Central Place System', Geographical Analysis 11(3):256-272; part II
in Geographical Analysis 13 (1981); Allen & Sanglier (1981) 'Urban
evolution, self-organization and decision-making', Environment and
Planning A. Verified via Wiley.

Brussels-school (Prigogine) dissipative-structure model. Population
at each lattice point grows logistically with carrying capacity set
by local employment; employment per function grows where demand
captured from surrounding population (gravity-type competition
between centers) exceeds costs. Positive feedback (jobs attract
people attract jobs) plus spatial competition differentiates a
near-uniform lattice into a hierarchy of centers; small early
fluctuations select *which* sites win — path dependence is intrinsic.

As code: fully forward. State per cell: population x_i, employment
per function s_ij. Coupled ODEs: dx_i/dt = b·x_i(1 − x_i/(N +
Σ_j c_j s_ij)) − m·x_i; ds_ij/dt = α·s_ij(D_ij − s_ij), demand D_ij
captured via distance-cost-discounted competition against all other
centers. Deterministic given initial conditions; the "fluctuations"
are seedable perturbations, or supplied deterministically by via's
suitability field — the natural replacement for their random seeding.
Inputs from via: per-cell carrying capacity/suitability, travel-cost
matrix from the corridor layer. Timestep-integrate; bitwise
determinism straightforward.

Era dependence: parameters only — distance-decay of demand capture
(transport tech), number/mix of functions, birth/migration rates,
economies-of-scale coefficients. Equation structure is era-invariant;
adding/removing functions over time (rail-dependent industry in the
19th century) is exogenous forcing.

### Entropy-maximizing spatial interaction (gravity) models

Wilson, A.G. (1970) Entropy in Urban and Regional Modelling, Pion,
London; Wilson (1967) 'A statistical theory of spatial distribution
models', Transportation Research 1:253-269. Standard citations,
verified via secondary literature (Osawa et al. 2017, J. Regional
Science).

Flows T_ij are the most probable (maximum-entropy) configuration
subject to constraints on trip totals and total travel cost:
T_ij = A_i O_i B_j W_j^α exp(−β c_ij) (production-constrained form);
α = returns to destination attractiveness/agglomeration, β = distance
deterrence. Replaces the Newtonian gravity analogy with a
statistical-mechanics derivation; A_i, B_j are balancing factors
computed by iteration.

As code: forward as the "fast" layer of a two-speed system. Given
attractiveness W_j and cost matrix c_ij (from via's corridor/network
layer), balance flows by deterministic fixed point (Furness/IPF
iteration — converges, bitwise-reproducible with fixed iteration
count/tolerance). Equilibrium-only by itself: it allocates flows for
a frozen configuration and says nothing about how W_j changes; must
be paired with Harris–Wilson slow dynamics for growth and death.

Era dependence: β is the era knob — high β (walking/cart) localizes
flows and supports many small centers; falling β (rail, car)
integrates and concentrates the system. α rises with era as
retail/service scale economies grow. Pure parameter dependence; c_ij
itself evolves as via's network layer builds roads and rail.

### Harris–Wilson center dynamics (Boltzmann–Lotka–Volterra)

Harris, B. & Wilson, A.G. (1978) 'Equilibrium values and dynamics of
attractiveness terms in production-constrained spatial-interaction
models', Environment and Planning A 10(4):371-388. Verified; see also
Osawa, Akamatsu, Kogure (2017) J. Regional Science 57(5)
(period-doubling analysis) and Ellam et al. (2018) Proc. R. Soc. A.

Couples the entropy-maximizing flows (fast) to slow center-size
dynamics: revenue D_j = Σ_i T_ij with T_ij ∝ W_j^α exp(−β c_ij);
size evolves as dW_j/dt = ε(D_j − k W_j) — grow if revenue exceeds
cost, shrink and die otherwise. For α > 1 the uniform state is
unstable and the system self-organizes into few large centers; α and
β jointly control number, size, and spacing of survivors, with
discrete jumps (bifurcations) as they vary. The canonical "centers
grow and die" model.

As code: fully forward, and per the source sweep the single
best-fitting mechanism for via's doctrine. State: W_j per candidate
site (grid cells or graph nodes), population field O_i from
suitability. Loop: (1) balance flows by IPF given current W; (2)
Euler/RK step on dW_j/dt = ε(D_j − k W_j); repeat. Deterministic
ODEs, no stochasticity required; seedable perturbations optional.
Inputs: via's suitability field (O_i and site cost k_j can be
terrain-modulated), travel-cost matrix c_ij from corridors, updated
as the network layer evolves — genuine terrain→hydrology→corridor→
settlement causality. Caution: multiple equilibria and (for large ε)
period-doubling/chaos (Osawa 2017) — gate on distributional
statistics, not exact configurations.

Era dependence: same equations; era = (α, β, ε, k) trajectory plus
the evolving cost matrix. Medieval: high β, α near 1 (many small
markets); 19th-century frontier: falling β along rail lines, site
costs from the land-survey grid as exogenous forcing; modern: low β,
high α (few dominant centers, big-box dynamics).

### New Economic Geography core-periphery (Krugman) and extension

Krugman, P. (1991) 'Increasing Returns and Economic Geography',
Journal of Political Economy 99(3):483-499; Fujita, M., Krugman, P.,
Venables, A.J. (1999) The Spatial Economy, MIT Press; Fujita, M.,
Krugman, P., Mori, T. (1999) 'On the evolution of hierarchical urban
systems', European Economic Review 43:209-251. All verified.

Micro-founded agglomeration: Dixit–Stiglitz monopolistic competition,
iceberg transport costs, factor mobility. Firms locate near large
markets (backward linkage); workers near firms for cheap variety
access (forward linkage); immobile agriculture pulls toward
dispersion. Agglomeration vs dispersion depends on transport cost τ,
manufacturing share μ, elasticity of substitution σ, with
catastrophic (tomahawk) bifurcations as τ falls. Fujita–Krugman–Mori
1999: as population grows on a line economy with multiple industries,
new cities emerge endogenously at frontier locations and a
central-place-like hierarchy self-organizes — the modern emergent
reformulation of Christaller.

As code: forward with effort. The instantaneous equilibrium (wages,
prices given worker distribution λ_r) is a nonlinear fixed point
solved by iteration; slow dynamics are ad hoc replicator migration
dλ_r/dt = γ λ_r(ω_r − ω̄) toward higher real wage. Loop: solve wage
equations on via's region graph, step migration, repeat. State:
worker share per region, per industry. Inputs: transport costs from
the corridor/network layer, agricultural productivity from
suitability. Caveats: fixed-point solves are expensive and
multi-equilibrium; the literature is almost entirely 2-region or
racetrack, so porting to a real heterogeneous landscape is genuine
research, not implementation. Its main value to via may be as citable
justification for an α > 1 agglomeration term in the cheaper
Harris–Wilson layer.

Era dependence: era = τ (falls monotonically across the three target
eras), μ (near 0 medieval, rising through the 19th century,
service-dominated modern), σ. The theory's headline result — era-like
change in τ produces qualitative regime shifts from unchanged
mechanisms — is the cleanest theoretical warrant for via's
one-mechanism-many-eras doctrine.

### Zipf's law / rank-size rule with Gabaix random-growth explanation

Zipf, G.K. (1949) Human Behavior and the Principle of Least Effort;
Gabaix, X. (1999) 'Zipf's Law for Cities: An Explanation', Quarterly
Journal of Economics 114(3):739-767 (verified); empirics: Rosen, K. &
Resnick, M. (1980) J. Urban Economics 8:165-186; Soo, K.T. (2005)
'Zipf's Law for cities: a cross-country investigation', Regional
Science and Urban Economics 35(3):239-263 (verified); counterpoint:
Eeckhout, J. (2004) 'Gibrat's Law for (All) Cities', American
Economic Review 94(5):1429-1451.

Empirical claim: city sizes follow a power law P(S > s) ~ s^−ζ with ζ
near 1 (rank ~ 1/size). Gabaix: if cities grow randomly with common
mean and variance independent of size (Gibrat's law), plus any small
friction preventing shrinkage to zero (lower reflecting barrier), the
distribution converges to Zipf with ζ → 1 exactly; ζ deviates from 1
when growth deviates from Gibrat (size-dependent variance gives
ζ = 1 only in the tail).

As code: not a generator — an emergent target. Implementing
"multiplicative random shocks" as the growth engine would be an
authored outcome, violating via's mechanism doctrine. The right use:
run the mechanistic growth layer (Harris–Wilson / Allen–Sanglier on
the suitability landscape) and test whether (a) realized growth rates
satisfy Gibrat and (b) the upper-tail rank-size exponent lands in the
empirical band; Gabaix's theorem guarantees the two gates are
mutually consistent. If stochastic forcing is wanted (harvest shocks,
epidemics), it is seedable-PRNG exogenous forcing in the forcing
tier, and Zipf remains an emergent check.

Era dependence: Zipf-like tails are reported from medieval and even
ancient settlement systems, so the gate is roughly era-invariant in
exponent but not in scale (total urban population and size floor
shift enormously). Convergence to ζ ~ 1 requires long random-growth
transients — young systems (19th-century frontier mid-settlement)
should be gated more loosely, or on the lognormal body (Eeckhout)
rather than the tail.

### Urban scaling laws (Bettencourt et al.)

Bettencourt, L.M.A., Lobo, J., Helbing, D., Kuehnert, C., West, G.B.
(2007) 'Growth, innovation, scaling, and the pace of life in cities',
PNAS 104(17):7301-7306 (verified); Bettencourt, L.M.A. (2013) 'The
Origins of Scaling in Cities', Science 340:1438-1441 (derives
β = 5/6 infrastructure, 7/6 socioeconomic from settled-network
geometry).

Across a national urban system, aggregates scale as Y = Y0·N^β with
population N: socioeconomic outputs (GDP, wages, patents, crime)
superlinear, β ~ 1.1-1.3; infrastructure (road length, cable length,
paved area) sublinear, β ~ 0.75-0.9 (theory: 5/6 ~ 0.83);
individual-needs quantities (housing, jobs, water use) β ~ 1.
Bettencourt 2013 derives these from four assumptions: mixing
population, incremental network growth, bounded human effort, and
socioeconomic output proportional to local social interactions on the
infrastructure network.

As code: descriptive-only as stated — cross-sectional allometry, not
a dynamic process; must not be implemented as "assign city
GDP = N^1.15" (an authored look-table). Correct use: (1) gate —
measure road length, built area, and interaction volume in generated
cities across the generated size distribution, regress log Y on
log N, and require exponents in the empirical bands; (2) the 2013
derivation's ingredients (incremental network densification,
interaction-proportional output) are themselves mechanistic and
overlap with via's morphology/network layer, so the gate tests real
generated geometry, not a formula.

Era dependence: claimed remarkably era- and country-robust (scaling
found in medieval European and pre-Columbian data, e.g. Cesaretti et
al. 2016 PLoS ONE for medieval city budgets; Ortman et al. 2014 for
pre-Hispanic Mexico) — a rare cross-era gate. The prefactor Y0 is
strongly era/technology-dependent; the exponents approximately are
not. Boundary-definition sensitivity (see pitfalls) means the gate
must fix a consistent city-delineation algorithm.

### Periodic market systems / Skinner's standard marketing communities

Skinner, G.W. (1964-65) 'Marketing and Social Structure in Rural
China', Journal of Asian Studies 24(1):3-44, 24(2):195-228,
24(3):363-399 (verified). See also Stine, J. (1962) on Korea
(temporal CPT: firm mobility when range < threshold).

In premodern low-density economies, demand within one-day travel
range is below the threshold for a permanent shop, so central
functions are shared *in time*: markets meet on rotating schedules
(e.g. 1-4-7 / 2-5-8 / 3-6-9 in a 10-day cycle) with itinerant traders
circuiting between them. Skinner mapped a nested hierarchy (standard
→ intermediate → central markets) approximating Christaller lattices,
standard-market areas sized so the farthest villager can walk to
market and back in a day; the standard marketing area, not the
village, is the basic social unit. Modernization (density +
transport) raises demand past thresholds, markets become continuous,
and the periodic system collapses from the bottom up.

As code: forward-simulable as a regime of the same threshold/range
machinery. State = market sites with meeting frequency f_j in
[1/cycle ... daily]; frequency rises where captured demand per
meeting exceeds trader opportunity cost, falls otherwise; trader
circuits = shortest tours on the corridor graph. Inputs:
population/suitability field, walking-speed cost surface. The elegant
version: frequency is a continuous intensity variable of the
Harris–Wilson W_j, so "periodic vs permanent" is an emergent
threshold, not a coded era switch; outputs are continuous (frequency,
catchment), satisfying the no-taxonomy rule.

Era dependence: the era test case — the premodern regime differs from
modern not by mechanism but by demand density and travel cost falling
below the permanence threshold. If via's settlement layer reproduces
the periodic-to-permanent transition endogenously as density and
transport improve, the "one framework, three eras" claim is
demonstrated. The 19th-century American frontier partially skipped
periodicity (high per-capita demand, wagon/rail) — a discriminating
prediction.

### Radiation model of inter-settlement flows

Simini, F., Gonzalez, M.C., Maritan, A., Barabasi, A.-L. (2012) 'A
universal model for mobility and migration patterns', Nature
484:96-100 (verified).

Parameter-free alternative to gravity for commuting/migration flows:
T_ij = T_i · m_i m_j / ((m_i + s_ij)(m_i + m_j + s_ij)), with m_i,
m_j the origin/destination populations and s_ij the total population
inside the circle of radius r_ij around i (excluding endpoints).
Derived from a job-search absorption process; distance enters only
through intervening opportunities, so effective distance decay adapts
to the population landscape instead of being fitted.

As code: fully forward and cheap — deterministic closed form given
the population field and a distance (or travel-cost generalization),
no calibration, which suits via's no-tuned-magic doctrine for the
flows layer. No state of its own; a flow allocator like Wilson's
model. Use the expected-value (deterministic) form. Known weakness:
underperforms gravity at intra-urban scales and short ranges; best
used for inter-settlement commuting/migration flows feeding the
network layer.

Era dependence: nominally era-free (no parameters) — both its appeal
and its limit. Era enters only through the population landscape and
by replacing Euclidean radius with travel-cost isochrones from the
era's network. Its derivation assumes job-search-like behavior, so
premodern applicability (pilgrimage, marketing trips) is an untested
extrapolation — safer to use Wilson-with-β for premodern eras and
radiation as a modern-era cross-check.

## Gate candidates

- **Upper-tail rank-size (Pareto/Zipf) exponent ζ** (OLS or Hill on
  log rank vs log size, with Gabaix–Ibragimov rank−1/2 correction):
  ζ ~ 1.0 canonical; cross-country spread 0.81-1.96, mean ~1.14
  (Rosen–Resnick 1980, 44 countries); Soo 2005 (73-75 countries):
  mean 0.90 cities proper, 1.17 urban agglomerations; sensible gate
  band for a mature system: 0.8-1.3 depending on delineation.
  [Soo (2005) Reg. Sci. Urban Econ. 35(3):239-263; Rosen & Resnick
  (1980) J. Urban Economics 8:165-186; Gabaix (1999) QJE
  114(3):739-767]
- **Gibrat gate** — regression slope of settlement growth rate (and
  growth-rate variance) on log size: slope statistically
  indistinguishable from 0 for city systems in steady state; the
  process-level counterpart that guarantees the Zipf tail.
  [Gabaix (1999) QJE; Eeckhout (2004) AER 94(5):1429-1451; Ioannides
  & Overman (2003) Reg. Sci. Urban Econ. 33:127-137]
- **Full-distribution shape** — lognormal body with Pareto upper tail
  for ALL settlements (not just large cities): US Census places 2000
  approximately lognormal over ~5 orders of magnitude, power-law only
  in the extreme upper tail; gate: fitted lognormal σ ~ 1.75
  (Eeckhout's US estimate) and a detectable Pareto tail crossover.
  [Eeckhout (2004) AER 94(5):1429-1451]
- **Clark–Evans nearest-neighbor index R** for same-tier spacing
  (observed mean NN distance / expected under Poisson at same
  intensity): R ~ 1 (random) for full settlement sets; modest
  regularity R ~ 1.1-1.5 for higher-tier centers only (Dacey's
  Wisconsin tests found only weak upper-tier regularity; perfect
  hexagonal 2.15 is never observed, and the 2.15 bound itself is
  contested). [Clark & Evans (1954) Ecology 35:445-453; Dacey (1962);
  Philo & Philo (2022) Geographical Analysis 54:? '2.15 or Not 2.15?'
  (doi:10.1111/gean.12284)]
- **Urban scaling exponents β** (log-log regression of aggregate vs
  population across the generated system): socioeconomic outputs
  (interaction volume, GDP-proxy, innovation-proxy) β ~ 1.1-1.3
  (theory 7/6 ~ 1.17); network infrastructure (total road length,
  paved area) β ~ 0.75-0.9 (theory 5/6 ~ 0.83); individual needs
  (housing units, water) β ~ 1.0. [Bettencourt et al. (2007) PNAS
  104(17):7301-7306; Bettencourt (2013) Science 340:1438-1441;
  cross-era support: Ortman et al. (2014) PLoS ONE, Cesaretti et al.
  (2016) PLoS ONE (medieval)]
- **Distance-decay of inter-settlement flows** — fitted β in
  T_ij ~ W^α exp(−β c_ij) (or power-law exponent on distance), plus
  flow-prediction skill vs the parameter-free radiation baseline:
  power-law distance exponents for commuting/migration typically 1-3
  (region-dependent, ~2 common); two-part gate: (a) fitted decay in
  range for the era's travel cost, (b) generated flows correlate with
  the radiation prediction at the county-equivalent scale where
  Simini et al. report good agreement. [Wilson (1970) Entropy in
  Urban and Regional Modelling; Simini et al. (2012) Nature
  484:96-100; Osawa et al. (2017) J. Regional Science]
- **Hierarchy nesting** — ratio of counts of centers in successive
  levels (bumping ratio) and spacing ratio between levels: Christaller
  K in {3,4,7} idealized; Skinner's Sichuan data roughly 2-3 standard
  markets per intermediate market, standard-market spacing set by a
  one-day round-trip walk (order 4-8 km between periodic markets in
  dense agrarian China) — treat as an order-of-magnitude gate for the
  premodern era only, not a sharp number. [Christaller (1933);
  Skinner (1964-65) J. Asian Studies 24(1-3); Parr (2017) review]

## Pitfalls and contested claims

- Classical central place geometry fails statistical tests on real
  landscapes: hexagonal regularity is essentially never observed
  (Dacey 1962 found only weak upper-tier regularity), so gating on
  strong spacing regularity (Clark–Evans R near 2.15) would gate
  against reality; the oft-quoted 2.15 upper bound is itself
  mathematically contested (Philo & Philo 2022).
- Zipf universality is contested at exactly the level via would gate
  on: Soo (2005) rejects Zipf by OLS for 53 of 73 countries; the
  exponent shifts systematically with city delineation (0.90 cities
  proper vs 1.17 agglomerations) and with truncation point; Eeckhout
  (2004) shows the full settlement distribution is lognormal with
  only an extreme-tail power law. Gate on a band and on Gibrat
  process statistics, never on ζ = 1 exactly; fix the delineation
  algorithm before measuring.
- OLS on log rank vs log size has well-known small-sample bias and
  severely underestimated standard errors (autocorrelated ranks); use
  the Gabaix–Ibragimov (rank − 1/2) estimator or the Hill estimator,
  or the gate will produce false passes/failures.
- Urban scaling exponents are sensitive to city boundary definition:
  Arcaute et al. (2015, J. R. Soc. Interface) show superlinearity for
  England/Wales can vanish or invert under different clustering
  cutoffs, and Leitao et al. (2016) show fitted β depends on the
  assumed noise model. Any scaling gate must specify the delineation
  and fitting procedure as part of the gate.
- Harris–Wilson dynamics have massive equilibrium multiplicity, and
  for large step/response parameters exhibit period-doubling cascades
  into chaos (Osawa et al. 2017). Bitwise determinism is achievable
  but configurations are sensitive to initial conditions and
  parameters — gates must be distributional (exponents, spacing
  indices, size distributions), not map-configurational; and the
  integrator step size is a scientific choice, not just numerics
  (echoes via's dt lesson from the terrain solve).
- Equilibrium-only components (Wilson flow balancing, NEG
  instantaneous wage equilibria, classical CPT) cannot produce the
  observed path dependence of real settlement systems — locational
  "frozen accidents" persist long after their cause is gone (Bleakley
  & Lin 2012 QJE on portage sites). A framework that re-equilibrates
  from scratch each epoch will wrongly relocate cities when era
  parameters change; slow state (built capital, sunk infrastructure)
  must carry memory between eras.
- NEG's analytical results live on 2-region or racetrack geometries;
  porting Krugman/FKV to a heterogeneous suitability landscape with a
  real transport graph is research-grade work with expensive
  multi-equilibrium fixed-point solves. Budget for using NEG as
  citable justification for agglomeration terms (α > 1) inside
  cheaper Harris–Wilson dynamics rather than as the engine.
- Gravity-model parameters are not transferable across regions or
  eras (each empirical study refits α, β); declaring them as era
  forcing in config is legitimate under via's forcing tier, but they
  cannot be claimed as gated process constants. The radiation model
  removes the fitting but is derived from modern job search and
  underperforms at intra-urban scale — its premodern use is an
  extrapolation, not a validated mechanism.
- Gabaix's Zipf convergence needs a lower reflecting barrier and long
  transients; young systems (a 19th-century frontier a few decades
  in) will not have converged, so era-specific gates must loosen tail
  expectations for young systems and lean on the lognormal body /
  Gibrat statistics instead.
- Temptation to implement outcomes as mechanisms: sampling city sizes
  from a power law, assigning Y = N^1.15, or hard-coding hexagonal
  lattices would each be an authored look-table violating via
  doctrine — every statistic listed under gates must be emergent from
  threshold/range demand, flow allocation, and growth-death dynamics,
  or the gate is circular.
- Skinner's periodic-market work shows premodern marketing is
  time-shared central function provision — a different operating
  regime, though reachable by the same threshold/range mechanism. If
  the framework hard-codes "permanent settlement with continuous
  services", the medieval era will be structurally wrong even with
  correct parameters; market frequency should be a continuous state
  variable so periodicity emerges and collapses endogenously (Skinner
  documents the collapse under modernization).
