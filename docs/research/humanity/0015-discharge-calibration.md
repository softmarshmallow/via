# Research 0015 — Discharge calibration and hydraulic conventions

Status: research corpus — pre-decision (2026-08-22). Nothing here is
adopted; adoption requires an ADR. Commissioned when ADR 0012 D3 made
k_Q calibration a prerequisite for any corridor number counting as
evidence. The sweep answered that question and, in doing so, turned up
corrections to conventions ADR 0011 already adopted — those are the
larger part of this dossier.

Verification status is marked per item: *primary* (full text read),
*primary-image* (printed page read as a scan, because OCR of mid-century
USGS material is unreliable), *abstract*, or *secondary via X*.

## 1 — k_Q is derivable from via's own hydrology, not free

The load-bearing implementation fact, established by reading
`via-terrain/src/stage.rs`: the flow accumulator is weighted by
`max(precip / precip_mean_m_per_yr, 0.05)`, so the shipped
`discharge` raster counts **mean-precipitation-equivalent upslope
cells**. One discharge unit therefore carries one cell's area of the
land-mean annual precipitation. Absolute discharge follows:

    Q [m³/s] = discharge_units × A_cell [m²] × P_mean [m/yr] × C / T_year

so that

    k_Q = A_cell × P_mean × C / T_year

with **C the runoff ratio** — the fraction of precipitation reaching
the channel rather than returning to the atmosphere. Every other term
is already declared in the terrain config. This demotes k_Q from a
free Tier-2 scale (ADR 0011 D1: "the one 'how big are rivers in this
world' knob… unverifiable from inside via") to a **derived quantity
carrying exactly one cited coefficient**.

Two checks on the derivation, measured on the reference run
(m4c-research-s42, 512² at 200 m, P_mean 1.2 m/yr):

- the 0.05 rain-shadow floor is not a material bias: 4,811 of 102,121
  land cells (4.7%) hit it, inflating total routed water by **0.04%**;
- land-mean precipitation equals the configured mean exactly, so
  `precip_mean_m_per_yr` is the right normalizer and no separate
  land-mean needs computing.

At C = 0.3 this gives k_Q ≈ 4.6 × 10⁻⁴ m³/s per unit, against the
shipped placeholder of 1.0 — a factor of ~2,200. Under the
placeholder the reference island's largest basin (479 km²) carried
14,607 m³/s, roughly the Mississippi; at the derived value it carries
6.7 m³/s, a specific runoff of ~11 L/s/km², squarely in the
humid-temperate range. **The placeholder is the sole reason every
channel on the reference world reads as an impassable wall.**

### The coefficient C

**Dai, A. & Trenberth, K.E. (2002)**, "Estimates of Freshwater
Discharge from Continents", *Journal of Hydrometeorology* 3(6):660–687
[primary, full text] state the ratio themselves rather than leaving it
to the reader: global continental discharge is **37,288 ± 662 km³/yr,
~35% of terrestrial precipitation**. Corroborated by Trenberth et al.
(2007), *JHM* 8(4):758–769 [primary]: land P 112.6 ×10³ km³/yr against
total runoff 40.0 ×10³, i.e. **0.355**. A chronological review of every
published global water budget (*Surveys in Geophysics* 42:1075–1107,
Table 6 [primary]) puts the whole published spread at **0.33–0.42,
clustering hard at 0.35** across fourteen studies from Manabe (1969) to
Abbott et al. (2019).

**The non-obvious part, and the reason this is a match rather than an
approximation.** Dai & Trenberth's 0.35 is Σ(R)/Σ(P) — a
*precipitation-weighted* mean of local runoff ratio, dominated by wet
regions; the area-weighted mean over Earth's land is materially lower.
via's `discharge_cells` is *itself* precipitation-weighted by
construction (each cell contributes P_i/P_mean), so the
precipitation-weighted global statistic is the correctly-paired one.
Expanding the formula makes this explicit: Q = Σ_i (A_cell·P_i·C)/T,
which is mass-consistent by inspection.

Regional structure, for the limitation that follows — FAO (2003),
*Review of World Water Resources by Country*, Water Reports 23, Table 2
[primary]: humid temperate Europe and North America run C ≈ 0.50–0.53,
arid Africa, Oceania and Central Asia C ≈ 0.19–0.21. A uniform C
therefore **over-produces discharge in arid cells and under-produces
it in wet ones**; the Budyko curve (below) puts true local C at ~0.8
where PET/P = 0.2 and ~0.04 where PET/P = 3.

**A uniform C forces Q ∝ A exactly** — and that is the *right*
exponent, not a defect: USGS regional regressions for mean annual flow
in humid regions land at **b = 0.96–1.02** (Pennsylvania 1.008,
Connecticut 0.975, north Georgia 0.993, Alabama 0.996, Maine 0.960,
Georgia/Carolinas 0.991) [all primary]. Sub-linear exponents in the
literature (0.72–0.83) belong to **flood peaks**, where hydrograph
attenuation applies, not to mean annual flow. Any sub-linearity via
produces will come from its own orographic precipitation gradients —
a clean, testable prediction. **Negative finding worth recording:
Leopold & Maddock (1953) contains no Q-vs-A relation at all** — its
abscissa is always mean annual discharge — so citing PP 252 for a
drainage-area exponent, a common error, is doubly wrong.

**Validation targets** from 438 minimally-disturbed USGS reference
gauges in the East Highlands and Northeast (1.6–8,265 km², ≥20 yr)
[primary, NWIS read directly]: `Q[m³/s] = 0.0214·A[km²]^0.965`
(R² 0.942); runoff depth median **585 mm/yr** (p10–p90 357–846);
runoff ratio median **0.47**, range 0.30–0.65. A 56-year-independent
cross-check: Thomas & Benson (1970) USGS WSP 1975 fit the Potomac at
`A^1.01·P^1.58`; the 2026 fit on those 438 gauges gives
`A^0.985·P^1.592`.

**Why not Budyko per cell.** The framework is well-attested — Fu (1981)
via Zhang et al. (2004) WRR 40:W02502 [primary], ω = 2.63 overall
(2.84 forest / 2.55 grass, observed range 1.7–5.0); Zhang, Dawes &
Walker's two-parameter form with w = 2.0 forest / 0.5 grass [primary
via CRC TR 99/12]; Choudhury (1999) with α scale-dependent, 2.6 at
~1 km² falling to 1.8 above 10⁶ km². All four variants agree within
0.03 in E/P, so the choice of variant matters far less than the choice
of PET. But adopting it would require via to invent a PET field, and
every temperature-only route fails here for a specific, verified
reason: **Oudin et al. (2005)** [primary, accepted manuscript] — the
best-validated option, which actually *beats* Penman in rainfall-runoff
use — computes extraterrestrial radiation from **latitude and Julian
day only**, and via's world declares no latitude; **Thornthwaite
(1948)** misbehaves on via's isothermal per-cell means, being
non-monotonic below ~7 °C and divergent above 26.5 °C; **Hamon
(1963)** degrades gracefully (set the day-length term to 1 and it
becomes a clean monotonic function of temperature) but that is a
declared convention, not a citation. Budyko would therefore replace one
declared number with four — and be applied at 200 m cells against a
framework its own author verified only above 1,000 km² and for
averaging periods much longer than a year. Donohue et al. (2007)
[primary] state the violation directly: "at A_c ≤ 1000 km² and
τ ≤ 1–5 years the inherent assumptions can be violated."

**Recorded upgrade path**, if per-cell variation is later wanted:
Zhang, Dawes & Walker's *reduced* form (Ez = 1410 mm, w = 2.0 forest;
Ez = 1100 mm, w = 0.5 herbaceous) needs only precipitation and a
forest fraction and **no PET at all**. It belongs inside `weights_of`
as `weight_i = P_i·C_i/P_mean` — which changes the accumulation field
that also drives stream power in the erosion and sediment stages, so
it is a gated, declared change to the landscape itself, never a
display-layer conversion.

**Sanity of the resulting scale.** At the reference config
(P_mean 1.2 m/yr, C = 0.35 → k_Q = 5.32 × 10⁻⁴, runoff depth
420 mm/yr) the formula lands within a factor of ~1.15 of three real
gauges spanning two decades of drainage area: Blockhouse Creek, PA
(98 km², 1.68 m³/s), Cowpasture River, VA (1,195 km², 15.35 m³/s), and
the Juniata at Newport, PA (8,657 km², 122.85 m³/s) [NWIS, primary].
That is inside the ±1.5× band the sweep identifies as
USGS-regional-regression-grade.

## 2 — Which discharge statistic does each formula want?

This was commissioned as a hazard check — feeding mean-annual
discharge into a bankfull-calibrated width law would misstate every
river dimension — and returned a more interesting answer than
expected.

### Finnegan et al. (2005) [primary, full text + Figure 1 image-verified]

Finnegan, N.J., Roe, G., Montgomery, D.R. & Hallet, B. (2005),
"Controls on the channel width of rivers: Implications for modeling
fluvial incision of bedrock", *Geology* 33(3):229–232,
doi:10.1130/G21171.1.

The paper is internally split, explicitly:

- **The derivation is bankfull.** "For a rectangular channel where
  **bank-full width, W, and depth, D**, are related by the
  width-to-depth ratio α = W/D…" — and mechanically so, since Q = UA
  with A = W²/α requires the discharge that fills that section.
- **The calibration is mean annual.** Twice, on two independent
  datasets: "**Mean annual discharge** was obtained by routing annual
  accumulated Tropical Rainfall Measuring Mission … satellite-derived
  rainfall"; and "**Mean annual discharge** was estimated for Oat
  Creek and Kinsey Creek by applying the relationship between **mean
  annual discharge** and drainage area for nearby Honeydew Creek".

The reconciliation is the regression: they fit "linear regressions
(forced through the origin) of measured channel width vs. Q^1/2 and
of measured channel width vs. Q^3/8 S^−3/16", validating the
*exponents* while the fitted constant silently absorbs
`[α(α+2)^(2/3)]^(3/8)·n^(3/8)` **and** whatever bankfull-to-mean ratio
applies. **Using the closed form with an explicit α and n to produce
an absolute width — which is what via does — is a mode the paper
never tested.**

**Figure 1 carries no discharge axis at all** (axes: Depth 0–16 m vs
Width 0–150 m), so the α values are not fit against discharge in any
form and the paper states no flow convention for them. The four
values are confirmed as via records them: bedrock 5, boulder 9,
cobble 21, gravel 59.

A new finding traces the gravel value: Finnegan's caption sources it
to Leopold & Maddock (1953), whose only channel-shape table is
**Appendix A, "Channel-shape characteristics of rivers at stage
corresponding to mean annual discharge"**. Refitting α through the
origin on that appendix's 26 Wyoming Yellowstone-basin stations gives
**66.3**, against the 59 plotted — ranges and fit both match. So
**α = 59 is a mean-annual-stage ratio**, and eq. 5 with α = 59 and
mean annual Q is accidentally self-consistent. For bedrock, boulder
and cobble (5/9/21, from "field surveys in Cascades of Washington
State") the convention is **unstated and unverifiable** — and via's
default α = 20 sits with those, not with gravel. High-confidence
inference, not an explicit statement by Finnegan.

### Leopold & Maddock (1953) [primary, full text]

USGS Professional Paper 252. Downstream hydraulic geometry (b = 0.5,
f = 0.40, m = 0.1) is defined **at mean annual discharge**, and a
full-text search returns **zero occurrences of "bankfull"** — the
concept enters hydraulic geometry later, with Wolman & Leopold
(1957). Mean annual discharge "represents roughly the discharge
equalled or exceeded 1 day in every 4 over a long period."

### Bankfull recurrence and the Qbf/Qmean ratio

- Wolman & Leopold (1957), USGS PP **282-C** (not 282-A — a
  bibliographic correction), p. 88 [primary]: overbank flow recurs
  "every year or every other year (**recurrence interval = 1 to 2
  years**)", and where the flood plain is well defined "**closer to 1
  than 2**". On the **annual maximum series**, stated explicitly —
  this is routinely misquoted as partial-duration.
- Williams, G.P. (1978), *WRR* 14(6):1141–1154 [primary abstract]:
  "Eleven possible definitions of 'bank-full' have been used…
  **Bank-full discharge does not have a common recurrence frequency
  among the rivers studied, and the discharge corresponding to the
  1.5-year recurrence interval in most cases does not represent the
  bank-full discharge.**" Also, load-bearing for via: "**the
  resistance coefficient *n* should be estimated at the field site for
  bank-full flow; a measured low-flow *n* should not be used.**"
- Ahilan et al. (2013), *Proc. ICE Water Management* 166(7):381–393
  [primary]: 88 Irish (humid-temperate) stations, **median bankfull
  recurrence 1.64 yr**.
- Liu et al. (2026), *Nature Communications* [primary, preprint]:
  8,519 stations; temperate median return period **1.4 yr observed /
  1.8 yr modelled**, IQR 0.8–3.0; the 2-year assumption overestimates
  bankfull in temperate regions by 44 ± 141%.
- Emmett (1975), USGS PP 870-A [primary]: **"Mean annual discharge is
  approximately 25 percent of bankfull discharge"** — Qbf ≈ 4 × Qmean,
  range 0.20–0.27 across drainage areas, with the ratio falling
  downstream. Emmett's own caveat: "the QA/QB value of 0.25 cannot be
  used indiscriminately." **Caveat for via: a snowmelt-dominated
  semi-arid Idaho basin, whose sustained hydrograph pushes Qmean up
  relative to Qbf — not humid-temperate.**
- **No published Qbf/Qmean ratio was found as such.** Two independent
  derivations bracket it: pairing Wolman & Leopold's Table 1 bankfull
  discharges with Leopold & Maddock's Appendix A mean annual
  discharges at four exactly-matching stations gives **1.6–4.6,
  median ≈ 3.0**; and a UK-average flow-duration curve puts the 2%
  exceedance flow at 3.45 × mean flow, implying **≈ 3–6**. Treat as
  order-of-magnitude.

### Error propagation

For W, D ∝ Q^(3/8) and U ∝ Q^(1/4), feeding mean annual where
bankfull belongs under-predicts by:

| Qbf/Qmean | W and D | U | area |
|---|---|---|---|
| 2 | 1.30× | 1.19× | 1.68× |
| 3 | 1.51× | 1.32× | 2.28× |
| 5 | 1.83× | 1.50× | 3.34× |

At the likely ratio (~3) the width/depth error is ~50% — real, but
heavily damped by the 3/8 exponent. **The α convention is the larger
exposure, not the discharge convention.**

## 3 — Langbein's f: Figure 8 digitized, and via's channels are out of domain

Langbein, W.B. (1962), USGS Water-Supply Paper 1539-W [primary, full
text; Figures 8, 11, 13 image-verified].

**The project's earlier correction is confirmed verbatim**: "The
abscissa is the ratio, *f*, of the resistance against a vessel moving
at a certain speed in shallow water to that at the same speed in deep
water. The ordinate is the ratio of draft to channel depth, and the
parameter, F, is the ratio of the speed of the vessel to that of a
gravity wave." *f* is a vessel property, not a bed-friction factor.

**Figure 8 is now digitized**, closing the pending flag ADR 0011
records. At Langbein's optimum draft d/D = 0.70:

| F = V/√(gD) | 0.25 | 0.50 | 0.75 | 0.90 |
|---|---|---|---|---|
| **f** | 1.21 | 2.48 | 4.71 | ≈7.8 |

Validated end-to-end against Figure 11 (two named cases reproduce
Ts to ~15%, inside the reading precision of a 1962 log-scale figure).
**For typical navigable rivers F is small (D = 10 ft, V = 3 ft/s →
F = 0.17), so f ≈ 1.1–1.4** — against via's shipped constant of 2.5.

Since F is computable per cell, *f* can stop being a declared constant
and become a lookup on the primary's own curve. But measuring F on
via's reference channels at the derived k_Q gives **median F = 1.25 —
supercritical**, with p10 0.71 and p90 1.76. This is structural, not a
k_Q artifact: F ∝ Q^(1/16) through the chain, essentially invariant.
The Manning algebra explains it — for a wide channel
F = D^(1/6)·S^(1/2)/(n√g), so at n = 0.035 and D ≈ 0.07 m, F exceeds 1
above **S ≈ 3%** — and the reference island's channels have median
slope **5.4%**, with **73% steeper than 3%**.

Two consequences. First, **Langbein's Figure 8 stops at F = 0.90**, so
most via channels lie outside its domain and any *f* read there is an
extrapolation; the honest treatment is to declare supercritical
reaches unnavigable by domain rather than extrapolate. Second, **zero
navigable cells on the reference world is the physically correct
answer**, not a defect: these are mountain torrents. Exercising the
water modes honestly needs a larger, flatter domain with lowland
reaches.

**What flow is the navigability criterion at?** Not a single
discharge: "the depth-velocity curve is defined as the enveloping line
GE… this enveloping line represents the variation between depth and
velocity in shallow river sections **up to bankfull stages**", and it
is applied at the **shallow controlling sections** (riffles,
crossovers), not the mean section — Langbein's Mississippi example
swings 13× between the gaged section and the crossover. He notes the
criterion "tends to be conservative over the range of depths shown"
(≈1.3× low water to bankfull), but his class boundaries are only 2×
apart (0.001 / 0.002), so stage choice can still move a marginal river
one class. Useful corrections he supplies: channel depth ≈ 1.25 ×
section mean depth, channel velocity ≈ 1.15 × section mean velocity.

The published verdict table (commercial use vs required specific
tractive force) is reproduced in the sweep record: Mississippi at
Vicksburg 0.00015 (major waterway) through Kansas R. 0.002 (ferry and
short-run) to Rio Grande 0.02 (no commercial navigation).

## 4 — The wading envelope: two traditions, differing by ~2×

ADR 0012 gates ford passability on Cox, Shand & Blacka (2010) bands
with D·V ≤ 0.8 m²/s, depth ≤ 1.2 m, velocity ≤ 3.0 m/s. The sweep
establishes that these are the **occupational / flood-stability** end
of the literature, and that a pre-modern traveller belongs at the
other end.

**Occupational product rules** (trained staff with wading rod, cleats,
tag line):

| Rule | Value | Source |
|---|---|---|
| "Rule of ten" | D·V ≥ 10 ft²/s = **0.93 m²/s** → do not wade | NMED SOP 7.0/8.2; NPS DWQ07; USGS TWRI 3-C2 [primary ×4] |
| Metric variant | D·V < **0.6 m²/s** | USFWS/DFO TOP 001.31 (2020) [primary] |
| Water Survey of Canada 1981 | "slightly less than **1 m²/s**" | via OMNR 2002 [secondary] |
| Bounded form (recommended) | **D·V ≤ 0.4 m²/s AND d ≤ 0.8 m AND v ≤ 1.7 m/s** | OMNR 2002 [primary] |
| Explicit 2-D envelope | no precautions at **v < 0.5 m/s AND d < 0.5 m**, firm regular bed | NEMS NZ Code of Practice v1.1 (2013) §1.6.2.2 [primary] |

OMNR states the scoping caveat directly: the product rule "applies to
**trained professionals** whose regular work accustoms them to the
dynamic forces of river flows… it is likely that the simple rule of
3 × 3 product (1 m²/s) represents an **upper limit for adult male
occupants**… it would be reasonable to consider something lower as
being more representative of a safe upper limit."

**Recreational anatomical rules** (a loaded traveller, no aids) — no
source in this tradition publishes a velocity in m/s; all use
"walking pace":

| Threshold | Value | Source |
|---|---|---|
| Shin depth, any current | ~0.35 m | ACEP Wilderness Medicine (2019) [primary] |
| Knee depth at walking pace | ~0.45–0.5 m | Bushwalking Victoria; US NPS; NZ Bushcraft Manual [primary ×3, independent] |
| Thigh of shortest member | ~0.7–0.8 m | NZMSC River Safety; Bushcraft Manual p.161 [primary] |
| Waist depth **but slow flow over level shingle** | ~1.0 m | NZ Bushcraft Manual p.159 [primary-image] |
| Foot fording maximum (military) | **1 m** | US FM 5-34 Table 3-6 [primary] |
| Stage rise that flips a safe crossing | **15 cm** | NZ Bushcraft Manual p.161 [primary-image] |

The two traditions differ by roughly a factor of two, and the sweep's
central caution is that they are not interchangeable: a model
calibrated on the occupational envelope "will systematically overstate
what a settler or traveller could ford." Note also that NEMS's
velocity axis is **surface velocity** (≈1.1–1.2 × depth-averaged), so
applying its bands to a depth-averaged field silently loosens them.

**Correction to a value 0013 already cites:** Grayson's 60 cm
pedestrian ford depth carries **no footnote and no citation** in its
own paper — it is one author's working convention in a county
archaeological journal, and every downstream repetition traces back to
that sentence. It should not be presented as archaeological consensus.
It does, however, converge independently with the USFS Trail
Construction and Maintenance Notebook (0723-2806-MTDC, pp. 90–92)
[primary]: "**Most fords are designed to be used just during low to
moderate flows. A ford for hikers and packstock… should be no deeper
than 400 to 600 millimetres (16 to 24 inches, about knee high) during
most of the use season. A horse ford shouldn't be deeper than 1
metre.**" The Notebook gives **no velocity guidance at all** — the
Forest Service pedestrian ford standard is depth-only, at a
seasonal-typical flow.

**Negative finding:** there is **no peer-reviewed depth–velocity
stability envelope framed for non-flood recreational wading**. The
entire quantitative human-stability literature (Abt, Jonkman, Xia,
Cox) is flood-hazard framed. The defensible citations for the
traveller case are national agency standards — grey literature, but
named and dated. Equally, the archaeological least-cost-path
literature treats fords **only as topological waypoints**, with no
depth/velocity criterion: via is not departing from a standard,
because none exists.

### What flow is a ford judged at? A named convention exists

The low-water-crossing engineering literature has a settled answer,
and it is **not** a low-flow statistic: passability is expressed as
**Q_e, the discharge equalled or exceeded e percent of the time on the
annual flow-duration curve, where e is the acceptable percentage of
the year the crossing may be closed.** Structural survival is a
separate flood-recurrence question. The lineage is Iowa DOT → Iowa
State → FHWA-CFL.

- Rossmiller, Lohnes, Ring, Phillips & Barnett (1983), *Design Manual
  for Low Water Stream Crossings*, Iowa DOT HR-247 [primary] — the
  origin: Q_e = aA^b, "**e is the exceedance probability in
  percent**", with the interpretation stated directly: "If the LWSC is
  designed for Q25%, the crossing will be closed on the average of
  three months each year. **If the LWSC is designed for Q2%, the
  crossing will be closed on the average of seven days each year.**"
  Valid only for e in 1–50%; "No attempt should be made to
  extrapolate the curve beyond the 50 percent exceedance."
- Ring, S.L. (1987), *TRR* 1106:309–318 [primary, subscripts read from
  a page image]: "**A decision to use an exceedence probability of 10
  percent would mean that water would flow over the road an average of
  about 37 days a year… The selection of a design discharge of Q₂%
  would mean that water would flow over the road an average of one
  week of the year.**"
- Lohnes, Gu, McDonald & Jha (2001), Iowa CTRE 01-78 [primary]:
  "**The acceptable closing percent of time per year (e) can be called
  as the design exceedence probability.**"
- McEnroe et al. (2017), FHWA-KS-16-19 [primary]: "an LWSC should be
  impassable **fewer than 10 times in an average year** and the
  duration of impassable conditions should not exceed 3 days"; vents
  sized to "**pass a discharge that is exceeded no more than 5% of the
  time**".
- Bhattarai et al. (2016), FHWA-ICT-16-020 [primary]: "the LWC should
  be designed such that it is **functional at least 95% of the time in
  a year**", with crossing components at 10–25-year flow and vents at
  the 0.5–1-year event.
- Clarkin et al. (2006) [primary] states the two-approach split:
  flow-duration data for "typical annual delay time", flood-frequency
  data for "total structure capacity".

**Explicit negative finding:** no source in this literature uses Q90,
Q95, 7Q10, mean annual low flow, or summer low flow as the ford
passability flow — those are drought statistics. The sweep also
flagged a fabricated claim to the contrary circulating in an
AI-generated encyclopedia; it contradicts every primary above.

Passability depth thresholds converge across national standards, all
stated against a *typical* rather than flood flow: 6 in (150 mm) in
the US lineage; 150 mm in TRL Overseas Road Note 9 ("**for most of the
year the maximum depth of water over the carriageway should be less
than 150mm**") [primary]; 200 mm with velocity < 2 m/s in IRC:SP:82-2008
(India) [primary, page-image verified]; 100 mm supercritical / 150 mm
subcritical in the Kenyan and SANRAL manuals [primary]; 200 mm for
cars and 500 mm for heavy vehicles in Main Roads Western Australia's
Floodway Design Guide, which names the concept "**trafficable
discharge**" and designs to a serviceability ARI plus a maximum
closure period [primary]. IRC:SP:82-2008 Table 3.1 sets the
interruption budget directly: **6 permitted interruptions per year**,
of 2–6 h (highways) or 6–12 h (village roads).

For via the mapping is clean: fords should be judged at a **declared
exceedance percentile of the flow-duration curve**, not at mean annual
flow, and §5's one-parameter FDC family is the instrument for getting
there from a mean-flow model. The USFS Notebook's "low to moderate
flows… during most of the use season" is the pedestrian statement of
the same idea, and Motayed et al. (1982) [primary] supply the
pre-modern-relevant fordability threshold directly: vented structures
are preferred "where **day-to-day flow exceeds fordable depth,
normally 4-6 in or 10-15 cm**".

**Provenance correction:** the widely repeated "FHWA 6 inches during
the high-design flow" does **not** originate in Motayed et al. (1982)
— that report's numbers are 4–6 in at *normal daily flow* and 1 ft at
the *2-year* flow for ADT < 100. The 6-inch design limit enters via
Looschen & Coy (1982, unpublished Iowa DOT) → Lohnes (2001) → Gu
(2003/05), and is "FHWA" only because Gu published under an FHWA-CFL
number.

## 5 — Seasonality: mean annual flow is a wet-season picture

- Mean annual discharge is exceeded only **~25% of the time** (Leopold
  & Maddock 1953, p. 9) — Langbein 1962 p. W-6 says ~30%, adding "**lesser
  depths prevail about 70 percent of the time.**"
- Gustard, Bullock & Dixon (1992), Institute of Hydrology Report 108
  [primary, Table 3.4 image-verified], n = 865 UK stations: **Q95 =
  16.9% of mean flow** (SD 11.2), MAM(7) = 18.5%, BFI mean 0.501.
  Strongly geology-dependent (Table 4.9): chalk 40.8%, tills/mudstones
  10.7%, very soft massive clays 1.1%.
- IH108 Table 5.2 supplies a **one-parameter family of complete
  normalized flow-duration curves indexed solely by Q95** — the
  citable instrument for converting a mean-flow model into any
  exceedance percentile. For a UK-average river: Q2 = 345% of mean
  flow, **median = 62%**, Q95 = 16.9%. So mean/median ≈ 1.60.

**Limitation to record:** a model that renders every river at mean
annual flow is showing a wetter-than-typical world roughly
three-quarters of the year.

## 6 — Measured response of via's chain

Instrument: `analysis/kq_calibration.py`, which runs the real
via-suitability stage once per (world, k_Q) under a namespaced config
label and reads back the emitted rasters — no hydraulics are
reimplemented in the sidecar, so what is measured is what the stage
computes. Protocol and ensemble are recorded with the figure.

Reference world (m4c-research-s42), largest basin 479 km²:

| k_Q | fordable | navigable | outlet W | outlet D | outlet V |
|---|---|---|---|---|---|
| 3.0e-4 | 100.0% | 0% | 7.43 m | 0.36 m | 0.89 m/s |
| **4.56e-4** (C=0.3) | **100.0%** | **0%** | **8.70 m** | **0.42 m** | **0.99 m/s** |
| 1.0e-3 | 98.1% | 0% | 11.68 m | 0.56 m | 1.20 m/s |
| 1.0e-2 | 73.9% | 0% | 27.69 m | 1.33 m | 2.13 m/s |
| 1.0 (placeholder) | 0.02% | 0.05% | — | — | — |

Two degeneracies bracket the placeholder: at k_Q = 1 essentially no
channel is fordable; at the derived value essentially every channel
is. Neither is a defect — brooks *are* crossable nearly everywhere —
but it means river structure on a world this size expresses itself
through accumulated crossing delay rather than through ford-seeking,
and that ford-seeking as a corridor mechanism needs rivers large
enough to be un-fordable in their lower reaches.

## Open gaps

- Qbf/Qmean has no published value; §2's brackets are derivations.
- No per-Köppen-zone runoff-coefficient table was obtained — McMahon
  et al. (2007), *J. Hydrology* 347:243–259 (1,221 unimpacted global
  rivers) is closed access. The highest-value remaining retrieval if C
  is ever keyed to climate class rather than to PET/P.
- Baumgartner & Reichel's commonly quoted 746/480/266 mm triple is
  **not** verified; only their runoff total is (via Dai & Trenberth
  and FAO, which agree).
- Budyko (1974), L'vovich (1979), Fu (1981), Choudhury (1999),
  Thornthwaite (1948) and Hamon (1961) were all read only through
  peer-reviewed restatements that print the equations verbatim.
- Finnegan's bedrock/boulder/cobble α flow convention is **unstated in
  the paper and unverifiable** — and via's default α = 20 sits there.
- Chow (1959) stage-dependence of *n* not verified from the primary;
  rely on Williams (1978)'s warning that *n* must match the modelled
  flow condition.
- Langmuir and Setnicka (mountaineering wading criteria) not
  obtainable; no numeric criterion should be attributed to them.
- Nixon (1959); Andrews (1980); Dury (1976); Castro & Jackson (2001)
  paywalled — abstract-level or secondary only.
- Three attribution errors not to propagate: the term "7Q10" is not in
  Riggs (1972); Smakhtin (2001) states no Q95/mean ratio; **the
  depth×velocity wading rule is not in USGS TM 3-A8** — it lives in
  WRD Memorandum 99.32, stated there *in order to caution against it*.

## What an adopting ADR must decide

1. Whether k_Q becomes derived (§1) with C as the single declared
   coefficient, and whether C is a global constant (the sweep's
   recommendation: **C = 0.35**, Dai & Trenberth 2002, matched to via's
   precipitation-weighted accumulation) or climate-derived (deferred,
   with the Zhang reduced form as the recorded upgrade path). Either
   way the ADR must declare that mapping a generated dimensionless
   climate onto absolute mm/yr and m² is itself a declaration, and
   record the 0.05 weight floor as a small positive bias.
2. Which discharge statistic the chain carries, and — if mean annual —
   whether the Finnegan closed form is used in regression mode or
   with an explicit bankfull conversion (§2). The two are not
   interchangeable.
3. Whether α stays a single declared constant given that only the
   gravel value has a verifiable flow convention (§2).
4. Whether *f* becomes a lookup on the digitized Figure 8 (§3), and
   whether supercritical reaches are declared unnavigable by domain
   rather than extrapolated.
5. Whether ford caps move from the occupational to the recreational
   envelope (§4).
6. Whether fords are judged at a **declared exceedance percentile**
   rather than at mean annual flow (§4), and which percentile — the
   engineering convention is an explicit "acceptable days closed per
   year" choice, and via has the FDC instrument (§5) to realize it.
7. What seasonality limitation is recorded (§5).
