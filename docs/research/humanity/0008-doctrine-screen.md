# Research 0008 — Doctrine screen: candidate tiers and refusals

Status: research corpus — pre-decision. This document records how an
adversarial screen sorted the surveyed mechanisms into via's tiers
(ADR 0003: process / forcing / interpretation) and what it rejected
outright. These are **candidate** assignments to be tested by future
ADRs, not rulings. Part of docs/research/humanity (see README).

Reading the table: *process* = a runnable, citable, falsifiable causal
rule; *forcing* = exogenous, declared in config, never hardcoded;
*interpretation* = descriptive/analytic, usable for gates and labels
only. A mechanism listed as process is still unadopted.

## Settlement systems

| Mechanism | Tier | Note |
| --- | --- | --- |
| Harris–Wilson BLV center dynamics (IPF flow balancing + dW/dt = ε(D_j − kW_j)) | process | Fully specified deterministic update; inputs (O_i from suitability, c_ij from corridors) exist in via. Best-fitting settlement engine. Gates must be distributional, never map-configurational (multi-equilibria, period-doubling — Osawa 2017). |
| Wilson entropy-maximizing spatial interaction | process | Deterministic IPF fixed point. α/β are NOT gated constants — every empirical study refits them per region/era; they enter as declared era forcing, with emergent distance-decay as the gate. |
| Allen–Sanglier dynamic central place ODEs | process | Runnable but structurally redundant with Harris–Wilson; adopt one engine, cite the other as convergent precedent. |
| Classical Christaller/Lösch central place theory | interpretation | Static equilibrium geometry, no update rule. Threshold/range primitives are absorbed into Harris–Wilson; geometric predictions survive only as weak advisory gates (never hexagons). |
| Periodic markets as continuous market-frequency state (Skinner regime) | process | Frequency as continuous intensity of W_j; the periodic→permanent transition must emerge from density and travel cost, never be an era switch. Discriminating test of the one-framework claim (frontier should partially skip periodicity). |
| New economic geography core–periphery (Krugman; Fujita–Krugman–Mori) | interpretation | Micro-founded but analytically confined to 2-region/racetrack geometries; role is the citable warrant for α>1 agglomeration inside Harris–Wilson, not an engine. |
| Zipf / Gabaix random-growth explanation | interpretation | Emergent target only; implementing multiplicative shocks as the engine authors the outcome. Gate on Gibrat statistics + tail band. |
| Bettencourt urban scaling laws | interpretation | Cross-sectional allometry, not dynamics. Measure generated geometry and regress; assigning Y = N^β is a look-table. |
| Radiation model of flows | interpretation | Parameter-free but derived from modern job search; premodern applicability untested; underperforms intra-urban. Modern-era null baseline only. |

## Urban morphology

| Mechanism | Tier | Note |
| --- | --- | --- |
| Conzen tripartite plan schema + persistence hierarchy (streets > plots > buildings) | process | Prescribes engine state (street graph, plot partition, footprints) with differential modification costs; the persistence ordering is a falsifiable emergent claim. Plan units emerge unlabeled — labeling is interpretation. |
| Burgage-cycle plot-coverage dynamics | process | Cheap per-plot saturation ODE + fallow/amalgamation operator — but a NOVEL formalization of a descriptive concept with no calibrated implementation; gate hard or it becomes an authored story. |
| Fringe-belt formation (bid-rent + building cycle + inertia) | process | Runnable; the boom/slump series B(t) must be declared forcing (Whitehand & Morton 2006 stress contingency); expect a noisy signal, gate leniently. |
| Parcel subdivision operator (frontage-maximizing OBB / straight-skeleton, Vanegas 2012) | process | Deterministic, implemented in literature; gates on frontage/area distributions. Era metrology (perch/chain/vara module) is forcing derived from the declared survey unit. |
| Caniggia "leading type" (typological process) | forcing | The type reduced to a small parameter vector (frontage, depth, storeys, coverage) per era is admissible forcing; the school gives no endogenous law for type mutation. Any geometry catalogue of house types is rejected (below). |
| Space syntax angular segment analysis (integration/choice) | process | Deterministic graph computation as coupling variable — a process-tier instrument. Gate on choice (~0.48 pooled), NOT integration (0.206); angular segment, never axial; buffer boundaries (Ratti edge-effect critique). |
| Natural movement / centrality-as-process feedback | process | The movement→land-use→movement loop is runnable — the interior analogue of via's flows→network coupling — but the multiplier strength has no published value; declared config parameter, flagged in the ledger. |

## Street and road networks

| Mechanism | Tier | Note |
| --- | --- | --- |
| Barthélemy–Flammini local-optimization street growth | process | Fully specified rule; via's suitability replaces their fiat center distribution P(r); metric-agnostic so terrain enters cleanly. The coupling voids their published statistics — re-gate on empirics. |
| Courtat potential-field morphogenesis (P_e, ω) | process | Strongest single street-fabric candidate: its two parameters are the organic↔planned axes the field itself uses; outputs all gateable. Validated on only 10 French towns — replace the potential with via's suitability, keep parameter semantics. |
| Helbing active-walker trail formation (macroscopic deterministic form) | process | PDE on via's grid for the pre-engineering corridor regime (footpaths, drove roads). Valid only before engineered construction — the handover is a declared forcing switch, not a fitted interpolation. |
| Louf–Jensen–Barthélemy cost–benefit intercity network growth | process | Deterministic argmax of R = B − C per new connection; d_ij from terrain least-cost paths. One effective parameter sweeps star→tree→MST. Era = cost/benefit ratio + distance-decay exponent, both citable forcing. |
| Sequential least-cost-path road building with reuse discount (Stahlberg et al., PNAS Nexus 2023) | process | Dijkstra LCP + sequential insertion + declared reuse discount; the archaeology-citable form of corridor consolidation. Composes with Louf (ordering economics) and Helbing (pre-engineering regime). |
| Strano densification/exploration decomposition | interpretation | Names and measures two link classes; supplies no update rule. Its statistics are the falsification battery — validated on ONE region, so tolerance bands, not law. |
| Masucci logistic street-network growth under green belt | interpretation | The logistic curve is an emergent gate: impose the boundary polygon as forcing and logistic N(t) must fall out. Imposing the curve authors the outcome. |
| Parish–Müller L-system city generation | interpretation | Cite only as engineering precedent (snap/extend/prune legalization; global-goals/local-constraints architecture). Pattern templates rejected (below). |
| Tensor-field street tracing | forcing | Admissible ONLY where the field is derived from declared inputs (survey baseline bearing, coastline, slope contours) and gated afterward. A painted field is authored content — rejected. |
| Block statistics / city fingerprints (Lämmer; Louf & Barthélemy) | interpretation | Outcome regularities used as gates on generated blocks, power law fitted over a declared window (lognormal alternative acknowledged). |

## Land use and density

| Mechanism | Tier | Note |
| --- | --- | --- |
| von Thünen bid-rent land allocation | process | Closed-form deterministic argmax per cell over network transport cost; rings and their corridor deformation are outcomes. Era freight rates and crop menus are forcing; the "30-mile haul limit" must emerge, never be hard-coded. |
| Alonso–Muth–Mills monocentric bid-rent | process | Closed-form/shooting solve on the network access-cost field; continuous rent/density/FAR spectra; gates on Clark/Mills gradient bands. Two income classes + mode costs make the medieval rich-center → modern poor-center flip emerge (LeRoy–Sonstelie). |
| Fujita–Ogawa endogenous polycentricity | process | Damped deterministic fixed-point iteration; t/α is the genuine era knob for monocentric→polycentric transitions. Equilibrium selection by prior-era seeding is a defensible path-dependence device but must be documented as a modeling choice. |
| White–Engelen constrained CA land-use transitions | process | Conditionally: the transition-potential update is runnable and its fractal outputs gateable — ONLY if every neighborhood-weight entry is derived from a citable externality or exposed as declared forcing. Hand-tuned matrices rejected (below). |
| SLEUTH urban growth CA | interpretation | Coefficients equifinal, city-specific, no forward era mapping (the field's own critique — Clarke 2008). At most a benchmark to beat. |
| UrbanSim microsimulation | interpretation | Architecture precedent (demand/supply/price-clearing loop) worth reimplementing with parametric forms; estimated coefficients cannot exist for a generated landscape or a medieval era. |
| Clark negative-exponential density law | interpretation | The gate, not the process — the monocentric model derives it. |

## Transport, demography, defense

| Mechanism | Tier | Note |
| --- | --- | --- |
| Marchetti/Zahavi travel-time-budget isochrone allocation | process | Isochrone-bounded accretion runnable on via's corridor graph; the ~1.0–1.3 h budget is declared forcing defensible only as an aggregate long-run regularity — gate the population mean, never per-agent. Use effective door-to-door speeds or cities over-size. |
| Müller / Newman–Kenworthy transport-era periodization | interpretation | Descriptive periodization; its content (streetcar stars, auto infill) must EMERGE from speed-anisotropy parameters. Era forcing applies to new accretion only, never restyles existing fabric. |
| Multimodal freight LCP catchments + transshipment nodes | process | Least-cost catchments over {wagon, river (navigability from via hydrology), canal, rail} with mode-change penalties; port/fall-line/railhead towns acquire centrality mechanically. Rate vectors per era are forcing; the mechanism is era-invariant. |
| Donaldson–Hornbeck market access | process | Pure graph computation plus one declared elasticity turning network edits into settlement growth/decline fields. Fogel's social-saving percentage stays an order-of-magnitude cross-check only (contested counterfactual). |
| Railroad water-stop / townsite founding + competitive culling | process | Candidate stations from two declared engineering constants (s_water 7–25 mi era-dependent, s_division ~100 mi); survival via the freight-catchment mechanism; diesel (a forcing event) triggers the historical die-off. Founding and culling modeled separately handles survivorship bias. |
| Wall perimeter-cost compactness constraint | process | Area-choice optimization (protection × threat − c_w × perimeter) with discrete rebuild events leaving fossil ring roads; threat(t) and c_w(t) are exogenous forcing. No compiled c_w series exists (0011). |
| Urban-graveyard mortality–density demographic equilibrium | process | dP = [b − d(ρ, W)]P + M with W from hydrology/lithology; the pre-modern size cap emerges, never hard-coded. Mortality-curve slope contested in magnitude — expose in config, gate the size DISTRIBUTION. |
| Height cap + rent-maximizing massing | process | Massing = marginal floor rent vs marginal cost per cell; the caps (walk-up ~5–6 storeys; elevator 1857; skeleton frame 1885; zoning envelopes) are dated technology/law forcing. |

## Founding and planning regimes

| Mechanism | Tier | Note |
| --- | --- | --- |
| Founding-act event stream (Beresford plantation, Ostsiedlung locator, townsite company) | forcing | Wave timing/intensity and actor identity are geopolitically contingent and uncitable as mechanism. Site selection scored on suitability/corridor fields, and the take/fail fill test, are the gated process parts; failed and partial plats are essential output. |
| Plat templates (Olynthus, bastide, Laws of the Indies, T-town, FHA grammar) | forcing | One era-invariant template schema (block module, street widths, plaza reservation, orientation law, lot module) with era/culture parameter values, including conditional rules (Indies: street width = f(climate)). Terrain clipping of the stamp is process. Ordinance dimensions are the mode of a distribution, not spec. |
| Law-family template inheritance (Magdeburg/Lübeck → daughters) | forcing | Parameter-bundle propagation over a declared transmission topology. Institutional content is not simulable; only the copying of declared bundles is representable. |
| Market-charter viability + spacing (Britnell/Bracton) | process | Catchment-viability survival test on the corridor cost field (reproduces the 1200–1349 overshoot-then-extinction); the charter event stream and the 6⅔-mile injury radius are forcing. Gate against evidenced-active markets, not raw grants. |
| Survey lattices (Roman centuriation, PLSS 1785) | forcing | One-time exogenous lattice written into landscape state; the corridor-cost discount along lattice lines is the process coupling that yields mechanistic palimpsest persistence (Po valley; Midwest section roads). |
| Speculative platting via bid-rent conversion threshold | process | Conversion when expected lot price exceeds agricultural value is era-invariant process, leaving the documented vacant-plat halo; the plat module derives from the era's declared survey unit (forcing). |
| Zoning / subdivision regulation (1916 NYC, Euclid 1926, FHA 1936–41) | forcing | Constraint layers on inherited plats; they generate nothing. Wedding-cake profiles must emerge from constraint + rent maximization ("wedding cake" itself is an interpretation label). Founding templates and regulation stay separate config blocks. |
| Informal settlement accretion | process | The same accretion + trail-consolidation process running wherever template forcing is absent, on the complement of formally suitable land (steep/flood cells from the nature side). The formalization fraction per era/region is the cleanest single era scalar. |

## Precedents and validation doctrine

| Mechanism | Tier | Note |
| --- | --- | --- |
| Household/settlement cost-minimizing location choice (Artificial Anasazi, VEP, site catchment) | process | Argmin of cost-weighted resource integrals (Tobler-cost kernels over suitability, water, fuel) with need-driven relocation. Janssen's replication is the standing warning: two calibration parameters carried the celebrated fit — multi-gate validation is mandatory. |
| Kvamme predictive-model gain | interpretation | Statistical association, no process — but converts to a gate: emergent placement scored against via's own covariates should reach G ~ 0.5–0.85. |
| MayaSim / MERCURY / SIMPOP / Schelling / percolation-DLA | interpretation | Architecture and method precedents only; SIMPOP's half-billion-run calibration is the identifiability warning; percolation/DLA are null models any gate must out-discriminate. |
| Pattern-oriented modeling (Grimm et al. 2005) | interpretation | The validation doctrine for the human side — the settlement equivalent of the θ/Hack/Horton battery: 3–6 independent gates per layer, joint passage, calibration/validation separation declared per gate. |
| Settlement scaling A ~ N^a (Ortman/Bettencourt) | interpretation | Closed-form constraint, not a simulation: the single best cross-era gate (exponent band era-invariant across two millennia; prefactor era-dependent). Needs nothing beyond via's own populations and footprints. |

## Refusals (the honest-refusal list)

Recorded now so no future ADR relitigates them silently. Each entry
names an idea the screen judged incompatible with via doctrine.

- **L-system street-pattern templates with named styles** ("radial",
  "raster"; the CityEngine lineage) as generative mechanism: named
  outcome look-tables with zero empirical validation (graphics
  papers, visual validation only). Only the snap/extend/prune
  legalization and the global-goals/local-constraints architecture
  are reusable, as engineering, with attribution.
- **Painted/authored tensor fields** for street layout: a
  designer-supplied field is authored content. Admissible only when
  derived from declared forcing and gated on network statistics.
- **Era house-type geometry catalogues** (Caniggia implemented as a
  shape library): an authored look-table of styles — and a claim to
  simulate culture, which the model cannot make.
- **Multiplicative random growth (Gibrat engine)** as the
  settlement-size generator: implements the target statistic (Zipf)
  directly — circular. Zipf/Gibrat are emergent gates.
- **Direct allometric assignment** (city GDP/road length = Y₀·N^β):
  scaling laws are cross-sectional gates, not dynamics.
- **Hexagonal Christaller lattices as generator**, or gating on
  strong spacing regularity (Clark–Evans R near 2.15): hexagonal
  regularity is essentially never observed (Dacey); the 2.15 bound
  is itself mathematically contested (Philo & Philo 2022).
- **Importing SLEUTH coefficients** or any CA calibration from a
  specific city: equifinal, non-transferable, no forward era mapping
  (Clarke 2008).
- **UrbanSim-style estimated behavioral coefficients**:
  place-and-period-specific econometrics with no transfer theory;
  the required micro-data cannot exist here. Architecture reusable,
  numbers not.
- **Hand-tuned White–Engelen neighborhood weight matrices**:
  functionally an authored look-table unless each entry is tied to a
  citable externality or exposed as declared forcing.
- **Endogenizing founding-wave timing** (Ostsiedlung, bastide boom,
  railroad boom): geopolitically contingent events with no accepted
  mechanistic model. Declared forcing; only site selection, spacing,
  and survival are gated process.
- **Hard-coding era regime facts as rules** ("medieval = rich
  center", "30-mile wagon limit", "2.5 km walking city",
  perch-quantized widths as law): each is an outcome the mechanisms
  must produce.
- **Gravity α/β presented as gated universal constants**: every
  empirical study refits them per region and era. Declared era
  forcing, with emergent decay as the gate.
- **Radiation model as the premodern flow mechanism**: derived from
  modern job-search behavior; untested extrapolation backward.
- **Commuting-flow gates from monocentric theory**: the model's
  known empirical failure (factor ~7 underprediction — Hamilton
  1982). Rent and density gates from the same model are trustworthy;
  commuting gates are not, except the excess-commuting ratio (~3).
- **Fogel social-savings percentage as a quantitative gate**: a
  contested counterfactual (Fogel 2.7% vs Fishlow ~15% of GNP);
  order-of-magnitude sanity check only.
- **Strong-form space syntax claims** (integration explains 60–80%
  of movement) presented as settled: the 2018 meta-analysis pooled
  effect for integration is 0.206; axial maps are analyst-dependent
  and degenerate on regular grids (Ratti 2004). Only angular-segment
  choice (~0.48) is robust enough to gate on, marked advisory.
- **A separate generator per era, or a global era switch that
  restyles existing fabric**: violates the palimpsest evidence
  (Conzen; Strano backbone persistence; coexisting fabrics) and the
  project's own thesis. Era forcing acts on NEW accretion; slow
  state (streets, plots, sunk capital) carries memory between eras —
  equilibrium-only re-solves would wrongly relocate cities (Bleakley
  & Lin frozen accidents).
- **Individual buildings' architecture, facades, or named styles**:
  below the model's causal resolution and unavoidably authored. The
  engine would stop at continuous massing fields (coverage, storeys,
  FSI/GSI); architectural style is downstream content.
- **Full NEG (Krugman) equilibrium as the running engine**:
  analytical results live on 2-region/racetrack geometries; a
  real-landscape port is unbudgeted research. Demoted to citable
  warrant for the agglomeration term.
