# Research 0002 — Urban morphology (within-settlement form)

Status: research corpus — pre-decision. Nothing in this document is
adopted; adoption of any mechanism requires its own ADR. Part of
docs/research/humanity (see README there).

## The question this literature answers for via

What happens *inside* a settlement footprint: how street systems,
plot partitions, and building fabric are laid down, filled, and
transformed. It informs the settlements → morphology end of the
causal chain, reading the corridor/street graph and settlement-scale
flows as inputs. The literature spans medieval accretion through
19th-century platting to modern zoning, and repeatedly poses the
project's own bet: one mechanism, era-varying parameters.

## Models and mechanisms

### Conzenian town-plan analysis (tripartite plan, plan units)

M.R.G. Conzen (1960), 'Alnwick, Northumberland: A Study in Town-Plan
Analysis', Institute of British Geographers Publication 27, London:
George Philip. Reappraisal: Whitehand, Samuels & M.P. Conzen (2009),
Progress in Human Geography 33(6). Verified.

The town plan decomposes into three superimposed element complexes:
streets (street system), plots (plot pattern), and building
block-plans. Each historical accretion phase deposits a 'plan unit' —
an area of internally homogeneous street/plot/building combination —
and later phases transform earlier units only slowly, because plots
and streets have high inertia (streets > plots > buildings in
persistence). Urban form is a palimpsest; the ordering of persistence
times is the causal claim.

As code: partly generative as architecture, not as a single rule — it
prescribes the state schema, not the dynamics. It would define engine
state: a street graph, a plot polygon partition referencing street
frontages, and building footprints referencing plots, with three
different modification rates/costs (street edits cost >> plot edits
>> building edits). Plan units then emerge as contiguous regions
sharing the parameter regime active when they were laid down; the
JSON's guidance is not to label them (interpretation tier). Inputs:
corridor/street graph from the existing network stage, terrain
constraints. The persistence hierarchy is falsifiable: street-line
survival time should exceed plot-boundary survival time should exceed
building survival time.

Era dependence: the schema is era-invariant; what varies is the
forcing regime per accretion phase (plot metrology, street width
standards, planning law). Era enters as time-varying parameters, not
structural change — exactly the project's 'one mechanism, era
forcing' bet, and Conzen's palimpsest evidence supports it.

### Burgage cycle (plot repletion dynamics)

M.R.G. Conzen (1960), Alnwick, IBG Publication 27 — the burgage cycle
concept; empirical follow-up: Scrase (1989), 'Development and change
in burgage plots: the example of Wells', Journal of Historical
Geography 15(4). Verified.

A plot under demand pressure fills incrementally: institutive phase
(frontage building), repletive phase (rear-of-plot accretion of
outbuildings/cottages along the plot tail), climax phase
(near-maximal building coverage), then recessive phase and 'urban
fallow' (coverage collapse, often followed by plot amalgamation and
redevelopment at larger grain). The driver is rent/demand per unit
frontage versus access and light constraints inside a fixed plot
boundary; collapse comes when obsolescence plus regulation (public
health, slum clearance) makes climax fabric uneconomic.

As code: generative and cheap. Per-plot state c_i(t) = built coverage
ratio plus a building-age field; update rule dc/dt = f(local demand
from settlement-level flows, frontage width, plot depth) with
saturation, a demolition/fallow transition when age × obsolescence
exceeds a redevelopment threshold, and an amalgamation operator
merging adjacent fallow plots. Inputs: plot geometry from the
subdivision stage, demand proxy from settlement flows/centrality.
Output is a continuous coverage field — no taxonomy needed.
Falsifiable against coverage-ratio trajectories (rise to climax,
fall) and against Spacematrix GSI ranges per fabric age.

Era dependence: parameters vary by era — demand growth rate, plot
fixity (medieval burgage boundaries near-frozen; 19thC American plats
allow easy resubdivision/amalgamation) — and the regulatory shock
that triggers the recessive phase (19th–20thC public-health law) is
exogenous forcing. The structure (fill–saturate–collapse–amalgamate)
is claimed era-invariant.

### Fringe-belt formation (Conzen; Whitehand's bid-rent + building-cycle mechanism)

Conzen (1960) IBG Pub. 27; J.W.R. Whitehand (1967), 'Fringe belts: a
neglected aspect of urban geography', Transactions of the Institute
of British Geographers 41; Whitehand (1972), 'Building cycles and the
spatial pattern of urban growth', Transactions IBG 56; Whitehand &
Morton (2006), Urban Studies 43(11). Verified.

During building-cycle slumps, housebuilding at the urban fringe
stalls but fringe land is cheap, so extensive, low-rent-bidding uses
(institutions, cemeteries, parks, utilities, barracks) capture it.
When the next boom resumes, residential accretion jumps over these
zones. Alternating booms and slumps deposit concentric 'fringe belts'
of large plots and low building coverage, which persist through
locational inertia and institutional site investment, occasionally
alienating back to housing when land value differentials grow large.

As code: generative — arguably the most mechanistic thing in the
Conzenian school. State: a land value field V(x,t) (bid-rent,
distance/access to center via the network stage), an exogenous or
endogenous construction-intensity cycle B(t), and a land-use class
per parcel with conversion costs. Update: when B(t) is high, fringe
cells convert to residential at high plot density; when B(t) is low,
extensive uses claim fringe cells (winning the bid only while
residential bidding is depressed); institutional cells get high
conversion cost (inertia). Inputs: settlement center(s), an
accessibility field, a declared boom/slump forcing series. The
prediction is spatial: rings of low-coverage large-plot fabric at
radii corresponding to slump-era fringes — checkable in the output
without any labeling.

Era dependence: building-cycle amplitude/period and the roster of
extensive uses are era parameters (medieval: town walls/commons and
monastic precincts act as fixation lines; 19thC: railways,
cemeteries, institutions; modern: ring roads, campuses, retail
parks). 'Fixation lines' (walls, ring roads) are forcing-tier
geometry in the JSON's terms. The mechanism itself (cyclical demand +
bid-rent sorting + inertia) is era-invariant.

### Plot metrology and parcel subdivision dynamics

T.R. Slater (1981), 'The analysis of burgage patterns in medieval
towns', Area 13(3): 211-216; burgage dimension studies (e.g. 'Burgage
plot patterns and dimensions in four Scottish burghs', Proc. Soc.
Antiquaries of Scotland); Fialkowski & Bitner (2008), 'Universal
rules for fragmentation of land by humans', Landscape Ecology; Usui &
Asami (2018), 'Statistical distribution of building lot frontage',
Journal of Geographical Systems 20; Usui (2019) on lot depth, EPB:
Urban Analytics and City Science. All verified.

Plots are created by subdividing block/land polygons under a
frontage-maximizing constraint: each plot needs street access, so
subdivision runs perpendicular to the street. Widths are set by a
metrological module (statute perch and its fractions in medieval
boroughs — typical English burgage width 28-32 ft, ~2 perches,
width:depth around 1:6) or by market minimum-frontage economics
later. Inheritance and sale drive further binary splitting of widths
into halves and quarters; amalgamation reverses it. Resulting
equilibrium distributions: lot frontage lognormal (Tokyo), parcel
areas power-law in urban cores and rural land, lognormal in suburbs.

As code: fully generative and already implemented in the graphics
literature — Vanegas, Kelly, Weber, Halatsch, Aliaga & Müller (2012),
'Procedural Generation of Parcels in Urban Modeling', Computer
Graphics Forum 31(2): 681-690 — deterministic OBB-split and
straight-skeleton block subdivision with target area/frontage
parameters. Engine version: recursive split of block polygons; split
axis chosen against street frontage; stop criterion an era-dependent
target width w* drawn deterministically from a seeded distribution;
plus a demand-driven resubdivision/amalgamation operator over time
(binary splitting reproduces the quarter-width structure Slater
observed). Inputs: block polygons from the street stage, frontage
graph, era metrology parameters. Gate on frontage/area distributions.

Era dependence: strongly parametric. Medieval = perch-quantized
widths, near-irreversible boundaries; 19thC US = surveyor's plat
(uniform lot widths, e.g. 25-ft lots, gridded blocks laid out in one
event — a forcing-tier 'platting' operator applied at foundation, in
the JSON's terms); modern = zoning minimum lot sizes and setbacks as
declared planning-law forcing. The subdivision operator is identical;
only the width prior, quantization, and reversibility change.

### Muratorian/Caniggian process typology (typological process, urban tissue)

G. Caniggia & G.L. Maffei (1979; English trans. 2001), 'Architectural
Composition and Building Typology: Interpreting Basic Building',
Alinea, Florence; Cataldi, Maffei & Vaccaro (2002), 'From Muratori to
Caniggia: the origins and development of the Italian school of design
typology', Urban Morphology 6(1); S. Muratori (1959), 'Studi per una
operante storia urbana di Venezia'. Verified.

Built fabric reproduces itself through a 'typological process': at
any moment a culture carries a 'leading type' — the house form
builders default to without design effort. Aggregation of the leading
type along routes produces homogeneous 'urban tissue' (tessuto); the
type mutates gradually between periods (mono-family row house →
multi-family infill → apartment block), and new tissue is built with
the current type while old tissue is transformed toward it. Route
hierarchy matters: matrix routes precede building, planned building
routes are laid to serve aggregation, connecting routes close loops —
a generative sequence.

As code: half-and-half. The route hierarchy (matrix route → building
routes → connecting routes) is a clean generative grammar for
tissue-scale street/plot layout and can run as code on the corridor
graph. The 'leading type' is the danger zone: implementing it as an
authored catalogue of era house-types violates the no-look-table
doctrine. The doctrinally safe reduction in the JSON: represent the
type as a small parameter vector (frontage, depth, storeys, coverage)
whose era values are declared forcing, and let aggregation + mutation
be the process. Otherwise treat as descriptive-only, for validation
vocabulary.

Era dependence: era dependence is the theory's core content — the
typological process IS the claim that types mutate continuously
between eras. It maps naturally onto 'same mechanism, drifting
parameters'; the mutation series itself must be exogenous forcing
(technology/culture), since the school gives no endogenous law for
why types change.

### Space syntax configurational analysis (Hillier & Hanson) + angular segment refinement

B. Hillier & J. Hanson (1984), 'The Social Logic of Space', Cambridge
University Press; B. Hillier & S. Iida (2005), 'Network and
psychological effects in urban movement', COSIT 2005, Springer LNCS,
475-490; A. Turner (2001) angular segment analysis. Verified.

Open space is represented as a graph of axial lines (fewest longest
sight-lines) or street segments; configurational measures are
computed on it — integration (normalized closeness: mean
topological/angular depth to all others) and choice (betweenness).
The claim: these purely configurational quantities carry social load
— integration cores host retail and strangers, segregated areas host
residents. Hillier & Iida show least-ANGLE distance predicts observed
movement better than metric or turn-count distance; the relevant
graph weight is angular deviation.

As code: an analytic measure, directly computable and deterministic —
on the engine's street graph, build the segment graph weighted by
angular change and compute closeness/betweenness at multiple radii.
It is not itself a growth rule, but it is the coupling variable that
makes other rules run (see natural movement / centrality as a
process). Continuous fields on a graph, no taxonomy. Cost: all-pairs
shortest path, fine at settlement scale.

Era dependence: the measure is era-invariant. Era enters through the
network it is computed on and through movement technology (pedestrian
angular radius ~ small; the vehicular era pushes the relevant radius
up — radius/mode weighting is a declared era parameter).

### Natural movement + 'centrality as a process' (the generative feedback loop of space syntax)

Hillier, Penn, Hanson, Grajewski & Xu (1993), 'Natural movement: or,
configuration and attraction in urban pedestrian movement',
Environment and Planning B 20(1): 29-66; B. Hillier (1999),
'Centrality as a process: accounting for attraction inequalities in
deformed grids', Urban Design International 4(3-4). Verified.

Grid configuration alone generates a baseline movement pattern
('natural movement'); movement-seeking land uses (retail) locate on
high-movement segments, act as multipliers attracting more movement,
which attracts further such uses and intensification — a positive
feedback that grows live centers on integration/choice cores and lets
them migrate when the grid changes. Empirically, configuration alone
accounts for roughly 60-80% of variance in observed pedestrian
movement rates in the 1993 studies.

As code: generative — the settlement-interior analogue of the
project's flows→network coupling. State: per-segment movement
potential M = f(choice, integration at era radius); land-use
intensity L per frontage. Update: dL/dt increases with M
(retail/commerce); M is recomputed including L as attractor weight;
building density/coverage (the burgage-cycle rule) reads local L as
demand. Deterministic fixed-point or slow relaxation each epoch.
Inputs: street graph, an exogenous total activity budget from
settlement-scale flows. Emergent output: continuous 'live center'
fields, unlabeled.

Era dependence: the radius of the governing centrality (walking vs
vehicular), the movement-multiplier strength, and use-separation law
(zoning can sever the feedback entirely — a modern forcing) vary by
era; the feedback loop itself is claimed universal.

### Street-network morphogenesis by local optimization + densification/exploration

M. Barthelemy & A. Flammini (2008), 'Modeling urban street patterns',
Physical Review Letters 100, 138702; Courtat, Gloaguen & Douady
(2011), 'Mathematics and morphogenesis of cities: a geometrical
approach', Physical Review E 83, 036106; Strano, Nicosia, Latora,
Porta & Barthelemy (2012), 'Elementary processes governing the
evolution of road networks', Scientific Reports 2, 296. Verified.

New centers (parcels/buildings) appear according to an attractiveness
field; the network extends by locally optimal connection of new
centers to the existing network (minimizing construction cost, one
segment growing toward the barycenter of unserved demand), producing
realistic deformed grids without global planning. Empirically
(Groane, 1833-2007): growth decomposes into 'exploration' (new roads
at the urbanization front, dominant early) and 'densification'
(infill subdividing existing cells, dominant late); mean intersection
degree stays ~2.7 over 174 years; cell shapes homogenize over time;
~90% of the 100 highest-betweenness 2007 routes already existed in
1833 — backbone persistence, the network-scale twin of Conzen's
street-persistence claim.

As code: fully generative, deterministic, and matched to the engine's
grid/graph substrate. State = planar graph + demand/attractiveness
field from suitability and flows; update = seeded sequential center
placement, connection via local cost minimization (cost includes
terrain slope from the nature side — the coupling to the existing
stages); a planned-grid operator (impose an orthogonal plat over a
region at foundation) is a forcing-tier event, in the JSON's terms,
for the 19thC frontier case. Gates: degree distribution, cell
statistics, backbone persistence.

Era dependence: the exploration/densification mix shifts with growth
stage (endogenous), construction-cost anisotropy shifts with
transport technology (era parameter), and top-down platting/planning
is a discrete forcing event. The structural form of the rule is
era-invariant; the 19thC American grid is the one case where forcing
dominates process.

### Universal block/cell statistics and city 'fingerprints'

Lämmer, Gehlsen & Helbing (2006), 'Scaling laws in the spatial
structure of urban road networks', Physica A 363(1): 89-95 (20
largest German cities); R. Louf & M. Barthelemy (2014), 'A typology
of street patterns', Journal of the Royal Society Interface 11:
20140924 (131 world cities). Verified.

The areas of cells/blocks enclosed by streets follow a
scale-invariant distribution P(A) ~ A^(-alpha) with alpha close to 2
across cities; the joint distribution of block area and shape factor
(block area / area of circumscribed circle) is a stable per-city
'fingerprint' that clusters cities into families (most US cities
separate from most European cities by abundance of mid-size regular
blocks vs broader shape mixtures). This is an outcome regularity, not
a mechanism — the fragmentation processes above must reproduce it.

As code: descriptive-only — but that is its value: a ready-made
falsifiable gate at the block scale, exactly parallel to the
slope-area theta and Horton gates on the nature side. Compute block
polygons from the generated street graph, fit the area distribution
tail, build the (area, shape-factor) conditional histogram, and
compare against the published families.

Era dependence: the exponent ~2 is claimed near-universal (an
era-independent gate); the shape-factor mix is era/regime-dependent
(planned-grid eras push mass toward regular mid-size blocks) — so the
fingerprint doubles as a check that era forcing moves the output in
the empirically observed direction.

### Spacematrix density typomorphology (FSI/GSI/L/OSR plane)

M. Berghauser Pont & P. Haupt (2010; revised open ed. 2021),
'Spacematrix: Space, Density and Urban Form', NAi Publishers / TU
Delft OPEN; earlier: 'The Spacemate: density and the typomorphology
of the urban fabric' (2005). Verified.

Any fabric sample is a point in a low-dimensional density space: FSI
(floor area ratio), GSI (ground coverage), L = FSI/GSI (mean
storeys), OSR (open space per floor area), plus network density N.
Empirical samples (Netherlands, Berlin, Barcelona) show recognizable
fabric regimes occupy distinct, compact regions of the FSI-GSI plane
— the plane, not the names, is the finding: form types are bands in a
continuous density spectrum.

As code: descriptive-only as theory, but ideal as the project's
output space: the engine's plot-coverage and storey fields integrate
directly to FSI/GSI/L per sample window, giving continuous spectra
(no taxonomy) and a gate — era runs should land in the empirically
occupied regions of the plane and move through it in the observed
direction as era parameters change (medieval core: high GSI, low L;
modernist: low GSI, high L at similar FSI).

Era dependence: purely a measurement space; era shows up as a
trajectory through the plane. Building-height technology (elevator,
steel) is the forcing that unlocks the high-L region — a clean,
citable era parameter.

### Urban morphometrics / numerical taxonomy (momepy, morphological tessellation)

Fleischmann (2019), 'momepy: Urban Morphology Measuring Toolkit',
Journal of Open Source Software 4(43):1807; Fleischmann, Feliciotti,
Romice & Porta (2022), 'Methodological foundation of a numerical
taxonomy of urban form', Environment and Planning B 49(4). Verified.

Replace subjective plan-unit delineation with measurement:
morphological tessellation (a Voronoi partition of space among
building footprints, capped at 100 m) gives a plot-proxy unit needing
no cadastral data; 74 primary + 296 contextual morphometric
characters per cell; Gaussian Mixture Model clustering yields
data-driven tissue types (demonstrated on Prague and Amsterdam). This
is the Conzenian program made computable and reproducible.

As code: descriptive-only (an analysis pipeline, not dynamics) — but
directly reusable as the project's gating instrument: run identical
morphometrics on generated fabric and on OSM/cadastral samples of
real medieval/19thC/modern fabric, and gate on distributional
agreement of the character vectors (two-sample tests per character,
or cluster-assignment cross-prediction). This sidesteps hand-picking
a few statistics and is citable methodology. The JSON notes GMM
clustering output is interpretation-tier if surfaced as labels.

Era dependence: none in the method; the reference samples supply
era-specific empirical targets.

## Gate candidates

- **Block/cell area distribution exponent**, P(A) ~ A^(-alpha) for
  street-enclosed blocks. Empirical: alpha ≈ 1.9 (20 largest German
  cities, Lämmer et al.) to alpha ≈ 2.0 (131 world cities, Louf &
  Barthelemy); scale-invariant over roughly 10^3-10^5 m^2. Source:
  Lämmer, Gehlsen & Helbing 2006, Physica A 363:89-95; Louf &
  Barthelemy 2014, J. R. Soc. Interface 11:20140924.
- **Conditional distribution of block shape factor given area** (city
  'fingerprint'); era/regime direction: planned-grid regimes
  concentrate mass in mid-size regular blocks, incremental regimes
  spread it. Empirical: four robust city families from hierarchical
  clustering of 131 cities; US and European cities fall in distinct
  sub-groups. Source: Louf & Barthelemy 2014, J. R. Soc. Interface
  11:20140924.
- **Mean intersection degree of the growing street network**, stable
  through growth. Empirical: ~2.7, approximately constant 1833-2007
  while node count grew from 255 to >5000 (Groane, Italy). Source:
  Strano, Nicosia, Latora, Porta & Barthelemy 2012, Scientific
  Reports 2:296.
- **Backbone persistence**: fraction of today's top-100
  highest-centrality routes already present at the historical start;
  plus the shift from exploration to densification and the
  homogenization of cell shapes over time. Empirical: ~90% of the 100
  most central 2007 routes already existed in 1833; exploration
  dominates early growth, densification late. Source: Strano et al.
  2012, Scientific Reports 2:296.
- **Configuration-movement coupling**: variance in pedestrian
  movement rates explained by street-network configuration
  (integration/choice); pooled effect sizes across studies.
  Empirical: R^2 ≈ 0.6-0.8 in the foundational London-area studies
  (reported range across models 0.57-0.86); meta-analysis pooled
  effect: choice 0.481 (p<0.001, strongest), integration 0.206
  (p<0.001), control insignificant; least-angle distance outperforms
  metric distance. Source: Hillier, Penn, Hanson, Grajewski & Xu
  1993, EPB 20:29-66; Hillier & Iida 2005, COSIT (Springer LNCS
  3693):475-490; Sharmin & Kamruzzaman 2018, Transport Reviews
  38(4):524-550.
- **Medieval burgage plot metrology**: frontage widths quantized to
  statute perch fractions; canonical width and aspect ratio.
  Empirical: English burgage width ~28-32 ft (~2 perches);
  width:depth ~1:6; plot-frontage series in surveyed burghs fit perch
  fractions (full, 3/4, 1/2, 1/4 widths); e.g. Cricklade mostly 2
  perches wide by 12 deep. Source: Slater 1981, Area 13(3):211-216;
  'Burgage plot patterns and dimensions in four Scottish burghs',
  Proc. Soc. Antiquaries Scotland; burgage studies literature.
- **Parcel/lot size distributions by zone**: area distribution family
  and frontage distribution. Empirical: parcel areas power-law in
  urban core and rural land, lognormal in suburbs (Fialkowski &
  Bitner); building lot frontages lognormal (Tokyo downtown
  districts, Usui & Asami); lot depths follow a related parametric
  distribution (Usui 2019). Source: Fialkowski & Bitner 2008,
  'Universal rules for fragmentation of land by humans', Landscape
  Ecology; Usui & Asami 2018, J. Geographical Systems 20; Usui 2019,
  EPB: Urban Analytics and City Science.
- **Position of generated fabric in the FSI-GSI(-L) density plane per
  era regime.** Empirical: fabric samples (NL, Berlin, Barcelona)
  occupy distinct compact regions — low-rise high-coverage historic
  cores (high GSI, L ~2-4) vs high-rise open modernist fabric (low
  GSI, high L); generated era runs must land in occupied regions, not
  empty ones. Source: Berghauser Pont & Haupt 2010/2021,
  'Spacematrix: Space, Density and Urban Form', TU Delft OPEN.
- **Full morphometric character distributions** (dimension, shape,
  spatial distribution, intensity, connectivity, diversity families)
  of tessellation cells and street segments vs real-city reference
  samples. Empirical: 74 primary + 296 contextual characters measured
  per morphological tessellation cell; reference distributions
  computable from open building/street data for any real benchmark
  city (Prague, Amsterdam demonstrated). Source: Fleischmann et al.
  2022, Environment and Planning B 49(4); Fleischmann 2019, JOSS
  4(43):1807 (momepy).
- **Persistence-time ordering of plan elements** (streets outlive
  plot boundaries outlive buildings) in any long-run simulation.
  Empirical: qualitative ordering with strong documentary support
  (Alnwick 1774-1956 plan sequence; Groane backbone stability); no
  single published exponent — gate as ordinal survival-curve
  separation. Source: Conzen 1960, IBG Pub. 27; Whitehand/Conzenian
  school; Strano et al. 2012 for the street-persistence
  quantification.

## Pitfalls and contested claims

- Space syntax's headline validation is weaker than folklore: Ratti
  (2004, EPB 31:487-499) shows regular grids (Manhattan-type) yield
  degenerate/contradictory axial results, small geometric distortions
  cause topological discontinuities (one configuration, two
  conflicting analyses), boundary conditions strongly distort
  integration (edge effect — critical when gating on finite generated
  maps), and the method ignores building height, density and land
  use. The 2018 meta-analysis pooled effect for integration is only
  0.206 — far below the 0.6-0.8 R^2 of the foundational studies —
  suggesting publication/selection effects; 'choice' (0.481) is the
  more robust correlate. Gate on choice/betweenness, use angular
  segment (not axial) representation, and buffer the study boundary.
- Axial map construction is partly subjective (fewest-longest-lines
  is under-specified), which historically made results
  analyst-dependent; use algorithmic segment/angular analysis (Turner
  2001; Hillier & Iida 2005) so the gate is reproducible.
- The Conzenian school is descriptive-analytic: burgage cycle, fringe
  belts and plan units were identified post hoc from map sequences of
  a handful of British towns; there is no calibrated, widely accepted
  computational implementation to lean on — the field's own
  computational turn (Fleischmann et al.) is measurement, not
  process. Any generative burgage-cycle or fringe-belt module will be
  a novel formalization; gate it hard or it becomes an authored
  story.
- Fringe-belt theory's mechanism (bid-rent + building cycles,
  Whitehand 1972) is contested in degree: later work (Whitehand &
  Morton 2006) emphasizes contingency, agency and planning decisions;
  treat the boom/slump series as declared forcing, not as an
  endogenous claim, and expect the belt signal to be noisy.
- Muratori/Caniggia 'typological process' resists doctrinally clean
  implementation: the 'leading type' is culturally authored content,
  and encoding era house-types as geometry catalogues is exactly the
  look-table the project forbids. Reduce types to small parameter
  vectors (frontage/depth/storeys/coverage) under forcing, or use the
  school only as validation vocabulary.
- Power-law claims in urban morphology share the field-wide fitting
  pathology: block-area exponents near 2 are typically fit over 2-3
  decades with binned OLS; rigorous MLE/Clauset-style tests often
  prefer truncated or lognormal alternatives. Gate on distribution
  shape over a declared range (and on the lognormal-vs-power-law zone
  contrast of Fialkowski & Bitner) rather than on a point exponent to
  two decimals.
- Named taxonomies pervade the literature (plan units, fabric types,
  city families, GMM tissue clusters) but are interpretation-tier
  under project doctrine: the citable move is to keep engine outputs
  as continuous fields/graph measures and let clustering happen only
  in labeled post-analysis — Fleischmann et al. 2022 is the precedent
  that even the morphology field now treats types as emergent
  clusters, not inputs.
- Era may be structural at one point: 19th-century American platting
  is a single top-down instantaneous act (grid imposed before
  settlement), unlike incremental medieval accretion — a
  same-mechanism framework must model platting as a discrete
  forcing-tier event (planned-grid operator) or the frontier case
  will falsify the 'one generator' claim; Strano et al.'s
  exploration/densification split and Louf & Barthelemy's US/European
  fingerprint separation are the quantitative tests that the forcing,
  not new mechanism, suffices.
- Burgage metrological analysis itself has been critiqued for
  confirmation bias (many perch variants tested until one fits;
  different regions used different perch lengths, e.g. 20-ft vs
  16.5-ft perches), so treat perch-quantization as a soft prior on
  medieval frontage widths, not a hard gate.
- Space syntax 'natural movement' correlations are cross-sectional;
  the generative feedback version ('centrality as a process', Hillier
  1999) is theory with case-study support, not a calibrated dynamic
  model — the movement→land-use multiplier strength has no published
  value and must be a free (declared) parameter.
- Building coverage trajectories for the burgage cycle (climax
  coverage fractions, fallow durations) exist only as case-study
  measurements in Conzen 1960 and successors (e.g. Scrase 1989 on
  Wells), not as compiled statistics; expect to digitize Alnwick's
  plot-coverage tables yourself if you want a quantitative gate, and
  treat any secondhand 'typical climax coverage' number as
  unverified.
