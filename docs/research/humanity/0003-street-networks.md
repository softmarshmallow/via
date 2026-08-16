# Research 0003 — Street & road network growth

Status: research corpus — pre-decision. Nothing in this document is
adopted; adoption of any mechanism requires its own ADR. Part of
docs/research/humanity (see README there).

## The question this literature answers for via

How street and road graphs grow — from use-reinforced footpaths between
settlements, through inter-city corridor construction, to intra-city
street fabrics that densify toward saturation. It informs the corridor,
network, and morphology layers of the causal chain, with settlements
and suitability entering as the source of growth centers and demand.
The era range runs from pre-engineering trail formation to modern
planned sprawl; most mechanisms below claim era-invariant rules with
era-dependent parameters, and the literature also supplies the
statistics any generated network could be gated against.

## Models and mechanisms

### Local-optimization street growth (leaf-venation rule)

M. Barthelemy & A. Flammini, 'Modeling urban street patterns', Phys.
Rev. Lett. 100, 138702 (2008); arXiv:0708.4360. Verified against full
text.

Growth centers (homes/businesses) appear at rate 1/tau_C with spatial
distribution P(r); road segments of fixed small length are added every
step from the existing network toward unconnected centers so as to
maximally reduce cumulative distance
Delta = [d(M,A)+d(M,B)] - [d(M',A)+d(M',B)]. Under |MM'| = const this
gives the update rule MM' ∝ u_A + u_B (sum of unit vectors toward the
centers stimulating point M) — the same rule as Runions' leaf-venation
model. A center stimulates network vertices in its relative
neighborhood (d(v,s) < max(d(s,u), d(u,v))), which lets several roads
reach one center and creates loops; centers become inactive once
reached. With no other ingredients this reproduces e = E/N ≈ 1.3,
L(N) ≈ 1.90·sqrt(N), and a form-factor distribution peaked at
0.4 < phi < 0.7; with exponential center density P(r) = exp(-r/r_c) it
yields cell areas P(A) ~ A^-1.9±0.05 matching Dresden.

As code: directly simulable, deterministic given seeded center
placement. State: planar graph (vertex positions, edges) plus a list
of active centers. Update: (i) every tau_C steps draw n new centers
from P(r) — P(r) is where via's suitability field would enter, centers
sampled from the settlement/suitability layer instead of by fiat;
(ii) each step, for every stimulated network vertex, compute the sum
of unit vectors to its relative-neighborhood centers and extend a
fixed-length segment in that direction; (iii) deactivate reached
centers. Terrain enters through P(r) and optionally an anisotropic
metric d(.,.) (slope-weighted distance) replacing Euclidean distance —
the rule itself is metric-agnostic. Parameter-free apart from tau_C,
segment length, and the center distribution; gateable on e, L(N), phi,
and P(A).

Era dependence: era is a parameter, not a structural change — center
arrival rate and P(r) (population forcing), segment length (surveying
granularity), and the distance metric (travel technology) vary; the
local-optimality rule itself is claimed universal ('in the absence of
a global design strategy'). Planned eras (gridiron platting) are
precisely the regime the model excludes — planning must be injected as
exogenous forcing (pre-laid segments), not by tuning this rule.

### City morphogenesis as division/extension of space (potential-field settlement + connection)

T. Courtat, C. Gloaguen, S. Douady, 'Mathematics and morphogenesis of
cities: a geometrical approach', Phys. Rev. E 83, 036106 (2011);
arXiv:1010.1762. Verified against full text.

The city induces a scalar potential
P_{C->x} = (alpha/d_min(x,C) - beta/sqrt(d_min(x,C)))
· integral(d mu_C / sqrt(d_perp(c,x))): a hard rejection tube of
radius lambda_0 = (alpha/beta)^2 around existing streets, attraction
~beta/d at long range, and local minima at 'good' settlement sites.
Each step one settlement is placed — with probability P_e at the
potential's argmin (organized city), else at random (unorganized) —
then connected to the network: candidate segments are those to the
relative neighborhood of the visible optimal points; the shortest is
always built and each additional admissible segment is built with
probability omega (n' ~ 1 + Binomial(omega, n-1)). omega = 0 gives
trees/organic dead-end fabrics, omega = 1 connects everything and
gives gridded, loopy fabrics; sprawl is added by placing a fraction
f_ext of settlements against an inflated rejection radius
K_ext·lambda_0. Four shaping parameters (P_e, beta, omega, f_ext).
Simulated meshedness M4 rises monotonically in both P_e and omega from
0 to 0.48, spanning the empirical range of the 10 French towns
measured (M4 0.20-0.47, organic ratio r_N 0.66-0.86, anisotropy
0.29-0.93); it reproduces bi-lognormal street-length distributions
(fit RMS 0.19-0.26 vs <0.2 for real towns) and small topological
radius.

As code: directly simulable, deterministic with seeded RNG. State:
planar graph plus settlement points. Inputs: an attractiveness prior
over space — in via this is exactly the suitability field, replacing
the paper's flat-land assumption; terrain/hydrology enter by
(a) modulating the potential and (b) masking the buildable domain.
The update rule is fully specified above; all parameters are scalars
declared in config. The JSON's sweep calls this the strongest single
candidate for a process-style implementation because its two
parameters (P_e, omega) are explicitly the 'organization' axes
academia itself uses to span organic-to-planned, and its output
statistics (M4, r_N, anisotropy, bi-lognormal lengths) are all
gateable. Caveat: the authors leave beta's influence unexplored, and
validation is on 10 French towns only.

Era dependence: era enters as a piecewise-constant parameter path
t -> (P_e, omega, l_max, f_ext, K, beta, lambda_0) — the authors model
city history exactly this way (e.g. souk core omega = 0.2, f_ext = 0
switching to industrial-sprawl omega = 0.8, f_ext = 0.15). Medieval
organic ~ low omega, low P_e; 19thC frontier platting ~ high omega,
high P_e with large lambda_0 (surveyed blocks); modern city ~ high
f_ext, K_ext (zoned large-parcel sprawl). Era is parametric; no
structural change to the rule set.

### Densification + exploration as the two elementary growth processes

E. Strano, V. Nicosia, V. Latora, S. Porta, M. Barthelemy, 'Elementary
processes governing the evolution of road networks', Scientific
Reports 2, 296 (2012); arXiv:1203.0300. Verified against full text.

From 7 georeferenced snapshots (1833, 1914, 1933, 1955, 1980, 1994,
2007) of the Groane area (125 km^2, north of Milan, never centrally
planned), network growth decomposes into exactly two classes of new
links, separated by a bimodal distribution of betweenness-centrality
impact delta_b(e*) = [b(G_t) - b(G_t \ e*)]/b(G_t): 'densification'
(low delta_b, k_min >= 2, links bridging existing streets, raising
local density around existing centres) and 'exploration' (high
delta_b, k_min = 1 dead-end links pushing the urbanisation front).
Exploration dominates early and almost disappears by 2007;
densification dominates late. Concurrently N grows 255 -> >5000, N is
linear in population (N ≈ 0.019·Pop), L_tot ~ N^0.54, <k> creeps
2.57 -> 2.8, r_N = (N1+N3)/sum(N_k) falls 0.87 -> 0.835, the degree-4
share rises 11% -> 15.5%, the cell-area exponent tau steepens
1.2 -> 1.9, cell-area relative dispersion falls 0.5 -> 0.26, and shape
factors homogenise toward a rectangular mode at Phi ≈ 0.62.

As code: descriptive-only as a generator — it names and measures
processes, it does not supply an update rule. Its value to via is as
the falsification battery for whatever generator runs: a correct
settlement-era simulation must (i) show bimodal delta_b of new links
with the exploration mode shrinking as free land is consumed,
(ii) reproduce L ~ N^0.5, (iii) homogenise cells (tau -> 1.9,
dispersion down), (iv) keep the high-BC backbone stable. Each is
computable on via's graph at successive epochs.

Era dependence: the mix ratio is era- and land-supply-dependent, not a
free parameter — exploration is characteristic of early/rural phases,
densification of mature urban phases, and the crossover is driven
endogenously by remaining land. The four historical phases (rural to
metropolitan-post-industrial) map to growth-rate forcing, not to
different mechanisms; the paper's core claim is mechanism invariance
across 200 years.

### Capacitated (logistic) network growth under boundary forcing

A.P. Masucci, K. Stanilov, M. Batty, 'Limited Urban Growth: London's
Street Network Dynamics since the 18th Century', PLoS ONE 8(8):e69469
(2013); arXiv:1206.5298. (Companion empirics: Masucci, Smith, Crooks,
Batty, Eur. Phys. J. B 71, 259 (2009) on London as random planar
graph.) Verified via PMC full text.

Nine time slices of Greater London 1786-2010 show intersections and
street segments each following a logistic (Verhulst) curve with
R^2 = 0.998/0.997 — growth is capacity-limited, a strong violation of
Gibrat's law; the 1953 green belt coincides with the
inflection/saturation. The A/B-road backbone grew only 4.2x in 224
years while minor roads absorbed most growth (densification again);
average degree is slightly decreasing (~2.6 downward, the network
becoming marginally more tree-like at the growing margin); link
lengths are lognormal in the urbanised core (R^2 0.96-0.97) vs
exponential in the pre-urban 1786 fabric.

As code: mostly descriptive-only, but it defines two things via could
implement mechanistically: (1) a hard exogenous boundary (planning law
as forcing) whose imposition should endogenously produce logistic N(t)
as infill exhausts space — a falsifiable emergent gate, not a curve to
be imposed; (2) the backbone-vs-capillary split: trunk roads saturate
early, growth continues in minor roads. State needed: none beyond the
growth model; the green belt is a config polygon.

Era dependence: the mechanism (space-filling densification) is
era-invariant; what is era-dependent is the forcing: legal growth
boundaries (green belt 1953), transport-driven demand pulses. Era is a
change in constraints/forcing, not in process.

### Active-walker trail formation (inter-settlement path emergence)

D. Helbing, J. Keltsch, P. Molnar, 'Modelling the evolution of human
trail systems', Nature 388, 47-50 (1997); full equations in Helbing,
Schweitzer, Keltsch, Molnar, 'Active walker model for the formation of
human and animal trail systems', Phys. Rev. E 56, 2527-2539 (1997).
Citations verified; equations from PRE companion (known to assistant,
abstract-level verified).

A ground comfort field G(r,t) obeys
dG/dt = (1/T(r))[G_0(r) - G(r,t)]
+ I(r)[1 - G/G_max] · sum_walkers delta(r - r_a(t)): walkers reinforce
the ground where they step (intensity I), and trails decay back to the
natural state G_0 with durability T. Walkers feel a trail potential
V_tr(r) = integral exp(-|r'-r|/sigma) G(r',t) dr' (sigma = visibility)
and steer by a compromise between the straight direction to their
destination and grad V_tr. Feedback: use makes trails more attractive,
attracting more use. A single effective parameter kappa
(durability × intensity / visibility) tunes output between the
direct-path system (weak trail attraction) and a minimal, bundled
'way system' with characteristic triangular junction islands (strong
attraction) — a Steiner-tree-like consolidation of many desire lines
into few shared paths.

As code: fully mechanistic and grid-native: state = a scalar field G
on via's terrain grid plus origin-destination demand between
settlements; deterministic PDE/agent-flux integration (the PRE paper
gives a macroscopic density formulation that avoids stochastic agents
entirely, fitting bitwise determinism). Terrain enters through G_0, T,
I (ground erodibility/regrowth — connecting to via's soil/vegetation
layers) and through anisotropic walking cost on slopes. This is the
natural mechanism for pre-engineering corridors (footpaths, drove
roads) that later get 'frozen' into the road backbone that Strano
shows persisting for centuries.

Era dependence: era-dependent parameters are traffic volume
(population forcing) and visibility/durability (vegetation, climate) —
and above all the transition OUT of this regime: once roads are
engineered (Roman, turnpike, modern), path formation stops being
use-reinforced and becomes cost-benefit construction (see Louf et
al.). Era is partly parametric (kappa, demand) and partly a structural
handover between two mechanisms; the handover date/technology would
have to be declared forcing.

### Cost-benefit growth of inter-city networks (hierarchy and hub-and-spoke emergence)

R. Louf, P. Jensen, M. Barthelemy, 'Emergence of hierarchy in
cost-driven growth of spatial networks', PNAS 110(22), 8824-8829
(2013); arXiv:1305.3282. Verified.

Nodes (cities, with masses/populations M_i) are connected sequentially
to the growing network: each new node links to the existing node/edge
maximizing R_ij = B_ij - C_ij, with benefit B_ij ~ k M_i M_j / d_ij^a
(gravity-expected traffic) and cost C_ij ~ d_ij per unit length. A
single effective parameter (the ratio of benefit scale to cost scale)
sweeps the topology continuously from star / hub-and-spoke (cost
negligible) through hierarchical trees to the minimum-spanning-tree
limit (cost dominant); hierarchical spatial organization emerges from
self-organization, not global optimization.

As code: directly simulable and deterministic: state = a set of
settlement nodes with populations (from via's settlement layer) plus a
growing corridor graph; update = argmax of an explicit scalar over
candidate connections; d_ij should be via's least-cost-path distance
over terrain (slope, river crossings) rather than Euclidean, which
couples the corridor layer to hydrology/terrain with no extra
machinery. Output feeds the corridor layer that seeds intra-city
exploration growth. Gateable qualitatively on hierarchy emergence and
against Strano's backbone-persistence statistic.

Era dependence: era enters through the cost/benefit ratio
(construction technology: railway vs turnpike vs highway), the
exponent a (distance decay of interaction, shrinking with transport
speed), and sequential arrival order (which settlements exist when).
The rule is era-invariant; parameters are era forcing. The same rule
with era-varying unit costs is the citable substitute for
hand-authored 'road eras'.

### Least-cost-path corridor reconstruction (sequential cost-benefit on terrain)

'Spatiotemporal reconstruction of ancient road networks through
sequential cost-benefit analysis', PNAS Nexus 2(2): pgac313 (2023).
Venue/existence verified via search; author list not independently
verified in this session.

Roads between known settlements are predicted by iteratively adding
least-cost paths (cost surface from slope and terrain friction) in an
order set by cost-benefit ranking, with each built road lowering the
cost surface for subsequent connections (reuse discount). The reuse
feedback produces trunk-and-branch consolidation — the Steiner-like
sharing of corridors — rather than a naive all-pairs LCP spaghetti.

As code: directly simulable on via's existing terrain: a cost raster
from slope/hydrology (fording/bridging penalties), Dijkstra LCP,
sequential insertion with a declared reuse discount factor.
Deterministic. This is the standard archaeology/geography-citable form
of inter-city corridor formation and composes naturally with Louf et
al. (which supplies the ordering economics) and Helbing (which
supplies the pre-engineering regime).

Era dependence: cost-surface weights are era technology (pack animal
vs cart vs rail gradients — maximum admissible grade is a hard,
citable era parameter), bridge cost vs ford, and the reuse discount
(how much existing infrastructure is worth). All parametric; the
structure of the rule is unchanged across eras.

### Parish-Mueller L-system city generation (CityEngine lineage)

Y.I.H. Parish & P. Mueller, 'Procedural Modeling of Cities', SIGGRAPH
2001, pp. 301-308. Verified.

Extended L-systems grow a highway/street graph from input image maps
(population density, water, elevation); global goals route highways
toward population peaks, local constraints snap/prune segments against
water and existing streets; street patterns come from named rule
templates ('rectangular raster', 'radial', 'elevation-following');
blocks are subdivided into lots and buildings by further grammars.

As code: runnable but authored-style, not mechanistic: the pattern
templates are look-tables — 'radial' and 'raster' are outcomes named
in advance, with no causal claim and no empirical gating anywhere in
the paper (it is a graphics paper; validation is visual). Its durable,
citable contributions are engineering ones via can reuse: the
global-goals/local-constraints split and the snap/extend/prune
geometric legalization of new segments, which any growth model needs.
Plumbing, not mechanism.

Era dependence: era is a style choice (template + parameters) —
precisely the failure mode via wants to avoid. No causal era content.

### Tensor-field-guided street networks

G. Chen, G. Esch, P. Wonka, P. Mueller, E. Zhang, 'Interactive
Procedural Street Modeling', ACM Trans. Graphics 27(3) (SIGGRAPH
2008). Verified.

Streets are traced as hyperstreamlines of a smooth 2D tensor field
assembled from basis fields (a grid element aligned to a direction, a
radial element around a center, boundary-aligned elements along
water/terrain contours), blended by distance weights; major/minor
eigenvector families give the two street directions; singularities of
the field become plazas/irregular junctions.

As code: runnable and deterministic, but the field is authored (the
user paints it) — as published it is a design tool with zero empirical
validation. However, unlike L-system templates, the substrate has a
defensible mechanistic reading via one honest fact: gridded fabrics
are locally aligned orientation fields, and terrain/coastline
alignment of those fields is empirical (Boeing 2019 shows single-grid
order as a measurable spectrum). If via ever generates planned-era
fabrics, deriving the tensor field FROM declared forcing (survey
baseline bearing, coastline, slope contours — all citable inputs) and
tracing streamlines is a transparent, config-declared implementation
of platting; the JSON notes it must be labeled as forcing-derived and
gated afterward on Boeing's phi/H_o statistics.

Era dependence: the field sources are era forcing: US 1785 Public
Land Survey / 1811 Commissioners' Plan = one global grid element (high
phi); medieval = no field (organic process instead); modern =
piecewise fields per planned district. Era changes which tier
generates streets — a structural change that would need an explicit
regime switch in config.

### Interactive geometric simulation of city growth (Weber et al.) and geometry-behavior coupling (Vanegas et al.)

B. Weber, P. Mueller, P. Wonka, M. Gross, 'Interactive Geometric
Simulation of 4D Cities', Computer Graphics Forum 28(2), 481-492
(Eurographics 2009); C.A. Vanegas, D.G. Aliaga, B. Benes, P. Waddell,
'Interactive Design of Urban Spaces using Geometrical and Behavioral
Modeling', ACM Trans. Graphics 28(5) (SIGGRAPH Asia 2009). Both
verified to exist at these venues.

Weber et al.: a time-stepped geometric simulation — land-use/traffic
proxies rank candidate street extensions, streets grow by templated
expansion rules, parcels subdivide, buildings age/upgrade; ~1 s per
step, fully interactive. Vanegas et al.: couples the procedural
geometry to a behavioral urban model in the UrbanSim tradition
(accessibility, land value, jobs/population allocation), so
street/parcel geometry responds to simulated demand rather than direct
authoring.

As code: runnable; middle of the spectrum. Weber et al. is
process-shaped but its scoring weights and expansion templates are
unvalidated heuristics — citable as engineering precedent, not as
mechanism; nothing is gated against network statistics. Vanegas et al.
is the most mechanistic of the graphics lineage because demand comes
from a behavioral model with its own (econometric) literature, but
that model is calibration-heavy and not deterministic-mechanistic in
via's sense. The JSON's verdict: cite this lineage for architecture
(the tight loop demand field -> street growth -> accessibility ->
demand), implement the demand side with via's own suitability/flows
layers, and gate with the physics-literature statistics these papers
never used.

Era dependence: era appears as scenario parameters (demand growth,
zoning); no claim of era-invariant mechanism. The structural content
is the coupling loop, which is era-invariant; all rates are forcing.

### Spatial-networks statistical framework (reviews anchoring all gates)

M. Barthelemy, 'Spatial Networks', Physics Reports 499, 1-101 (2011);
M. Barthelemy, 'The Structure and Dynamics of Cities', Cambridge
University Press (2016). Also A. Cardillo, S. Scellato, V. Latora, S.
Porta, 'Structural properties of planar graphs of urban street
patterns', Phys. Rev. E 73, 066107 (2006) — read in full this session.

Not a growth mechanism: the consolidated empirical phenomenology of
planar street graphs. Key content verified from the Cardillo full
text: twenty 1-sq-mile samples; degree distributions peaked (planarity
forbids fat tails, k rarely >5-6); meshedness M = (E-N+1)/(2N-5) spans
0.014 (Irvine lollipop suburb) to 0.348 (New York); normalization
against the two planar extremes, minimum spanning tree and greedy
triangulation, places real cities at ~70-80% of GT efficiency at a
cost near tree level (E_rel saturates ~0.8 for Cost_rel > 0.3;
grid-irons Cost_rel 0.24-0.4); medieval fabrics are cheaper and
slightly less efficient; self-organized fabrics have P(k=3) > P(k=4),
planned grids the reverse.

As code: descriptive-only; it supplies the normalization trick via
should adopt wholesale: gate generated networks by where they sit in
the (Cost_rel, E_rel) plane relative to the MST and GT computed on the
SAME node set — scale-free, dimensionless, and citable, ideal for
era-spanning gates.

Era dependence: the review literature itself documents which
statistics separate fabrics of different eras (degree-3 vs degree-4
dominance, meshedness, cost-efficiency position) — i.e., era should
appear as movement within these continuous spectra, matching the
'outputs are spectra' doctrine.

## Gate candidates

Numbers are quoted exactly from the sweep, including its hedges;
sampling and graph-extraction conventions matter (see pitfalls).

- **Edge-to-node ratio e = E/N (equivalently <k>/2) of the planar
  street graph.** 1.05-1.69 across world cities (tree = 1, 2D
  lattice = 2); model value ~1.3; Cardillo <k> range ~2.1-3.4; Boeing
  city means k-bar 2.35-3.55 (Table 1). — Barthelemy & Flammini PRL
  100:138702 (2008), from Cardillo 2006 + Buhl 2006 data; Boeing Appl.
  Netw. Sci. 4:67 (2019).
- **Total network length vs intersections: L_tot = mu·N^beta.**
  beta ≈ 0.49 (mu ≈ 1.51) cross-sectional; beta = 0.54 longitudinal
  (Groane 1833-2007); theoretical 1/2 for homogeneous filling. —
  Barthelemy & Flammini 2008 (fit to Cardillo/Buhl data); Strano et
  al. Sci. Rep. 2:296 (2012).
- **Meshedness coefficient M = (E-N+1)/(2N-5).** 0.014 (Irvine-2
  lollipop) to 0.348 (New York); organic/medieval 0.15-0.26; planned
  grids 0.26-0.35; tree-like suburbs <0.1. Courtat M4 for French towns
  0.20-0.47. — Cardillo et al. PRE 73:066107 (2006) Table II (read in
  full); Courtat et al. PRE 83:036106 (2011) Table I.
- **Block/cell area distribution P(A) ~ A^-tau.** tau ≈ 1.9 (Dresden),
  universal ~2 across the 20 largest German cities; Groane tau evolves
  1.2 (1833) -> 1.9±0.1 (2007) as urbanisation proceeds; exponent
  fluctuations of order 10%. — Laemmer, Gehlsen, Helbing, Physica A
  363:89 (2006); Strano et al. 2012 (read in full); Barthelemy &
  Flammini 2008.
- **Cell shape (form) factor Phi = 4A/(pi D^2).** Bulk of cells
  0.3-0.6 (Dresden); Groane: pre-1933 single Gaussian mean ~0.5, sd
  0.25; post-1955 bimodal with a second (rectangular) mode at 0.62;
  relative dispersion of cell areas falls 0.5 -> 0.26 with
  urbanisation. — Laemmer et al. 2006; Strano et al. 2012 (read in
  full).
- **Organic ratio r_N = (N1+N3)/sum_{k!=2} N_k and degree-4 share
  N4/N.** Groane: r_N 0.87 -> 0.835, N4/N 11% -> 15.5% over 1833-2007;
  French towns r_N 0.66-0.86 (whole city); Boeing P_4w regional means:
  US/Canada 0.334, Europe 0.172, Latin America 0.257. — Strano et al.
  2012; Courtat et al. 2011 Table I; Boeing 2019 Tables 1-2 (all read
  in full).
- **Dead-end (degree-1) node fraction P_de.** City medians 0.027
  (Manhattan) - 0.395 (Helsinki); regional means US/Canada 0.116,
  Europe 0.172; correlates negatively with grid order
  (r(phi, P_de) = -0.376). — Boeing, Applied Network Science 4:67
  (2019), Tables 1-2 (read in full).
- **Street orientation entropy H_o (Shannon, 36 x 10-degree bins,
  nats) and orientation-order
  phi = 1 - ((H_o - H_g)/(H_max - H_g))^2, H_max = ln 36 = 3.584,
  H_g = ln 4 = 1.386.** H_o from 2.083 (Chicago, phi = 0.899) to 3.582
  (Charlotte, phi = 0.002) over 100 world cities; regional means phi:
  US/Canada 0.427, Europe 0.033; 49% of cities have N-S-E-W as modal
  bins. — Boeing 2019 (read in full).
- **Average circuity zeta = L_net/L_greatcircle.** 1.011 (Buenos
  Aires) - 1.148 (Caracas); gridded US cities 1.1-1.6% above
  straight-line; topography-constrained cities 13.3-14.8%;
  r(zeta, k-bar) = -0.672. — Boeing 2019 (read in full).
- **Betweenness-centrality backbone persistence.** >90% of the 100
  highest-BC links (and ~60% of the top 1000) of the 2007 Groane
  network already existed in 1833; the BC-impact distribution of new
  links is bimodal (densification vs exploration modes), with the
  exploration mode vanishing by 2007. — Strano et al. Sci. Rep. 2:296
  (2012), Figs 5-6 (read in full).
- **Betweenness distribution invariance (bimodal tree+loop regime).**
  BC distribution statistically invariant across 97 world cities;
  high-BC nodes form an underlying tree, low-BC regime = loops;
  spatial clustering of high-BC nodes grows with edge density. —
  Kirkley, Barbosa, Barthelemy, Ghoshal, Nature Communications 9:2501
  (2018).
- **Street-length distribution form.** Lognormal in urbanised fabrics
  (London R^2 0.96-0.97; Amiens bi-lognormal mixture, e.g. p = 0.5,
  m_- = 2.2, sigma_- = 0.3, m_+ = 4.3, sigma_+ = 1.2 in log-space);
  exponential in pre-urban/rural fabric (London 1786); new-link length
  ell_90% shrinks 625 m -> 225 m over Groane's urbanisation. — Courtat
  et al. 2011; Masucci, Stanilov, Batty, PLoS ONE 8:e69469 (2013);
  Strano et al. 2012.
- **Logistic (capacitated) growth of N(t), E(t) under a boundary
  constraint.** London 1786-2010, 9 slices: logistic fits R^2 = 0.998
  (nodes), 0.997 (edges); trunk (A/B) roads grew only 4.2x while minor
  roads absorbed most growth; explicit Gibrat violation. — Masucci,
  Stanilov, Batty 2013 (PMC full text).
- **Cost-efficiency position relative to MST/GT bounds on the same
  node set.** Real cities reach E_rel ~ 0.7-0.8 (70-80% of
  greedy-triangulation efficiency) at Cost_rel 0.24-0.40; medieval
  fabrics cheaper/slightly less efficient than grid-irons; E saturates
  ~0.8 for Cost_rel > 0.3. — Cardillo et al. 2006, Table III / Fig. 3
  (read in full).
- **Nodes-population proportionality during growth.** N linear in
  population over two centuries in Groane (N ≈ 0.019 x Pop, i.e. ~53
  inhabitants per intersection, constant in time). — Strano et al.
  2012, Fig. 2a (read in full).

## Pitfalls and contested claims

- The block-area power law is fragile: Barthelemy-Flammini get
  EXPONENTIAL cell areas under uniform center density and recover
  A^-1.9 only with an exogenous exponential population-density
  gradient — so the exponent is a joint property of network mechanism
  AND settlement distribution; gating the road model alone on tau
  double-counts the suitability layer. The exponent also fluctuates
  ~10% between runs, and parts of the literature (Fialkowski & Bitner)
  argue log-normal fits blocks better than a power law. Use tau as a
  soft gate with a declared fitting window, never a hard one.
- Degree distributions carry almost no information for planar street
  graphs (peaked, k <= 5-6 by planarity); any 'scale-free streets'
  claim in older literature refers to dual/named-street graphs or
  betweenness, not junction degree. Gate on meshedness, r_N, and the
  k=3 vs k=4 balance instead of degree tails.
- Boeing's own caveats on orientation order: H_o and H_w are
  statistically redundant (r > 0.99); phi is the 'extent of a SINGLE
  grid', so multi-grid cities score near-disordered (Buenos Aires
  phi = 0.151 despite being fully gridded — its histogram shows
  several competing grids); and the paper explicitly warns spatial
  order must not be conflated with planning or functional order. phi
  is a good spectrum output, a bad planned/organic classifier.
- Densification/exploration is validated on ONE region (Groane);
  Strano et al. state explicitly it 'cannot be extended to the
  generality of urbanisation processes' without further cases. Treat
  it as a falsification battery, not established universal law. Same
  for Courtat (10 French towns) and Masucci (London only).
- Backbone-persistence statistics are vulnerable to survey bias:
  older historical maps systematically record major roads and omit
  minor ones, which inflates the measured persistence of
  high-centrality links. The >90% figure survives scrutiny in Groane
  but should be gated with tolerance in via.
- All OSM-derived statistics (Boeing's H_o, P_de, P_4w, circuity)
  depend on graph-simplification conventions (node consolidation,
  roundabout collapsing, whether service roads are included). Via must
  pin its own graph-extraction convention and compare like-with-like,
  or gates will fail for bookkeeping reasons.
- Meshedness/efficiency values from Cardillo are 1-square-mile
  samples, not whole cities; the sampling window changes the numbers
  (whole-city M is generally lower). Declare the sampling protocol as
  part of every gate.
- The graphics lineage (Parish-Mueller L-systems, CityEngine
  templates) is authored style by construction: pattern names
  ('radial', 'raster') are look-tables with zero empirical validation;
  its legitimate reuse is geometric legalization machinery
  (snap/extend/prune) and the global-goals/local-constraints
  architecture. Tensor-field streets (Chen 2008) are salvageable only
  if the field is DERIVED from declared forcing (survey bearings,
  coastlines, slope) rather than painted, and then gated on Boeing
  statistics.
- Two structurally different growth regimes must not be blended
  silently: use-reinforced path formation (Helbing) versus engineered
  cost-benefit construction (Louf; sequential LCP). The literature
  gives no continuous interpolation between them — the handover (when
  a society starts building rather than treading roads) is an
  era-forcing switch and via should declare it, not fit it.
- Courtat's model has known soft spots the paper itself concedes: the
  influence of the long-range potential parameter beta is left
  unexplored, EM bi-lognormal fits (RMS thresholds ~0.2) are weak
  evidence, and the potential-field form is chosen for analytic
  convenience among 'several possible fields'. Its parameter semantics
  (P_e, omega) are its real strength; its specific potential is
  replaceable by via's suitability field without loss of citability.
- Barthelemy-Flammini's centers are assumed independent of the
  network — the authors flag this as the model's most important
  limitation ('integrating the correlation centers-network is the next
  most important step'). Via's coupled settlements -> roads ->
  suitability loop is exactly that correction, but it means published
  B&F statistics were obtained WITHOUT the coupling; after coupling,
  expect quantitative drift and re-gate on empirics, not on the
  model's own published outputs.
- L ~ sqrt(N) presumes statistically homogeneous space-filling; it
  breaks at hard boundaries and under sprawl-vs-infill regime shifts
  (Masucci's Gibrat violation). Fit exponents per growth phase, not
  across the whole history.
- The PNAS Nexus sequential cost-benefit ancient-roads citation was
  verified only to venue/title level in this session (author list
  unverified) — verify before putting it in an ADR.
- Era-statistics conflation: US grid order is downstream of specific
  legal instruments (1785 Land Ordinance, 1811 Commissioners' Plan,
  Law of the Indies), i.e. forcing — a generator that 'discovers'
  gridiron endogenously for 19thC America without declared platting
  forcing is overfitting the wrong mechanism, however good its
  Boeing-gate scores.
