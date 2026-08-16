# Research 0005 — Transport technology & city form across eras

Status: research corpus — pre-decision. Nothing in this document is
adopted; adoption of any mechanism requires its own ADR. Part of
docs/research/humanity (see README there).

## The question this literature answers for via

How the speed and cost structure of the dominant transport mode sets
the size, shape, spacing, and vertical form of settlements — informing
the corridors, settlements, flows, network, and morphology layers of
the causal chain. The era range runs from the pre-modern walking city
(walls, wagon hauls, sanitation caps) through canal/rail (1820s–1890s)
to streetcar and automobile metropolitan form (1890–1945+), with a
recurring finding: the mechanisms are era-invariant and era enters
almost everywhere as a parameter or dated technology forcing.

Note on tier language: phrases like "process-tier" and "descriptive"
below are the source sweep's own simulability judgements, reproduced
faithfully. They are not via tier assignments; the doctrine screen is
a separate document (0008).

## Models and mechanisms

### Marchetti's constant / Zahavi travel-time budget

Marchetti, C. (1994) 'Anthropological Invariants in Travel Behavior',
Technological Forecasting and Social Change 47:75-88 (verified, IIASA
RR-95-04); Zahavi, Y. (1979) 'The UMOT Project', US DOT/World Bank;
Zahavi & Ryan (1980); Schafer & Victor (2000) 'The future mobility of
the world population', Transportation Research A 34:171-205.

Humans hold total daily travel time roughly invariant at ~1.0–1.3
h/person/day (Zahavi's travel-time budget; Marchetti's "one hour"). A
settlement's practical extent is therefore the ~30-minute one-way
isochrone from its functional core at the prevailing door-to-door
speed of the dominant mode. Extent scales linearly with mode speed:
walking at 5 km/h gives a ~2.5 km radius; each transport transition
multiplies reachable area by (v_new/v_old)². Zahavi adds a travel
money budget (~10–15% of income) that couples mode choice to income,
selecting which speed is "prevailing".

As code: the sweep rates this process-tier, directly simulable.
State: settlement core node(s) on the corridor/road graph; per-edge
speed v(edge_class, era). Update: compute the shortest-time isochrone
T = 0.5 h (config) from the core over the multimodal cost surface
(off-network cells get walk speed penalized by terrain slope, already
in via's cost model); allocate new built-area increments only to
cells inside the isochrone, weighted by suitability. Inputs: corridor
graph from via's corridor stage, terrain slope for off-road walk
cost. The gate is geometric: emergent built-area radius vs era speed
must track linearly.

Era dependence: era is almost purely a parameter: v_walk ~5 km/h (all
pre-1850 eras), horsecar ~6–8 mph (10–13 km/h), electric streetcar
~2× horsecar on corridors (Warner: radius 2 → 4 → 6–10 mi, Boston
1850–1900), automobile ~40–50 km/h door-to-door. One structural
nuance, not a new model: pre-auto fast modes are corridor-bound
(anisotropic — speed lives on graph edges, walk-access ~800 m to
stops), auto speed is quasi-isotropic on a dense road grid. The same
isochrone code handles both; only the speed field's support changes.

### Transport-era periodization of metropolitan form

Muller, P.O. (1995, orig. 1986) 'Transportation and Urban Form:
Stages in the Spatial Evolution of the American Metropolis', in
Hanson (ed.) The Geography of Urban Transportation, Guilford,
pp. 26-52 (verified); Newman, P. & Kenworthy, J. (1989) Cities and
Automobile Dependence, Gower; (1999) Sustainability and Cities,
Island Press; Newman, Kosonen & Kenworthy (2016) 'Theory of urban
fabrics', Town Planning Review 87(4) (verified).

Muller: four intrametropolitan transport eras each produce a
distinctive accretion pattern — Walking/Horsecar (1800–1890, compact
pedestrian city), Electric Streetcar (1890–1920, star-shaped growth
along trolley corridors with walk-to-stop fingers), Recreational
Automobile (1920–1945, interstitial infill between the fingers),
Freeway (1945–, dispersed low-density extension). Newman & Kenworthy:
the three fabrics coexist in every modern city as concentric or
overlapping layers — walking fabric ~2.5 mi (~4 km) diameter, transit
fabric up to ~25 mi (~40 km), automobile fabric up to ~50 mi — each
laid down while its mode dominated, each with characteristic density
and land use.

As code: the sweep calls this mostly a descriptive periodization, but
its content is simulable as the consequence of the Marchetti
mechanism plus corridor anisotropy: run growth accretion under
era-indexed speed fields and the star (streetcar) vs infilled-disc
(auto) shapes must emerge rather than be authored. State: built-cell
raster with an era-of-accretion stamp (a continuous field, not a
style label). Gate: shape statistics of accretion rings —
corridor-elongation/compactness index of 1890–1920 growth vs 1945+
growth; density-gradient steepness per ring. Do not implement as a
lookup of "era styles" — implement only speeds and let the fabric
follow.

Era dependence: era enters as (a) the speed/anisotropy parameters of
the dominant mode (parameter), and (b) the dates at which modes
become available (exogenous technology forcing, declared in config).
No structural model change across eras; the periodization's US dates
(1890, 1920, 1945) are forcing constants that differ by country and
should be config, not code.

### Freight cost structure and the wagon-haul limit

Taylor, G.R. (1951) The Transportation Revolution 1815-1860,
Rinehart; US Senate report (1852) wagon cost estimate; von Thunen,
J.H. (1826) Der isolierte Staat; antebellum freight-rate dataset:
European Review of Economic History (2025, county-level US freight
rates 1820-1860).

A good is marketable only where price at market exceeds production
cost plus t·d (rate × distance), with t differing by orders of
magnitude across modes. Verified rates: wagon ~15 c/ton-mile (US
Senate 1852; ~70 c/ton-mile in 1816, when 30 miles overland cost as
much as shipping to England); canal ~1 c/ton-mile (1850s); rail 2–9
c/ton-mile (1850s), falling toward ~1.5 c and below by the
1880s–90s. For grain, the wagon rate implies an economic haul limit
of roughly 20–40 km (the classic "~30 miles") to the nearest water or
rail transshipment point; 1817 Buffalo → NYC wagon carriage cost 3×
the wheat's value, 6× for corn. Rail and canal collapse the t of
long-haul by ~10–30×, converting the transport surface from "water is
everything" to a graph of low-cost corridors.

As code: process-tier per the sweep, directly simulable, and it is
the settlement-siting engine: multimodal least-cost-path over
{walk/wagon on a terrain-slope-weighted grid, navigable river reaches
from via's hydrology (flow above threshold, slope below threshold),
canal edges, rail edges} with per-mode cost per tonne-km and
transshipment penalties at mode-change nodes. State: cost-to-market
field, catchment partition. Update: settlement/market viability =
integral of producible surplus inside the profit > 0 catchment.
Transshipment nodes (river confluence + fall line, canal terminus,
railhead) acquire high centrality mechanically — port and rail towns
emerge. Inputs: hydrology (navigability), terrain, era mode set and
rates from config.

Era dependence: era is a cost-vector parameter: {wagon 15–70, canal
~1, rail 9 → 1.5, truck ~} c/ton-mile (declared per era in config),
plus which edge types exist (forcing: canals after ~1820s, rail after
~1840s locally). The mechanism (least-cost catchments, transshipment
centrality) is era-invariant. The haul "limit" must be derived from
value density / rate, never hard-coded, because it differs by crop
(wheat vs corn, 2×).

### Market access and the Fogel social-savings tradition

Fogel, R.W. (1964) Railroads and American Economic Growth: Essays in
Econometric History, Johns Hopkins (verified: agricultural social
saving <=2.7% of 1890 GNP; 0.6% interregional + 2.1% intraregional);
Fishlow, A. (1965) American Railroads and the Transformation of the
Ante-bellum Economy, Harvard (~4% of GNP 1859, extrapolating to ~15%
by 1890); Donaldson, D. & Hornbeck, R. (2016) 'Railroads and American
Economic Growth: A Market Access Approach', QJE 131(2):799-858.

Fogel's counterfactual: remove railroads, expand canals and wagons,
and measure the cost difference — small in aggregate (<=2.7% GNP) but
spatially decisive: land more than ~40 miles from navigable water
drops out of commercial agriculture. Donaldson–Hornbeck reformulate
as market access MA_i = Σ_j pop_j · τ_ij^(−θ), with τ from multimodal
least-cost freight routes; county land values and population growth
respond elastically to MA changes (removing rail in 1890 lowers total
US agricultural land value ~60% in their central estimate).

As code: process-tier per the sweep. MA_i is a pure graph
computation: pairwise least-cost freight costs τ_ij over the era's
multimodal network (same cost engine as the von Thunen mechanism),
then MA as a weighted sum; settlement population update dP_i
proportional to d(log MA_i) with a single elasticity. Deterministic,
seedable, cheap at via's scales with Dijkstra from settlement nodes.
This is the natural flows-stage coupling: it turns network edits (a
new rail line) into settlement growth/decline fields.

Era dependence: parameter only: the network edge set and per-mode
rates are era forcing; θ (trade elasticity) and the MA-growth
elasticity are calibration constants that Donaldson–Hornbeck estimate
for the 19th century and should be declared per era in config. The
social-saving percentage itself is an accounting output, not a model
input — usable as a magnitude sanity check, not a gate.

### Railroad-imposed town founding and spacing

Hudson, J.C. (1985) Plains Country Towns, University of Minnesota
Press; steam-era water-stop engineering (water tanks 7-10 miles apart
early era; ~25 miles for 1870s Kansas Pacific locomotives with
1,800-1,900 gal tenders; division points ~100 miles where
crews/engines changed); Union Pacific corridor town-origin record
(thousands of towns originating as depots/water stops).

Steam locomotives required water every 7–25 miles (era- and
boiler-dependent) and crew/servicing division points every ~100
miles; railroads also platted townsites at regular intervals to
maximize land sales and grain-elevator catchments (Hudson documents
deliberate ~8–10 mile spacing on Plains lines, roughly a half-day
wagon round trip for farmers). Result: quasi-periodic town spacing
along rail alignments with a size hierarchy — ordinary stops small,
division points larger — imposed by the network rather than by
terrain.

As code: process-tier per the sweep. Along each rail alignment
(itself from via's corridor stage), place candidate station nodes at
an interval s drawn deterministically from era tech (s_water(era)
plus s_division ~100 mi for larger seeds), then let candidates
compete through the freight-catchment mechanism — towns whose
catchment surplus falls below viability are culled (this reproduces
the historical die-off when diesel removed water stops). State:
station nodes with catchment surplus. Inputs: rail graph,
agricultural suitability field. No authored placement; spacing
emerges from two declared engineering constants.

Era dependence: s_water is a parameter that grows with locomotive
technology (7–10 mi early steam → ~25 mi 1870s → unbounded with
diesel c. 1940s, a forcing event that triggers culling, not a new
model). The platting-interval motive (wagon-access radius of farmers)
reuses the wagon cost parameter from the freight mechanism — a good
cross-mechanism consistency check.

### Wall-perimeter cost as a pre-modern compactness constraint

Ioannides, Y. & Zhang, J. (2017) 'Walled cities in late imperial
China', Journal of Urban Economics 97:71-88 (verified via Clark U.
copy); Dincecco, M. & Onorato, M. (2016) 'Military conflict and the
rise of urban Europe', Journal of Economic Growth 21:259-282;
Cesaretti et al. (2016) 'Population-Area Relationship in Medieval
European Cities', PLoS ONE 11(10) (SFI working paper 15-10-036).

Wall cost scales with perimeter (~area^0.5 for compact shapes) while
protected value scales with area, so fortified settlements minimize
perimeter per unit area: near-circular compact footprints, high
interior density, and lumpy growth (expansion requires a discrete,
expensive wall-rebuild event; suburbs accrete extramurally between
rebuilds). Dincecco–Onorato: war threat drives population into walled
"safe harbors", coupling threat intensity to urban concentration.
Cesaretti et al. give the empirical population–area scaling for 173
medieval European cities: A ~ P^0.85 (superlinear density growth with
size).

As code: process-tier per the sweep, with one forcing input. State:
settlement footprint polygon + wall polygon + interior density field.
Update: given population P, era wall unit cost c_w, and threat level,
choose enclosed area A maximizing utility (protection benefit ×
threat − c_w · perimeter(A)); density inside = P/A; when P exceeds a
rebuild threshold, emit a wall-expansion event (new ring) — old wall
lines persist as ring roads in the street graph (a real, observable
morphological fossil: the Vienna Ringstrasse pattern). Inputs:
threat(era) and c_w(era) from config. Emergent gates: compactness
index of pre-modern footprints, density jump at the wall line,
population–area exponent ~0.85.

Era dependence: threat level and c_w are exogenous forcing. Artillery
(trace italienne, 16th–17th c.) multiplies effective c_w several-fold
(bastions massively increase cost per protected area), further
squeezing compactness; the modern era sets threat ~0, removing the
constraint entirely — wall lines remain only as inherited graph
structure. Structural mechanism unchanged across eras; only (threat,
c_w) move.

### Sanitation/water/food-shed carrying capacity (urban graveyard)

Bairoch, P. (1988) Cities and Economic Development: From the Dawn of
History to the Present, U. Chicago Press; Bairoch, Batou & Chevre
(1988) La population des villes europeennes 800-1850 (2,204-city
dataset); Jedwab, R., Johnson, N. & Koyama, M. (2021) 'Medieval
cities through the lens of urban economics', Regional Science and
Urban Economics (verified GWU working paper 2020-9); Wrigley, E.A.
(1967) on London's demographic sink.

Pre-modern cities above ~5,000 inhabitants ran negative natural
growth (density-dependent mortality from waterborne disease with no
sewerage), so city size was a migration-fed equilibrium capped
jointly by (a) the mortality–density gradient, (b) potable water
supply, and (c) the food shed reachable at wagon/boat cost (linking
back to the freight mechanism). Outcome: most medieval cities
5,000–20,000; the largest (Paris ~200k c. 1300, Naples/Rome ~50k in
800) required exceptional water transport for grain. The cap
dissolves with 19th-century water/sewer engineering (post-1850s),
after which natural increase turns positive and size decouples from
local carrying capacity.

As code: process-tier per the sweep. Population update per
settlement: dP = [b − d(ρ, W)] P + M, with mortality d rising in
density ρ and falling in water capacity W (W computable from via's
hydrology: upstream discharge, spring/well proxy from lithology), and
migration M drawn from the rural surplus of the freight catchment.
Equilibrium P* emerges; no cap is hard-coded. Deterministic ODE step
per settlement per epoch. Gate: pre-modern equilibrium sizes must
land in the Bairoch distribution (log-normal-ish, bulk 5–20k, tail to
~10^5 only on navigable water).

Era dependence: the mortality function's parameters are era forcing:
pre-modern d(ρ) steep; post-sanitation (declared date/tech level in
config) d flattens and b − d > 0. Structural form of the update is
identical across eras — exactly the "era as parameter" pattern. The
food-shed component inherits era freight costs automatically.

### Building-technology height limit (walk-up cap → elevator + frame)

Otis safety elevator 1852-1854, first passenger installation E.V.
Haughwout Building, New York, 1857 (verified, Guinness/EBSCO); Home
Insurance Building, Chicago, 1885, first metal-skeleton-frame tall
building, 10 storeys/138 ft (verified, contested 'first skyscraper'
status); Bernard, A. (2014) Lifted: A Cultural History of the
Elevator, NYU Press; Barr, J. (2016) Building the Skyline, Oxford UP
(economics of Manhattan heights).

Before mechanical vertical transport, willingness to climb capped
economic building height at ~5–6 walk-up storeys (~20 m), with rent
decreasing with floor number; masonry bearing walls added a
structural cap (taller means impossibly thick ground-floor walls).
The 1857+ passenger elevator inverted the rent gradient (top floors
premium) and the 1885+ metal skeleton frame removed the structural
cap, making height an economic variable set by land rent vs
construction-plus-elevator cost — CBD floor-area ratios and skylines
follow land value peaks thereafter.

As code: process-tier per the sweep, at the morphology stage,
trivially: per-cell allowed height h_max(era) = min(structural cap,
access cap); pre-1857 both ~5–6 storeys; 1857–1885 access cap lifted
but structure caps ~10–12; post-1885 h chosen per cell by rent
maximization: build floors while marginal floor rent (from the
settlement's land-value/access field, which via's flows stage
produces) exceeds marginal cost (increasing in h). Output is a
continuous height field — a spectrum, not building styles. Gate:
pre-modern synthetic cities must show flat ~2–6 storey height fields;
modern ones a rent-following height peak at maximum-access nodes.

Era dependence: pure parameter/forcing: (access_cap, structural_cap,
cost-vs-height curve) declared per era; two dated technology events
(1857 elevator, 1885 skeleton frame) are config forcing. Planning law
(height/zoning limits, e.g. the 1916 New York setback ordinance) is
additional forcing-tier input in the modern era. No structural model
change.

## Gate candidates

| Statistic | Empirical range | Source |
| --- | --- | --- |
| Daily per-capita travel time budget (emergent mean over synthetic population) | 1.0–1.3 h/person/day, stable across eras, cities, and modes (Marchetti's ~1.0 h; Zahavi & Ryan 1.1–1.3 h; Schafer & Victor confirm cross-nationally) | Marchetti 1994 TFSC 47:75-88; Zahavi & Ryan 1980; Schafer & Victor 2000 TR-A 34:171-205 |
| Built-area radius vs dominant-mode speed (linear law: R ~ v × 0.5 h) | Walking city radius ~2.5 km (5 km/h); Boston practical commute radius 2 mi (walk, 1850) → ~4 mi (horsecar ~8 mph) → 6–10 mi (electric streetcar, 1900); auto metro radius 20–40 km; N&K fabric diameters ~4 km / up to 40 km / up to 80 km | Marchetti 1994; Warner 1962 Streetcar Suburbs (Boston 1870-1900); Newman, Kosonen & Kenworthy 2016 TPR 87(4) |
| Freight cost per tonne-km by mode (input parameters, but their ratios are checkable against emergent catchments) | Wagon ~70 c/ton-mile (1816) to 15 c (1852 US Senate est.); canal ~1 c (1850s); rail 2–9 c (1850s) → ~1.5 c and below by 1880s–90s; implied wagon:rail ratio ~10:1, wagon:canal ~15–30:1 | Taylor 1951 The Transportation Revolution; US Senate 1852 report; antebellum freight-rate dataset EREH 2025 |
| Emergent grain-catchment radius of pre-rail market/river towns (wagon-haul limit) | ~20–40 km (the classic ~30 miles); 1817 benchmark: wagon carriage Buffalo → NYC = 3× wheat value, 6× corn value — the limit must scale with crop value density | Taylor 1951; standard economic-history freight benchmarks (1816 30-mi-overland = transatlantic cost parity) |
| Town spacing along steam-era rail alignments (nearest-neighbor distance distribution + coefficient of variation) | Regular ~7–10 mile spacing (early water stops and Plains platting; Hudson documents ~8–10 mi); ~25 mi water range for 1870s Kansas Pacific locomotives; division-point towns (larger) at ~100 mi intervals | Hudson 1985 Plains Country Towns; Kansas Pacific operating records (CPRR discussion, primary locomotive specs); steam water-stop engineering literature |
| Population–area scaling exponent of pre-modern (walled) settlements | A ~ P^b with b ≈ 0.85 (superlinear density; 173 medieval European cities); walled-city interior density significantly above extramural density | Cesaretti, Lobo, Bettencourt, Ortman & Smith 2016 PLoS ONE 11(10) (SFI WP 15-10-036); Ioannides & Zhang 2017 JUE 97:71-88 |
| Pre-modern city-size distribution and maximum size conditional on water access | Bulk of medieval cities 5,000–20,000; largest c. 800 ~50k (Rome, Naples); ~200k (Paris c. 1300) only with waterborne grain supply; natural growth negative above ~5k (urban graveyard), so growth is migration-fed | Bairoch, Batou & Chevre 1988 (2,204-city dataset, 800-1850); Jedwab, Johnson & Koyama 2021 RSUE |
| Building height field by era (mean and max storeys) | Pre-1857: ~5–6 storey walk-up cap (~20 m), rent falling with floor; 1885+: 10 storeys/138 ft (Home Insurance Bldg) then rapid growth; modern heights set by land rent, peaked at max-access nodes | Otis/Haughwout 1857 installation record; Home Insurance Building 1885 (Chicago); Barr 2016 Building the Skyline |
| Aggregate transport-cost sensitivity of settlement system (magnitude sanity check, not a hard gate) | Removing rail from the 1890 US network: social saving <=2.7% GNP (Fogel, agricultural: 0.6% inter- + 2.1% intraregional) but ~60% loss of agricultural land value via market access (Donaldson–Hornbeck); Fishlow ~4% GNP for 1859 | Fogel 1964; Fishlow 1965; Donaldson & Hornbeck 2016 QJE 131(2):799-858 |
| Shape/anisotropy of growth rings by era (compactness or corridor-elongation index of accretion stamped by era) | Streetcar-era (1890–1920) accretion star-shaped along corridors with ~800 m (10-min walk) lateral reach to stops; freeway-era (1945+) accretion quasi-isotropic infill and extension — qualitative but quantifiable on the synthetic raster; no single published exponent | Muller 1995 (in Hanson ed., Geography of Urban Transportation); Warner 1962; Newman, Kosonen & Kenworthy 2016 |

## Pitfalls and contested claims

- Travel-time-budget constancy is contested at the individual/city
  level: Zahavi himself called the TTB "stable and predictable, but
  not necessarily constant", and Mokhtarian & Chen (2004, TR-A, 'TTB
  or not TTB') find it varies ~1.0–1.5 h with income, city speed, and
  demographics. It is defensible only as an aggregate long-run
  regularity — gate on the population mean over the whole settlement
  system, not per-agent.
- Newman–Kenworthy/Muller eras are US/Western periodizations; the
  dates (1890/1920/1945, or 1850s/1950s) are not universal and the
  fabrics coexist within one city — "era" must act as forcing on new
  accretion at time t, never as a global regime switch that restyles
  existing fabric. Getting this wrong produces three separate
  generators through the back door.
- The "~30-mile wagon limit" is a stylized number, not a law: it
  varies ~2–6× with crop value density (wheat vs corn) and road
  quality. Hard-coding 30 mi would be an authored look-table; the
  limit must fall out of rate × distance vs value. Same for "radius
  2.5 km walking city" — derive from speed × time, don't assert.
- Fogel's social savings is a counterfactual accounting exercise,
  methodologically contested (Fishlow's extrapolation gives ~15% GNP
  for 1890 vs Fogel's 2.7%; the debate turns on canal-expansion
  counterfactual assumptions and demand elasticity). Use the
  market-access formulation (Donaldson–Hornbeck) for mechanism, and
  treat the social-saving % only as an order-of-magnitude
  cross-check.
- Railroad town spacing has survivorship bias: railroads platted far
  more towns than survived, and spacing was set jointly by water-stop
  engineering and townsite land-sale economics (Hudson). Gating on
  observed modern spacing conflates founding rule with subsequent
  culling — the sim should reproduce spacing through founding +
  competition + diesel-era culling, and be gated on the founding-era
  plat record where available.
- Bairoch's medieval population figures carry large error bars
  (pre-census estimates from tax rolls, hearth counts); the Cesaretti
  et al. b ~ 0.85 exponent is fitted on those uncertain data and on
  wall-enclosed area, which undercounts extramural suburbs. Treat as
  a soft gate (exponent in ~0.75–0.95), not a tight one.
- Wall-constraint causality runs both ways: walls compress density,
  but dense rich cities are also the ones that could afford walls
  (Dincecco–Onorato's threat channel). The sim's wall mechanism is
  defensible only with threat as declared exogenous forcing; do not
  infer threat from outcomes.
- Home Insurance Building's "first skyscraper" status is contested
  (hybrid iron/steel frame, partially load-bearing masonry); the
  robust claim is the joint arrival of safety elevator (1857) +
  skeleton frame (1880s) as the height-cap removal event. Use the
  technology dates as forcing, avoid "first" claims.
- Effective door-to-door speeds are much lower than vehicle top
  speeds (streetcar ~10–15 km/h effective incl. stops/access vs ~30
  km/h cruising; auto ~30–50 km/h effective in metro traffic vs 100+
  free-flow). Calibrating isochrones with vehicle speeds instead of
  effective speeds systematically over-sizes cities — the classic
  implementation error in Marchetti-style models.
- Pre-modern water/sanitation carrying capacity is a
  demographic-equilibrium claim, not a hard wall: the "urban
  graveyard" effect's magnitude is debated (some recent work finds
  near-zero rather than strongly negative natural growth in smaller
  towns). Implement as a mortality-density gradient with uncertain
  slope exposed in config, and gate on the resulting size
  distribution, not on any single city's cap.
