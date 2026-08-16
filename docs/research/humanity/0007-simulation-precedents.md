# Research 0007 — Validated simulation precedents

Status: research corpus — pre-decision. Nothing in this document is
adopted; adoption of any mechanism requires its own ADR. Part of
docs/research/humanity (see README there).

## The question this literature answers for via

Which published models have grown settlement, trade, urban-form, or
network patterns mechanistically on a physical substrate — and which
of their statistics survived contact with data. The precedents span
via's human layers (suitability -> settlements -> flows -> network ->
morphology) and run from prehistoric agriculturalists to 20th-century
road networks; several explicitly test whether one mechanism spans
eras under changed parameters, which is via's design thesis.

## Models and mechanisms

### Artificial Anasazi / Long House Valley household ABM

Axtell, R., Epstein, J.M., Dean, J.S., Gumerman, G.J., et al. (2002)
'Population growth and collapse in a multiagent model of the Kayenta
Anasazi in Long House Valley', PNAS 99(suppl.3):7275-7279. Verified.
Replication/critique: Janssen, M.A. (2009) 'Understanding Artificial
Anasazi', JASSS 12(4):13. Verified.

Household agents on a 96x180-cell gridded reconstruction of Long
House Valley (800-1350 CE). Each year every cell gets a maize yield
from an exogenous paleoclimate reconstruction (dendroclimatic PDSI +
alluvial groundwater level by landform zone). A household consumes a
fixed maize quantity, stores surplus with spoilage, fissions when a
daughter reaches marriageable age, and when harvest plus storage
falls below need relocates its farm plot to the nearest currently-
sufficient unfarmed cell and its dwelling to within 1 km of water.
Population trajectory and settlement map emerge; the ~1300 CE
abandonment is NOT fully reproduced — itself the headline finding
(environment alone insufficient).

As code: directly simulable and close to via's architecture. State =
per-cell yield field + household records (location, age, stock);
update = deterministic annual rule given the climate forcing series;
inputs = a productivity field from climate/hydrology (via already
produces the physical side) and a water-access map. All stochastic
draws (demographic rates) are seedable. The one non-mechanistic piece
is the calibrated demographic/harvest-variance parameters. Fully
compatible with "no AI inside the engine".

Era dependence: era enters only as forcing (climate series) and
technology constants (maize yield per ha, storage life, household
consumption). The structural rules are era-generic; changing era
means changing the suitability function and mobility radius, not the
code path.

### Village Ecodynamics Project (VEP)

Kohler, T.A. & Varien, M.D. (eds.) (2012) 'Emergence and Collapse of
Early Villages: Models of Central Mesa Verde Archaeology', Univ. of
California Press; Kohler et al. (2012) 'Modelling prehispanic Pueblo
societies in their ecosystems', Ecological Modelling 241:30-41.
Verified via search (ScienceDirect S0304380012000038).

Household agents on an 1817 km2 real landscape (southwest Colorado,
600-1280 CE) choose residence and field locations to minimize caloric
costs of farming, water procurement, fuelwood, and hunting, on a
landscape with cell-level maize paleoproductivity (retrodicted from
tree-ring proxies + soil type), spring/stream locations, and
depletable wood and game. Validated against >18,000 recorded sites:
population curve, timing of aggregation into villages, and
site-location optimality tests (simulated ideal locations vs actual
site distribution).

As code: the strongest precedent for via's coupling — settlement
choice is literally cost-surface minimization over terrain-,
hydrology-, and resource-fields via's nature side already generates.
State = per-cell productivity/water/fuel stocks + households; update
= annual harvest, resource depletion/regrowth, discrete relocation to
the argmin-cost cell. Deterministic given seed. Depletion feedback
(soil, wood) gives endogenous relocation without authored styles.

Era dependence: era = parameter vector (crop yields, transport cost
per kg-km, water technology, fuel substitution). Structure
(cost-minimizing location choice on physical fields) is
era-invariant; the VEP explicitly frames later periods as changed
forcing, not changed mechanism.

### MayaSim coupled settlement-ecosystem-trade model

Heckbert, S. (2013) 'MayaSim: An Agent-Based Model of the Ancient
Maya Social-Ecological System', JASSS 16(4):11. Verified; Python
reimplementation at github.com/pik-copan/MayaSim (Kühlein/PIK).

Settlement agents on a gridded Yucatan landscape with cellular
processes: rainfall-driven water flow, soil degradation from
agriculture, forest succession, net primary production. Settlements
derive income from agriculture, ecosystem services, and a trade
network forming between settlements as they grow; population follows
income; overshoot degrades soil/forest, collapsing agriculture and
the trade network — reproducing a rise-and-fall trajectory and
(qualitatively) Classic Maya spatial patterns.

As code: simulable — all subsystems are explicit field/graph update
rules (flow routing, soil and forest state per cell; settlement nodes
with population; trade edges by gravity/threshold). The closest
published template for via's full chain terrain -> hydrology ->
suitability -> settlements -> flows -> network. But its validation is
admittedly weak ("proof-of-concept ... requires refinement and
further archaeological data for calibration"): an architecture
precedent, not a gate source.

Era dependence: era only as parameters (agricultural technology,
trade cost per distance). No structural era switch. Its instability
(boom-bust) is a warning: coupled feedback loops need damping
parameters that are themselves era-dependent.

### MERCURY Roman trade-network ABM

Brughmans, T. & Poblome, J. (2016) 'MERCURY: an Agent-Based Model of
Tableware Trade in the Roman East', JASSS 19(1):3; and Brughmans &
Poblome (2016) 'Roman bazaar or market economy?', Antiquity
90(350):393-408. Both verified.

Trader agents on a social network exchange commercial information and
goods; the experiment varies market integration (degree of
inter-community links) to discriminate two macro-hypotheses about the
Roman economy (Bang's fragmented bazaar vs Temin's integrated
market). Validation is comparative: simulated distribution widths of
four tableware classes across sites vs the archaeologically observed
distributions of eastern terra sigillata; only high-integration
parameterizations reproduce the observed wide, strongly
differentiated ware distributions.

As code: simulable as a graph process — node state = stock/demand per
good, edge process = information diffusion + transactions;
deterministic given seeded RNG. The lesson is methodological: use the
model to discriminate hypotheses by matching a distributional
statistic of flows, not to "grow" one history. Flow-on-network
statistics (how concentrated goods from one origin are across
destinations) are gateable outputs of via's flows layer.

Era dependence: integration degree, transport cost, and information
range ARE the era parameters; the exchange mechanism is
era-invariant. Directly supports via's doctrine that medieval vs
modern is a parameter change in interaction range/cost.

### Site catchment analysis

Vita-Finzi, C. & Higgs, E.S. (1970) 'Prehistoric economy in the Mount
Carmel area of Palestine: site catchment analysis', Proceedings of
the Prehistoric Society 36:1-37. Standard citation, not re-verified
this session but uncontested in the literature.

A settlement's economic viability is determined by the resources
within its habitual exploitation radius: canonically ~5 km (~1 hour
walk) for agriculturalists, ~10 km (~2 hours) for hunter-gatherers,
with exploitation intensity declining with travel cost. Sites persist
where the cost-weighted resource integral over the catchment exceeds
subsistence needs.

As code: trivially simulable and already terrain-coupled — compute
anisotropic cost-distance (slope-dependent, e.g. Tobler hiking
function) from each candidate cell, integrate suitability fields
(arable, water, pasture) with a distance-decay kernel; viability =
catchment integral. A suitability-layer component, not a full model.
Inputs: via's slope, land-cover/soil, hydrology fields.

Era dependence: catchment radius and cost function are explicitly
technology-dependent (walking -> cart -> rail -> car) — one of the
cleanest single era parameters in the whole literature: same
integral, era-scaled kernel.

### Archaeological predictive modeling on terrain covariates

Kvamme, K.L. (1988) 'Development and testing of quantitative models',
in Judge & Sebastian (eds.) 'Quantifying the Present and Predicting
the Past'; Kvamme (1990) J. Archaeological Science; Verhagen, P. &
Whitley, T.G. (2012) 'Integrating Archaeological Theory and
Predictive Modeling', J. Archaeol. Method and Theory 19:49-100. Field
and the Kvamme gain statistic verified via multiple applications
(e.g. PLOS ONE 10.1371/journal.pone.0239424).

Site presence/absence is regressed (classically logistic regression)
on terrain covariates — slope, elevation, aspect, curvature, distance
to water, soil class, viewshed — yielding a site-probability surface.
Quality is scored by Kvamme's gain G = 1 - (%area classified
high-potential / %sites captured); distance-to-water and slope are
consistently the dominant covariates across regions and periods.

As code: descriptive-only as published (statistical association, no
process). But it defines exactly which physical covariates a
mechanistic suitability field must depend on, and its gain statistic
converts directly into a gate: via's emergent settlements, scored
against via's own terrain covariates, should achieve gain in the
empirically accepted band (steepness avoidance, water proximity).

Era dependence: covariate weights shift with era (defensibility and
water dominate medieval; railroad/road access dominates 19thC
frontier; almost pure network access modern). The covariate list is
stable; the weight vector is the era parameter — precisely how via
should encode era in the suitability tier.

### SIMPOP family / evolutionary theory of urban systems

Sanders, L., Pumain, D., Mathian, H., Guérin-Pace, F. & Bura, S.
(1997) 'SIMPOP: a multiagent system for the study of urbanism',
Environment and Planning B 24:287-305. Verified. Bretagnolle, A. &
Pumain, D. (2010) 'Simulating urban networks through multiscalar
space-time dynamics: Europe and the United States, 17th-20th
centuries', Urban Studies 47(13). Verified. Schmitt, C.,
Rey-Coyrehourcq, S., Reuillon, R. & Pumain, D. (2015) 'Half a billion
simulations: evolutionary algorithms and distributed computing for
calibrating the SimpopLocal geographical model', Environment and
Planning B 42(2):300-315. Verified (arXiv:1502.06752).

Settlements are agents whose growth is Gibrat-like (proportional
stochastic growth) but modulated by inter-city exchanges: each city
holds economic functions (market, administration, innovation waves),
acquires new functions by size/network thresholds, and trades within
technology-limited interaction ranges. Spatial interaction amplifies
hierarchical differentiation beyond pure Gibrat, producing the
observed over-concentration at the top of the hierarchy. SimpopLocal
grows an urban hierarchy from a homogeneous rural system over
millennia; Simpop2 reproduced the Old-World dense-substrate vs US
frontier colonization trajectories with one model, differently
parameterized.

As code: simulable as a node-population process on via's settlement
graph — state = population + function set per settlement; update =
growth from base rate + gains from feasible exchanges
(distance-limited on the network via provides); function acquisition
by deterministic thresholds. Needs settlement seeds and network
distances from via. Caution: as published it is stochastic and was
calibrated by massive evolutionary search; via must re-derive with
seeded RNG and accept weaker calibration. Its aspatial-substrate
character means via supplies the missing terrain coupling.

Era dependence: the most explicitly era-structured precedent —
successive innovation waves change interaction range, transport
speed, and available urban functions as declared exogenous forcing
(matching via's forcing tier); Bretagnolle & Pumain 2010 shows
medieval-substrate Europe vs 19thC frontier US as the SAME mechanism
under different substrate and transport-era parameters — exactly
via's design thesis.

### Correlated gradient percolation model of urban growth

Makse, H.A., Havlin, S. & Stanley, H.E. (1995) 'Modelling urban
growth patterns', Nature 377:608-612. Verified. Makse, Andrade,
Batty, Havlin, Stanley (1998) 'Modeling urban growth patterns with
correlated percolation', Physical Review E 58:7054-7062. Verified
(arXiv:cond-mat/9809431).

Development units occupy sites with probability p(r) decaying with
distance from the urban core (gradient percolation), with long-range
correlated occupancy because development attracts development.
Reproduces (i) the power-law area distribution of satellite towns
around a metropolis (Berlin 1920/1945, London 1981/1991), (ii) the
fractal urban boundary of gradient-percolation hulls, (iii)
qualitative interpenetration of urban and rural land — where
uncorrelated percolation and DLA both fail quantitatively.

As code: simulable as a field process — occupancy field on grid,
deterministic once the correlated random field is generated from the
seed; via can make the centrality/attractiveness potential physical
(accessibility over terrain) instead of isotropic distance decay.
However it is morphology-mimicry: the "mechanism" is a statistical
ansatz, not a causal human process. Best used as a null model and as
the source of morphological gate statistics that via's causal chain
is expected to reproduce.

Era dependence: density-gradient steepness maps onto transport era
(steep pre-transit, flat with automobiles — the classic Clark 1951
flattening); correlation length maps onto planning scale. Era is
purely parametric.

### Batty lineage: DLA and cellular-automata urban growth

Batty, M. & Longley, P. (1994) 'Fractal Cities: A Geometry of Form
and Function', Academic Press; Batty, M. (2005) 'Cities and
Complexity: Understanding Cities with Cellular Automata, Agent-Based
Models, and Fractals', MIT Press. Standard citations. Constrained-CA
operationalization: White, R. & Engelen, G. (1993) 'Cellular automata
and fractal urban form', Environment and Planning A 25:1175-1199.

Cities grow by local attachment: DLA-style accretion gives dendritic
growth with fractal dimension ~1.71, close to measured urban
dimensions (~1.7) but too sparse/dendritic in detail; constrained
cellular automata (transition rules over neighborhood land-use
composition + suitability + accessibility + stochastic perturbation)
grow land-use mosaics whose cluster-size distributions, fractal
dimensions, and map overlap (kappa) are compared with observed maps.
This lineage became the operational planning models (MOLAND,
Metronamica).

As code: highly simulable and terrain-friendly — CA state = land-use
class per cell; update = deterministic ranking of transition
potentials (suitability from via's terrain/hydrology, accessibility
from via's network, neighborhood kernels) with seeded perturbation.
Danger for via's doctrine: neighborhood-effect weight matrices are
effectively authored look-tables unless constrained/calibrated
against data — keep the kernel parametric and few-parameter, gate on
emergent cluster statistics.

Era dependence: accessibility term (network speed), suitability
weights, and demand-per-capita are era parameters; planning
constraints (zoning masks) enter as via's forcing tier. Structural
rules unchanged across eras.

### Schelling segregation model (cautionary minimal model)

Schelling, T.C. (1971) 'Dynamic models of segregation', Journal of
Mathematical Sociology 1(2):143-186. Standard citation.

Two agent types on a lattice relocate when the fraction of like
neighbors falls below a mild tolerance threshold; global segregation
emerges from preferences far weaker than the macro-pattern would
suggest. Canonical demonstration that macro patterns radically
underdetermine micro mechanisms.

As code: trivially simulable (deterministic sweep order + seeded
tie-breaking), and a Schelling-type sorting rule is a legitimate
intra-settlement morphology mechanism (district differentiation). But
its role here is cautionary: never validated against any specific
city, and it proves that reproducing one pattern is weak evidence for
a mechanism — via's multi-gate stance is the correct response.

Era dependence: none in the model; tolerance/mobility would be era
parameters if used. Its lesson is era-independent: equifinality.

### Pattern-oriented modeling (POM) validation methodology

Grimm, V., Revilla, E., Berger, U., Jeltsch, F., Mooij, W.M.,
Railsback, S.F., Thulke, H.-H., Weiner, J., Wiegand, T. & DeAngelis,
D.L. (2005) 'Pattern-Oriented Modeling of Agent-Based Complex
Systems: Lessons from Ecology', Science 310(5750):987-991. Verified.

A model is structurally validated only if it SIMULTANEOUSLY
reproduces multiple observed patterns at different scales and
hierarchical levels (individual + system, spatial + temporal), each
pattern acting as a filter that rejects candidate structures and
parameterizations; single-output curve-fitting is explicitly
rejected. Patterns used for calibration must be distinct from
patterns used for validation.

As code: not a simulation but THE validation doctrine for via's human
side — the settlement-world equivalent of via's existing
geomorphology gate suite (slope-area theta + Hack + Horton + SPL
residual are already a POM battery). Prescribes: choose 3-6
independent gate statistics per layer (rank-size, spacing, network
meshedness, scaling exponent, morphology fractal dimension) and
require joint passage.

Era dependence: method is era-free; it implies era-conditional gate
values where the empirical literature shows drift (e.g. Zipf
coefficient drifts with urbanization stage).

### Settlement scaling theory (cross-era urban scaling)

Ortman, S.G., Cabaniss, A.H.F., Sturm, J.O. & Bettencourt, L.M.A.
(2014) 'The Pre-History of Urban Scaling', PLOS ONE 9(2):e87902.
Verified. Ortman et al. (2015) 'Settlement scaling and increasing
returns in an ancient society', Science Advances 1(1):e1400066.
Verified. Underlying theory: Bettencourt, L.M.A. (2013) 'The Origins
of Scaling in Cities', Science 340:1438-1441; Bettencourt et al.
(2007) 'Growth, innovation, scaling, and the pace of life in cities',
PNAS 104:7301-7306. Verified.

Settled area scales with population as A ~ N^a with a between 2/3
(amorphous/social-network-limited) and 5/6 (networked/infrastructure-
structured), derived from balancing social interaction benefits
against movement costs within the settlement. Tested on >1,500
pre-Hispanic Basin of Mexico settlements across four cultural periods
(two millennia): the same scaling as modern cities holds, and
increasing returns (superlinear output) appear in ancient
monument/production proxies.

As code: the derivation is a closed-form optimization, not a
simulation, but it constrains any simulation — via's emergent
settlements should reproduce a in [2/3, 5/6] as an EMERGENT relation
between built extent and population across all three target eras. It
can also act as an intra-settlement land-allocation rule (area grows
to balance interaction benefit vs transport cost given era movement
speed). Inputs: none beyond via's own settlement populations and
footprints — a pure output gate.

Era dependence: the headline empirical result is era-INVARIANCE of
the exponent range; era changes the prefactor (movement speed/cost),
i.e. absolute density, not the exponent. The single best cross-era
gate available for via's thesis that one mechanism spans medieval to
modern.

### Street-network growth by local optimization + empirical evolution

Strano, E., Nicosia, V., Latora, V., Porta, S. & Barthélemy, M.
(2012) 'Elementary processes governing the evolution of road
networks', Scientific Reports 2:296. Verified. Model: Barthélemy, M.
& Flammini, A. (2008) 'Modeling urban street patterns', Physical
Review Letters 100:138702. Structure statistics: Cardillo, A.,
Scellato, S., Latora, V. & Porta, S. (2006) 'Structural properties of
planar graphs of urban street patterns', Physical Review E 73:066107.
Verified.

Empirically (Groane/Milan, 14 municipalities, 1833-2007): road
networks grow by two elementary processes — densification (infill
around existing centers) and exploration (extending the urbanization
front); exploration dominates early eras, densification late; cell
(block) shapes homogenize over time; the high-betweenness backbone is
extremely stable (~90% of the 100 most central 2007 routes already
existed in 1833). Barthélemy-Flammini model: new centers appear, and
the network adds road segments that greedily minimize construction
cost to connect them, reproducing observed cell-area distributions
and perpendicular intersections.

As code: directly simulable on via's substrate — state = planar
graph; update = when a new settlement/demand node appears (from via's
settlement layer), grow the cheapest connection increment over the
terrain cost surface (slope- and river-crossing-weighted), plus a
densification rule triggered by local demand density. Deterministic
given seeded node arrivals. Via's corridor layer already computes
terrain-weighted costs; this closes the loop network -> morphology.

Era dependence: era sets the densification:exploration ratio
(frontier exploration-heavy, modern densification-heavy —
empirically documented), construction cost per unit length vs terrain
(engineering capability), and design regularity (planned grid vs
organic incremental). The mechanism (incremental cost-minimizing
connection) is era-invariant; the 19thC American grid additionally
needs a planning forcing (survey-grid alignment field) in via's
forcing tier.

## Gate candidates

- **Zipf/rank-size exponent** of settlement population distribution
  (Lotka form, rank ~ size^-alpha). Distribution of 1,962 published
  estimates: mean 1.025, median 0.986, sd 0.282; approx 40% of the
  variance across studies is attributable to technical/measurement
  choices, so the gate must fix the city-definition and estimation
  protocol and then test alpha in roughly [0.8, 1.2]. Source:
  Cottineau, C. (2017) 'MetaZipf. A dynamic meta-analysis of city
  size distributions', PLOS ONE 12(8):e0183919. Verified.
- **Settled-area vs population exponent** a in A ~ N^a (per system,
  cross-sectional). 2/3 <= a <= 5/6 for >1,500 pre-Hispanic Basin of
  Mexico settlements across four periods; modern infrastructure
  scaling beta ~ 0.8-0.9 (e.g. road surface ~0.83); the exponent band
  is era-invariant, prefactor era-dependent. Source: Ortman et al.
  (2014) PLOS ONE 9(2):e87902; Bettencourt et al. (2007) PNAS
  104:7301-7306. Both verified.
- **Superlinear socioeconomic scaling exponent** (production,
  innovation vs population). beta ~ 1.1-1.3, typically ~1.15 (GDP,
  wages), ~1.27 (patents); increasing returns also detected in
  ancient monument-construction proxies. Source: Bettencourt et al.
  (2007) PNAS 104:7301-7306; Ortman et al. (2015) Science Advances
  1:e1400066. Verified.
- **Power-law area distribution of towns/urban clusters** (pdf
  exponent of cluster areas). pdf exponent approx 2 (equivalently
  Zipf-like rank exponent approx 1 for areas), measured for Berlin
  (1920, 1945) and London (1981, 1991) satellite towns; reproduced
  only by the correlated (not uncorrelated) percolation model. Exact
  fitted values are in the 1998 paper (~1.9-2.1 band); flag: pull
  exact digits from the PRE full text before hard-coding a gate.
  Source: Makse, Havlin & Stanley (1995) Nature 377:608-612; Makse et
  al. (1998) Phys. Rev. E 58:7054-7062. Papers verified; exact
  exponent digits not re-extracted this session.
- **Fractal dimension of urban built-up form** (box-counting/radial)
  and of the urban boundary. Urban area D typically ~1.4-1.9 with
  large cities ~1.7 (e.g. London ~1.77 in Batty & Longley's
  compilations); gradient-percolation urban perimeter dimension ~4/3;
  DLA reference value 1.71. Source: Batty, M. & Longley, P. (1994)
  'Fractal Cities', Academic Press; Makse et al. (1998) PRE 58:7054.
  Standard values; individual city digits from memory of the
  compilation, verify before gating hard.
- **Street-network structural statistics**: average node degree,
  meshedness M = (E-N+1)/(2N-5), fraction of dead-ends and 4-way
  intersections. Average node degree ~2.5-3.3 (organic Bologna 2.71
  vs gridded San Francisco 3.21); meshedness higher in planned than
  organic cities (organic systems more tree-like); Cardillo et al.'s
  twenty 1-square-mile samples span roughly M ~ 0.02-0.35 (planned
  grids at the top). Use degree + meshedness jointly as an
  era/planning discriminator. Source: Cardillo, Scellato, Latora &
  Porta (2006) Phys. Rev. E 73:066107 (verified); Barthelemy (2022)
  'A Review of the Structure of Street Networks', Findings
  (verified); Boeing (2019) Applied Network Science 4:67 for US-scale
  orientation-entropy and intersection-type shares.
- **Road-network temporal evolution**: densification vs exploration
  mix and backbone persistence. Exploration dominates early
  urbanization, densification late; ~90% of the 100 highest-
  betweenness routes in 2007 already present in 1833 (Groane/Milan,
  14 municipalities); block-shape distribution homogenizes toward
  regular cells over time. Source: Strano, Nicosia, Latora, Porta &
  Barthélemy (2012) Scientific Reports 2:296. Verified.
- **Kvamme's gain** G = 1 - (%area predicted high-potential / %sites
  captured) for settlement-location suitability. G in [0,1);
  operationally "useful" predictive models report G ~ 0.5-0.85 on
  terrain covariates (slope, distance to water, aspect, soils); via's
  emergent settlement placements should yield comparable gain against
  its own suitability covariates, i.e. sites concentrate in a small
  high-suitability fraction of the landscape. Source: Kvamme, K.L.
  (1988) in 'Quantifying the Present and Predicting the Past' (BLM);
  applied with reported gains in e.g. Yaworsky et al. (2020) PLOS ONE
  15(10):e0239424. Field verified; canonical 1988 chapter not
  directly fetched.
- **Clark-Evans nearest-neighbor statistic R** for inter-settlement
  spacing (R=1 random, R=2.15 perfectly regular hexagonal). Agrarian
  settlement systems typically R ~ 1.1-1.5 (weak-to-moderate
  regularity, the empirical residue of central-place spacing);
  strongly clustered systems R < 1. Requires edge-effect correction.
  Treat exact band as region-dependent and calibrate the protocol
  before gating. Source: Clark, P.J. & Evans, F.C. (1954) Ecology
  35:445-453; archaeological usage codified in Hodder, I. & Orton, C.
  (1976) 'Spatial Analysis in Archaeology', Cambridge UP. Standard
  citations; band from field practice, not a single meta-analysis.
- **Site-catchment radii / distance-to-water concentration**.
  Canonical exploitation radii ~5 km (1 hr walk) agrarian, ~10 km
  (2 hr) forager; distance-to-water is the consistently dominant
  covariate in predictive models across regions — premodern site
  densities fall off within the first 1-2 km of permanent water. Use
  as a distance-decay gate on emergent site placement, era-scaled by
  transport speed. Source: Vita-Finzi & Higgs (1970) Proc. Prehist.
  Soc. 36:1-37; covariate dominance across the predictive-modeling
  literature (e.g. Verhagen & Whitley 2012 JAMT 19:49-100).
- **Deviation from Gibrat's law** at the top of the hierarchy (growth
  advantage of largest cities; growth-rate variance vs size).
  Largest, best-connected cities grow persistently faster than
  size-independent Gibrat growth predicts, producing hierarchy
  steepening over time (Europe and US, 17th-20thC); this deviation is
  the calibrated target of the SIMPOP family rather than a single
  exponent — gate as: hierarchy Gini/top-share increases under
  interaction, stays flat under pure Gibrat. Source: Bretagnolle, A.
  & Pumain, D. (2010) Urban Studies 47(13); Pumain, D. (2018) 'An
  Evolutionary Theory of Urban Systems', Springer; Verbavatz, V. &
  Barthélemy, M. (2020) 'The growth equation of cities', Nature
  587:397-401 (migration shocks as the driver). Verified.

## Pitfalls and contested claims

- Equifinality is the field's central demon: Janssen's 2009
  replication of Artificial Anasazi (JASSS 12(4):13) showed the
  celebrated fit is carried by two carrying-capacity calibration
  parameters, agent behavior contributing "only a modest improvement"
  over population = carrying capacity. Matching one time series or
  one map is near-zero evidence for mechanism; via must gate on
  multiple independent statistics simultaneously (pattern-oriented
  modeling), mirroring its geomorphology gate suite.
- Zipf's law is measurement-fragile: Cottineau 2017 shows ~40% of the
  variance in published exponents comes from technical choices (city
  delimitation, size threshold, rank truncation, estimation method).
  A Zipf gate is only falsifiable if via fixes one clustering/
  definition algorithm (e.g. CCA on the population field) and one
  estimator, and states the band conditional on that protocol. Also
  disputed whether Zipf vs lognormal even holds (Eeckhout vs Levy
  debate).
- Morphology-matching without process: correlated percolation and DLA
  reproduce urban statistics with no human mechanism, proving those
  statistics are weakly diagnostic on their own. Use them as null
  models: any gate a null model also passes cannot certify via's
  causal chain — prefer gates that couple layers (network backbone
  persistence, scaling of area WITH population, suitability gain),
  which nulls fail.
- Urban scaling exponents are boundary-sensitive: Arcaute et al.
  (2015, J. R. Soc. Interface 12:20140745) showed beta for many
  quantities drifts across city-definition cutoffs, and part of the
  scaling literature considers the 1.15/0.85 dichotomy overstated.
  Gate on the exponent band with a declared settlement-delimitation
  procedure, and prefer the Ortman premodern band (2/3-5/6 for area)
  which survived this critique.
- Survey bias contaminates archaeological gates: predictive-model
  gain statistics and "distance to water" effects are partly
  self-fulfilling because archaeologists survey near water and roads;
  recorded-site databases (including VEP's 18,000 sites) undercount
  some site classes. Treat archaeological location statistics as
  bands, not points.
- MayaSim is an architecture precedent, not a validation precedent:
  its own authors call it a proof-of-concept requiring calibration;
  do not import its parameter values, and note that tightly coupled
  settlement-ecosystem feedbacks are prone to unrealistic boom-bust
  unless damped.
- Calibration cost and identifiability: SimpopLocal needed
  evolutionary search over ~half a billion runs to find viable
  parameterizations (Schmitt et al. 2015), i.e. the mechanism's
  parameters are poorly identified by the data. Via should keep the
  human-side parameter count small and physically interpretable, or
  gate-based falsification becomes gate-based fitting.
- Published ABMs are almost all stochastic Monte Carlo; none of the
  precedents is bitwise-deterministic. Via's determinism requirement
  is satisfiable (seeded RNG, fixed iteration order) but means
  published "validated" behavior — often ensemble means — must be
  re-established for single seeded runs or run-ensembles; a single
  run passing a distributional gate is a different (and stricter or
  noisier) claim than an ensemble mean passing it.
- Central-place regularity is largely a failed strong prediction:
  clean Christaller lattices are rarely observed; only weak spacing
  regularity (Clark-Evans R modestly above 1) survives empirically.
  Gate on weak regularity, not hexagonal geometry — the latter would
  be gating on a contested idealization.
- Pattern-oriented modeling requires calibration/validation
  separation: several archaeology ABMs (including Artificial Anasazi)
  tuned parameters on the same record they claim to reproduce. Via
  should declare, per gate, whether the statistic was used in
  calibration; only untouched gates count as validation.
- Era drift vs era invariance must be encoded per gate: settlement
  scaling (area-population exponent) is empirically era-invariant,
  while Zipf coefficients drift with urbanization stage and
  street-network statistics drift with the planning regime. A single
  "pass band" applied across all three eras would be wrong for the
  drifting gates and is exactly the kind of era-dependence the
  forcing tier must declare.
- Two search-verification gaps to close before hard-coding numbers:
  the exact fitted area-distribution exponents in Makse et al. 1998
  (PDF not parsed this session) and the exact meshedness values per
  city sample in Cardillo et al. 2006; both papers are verified to
  exist and to contain these tables, but the digits above are from
  secondary reporting and memory.
