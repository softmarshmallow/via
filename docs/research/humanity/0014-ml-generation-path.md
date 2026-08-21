# 0014 — The ML path: learned generation of settlement fabric

Status: **commissioned dossier — pre-decision** (per this directory's
README: adopting any mechanism here requires its own ADR with its own
review). Commissioned 2026-08-21. Companion, not successor, to the
simulation path: the project thesis holds that macro-level simulation
is the way to real-world-quality output, and this dossier plans the
parallel spike that tests the alternative — a custom-trained
generative model — so that a verdict on either path is earned, not
assumed. IMPOSSIBLE remains an acceptable verdict for this path
exactly as for the other.

Method: four verified literature sweeps (street-network generation;
settlement/building generation and attribute prediction; training
data and licensing; compute precedents and novelty), 84 documented
search queries, every retained citation verified against its primary
page (arXiv/DOI/publisher/official repo; three papers read in full);
negatives recorded as first-class results (§9). Repo facts cite the
benchmark chain (ADR 0010, `reference/protocol-v1.1.md`,
`reference/benchmark-report-v0.md`). Planning only — no code, no
training runs.

---

## 1. Commission

The question, as put: define the input, the output, the format and
modality, the available training data, the candidate architectures,
the relevant prior work, and an optimistic schedule with a computing
budget, for a custom-trained ML model that generates OSM-quality
street and settlement data — streets, settlements, and the macro
metadata (building levels, inferring economics), with terrain.

What makes this spike unusual: the evaluation instrument already
exists and is generator-agnostic. via-bench + protocol v1.1 +
`reference/populations-v1.1.json` judge a generated town by the same
battery as a real one, with a sealed 44-town held-out partition for
a final validation event (mechanical split in
`reference/pre-registration.md`; held-out policy stated in
`reference/benchmark-report-v0.md`, under ADR 0010's
pre-registration doctrine). Most generative-model papers invent
their own evaluation — the sweeps found **no published generator
evaluated against a settlement-morphology benchmark battery** (the
field evaluates with FID, overlap/validity indices, Wasserstein
distances on low-dimensional geometry, occasionally urban-planning
or space-syntax statistics — §4.1; none compares against fitted
per-class populations over null-model floors). via froze its
instrument before any generator existed. The spike therefore
reduces to a falsifiable question: can a trained model land inside
the fitted per-class character populations that no null model
reaches? (Benchmark report v0's D9 tables — grid, matched random
planar, DLA: "no null reproduces any class across the battery"; a
generator matching a class on one character "has, by these tables,
matched nothing a null could not".)

## 2. Problem definition

### 2.1 The task

Learn the conditional distribution

    p(fabric | site, controls)

where **site** is a terrain/hydrology/coast raster stack over a
tile, **controls** are discrete macro labels (settlement class,
region token, size target), and **fabric** is:

- **streets** — a planar-embedded graph: nodes with coordinates,
  edges with polyline geometry and a class label (the operative-set
  vocabulary of protocol v1.1 P4);
- **buildings** — closed footprint polygons, each with macro
  metadata: `building:levels` (integer), use class;
- blocks are implicit (faces of the street graph, per protocol v1.1
  Elements); plot tessellation is deferred with the benchmark's own
  momepy gap.

This is generation, not extraction: no imagery of the target exists.
The nearest well-studied problems (road extraction from satellite,
HD-map perception) share output representations but not the task.

### 2.2 Conditioning input

The conditioning channels must be computable from **both**
real-world data (training time) and via's own artifacts (inference
time). That dual-source constraint picks the channels:

| Channel | Training-time source | Inference-time source (via artifact) |
| --- | --- | --- |
| elevation (tile-relative) | Copernicus GLO-30 (§3.5) | `heights_cm.vrast` |
| slope, aspect, hillshade ensemble | derived from DEM | derived |
| river presence / size | OSM `waterway` + width class | `strahler.vrast` (>0), `discharge.vrast` |
| standing water mask | OSM `natural=water`, coastline | `water_depth.vrast` (>0), ocean |
| coast distance | derived from OSM coastline | `coast_dist` export |
| (v1, optional) climate | WorldClim/CHELSA — licence unchecked, open item | `temperature.vrast`, `precip.vrast` |
| (v1, optional) affordances | via-suitability run *on real DEM tiles* | `suitability.<label>.*` |

The hillshade row is not decoration: the one serious
terrain-conditioned deep model (Fang et al. 2022, §4.1) reports —
citing its own earlier attempt (Fang et al. 2020b) — that
elevation+aspect channels alone were insufficient, and its working
configuration combines a four-azimuth hillshade ensemble with
learned per-channel soft gating and staged generation; the gating
and staging, not the extra channels alone, are credited with making
hilly-area generation work. That is a transferable engineering
result, proposed here (for the ADR) as the default conditioning
stack.

Two properties worth stating explicitly:

1. **via-suitability runs on real terrain too.** Nothing in the
   stage requires synthetic input; fords/passes/harbours computed on
   a real DEM+hydrology tile are legitimate conditioning features,
   and using them would inject the simulation path's physics into
   the ML path's conditioning — the two paths are composable, not
   exclusive.
2. **The domain gap is a named risk.** A model trained on real DEM
   tiles conditions on real-terrain statistics; via terrain must be
   in-distribution for the conditioning encoder or transfer fails
   silently. via-terrain realism becomes a measurable precondition
   of the ML path (§8, R4).

Proposed tile geometry, v0: generation window sized to the
benchmark's acquisition window (750 m radius, the protocol's
default *fetch* radius — the instrument itself measures a **300 m
disc** with fabric participating out to 1.45·r = **435 m**,
protocol v1.1 P1/P2), with a wider context window (~4×4 km) for
the terrain encoder, raster resolution 10–30 m. A judged result
requires generating at least the 435 m participation window;
matching the 750 m fetch default keeps the synthetic extract
shaped like a real one end to end. Settlement-system scale (where
towns sit in a region) stays with the simulation path: it is Tier A
of 0012, and the urban-fabric battery does not judge it.

### 2.3 Output format

Two layers, deliberately separated:

- **Interchange format (the contract): OSM-schema JSON**, the same
  shape the Overpass extracts already take (ways + nodes,
  `highway=*`, `building=*`, `building:levels=*`). A generated town
  is written as a synthetic extract, and via-bench measures it
  **through the identical code path** as a real town — same graph
  construction (P3), same street sets (P4), same characters. No
  conversion layer that could flatter the model.
- **Model-native representation (per architecture, §5):** token
  sequences over quantized coordinates, graph tensors, or raster +
  vectorizer. The interchange format is fixed; the native
  representation is the architecture's business.

Coordinate quantization has direct precedent: NTG used discrete
1 m Δx/Δy offset tokens (clamped ±100 m) and PolyGen 8-bit
coordinate bins; a 1 m grid on a 4 km tile is 12 bits per axis, and
the benchmark's own envelopes (segment_len_median ≤ 17% rel)
tolerate metre-scale precision comfortably.

### 2.4 Modality: vector-native, raster, or hybrid

Three families, with the project's own benchmark evidence bearing
directly on the choice:

- **M1 — vector/graph-native generation** (autoregressive tokens or
  graph diffusion). The output *is* the judged object: topology
  (degree shares, meshedness, self-loop proportion), geometry
  (circuity, segment lengths, orientation entropy) and semantics
  (street classes, levels) are generated explicitly.
- **M2 — raster generation + vectorization** (image diffusion over
  semantic maps, then skeletonize/polygonize). Mature tooling, but
  every topological character then measures the *vectorizer*, not
  the model. The project has first-hand evidence of how hard
  representational convention hits the battery: benchmark report v0
  spent its Level-2 investigation on exactly such construction
  artifacts (carriageway conventions moving degree shares by 0.125
  abs; ring canonicalization; quantile conventions moving p90 by
  ~9%). A vectorizer is one more such convention layer, owned by
  neither the model nor the instrument. The raster literature's own
  authors concede the ceiling (§4.1: Kempinska & Murcio's generated
  images "lack the detail of real street images").
- **M3 — learned-field hybrid**: the model predicts continuous
  fields (street orientation/tensor field, density, block-size
  field), a deterministic tracer emits the vectors.
  Guaranteed-valid geometry, small models, data-efficient — but the
  tracer's biases are the procedural biases the ML path exists to
  escape; M3 is half-way back to the simulation thesis (which may
  be a feature: it is the natural *hybrid* if pure generation fails
  on topology).

Position taken by this dossier (for the ADR to ratify or reverse):
**M1 primary, M2 as a cheap control baseline, M3 held as the named
fallback.** The benchmark's most diagnostic characters are
topological; only M1 makes the model own them end to end.

### 2.5 The era problem (honest limit)

OSM records the *present day*. Every real town in any training
corpus is a palimpsest — a medieval core wearing 19th-century
widening and 20th-century infill — and no OSM attribute dates the
fabric. The simulation path generates history and reads any era out
of it; the ML path natively generates **present-day fabric only**,
with era reachable solely through proxies:

- the five corpus classes (organic pre-modern core, planted
  pre-modern grid, 19th-century survey plat, mid-20th-century
  suburb, contemporary informal) are *morphology-regime* labels,
  not dates — but they are exactly the regimes the benchmark
  distinguishes, so class-conditioning is both feasible and judged;
- region/culture tokens capture the geographic style axis;
- true era control (the same town at 1300 vs 1900) has **no
  training signal in OSM**. Historical-map corpora were not
  surveyed in this sweep (open item); none is known to be scaled
  and vector-aligned.

This is a structural asymmetry between the paths, recorded as such:
if era control is a hard requirement, the ML path alone cannot meet
it; a hybrid (simulated growth calibrated or textured by learned
distributions) is the recorded escape.

## 3. Training data

### 3.1 Scale (taginfo live counts, accessed 2026-08-21)

| Item | Count |
| --- | --- |
| `building=*` objects | 705,397,088 (703.2M ways) |
| `highway=*` objects | 300,083,976 (266.2M ways) |
| `place=city` / `town` / `village` | 15,495 / 117,236 / 1,767,272 |
| `building:levels` tagged | 42,232,216 (~6.0% — derived tag-count ratio; the tag also rides on `building:part` objects) |
| `height` tagged (all feature types) | 26,770,013 |

≈1.9M tagged settlements exist; the usable tile pool is 1–2 orders
smaller after completeness gating (ESTIMATE: order 10⁴–10⁵ towns
with fabric complete enough to train on; requiring levels coverage
shrinks it roughly another order and concentrates it
geographically). No authoritative current OSM road-km figure exists
(documented negative); anchors: ~39.7M km estimated true global
car-navigable roads (Barrington-Leigh & Millard-Ball 2017), 86M km
in Overture's broader traversable set (Dec 2024 GA announcement).

External footprint sets: Microsoft Global ML Building Footprints
(~1.4B detections, 174M with ML-estimated heights,
**CDLA-Permissive 2.0**); Google Open Buildings v3 (1.8B
detections, CC BY-4.0 *or* ODbL at user's choice,
geometry+confidence only). Both are label-free geometry — useful
for completeness QA and gap-filling, useless for levels
conditioning. RoBus (arXiv:2407.07835) is an off-the-shelf paired
roads+buildings corpus (72,400 samples, ~80,000 km²) worth a
licence check during the corpus phase.

### 3.2 Completeness heterogeneity (verified)

- **Roads** — Barrington-Leigh & Millard-Ball, PLOS ONE 2017
  (PLOS ONE, not PNAS as sometimes miscited): OSM ~83% complete
  globally for car-navigable roads as of Jan 2016 (95% CI 81–84%),
  >40% of countries fully mapped; completeness U-shaped in
  population density.
- **Building footprints** — Herfort et al., Nature Communications
  14:3985 (2023): of 13,189 urban centres, 1,848 (16% of urban
  population) exceed 80% completeness; 9,163 (48% of urban
  population) sit below 20%; global urban average ≈24%. Over half
  of Sub-Saharan Africa's building data comes from humanitarian
  mapping events (>50% of the region's building *edits* relate to
  organized humanitarian mapping) — footprints-without-attributes
  fabric, a distinct mapping style a model would learn as "regional
  character".
- **Attributes** — Biljecki, Chow & Lee, Building and Environment
  237:110295 (2023; full PDF read by the sweep): 19.5% of buildings
  carry a type, **4.6% `building:levels`**, 2.9% height, 7% either
  (2023 values; the live 2026 taginfo ratio ~6.0% shows slow
  growth); only **443 level-3 admin units worldwide** exceed 80%
  levels completeness (22,710 at district scale). Floors beat
  height because floors are visually discernible to mappers.
- **No published study of generative models inheriting OSM
  completeness bias exists** (documented negative). via's medina
  finding — four organic towns whose operative street set reads
  mapping practice, not town form (benchmark report v0) — appears
  to be ahead of the literature. The adjacent literature treats OSM
  only as noisy labels for supervised extraction.

### 3.3 Overture as substrate

Linux Foundation project, monthly releases (latest 2026-08-19.0).
The themes via would train on — transportation, buildings, base —
are **ODbL exactly like OSM** (buildings theme conflates OSM +
Esri + authoritative sources + Microsoft + Google, ~2.3B footprints
approx., with heights merged from Microsoft's ML estimates).
Verdict: a better *geometry* substrate (documented stable schema,
GERS ids, gap-filled heights), but the *metadata style* signal
(levels, highway-class nuance, mapping culture) lives in raw OSM
tags, and licensing buys nothing. Reasonable hybrid: geometry from
Overture, attribute style from OSM, one pinned release per corpus
state.

### 3.4 Licensing (decision-relevant, verified against OSMF text)

Operative text: the machine-learning section of the **OSMF
Attribution Guidelines** (board-adopted 2021-06-25) plus the OSMF
Licence/Legal FAQ. Verified positions:

1. **Internal training is clean.** ODbL obligations trigger on
   *public use*; a private corpus, model, and internal outputs
   carry no share-alike duty ("If you do not make Public Use of the
   data, then you do not have to share anything with anybody").
2. **Distributing weights**: the settled minimum is attribution
   (README / download page). Whether weights are a Derivative
   Database (share-alike attaches) or a Produced Work (any licence)
   is **unsettled** — the guideline is silent on classification,
   and the sweep found **no precedent of any organization releasing
   OSM-trained weights under any licence** (Meta's Map-With-AI
   weights were never released; Microsoft released road
   *detections* under ODbL, not weights). A publicly *used*
   training dataset is a Derivative Database and must be made
   available under ODbL.
3. **Distributing generated output**: predictions are not
   implicated by ODbL — *unless* output "recreates substantial
   parts" of OSM (the guideline's overtraining caveat, written with
   exactly this edge in mind). Consequence: a **memorization audit
   (nearest-training-tile check on generated output) is a licensing
   control, not just an evaluation nicety.**

For the spike as scoped (internal training, internal evaluation,
results published as measurements and figures): clean today. The
weights question only opens if via ever ships the model.

### 3.5 Terrain pairing

**Copernicus GLO-30** is the default: global 30 m, free for any use
including commercial, attribution-only (licence PDF read in full by
the sweep). One trap, recorded: GLO-30 is a **DSM** — building and
vegetation heights are in the signal, so the conditioning channel
mildly leaks the answer (buildings) into the question (terrain).
Mitigations: smoothing/percentile filtering, and MERIT DEM under
its ODbL option (90 m, bare-earth-corrected) as a cross-check.
**FABDEM is excluded** (CC BY-NC-SA poisons a commercial corpus).
US-only work can use 3DEP (public domain, 10 m).

### 3.6 Corpus rules (repo-grounded)

- The 138-town corpus is an **evaluation** corpus (94 fitted / 44
  sealed held-out, blake3-mechanical split) and must never enter
  training. Exclusion is mechanical: the pre-registered town list
  with a geographic buffer is subtracted from any training corpus;
  the held-out partition stays sealed regardless.
- The fetch pipeline (`analysis/fetch_extract.py`: attic-dated
  Overpass queries, blake3+licence manifest) is the *procedural
  seed* of a corpus builder but not its scale path — Overpass
  etiquette caps it at ~10² towns per run. A 10⁴–10⁵-tile corpus
  processes a pinned `planet.osm.pbf` snapshot locally
  (osmium-class tooling), keeping the same discipline: one snapshot
  date = one corpus state, one manifest line per tile.
- Completeness filtering is a corpus-build *stage*, not an
  afterthought (R1): region gating from the Herfort/Biljecki
  completeness geographies, cross-checks against the independent
  footprint sets, and per-tile quality statistics recorded in the
  manifest.
- Levels training regions are chosen where the attribute is dense
  (the 443/22,710 high-completeness units), with the geographic
  skew recorded, not hidden.

## 4. Prior work (verified)

### 4.1 Street-network generation

Graph/token-native lineage — the one this dossier adopts:

- **Neural Turtle Graphics** (Chu et al., ICCV 2019 oral,
  arXiv:1910.02055; full paper read): encoder–decoder GRUs over
  road graphs; **discrete 1 m Δx/Δy offset tokens** (clamped
  ±100 m), city-style conditioning, sketch completion; trained on
  17 OSM cities (233.6k nodes / 262.1k edges / 7,410.7 km road);
  evaluated with domain-adapted FID + urban-planning statistics +
  a diversity measure. Its adapted **GraphRNN-2D baseline failed**
  ("unnatural structures") because BFS ordering destroys spatial
  coherence — a warning against naive graph autoregression.
- **Birsak et al. 2022** (arXiv:2209.00281; CoRR only — no
  peer-reviewed venue found, flagged): sliding-window transformer
  over a **VQ dictionary of local street-context vectors**;
  generates traversable street graphs covering **400+ km²**,
  trained on OSM up to entire-US-region scale. The
  largest-extent street generator found; the scale-mechanism
  precedent for via.
- **RoadNetGAN** (Owaki & Machida, ICONIP 2020): NetGAN-style
  random walks with displacement attributes; graph-native, small
  scale.
- **Kempinska & Murcio** (Applied Network Science 4:114, 2019):
  conv-VAE over 64×64 binary rasters of 3×3 km crops, 1,059 OSM
  cities — a morphology *embedding*, not a usable generator, by
  the authors' own admission.

Raster lineage (the M2 ceiling, documented): StreetGAN (Hartmann
et al., WSCG 2017 — the first GAN attempt, raster with lossy
re-vectorization; triangulated, primary page unfetchable); Kelvin &
Anand (SIGGRAPH Posters 2020) Pix2Pix tiles; **Fang et al.** (IJGIS
36(10):2035, 2022; full paper read) — terrain-conditioned
*inpainting* of masked street regions, 10 raster channels (7
topographic: DEM, slope, aspect, 4 hillshades, with learned
gating), ~56k samples from 4 Italian cities, evaluated by pixel
error plus space-syntax metrics on re-vectorized output; **Gu et
al.** (ISPRS IJGI 13(6):203, 2024) — conditional-diffusion road
rasters over land use/elevation/slope/intersections in 5 US
cities, whose reported finding that **intersection placement
(human design) outweighs natural terrain factors** is a
counter-signal to over-weighting terrain conditioning (recorded in
R7); Yang et al. (arXiv:2305.08186) cGAN conditioned on natural +
socioeconomic factors, raster-first.

Adjacent representations (not generation): Sat2Graph's
graph-tensor encoding (ECCV 2020); MapTR / VectorMapNet
(ICLR/ICML 2023) polyline decoders from sensors; Belli & Kipf
(arXiv:1910.14388) image-conditioned graph transformer with the
**StreetMover** Sinkhorn distance — a candidate auxiliary metric;
PolyGen (ICML 2020) quantized-coordinate mesh transformer, the
canonical vector-token precedent; GraphWalker (IJCAI 2025)
latent-diffusion road graphs from GPS trajectories (map inference,
not synthesis).

### 4.2 Settlement / building generation

The frontier is one group's line — **He & Aliaga**:

- **GlobalMapper** (ICCV 2023, arXiv:2307.09693): buildings inside
  arbitrary road-bounded blocks; per-block grid-topology graph
  under a canonical spatial transform, graph-attention VAE; OSM
  data, **119,236 blocks / 2,513,697 buildings / 28 NA cities**;
  L-Sim, overlap, out-of-block, FID, Wasserstein metrics; code and
  datasets released. Generates abstracted building boxes/shape
  classes in canonical slots, not free-form polygons.
- **COHO** (ECCV 2024 oral, arXiv:2407.11294): city-scale
  hierarchical graph (buildings→blocks→communities→city) with a
  graph-based masked autoencoder; **2.5D — footprints and heights
  are generated**; **330 US cities, 833,473 blocks, 17,663,607
  buildings**; roads/blocks from TIGER, footprints+heights from
  OSM + Microsoft Footprints; code released. Streets are *input*;
  terrain absent.
- **npj Urban Sustainability 2026 follow-up** (He, Kamath, Fei,
  Niyogi, Aliaga; DOI 10.1038/s42949-026-00369-2): regenerates a
  3D city from a few percent of blocks and feeds **socio-economic
  metric prediction** — independent support for the "macro
  metadata infers economics" reading of via's goal.

Around it: **BlockPlanner** (ICCV 2021) — city blocks as land-lot
subdivisions, ring-topology vector graph, lots carry 3D geometry
and land-use semantics (the plot-stage precedent); **HouseDiffusion**
(CVPR 2023, arXiv:2211.13287) — diffusion directly on polygon
corner coordinates with joint continuous/discrete denoising, the
strongest verified mechanism for free-form vector polygons
(floorplan scale); House-GAN/++ (ECCV 2020 / CVPR 2021) —
graph-constrained relational generation with iterative validity
refinement; LayoutGAN/LayoutVAE/LayoutTransformer (2019–2021) —
the generic labeled-geometry lineage; **GANmapper / InstantCITY**
(Wu & Biljecki, IJGIS 2022 / ISPRS 2022) — streets→building
rasters aimed explicitly at OSM gap-filling (InstantCITY
re-vectorizes to polygons; both are the natural M2 baselines);
**Zheng et al.** (Nature Computational Science 3:748, 2023) —
GNN+RL jointly *plans* land use and road layout, the only verified
joint roads+land-use vector producer, but it optimizes one
normative plan against objectives rather than learning the
distribution of real morphology.

City-scale visual (raster/3D, no schema): CityDreamer (CVPR 2024,
80 cities / >6,000 km² of rasterized OSM), InfiniCity (ICCV 2023),
CityGen (CVPR 2025 *workshop*), ControlCity (arXiv:2409.17049,
SDXL+ControlNet, 3,140 tiles / 22 cities — also used for OSM
completeness assessment).

### 4.3 Attribute metadata (levels / height / use)

Generative emission of OSM-schema attributes barely exists (only
COHO's 2.5D heights). As **prediction from urban form**, it is
solidly tractable:

- Milojevic-Dupont et al. (PLOS ONE 15(12):e0242010, 2020):
  XGBoost on 152 urban-form features, ~11.5M buildings / 920
  cities (FR/IT/NL/DE), height MAE **1.47 m** — sub-storey;
  out-of-country transfer 1.72 m.
- 3D-GloBFP (Che et al., ESSD 16:5357, 2024): heights for ~1.3B
  footprints globally (XGBoost, 33 regional models), R²
  0.66–0.96.
- Arruda et al. (Scientific Data 2024): 67.7M US buildings
  classified residential/non-residential from OSM inputs.

Reading for via: `building:levels` is largely **inferable from the
fabric itself**, so the attribute head can be a conditional
predictor over generated footprints+context, trained on the tagged
subset (§3.6), and R3 is a training-region selection problem, not
a wall.

### 4.4 Scale and compute precedents

| Work | Modality | Training data | Compute (as disclosed) |
| --- | --- | --- | --- |
| NTG (ICCV 2019) | vector street graphs | 17 cities, 7,410.7 km road | not stated; 500-unit GRUs |
| GlobalMapper (ICCV 2023) | vector building layouts | 2.5M buildings, 119k blocks | **9 h on one A5000** (~0.4 GPU-days) |
| COHO (ECCV 2024) | vector 2.5D layouts | 17.7M buildings, 330 cities | not stated |
| Fang et al. (IJGIS 2022) | raster road inpainting | ~56k patches, 4 cities | 8×P100, 50 epochs |
| arXiv:2509.23804 (2025) | vector footprints + heights | ~5k blocks / 25k footprints | 1×4090-class, ~48 h |
| CityDreamer (CVPR 2024) | raster + neural rendering | 80 cities, >6,000 km² | 4–8 GPUs per component (repo); wall-clock undisclosed |
| Gu et al. (IJGI 2024) | raster road diffusion | 5 US cities | 4×A100 (secondary rendering; publisher 403'd) |

Reading: the largest data volume anyone has used (~17.7M
buildings) is ~2.5% of what OSM holds, and no work in the class
approaches even 100 GPU-days. The spike is budget-bounded by
precedent at a single 8-GPU node for days-to-weeks.

### 4.5 Novelty verdict

Every pairwise slice of via's target exists; the triple does not
(27 documented queries in the novelty sweep alone):

- terrain → streets: **raster only** (Fang 2022 inpainting; Gu
  2024 diffusion);
- streets → buildings: raster (ControlCity, GANmapper,
  InstantCITY) and vector (GlobalMapper, COHO) — streets always
  *input*;
- footprints + heights in vector: small scale, roads given, no
  terrain, no tags (arXiv:2509.23804);
- joint streets+buildings: **raster only**, for visual worlds
  (RoBus baselines, CityGen, CityDreamer);
- terrain → whole settlement: **voxel Minecraft** (IEEE CoG 2024
  terrain-adaptive PCGML; World-GAN), explicitly not map realism;
  the PCGML survey lineage (Summerville et al. 2018) does not
  cover it;
- attributes: predictive regression on existing buildings only.

**Unoccupied, specifically**: (a) DEM-conditioning of any *vector*
settlement generator; (b) joint vector generation of streets and
buildings by one learned system; (c) generative emission of
OSM-schema attribute tags as part of synthesis; (d) evaluation of
any generator against a settlement-morphology battery. The
first-in-the-world suspicion is **confirmed as far as a documented
negative can be**. Closest threats: arXiv:2509.23804 (vector +
heights; add terrain and streets) and Fang 2022 (terrain
conditioning; raster roads-only).

## 5. Architecture candidates

- **A1 — hierarchical autoregressive transformer over quantized
  vector tokens** (primary). Stage 1: street graph as a token
  sequence (NTG's 1 m offsets modernized to a transformer; Birsak's
  VQ-context sliding window is the demonstrated scale mechanism if
  tiles must grow). Stage 2: building footprints conditioned on
  streets (COHO-style block conditioning; HouseDiffusion-style
  corner generation if free-form polygons prove necessary).
  Stage 3: attribute head (levels, use) over generated
  footprints+context — a conditional predictor with §4.3's
  regression results as its feasibility floor. Terrain enters as a
  raster encoder (hillshade-ensemble stack, learned gating — Fang's
  lesson) with cross-attention at every stage. ESTIMATE 25–300M
  params across stages.
- **A2 — graph diffusion** (DiGress-class discrete diffusion +
  continuous coordinates). Attractive for global coherence; **no
  street application exists** (verified negative) and demonstrated
  graph sizes (≤5k nodes, GRAN) sit at the low end of town scale —
  higher risk, kept as the research-upside alternative.
- **A3 — raster latent diffusion + vectorizer** (the M2 control;
  ControlCity/CityGen/Gu lineage). Cheap to stand up; run it to
  *measure* the vectorization penalty on the battery rather than
  assert it.
- **A4 — learned fields + deterministic tracer** (the M3 fallback;
  half-way to the simulation thesis).

Recommendation to the ADR: **A1 primary, A3 as control, A2/A4 named
fallbacks.** The spike's first falsifiable milestone is
deliberately narrow: streets-only A1, class-conditioned,
unconditional-terrain first (matching NTG's problem), judged by
via-bench against the nulls — before any terrain encoder or
building stage is built.

## 6. Evaluation

Unchanged instrument, by design:

- Generated towns are written as synthetic extracts (§2.3) and
  measured by via-bench under the frozen protocol (v1.1, or its
  successor if the nine-point v1.2 agenda lands first — the
  instrument version is pinned per experiment either way).
- Ensembles, never single seeds: ≥5 seeds per condition (working
  rule; the D5/D9 precedent is 8), per-class comparison against
  the fitted-partition aggregates in
  `reference/populations-v1.1.json`.
- The D9 null tables are the floor: no null (matched grid, random
  planar, DLA) reproduces any class across the battery; "a future
  generator matching a class on one character has matched nothing
  a null could not" (benchmark report v0). A trained model must
  clear the nulls before any stronger claim.
- **Memorization audit** joins the battery for this path:
  nearest-training-tile similarity on generated output (candidate
  metric: StreetMover distance + footprint IoU), serving as
  overfitting check and ODbL licensing control simultaneously
  (§3.4).
- The 44-town held-out partition is spent only at a declared
  validation event (held-out policy, benchmark report v0; ADR
  0010's pre-registration doctrine), after the fitted-side result
  looks real.
- Success vocabulary: the spike is *alive* if class-conditioned
  generation lands inside fitted envelopes on the topological core
  (degree shares, meshedness, bc_gini, orientation order) for ≥3
  of 5 classes — a declared acceptance threshold in ADR 0008 D8's
  sense, a conjunction of named individual characters with no
  composite score, which the adopting ADR must ratify with
  rationale; it earns IMPOSSIBLE-so-far if topology collapses
  (mode-collapsed grids, broken planarity) or completeness bias
  dominates after the named mitigations. Auxiliary diagnostics
  (NTG's adapted-FID protocol, StreetMover) may be reported but
  never replace the battery.

## 7. Schedule and budget (optimistic; every number ESTIMATE)

### 7.1 Verified price floor (accessed 2026-08-21)

| Resource | Price | Source |
| --- | --- | --- |
| H100 SXM 80GB | $2.69/hr (RunPod community) – $4.29/hr (Lambda 1×) | runpod.io/pricing, lambda.ai |
| 8×H100 node | ~$21.52/hr (RunPod community, 8×$2.69) – $31.92/hr (Lambda) | same |
| A100 80GB | $1.19–1.59/hr (RunPod verified; a Lambda $1.99 listing reads as the 40 GB tier on review) | same |
| RTX 4090 | $0.34/hr (community) – $0.74/hr (secure) | runpod.io |
| AWS p5 (8×H100) | $55.04/hr on-demand us-east-1 | instances.vantage.sh (mirrors AWS API) |

Vast.ai floors (~$0.13–0.59/hr 4090-class) are UNVERIFIED (dynamic
page; secondary sources only).

### 7.2 Budget (protocol: §4.4 precedent budgets × §7.1 prices)

| Item | Hardware shape | Est. cost |
| --- | --- | --- |
| corpus build (planet.pbf, tiling, filters) | CPU-only; local machine + ~1 TB disk | ~$0 cloud (low $100s if remoted) |
| ML-2 streets-only baseline | 1× 4090/A100, 2–5 days | $30–250 |
| ML-3 terrain conditioning + scale-up | 1–8× A100/H100, ~1 week | $500–5,000 |
| ML-4 full fabric + attribute head | 8×H100 node, 1–2 weeks upper bound | $3,600–10,700 |
| ablations, reruns, seed variance (×2–3) | — | ceiling ~$15k–30k |

Precedent check: a GlobalMapper-scale run (9 GPU-hours) costs
$3–7 on a RunPod community 4090 and ~$18 on Lambda's cheapest
listed GPU; CityDreamer-scale (8-GPU node, 1–2 weeks assumed —
wall-clock undisclosed) ≈ $5.4k–10.7k on Lambda. A 10× overshoot
of every published precedent stays in five figures. Realistic
optimistic spike total: **$5k–10k**; ceiling $30k. Training does
not fit the local Apple-silicon machine beyond toy scale; corpus
work does.

### 7.3 Schedule (optimistic, single owner + agent support)

| Phase | Weeks | Exit criterion |
| --- | --- | --- |
| ML-0: this dossier + ADR | 0–1 | dossier reviewed; ADR (spike scope + corpus design) proposed |
| ML-1: corpus | 1–3 | pinned planet snapshot; pilot corpus ~1–5k tiles from 2–3 well-mapped regions; completeness filters measured; manifest discipline (one snapshot = one corpus state); RoBus licence check; ODbL memo |
| ML-2: streets-only baseline | 3–6 | class-conditioned vector AR model; via-bench run clears all three nulls per class on the topological core |
| ML-3: terrain conditioning | 6–9 | the unoccupied slice: DEM-paired corpus; conditioning ablation — does terrain measurably move the generated fabric (vs the Gu et al. counter-signal, R7)? |
| ML-4: full fabric | 9–13 | buildings + levels head; benchmark report vML-0 against fitted populations; verdict point |

Held-out validation only if the fitted-side result warrants it (the
44 towns are spent once). Verdict at ~3 months optimistic;
"optimistic" assumes no corpus rebuild and no architecture pivot —
either adds 2–4 weeks.

## 8. Risks

- **R1 — mapping bias**: the model learns OSM's mapping, not the
  world (measured precedent: the medina finding; no literature
  covers this failure mode for generative models — via would be
  documenting it first). Mitigations: completeness-gated corpus
  (§3.6), class/region stratification, footprint cross-checks
  against the independent building datasets.
- **R2 — evaluation leakage**: any pre-registered town in training
  invalidates the benchmark. Mitigation: mechanical exclusion with
  geographic buffer; held-out stays sealed.
- **R3 — attribute sparsity**: 4.6% levels coverage (2023; ~6.0%
  live 2026), extreme district-level skew, 443 high-completeness
  units. Mitigation: §4.3 — levels are predictable from form
  (MAE 1.47 m height equivalent); train the head where tags are
  dense, record the skew.
- **R4 — terrain domain gap**: via terrain must be
  in-distribution for the conditioning encoder (§2.2); testable by
  encoder-feature statistics before any generation. Plus the
  GLO-30 DSM leak (§3.5).
- **R5 — era**: structural (§2.5); the ML path cannot deliver true
  era control from OSM alone.
- **R6 — licensing**: internal spike clean; weight distribution
  and output memorization are the two genuinely open legal
  questions (§3.4); the memorization audit is the standing
  control.
- **R7 — terrain may under-determine fabric**: Gu et al. 2024
  report human design (intersection placement) outweighing
  natural terrain factors in their conditioning ablation. If that
  generalizes, terrain-conditioning gains are modest and the
  differentiator shifts to the joint-vector + metadata slices.
  The ML-3 ablation is designed to answer exactly this.
- **R8 — scale/coherence**: town-tile token counts (10³–10⁴
  street segments + 10³–10⁴ buildings) exceed anything the
  vector-native literature has generated in one pass except
  Birsak's streets-only result. Hierarchical factorization (§5)
  is the mitigation; if it fails, M3 is the fallback.

## 9. Sweep record

Four sweeps, run 2026-08-21 as independent background agents, 84
documented queries total (18 street-generation + 22
settlement/building + 17 data/licensing + 27 novelty/compute), ~25
targeted verification fetches each. Verification standard: claim
retained only if confirmed against a fetched primary page (arXiv
abs, DOI/Crossref, publisher, official repo, OSMF wiki, taginfo
API, licence PDF); three papers read in full (NTG, Fang et al.
2022, Kelvin & Anand 2020) plus the Biljecki 2023 PDF and the
GLO-30 licence text; everything else marked UNVERIFIED in the
sweep outputs and either dropped or flagged inline here.

Corrections to received wisdom caught by verification: Barrington-
Leigh & Millard-Ball is PLOS ONE, not PNAS; CityGen's venue is a
CVPR 2025 workshop, not the main conference; ControlCity was
retitled in v3; InfiniCity's venue (ICCV 2023) appears only on its
project page; Birsak et al. remains unpublished beyond arXiv; the
"≈3%/4%" OSM attribute statistic traces to Biljecki et al. 2023
with exact values 2.9%/4.6%; Kempinska & Murcio is 64×64 px per
3×3 km — coarser than commonly assumed — with weak generative
results by its authors' own statement.

Adversarial review (2026-08-21, independent agent, three tracks):
all ten load-bearing external claims re-verified CONFIRMED against
primary pages, taginfo counts exact to the digit; one MAJOR finding
(this document originally conflated the protocol's 750 m fetch
radius with its 300 m study disc — corrected in §2.2), four MINOR
(citation pointers for the held-out policy, an overstated
"field's ceiling" parenthetical, two budget-table imprecisions) and
six precision notes (Fang-lesson attribution, Herfort edits-vs-data
wording, tag-ratio caveat, quote elision, A100 price bracket,
adoption-language softening) — all corrected in place before
commit.

Standing negatives (first-class): no graph-diffusion street
application; no vector terrain-conditioned from-scratch generator;
no joint vector streets+buildings system; no generative emission
of OSM attribute tags; no generator evaluated against a
morphology battery; no LLM street-graph generator found; no study
of generative models inheriting OSM completeness bias; no
published OSM road-km total; no precedent of released OSM-trained
weights; Vast.ai prices and several publisher pages (MDPI,
ResearchGate, Nature cookie-walls) unfetchable — affected claims
carry their secondary-source flags inline.
