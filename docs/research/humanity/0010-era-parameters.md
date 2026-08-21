# Research 0010 — The era-parameter table

Status: research corpus — pre-decision. This table collects the
concrete numbers that would let ONE mechanism set span the target
eras (medieval / 19th-century frontier / modern) as declared tier-2
forcing — the same pattern uplift and lithology use on the nature
side. None of these values is adopted; the adopting ADR would fix
units, real-terms normalization (0011), and which parameters are
dated switches vs continuous paths (0011). Part of
docs/research/humanity (see README).

Eras are shorthand columns, not categories the engine would know: an
era is a point in this parameter space, and mixed/transitional
configurations are legitimate (that is the point).

## Movement and interaction

**Effective door-to-door passenger speed (dominant mode) + anisotropy**
(Marchetti 1994 TFSC 47:75–88; Warner 1962 Streetcar Suburbs;
Newman–Kenworthy fabric radii)
- medieval: walk 4–5 km/h, isotropic (terrain-slope-modified);
  pack/cart slower off-road
- frontier 19c: horsecar 10–13 km/h, electric streetcar ~2× on
  corridors only (anisotropic; ~800 m walk access to stops);
  intercity rail 30–60 km/h on edges
- modern: auto 30–50 km/h effective metro (quasi-isotropic on a
  dense grid; free-flow 100+ is the classic over-sizing error)

**Daily travel-time budget T (isochrone half-width)**
(Marchetti 1994; Zahavi & Ryan 1980; Schafer & Victor 2000 TR-A 34)
- all eras: ~1.0–1.3 h/day (0.5 h one-way) — the deliberately
  era-INVARIANT constant; extent scales as v × T

**Freight cost per ton-mile by mode** (declare as within-era relative
prices, not cross-era nominal — see 0011)
(Taylor 1951 The Transportation Revolution; US Senate 1852;
AAR/Costmine 2022–23 and Steel Wheel Logistics 2020s benchmarks,
verified at survey time)
- medieval: wagon/cart only, 15–70 ¢/ton-mile (1816–1852 USD
  benchmarks); water ~1/15th–1/30th of wagon where navigable
- frontier 19c: wagon 15 ¢; canal ~1 ¢; rail 2–9 ¢ falling to
  ~1.5 ¢ by the 1880s–90s (wagon:canal:rail ≈ 30:1:3)
- modern: rail ~4.8 ¢/ton-mile (2022), truck 8–25 ¢ (2020s USD);
  transport share of delivered value near-negligible for most goods

**Maximum admissible route grade (least-cost-surface hard constraint)**
(WV Historic Turnpikes, Aurora LLC 2008; Trains.com "How railroads
design grades"; ruling-gradient references, verified at survey time)
- medieval: foot/pack — steep tolerated (>10%; soft number, no
  engineering standard exists; needs a primary source — 0011)
- frontier 19c: wagon/turnpike ~5% practical ceiling (verified);
  rail ruling grade ≤1% typical, 2.2% rare (verified)
- modern: highway design ~6–8% max; grade near-irrelevant to siting

**Market/interaction range and founding-spacing law**
(Vita-Finzi & Higgs 1970; Britnell 1981 EcHR 34(2); Hudson 1985;
Kansas Pacific locomotive records)
- medieval: day-walk catchment ~5 km (1 h) agrarian; market spacing
  realized 5–10 km; Bracton legal injury radius 6⅔ mi as declared
  forcing
- frontier 19c: wagon-haul feeder ~7–11 mi sets depot-town spacing;
  s_water 7–25 mi (locomotive era-dependent), s_division ~100 mi
- modern: metro-wide low distance-deterrence; spacing dissolves into
  hierarchy by agglomeration

## Built form

**Building height cap (access × structure) and envelope law**
(Otis/Haughwout 1857 record; Home Insurance Building 1885; NYC
Building Zone Resolution 1916; Barr 2016)
- medieval: walk-up cap ~5–6 storeys (~20 m), masonry structural
  cap; rent DECREASES with floor
- frontier 19c: elevator 1857 lifts the access cap; skeleton frame
  1885 lifts the structural cap (10 storeys / 138 ft Home
  Insurance); the rent gradient inverts
- modern: height = rent-vs-cost optimum under the zoning envelope:
  1916 NYC street-wall 1–2.5× street width, setback plane ~1:3–1:4,
  tower on ≤25% of lot; FAR post-1961

**Plot module (frontage × depth) and street/alley widths**, derived
from the declared survey unit
(Slater 1981 Area 13(3); Conzen 1960; Chicago Thompson plat 1830;
FHA Planning Profitable Neighborhoods 1938)
- medieval: burgage 28–32 ft (~2 perches; perch 16.5 ft with
  regional 20-ft variants) × ~1:6 depth; near-irreversible
  boundaries
- frontier 19c: 25 × 125 ft lot, 66-ft street (1 chain), 16-ft
  alley, ~300 × 600 ft blocks; freely resubdividable; platted in
  one event
- modern: FHA-era 60–100 ft frontage, minimum pavement 26 ft
  (1941), blocks to ~1,300 ft, cul-de-sac grammar; zoning
  minimum-lot/setback forcing

**Survey lattice (pre-settlement cadastral forcing)**
(Land Ordinance 1785; Johnson 1976 Order Upon the Land; Campbell
2000 on centuriation)
- medieval: none continental; inherited centuriation locally
  (module ~710 m, 20 actus) where a Roman substrate exists
- frontier 19c: PLSS — 6-mi townships, 1-mi sections (640 ac),
  strict cardinal orientation, survey-before-sale, quarter-section
  allotment; terrain-blind
- modern: the inherited lattice persists in the road spectrum;
  subdivision regulations replace new lattices

## Constraints and closure

**Wall/defense constraint (threat, wall unit cost)**
(Ioannides & Zhang 2017 JUE 97; Dincecco & Onorato 2016 J Econ
Growth; Cesaretti et al. 2016 — no compiled c_w series exists, 0011)
- medieval: threat > 0, wall cost ~ perimeter; artillery (post-1500
  trace italienne) multiplies effective c_w severalfold; emergent
  A ~ P^0.85
- frontier 19c: threat ~ 0 (transient stockades); constraint absent
- modern: threat = 0; wall lines persist only as inherited
  ring-road graph structure

**Mortality–density gradient / sanitation regime**
(Bairoch 1988; Wrigley 1967; Jedwab, Johnson & Koyama, RSUE 94, 2022 —
gradient slope uncertain, expose in config)
- medieval: natural growth negative; size = migration-fed equilibrium;
  bulk 5–20k. **Corrected 2026-08-22 (research 0016): the size
  threshold previously stated here (~5k) was via's own invention — no
  published d(W) or threshold exists, and Davenport (2020) records
  market towns of 2,000–3,000 with infant mortality 209–270 per 1,000
  against under 100 rural. The citable substitute is de Vries (1984)
  Table 10.1's own modelling assumption — natural decrease 5 per
  1,000/yr for cities >= 10,000, neither source nor sink for
  5,000–10,000 — adopted as an assumption he published, not as data.
  The sign itself is contested: Jedwab & Vollrath (2019) measure +2
  per 1,000 across 392 pre-1800 city-periods against Wrigley's −10.**
- frontier 19c: transitional; sanitation engineering diffuses
  post-1850s (declared date forcing)
- modern: d(ρ) flat, b − d > 0; size decoupled from local
  water/food-shed carrying capacity

**Harris–Wilson (α, β) regime + market permanence**
(Harris & Wilson 1978 EPA 10; Osawa et al. 2017 JRS; Skinner
1964–65 J Asian Studies; Fujita & Ogawa 1982 RSUE 12)
- medieval: β high, α ~1 — many small centers; market frequency
  < daily (periodic circuits, 10-day cycles)
- frontier 19c: β falling along rail; α rising; periodicity largely
  skipped (high per-capita demand) — a discriminating prediction
- modern: β low, α > 1 — few dominant centers, subcenters via the
  Fujita–Ogawa t/α knob

**Street-fabric organization parameters (Courtat P_e, ω; grammar
choice)**
(Courtat et al. 2011 PRE 83:036106; Cardillo 2006; FHA TB7 1938;
Boeing 2020 JAPA)
- medieval: low P_e, low ω (organic, tree-ish, M 0.15–0.26)
- frontier 19c: platting operator dominant — high P_e, high ω,
  orientation law lattice/track-relative; M 0.26–0.35
- modern: district-piecewise — FHA street-tree grammar in suburbs
  (low local ω, high dead-end fraction), high exploration-fraction
  sprawl; post-1995 partial grid rebound

**Formalization fraction** (share of new accretion captured by
template forcing vs free accretion)
(Beresford 1967; Reps 1965; UN SDG 11.1.1 / UN-Habitat WCR 2022–24)
- medieval: low-moderate — planted cores + organic suburbs coexist
  (bastide grids ARE 13th-century forcing)
- frontier 19c: near 1.0 (survey-before-sale; speculative platting
  ahead of demand, vacant-plat halo)
- modern: regional — <0.05 informal in developed regions to ~0.5 in
  sub-Saharan cities (global urban informal 24.8%, 2022)

**Growth-boundary / regulatory constraint set**
(Conzen 1960 fixation lines; Masucci, Stanilov & Batty 2013 PLoS
ONE 8:e69469)
- medieval: wall polygon (endogenous from the wall mechanism) +
  commons fixation lines
- frontier 19c: effectively unbounded (open frontier); land supply
  exogenous via the land office
- modern: green-belt/zoning polygons as declared forcing; the
  emergent logistic N(t) is the gate
