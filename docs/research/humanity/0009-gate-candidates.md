# Research 0009 — Gate candidates

Status: research corpus — pre-decision. These are candidate gates: a
falsification battery the human side could be held to, following the
pattern-oriented-modeling doctrine (Grimm et al. 2005 — several
independent gates per layer, passed jointly). No gate here is
adopted; bounds would be fixed by the adopting ADR. Part of
docs/research/humanity (see README).

A standing caveat from the screen applies to every distributional
gate below: all published bands are cross-sectional or ensemble
statistics, while via runs are single deterministic seeds — the
scoring protocol (n-seed ensembles vs widened bands) is an open
question (0011) that must be settled before any gate is scored.
*(Note, 2026-08-20: superseded as a scoring proposal by ADR 0008 —
scoring is per-character against reference populations, and the
ensemble question is resolved by its D5. Retained as the
empirical-range compilation.)*

## Core candidates

| Statistic | Empirical range | Source |
| --- | --- | --- |
| Upper-tail rank-size exponent ζ (Gabaix–Ibragimov rank-1/2 or Hill estimator, frozen delineation) with a Gibrat companion gate (growth mean/variance slope vs log size ≈ 0) | ζ ∈ 0.8–1.3 for mature systems (Soo: 0.90 cities proper, 1.17 agglomerations; Cottineau meta: mean 1.025, sd 0.28, ~40% of variance is technical choices); loosen tail for young systems, lean on the lognormal body (Eeckhout σ ≈ 1.75 US) | Soo 2005 RSUE 35:239; Gabaix 1999 QJE; Eeckhout 2004 AER; Cottineau 2017 PLOS ONE 12(8) |
| Settled area vs population scaling A ~ N^a across the generated system | a ∈ [2/3, 5/6], era-invariant (survived the Arcaute boundary critique); walled-city variant b ≈ 0.85 (band 0.75–0.95) | Ortman et al. 2014 PLOS ONE 9(2):e87902; Bettencourt 2013 Science; Cesaretti et al. 2016 PLoS ONE |
| Street-graph edge/node ratio e and meshedness M (declared 1-sq-mi sampling protocol) | e 1.05–1.69; M: tree suburbs <0.1, organic/medieval 0.15–0.26, planned grids 0.26–0.35; P(k=3)>P(k=4) self-organized, reversed for grids | Cardillo et al. 2006 PRE 73:066107; Buhl 2006; Boeing 2019 ANS 4:67 |
| Network length vs intersections L_tot ~ N^β, fitted per growth phase | β 0.49–0.54 (theoretical 1/2); plus N ≈ 0.019 × Pop linearity (~53 inhabitants/intersection, Groane) | Barthélemy & Flammini 2008 PRL 100:138702; Strano et al. 2012 Sci Rep 2:296 |
| Population density gradient b in D = D₀e^(−bx) vs network cost to center, era-resolved, monotone declining in era speed | walking/horsecar era ~1–2/mile (US four-city 1880: 1.22/mi); 1963: 0.31/mi; 1970 samples 0.12–0.38/mi; use era BANDS (two-point estimates are crude); switch to subcenter statistics in the modern polycentric regime | Mills 1972; Edmonston 1975; compiled in Anas, Arnott & Small 1998 JEL 36:1426 |
| Street orientation-order φ and the dead-end/4-way flip under platting and FHA forcing | regional means φ: US/Canada 0.427 vs Europe 0.033; P_4w US 0.334 vs Europe 0.172; pre-1940 grid-dominant vs post-1950 cul-de-sac flip with post-2000 partial rebound; φ is a spectrum, NOT a planned/organic classifier (multi-grid cities score low) | Boeing 2019 Applied Network Science 4:67; Boeing 2020 JAPA 87(1) |
| Cost-efficiency position relative to MST and greedy triangulation on the same node set | real fabrics reach E_rel ~ 0.7–0.8 at Cost_rel 0.24–0.40; dimensionless and era-spanning — medieval cheaper/slightly less efficient than grid-irons | Cardillo et al. 2006 PRE 73:066107 Table III |
| Rail-era town spacing along alignments (nearest-neighbor distribution), emergent from founding + culling | ~7–11 mi ordinary depot/water-stop spacing (Plains), division-point towns ~100 mi; the diesel forcing event must reproduce the die-off | Hudson 1985 Plains Country Towns; steam water-stop engineering records |
| Building-height field by era regime | pre-1857: flat 2–6 storey field, rent falling with floor; 1857–1885 transitional ~10–12; post-1885 rent-following height peak at max-access nodes; under 1916-style zoning, setback breaks cluster at k × street_width for declared k ∈ {1..2.5} | Otis/Haughwout 1857; Home Insurance 1885; NYC Building Zone Resolution 1916; Barr 2016 |
| Plot-frontage histogram structure per regime: discrete modal module + rational multiples, lognormal family | medieval mode ~28–32 ft with ½, ¾, 1.25× peaks (soft prior — perch variants and a documented confirmation-bias critique); Chicago 25 ft; FHA 60–100 ft; frontage distributions lognormal (Usui & Asami) | Conzen 1960; Slater 1981 Area 13(3); Usui & Asami 2018 JGS 20; Chicago 1830 plat; FHA TB7 1938 |

## Advisory candidates

| Statistic | Empirical range | Source |
| --- | --- | --- |
| Block/cell area tail P(A) ~ A^(−τ) over a declared window, with maturity drift | τ ~ 1.9–2.0 mature urban; drifts 1.2 → 1.9 with urbanization (Groane); joint property of network AND settlement layers; lognormal alternative acknowledged — soft gate | Lämmer et al. 2006 Physica A 363:89; Louf & Barthélemy 2014 JRSI; Strano et al. 2012 |
| Backbone persistence + densification/exploration bimodality over a run's history | ~90% of top-100 betweenness routes persistent over ~170 y (60% of top 1000); exploration mode shrinks as land is consumed — validated on ONE region, survey-bias caveat: tolerance bands | Strano et al. 2012 Sci Rep 2:296 |
| Urban scaling exponents on the generated system (declared delineation + fitting/noise model) | infrastructure β 0.75–0.9 (theory 5/6); socioeconomic/interaction 1.1–1.3 (theory 7/6); individual needs ~1.0; boundary-sensitive (Arcaute 2015) hence advisory | Bettencourt et al. 2007 PNAS 104:7301; Bettencourt 2013 Science 340; Arcaute et al. 2015 JRSI |
| Clark–Evans R for same-tier settlement spacing (edge-corrected) | full sets R ~ 1; upper-tier centers R ~ 1.1–1.5 weak regularity only — never gate toward hexagonal 2.15 | Clark & Evans 1954 Ecology 35; Dacey 1962; Hodder & Orton 1976 |
| Pre-modern settlement-size distribution and water-conditional maximum | bulk 5,000–20,000; tail to ~10⁵ only with navigable-water grain supply; equilibrium sizes migration-fed (graveyard-effect magnitude debated — distribution gate, not a per-city cap) | Bairoch, Batou & Chèvre 1988; Jedwab, Johnson & Koyama 2021 RSUE |
| FSI–GSI(–L) density-plane occupancy per era run | generated fabric must land in empirically occupied regions (historic cores high GSI / L 2–4; modernist low GSI / high L) and move through the plane in the observed direction as era parameters change | Berghauser Pont & Haupt 2010/2021 Spacematrix, TU Delft OPEN |
| Population-mean daily travel time (emergent over the synthetic population) | 1.0–1.3 h/person/day across eras; aggregate only — individual-level constancy is contested (Mokhtarian & Chen 2004) | Marchetti 1994 TFSC 47:75; Zahavi & Ryan 1980; Schafer & Victor 2000 |
| Angular-segment choice vs generated movement/land-use intensity | pooled correlation ~0.48 for choice (integration 0.206 — do not gate on it); angular representation and buffered boundary mandatory | Hillier & Iida 2005 COSIT; Sharmin & Kamruzzaman 2018 Transport Reviews 38(4) |
| Logistic N(t)/E(t) emergence under declared growth-boundary forcing | logistic fits R² > 0.99 (London 1786–2010, 9 slices); trunk/minor split: A/B backbone grows ~4× while minor roads absorb growth | Masucci, Stanilov & Batty 2013 PLoS ONE 8:e69469 |
| Kvamme gain of emergent settlement placement against via's own covariates | G ~ 0.5–0.85 band from the predictive-modeling literature; survey bias in the empirical bands acknowledged | Kvamme 1988; Yaworsky et al. 2020 PLOS ONE 15(10); Verhagen & Whitley 2012 |
