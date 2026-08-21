# Research 0013 — Affordance detection and movement cost

Status: research corpus — pre-decision. Nothing in this document is
adopted; adoption of any mechanism requires its own ADR. Part of
docs/research/humanity (see README there). This dossier was
commissioned for the via-suitability stage decisions (the adopting
ADR, if any, will cite it) and, unlike the field dossiers, it surveys
a question the corpus was found not to answer.

## The question this literature answers for via

The README's causal chain names four site affordances — harbours,
fords, confluences, passes — and ADR 0009 records that the settlement
spike failed partly because none of them was ever implemented. A
sweep of the existing corpus (0001, 0005, 0006, 0008, 0009, 0011)
found **no detection criteria for any of them**: the dossiers name
affordance-like features in passing (river crossing, bridging point,
defensible spur, locomotive watering), always as scoring terms over
continuous fields, and 0005 records the intended *form* of
water-affordances ("navigable river reaches from via's hydrology —
flow above threshold, slope below threshold"; "transshipment nodes
acquire high centrality mechanically") — but nowhere is there a
citable route from a terrain artifact to a ford, a harbour, or a
pass. This dossier surveys those routes, plus the slope-dependent
movement-cost functions the corridor stage will consume (which
resolves one of 0011's recorded open questions).

Constraint honoured throughout: every scheme is assessed against
via's *shipped* artifacts — heights_cm (i32 centimetres, true surface
including lake and ocean bathymetry against `sea_level_m`), receivers
(D8 steepest-descent tree, self-receiver = ocean), area_cells,
strahler, discharge (MFD precipitation-weighted accumulation,
**relative units**), water_depth, sediment, lithology — at 16 m
cells. No wind field exists; no channel hydraulic geometry exists; no
vector artifacts exist.

Verification note: citations below were checked against primary
sources or bibliographic records during this sweep. Where a number or
document could not be confirmed first-hand it is marked so, in the
tradition of 0011's "unverified digits" flags. Do not promote a
marked value into config without closing its flag.

## Passes

### Morse-theoretic saddles with persistence (detection + intrinsic rank)

Peucker, T.K. & Douglas, D.H. (1975) 'Detection of surface-specific
points by local parallel processing of discrete terrain elevation
data', Computer Graphics and Image Processing 4(4):375–387 (verified,
ScienceDirect record); Takahashi, S., Ikeda, T., Shinagawa, Y.,
Kunii, T.L. & Ueda, M. (1995) 'Algorithms for extracting correct
critical points and constructing topological graphs from discrete
geographical elevation data', Computer Graphics Forum 14(3):181–192
(verified, Eurographics DL); Pfaltz, J.L. (1976) 'Surface networks',
Geographical Analysis 8(1):77–93 (verified, DOI); Edelsbrunner, H.,
Letscher, D. & Zomorodian, A. (2002) 'Topological persistence and
simplification', Discrete & Computational Geometry 28:511–533
(verified, DOI); Kirmse, A. & de Ferranti, J. (2017) 'Calculating the
prominence and isolation of every mountain in the world', Progress in
Physical Geography 41(6):788ff (verified: DOI and start page; end
page not independently confirmed).

A pass is a saddle of the height field. The Peucker–Douglas 3×3
operator detects saddle candidates in one scan: take the cyclic signs
of (neighbour − centre) around the 8-ring; four or more sign changes
— two separated higher runs alternating with two lower — is a saddle.
Takahashi et al. show the raw operator yields topologically
inconsistent feature sets and give the correction; Pfaltz's surface
network is the graph the corrected features form. Persistence
(Edelsbrunner et al.) attaches a continuous importance to each
saddle: in a superlevel-set sweep, each saddle is paired with the
peak it merges away, and persistence = z(paired peak) − z(saddle) —
the depth of the col. Kirmse & de Ferranti instantiate exactly this
at global DEM scale (divide tree via union-find over height-sorted
cells) and prune with a ~30 m minimum-prominence floor.

As code: sort cells by exact integer heights_cm, one union-find sweep
— O(N log N), the same machinery family as the flow-tree code. The
8-ring test supplies candidates; the sweep makes them consistent and
attaches persistence. One declared parameter (minimum persistence,
provenance Kirmse & de Ferranti's ~30 m) plus a declared
deterministic tie-break for equal integer-cm heights. Output: a
saddle site set carrying a continuous persistence spectrum —
detection by a recognized scheme, importance as a spectrum,
discretization only at the published pruning floor.

Cross-check available for free: via's basin raster gives, for each
pair of adjacent land-draining D8 basins, the minimum-height cell on
their shared boundary — the optimal vertical crossing of that
drainage divide. The two sets should largely coincide, but the
correspondence is a diagnostic, not an invariant: via's basins are
labeled by ocean outlet (coastal adjacencies bottom out at the
shoreline, not at cols), and D8 divides derive from the
epsilon-filled routed surface, so they can wander off heights_cm
ridges across filled flats. (This cross-check is a project
observation, not literature.)

Caveats: persistence prices only the vertical wall — how much a
route saves against climbing the flanking ridge — and ignores
horizontal detour and approach steepness. It ranks passes on one
divide well; it does not price a route. Passes narrower than the
16 m cell are invisible at this resolution.

### Continuous "passness" (Wood; Fisher–Wood–Cheng)

Wood, J. (1996) 'The Geomorphological Characterisation of Digital
Elevation Models', PhD thesis, University of Leicester (verified:
Leicester figshare record; reference implementation GRASS
r.param.scale, manual fetched); Fisher, P., Wood, J. & Cheng, T.
(2004) 'Where is Helvellyn? Fuzziness of multi-scale landscape
morphometry', Transactions of the Institute of British Geographers
29(1):106–128 (verified, Wiley landing page and DOI).

Wood fits a bivariate quadratic over an odd n×n window and
classifies each cell into six morphometric features — peak, ridge,
**pass**, plane, channel, pit — from fitted slope and principal
curvatures (parameters: window size; slope tolerance, default 1°;
curvature tolerance, default 10⁻⁴). Fisher–Wood–Cheng run the
classifier over a ladder of window sizes and define fuzzy membership
as the fraction of scales at which a cell is classified into a
feature — a continuous per-cell "passness" field in [0,1], the best
native fit to ADR 0003's spectra-over-taxonomy corollary among the
detection schemes. At 16 m cells a 3–33 window ladder spans 48–528 m
footprints.

Caveats: single-scale Wood output is strongly window-dependent (the
multi-scale form exists to fix this); the fraction-of-scales
aggregation is the citable content — substituting another aggregation
demotes the output to heuristic. The field form still needs
local-maxima extraction to yield pass *sites* (the paper itself does
this for named peaks and passes).

### Negative finding: geomorphons have no saddle class

Jasiewicz, J. & Stepinski, T.F. (2013) 'Geomorphons — a pattern
recognition approach to classification and mapping of landforms',
Geomorphology 182:147–156 (verified, DOI; GRASS r.geomorphon manual
fetched).

The corpus (ADR 0003, ROADMAP) names geomorphons as the recognized
tier-3 landform grammar, and it is the obvious first reach for pass
detection. It does not work: the published reduction of the 498
ternary patterns to 10 landform forms — flat, peak, ridge, shoulder,
spur, slope, hollow, footslope, valley, pit — **contains no saddle
class** (verified against the GRASS r.geomorphon manual and ArcGIS
documentation). Saddle configurations exist only among the raw
patterns, and no published rule isolates them; any such selection
would be a via heuristic layered on the representation. Geomorphons
remain useful as a general landform context layer; for passes they
would force exactly the invented discretization the doctrine
forbids.

### Pass value is emergent, not detected

White, D.A. & Barber, S.B. (2012) 'Geospatial modeling of pedestrian
transportation networks: a case study from precolumbian Oaxaca,
Mexico', Journal of Archaeological Science 39(8):2684–2696 (verified:
full PDF fetched and bibliographic record confirmed; DOI not fetched
directly); Verhagen, P. (2013) 'On the
Road to Nowhere? Least Cost Paths, Accessibility and the Predictive
Modelling Perspective', Proc. CAA 2010 (verified, repository PDF);
Herzog, I. (2014) 'Least-cost Paths — Some Methodological Issues',
Internet Archaeology 36 (verified, DOI).

From-everywhere-to-everywhere cumulative least-cost modelling (FETE):
solve LCPs between a dense regular grid of origin–destination pairs
over a slope-dependent cost surface and accumulate per-cell traversal
counts. Passes appear as corridor pinch-points at saddles with no
detection step at all, and per-pass *value* = corridor traffic
through the saddle — continuous, and priced against realized demand
rather than landform geometry. Herzog's standing warning applies:
the result is relative to the cost model and OD distribution — it
measures usefulness to the simulated pattern, not an intrinsic
property. The literature splits cleanly: topological measures
(persistence) price the vertical saving; emergent corridors price the
realized saving. No published scheme combines the two into one
intrinsic scalar — any combined number would be a via heuristic.

## Fords

### The hydraulic chain: relative discharge → width → depth and velocity

Leopold, L.B. & Maddock, T. (1953) 'The Hydraulic Geometry of Stream
Channels and Some Physiographic Implications', USGS Professional
Paper 252 (verified, full PDF); Park, C.C. (1977) 'World-wide
variations in hydraulic geometry exponents of stream channels',
Journal of Hydrology 33:133–146 (verified, ScienceDirect record);
Whipple, K.X. & Tucker, G.E. (1999) 'Dynamics of the stream-power
river incision model', JGR Solid Earth 104(B8):17661–17674 (verified,
DOI); Finnegan, N.J., Roe, G., Montgomery, D.R. & Hallet, B. (2005)
'Controls on the channel width of rivers', Geology 33(3):229–232
(verified, GeoScienceWorld record); Manning, R. (1891) 'On the flow
of water in open channels and pipes', Trans. Institution of Civil
Engineers of Ireland 20:161–207 (verified, NLI catalogue); Chow, V.T.
(1959) 'Open-Channel Hydraulics', McGraw-Hill (standard n tables).

Downstream hydraulic geometry: w = aQᵇ, d = cQᶠ, v = kQᵐ with
b+f+m = 1, a·c·k = 1; canonical downstream exponents b ≈ 0.5,
f ≈ 0.4, m ≈ 0.1 (at-a-station: 0.26/0.40/0.34, usable for stage
modulation). Park documents worldwide spread in the exponents —
they belong in config with the PP 252 defaults and Park cited for
the spread; the coefficients are regional and are config, full stop.

The units problem — via's discharge is relative — has a citable
answer: the stream-power/LEM literature routinely represents
discharge by a proxy and lumps all dimensional constants into
declared calibration coefficients (Whipple & Tucker 1999 is the
standard statement). One declared constant k_Q (m³/s per relative
unit) is that practice exactly; the exponents are dimensionless, so
k_Q is the single free scale — "how big are rivers in this world" —
and everything downstream inherits it. It is pure declared forcing,
unverifiable from inside via.

For width, Finnegan et al. derive W ∝ Q^(3/8) · S^(−3/16) · n^(3/8)
(width-to-depth ratio α ≈ 20) — built for exactly via's situation
(model-derived Q and S, no measured channels) and reducing to the
Leopold–Maddock scaling on typical concave profiles. Manning
(v = (1/n)·R^(2/3)·S^(1/2); wide-channel R ≈ d) then closes depth
and velocity per river cell: d = (n·(Q/W)/√S)^(3/5), v = Q/(W·d),
with reach-averaged slope from heights_cm along the receivers path
(single-cell slopes are noisy at cm quantization) and n from the
Chow tables (~0.030–0.050 natural streams) — the lithology-to-n
lookup is a named heuristic. Finnegan and Manning share the friction
assumption, so the pair is internally consistent, and n is declared
once.

### Crossability: the depth×velocity spectrum

Abt, S.R., Wittler, R.J., Taylor, A. & Love, D.J. (1989) 'Human
stability in a high flood hazard zone', Water Resources Bulletin
25(4):881–890 (verified via citing literature); Jonkman, S.N. &
Penning-Rowsell, E. (2008) 'Human Instability in Flood Flows', JAWRA
44(5):1208–1218 (verified, DOI and full text); Cox, R.J., Shand,
T.D. & Blacka, M.J. (2010) 'Appropriate Safety Criteria for People',
Australian Rainfall & Runoff Project 10, P10/S1/006 (verified,
official PDF); AIDR (2017) 'Flood Hazard', Australian Disaster
Resilience Guideline 7-3 (verified: PDF read directly, tables
first-hand); Smith, G.P., Davey, E.K. & Cox, R.J. (2014) WRL
TR2014/07 (verified, URL).

The flume-verified crossability currency is the product D·V (m²/s)
with independent caps. People-stability bands (Cox et al., reproduced
in Guideline 7-3): D·V ≤ 0.4 low hazard for children, ≤ 0.6 for
adults, 0.6–0.8 moderate (0.8 is the recommended working limit for
trained or well-equipped persons), 0.8–1.2 significant, > 1.2
extreme — the upper limit of stability observed in most
investigations; limiting still-water depth 0.5 m (children) /
1.2 m (adults); limiting velocity 3.0 m/s regardless of depth
(Jonkman & Penning-Rowsell: sliding instability precedes toppling
when shallow and fast — the velocity cap is mechanical, not
cosmetic). The general AIDR H1–H6 classification extends the same
product to vehicles and buildings (H1: D·V ≤ 0.3, D ≤ 0.3, V ≤ 2.0
… H6 > 4.0). Ford crossability is therefore a continuous spectrum
(D·V along the river graph, still-water depth alone on lake cells)
whose only discretizations are published bands.

Caveat: the thresholds are for modern adults in flood conditions —
conservative for a deliberate crossing on firm gravel, generous for
loaded travellers; the bands are config with these citations as
provenance.

### Historical corroboration and the riffle argument

Grayson, A.J. (2010) 'Thames Crossings near Wallingford from Roman to
Early Norman Times', Oxoniensia 75 (verified: PDF read directly);
Harrison, D. (2004) 'The Bridges of Medieval England', OUP (verified,
publisher record); Keller, E.A. & Melhorn, W.N. (1978) 'Rhythmic
spacing and origin of pools and riffles', GSA Bulletin 89(5):723–730
(verified, DOI).

Grayson's criteria for a serviceable ford: usable most of the year;
foot crossing at maximum depth ~2 ft (60 cm); firm approaches;
gravel bed; rivers shallower in straight reaches and at cross-over
positions between bends. The 60 cm wading depth brackets neatly
between Cox's 0.5 m child and 1.2 m adult still-water limits —
archaeology and flume experiments converge. Harrison: fords were the
normal crossing form in early Anglo-Saxon England, and bridges were
later built *at* ford sites — ford crossability is also the
bridge-siting prior. Keller &
Melhorn: pool–riffle spacing is 5–7 bankfull widths, so any reach
longer than ~7W contains a natural shallow — the citable licence for
treating crossability as a reach property at 16 m cells (any "riffle
depth-reduction factor" magnitude is uncited and would be a named
heuristic).

## Harbours

### Shelter: the wave-fetch index

Burrows, M.T., Harvey, R. & Robb, L. (2008) 'Wave exposure indices
from digital coastlines and the prediction of rocky shore community
structure', Marine Ecology Progress Series 353:1–12 (verified: full
PDF read, methods first-hand); Burrows, M.T. (2012) MEPS 445:193–207
(verified, DOI); Hill, N.A. et al. (2010) MEPS 417:83–95 (verified,
listing); lineage: Saville, T. (1954) 'The effect of fetch width on
wave generation', Beach Erosion Board TM-70 (verified, ERDC scan);
USACE (1984) Shore Protection Manual, 4th ed. (verified, archive
scan); Rohweder, J. et al. (2008) USGS OFR 2008-1200 (verified,
USGS record); Mason, L.A. et al. (2018) Scientific Data 5:180295
(verified: full article read).

Burrows' index: for each coastal cell of a binary land/sea raster,
fetch = distance to nearest land in each of **16** angular sectors,
capped at 200 km (provenance for the cap: the wave transition point
gF/U² < 22,000); index F = mean per-sector fetch (log₁₀ for
display), with a published three-scale hierarchical search and a
neighbour-averaging smoothing step. (Burrows' later wave-fetch GIS
layers use 32 sectors — data-product provenance, not the 2008
paper; a 32-sector variant is a declared adaptation.) F alone, on a
200 m grid, explained >50% of variation in the first principal
component of rocky-shore community structure across 185 Scottish
sites — the citable defence for shipping the wind-free form. Fully computable from heights_cm + sea_level (sea =
ocean-connected cells via self-receiver; via wants the index on
coastal *water* cells — a declared, trivial adaptation of the
focal-cell choice).

Not computable as published: the entire wind-weighted family —
Keddy (1982) REI, Ekebom et al. (2003) wave power, Burrows' W,
SPM design-wind effective fetch — needs a directional wind
climatology via does not have. With a uniform wind rose they all
degenerate to a constant times F, which is why F is the honest
form; the family is recorded as the upgrade path if via ever grows
winds. Bekkby et al. (2008, Marine Geodesy 31(2):117–127, verified)
attenuate surface exposure to the seabed with depth — a sanctioned
future refinement using via's exact bathymetry. Far-travelled swell
is invisible to every fetch scheme (accepted proxy limitation).

### The pre-modern depth window and sedimentation

Marriner, N. & Morhange, C. (2007) 'Geoscience of ancient
Mediterranean harbours', Earth-Science Reviews 80(3–4):137–194
(verified, DOI); Salomon, F., Keay, S., Carayon, N. & Goiran, J.-P.
(2016) PLoS ONE 11(9):e0162587 (verified: full article read);
Boetto, G. (2010) Bollettino di Archeologia on line, Vol. Speciale
(verified, venue); Votruba, G.F. (2017) The Mariner's Mirror
103(1):7–29 (verified, DOI); Morton, J. (2001) 'The Role of the
Physical Environment in Ancient Greek Seafaring', Brill (verified,
catalogue); de Graauw, A. (2025) Coastal Engineering Proceedings 38,
keynote (verified, DOI; his 2–5 m breakwater-founding figure is grey
literature — site returned 403, quoted from snippets).

The citable factual envelope a harbour score must respect: ordinary
merchantmen drew ~1–3.5 m, large ships to ~4.5 m (Boetto, applied
quantitatively by Salomon et al.); a harbour died for ships at ~1 m
of water column (Salomon et al., Portus/Ostia); anchoring — not
beaching — was standard for cargo vessels from the Classical period
(Votruba), so usable water depth defines the anchorage; sedimentation
is the life-limiting process (measured infill 8.5–10.5 mm/yr at
Ostia, 26.5 mm/yr at Portus — Salomon et al.; river-mouth proximity
and progradation destroyed harbours — Marriner & Morhange); shelter
is directional
with holding ground mattering (Morton, qualitative).

No published composite "natural harbour index" exists — the
geoarchaeology is descriptive, not algorithmic. The computable
components are all in reach (shelter from fetch; depth window from
bathymetry; sediment risk from river-outlet proximity weighted by
relative discharge, where ranking needs no units); any function
joining them is a named project heuristic. All sources are
Mediterranean-microtidal; tide is uncovered. Maximum practical
anchoring depth (anchor-cable scope) found no quantitative source
this sweep.

## Confluences and navigability

### Confluence detection and importance

Strahler, A.N. (1957) Trans. AGU 38(6):913–920; Shreve, R.L. (1966)
J. Geology 74(1):17–37; Shreve, R.L. (1967) J. Geology 75(2):178–186
(all verified, DOIs); Benda, L., Andras, K., Miller, D. & Bigelow,
P. (2004) 'Confluence effects in rivers', Water Resources Research
40, W05402 (verified, DOI; 167-confluence synthesis); Benda, L. et
al. (2004) BioScience 54(5):413–427 (verified); Rice, S.P., Roy,
A.G. & Rhoads, B.L. (eds., 2008) 'River Confluences, Tributaries and
the Fluvial Network', Wiley (verified, DOI).

Detection is definitional, not thresholded: in the link–junction
formalism a confluence is an interior node where ≥2 *channel* links
join — on via's artifacts, a strahler > 0 cell with ≥2 river donors
under the inverted receivers tree. Zero new parameters; the
channelization threshold already declared for strahler silently
parameterizes confluence counts (a fact to restate with any number).
Importance has a citable continuous form: Benda's symmetry ratio
A_t/A_m (drainage areas of the joining branches — exact from
area_cells, or the discharge ratio, where relative units cancel);
significant confluence effects become likely as the ratio approaches
~0.6–0.7 — an empirical tendency, kept continuous, never a cut.
Mapping geomorphic importance to settlement desirability is a
declared heuristic even though the measured quantity is standard.

### Navigability

Magirl, C.S. & Olsen, T.D. (2009) 'Navigability potential of
Washington rivers and streams determined with hydraulic geometry and
a geographic information system', USGS SIR 2009-5122 (verified: full
PDF read, thresholds extracted verbatim); Langbein, W.B. (1962)
'Hydraulics of river channels as related to navigability', USGS
Water-Supply Paper 1539-W (verified: full PDF read); Eckoldt, M.
(1980) 'Schiffahrt auf kleinen Flüssen Mitteleuropas', Stalling;
Eckoldt, M. (1984) IJNA 13(1):3–10 (both verified only via citing
literature — primary not read); Appel, E. et al. (2024) E&G
Quaternary Science Journal 73:179–202 (verified: open-access page
fetched); Filet, C., Laroche, C., Coto-Sarmiento, M. & Bongers, T.
(2025) 'As the water flows: A method for assessing river navigability
in the past', Journal of Archaeological Science 182:106315 (verified,
DOI and records); the medieval-extent debate: Edwards & Hindle
(1991) JHG 17(2):123–134; Langdon (1993) JHG 19(1):1–11; Jones
(2000) JHG 26(1):60–75 (all verified, listings).

Three citable instruments, one calibration caveat:

- **Published predicate bands** (Magirl & Olsen, from Washington DNR
  determinations): depth — probably not navigable below 0.61 m,
  indeterminate to 1.07 m, probably navigable above; slope —
  probably not above 0.0047, indeterminate to 0.0019, probably below
  0.0019. Literally 0005's anticipated "flow above threshold AND
  slope below threshold" form, with provenance. The slope bands are
  dimensionless and apply as-is; the depth bands need the k_Q
  calibration. Their regional regression (Dh = 0.23·Q^0.37, ft/ft³s)
  is humid-Washington-specific — adopted-with-provenance only.
- **A continuous spectrum underneath** (Langbein): minimum specific
  tractive force Ts = V²(f+0.6)/(1600·D^{4/3}) — V in ft/s, D in ft:
  Ts is dimensionless but the 1600 is unit-bearing, convert before
  use — with the published anchor "rivers requiring Ts > 0.002 are
  usually considered unnavigable" (Mississippi ≈ 0.00015, San Juan
  ≈ 0.02) — an ideal spectra-over-taxonomy citizen, derived for
  powered craft (the 0.002 anchor is a proxy for pre-modern
  rowing/towing).
- **A calibration-free cross-check** (Filet et al. 2025): detect the
  "plain section" of each river's longitudinal profile by
  change-point detection on reach slopes; validated against 18
  Gallic rivers with known ancient navigable extents. Needs only
  heights along the flow path — the best pure-DEM instrument if k_Q
  is deferred, and the natural co-driver otherwise (it ignores
  discharge; a flat trickle is not navigable).

The pre-modern depth anchor: Eckoldt's reconstruction — "even
shallow water depths between 0.3 and 0.7 m would have been
sufficient" for Roman keelless flat-bottomed barges and prams
(quoted from Appel et al. 2024, peer-reviewed secondary, who cite
"Eckoldt, 1985" — a work not matched to the 1980/1984 entries
above, a citation-chain discrepancy flagged in the open gaps;
Eckoldt's own minimum-discharge formula remains unextracted). Canal-era standards
(Freycinet 1879: 2.2 m water, 1.8 m draught; ECMT class I — both
verified only via secondary sources) bound the spectrum from above:
pre-modern navigation sits far below Class I. Navigability at low
water is governed by shallows, not mean flow (Langbein says so
explicitly; Magirl & Olsen's prediction intervals are wide) — via
has no flow-regime model, and no citable correction from its inputs
was found.

### Head of navigation, fall line, transshipment

Renner, G.T. (1927) Geographical Review 17(2):278–286 (verified);
Shankman, D. & Hart, J.L. (2007) Geographical Review 97(4) (verified,
DOI; pages not captured); Cooley, C.H. (1894) 'The Theory of
Transportation', Publ. American Economic Association 9(3):13–148
(verified, JSTOR/full text); Burghardt, A.F. (1971) Annals AAG
61(2):269–285 (verified, DOI); Michaels, G. & Rauch, F. (2018)
Economic Journal 128(608):378–412 (verified, DOI); Willan, T.S.
(1936) 'River Navigation in England 1600–1750', OUP (verified);
Satchell, M. (2017) CAMPOP navigable-waterways GIS documentation
(verified, listing).

Head of navigation is compositional — the upstream boundary of the
navigable set along each flow path; fall-line-like features are that
boundary coinciding with a slope break or lithology contact (via has
both). The economics is the already-recorded transshipment story:
Cooley's break-in-transportation theorem, Burghardt's gateway
cities, and — complementing Bleakley & Lin 2012 — Michaels & Rauch's
finding that Britain's post-Roman urban reset shifted towns to
navigable water. This literature justifies *why* such nodes get
settlement weight; it supplies no numeric scoring function — the
weight is a declared heuristic, and per 0008-doctrine-screen the
value should arise mechanically (mode-change penalties), not as an
authored label.

## Movement cost (what the corridor stage will consume)

This section resolves the open question research 0011 records under
"Pack/foot maximum grade" — the pre-wagon grade tolerance "has no
engineering-standard source" — and surveys the cost functions
themselves; adoption belongs to the corridors ADR.

### Energy currency: Minetti, with the critical-slope envelope

Minetti, A.E., Moia, C., Roi, G.S., Susta, D. & Ferretti, G. (2002)
'Energy cost of walking and running at extreme uphill and downhill
slopes', Journal of Applied Physiology 93(3):1039–1046 (verified:
full paper read); Llobera, M. & Sluckin, T.J. (2007) 'Zigzagging:
theoretical insights on climbing strategies', Journal of Theoretical
Biology 249(2):206–217 (verified via PubMed metadata; **paper body
not read — the critical-angle values below are from secondary
summaries and are unconfirmed**).

Minetti's walking cost per kilogram per metre, fitted over measured
gradients i ∈ [−0.45, +0.45] (R² = 0.999):

Cw(i) = 280.5·i⁵ − 58.7·i⁴ − 76.8·i³ + 51.9·i² + 19.6·i + 2.5 J/kg/m

(level 1.64 ± 0.50; minimum 0.81 ± 0.37 at i = −0.10; the quintic
diverges outside its fit range — clamp before evaluating, the classic
LCP pitfall). Subjects were ten elite mountain athletes: the curve
*shape* is the citable content; absolute magnitudes are config.

Llobera & Sluckin turn the unsourceable "maximum foot grade" into a
metabolic phase transition: minimizing cost per distance-made-good,
direct ascent is optimal below a critical slope and switchbacking at
a fixed effective gradient above it (reported ≈16° up / ≈12.4° down
— secondary-source numbers, pending primary confirmation). The
implementable form is a cost envelope: below s_crit charge Cw(s);
above it charge traversal at s_crit with length inflated by
|s|/s_crit — continuous, anisotropic, finite everywhere, no binary
passability mask. Their optimum mountain-path gradient (0.20–0.30 in
Minetti's data) is consistent.

### Time currency: Tobler, with its provenance stated

Tobler, W. (1993) 'Three Presentations on Geographical Analysis and
Modeling', NCGIA Technical Report 93-1 (verified: primary PDF read —
formula and multipliers verbatim; **non-peer-reviewed technical
report**, empirical basis Imhof 1950); Irmischer, I.J. & Clarke,
K.C. (2018) 'Measuring and modeling the speed of human navigation',
Cartography and GIS 45(2):177–186 (verified, DOI; body not read).

W = 6·exp(−3.5·|S + 0.05|) km/h, S = dh/dx (dimensionless gradient —
*not* percent, *not* degrees); flat 5 km/h; ×0.6 off-path; ×1.25
horseback (asserted without data). Irmischer & Clarke's
GPS-instrumented Gaussian refit (~200 West Point cadets; peak
~4 km/h, slightly downhill; slope in percent — another unit trap) is
the peer-reviewed corroboration of the shape. Naismith 1892 /
Langmuir 1984 (originals not inspected; the numbers here via Herzog
2014) survive as a sanity anchor (1 m climb ≈ 7.92 m level
distance) but the Langmuir piecewise form has a cost discontinuity
at 21.25% downhill that poisons least-cost search.

### Load and pack-route engineering standards

Pandolf, K.B., Givoni, B. & Goldman, R.F. (1977) J. Applied
Physiology 43(4):577–581 (verified, PubMed/journal record): the only
verified scheme with load as a parameter — M = 1.5W +
2.0(W+L)(L/W)² + η(W+L)(1.5V² + 0.35VG) — but it predicts energy at
a *given* speed, was derived at 0–12% grades, and goes negative
downhill without the Santee correction (USARIEM T01-11 —
**unverified**, existence known only from secondary sources). Cite
it for the (L/W)² load term and the terrain-factor η concept only.

USDA Forest Service (2008) Trail Fundamentals / FSH 2309.18 §23.1
design matrices (verified: training-package PDF read verbatim; the
canonical FSH text itself is 403-blocked): pack & saddle target
grades 2–20% by trail class, short-pitch maxima 15–30%. USDA Forest
Service (1935) Forest Trail Handbook (verified: scanned primary
read): ruling grade 15% for loaded pack animals, permissible
exceedances 16–20% ≤ ½ mi, 21–25% ≤ ¼ mi, 26–30% ≤ 100 yd, way
trails to 40% ≤ 100 yd. Hancock, J. et al. (2007) Equestrian Design
Guidebook, MTDC 0723-2816 (verified: FHWA-hosted text, Table 4-3):
targets ≤5–12%, exceptions 15–20% ≤ 200 ft — with the decisive
statement that stock easily master steady grades over 10% and even
20%: **the caps are erosion control, not animal capability**. These
are federal engineering guidelines, not peer-reviewed science —
declared-forcing provenance class, remarkably consistent across 70
years (1935 ruling 15% ≈ modern TC4).

### The audit doctrine

Herzog, I. (2013) in Bevan & Lake (eds.), 'Computational Approaches
to Archaeological Spaces', pp. 179–211 (chapter verified
bibliographically, not read; its 6th-degree Minetti refit is known
only via the movecost package docs, which also wrongly symmetrize
it with abs() — **do not adopt those coefficients without the
chapter**); Herzog, I. (2014) Archeologia e Calcolatori 25:223–239
(verified: full PDF read); Herzog, I. (2014) Internet Archaeology 36
(verified, DOI).

The selection doctrine any corridor ADR must satisfy: raw
slope-as-cost is indefensible; Tobler's two failure modes are
slope-unit confusion and using speed instead of time; discontinuous
rules (Langmuir) create search artifacts; Pandolf has no downhill;
currency (time vs energy) is an explicit declared choice; wheeled
vehicles are a separate, gentler regime (critical slope 12–15%;
"Roman roads almost never take slopes over 15%") — corroborating the
already-recorded turnpike ~5% as wagon-era engineering, distinct
from foot/pack tolerance.

## Open gaps (honest negatives from this sweep)

- No published intrinsic "crossing value" scalar for passes combining
  vertical saving with detour; the literature splits into persistence
  (vertical) and emergent corridors (demand-dependent).
- Geomorphons' 10-class reduction has no saddle class — the corpus's
  named landform grammar cannot detect passes without a heuristic.
- No citable riffle-location or riffle depth-reduction magnitude at
  16 m cells; no quantitative substrate→footing scheme (Grayson is
  qualitative); no bank/approach-condition raster scheme.
- k_Q (relative→absolute discharge) is unverifiable from inside via
  — pure declared forcing with Whipple & Tucker as the practice
  precedent; every metre and m/s downstream inherits it. Filet et
  al. 2025 is the only found navigability instrument that avoids it.
- No wind climatology → the whole wind-weighted exposure family is
  uncomputable; fetch F is the honest degenerate form. Swell is
  invisible to fetch entirely.
- No published composite harbour-quality index; no quantitative
  pre-modern maximum anchoring depth; all harbour sources are
  Mediterranean-microtidal.
- No flow-regime/seasonality model → low-water navigability and
  seasonal pass closure are uncoverable from via's inputs; no
  citable seasonal correction found.
- No published numeric settlement-pull weighting for confluences,
  gateways, or heads of navigation — mechanism yes, coefficients no.
- No slope-dependent cost function for loaded pack equids exists in
  verifiable form — the pack-animal Minetti is missing from the
  literature; Tobler's ×1.25 is asserted without data.
- Eckoldt citation chain: Appel et al. 2024 cite "Eckoldt, 1985"
  for the 0.3–0.7 m band — not matched to the 1980 monograph or
  1984 IJNA entries listed here; the chain is unresolved until the
  1985 work is identified.
- Pending-verification flags: Llobera & Sluckin's critical-angle
  values; Santee/USARIEM T01-11; Eckoldt's primary formula; Herzog
  2013 chapter coefficients; ECMT 92/2 and Freycinet primary texts;
  Hiscock 1996 (MNCR exposure classes); WEMo (Malhotra & Fonseca
  2007); Kirmse & de Ferranti end page; Shankman & Hart pages;
  Scarf 2007 details; Naismith 1892 and Langmuir 1984 originals.
