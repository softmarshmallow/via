# Research 0014 — Corridor network extraction and multimodal movement

Status: research corpus — pre-decision (2026-08-22). Nothing in this
document is adopted; adoption of any mechanism requires its own ADR.
Commissioned for the via-corridors ADR as the third corridor-bearing
dossier beside 0005 (transport eras: freight mechanisms, era cost
vectors) and 0013 (affordance detection; §Movement cost carries the
foot/pack cost functions). Those two leave exactly two gaps this
dossier closes: **how a corridor network is extracted from a cost
surface**, and **what multimodal land+water cost structure is citable
for the pre-modern era**. Compiled from three verified sweeps
(2026-08-22); every item carries its verification status — *primary*
(full text read), *abstract* (abstract/landing page only), or
*secondary via X* (named intermediary read). The number 0014 was
briefly used by an ML-path dossier that was moved out of the repo
(now pinned GitHub issue #2); that document never entered the corpus
and the number is reused here.

## The endpoint problem

Every corridor-network method carries endpoint structure somewhere;
the sweep's central negative finding is that **no published method
derives a network from a cost surface with no endpoint choice at
all**. An accumulated cost surface is defined relative to an origin,
so a fully source-free construction is arguably ill-posed. The
published frontier is *endpoint-honesty*: endpoints that are uniform
(FETE's lattice), terrain-derived (morphometric sites), or randomized
under a declared distribution (Frachetti's iterated random origins).
The taxonomy:

| method | endpoint structure |
|---|---|
| FETE (White & Barber 2012) | uniform sampling lattice (spacing + boundary are the residual choices) |
| MADO / focal mobility networks | origins required, destinations eliminated |
| circuit theory (McRae et al. 2008) | source/ground node pairs required |
| Frachetti et al. 2017 | 5,000 random origins × 500 iterations (an endpoint *distribution*) |
| Helbing active walkers | entry points, destinations, and usage rates required |
| Llobera 2000 accessibility | endpoint-free but not a network (local kernel average) |

A second consolidated negative finding, from the growth-model sweep:
**no published model grows a road network between natural affordance
sites** (passes, fords, heads of navigation, harbours) as first-class
endpoints. The pieces exist separately — Stahlberg's connection set K
is formally endpoint-agnostic; Verhagen, Nuninger & Groenhuijzen
(2019) §11.4.1 explicitly blesses fords and waypoints as "additional
nodes to be connected" or as cost-surface attractors — but nobody has
published the combination. An adopting ADR assembles cited pieces; it
does not follow a single published pipeline, and must say so.

## Path-density methods (endpoint-honest spectra)

### FETE — White & Barber 2012 [primary]

White, D.A. & Barber, S.B. (2012), "Geospatial modeling of pedestrian
transportation networks: a case study from precolumbian Oaxaca,
Mexico", *Journal of Archaeological Science* 39(8): 2684–2696,
doi:10.1016/j.jas.2012.04.017. Full PDF read (author-hosted).

Procedure, verbatim where quoted: a "regularly-spaced sampling grid"
is placed over the terrain (production runs: 100-pixel spacing on a
5000×4000 grid; illustration: 10 pixels); Dijkstra runs from each
sample point; backlinks give least-cost paths to every other sample
point; "the locations of the paths are then recorded in a shared
accumulative surface" — **per-cell path-traversal counts**. The result
is "a travel probability surface that, with some processing,
resembles a dense circulatory system or road network." Corridors are
read off by thresholding: the path-frequency distribution "appears to
follow that of a scale-free power law (Clauset et al., 2009)" so they
threshold at the "80/20 Pareto Principle" — but the power-law claim is
asserted, not fitted, so the threshold is best treated as a tunable
quantile, not a law. Optional post-processing: natural-neighbour
interpolation of traversed points, or inverting density into a
second-generation cost surface for site-to-site "most probable
paths."

Cost model: 8-neighbour ("Queen's Case") anisotropic graph, two
directed costs per edge; Tobler (1993) velocity (with the explicit
warning that supplying degree or percent slope "is a common mistake");
Pandolf et al. (1977) metabolic rate with the Santee et al. (2001)
downhill subtraction; IGBP landcover terrain coefficients; water
edges "flagged as being off limits to travel." Baseline traveler
75 kg + 7 kg load. Compute: 2.5 h on 12 cores (2012, custom C++) for
2,000 cost surfaces / 4,000,000 paths.

Robustness (their §4.1): "the most frequently traveled routes changed
little when the sampling interval changed or the weight of the load
was adjusted"; coarser lattices produce fewer routes but the same
high-traffic ones. Validation: documented precolumbian corridors
overlap the high-traffic zones; major towns lay within 5 km of
high-traffic path intersections. **Undocumented anywhere: boundary
bias** — cells near the study-area edge participate in fewer pairs, so
density is suppressed near borders; if adopted, this needs an
in-house QA treatment (crop halo), not a gate.

Implementations corroborating the procedure: R
`leastcostpath::create_FETE_lcps`; QGIS *Itinera* plugin (README
read): "LCPs between all n(n−1) directed ordered pairs (both
directions, for anisotropy), traversal frequency per cell." Follow-up
White (2015), *Advances in Archaeological Practice* 3(4): 407–414
[abstract] is a tutorial, not a refinement. Related: Frachetti,
Smith, Traub & Williams (2017), *Nature* 543: 193–198 [abstract +
extended-data captions] — flow accumulation from 5,000
spatially-balanced random settlements, summed over 500 iterations; a
randomized cousin (deterministic only if seeded).

Determinism: exact given a fixed tie-breaking rule for equal-cost
paths (the papers are silent on ties; an implementation must fix
lexicographic tie-breaks). Counting is integer, so accumulation
order cannot matter.

### MADO / focal mobility networks [primary for the origin paper]

Fábrega-Álvarez, P. & Parcero-Oubiña, C. (2007), "Proposals for an
archaeological analysis of pathways and movement", *Archeologia e
Calcolatori* 18: 121–140 [primary, open PDF]. MADO ("optimal
accumulation model of movement from a given origin"), verbatim: "the
representation of an accumulation model of lowest cost movement
calculated from a given origin and **without specific destination
points**." Hydrology machinery (flow direction + accumulation) run on
an accumulated cost surface; drainage-like ridges are the likely
routes from that origin. Their empirical headline: river **crossing
points** act as network nodes on a par with towns. Rivers handled as
exclusion masks so paths don't run along riverbeds. The extension
paper Llobera, Fábrega-Álvarez & Parcero-Oubiña (2011), *JAS* 38(4):
843–851, doi:10.1016/j.jas.2010.11.006, is verified at
**abstract/secondary level only** (paywalled; CSIC deposit
restricted): it names the "focal mobility network" and demonstrates
it on synthetic surfaces and Galician hillforts. Herzog (A&C 25,
primary) endorses the method where origins are known and destinations
are not, and notes the drainage readout of an ACS "may become trapped
in localised plateaux or pits" — back-link retracing from actual
Dijkstra runs is mandatory; naive steepest-descent on the ACS provably
misses the LCP (her Fig. 1 counterexample).

Endpoint status: half-honest — destinations eliminated, origins
required. Origins that are themselves terrain-derived (affordance
sites) keep it honest. Deterministic; cheap (one Dijkstra + one
accumulation per origin).

### Circuit theory — McRae et al. 2008 [primary]

McRae, B.H., Dickson, B.G., Keitt, T.H. & Shah, V.B. (2008), *Ecology*
89(10): 2712–2724, doi:10.1890/07-1861.1. Full PDF read. Current
density = expected *net* traversals of a random walker between a
source/ground pair; gives corridor **breadth and redundancy** that LCP
density cannot ("an ability to evaluate contributions of multiple
dispersal pathways"; redundancy = least-cost distance / resistance
distance). Two structural restrictions for via: **resistors are
isotropic** — "the methods described here cannot accommodate movement
that is biased in one direction" — so slope-anisotropic walking costs
must be symmetrized; and focal node pairs are required. Compute is a
sparse SPD solve per pair; Circuitscape.jl demonstrates 437M cells.
Role if adopted: an audit instrument on top of a skeleton, not the
skeleton.

### Helbing active walkers [primary for both papers]

Helbing, Keltsch & Molnár (1997), *Nature* 388: 47–50 (preprint
cond-mat/9805158 read); Helbing, Schweitzer, Keltsch & Molnár (1997),
*Phys. Rev. E* 56(3): 2527–2539 (preprint cond-mat/9806097 read).
Ground-potential reinforcement + trail-potential attraction; two
dimensionless parameters κ = IT/σ² (trail attractiveness) and
λ = V⁰T/σ; small κ → direct-way system, large κ → minimal way system.
A **deterministic macroscopic solve is published** (PRE §IV.B:
continuity equation per (entry, destination) subpopulation, stationary
state by self-consistent-field iteration, "the results agree with the
ones of the related microsimulations") — but it needs one density
field per origin–destination pair, and **no calibrated values of κ, λ
are published**. The only mechanism in this family producing emergent
off-endpoint junctions and bundling; a spike candidate, not a
workhorse. The doctrine screen's note stands: valid only for the
pre-engineering regime, with the handover a declared forcing switch.

### Grid geometry corrections — Herzog [primary]

Herzog, I. (2014), "Least-cost Paths — Some Methodological Issues",
*Internet Archaeology* 36, doi:10.11141/ia.36.5 [primary, full HTML];
Herzog, I. (2014), *Archeologia e Calcolatori* 25: 223–239 [primary].
Binding numbers for any raster corridor code (her §3.1, quoting
Huber & Church 1985 [secondary via Herzog] and Herzog 2013b):
8-neighbour worst-case **path-length elongation ≈ 8%**, reduced to
2.8% with Knight's moves and 1.4% with A/B-moves (3-1 and 3-2 steps);
worst-case **distance between the optimal route and the computed LCP =
20% of path length** for Queen's moves only, **11% with Knight's
moves, 4.6% with A/B-moves** — "only LCP software supporting Knight's
moves or even A- and B-moves is appropriate for LCP calculations
aiming to reconstruct routes." Long moves must be subdivided
(Knight's into 2, A/B into 3) so they pay interpolated intermediate
costs and cannot skip barriers. Further rules inherited by any
adopter: costs strictly positive (Dijkstra requirement, and negative
regions attract detours); "the only transformation of a cost function
that does not alter the LCP outcome is multiplication by a positive
constant" (classified/ordinal cost layers change the network);
anisotropic costs may be symmetrized by averaging both directions
when a symmetric cost-distance is needed; Tobler returns a **speed**
and must be inverted to time (Surface-Evans's 4.8 km/h average
"indicates that the Tobler function was not applied correctly");
"Focusing only on slope, LCPs will often run in riverbeds. It might
be necessary to model rivers as barriers and to identify the fords"
— which is exactly the piercing role of via's ford/crossability
spectrum. For wheeled vehicles her quadratic critical-slope function
**Cost(s) = 1 + (s/ŝ)²** (after Llobera & Sluckin 2007), with ŝ =
8–15% working well for historical cart-road reconstruction, is the
citable wagon-era form (corroborating 0013's wheeled-regime note and
0010's turnpike ~5%).

## Sequential network growth (site-anchored trunks)

### Stahlberg et al. 2023 — sequential LCP with reuse discount [primary]

Stahlberg, M.J., Sagnol, G., Ducke, B. & Klimm, M. (2023),
"Spatiotemporal reconstruction of ancient road networks through
sequential cost–benefit analysis", *PNAS Nexus* 2(2): pgac313,
doi:10.1093/pnasnexus/pgac313 [full text read via PMC; the doctrine
screen's citation is confirmed as spelled]. The model, verbatim:
edges have cost c_e; "initial construction costs are (1 − α)c_e and
… ongoing travel costs amount to α·c_e, where α ∈ [0, 1]"; to
establish a connection {u, v} from the connection set K in order π,
"compute a least cost u–v-path P in G, add all its nonroad edges … to
the set of road segments R" and "multiply the cost of edges that were
newly added to R by α, making them cheaper in subsequent iterations."
**The reuse discount is exactly a multiplicative α on built edges.**
K is formally an arbitrary input (their application: all pairs among
21 Roman Sardinian sites); the order π is a free input they *infer*
from milestone evidence via an estimation-of-distribution search
(NP-hard; no PTAS for α < 1 unless P=NP). Cost function: Tobler
friction on a 50 m DEM. Fitted α ∈ [0.4, 0.5] for Roman Sardinia
(scanned [0.3, 0.7] in 0.05 steps) — a **single-region fit; portability
unknown**. Deterministic given (α, π, cost surface) up to LCP ties —
the paper gives no tie-breaking rule; an implementation must fix one.

### Ordering without demand — Molinero & Hernando 2020 [primary, arXiv-only]

Molinero, C. & Hernando, A. (2020), "A model for the generation of
road networks", arXiv:2001.08180 [algorithm section read; **never
journal-published** — cite as preprint]. Same mechanism as Stahlberg
(α = on-road/off-road speed ratio; shortest paths with weights α·l on
built edges; built edges tagged) with the order fixed deterministically:
"Order all the pairs of vertices by the number of trips that there is
going to exist between them (N_ij), in decreasing order" — gravity
demand ordering. With populations unavailable (uniform masses),
gravity N_ij ∝ 1/d^a makes decreasing-demand order coincide with
**ascending-distance order** — a deterministic, settlement-free
ordering rule.

### Pruning the pair set — proximity graphs [secondary + abstract]

Definitions (standard computational geometry; cross-checked secondary):
Gabriel graph (Gabriel & Sokal 1969, *Systematic Zoology* 18(3):
259–278): edge (p,q) iff the closed disk with diameter pq is empty —
d²(p,q) < d²(p,r) + d²(q,r) ∀r. Relative neighbourhood graph
(Toussaint 1980, *Pattern Recognition* 12(4): 261–268): edge iff no r
has max(d(p,r), d(q,r)) < d(p,q). β-skeleton (Kirkpatrick & Radke
1985): family with β=1 → GG, β=2 → RNG. Nesting NNG ⊆ EMST ⊆ RNG ⊆
GG ⊆ Delaunay. Archaeology precedents: proximal point analysis
(Terrell 1977; Broodbank 2000, k=3 Cyclades); Jiménez & Chapman
(2002), RNG in archaeological spatial analysis. The empirical ranking
that matters: **Groenhuijzen & Verhagen (2017)**, "Comparing network
construction techniques in the context of local transport networks in
the Dutch part of the Roman limes", *JAS: Reports* 15: 235–251,
doi:10.1016/j.jasrep.2017.07.024 [abstract] — comparison run on
**cost-distances, not Euclidean**; "the Gabriel graph and proximal
point networks with a high number of neighbours proved to be the best
representation … with the Gabriel graph being slightly better due to
a smaller number of links needed"; Delaunay rejected for "unrealistic
long links." Caution from Herzog (A&C 25, reporting White's Papaguería
n-nearest variant): nearest-neighbour graphs can leave "several
unconnected components" — a connectivity check is required after any
pruning.

### Later-stage growth — Louf, Jensen & Barthélemy 2013 [primary]

*PNAS* 110(22): 8824–8829, doi:10.1073/pnas.1222441110 [full preprint
read]. R_ij = B_ij − C_ij argmax growth; effective budget R′ =
k M_i M_j / d^{a−1} − β d; single effective parameter β = κ/η;
β ≪ β* star, β ≫ β* MST (formally Prim's algorithm), intermediate →
hierarchy of hubs (crossover, not transition); validated against 8
national rail systems (β/β* ∈ [0.20, 1.56]). **Population-dependent
and Euclidean — confirmed post-settlement mechanism** (corridor
class/width upgrading once settlements with sizes exist), not initial
formation. The Barthélemy (2011) *Physics Reports* 499 growth-model
family (FKP, Gastner–Newman, Barthélemy–Flammini/Runions venation,
Courtat) all need center *locations* only, but all are Euclidean —
nothing in that family grows on a heterogeneous terrain-cost surface;
that gap is what Stahlberg and the archaeology fill. ("Runge" in
earlier project notes is a garbled memory of **Runions** leaf-venation;
no such road model exists — negative finding.)

### The archaeological taxonomy — Verhagen et al. 2019 [primary]

Verhagen, P., Nuninger, L. & Groenhuijzen, M.R. (2019), "Modelling of
Pathways and Movement Networks in Archaeology", in *Finding the
Limits of the Limes*, Springer, pp. 217–249,
doi:10.1007/978-3-030-04576-0_11 [open access; full chapter read].
Taxonomy: single LCPs; movement potential (Llobera 2000; Mlekuž
potential path fields); cumulated-LCP density (Whitley & Hicks 2003;
Zakšek et al. 2008; White & Barber 2012; Llobera 2015 random-pairs
variant — with Herzog's caveat that "the concentration of paths is
not dependent on the absolute costs involved"); focal mobility
networks (Fábrega-Álvarez 2006; Llobera et al. 2011; Frachetti 2006);
site-graph construction (distance-capped, gravity, PPA, Steiner —
"applying this principle to cost surfaces is very challenging").
Directly licensing via's node choice, §11.4.1 verbatim: "Certain
waypoints will strongly concentrate movement because they provide
easy access through difficult terrain, like fords, bridges and stairs
… we can either treat them as additional nodes to be connected, or
add them as attractors to the cost surfaces using a distance decay
function." Their standing caveat: validation best practice for
generated networks "is still largely lacking."

## Multimodal land+water cost (the pre-modern era)

### ORBIS — Scheidel & Meeks [primary]

Scheidel, W., Meeks, E. & Weiland, J. (2012), *ORBIS: The Stanford
Geospatial Network Model of the Roman World*, v1 paper [primary, pp.
1–36]; Scheidel, W. (2014), "The shape of the Roman world", *JRA* 27:
7–32 [primary via working-paper version]; Scheidel, W. (2013) on the
Edict's maritime charges [primary, working paper]. The flagship
multimodal precedent: 751 sites; modes = fourteen road modes, two
river modes, two sail types, coastal vs open sea, canals; Dijkstra
over a routing table; sea legs from a wind-rose-weighted 0.1° mesh.
Load-bearing numbers, verbatim: road speeds (km/day) ox cart **12**,
porters/heavily-loaded mules **20**, foot/army/pack/camel **30**,
routine horseback 56, relays 250 (ceiling). Civilian river boats
**65 down / 15 up km/day** (time asymmetry ≈ 4.3:1); military oared
120/50; canals 15 both ways (towing). Price ratios from Diocletian's
Price Edict (301 CE): "the envisaged price ratio for moving a given
unit of cargo over a given unit of distance is **1 (sea) to 5
(downriver)/10 (upriver) to 52 (wagon)**" (pack animal ≈ 42); river
prices 1 den./modius per 20 miles down, 2 up. Scheidel's own
defensibility frame: "only price ratios between different modes of
transport are of relevance to our model. All that is required is that
these price ceilings are not both massively and inconsistently
wrong"; simulations carry "a margin of error of up to +/-30 percent."
**Key negative finding: ORBIS v1 prices transshipment at zero** — "a
function that is not provided on this site" (v1 p. 16); users are told
to add port time ad hoc. Mountain passes are flat additive day
penalties (+0.5/1/1.5 days by class, with winter closures for some
vehicle modes) — the precedent for treating passes as node penalties
rather than resolved switchback geometry.

### The price-ratio family [mixed status]

- Masschaele, J. (1993), "Transport costs in medieval England",
  *EcHR* 46(2): 266–279 at 273 — **land : river : sea = 8 : 4 : 1**
  per ton-mile (royal purveyance accounts, 1296–1348). Verified via
  Langdon & Claridge (2011), *History Compass* 9/11: 864–875
  [primary]: "sending goods by land cost twice as much per unit weight
  per mile than sending it by inland waterways … and eight more times
  than sending it by coastal shipping." The **pre-modern within-era
  anchor**.
- Duncan-Jones (1982), *The Economy of the Roman Empire*, p. 368: sea
  : river : land = 1 : 4.9 : 56 [secondary via Wiseman et al. 2024
  snippet; the often-quoted pack-animal "28" is **unconfirmed** —
  treat only 4.9 and 56 as secondary-confirmed]. Deman (1987):
  1 : 5.8 : 39 [secondary, same route].
- Willan (1936/1964), *River Navigation in England 1600–1750*, via
  Bogart's transport-revolution survey [primary]: c. 1700 road ≈ 1.2
  s./ton-mile, old rivers 0.12 ("one-tenth the freight rate by
  road"), improved rivers 0.41; coastal coal 0.019 s./ton-nm → sea :
  river : road ≈ 1 : 6 : 63 (arithmetic ours).
- Empirical counterpoint: Wiseman, Ortman & Bulik (2024), *JAS* 170:
  106059 [abstract/secondary]: late Romano-British pottery
  distributions fit a much flatter road = 3× river, 4× sea — the
  honest uncertainty band on any declared vector is wide.
- Payload per motive unit (Satchell, Campop atlas chapter, Table 2
  after Skempton [primary]): river barge 30 t per horse vs soft-road
  wagon 0.625 t vs pack-horse 0.125 t — per-horse ratio 240 : 5 : 1.

### Small-craft speeds and the only citable switch penalty [primary]

Livingood, P. (2012), "No Crows Made Mounds", in *Least Cost Analysis
of Social Landscapes* (White & Surface-Evans eds.), U. Utah Press,
pp. 174–187 [primary, author PDF]. Foot = Tobler unmodified. Canoe =
"**4 km/hr for the base speed** of canoe travel plus or minus the
speed of the current," admissible on waterways ≥ 100 cfs; "probably
any value between 3.5 and 5 km/hr could be defended, with a range
between 4 and 4.5 km/hr as the most likely." Upstream ≈ 2× downstream
time (Little 1987: 59); historic corroboration 16–45 km/day up,
45–110 down (Champlain, Marquette/Joliet et al.). **The only
quantified land↔water interface penalty found in the sweep**: his
Table 10.1 water-crossing delays banded by flow — <10 cfs: 0; 10–100:
3 s; 100–1,000: 5 min; 1,000–10,000: 10 min; >10,000: 30 min — with
**half the banded value charged per land↔water switch** (embark or
disembark), i.e. up to 15 min on the largest rivers. Portage
explicitly not modeled (no waterfall data). His result — water legs
almost never chosen at near-parity speeds (2% of distance) — is
itself a datum: mode share is decided by the declared speeds/costs.

Adjacent negative on the era-0 depth anchor: Appel et al. (2024),
*E&G Quaternary Science Journal* 73: 179–202 [primary page fetched]
contains **no speed, cost, or LCP content** — only the 0.3–0.7 m
draught-sufficiency claim citing Eckoldt (1985). The Eckoldt
citation-chain flag from 0013 stands.

### Multimodal negatives (documented absences)

1. **No citable boat-propulsion energy cost in J/kg/m** commensurable
   with walking metabolic cost. Closest: di Prampero (1986), *Int J
   Sports Med* 7(2): 55–72 [secondary] — rowing ≈ 0.22 kJ/m for a
   modern athlete in racing craft, no cargo-mass normalization, no
   pre-modern hull. The calorie-currency LCP literature (Hare 2004;
   Wood & Wood 2006; etc.) never extends onto water. **Land and water
   cannot share an energy currency on citable grounds**; a joint
   currency must be time or a declared price vector.
2. **No citable porter(pre-wheel) : boat cost ratio.** Ames (2002)
   paywalled; Livingood implies speed near-parity but encodes no
   payload economics; Skempton's ×240 payload ratio is horse-era.
3. **Transshipment costs are a literature gap until 2026**: ORBIS
   prices them at zero; Livingood's table is the only quantified
   precedent; Page, J. (2026), "Lost in Transit: Reconstructing
   Transhipment Costs from the Roman Era", *J. Maritime Archaeology*
   21(1) [abstract] confirms the gap and proposes methods — numbers
   paywalled. Filet & Rossi (2025), *JAS* (S0305440325000263)
   [abstract; HAL preprint hal-03911362 bot-walled] is the closest
   current methodological precedent (wagon + river boat LCP with
   up/downstream asymmetry and uncertain navigability) — its
   calibrated ratios are the top retrieval target.

## A note on recomputing the critical gradients (via, 2026-08-22)

0013 records that the Llobera & Sluckin critical gradients (+0.28 /
−0.22 m/m) solve their Eq. 20, M(s) − s·M′(s) = 0, on the Minetti
et al. (1995) quartic, and that an adopting ADR "may recompute s_crit
from whichever cost function it adopts (Eq. 20 is cost-function-
generic)." Via ran that recomputation (mpmath, 30-digit precision,
bracketed bisection). On the quartic the criterion reproduces the
published values (+0.2788 / −0.2209 — the figure-level precision).
On the **Herzog IA36 sextic the criterion is multi-rooted on the
downhill limb**: sign changes at −0.4136, −0.3316, −0.1392 (and
+0.3814 uphill) — the high-degree fit's oscillation, not physiology.
Consequence for adoption: recomputing the envelope from the sextic
would require an arbitrary root-selection rule; the defensible
position is to freeze L&S's own published constants (computed on
their own cost function) and declare the envelope/cost-polynomial
pairing as via assembly.

## Open gaps

- Filet & Rossi (2025) calibrated land:river ratios (paywalled/HAL
  bot-walled) — top retrieval target; Page (2026) transshipment
  numbers — second.
- Duncan-Jones's pack-animal "28" unverified; needs 1982 ed. p. 368.
- Llobera et al. (2011) full text (exact ACS flow-direction variant);
  Huber & Church (1985) originals; Helbing κ, λ calibrations (none
  published); White (2015) full text.
- Howey (2007) and Gustas & Supernant (2017): method genus citable,
  parameter values unrecoverable without full texts.
- FETE boundary bias and lattice-phase sensitivity: unanalyzed in the
  literature; needs an in-house QA diagnostic if adopted.
- Stahlberg's α portability beyond Roman Sardinia unknown; order
  sensitivity quantifiable only by ensemble over orders.
- No published intrinsic node weights for affordance sites (0013's
  negative finding stands); any demand proxy is declared config.

## What the adopting ADR must decide

1. The network representation: density spectrum (FETE-family),
   site-anchored trunk graph (Stahlberg-family), or both; and the
   endpoint-honesty declaration for each.
2. The common currency for the multimodal solve (time is the only
   citable cross-mode currency; energy is land-only; price vectors
   are within-era relative).
3. The mode set for era 0 and each mode's cited speed/cost, with the
   declared-forcing items flagged (coastal speed, transshipment
   penalty, any demand proxy).
4. Grid neighbourhood and tie-breaking (Herzog's corrections; exact
   determinism).
5. The trunk pair set (pruning graph), insertion order, and α.
6. Which checks are gates (artifact-contract identities) vs QA
   diagnostics (boundary halo, energy-vs-time divergence).
