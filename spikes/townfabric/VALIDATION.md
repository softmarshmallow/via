# Step 1 — the standard

The first version of this spike measured three statistics, found one
inside a literature band, and called that partial validation. That was
wrong three times over, and this document exists so it cannot happen
again.

1. The topology statistics were computed on an **unsimplified** graph.
   Every geometry vertex counted as a node, which drives meshedness
   toward zero and makes betweenness meaningless. Every published figure
   refers to the simplified graph — junctions and whole streets.
2. A single aggregate statistic is **weakly diagnostic**. Percolation
   and DLA models reproduce urban statistics with no human mechanism in
   them at all (docs/research/humanity/0007). Matching one number is
   near-zero evidence.
3. The literature band was treated as the target. Measured under one
   fixed protocol, **real towns do not sit in those bands** — see
   Alnwick below. The band is not the standard; a real town is.

So: the standard is a set of real towns, measured by the same code that
measures the synthetic ones.

## Protocol (frozen; every number below depends on it)

- **Study area**: the central 600 m disc (radius 300 m).
- **Buffer**: fabric is imported to 1.45× the study radius and
  participates in the graph, so streets leaving the study area are not
  counted as dead ends and routes through the edge still exist (Ratti's
  edge-effect critique).
- **Graph**: degree-2 vertices dissolved; nodes are junctions and dead
  ends, edges are whole streets.
- **Street set**: motorway…residential, unclassified, service,
  living_street, pedestrian, minus driveways and parking aisles.
  Footways, paths, steps and cycleways are **excluded** by default —
  `alnwick-paths` shows the same town with them included, and the
  difference is large enough that the protocol must be stated with
  every number.
- **Setback** is measured from a building's nearest corner to the
  nearest street **centreline**, so it includes half the carriageway.

## Reference towns

Fetched from OpenStreetMap (© OpenStreetMap contributors, ODbL) into
the gitignored `runs/` tree; nothing is redistributed from this repo.

```bash
curl -s -o runs/reference/osm/alnwick.json "https://overpass-api.de/api/interpreter" \
  --data-urlencode 'data=[out:json][timeout:120];(way["highway"](around:750,55.4147,-1.7061);way["building"](around:750,55.4147,-1.7061););(._;>;);out body;'
```

| town | why this one |
| --- | --- |
| **Alnwick**, Northumberland | The town Conzen's 1960 study *is*. Organic medieval core. |
| **Monpazier**, Dordogne | The canonical 1284 bastide: a planted medieval grid, largely intact. |
| **Abilene**, Kansas | 19th-century railroad plat town — the frontier case. |
| **Levittown**, New York | The FHA-era suburb — the modern case. |

## Measured, one protocol, same code

| | M | dead % | seg m | blocks | blk med m² | blk p90 m² | bldgs | foot med m² | foot p90 m² | setback m | wall % | backbone |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| REAL alnwick | 0.057 | 37 | 37 | 13 | 1675 | 28843 | 303 | 113 | 280 | 6.9 | 46 | 0.53 |
| REAL alnwick (+alleys) | 0.138 | 25 | 28 | 37 | 1426 | 15820 | 303 | 113 | 280 | 4.6 | 63 | 0.54 |
| REAL monpazier | 0.181 | 13 | 48 | 26 | 2515 | 9254 | 393 | 77 | 184 | 3.4 | 79 | 0.33 |
| REAL abilene | 0.215 | 0 | 73 | 28 | 8341 | 14709 | 116 | 216 | 728 | 12.1 | 4 | 0.39 |
| REAL levittown | 0.084 | 25 | 50 | 12 | 1587 | 19962 | 144 | 147 | 358 | 16.0 | 2 | 0.42 |
| SPIKE medieval organic | 0.216 | 34 | 60 | 58 | 1914 | 4688 | **864** | 80 | 132 | **1.7** | **94** | 0.59 |
| SPIKE frontier plat | 0.214 | 33 | 71 | 38 | 2775 | 13921 | 533 | 103 | 165 | 5.7 | 52 | 0.53 |
| SPIKE modern zoned | −0.039 | 59 | 106 | 8 | 5914 | 11636 | 61 | 322 | 486 | 8.6 | 44 | 0.51 |

## What the gaps say

1. **Every block is tiled with buildings.** Alnwick holds 303 buildings
   in the study disc; the medieval spike holds 864. Real blocks are
   mostly *unbuilt* — a frontage of buildings with gardens, yards and
   back-land behind. This is the burgage structure, and it is the
   largest single reason the synthetic fabric does not read as a town.
   The block-area p90 says the same thing from the other side: 28,843 m²
   at Alnwick against 4,688 m² in the spike — real towns keep big
   undivided blocks, the spike chops everything.
2. **Setbacks are not honoured in any era.** Real: 6.9 m (Alnwick),
   12.1 m (Abilene), 16.0 m (Levittown). Spike: 1.7 / 5.7 / 8.6 m, and a
   street-wall share of 94% where Levittown's is 2%. The cause is in the
   code: the setback insets the parcel's bounding box about its centre,
   and the coverage cap then re-centres the footprint, so buildings end
   up in the middle of their lot rather than set back from a frontage
   line.
3. **Buildings are too uniform.** p90/median footprint: 2.5 (Alnwick),
   3.4 (Abilene) against 1.5–1.65 in the spike. Real towns carry a long
   tail — churches, halls, barns, commercial blocks.
4. **Too many dead ends, and meshedness is uncalibrated.** Abilene's
   plat has 0% dead ends; the spike's plat has 33%. Organic Alnwick is
   0.057–0.138 meshedness; the spike's organic town is 0.216.
5. **The extent rule does not work.** The README claimed a town's extent
   follows from households needing lots. It does not: the growth loop
   breaks before updating the radius, and the soft frontier leaks, so
   extent is set by leakage. Declared radius 132 m against an observed
   built radius of 297 m.

## Retracted

The earlier claim that "medieval meshedness 0.177 lands in the organic
band (0.15–0.26)" is withdrawn. It was measured on an unsimplified
graph, and the band itself does not describe Alnwick measured under a
fixed protocol. No statistic in this spike should be quoted without the
protocol above.

---

# Step 2 — plots run back from the street

## What changed, and why

The subdivision was rebuilt on the mechanism the sources describe rather
than on recursive halving:

- **Plots front a street and run back into the block** (Conzen 1960's
  plot pattern; Vanegas et al. 2012 choose the split axis *against* the
  frontage). Widths are drawn as multiples of the era's module, with
  half- and one-and-a-half-width draws, which is what puts Slater's
  (1981) quarter-width structure into a burgage series.
- **Buildings fill a plot in phases** — Conzen's burgage cycle:
  *institutive* (a building on the frontage), then *repletive*
  (outbuildings accreting down the tail) until the coverage demand
  supports is reached. Coverage is a per-plot state driven by demand,
  not a constant.

Three geometry defects were fixed in the process, each of which had been
silently producing wrong numbers: building rectangles were never clipped
to their own plot (so they spilled past block corners into the street);
the block inset fell back to centroid scaling on concave blocks, which
is not a constant offset, so frontage lines were not where the
parameters said; and the growth loop updated the built radius *after*
its stopping test, so "extent follows from households" was never true
(declared 132 m against an observed 297 m).

## Calibration, and why it is not validation

Growth parameters (λ₀, reach, ω) were fitted by sweeping 36 combinations
against **Alnwick** and scoring on block count, block-area median and
p90, meshedness, dead-end share and segment length
(`scripts/calibrate.py`). Best: λ₀ 60 m, reach 110 m, ω 0.42.

Those parameters were then compared against **Lavenham**, a medieval
town the fit never saw. The two references agree closely with each other
(meshedness 0.057 / 0.073; 13 / 11 blocks; 303 / 257 buildings), which
is what makes them usable as a class.

| medieval | spike | Alnwick (fitted) | Lavenham (held out) |
| --- | --- | --- | --- |
| meshedness | 0.063 | 0.057 | 0.073 |
| dead-end share | 0.29 | 0.37 | 0.28 |
| blocks | 16 | 13 | 11 |
| block area p90 | 29,697 | 28,843 | 32,575 |
| buildings | 299 | 303 | 257 |
| footprint p90/median | 2.1 | 2.5 | 2.4 |
| backbone concentration | 0.53 | 0.53 | 0.55 |
| median setback | **2.5** | 6.9 | 6.0 |
| street-wall share | **0.95** | 0.46 | 0.51 |
| footprint median | **87** | 113 | 125 |

## The battery grew again, for the same reason as last time

The numbers above matched while the picture still looked wrong: the
blocks were long triangular wedges. No statistic in the battery could
see that, so two were added — median corner count and compactness
(4πA/P²):

| | corners | compactness |
| --- | --- | --- |
| REAL alnwick | 7.0 | 0.52 |
| REAL lavenham | 5.0 | 0.59 |
| REAL abilene | 4.0 | 0.74 |
| REAL levittown | 4.0 | 0.52 |
| SPIKE medieval | 4.0 | **0.39** |
| SPIKE frontier | 4.0 | 0.64 |
| SPIKE modern | 4.0 | 0.43 |

Real medieval blocks are irregular (5–7 corners) and reasonably compact;
the spike's are four-cornered wedges. The cause is in the growth rule:
new sites connect to existing nodes with straight chords, which
triangulates.

## Still failing after step 2

1. **One uniform street width.** Setback 2.5 m against 6–7 m, and a 95%
   street wall against ~50%. Real towns mix narrow lanes with wide main
   streets, and about half their buildings are more than 6 m from a
   centreline as a result. Street classes exist in the graph but are
   assigned at construction, not earned — which is step 3's centrality
   work.
2. **Block shape**: compactness 0.39 against 0.52–0.59 (above).
3. **The frontier plat cannot be validated yet.** Its reference is a
   *downtown*: Abilene's central 600 m holds 116 large commercial
   buildings, while the spike fills every frontage with 25-ft
   residential lots (595 buildings). This is the missing land-use
   dimension, not a parameter error. Its plat is also chewed up by
   organic growth: 31% dead ends against Abilene's 0%.
4. **The modern suburb is under-housed**: 57 buildings against
   Levittown's 144, because lots are only generated inside closed
   blocks and cul-de-sac fabric encloses few.
5. OSM building coverage is patchy in US residential areas
   (`abilene-res` has 7 footprints in the study disc), so the frontier
   case can validate streets but not buildings.

---

# Step 3a — streets earn their class

## What changed

- **Angular segment choice** (Hillier & Iida 2005): betweenness over the
  segment graph where the cost of a route is the *turning* it requires,
  not its length. Only choice is used — docs/research/humanity/0008
  records why: the 2018 meta-analysis puts choice at ~0.48 but
  integration at 0.206.
- **Class follows choice, width follows class.** A street is a main
  street because movement uses it; it is wide because it is a main
  street. Previously class was decided at insertion (corridors were
  corridors because they were drawn first), and one effective width put
  every building 2.5 m from a centreline.
- **Densification** (Strano et al. 2012 name densification and
  exploration as the two elementary processes of road-network growth;
  this spike had only exploration): links joining junctions that are
  close in space but far apart through the network.
- **Protocol parity.** The reference measurement excludes footways and
  alleys, and this spike's Lane class is that category. Comparing a
  synthetic graph that includes lanes against a real one that excludes
  them measures the protocol, not the town — so every synthetic town is
  now measured both ways, and compared against the matching reference
  variant.
- **Class shares calibrated from the reference**, not guessed: Alnwick's
  mapped streets are 60% of its network length (4.04 km of 6.76 km with
  paths included).

## Single-seed numbers were misleading

Seed 42 produced a meshedness of 0.072 against Alnwick's 0.057 — an
apparent bullseye. Over five seeds the model's actual spread is
0.028–0.113. Every number below is therefore reported as mean ± sd over
five seeds, which is what docs/research/humanity/0011 asks for and what
the earlier steps failed to do.

**Medieval, five seeds, streets-only protocol:**

| statistic | spike (mean ± sd) | Alnwick (fitted on) | Lavenham (HELD OUT) |
| --- | --- | --- | --- |
| meshedness | 0.069 ± 0.036 | 0.057 | 0.073 |
| dead-end share | 0.32 ± 0.05 | 0.37 | 0.28 |
| blocks | 18 ± 5 | 13 | 11 |
| buildings | 364 ± 68 | 303 | 257 |
| median setback | 6.4 ± 0.8 m | 6.9 m | 6.0 m |
| street-wall share | 0.41 ± 0.05 | 0.46 | 0.51 |
| block compactness | **0.40 ± 0.06** | **0.52** | **0.59** |

Five of seven statistics contain both references inside the seed spread.
Blocks and buildings run slightly high; compactness is outside it in
both directions and is therefore a systematic defect, not seed noise.

Growth parameters were re-fitted by sweep (54 combinations, scored on
the streets-only statistics) after the mechanisms changed, because the
step-2 fit was stale. Medieval: λ₀ 34 m, reach 110 m, ω 0.30, 3
densification links per round.

## Still failing after step 3a

1. **Blocks are wedges** — compactness 0.40 against 0.52–0.59, and it is
   what the eye sees first in the figure. The cause is the growth rule:
   sites connect to the network, and densification links join existing
   junctions, both with straight chords, which triangulates. Barthélemy
   & Flammini (2008) grow roads by *extending existing tips* toward
   unconnected demand instead, which is what produces elongated,
   non-triangulated fabric. That is the next mechanism to replace.
2. **The frontier plat still cannot be validated on buildings**: 553
   against Abilene's 116, because its reference is a downtown of large
   commercial buildings and the spike builds 25-ft residential lots
   everywhere. Land use remains the missing dimension. Its streets do
   match: setback 11.5 m against 12.1, street wall 0.00 against 0.04.
3. **The modern suburb's meshedness overshoots** (0.31 against 0.084):
   Levittown's combination of loops *and* 25% dead ends is a shape the
   growth rule cannot currently produce — it makes either trees or
   meshes.
