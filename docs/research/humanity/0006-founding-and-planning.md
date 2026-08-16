# Research 0006 — Settlement founding & planning regimes

Status: research corpus — pre-decision. Nothing in this document is
adopted; adoption of any mechanism requires its own ADR. Part of
docs/research/humanity (see README there).

## The question this literature answers for via

How settlements come into being and acquire street/plot structure:
the founding act (charter, plantation, plat), the template it stamps,
and the accretion that fills and outgrows it. It informs the
settlement, network, and morphology layers, with hooks back into
suitability and corridors (site selection, market catchments). The
evidence runs from classical Greece to 21st-century informal
settlement; era mostly enters as parameters on a few recurring
operators, with flagged structural exceptions.

## Models and mechanisms

### Conzenian town-plan analysis: organic accretion, plan-units, and the burgage-plot module

Conzen, M.R.G. (1960) 'Alnwick, Northumberland: A Study in Town-Plan
Analysis', Institute of British Geographers Publication 27; verified
via Society of Antiquaries of Scotland burgage-plot studies
(St Andrews, Perth) and Alnwick Civic Society survey notes.

A town plan is a composite of discrete plan-units, each laid down by
one founding or growth episode; later episodes accrete at the fringe
(fringe belts) or repletively infill earlier units — "organic" towns
are palimpsests of many small planned acts, not unplanned noise. The
generative module is the burgage plot: a strip of standard frontage
(2–3 perches; Conzen's "normal" English burgage 28–32 ft, with plots
at 3/4, 1 1/4 and other rational multiples of the modal width)
running back from a market street, subdivided and amalgamated over
centuries while the plot-boundary skeleton persists.

As code: simulable. State: street-segment graph with per-segment
frontage occupancy; plots as (frontage_width, depth) strips keyed to
a parent segment. At each growth event, allocate plots along the
highest-value unoccupied frontage (value from via's corridor/flow
field) at width = rational multiple of a modal-width parameter;
subdivision splits at 1/2 or 3/4 of the mode; fringe belts form when
growth pauses (institutional/large plots claim the perimeter).
Inputs: corridor graph, a market/nucleus seed, modal plot width and
depth as culture parameters. Skeleton persistence = never delete
boundaries, only mark built/unbuilt.

Era dependence: parameters only — modal frontage (medieval 28–32 ft;
Chicago 25 ft; modern subdivision 50–100 ft), plot depth, infill
rate. The accretion process is era-invariant; modern informal
settlements follow the same frontage-strip logic. Structural change
only in whether a zoning layer caps infill density.

### Medieval town plantation (Beresford): founding as a discrete seigneurial act with a standard package

Beresford, M.W. (1967) 'New Towns of the Middle Ages: Town Plantation
in England, Wales and Gascony', Lutterworth/Praeger (2nd ed. 1988);
verified counts: 124 plantations in English Gascony, >70 bastides
planted 1263–1297; ~400 bastides in SW France 1222–1373
(definitional range 300–700).

New towns were founded by a datable legal act: a lord (king, duke,
bishop, monastic house) selects a demesne site — typically at a
castle, river crossing, or road junction — grants a charter with
burgage tenure (fixed money rent, personal freedom, market right),
and lays out a grid of streets and equal plots to attract settlers.
The town either "takes" (fills its plots within a generation) or
fails, leaving an under-filled or deserted grid; Beresford documents
both. Plantation and organic growth coexist: planted grids accrete
organic suburbs, organic towns receive planned extensions.

As code: simulable as forcing + process. Forcing (declared per
era/culture): a founding-event stream {date, actor, site-selection
policy, plat template}. Process (gated): site selection scored on
via's suitability/corridor fields (defensible spur, bridging point,
distance from competing towns); template stamped as a graph patch
(street grid, plaza block, plot module); then a "take" test —
plot-filling driven by simulated market catchment, towns below
threshold stalling as partial grids. Same plot/segment graph as the
Conzen unit; needs slope, river network, and the existing town set.

Era dependence: the founding-act structure (actor declares site +
template + tenure incentive) recurs in every era — bastide lord,
Ostsiedlung locator, US townsite company are one operator with
different templates. Era changes parameters: template geometry, actor
type, incentive (burgage rent vs lot sale), site-selection weights
(defense weight collapses after ~1500, rail-access weight appears
~1850). Parametric, not structural.

### Bastide plat geometry as an explicit rule set

Beresford (1967); Barrett, C.J. (2018) 'Origins of the French
Bastides', Journal of Urban History 44(6); Monpazier (founded 1284)
dimensions verified via Historic Urban Plans and Perigord heritage
documentation.

The template: rectangular perimeter (Monpazier, the most regular
exemplar: 400 m × 220 m), orthogonal streets (principal ~8 m, sized
for two carts to pass on market days; lesser lanes and narrow
carreyrous/androns between plots), one central block reserved as an
arcaded market square (place des cornières) with covered halle,
church on a separate block off the square, equal house plots
(Monpazier ~160 m², roughly 8 m frontage × 20–24 m depth), plus
garden and field allotments outside the walls.

As code: simulable as a parameterized plat template (pure forcing):
{perimeter_w, perimeter_h, block module, main_street_width = 8 m,
lane_width, plaza_reservation = 1 central block, church_offset,
plot = (frontage, depth), extramural garden strip}. Stamping = local
grid aligned to the site's dominant terrain axis (bastides align to
ridge/contour, not cardinal points), rows dropped or bent where slope
exceeds threshold — this terrain-clipping is the process part and
distinguishes real bastides from ideal diagrams.

Era dependence: one era/culture instance of the general plat-template
forcing type. The slots (street width, plot module, plaza rule,
orientation law) are the invariant schema; values are 13th-century
Gascon. Orientation law is the interesting era variable: bastides
terrain-relative, Rome and the PLSS astronomical/cardinal, Laws of
the Indies wind/solar.

### German town law and the Ostsiedlung locator system: chartered law-families as replicable town DNA

German town law and Lübeck law literature (standard references:
Wikipedia/Grokipedia syntheses over Higounet, C. (1986) 'Die deutsche
Ostsiedlung im Mittelalter'; verified: ~1,000 towns adopted
Magdeburg-law variants from the Elbe to the Black Sea, ~100 towns
under Lübeck law around the Baltic; peak 12th–14th centuries).

Eastward colonization was executed by contract: a territorial lord
engaged a locator (entrepreneur) who recruited settlers, surveyed the
plat, and received a hereditary office (Vogt/Schulze) plus plots.
The town's constitution was copied from a mother city's law
(Magdeburg or Lübeck), producing law-families — explicit replication
in which each daughter town inherits market rights, council
structure, and plat conventions (central rectangular Ring/market
square of one or more full blocks, church off-square, standard
plots). ~1,000 Magdeburg-law and ~100 Lübeck-law towns demonstrate
template propagation at continental scale.

As code: simulable as forcing with a propagation process: law-family
= a named parameter bundle (plat template + institutional
coefficients such as market-day count, toll rates for via's flow
layer); new foundations copy the bundle of the nearest/parent town
(a preferential-attachment tree over the town graph) — inheritance
on the settlement graph, mutating only through declared config.
Foundation wave = a moving frontier field (colonization forcing)
intersected with local suitability. Deterministic given the seed of
the founding-event stream.

Era dependence: template inheritance recurs (Spanish colonial towns
from the 1573 Ordinances; US railroad towns from a company plat
book). Era sets the transmission topology: medieval law-families
propagate town-to-town (tree), early-modern empires broadcast one
codified law (star), railroads propagate along a line. Parametric
within one schema.

### Market charter as founding trigger and market-spacing law (Britnell)

Britnell, R.H. (1981) 'The Proliferation of Markets in England,
1200–1349', Economic History Review 34(2), 209–221; Letters, S.
'Gazetteer of Markets and Fairs in England and Wales to 1516'
(Institute of Historical Research); Kent case: <20 markets in 1200 to
>80 by 1350 (Kent Archaeological Society).

From the mid-12th century the crown licensed markets for a fee; the
charter (weekly market right, market court, tolls) is the economic
founding act, distinct from and often preceding physical planning.
Charters proliferated massively 1200–1349 (Kent quadrupled), then
thinned after the Black Death as unviable markets failed. Legal
doctrine (Bracton) held a new market injurious within 6 2/3 miles of
an existing one — one-third of a 20-mile day's round trip — an
explicit, citable spacing rule; in practice spacing was set by
competitive failure rather than the rule alone.

As code: simulable — the bridge between via's flow layer and
founding. State: chartered market points with weekly periodicity.
Founding rule: a candidate site receives a charter (forcing event)
and survives iff its catchment (Voronoi/gravity partition over via's
corridor-cost field, radius ~ half-day travel) captures demand above
a viability threshold; failures revert to village status. The
6 2/3-mile legal spacing is a declared forcing parameter; realized
spacing is emergent and gateable. Deterministic given demand field
and event stream.

Era dependence: the trigger institution changes name but not form —
market charter (medieval England), Marktrecht (Empire), county-seat
designation and rail depot siting (19thC US). The spacing parameter
scales with transport speed: half-day foot/cart radius ~10 km
medieval, ~8–11 mile rail-depot spacing on the Plains — era enters as
a travel-speed parameter in the same viability mechanism.

### Classical orthogonal planning (Hippodamian grid)

Standard syntheses over Castagnoli, F. (1971) 'Orthogonal Town
Planning in Antiquity', MIT Press; Cahill, N. (2002) 'Household and
City Organization at Olynthus', Yale; verified dimensions: Olynthus
blocks ~35 m × 86 m, avenues 5–7 m, streets ~5 m; Piraeus blocks
~2,400 m²; Miletus rebuilt on grid after 479 BC.

Greek colonial and rebuilt cities were laid out as uniform orthogonal
blocks (per-city constant modules; Olynthus ~35 × 86 m holding 10
houses in two rows of 5) between wide plateiai and narrow stenopoi,
with public zones (agora, sanctuaries) reserved as unbuilt block
groups. The plan expresses equal allotment to colonists (isonomia) —
the same equal-plot attractor as burgage plots and township sections.
Attribution to Hippodamus personally is largely retrospective
mythology; grids predate him.

As code: simulable as the earliest instance of the same plat-template
forcing: {block_w, block_h, plateia_width, stenopos_width,
agora_reservation_fraction, orientation = terrain-adapted}. Same
stamping operator as bastide/railroad templates; no distinct process
content beyond template + terrain clipping.

Era dependence: pure parameter instance of the template schema;
included because it anchors the claim that the grid is not a modern
invention — a "planted grid" operator must span 2,500 years with only
parameter changes (block module, street width, reservation rule).

### Roman centuriation: rural cadastral grid as persistent landscape forcing

Standard references on centuriation (Castagnoli; Campbell, B. (2000)
'The Writings of the Roman Land Surveyors', JRS Monograph; verified
module: centuria = 20 × 20 actus ~ 710 m square = 200 iugera = 100
heredia; internal 2-actus (71 m) strip subdivision; axes
cardo/decumanus).

Roman colonization divided rural land into centuriae of 20 × 20 actus
(~710 m square, subdivided into 100 heredia of ~0.5 ha) along two
orthogonal axes (cardo/decumanus) tied to a colonial town's grid,
with limites (boundary roads) on the module lines. The cadastral grid
outlives the empire: modern road and field patterns in the Po valley,
Tunisia and Provence still align to it — a survey lattice, once
stamped, constrains all subsequent settlement geometry for millennia.

As code: simulable, important for via's layering: centuriation (like
the PLSS) is a one-time forcing raster — an orientation field +
module lattice written into landscape state — that later eras' road
building and platting snap to (roads preferentially follow
limites/section lines as pre-cleared rights-of-way). Implementation:
a persistent vector field + line set on the terrain grid; subsequent
corridor costs discounted along lattice lines. Palimpsest behavior
arises mechanistically instead of by authoring.

Era dependence: structurally identical to PLSS 1785 (both exogenous
survey lattices predating settlement); parameters differ: module
710 m vs 1,609 m (1 mile), orientation astronomical/road-aligned vs
strict cardinal, allotment unit heredium vs quarter-section. The
persistence mechanism (corridor-cost discount on lattice lines) is
era-invariant.

### Laws of the Indies (1573): codified colonial town-founding ordinances

Ordenanzas de descubrimiento, nueva población y pacificación,
Philip II, 1573 (148 ordinances); English translation and analysis:
Crouch, D.P., Garr, D.J., Mundigo, A.I. (1982) 'Spanish City Planning
in North America', MIT Press; Mundigo & Crouch (1977) Town Planning
Review 48; dimensions verified via ArchDaily/PLEA analyses.

The most explicit written town-generation algorithm before the 20th
century: plaza mayor rectangular with length = 1.5 × width (ideal
~600 × 400 ft, min ~200 × 300 ft), sized to population ("not less
than 200 ft wide and 300 ft long, nor larger than 800 × 532 ft");
twelve streets leaving the plaza — four from the corners aligned so
the corners face the four winds (protecting streets from wind
exposure); street width by climate (wide in cold, narrow in hot);
church prominent but per-ordinance not necessarily on the plaza in
coastal towns; merchant arcades around the plaza; site rules
(elevation, water, not swampy, inland preference). Codification
largely formalized 80 years of prior practice, and compliance was
partial.

As code: simulable as the richest historical plat-template config:
every ordinance maps to a named parameter (plaza_aspect = 1.5,
plaza_area = f(projected_population), corner_street_rule = true,
street_width = f(climate_temperature) — coupling the human template
to via's existing climate field, a genuine cross-layer hook), plus
site-selection weights (elevation, water access, wind). The
population-scaled plaza makes one template parameter an explicit
function of a simulated variable.

Era dependence: template forcing with two features that become
standard in modern regimes: (1) rules as conditionals on environment
(climate-dependent street width) rather than constants; (2) rules
scaled to projected growth ("leave the plan open so it can spread
symmetrically"). Era is parametric; the conditional-rule structure
is what via should adopt for all template configs.

### US Public Land Survey System (Land Ordinance 1785): continental survey lattice prior to settlement

Land Ordinance of 20 May 1785, Confederation Congress; standard
analyses: Johnson, H.B. (1976) 'Order Upon the Land', OUP;
Linklater, A. (2002) 'Measuring America'; verified: townships 6 miles
square, 36 sections of 1 sq mi = 640 acres, section 16 reserved for
schools, minimum sale $1/acre, survey-before-sale.

Survey precedes settlement: townships 6 miles square on true N–S/E–W
lines, subdivided into 36 sections of 640 acres, sold sight-unseen in
aliquot parts (half, quarter, quarter-quarter = 160, 40 acres);
section 16 reserved for schools. Roads later follow section lines
(1-mile spacing), producing the cardinal road lattice of the Midwest;
town plats snap to quarter-section boundaries. The largest single
geometric forcing ever applied to a landscape — settlement pattern is
determined by the cadastre, not vice versa.

As code: simulable, same operator as centuriation: one-time lattice
forcing (module 1 mile, strict cardinal orientation; correction lines
for meridian convergence ignorable at via's scales or included as
declared kinks), with (a) corridor-cost discount along section lines,
(b) plat-snapping of later town templates to the lattice, (c)
allotment units (quarter-sections) as the farm-population capacity
field feeding via's demand layer. Contrast case for gating: PLSS vs
metes-and-bounds regions (e.g. Ohio vs Kentucky) show measurably
different road orientation entropy.

Era dependence: era-specific parameter values on the era-invariant
survey-lattice operator (shared with centuriation). Structurally new
vs Rome: the lattice is continuous over millions of km² and ignores
terrain entirely (roads climb slopes on section lines), so the
terrain-adaptation weight of the road process drops near zero — an
era parameter with visible geometric consequences.

### Railroad townsite platting and the T-town (Hudson)

Hudson, J.C. (1985) 'Plains Country Towns', University of Minnesota
Press; Encyclopedia of the Great Plains, 'T-Towns'
(plainshumanities.unl.edu, verified); Illinois Central townsite
program 1850s (Associates Land Co.); example: Buffalo Gap SD platted
1885 by townsite subsidiary of Fremont, Elkhorn & Missouri Valley RR.

Railroads (through townsite subsidiaries) platted standardized towns
at roughly regular intervals along new lines — spacing set by
grain-haul distance for farm wagons and locomotive watering, ~7–11
miles on the Plains — selling lots speculatively before settlement.
Plat morphologies form a small typology: "symmetric" (business
streets facing each other across ~300 ft of right-of-way holding
elevators and coal yards), "orthogonal", and the dominant "T-town"
(Main Street perpendicular to the track, depot at the head of the T).
Depot, elevator row, and Main Street are fixed template elements;
many plats never filled (paper towns).

As code: simulable and unusually clean: the rail line is an existing
via network edge; founding rule = place a plat template every d miles
along new track (d from haul-distance economics, declared or derived
from wagon speed × half-day); template = {orientation: track-relative
(a third orientation law after terrain-relative and cardinal — though
on the Plains the track follows the PLSS so the two coincide),
right_of_way = 200–300 ft, Main_St_width ~80–100 ft, block ~300 ft
with 25-ft lots, depot reservation}. "Take" test as in Beresford:
plats compete for the same catchment and most stay partial. Gate:
observed Plains depot-town spacing distributions.

Era dependence: same founding-act operator as bastides with
actor = corporation, incentive = lot sale, spacing law = transport
economics, orientation law = infrastructure-relative. By the 19thC
the founding trigger has moved from market law to transport
infrastructure — in via terms the founding-event generator becomes
endogenous to the network layer (new edge => candidate sites), a
structural coupling change, not just parameters.

### 19th-century speculative platting: the fungible lot

Thompson plat of Chicago, 1830 (58 blocks, verified via Chicago
Public Library and Chicago architectural histories); general
analysis: Reps, J.W. (1965) 'The Making of Urban America',
Princeton UP.

Subdividers platted grids ahead of demand to maximize salable
frontage: Chicago's 1830 plat set 66-ft streets (one surveyor's
chain), 16-ft alleys, and 25 × 125-ft lots; the module propagated
because lots became a standardized financial commodity — buildings
citywide are multiples of the 25-ft module. Speculative platting
decouples plat from settlement: vast platted areas remained vacant
for decades (some plats were vacated), so the plat is supply forcing
and occupation is the demand process.

As code: simulable: subdividers convert farmland parcels adjacent to
the growing edge into plat patches when expected lot price (a
function of distance/travel-time to center via via's corridor field,
plus rail/streetcar access) exceeds agricultural value — the classic
bid-rent threshold, computable deterministically on the graph.
Template: {street = 66 ft, alley = 16 ft, lot = 25 × 125 ft, block
from lot arithmetic (e.g. 2 rows × N lots)}. Occupancy fills by
accessibility order, leaving the documented vacant-plat halo.

Era dependence: the bid-rent conversion process is era-invariant (it
also drives Roman suburbium growth and modern sprawl); era parameters
are the plat module, transport speed field, and whether a regulatory
layer (subdivision regulations post-1920s) constrains the template.
The 66-ft street and 25-ft lot are chain/rod survey-unit artifacts —
via should derive era modules from the era's declared survey unit
(perch, vara, chain) rather than listing raw numbers.

### Modern zoning as envelope regulation (1916 NYC; Euclidean zoning 1926)

Building Zone Resolution, City of New York, 25 July 1916 (text at
nyc.gov); Village of Euclid v. Ambler Realty Co., 272 U.S. 365
(1926); analysis: Willis, C. (1995) 'Form Follows Finance'; verified
rules via Skyscraper Museum and MCNY Greatest Grid documentation.

The 1916 resolution overlaid three independent maps (use districts:
residence/business/unrestricted; height districts; area/yard
districts) on the existing street plat. Height rule: street-wall
height limited to a district multiple (1, 1.25, 1.5, 2, 2.5×) of
street width, above which the building sets back within a
sky-exposure plane (e.g. 1 ft back per 3–4 ft rise by district);
towers of unlimited height on <=25% of the lot. Euclid v. Ambler
(1926) upheld use-separation constitutionally, triggering nationwide
single-use district zoning. Zoning does not generate streets or
settlements — it clips the 3D envelope and sorts uses on an
inherited plat.

As code: simulable as a constraint layer, not a generator: per-parcel
config {use in allowed set, max street-wall height = k ×
street_width, setback plane slope, tower_lot_fraction = 0.25, min
yards}; a building-mass process maximizes floor area subject to these
— the documented "wedding-cake" massing emerges from the constraint,
exactly the tier separation via wants: zoning text = forcing, massing
= process, "wedding cake" = interpretation label.

Era dependence: structural change, not parameter change: zoning
introduces a per-parcel regulatory state variable earlier eras lack
(medieval building custom existed but was not parcel-mapped). Within
the modern era parameters vary hugely (height multiples, FAR after
1961, use matrices); across modern jurisdictions the schema is
stable. Pre-1916: constraint layer absent or reduced to
fire/street-line rules.

### Subdivision regulation and the engineered street hierarchy (FHA era)

Federal Housing Administration (1936) 'Planning Neighborhoods for
Small Houses' / (1938) Technical Bulletin No. 7 'Planning Profitable
Neighborhoods' (archive.org, verified); Southworth, M. &
Ben-Joseph, E. (1997) 'Streets and the Shaping of Towns and Cities',
McGraw-Hill; verified: FHA labeled gridiron 'bad', promoted
curvilinear loops and cul-de-sacs, blocks up to 1,300 ft, discourage
through traffic, 1941 minimum paved width 26 ft.

Federal mortgage-insurance standards (not law, but binding via credit
access) rewrote residential street geometry after 1936: discourage
through traffic, hierarchical streets (arterial > collector > local),
long blocks (up to ~1,300 ft), curvilinear alignment adapted to
topography, cul-de-sacs favored, minimum pavement widths (26 ft from
1941). The result is the measurable postwar signature: tree-like
local networks, low intersection density, high dead-end fraction,
high circuity — the geometric inverse of the 19thC grid, produced by
an equally explicit rule set.

As code: simulable: the same subdivider agent as 19thC platting with
a different template grammar — instead of stamping a lattice, grow a
street tree inside the parcel: one or two entry connections to the
arterial (max_connections), recursive branching with min/max block
length, cul-de-sac termination, curvature following terrain contours
(reuse via's slope field), lot module 60–100 ft frontage. All
parameters citable from FHA bulletins. Gate: dead-end fraction and
4-way intersection share flip between pre-1940 and post-1950 fabric
(Boeing 2020 documents the arc).

Era dependence: a within-modern-era regime shift (~1936–1990s,
partially reversed by New Urbanist codes after ~1995 — Boeing (2020)
'Off the Grid... and Back Again?', JAPA, measures the rebound). Via
needs era forcing at decade granularity in the modern period, and the
template grammar (lattice vs tree) is itself a config choice, not
just dimensions.

### Modern informal settlement: organic morphogenesis under no-forcing conditions

UN-Habitat World Cities Report / SDG 11.1.1 statistics
(unstats.un.org, verified: 1.12 billion slum/informal residents in
2022 = 24.8% of world urban population; sub-Saharan Africa ~51% of
urban population); morphology: standard references incl. Sobreira &
Gomes (2001) on fragmentation, Barros & Sobreira space-syntax studies
of favelas.

Roughly a quarter of the modern urban world (1.1+ billion people)
lives in settlements built without plat, charter, or code —
incremental plot-by-plot accretion on marginal land (steep slopes,
floodplains, rail/utility reserves), path networks emerging from
repeated pedestrian traffic, then consolidation and partial
regularization. Morphologically the medieval organic process re-run
at modern speed — the strongest empirical support for via's premise
that one mechanism set with different forcing spans all eras.

As code: simulable with mechanisms via already needs: occupation
events claim the cheapest available land not claimed by the formal
layer (informal growth is the complement of the forcing layers —
land with low formal suitability: steep slope from via's terrain,
flood-prone from via's hydrology); access paths as least-cost trails
that consolidate by use (trail-reinforcement, Helbing-style
active-walker dynamics, deterministic variant); plot sizes from a
declared small module. No new operators — the accretion process run
where template forcing is absent.

Era dependence: era-invariant process; what varies is the fraction of
growth captured by formal forcing vs left to accretion (a single
scalar per era/region: ~0 formal in early medieval suburbs, near 1.0
in 1950s US, ~0.5 in 21stC sub-Saharan cities). This "formalization
fraction" is the cleanest single era parameter in the domain and
directly measurable (24.8% global urban 2022).

## Gate candidates

- **Street-orientation entropy H / order phi** (phi = 1 single grid,
  0 uniform; H bounds ln(4) = 1.386 perfect grid to ln(36) = 3.584
  uniform over 36 bins). US/Canadian cities average roughly 13× the
  phi of European cities; most-gridded US (Chicago) approach phi ~0.9
  / H near the lower bound; organic European cores (Charlotte the
  least-ordered US case; Rome, São Paulo near maximum entropy) sit
  near H ~3.5. Gate: planted-grid output in the high-phi tail,
  organic accretion in the low-phi tail, palimpsest cities bimodal.
  Source: Boeing, G. (2019) 'Urban Spatial Order: Street Network
  Orientation, Configuration, and Entropy', Applied Network Science
  4:67 (arXiv:1808.00600); exact per-city phi values in the paper's
  table — pull before hard-coding thresholds.
- **4-way intersection share and dead-end (degree-1) fraction**.
  US/Canadian cities average roughly double the European 4-way share
  (Boeing 2019, 100 cities); within the US, pre-1940 fabric is
  grid-dominant, post-1950 FHA-era fabric flips to low 4-way share
  and high cul-de-sac fraction, partial recovery after ~2000 (Boeing
  2020, county-level). Gate: era-tagged fabric must reproduce the
  pre-1940 vs post-1950 flip. Sources: Boeing (2019) ANS 4:67;
  Boeing (2020) JAPA 87(1) (arXiv:2010.04771).
- **Plot frontage module by regime** (modal value, rational-multiple
  structure). Medieval burgages modal 28–32 ft (2 perches), peaks at
  3/4, 1.25, 1.5, 2× the mode (Alnwick; Scottish burghs); bastide
  plots ~8 m frontage, ~160 m² (Monpazier); Chicago lot 25 × 125 ft;
  FHA-era 60–100 ft. Gate: generated histograms must show discrete
  rational-multiple structure, not a continuous distribution.
  Sources: Conzen (1960); 'Burgage plot patterns and dimensions in
  four Scottish burghs', Proc Soc Antiq Scot; Monpazier heritage
  surveys; Chicago Thompson plat 1830 documentation.
- **Block dimensions by regime**. Olynthus ~35 × 86 m (streets
  5–7 m); bastide blocks from a ~400 × 220 m town with 8 m principal
  streets; Chicago-type ~300–330 × 600–660 ft with 66-ft streets and
  16-ft alleys; FHA blocks up to ~1,300 ft; PLSS rural "block" =
  1-mile section. Gate: block-area and aspect distributions per era
  template within these ranges, within survey-unit quantization
  (actus, perch, chain, vara). Sources: Cahill (2002); Monpazier
  surveys; Chicago 1830 plat; FHA Technical Bulletin No.7 (1938);
  Land Ordinance 1785.
- **Market/town spacing** (nearest-neighbor distributions, chartered
  markets and rail depot towns). Medieval England: legal injury
  radius 6 2/3 miles (Bracton's third of a 20-mile day journey) as
  declared parameter; realized density from Letters' Gazetteer
  (thousands of grants 1200–1349; Kent <20 to >80 markets, many
  later failing) gives realized spacing ~5–10 km in settled lowland;
  Plains depot spacing ~7–11 miles from wagon grain-haul economics.
  Gate: founding+survival must reproduce era-scaled spacing including
  the overshoot-then-extinction of the 1200–1349 boom. Sources:
  Britnell (1981) EcHR 34(2); Letters (IHR); Hudson (1985); Kent
  evidence: Archaeologia Cantiana 117.
- **Road alignment to survey lattice** (fraction of road length near
  cardinal bearings / centuriation axes; module recovery by Fourier/
  autocorrelation). PLSS: 1-mile section-line spacing, cardinal
  orientation dominating rural Midwest bearing histograms (vs
  metes-and-bounds Kentucky/Tennessee as control); centuriation
  20-actus (~710 m; 71 m substrip) module still recoverable in Po
  valley/Provence/Tunisia after ~2,000 years. Gate: after a lattice
  is stamped and later-era road growth runs, the module must be
  recoverable from the road-network spectrum. Sources: Land Ordinance
  1785; Johnson (1976); centuriation surveys (Campbell 2000;
  Istrian/Po valley studies).
- **Plaza reservation** (central-open-space area ratio and aspect).
  Laws of the Indies: aspect 1.5:1, 200 × 300 ft min to 800 × 532 ft
  max, scaled to projected population (ideal ~600 × 400 ft); bastides
  one full block (Monpazier place ~1 block of a ~10 × 6 block town);
  Ostsiedlung Ring often 1–2 blocks. Gate: planted towns reserve
  central open space within these ranges; organic towns instead show
  market-street/triangular widening (no block reservation). Sources:
  Ordenanzas 1573 (Mundigo & Crouch 1977, Town Planning Review 48);
  Beresford (1967); Monpazier plan.
- **Formalization fraction** (urban share in unplanned informal
  settlement). 24.8% of world urban population in 2022 (1.12 billion;
  1.1+ billion and 24.9% by end-2024); sub-Saharan Africa ~51%;
  developed regions <5%. Gate: for a modern-era run, emergent
  informal share should be monotone in the formalization parameter
  and land-suitability inequality, spanning this range. Sources: UN
  SDG 11.1.1, UN Statistics Division SDG Report 2024, Goal 11;
  UN-Habitat World Cities Report.
- **Built-envelope response to zoning** (street-wall height /
  street-width ratio distribution). 1916 NYC: district multiples 1×
  to 2.5× street width before mandatory setback; setback slope ~1:3
  to 1:4 by district; unlimited tower on max 25% of lot. Gate:
  massing under the declared constraints must show stepped setback
  profiles with break heights clustering at k × street_width for the
  declared k. Sources: Building Zone Resolution, NYC, 1916 (nyc.gov
  full text); Skyscraper Museum and Willis (1995) analyses.

## Pitfalls and contested claims

- Planned vs organic is a spectrum and a palimpsest, not a binary or
  an era property: every era produces both (bastides are 13thC grids,
  favelas 21stC organic fabric), and every real city mixes regimes in
  space and time. Any architecture selecting one generator per era
  will fail every gate on real cities; the correct structure is
  coexisting template-forcing and accretion processes with an
  era-dependent mixing fraction.
- Bastide (planted-town) counts are definitionally unstable: sources
  give 300–700 depending on definition (~400 for 1222–1373 common;
  Beresford counted 124 in English Gascony under a stricter
  criterion; Barrett 2018 challenges parts of the standard founding
  narrative). Do not gate on counts of foundings; gate on geometry
  and spacing.
- Survivorship/selection bias in the geometric record: preserved
  showcase plans (Monpazier, "the most regular of the planned towns")
  are the extreme tail; most planted towns were irregular,
  terrain-clipped, or failed partially filled (Beresford documents
  many failures). Calibrating to famous exemplars over-regularizes;
  the failure/partial-fill process is as essential as the template.
- Hippodamus attribution is largely myth (grids predate him;
  attributions of Rhodes and Olynthus are considered erroneous) —
  "Hippodamian" labels a plan type, not a single doctrine. Similarly
  the Laws of the Indies codified ~80 years of prior practice post
  hoc, with partial compliance: written rule sets lag and idealize
  practice, so treat ordinance parameters as the mode of a
  distribution, not a deterministic spec.
- Modern street-network statistics (Boeing's entropy/phi,
  intersection shares) are measured on today's OSM network — an
  aggregate of all eras plus demolitions and retrofits. They gate the
  palimpsest end state, not any single era's output; using them on a
  single-era generator without era-stratified fabric (e.g. Boeing
  2020's decade-tagged US data) is a category error.
- Ideal template dimensions were quantized in period survey units
  (actus, perch/rod, vara, chain); much apparent variation is unit
  variation plus survey error. Deriving modules from the declared
  unit system (as via config) is more mechanistic and more citable
  than hard-coding metric values from secondary sources — several
  disagree (e.g. Laws of the Indies plaza dimensions differ across
  analyses; verify against Mundigo & Crouch's translation before
  freezing config numbers).
- Zoning and subdivision regulation are constraint layers on an
  inherited plat, not settlement generators — conflating founding
  regimes (which create street/plot structure) with growth regulation
  (which clips envelopes and sorts uses) puts rules in the wrong
  tier. In via terms: founding templates and survey lattices are
  forcing on the plat process; zoning is forcing on the massing/use
  process; keep them separate config blocks.
- The founding-act event stream (which lord, which railroad, which
  year) is genuinely exogenous and historically contingent — the
  field has no accepted mechanistic model for when a founding wave
  starts (Ostsiedlung, bastide boom, railroad boom are geopolitical
  events). Model wave timing/intensity as declared forcing, gate only
  site selection, spacing, and survival; endogenizing the booms would
  be uncitable.
- Market-charter data measure legal grants, not functioning markets:
  many chartered markets never operated or died quickly (charter boom
  1200–1349, then mass extinction), so charter density over-counts
  realized central places by a large factor. Gate spacing/survival
  against the gazetteer's evidenced-active subsets, not raw grant
  counts.
- Quantitative morphology literature is thin precisely where the
  historical claims are strongest: burgage-plot metrology rests on a
  handful of case studies (Alnwick plus a few Scottish burghs),
  bastide plot statistics are heritage-survey grade, and there is no
  accepted global statistic for "fraction of historical fabric
  planned vs organic" before the modern slum indicator — expect to
  construct gates from primary plan measurements (cadastral maps)
  rather than find them ready-made.
