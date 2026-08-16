# Research 0004 — Land use & land value

Status: research corpus — pre-decision. Nothing in this document is
adopted; adoption of any mechanism requires its own ADR. Part of
docs/research/humanity (see README there).

## The question this literature answers for via

Given settlements, corridors, and a suitability field, what claims
each cell of land, at what value, at what density? The literature
spans the agricultural hinterland (von Thünen), the internal
structure of a single city (Alonso–Muth–Mills, Fujita–Ogawa), and
generative pattern models (the CA families), with era entering
almost everywhere as transport-cost parameters — oxcart to truck.

## Models and mechanisms

### von Thünen ring model (agricultural bid-rent vs transport cost)

von Thünen, J.H. (1826) Der isolierte Staat in Beziehung auf
Landwirtschaft und Nationalökonomie, Hamburg (English: Hall, P. ed.,
1966, Von Thünen's Isolated State, Pergamon). Foundational;
universally cited in land-use economics.

Each use i earns location rent R_i(d) = Y_i(p_i − c_i) − Y_i f_i d
at distance d from the market: yield times (price minus production
cost) minus yield times freight rate times distance. Land goes to
the highest bidder, so uses sort into concentric zones ordered by
transport-cost intensity per unit land (perishable/heavy/high-yield
goods closest). Rings are an outcome of bid competition, not an
input.

As code: forward, trivial, exactly deterministic. State: per-cell
use and rent. Per cell, compute generalized transport cost to the
market node along via's corridor network (not Euclidean — von Thünen
himself showed a navigable river deforms rings into bands), evaluate
the linear bid R_i per use with per-era prices/freight rates, assign
argmax with rent = max bid; cells below zero max bid stay wild.
Inputs from via: yield/suitability field (soil, climate), corridor
cost field, market nodes from the settlement layer. No iteration
unless prices are endogenous.

Era dependence: purely parametric — freight rate f per ton-km
(oxcart vs rail vs truck: orders-of-magnitude drops), crop/use menu
and prices, number of competing markets. Ring radii scale as
(p−c)/f, so rings explode outward with rail (19thC frontier:
single-market rings around railheads are historically documented)
and dissolve into near-uniform use under modern trucking.

### Alonso bid-rent / Muth–Mills monocentric city model (AMM)

Alonso, W. (1964) Location and Land Use: Toward a General Theory of
Land Rent, Harvard University Press; Muth, R. (1969) Cities and
Housing, University of Chicago Press; Mills, E.S. (1967) 'An
aggregative model of resource allocation in a metropolitan area',
American Economic Review 57(2):197-210. Unified treatment:
Brueckner, J. (1987) 'The structure of urban equilibria', Handbook
of Regional and Urban Economics vol 2. Verified.

Households maximize u(z, q) (composite good, floorspace) subject to
y = z + p(x)q + T(x), T(x) the commuting cost to the CBD. Spatial
equilibrium (equal utility everywhere) forces price to fall exactly
enough to offset rising T: dp/dx = −T′(x)/q. Producers substitute
capital for land, so structural and population density fall with x;
land rent at the edge equals agricultural rent, closing the city
boundary. With Cobb-Douglas preferences and T′/(y−T) constant,
density is exactly negative exponential D(x) = D0·e^{−γx} with
γ = ((1−α)/α)·T′/(y−T) (Anas, Arnott & Small 1998, JEL 36:1426-1464,
eq. 5).

As code: forward, deterministic, cheap — the source rates it the
single best fit to via's doctrine. State per cell: generalized
access cost τ to the CBD/employment node, computed on via's
corridor/street network over real terrain — this replaces
"distance" and deforms gradients along valleys and roads — plus
rent r, housing capital K, density D. Open-city (fixed utility from
the regional system — natural for via since settlements compete)
makes r and D closed-form in τ; closed-city solves by 1-D shooting
on the population constraint. Outputs are continuous
rent/density/FAR spectra, no taxonomy; gates directly against the
Clark/Mills gradient numbers. Multi-center: τ = min or
logit-smoothed cost over several employment nodes.

Era dependence: a parameter set — commute speed (walking ~5 km/h,
horsecar/streetcar ~10-15, auto 30-50), value of time vs money,
income y, housing expenditure share, agricultural opportunity rent.
γ ∝ T′/(y−T), so gradients flatten mechanically as speed and income
rise, reproducing the measured 1880→1963 collapse (see gates). One
genuine structural flip: who lives centrally. Pre-industrial cities
put the rich central and the poor peripheral; cheap mass transport
inverts it (LeRoy, S. & Sonstelie, J. 1983 'Paradise lost and
regained: transportation innovation, income, and residential
location', J. Urban Economics 13:67-89 — sorting order set by which
income group has the higher ratio of time-cost to housing demand,
itself era-dependent; Sjoberg, G. 1960 The Preindustrial City
documents the medieval pattern). The flip falls out of the same
bid-rent math with two income classes and mode-specific costs — era
stays parametric.

### Fujita–Ogawa non-monocentric equilibrium (endogenous polycentricity)

Fujita, M. & Ogawa, H. (1982) 'Multiple equilibria and structural
transition of non-monocentric urban configurations', Regional
Science and Urban Economics 12(2):161-196; precursor Ogawa & Fujita
(1980) J. Regional Science 20:455-475. Verified
(ScienceDirect/RePEc). Modern quantitative descendants: Lucas &
Rossi-Hansberg (2002) Econometrica 70:1445-1476; Ahlfeldt, Redding,
Sturm & Wolf (2015) Econometrica 83:2127-2189.

Neither jobs nor homes are placed a priori. Firm productivity at x
includes a locational potential F(x) = ∫ K·e^{−α|x−y|} b(y) dy —
agglomeration benefit from other firms, decaying at rate α. Workers
pay commuting cost t per unit distance. Firms and households bid
for land; equilibrium is the assignment where no agent gains by
moving. The ratio t/α controls the phase: high gives one mixed or
monocentric city, intermediate gives duocentric/tricentric
configurations, low commuting cost gives dispersion. Multiple
equilibria exist at fixed parameters, and the city undergoes
catastrophic structural transitions as parameters cross critical
values.

As code: forward equilibrium solve, deterministic if disciplined —
discretize onto via's grid with network travel costs replacing
|x−y|; iterate: compute F from current firm density, compute bid
rents for firms and households given wage/commute fields, reassign
each cell to the highest bidder, relax with fixed damping and fixed
sweep order to a fixed point. Seedability handles equilibrium
selection: initialize from the prior era's settlement pattern,
making history the selection device (a feature — path dependence is
real). Inputs: commuting cost field (corridors), decay α, commuting
rate t per era. Output: endogenous employment subcenters as a
continuous employment-density field — the mechanism that turns
"town" into "complex modern city" without a separate generator.

Era dependence: t/α is the era knob — walking-era commuting cost is
enormous → single compact mixed-use center (medieval town);
rail/streetcar lowers t → strong monocentric CBD with segregated
residential ring (19thC); the automobile lowers t further and
telecom arguably lowers α → subcenter formation and dispersion.
Era is parametric, but the output undergoes genuine structural
transitions (the paper's own point).

### Constrained cellular-automata land use (White–Engelen family)

White, R. & Engelen, G. (1993) 'Cellular automata and fractal urban
form: a cellular modelling approach to the evolution of urban
land-use patterns', Environment and Planning A 25(8):1175-1199;
White, R., Engelen, G. & Uljee, I. (1997) 'The use of constrained
cellular automata for high-resolution modelling of urban land-use
dynamics', Environment and Planning B 24:323-343. Verified.

Cell transitions driven by a potential
P_j = s_j · a_j · (1 + Σ_k Σ_d m_{kj}(d)·I_{kd}) · ε: physical
suitability s, network accessibility a, and a distance-weighted
neighborhood effect where m_{kj}(d) encodes attraction/repulsion
between use k and candidate use j at ring distance d (commerce
attracts commerce at short range, industry repels housing). Each
step an exogenous macro demand fixes how many cells of each use
must exist; the highest-potential cells convert. Emergent form is
fractal/bifractal, matching measured urban land-use fractal
dimensions.

As code: forward generative, a near-perfect fit to via's
architecture per the source: state = per-cell use + potential;
inputs = via's suitability field (terrain/soil/flood),
accessibility from the corridor network, the weight matrix; macro
demand per era from the settlement/flows layer. The perturbation ε
is a seeded deterministic draw — bitwise reproducible. The source's
caution: m_{kj}(d) is dangerously close to an authored look-table
unless each entry is tied to a citable externality mechanism and
exposed as declared config, and outputs must be validated on
spectra (fractal dimension, cluster-size distributions, gradient
statistics), not visual resemblance.

Era dependence: macro demands (area per capita an economy of era E
requires by use), the accessibility definition (road vs rail vs
walk), interaction ranges (scaling with transport speed), and the
use menu itself (no "industrial" use in a medieval run). Mostly
parametric, but the use-menu change is a structural config change
between eras — the source says it should be declared, not hidden.

### SLEUTH urban growth CA (Clarke)

Clarke, K.C., Hoppen, S. & Gaydos, L. (1997) 'A self-modifying
cellular automaton model of historical urbanization in the San
Francisco Bay area', Environment and Planning B 24(2):247-261.
Verified. Calibration record and self-critique: Clarke (2008) 'A
decade of cellular urban modeling with SLEUTH: unresolved issues
and problems', in Planning Support Systems for Cities and Regions.

Binary urban/non-urban growth from Slope, Land use, Exclusion,
Urban extent, Transportation, Hillshade inputs. Five coefficients
(diffusion, breed, spread, road gravity, slope resistance) drive
four growth rules per step: spontaneous seeding, new spreading
centers, edge/organic growth, road-influenced growth;
self-modification boosts or damps coefficients when growth rate
crosses thresholds (boom/bust). Calibrated by brute-force sweeps of
the 5-coefficient space against historical urban extents (SF Bay
1850-1990 in the original).

As code: forward generative, easily made seeded-deterministic
(Monte Carlo over a seeded RNG). Its inputs are exactly what via
already produces: slope, exclusion (water/flood), road network,
existing seeds. But it outputs only urban extent — no density,
rent, or use mix — so for via it is at most a sprawl/extent
envelope process, and its 26-year multi-city calibration record
shows the coefficients are city-specific with no transferable
physical meaning. The source's verdict: a benchmark to beat, not a
component.

Era dependence: no principled era structure. Coefficients absorb
era effects during calibration (road-gravity high in the auto era,
spread in the rail era) but nothing maps eras to coefficients
forward — era is neither a clean parameter nor a declared
structural change, a key reason the source judges it a poor fit as
a core mechanism.

### UrbanSim microsimulation (the data-hungry pole)

Waddell, P. (2002) 'UrbanSim: modeling urban development for land
use, transportation, and environmental planning', Journal of the
American Planning Association 68(3):297-314. Verified.

Represents every household, job, parcel and building. Interacting
components: household relocation and residential location choice
(estimated multinomial logit on accessibility, price, neighborhood
attributes), firm location choice, developer supply (what to build
where, by pro-forma profitability), land-price hedonic updating;
coupled to an external travel demand model. Market outcomes emerge
from simulated demand-supply interaction year by year.

As code: forward in form but descriptive-only for via's purpose —
every behavioral coefficient is estimated from parcel-level micro
data (census, assessor files, travel surveys) for one specific
metro, data that do not exist for a generated landscape or a
medieval era. Stochastic (needs seeding, which it supports). Its
lasting value to via is architectural: the demand/supply/
price-adjustment loop (choosers bid via accessibility-and-price
logit; developers add floorspace where price exceeds construction
cost; prices clear) can be reimplemented with AMM/Fujita-Ogawa-
derived parametric forms instead of estimated coefficients.

Era dependence: none by design — coefficients are place-and-period-
specific with no transfer theory; crossing eras would mean
re-estimating from data that cannot exist. The cautionary pole:
maximum realism, minimum generative portability.

### Clark negative-exponential density law (empirical target, not mechanism)

Clark, C. (1951) 'Urban population densities', Journal of the Royal
Statistical Society Series A 114(4):490-496. Verified. Surveys:
McDonald, J. (1989) 'Econometric studies of urban population
density: a survey', J. Urban Economics 26:361-385; Mills & Tan
(1980) Urban Studies 17:313-321.

Empirical regularity from ~36 cities across countries and
centuries: D(x) = D0·e^{−bx} fits residential density profiles, and
b falls over time in virtually every city as transport improves.
Not itself causal — AMM later derived it (Muth 1969 ch.4:
Cobb-Douglas + unit price elasticity of housing demand). Anas,
Arnott & Small (1998) call declining density with distance and
secular gradient decline "two of the strongest empirical
regularities" in urban structure.

As code: descriptive-only — this is the gate, not the process. via
would fit log-density vs network-distance-to-center on simulated
output and check b against era-specific empirical ranges (below),
exactly parallel to how the terrain side gates on slope-area theta.

Era dependence: b itself is the era-dependent observable —
~1.2/mile in 1880 US walking/horsecar cities down to ~0.1-0.4/mile
in 1970 auto cities (numbers in gates).

## Gate candidates

- **Density gradient b in D(x) = D0·e^{−bx}**, log-linear fit of
  density vs distance/network cost to center, era-resolved. US
  four-city average (Baltimore, Milwaukee, Philadelphia,
  Rochester): b ≈ 1.22/mile in 1880 falling monotonically to
  0.31/mile in 1963 (Mills' two-point estimates). US samples in
  1970: average b ≈ 0.38/mile (Edmonston 1975) and 0.12/mile
  (Mills & Ohta 1976); theory-side prediction γ ≈ 0.234/mile for US
  ~1970 and 0.318/mile for 1950 (a 26% model decline vs 41%
  observed decline 1950-1970, Edmonston). Walking-era cities: b of
  order 1-2/mile. Gate: simulated b must land in the era band and
  must decline as the era commute-speed parameter rises. Source:
  Mills, E.S. (1972) Studies in the Structure of the Urban Economy,
  Johns Hopkins Press; Edmonston (1975); Mills & Ohta (1976); all
  as compiled in Anas, Arnott & Small (1998) 'Urban Spatial
  Structure', J. Economic Literature 36(3):1426-1464 (values read
  directly from the paper's text).
- **Log-log elasticities of rent and density to distance, and
  implied transport-cost concavity** (modern global cross-section).
  Mean rent-distance elasticity ≈ −0.064; mean density-distance
  elasticity ≈ −0.356 (density gradient steeper than rent gradient
  in most cities — itself a testable ordering); implied
  transport-cost-distance elasticity θ ≈ 0.3 (strongly concave, not
  linear). Rent gradients flatter in smaller cities and in
  upper-middle-income-country cities. Gate for the modern-era
  regime. Source: Nöbauer, B., 'Rent, density, and transportation
  cost gradients in 734 cities' (working paper, Airbnb-based; PDF
  verified and text-extracted). Corroborating monocentric-model
  tests in 192 cities: Liotta, Avner, Viguié & Selod (2022)
  'Testing the monocentric standard urban model in a global sample
  of cities', Regional Science and Urban Economics 97.
- **Ordering and dynamics of employment vs population gradients.**
  Employment density gradient > population density gradient at any
  date, and the employment gradient falls faster over the 20th
  century; large modern US metros show ~20 identifiable employment
  subcenters each, with subcenter employment still a minority of
  total suburban employment (Chicago 1990: 558,600 in subcenters vs
  2,381,900 total suburban jobs). Gate for the Fujita-Ogawa layer:
  subcenter count and employment share must grow with the era speed
  parameter, not be authored. Source: Mieszkowski, P. & Mills, E.S.
  (1993) 'The causes of metropolitan suburbanization', J. Economic
  Perspectives 7(3):135-147; McMillen & McDonald subcenter counts;
  both as reported in Anas, Arnott & Small (1998) JEL.
- **Land-value surface shape and its century-scale evolution**
  (rent-gradient analog of the density gate). Chicago 1913-2010
  (Olcott's Blue Book, >600k points on 1/8-mile squares): strong
  negative-exponential land-value gradient from the CBD in 1913;
  gradient flattens mid-century; CBD value peak re-intensifies
  after ~1980 (center rebound). Use as shape target: log land value
  approximately linear in distance in the early auto era, with
  era-dependent slope; companion height data give log-linear
  height-distance profiles (Ahlfeldt & McMillen 2018 REStat
  100(5):861-875). Source: Ahlfeldt, G. & McMillen, D. (2014) 'Land
  values in Chicago, 1913-2010', Lincoln Institute Land Lines
  (dataset paper, verified); Ahlfeldt & McMillen (2018) 'Tall
  buildings and land values', Review of Economics and Statistics
  100(5).
- **Fractal dimension / cluster-size spectrum of the land-use
  pattern.** Urbanized area and individual use classes are
  fractal/bifractal; measured radial fractal dimension typically
  ≈ 1.7 (range ~1.4-1.9 across cities; Batty & Longley 1994 compile
  D ≈ 1.70 mean). White & Engelen (1993) show US cities
  (Cincinnati, Houston, Milwaukee, Atlanta) fit bifractal structure
  and their CA reproduces it. Gate: cluster-size distribution
  should be heavy-tailed (approx power law), not compact-blob.
  Source: White, R. & Engelen, G. (1993) Environment and Planning A
  25:1175-1199; Batty, M. & Longley, P. (1994) Fractal Cities,
  Academic Press.
- **NEGATIVE gate — commuting: do not require simulated commute
  lengths to match monocentric predictions.** Monocentric
  assignment predicts average US commutes of ~1 mile vs observed
  ~7x longer ('wasteful commuting', factor ≈ 7); even full
  polycentric job/housing geography leaves observed commutes ~3x
  the distance-minimizing assignment. Any via gate on commuting
  must use the observed excess-commuting ratio (~3), not
  bid-rent-optimal commutes. Source: Hamilton, B. (1982) 'Wasteful
  commuting', J. Political Economy 90:1035-1053; Small & Song
  (1992); Giuliano & Small (1993); as compiled in Anas, Arnott &
  Small (1998) JEL.
- **von Thünen ring ordering and deformation along corridors.**
  Qualitative-but-falsifiable: uses around each market settlement
  must order by transport-cost intensity (perishable/intensive uses
  innermost), ring boundaries must sit at bid-curve intersections
  given declared era freight rates, and iso-use boundaries must
  deform outward along low-cost corridors (rivers/roads) rather
  than remain circular. Historical anchors: documented rail-era
  crop zonation around 19thC market towns; Kopecky & Suen (2010, 'A
  quantitative analysis of suburbanization and the diffusion of the
  automobile') for transport-cost-driven boundary shifts. Source:
  von Thünen (1826/1966); empirical reviews in Sinclair (1967) AAAG
  and standard economic-geography literature.

## Pitfalls and contested claims

- Two-point density-gradient estimates (much of the historical
  1880-1963 record, including Mills' four-city numbers) are known
  to be crude; the field itself flags them (Anas, Arnott & Small
  1998 note "many of the density gradient estimates are based on
  just two points"). Use the era bands, not the point values, as
  gates.
- The declining density gradient is contested as a decentralization
  measure: modern polycentric cities fit the single negative
  exponential poorly, and central-density "craters" (Newling 1969)
  appear in auto-era CBDs. Gate the exponential fit only in regimes
  where the monocentric assumption holds
  (medieval/19thC/early-auto); switch to polycentric statistics
  (subcenter counts, employment share) for the modern era —
  otherwise the gate itself is misspecified.
- Isolating transport cost as the cause of gradient flattening has
  "not been very successful" empirically (Mieszkowski & Mills
  1993): value of time rises with wages, partially offsetting speed
  gains. via's era parameterization should use generalized cost
  (time·value-of-time + money), not raw speed, or it will
  overpredict flattening.
- Fujita-Ogawa has multiple equilibria at fixed parameters with
  catastrophic transitions; a deterministic solver silently
  performs equilibrium selection via its initial condition and
  sweep order. Defensible (seed with the prior era's pattern = path
  dependence), but it must be documented as a modeling choice, not
  presented as a unique prediction.
- CA land-use models have a calibration credibility problem the
  field itself documents: SLEUTH's brute-force calibration is
  computationally enormous, its dozens of goodness-of-fit metrics
  disagree (Clarke 2008 'unresolved issues'), coefficients are
  equifinal and non-transferable across cities, and it tends to
  overpredict dispersed edge growth. Do not import SLEUTH
  coefficients; if a CA layer is used, gate it on pattern spectra
  (fractal dimension, cluster-size tails), never on map-overlay fit
  to one historical city.
- White-Engelen neighborhood weight matrices m_{kj}(d) are in
  practice hand-tuned — functionally an authored look-table, which
  violates via doctrine unless every attraction/repulsion entry is
  derived from a citable externality mechanism or explicitly
  declared as config.
- UrbanSim-style microsimulation coefficients are
  place-and-period-specific with no transfer theory; re-using them
  for a generated or historical landscape is uncitable. Take its
  market-clearing architecture, not its numbers.
- The best modern rent-gradient dataset (Nöbauer's 734 cities) is
  built from Airbnb short-term rental prices — a proxy for
  residential rents with known tourist-district bias; treat its
  −0.064 rent elasticity as a soft band, and note its own finding
  that transport costs are strongly concave (θ ≈ 0.3),
  contradicting the linear-in-distance cost most simple
  implementations assume.
- Era changes are not all parametric: the rich-center-to-poor-
  center sorting flip (LeRoy & Sonstelie 1983) and the
  monocentric-to-polycentric transition (Fujita-Ogawa) are regime
  changes produced by parameter motion through bifurcations. "Era
  as parameters" holds only if the mechanisms are ones whose
  outputs undergo the structural change endogenously — hard-coding
  "medieval = rich center" would be a look-table.
- Commuting is the monocentric model's known empirical failure
  (factor ~7 underprediction, Hamilton 1982); land-value and
  density gates are trustworthy, commuting-flow gates from the same
  model are not.
- Medieval-era quantitative gradient data are thin: pre-1800
  density gradients rest on walls, tax rolls and Sjoberg-style
  qualitative structure, not fitted exponentials. Expect the
  medieval gate to be structural (compactness, steep edge at wall,
  rich-center sorting) rather than a numeric b-range — and say so
  in the ledger rather than inventing a number.
