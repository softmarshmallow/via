# Research 0012 — Benchmark specification: settlement systems and urban fabric

Status: **adopted as the project's validation instrument by ADR 0010
(2026-08-20)**, with three deferrals named there: the §9.4 scale
mismatch (resolved at protocol freeze, together with the radius),
acceptance thresholds (none declared), and the disc radius and
adopted character list (fixed at protocol freeze). ADR 0010
Decision 2 also records one exception to §5.4: the pilot towns —
five measured before the rule existed, one designated by committed
addendum before fetch — are pinned to the fitted partition. This document specifies an *instrument*, not a result,
and adopts no mechanism. It is written to
satisfy ADR 0008 (validation doctrine), which requires that
characters come from citable sources, that references be populations,
and that protocols be frozen and restated with every number.

Nothing here has been run. Every band, count and definition below is
quoted from the source named beside it.

## 1. The claim under test

A generative model of inhabited landscape can be tested against
exactly one honest claim:

> Under a stated protocol, at a stated scale, the generated fabric is
> **not distinguishable** from measurements of real fabric of the same
> class, on a declared set of standard morphometric characters.

Three things this claim is not:

- It is not "looks real". That is unfalsifiable.
- It is not a single realism score. None exists (§7).
- It is not a claim about mechanism. Matching a distribution is
  compatible with the wrong mechanism (ADR 0008 D9), which is why
  null models are part of the instrument (§6).

## 2. Two tiers, because the units and the data differ

| | Tier A — settlement system | Tier B — urban fabric |
| --- | --- | --- |
| Unit | a settlement; a corridor | a street; a block; a plot; a building |
| Extent | a region (10²–10⁴ km²) | a neighbourhood (10⁻¹–10⁰ km²) |
| Question | how many, how big, how spaced, how connected | how the ground is divided and built |
| Reference | GHS Urban Centre Database | OSM-derived street/building measurements; Boeing's global indicators |
| Literature | spatial economics, quantitative geography | urban morphology, morphometrics |

The tiers are separated because a character that is meaningful at one
scale is meaningless at the other, and because their reference data
have different provenance, different completeness and different
failure modes.

## 3. Protocol (frozen; restated with every number)

- **P1 — Extent.** Tier B statistics describe a disc of stated radius
  about a stated centre. The spike used r = 300 m; the radius is a
  parameter of the protocol, not a constant, and must be quoted.
- **P2 — Buffer.** Elements are loaded to 1.45 × r and participate in
  the graph; statistics are computed only over elements whose geometry
  lies inside r. Without this, every street crossing the boundary
  becomes a false dead end and every route through the boundary
  disappears (measured effect: dead-end share 41.7% → 32.7% at
  Alnwick).
- **P3 — Simplification.** Topological characters are computed on the
  graph with degree-2 vertices dissolved: nodes are junctions and dead
  ends, edges are whole streets. This is the graph every published
  street statistic refers to. (Measured effect of getting this wrong:
  Alnwick meshedness 0.018 instead of 0.057.)
- **P4 — Element sets.** Two street sets are measured and reported
  separately, because the difference is large and because a generated
  network must be compared against the set that corresponds to it:
  - *carriageway set*: OSM `highway` in {motorway, trunk, primary,
    secondary, tertiary, unclassified, residential, living_street,
    service, pedestrian} and their `_link` variants, excluding
    `service=driveway|parking_aisle`;
  - *all-ways set*: the above plus footway, path, steps, cycleway.
  Measured difference at Alnwick: meshedness 0.057 vs 0.138; dead-end
  share 0.37 vs 0.25; mapped carriageways are 60% of all-ways length
  (4.04 km of 6.76 km).
- **P5 — Projection.** Local equirectangular about the study centre;
  distortion over a 1 km extent is far below the resolution of any
  input used here.
- **P6 — Provenance.** Every result carries: protocol identifier, code
  revision, OSM extract date, GHSL/Boeing dataset release, seed set.

## 4. Tier A — settlement system

### 4.1 Reference population

**GHS Urban Centre Database (GHS-UCDB)**, Joint Research Centre,
European Commission. Release R2019A characterised >10,000 urban
centres with 28 variables across five domains for epochs 1975, 1990,
2000, 2015; release R2024A reports 11,422 quality-controlled urban
centres across 15 thematic domains, 471 indicators and 2,600
attributes. Urban centres are delineated by cut-offs on resident
population and built-up surface share in a 1 × 1 km global grid (the
"degree of urbanisation" method), which is a *published, fixed
delineation* — the property ADR 0008 D4 requires and that
docs/research/humanity/0011 recorded as unresolved.

### 4.2 Characters

| Character | Definition / source |
| --- | --- |
| Settlement size distribution, upper tail | Rank-size exponent ζ estimated with the rank − ½ correction (Gabaix & Ibragimov 2011), fitted over a declared size threshold |
| Settled area vs population | A ~ N^a; Ortman et al. (2014) report a ∈ [2/3, 5/6] and find it era-invariant from prehispanic to modern settlement |
| Nearest-neighbour spacing | Distribution of distances to the nearest same-tier centre; summarised by median and IQR, compared as a distribution, not as a mean |
| Spatial regularity | Clark & Evans (1954) R, edge-corrected. Advisory only: hexagonal regularity is essentially never observed (Dacey 1962) and the 2.15 ceiling is contested |
| Primacy | Largest centre's share of system population |

### 4.3 Known limits

GHS-UCDB is contemporary and its unit is the *urban centre*, which is
larger than a pre-modern town. Tier A therefore benchmarks the
**structure of a settlement system** (how sizes and spacings are
distributed) and not the absolute sizes of historical towns; for those,
historical demography (Bairoch, Batou & Chèvre 1988) is the only
source and it is a compilation, not a survey.

## 5. Tier B — urban fabric

### 5.1 Reference population A: Boeing's global street-network indicators

Boeing (2021) publishes street-network models **and computed
indicators** for 8,914 urban areas across 178 countries (>160 M nodes,
>320 M edges), built from OpenStreetMap with OSMnx and deposited at the
Harvard Dataverse, joined to GHS urban-centre attributes. This gives a
reference *population* with standardised definitions — the single most
important asset for this benchmark, because it removes both the
"one reference town" problem and the "invented definition" problem at
once.

Indicator names and definitions are those of the published dataset,
quoted verbatim from its metadata generator:

| Indicator | Definition (verbatim) |
| --- | --- |
| `circuity` | "Ratio of street lengths to straightline distances" |
| `straightness` | "1 / circuity" |
| `k_avg` | "Average node degree (undirected)" |
| `prop_4way` | "Proportion of nodes that represent 4-way street intersections" |
| `prop_3way` | "Proportion of nodes that represent 3-way street intersections" |
| `prop_deadend` | "Proportion of nodes that represent dead-ends" |
| `intersect_count_clean` | "Count of street intersections (merged within 10 meters geometrically)" |
| `length_mean` / `length_median` | "Mean/Median street segment length (undirected edges), meters" |
| `length_total` | "Total street length (undirected edges), meters" |
| `street_segment_count` | "Count of streets (undirected edges)" |
| `orientation_entropy` | "Entropy of street network bearings" |
| `bc_gini` | "Gini coefficient of normalized distance-weighted node betweenness centralities" |
| `bc_max` | "Max normalized distance-weighted node betweenness centralities" |
| `cc_avg_undir` | "Average clustering coefficient (unweighted/undirected)" |
| `self_loop_proportion` | "Proportion of edges that are self-loops" |
| `grade_mean` / `grade_median` | "Mean/Median absolute street grade (incline)" |
| `built_up_area_percap` | "Built-up surface area per-capita, square meters per person (GHS)" |

Two notes carrying consequences for this project:

- `bc_gini` is a published, globally computed measure of how
  concentrated through-movement is. It **replaces** the "backbone
  concentration" statistic invented in the spike (ADR 0008 D2).
- `orientation_order` (φ), defined in Boeing (2019) as a linearised,
  normalised measure of how far a network follows a single grid's
  ordering logic, is reported in later releases and is the correct
  character for the "planned versus organic" axis. Boeing (2019) also
  warns it is a spectrum, not a classifier.

### 5.2 Reference population B: morphometric characters

Boeing's indicators cover the street network only. For blocks, plots
and buildings the standard is the numerical taxonomy of urban form
(Fleischmann et al. 2022): **74 primary characters plus 296 contextual
characters (74 × 4) = 370**, computed with the open toolkit `momepy`
over six categories — dimension, shape, spatial distribution,
intensity, connectivity and diversity — on three morphological
elements (building, street, plot) using *morphological tessellation*,
a Voronoi-derived partition generated from building footprints.

This matters beyond convenience: the tessellation gives a plot-like
partition **even where cadastral data does not exist**, which is the
condition of every reference town we can obtain. A generated town has
true plots; a real town usually does not; tessellation puts both on
the same footing.

Adoption rule: characters are taken from this set by name. Where the
project needs a property the set does not cover, ADR 0008 D2 applies —
define it, justify it, and label it an invention.

### 5.3 The era problem, and why surviving plans are admissible evidence

No medieval town can be observed. What can be observed is the
surviving plan of a medieval town, recorded today. Using it as
evidence about historic fabric is licensed by the same claim the
project's mechanism rests on: Conzen's (1960) persistence hierarchy —
street systems outlive plot patterns, which outlive buildings — and
its empirical support (Strano et al. 2012 measure ~90% persistence of
the highest-betweenness routes in a region over ~170 years).

The consequences must be stated with every era-specific number:

- Buildings observed today are largely **not** the historic buildings;
  building-scale characters from a medieval reference describe a
  modern building stock on a historic plan.
- The plan itself has been modified — widenings, clearances,
  insertions.
- Preserved towns are **survivors**, selected by history and often by
  conservation policy; they are not a random sample of medieval towns.

Therefore: street and plot characters from historic cores are used as
evidence; building characters from them are used as context only.

### 5.4 Sampling design

- **Class definitions first, cases second.** Each reference class
  (organic pre-modern core; planted pre-modern grid; 19th-century
  survey plat; mid-20th-century suburb; contemporary informal) is
  defined by documented origin, not by appearance.
- **Pre-registration.** The list of reference towns and their study
  centres is written down *before* any of them is measured, so that
  cases cannot be selected after seeing which ones flatter the model.
  *(One recorded exception: the pilot towns of ADR 0010 Decision 2 —
  five measured before this rule existed, one designated by committed
  addendum before fetch — all pinned to the fitted partition.)*
- **Size.** A minimum of 20 towns per class for distributional
  characters; fewer than 10 makes dispersion meaningless. The spike
  used one per class, which is the principal reason its conclusions
  were unsafe.
- **Held-out split.** At least 30% of each class is reserved for
  validation and never used for fitting (ADR 0008 D6).

## 6. Statistical procedure

- **Distributional characters** (segment lengths, block areas,
  footprint areas, plot frontages, spacings): two-sample
  Kolmogorov–Smirnov distance between the pooled generated ensemble and
  the pooled reference class, reported as D with its p-value, plus the
  quantile curves. KS is chosen because it is distribution-free and
  standard; it is sensitive to the middle of a distribution and weak in
  tails, so heavy-tailed characters additionally report the tail
  exponent with its own estimator and confidence interval.
- **Scalar characters** (meshedness, k_avg, prop_deadend, circuity,
  orientation_entropy, bc_gini): the generated ensemble's position in
  the reference class distribution, reported as a percentile with the
  reference's median and interquartile range. Never a bare difference.
- **Ensembles.** ≥5 seeds, reported as mean ± sd; the model's own
  dispersion is compared against the reference class's dispersion,
  since a model that is too consistent is as wrong as one that is too
  variable.
- **Multiple comparisons.** With dozens of characters, some will
  differ by chance; per-character p-values are reported with a stated
  correction (Benjamini–Hochberg) and no character is promoted to a
  headline after the fact.
- **No composite score** (ADR 0008 D7).

## 7. What no source provides

There is no accepted benchmark for the realism of a generated town.
The published work addressing exactly this problem — Shaw et al.
(2023), on determining the realism of procedurally generated city road
networks — proposes entropy and orientation-order measured against
real cities as *"a reasonable baseline for future projects"*, and
survey work in the field states that quantifying the realism of a
simulated city with standardised metrics remains an open challenge. Any acceptance threshold this
project adopts is therefore a **declared convention** (ADR 0008 D8),
and this document proposes none.

## 8. Null models (required by ADR 0008 D9)

Run and reported beside the model, on the same protocol:

1. **Random planar graph** at matched node count and density.
2. **Regular grid** at matched intersection density.
3. **Correlated percolation / DLA** growth, which reproduces urban
   statistics without any human mechanism.

A character on which the model and all three nulls are
indistinguishable carries no evidential weight and is reported as
such.

## 9. Threats to validity

1. **OSM completeness varies by geography and feature.** Building
   footprints are near-complete in some countries and sparse in
   others; a residential disc in Abilene, Kansas returned 7 footprints
   under this protocol. Building characters must not be computed where
   coverage is unverified.
2. **Survivorship** (§5.3).
3. **Protocol sensitivity** — demonstrated, not hypothetical (§3, P3
   and P4).
4. **Scale mismatch.** Boeing's indicators are computed over whole
   urban areas; a 300 m disc is not the same object. Either the
   protocol is matched by recomputing indicators from the published
   *models* at our scale, or the comparison is restricted to
   scale-robust characters. This must be resolved before Tier B uses
   the global population, and it is the largest open item in this
   specification.
5. **Class contamination.** A "medieval" core carries later fabric; a
   plat town carries modern infill. Classes are approximate by nature.
6. **Circularity.** A character used to fit parameters cannot also
   evidence the model (ADR 0008 D6).

## 10. References

- Boeing, G. (2019). Urban spatial order: street network orientation,
  configuration, and entropy. *Applied Network Science* 4, 67.
- Boeing, G. (2020). A multi-scale analysis of 27,000 urban street
  networks. *Environment and Planning B* 47(4), 590–608.
- Boeing, G. (2021). Street Network Models and Indicators for Every
  Urban Area in the World. *Geographical Analysis* 54(3), 519–535.
  Data: Harvard Dataverse, "Global Urban Street Networks".
- Fleischmann, M. (2019). momepy: Urban Morphology Measuring Toolkit.
  *Journal of Open Source Software* 4(43), 1807.
- Fleischmann, M., Feliciotti, A., Romice, O., Porta, S. (2022).
  Methodological foundation of a numerical taxonomy of urban form.
  *Environment and Planning B* 49(4), 1283–1299.
- Florczyk, A. J. et al. (2019). *GHS Urban Centre Database 2015*,
  R2019A. European Commission, Joint Research Centre. Successor:
  GHS-UCDB R2024A.
- Gabaix, X., Ibragimov, R. (2011). Rank − 1/2: A simple way to improve
  the OLS estimation of tail exponents. *Journal of Business &
  Economic Statistics* 29(1), 24–39.
- Ortman, S. G., Cabaniss, A. H. F., Sturm, J. O., Bettencourt, L. M.
  A. (2014). The pre-history of urban scaling. *PLOS ONE* 9(2),
  e87902.
- Shaw, A., Wünsche, B. C., Yde, J., Vergerakis, P., Chaney, L., Jin,
  Y., Sarwate, N. (2023). Determining Realism of Procedurally
  Generated City Road Networks. In *Image and Vision Computing*
  (IVCNZ 2022), LNCS 13836, 272–287. Springer.
- Clark, P. J., Evans, F. C. (1954). Distance to nearest neighbor as a
  measure of spatial relationships in populations. *Ecology* 35(4).
- Conzen, M. R. G. (1960). *Alnwick, Northumberland*. IBG Publication 27.
- Strano, E., Nicosia, V., Latora, V., Porta, S., Barthélemy, M.
  (2012). Elementary processes governing the evolution of road
  networks. *Scientific Reports* 2, 296.
- Cardillo, A., Scellato, S., Latora, V., Porta, S. (2006). Structural
  properties of planar graphs of urban street patterns. *Physical
  Review E* 73, 066107.
- Grimm, V. et al. (2005). Pattern-oriented modeling. *Science*
  310(5750), 987–991.
