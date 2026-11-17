# Ground, water and growth

Sections 2 to 5 of the [ledger](README.md). The row shape and the marks are explained in the
README; the archetypes in [archetypes.md](archetypes.md).

## 2. Terrain and ground

From general knowledge (2026-11-12), re-read against the first cut on 2026-11-18. The
subsections were read from sources in 2026-11 (`docs/references.md`); a row marked `(?)` is from
memory.

| Term | Meaning | Archetype | Whose | Now |
|---|---|---|---|---|
| heightmap, heightfield | the ground as one height per grid post; it cannot overhang | Field | core | shown: `[field.<name>]`, `fields.md 1` |
| heightmap format, bit depth | how heights are stored; 8 bits shows as steps | Field | core | shown: `fields.md 4`, `examples/relief` |
| height scale, vertical range | what world heights the stored values stand for | Field | core, loader | shown: `range` |
| tiled terrain import | very large terrain as a grid of files | Field | core, loader | none |
| terrain resolution | metres between posts | Field | core | shown: `resolution` |
| feature narrower than the posts | a rut or ditch the height field cannot hold | Representation switch | core | none |
| landform: ridge, valley, saddle, pass, mesa, canyon, plateau | named shapes of land that rules and people refer to | Area partition | lib | none |
| relief at three scales | terrain shape read large, medium and fine | Guidance | core | none |
| slope, aspect, curvature | measures of the ground that rules read: steepness, the compass direction a slope faces, convex against concave; they stand in for soil depth and stability, sun and shade, exposure and gathering water | Derivation (focal) | core | partly: `slope()` |
| flow accumulation, drainage, wetness | how much upstream ground drains through each post, and that weighed against slope: where water runs, gathers and keeps the soil moist | Derivation (global) | core | partly: `lib/land/uplands.py`, `lib/terrain/kinds.toml`, `derived.md 4` |
| erosion | water and slumping shaping the land, simulated at build time | Simulation | lib, game | shown: `lib/terrain/kinds.toml`, `examples/basin` |
| deposition, talus, alluvial fan | where eroded material comes to rest: broken rock piled at its resting angle below a cliff, a fan where a stream leaves the hills | Simulation | lib | partly: `lib/land/weather.py` |
| erosion masks: flow, wear, deposit | by-products of erosion that drive ground and scatter | Simulation | core | none |
| strata, bedrock, outcrop | rock type and layering showing through the soil | Field | lib, pack | none |
| cliff | a face too steep or overhanging for a height field, replaced by mesh geometry | Representation switch | lib | shown: `land.sheer_cliff`, `examples/coast` |
| overhang, cave, arch | ground above ground, with air beneath it | Representation switch | core | partly: `land.cave`, `surfaces.md 6` |
| pit, chasm, sinkhole | a vertical opening joining surface and underground | Representation switch | core | none |
| floating island | a landmass with no ground link and a visible underside | Representation switch | lib | none |
| terrain hole | a gap cut in the height field | Representation switch | core | shown: `cut` |
| building pad, platform | ground levelled or reshaped for something to stand on | Profile | lib | shown: `land.pad` |
| natural or farmed terrace | stepped ground made by a river or by farming | Profile | lib | partly: `town.terrace` |
| stamp | a reusable shape pressed into the terrain | Field | core | shown: `examples/relief` |
| weight map, splat map | how much of each ground material lies at each post | Field | core | shown: `type`, `fields.md 1` |
| ground material, terrain layer | one material the ground is painted with, with its own small repeating texture | Asset | pack | partly: `[material.<name>]` |
| material limit per tile | a cap on how many materials meet in one terrain piece | Delivery | core, loader | none |
| rule-driven ground material | material chosen by slope, height and wetness, not painted | Derivation (local) | lib | shown: `lib/land/ground.md`, `paint` |
| height blend, layer blend | how two ground materials mix at their border | Meeting | pack, loader | none |
| macro variation, colour map | large-scale colour that hides texture repeats | Field | core, loader | none |
| triplanar mapping | texturing steep faces without stretching | Runtime | engine, pack | none |
| mesh-to-terrain blend | softening the line where a mesh enters the ground, by fading its base into the terrain's own colour | Meeting | loader, engine | none |
| snow cover, snow line | where snow lies, by height, slope and aspect | Derivation (local) | core | partly: `examples/vale`, `lib/land/ground.md` |
| ground wetness, puddles, mud | ground changed by rain or thaw | Runtime | loader, engine | none |
| deformable ground | snow and mud that record what passed over | Runtime | engine, game | none |
| coarser ground with distance | terrain drawn simpler further away | Runtime | engine | none |
| displacement | fine relief added when drawn | Runtime | engine | none |
| terrain texture streaming | the ground's texture loaded by need | Runtime | engine | none |

### How terrain is made

Three families of method, usually combined (Galin et al. 2019):

| Family | Idea | Control | Realism |
|---|---|---|---|
| Procedural | A rule or function produces the shape directly, with no physics. | Varies by sub-family | Plausible, not geologically correct |
| Simulation | Physical processes (erosion, uplift) are run on a starting shape. | Indirect: you set conditions and wait | High |
| Example-based | New terrain is assembled or learned from real elevation data. | By sketch or exemplar | As good as the data |

| Term | Meaning | Archetype | Whose | Now |
|---|---|---|---|---|
| noise | layered random functions of position; fBm, below, is the usual form | Field | core | shown: `fbm()`, `ridged()` |
| fBm | layered noise at falling scale and strength; the base of most landforms | Field | core | shown: `fbm()` |
| ridged multifractal | noise folded to give sharp ridgelines | Field | core | shown: `ridged()` |
| domain warping | distorting the lookup position of a noise with another noise | Derivation (local) | core | none |
| subdivision | refine a coarse shape by repeatedly splitting and randomly displacing (midpoint displacement, diamond-square) | Grammar | lib | none |
| faulting | repeatedly cut the terrain along random lines and offset one side | Field | lib | none |
| feature-based construction | place ridges, rivers and hills as controlled features and build the ground from them; the next subsection | Profile | core | shown: `authority.md 2` |
| tectonic uplift | ground rises at a mapped rate while rivers cut it down; ranges and branching valleys result (Cordonnier et al. 2016); scale: tens to hundreds of km | Simulation | lib | none |
| hydraulic erosion | running water cuts gullies and drops sediment in fans and basins; scale: kilometres | Simulation | lib | shown: `lib/terrain/kinds.toml`, `examples/basin` |
| thermal erosion | over-steep rock breaks and slides; leaves cliffs above rubble slopes; scale: local | Simulation | lib | shown: `lib/land/weather.py` |
| stream power law | large-scale river incision from drainage area and slope | Simulation | lib | none |
| drainage consistency | every valley connects downhill to an outlet; the clearest mark of real or simulated terrain | Network (directed) | lib | partly: `terrain.terrain_river` |
| amplification | adding plausible fine detail to a coarse terrain | Derivation (focal) | lib | none |
| sparse modelling | terrain described as a combination of a few patches from a learned dictionary (Guérin et al. 2016) | Derivation (global) | lib | none |
| patch synthesis | stitch pieces of real elevation data to follow a user sketch (Zhou et al. 2007) | Derivation (global) | lib | none |
| learned synthesis | a trained network turns a sketch of ridges and rivers into terrain (Guérin et al. 2017) | Derivation (global) | lib | none |
| voxel terrain | volume representation for caves and overhangs (SDF, marching cubes, dual contouring) | Representation switch | core | none |

How to recognise a method from its result (the same review, figure 34):

| Method | Visual signature |
|---|---|
| Ridged noise | Sharp crests, equally rough everywhere, valleys that lead nowhere. |
| Multifractal | Roughness varies with height: smooth lowlands, rugged peaks. Still no drainage. |
| Subdivision | Lumpy and uniform, no structure at any scale. |
| Thermal erosion | Flat-topped blocks, vertical faces, rubble aprons. |
| Tectonic | Long ridges and a branching valley network that drains consistently. |
| Hydraulic erosion | Fine gullies on slopes, flat sediment-filled basins. |
| Example-based | Real-looking detail, arrangement not chosen by the author. |
| Controlled example | Real-looking detail following a sketched layout. |
| Deep learning | Follows sketched rivers and ridges; plausible, slightly soft. |
| Sparse | Real detail recombined from a learned set of small patches. |

### Feature-based modelling

Describing a world by its named, meaningful things and deriving the cells from them, as opposed
to editing cells directly. Galin et al. 2019 name this family "feature-based construction"
within procedural terrain methods.

| Term | Meaning | Archetype | Whose | Now |
|---|---|---|---|---|
| feature (geography) | one real-world thing as a shape plus attributes: a river, a road, a parcel | Shape | core | shown: `[feature.<id>]` |
| feature-based terrain | terrain defined by ridge lines, river lines and cliff lines, each with a side profile; the ground between is filled in (Hnaidi et al. 2010) | Profile | core | partly: `land.cliff` |
| diffusion | filling the space between features by smoothly spreading their heights and slopes outward | Derivation (global) | core | none |
| primitive | a small parametric landform (hill, crater, valley segment) with a limited area of effect | Field | core | shown: `water.pond`, `land.cliff`, `radius` |
| construction tree | primitives combined by add, blend and replace operators into one terrain (Génevaux et al. 2015) | Grammar | core | partly: `authority.md 4` |
| feature tree (engineering design) | a part recorded as an ordered history of meaningful operations (hole, fillet), re-run when one changes | Principle | core | partly: `graph.md 1`, `authority.md 4` |
| rasterisation | turning features into cells | Derivation (local) | core | shown: `authority.md 2` |
| feature extraction | the reverse: recovering ridges, rivers or outlines from cells | Derivation (global) | core | shown: `where`, `derived.md 1` |

### Terraforming

Deliberately reshaping existing ground to suit something, as opposed to generating the ground in
the first place. The word has three uses: an authoring operation, a pipeline step where placed
things reshape terrain, and a gameplay mechanic.

| Term | Meaning | Archetype | Whose | Now |
|---|---|---|---|---|
| brush | a footprint with a radius, a strength and a falloff towards its edge | Profile | core | shown: `[kind.<k>.effect.<id>]`, `radius`, `mask` |
| spline terraforming | ground reshaped along a curve to carry a road or river, blending back to natural ground at the sides | Profile | lib | shown: `ways.road` |
| grading | shaping ground to a designed slope, for drainage or for a route | Profile | lib | shown: `ways.road` |
| cut and fill | lowering ground in one place and raising it in another to fit a grade | Profile | lib | shown: `ways.freeway` |
| earthwork balance | arranging that the volume cut roughly equals the volume filled | Measure | lib | none |
| conform direction | whether the ground adapts to the object (pad) or the object to the ground (stilts, stepped foundations) | Meeting | core | shown: `ground`, `rules.md 6` |
| overlap priority | which reshaping wins where two meet, such as a road crossing a river | Meeting | core | shown: `authority.md 4` |
| edit layer | a reshaping kept as a separate, removable layer over the base terrain | Overlay | core | shown: `[field.<name>.then.<id>]` |
| runtime terraforming | players reshape ground during play (digging, building); needs ground that can change and be re-meshed | Runtime | game | none |
| delta storage | saving only the changes made to a generated terrain, not the terrain itself | Overlay | game | none |

### Geology and cliffs

What the ground is made of beneath the surface, and how that shows where it is steep enough to
be exposed. The layer-stack rows are from Peytavie et al. 2009.

| Term | Meaning | Archetype | Whose | Now |
|---|---|---|---|---|
| stratum (bed) | one layer of rock laid down over a period; strata stack like pages | Field | lib | none |
| bedding plane | the flat boundary between two strata | Field | lib | none |
| dip and strike | the tilt of the strata: how steeply they lean and along which compass line | Field | lib | none |
| rock hardness | how well a stratum resists erosion | Field | lib | none |
| differential erosion | soft strata wear back faster than hard ones, so a rock face becomes stepped | Simulation | lib | none |
| ledge / bench | the shelf a hard stratum leaves standing out | Derivation (local) | lib | none |
| fold, fault | strata bent, or broken and shifted, by movement of the crust | Field | lib | none |
| strata function | layer chosen from a position's distance along the tilted stacking direction, with noise to bend the bands | Derivation (local) | lib | none |
| band displacement | each stratum on a cliff mesh pushed out or in by its hardness, forming ledges and recesses | Derivation (local) | lib | none |
| terracing | the same stepping applied to a heightfield: heights snapped towards levels | Derivation (local) | lib | none |
| layered heightfield | per column, a thickness for each material in a fixed order (bedrock, sand, water); still no air gaps | Field | core | none |
| layer stack | per column, an ordered list of material intervals that may include air, so rock can sit above a gap (Peytavie et al. 2009) | Field | core | none |
| implicit surface | a surface defined as where a function over space crosses a threshold; gives a smooth skin over a layer stack | Representation switch | core | none |
| bedrock, granular material | solid rock versus loose sand and stones; erosion converts the first into the second | Field | lib | none |
| block detachment | a chunk of an over-steep face breaks off; its outline taken from a cell of a random partition of space | Simulation | lib | none |
| stabilisation | loose material falls and slides until it rests in piles at its resting angle | Simulation | lib | shown: `lib/land/weather.py` |
| rock pile | individual stones generated from a granular layer, resting on one another | Scatter | lib | none |
| volumetric extent limit | a full-volume model is costly (the one read covers about a cubic kilometre), so it is used locally | - | core | none |

### Ground surface

What the ground looks like, as distinct from its shape. Decided late, because most of it follows
from what stands on the ground.

| Term | Meaning | Archetype | Whose | Now |
|---|---|---|---|---|
| tiling texture | a small detail image repeated across the ground; sharp up close, visibly repetitive from afar | Asset | pack | partly: `repeat` |
| tint map | a low-resolution colour field over the whole terrain, multiplied onto the detail textures | Field | core | none |
| ground-colour inheritance | grass and small plants take the tint of the ground beneath so they blend in | Scatter | loader | none |
| ground footprint | the material an object asks for beneath itself; placing it stamps that into the weight map | Derivation (local) | lib | partly: `ground`, `paint` |
| write-back | a later stage (scatter, road, water) changing the surface an earlier stage produced | Derivation (local) | core | shown: `[kind.<k>.effect.<id>]`, `paint` |
| skirt | extra geometry at an object's base, textured like ground, that overlaps the terrain | Meeting | pack | partly: `land.sheer_cliff`, `build.md 10` |

### Terrain analysis (abiotic data)

Abiotic means the non-living conditions of a place. These are measured from the terrain alone and
are the inputs every living or built layer is decided from; the science is geomorphometry. Each
row ends with what the measure stands in for. Slope, aspect, curvature, flow accumulation and
wetness are rows of the section's first table.

| Term | Meaning | Archetype | Whose | Now |
|---|---|---|---|---|
| altitude | height above a datum; stands in for temperature (lapse rate), tree line, snow line | Field | core | shown: `f()` |
| topographic position | height relative to the neighbourhood: ridge, mid-slope, valley floor; stands in for landform class | Derivation (focal) | core | none |
| occlusion / sky-view | how much of the sky surrounding terrain blocks; stands in for shelter, shade, retained moisture | Derivation (global) | core | none |
| illumination / insolation | sunlight received over a day or year, with terrain shadows; stands in for warmth, dryness, growing conditions | Derivation (global) | core | none |
| wind exposure | how open a location is to a prevailing wind; stands in for stunted growth, snow drift, rain shadow | Derivation (global) | core | none |
| latitude, longitude | position across the map used as a gradient; stands in for regional climate variation | Field | core | none |

## 3. Water

From general knowledge (2026-11-12), re-read against the first cut on 2026-11-18.

| Term | Meaning | Archetype | Whose | Now |
|---|---|---|---|---|
| sea level | the height below which there is sea | Field | core | shown: `examples/coast` |
| ocean, sea | open water to the horizon | Field | lib, engine | partly: `examples/coast` |
| shoreline, coast, beach | where land meets water and how the ground grades in | Meeting | core | shown: `where`, `derived.md 1`, `lib/land/ground.md` |
| lake, pond | still water held by the ground: a level surface filling a hollow | Derivation (global) | lib | shown: `water.lake`, `water.tarn`, `water.pond` |
| water depth, bathymetry | the ground under water, as a depth field | Derivation (local) | core | shown: `lib/water/kinds.toml`, `water.pond` |
| wade, swim, dive depths | the depths at which movement changes | Measure | core, game | partly: `lib/review/reach.py` |
| watershed, drainage basin, divide | the land draining to one outlet, and its edge | Area partition | lib | none |
| river network, tributary, confluence | rivers as a tree joining downstream | Network (directed) | lib | none |
| river, stream | water flowing downhill along a course: a curve that carves a bed and carries a water surface | Network (directed) | lib | shown: `water.beck`, `terrain.terrain_river` |
| channel, bed, bank | the trench a river runs in, and its sides | Profile | lib | shown: `water.beck` |
| meander, oxbow, braided channel | forms a river takes on flat ground | Shape | lib | none |
| floodplain, river terrace | flat ground a river made | Simulation | lib | none |
| delta, estuary, river mouth | where a river spreads into still water | Meeting | lib | none |
| spring, source | where a stream begins | Network (directed) | lib | none |
| ford, shallows | water shallow enough to cross | Meeting | lib, game | shown: `ways.ford` |
| rapids | a steep, rough reach of river | Network (directed) | lib | none |
| waterfall, plunge pool | where a river drops | Meeting | lib | partly: `water.weir` |
| flow map, current | direction and speed of water | Field | lib, game | none |
| wetland, marsh | ground at water level | Area partition | lib | none |
| changeable water level | a water body whose height is a world state | Overlay | core | none |
| ice, frozen lake, glacier | water in its solid states | Overlay | lib, engine | none |
| hazard liquid: lava, poison | a liquid surface that harms or slows | Markup | game, carries | none |
| cloud sea | a cloud sheet used as a false ground or sea | Field | lib, engine | none |
| quay, wharf, pier, seawall, dock, jetty | the built edge between land use and water | Meeting | lib | shown: `town.waterfront` |
| swim region | a region that counts as "in water" | Markup | loader, game | none |
| waves, tides, foam, caustics | how water moves and looks | Runtime | engine | none |
| underwater | the world seen and moved through below the surface | Runtime | engine, game | none |

### Networks and filling

Read from sources in 2026-11.

| Term | Meaning | Archetype | Whose | Now |
|---|---|---|---|---|
| Strahler order | stream rank that grows where equal tributaries merge | Network (directed) | lib | none |
| priority-flood | algorithm that fills depressions so every location drains (Barnes et al. 2014) | Derivation (global) | core | shown: `derived.md 4`, `lib/land/uplands.py` |
| river-first terrain | lay out the river network, then build the terrain around it (Génevaux et al. 2013) | Network (directed) | lib | shown: `lib/terrain/kinds.toml`, `examples/basin` |

## 4. Vegetation and ecology

From general knowledge (2026-11-12), re-read against the first cut on 2026-11-18. The
subsections were read in 2026-11 from the talk on *Far Cry 5*'s procedural world
(`docs/references.md`) and from forest ecology.

| Term | Meaning | Archetype | Whose | Now |
|---|---|---|---|---|
| vegetation, foliage | everything that grows | Scatter | lib | shown: `[scatter.<id>]`, `examples/glade` |
| climate fields: temperature, moisture, rainfall | the fields biomes are derived from | Field | core | partly: `[field.<name>]` |
| altitude band, tree line | growth changing in belts with height (montane, subalpine, alpine) until trees stop, where the growing season is too cold | Area partition | lib | shown: `density`, `examples/vale` |
| ecotone, biome transition | the mixed strip between two biomes: a gradual band, not a hard edge | Meeting | lib | none |
| scatter, procedural placement | many things placed by a rule | Scatter | core | shown: `[scatter.<id>]`, `rules.md 7` |
| density map, mask | where and how thick, as a field or painted picture | Field | core | shown: `density` |
| points and attributes | what placement rules pass along: positions with named values | Scatter | core | partly: `[scatter.<id>]` |
| minimum spacing, Poisson-disk sampling | keeping scattered things apart: random points with a least distance between them (blue noise) | Scatter | core | partly: `spacing`, `lib/vegetation/clumps.py` |
| clumping, clustering | things gathering in groups with gaps | Scatter | core | shown: `clump`, `examples/glade` |
| species mix, palette | which plants, in what share | Recipe | lib, pack | shown: `assets`, `weight` |
| per-species settings | a plant's density, draw distance and collision | Asset | pack, loader | none |
| growth stage, size variation | the same species young, mature and dying | Scatter | pack, lib | partly: `scale` |
| hero tree, specimen | one hand-placed plant that anchors a view | Placement | pack | shown: `[instance.<id>]` |
| canopy, forest edge, glade, meadow | the forest's cover and its openings | Area partition | lib | partly: `vegetation.wood`, `vegetation.clearing` |
| understory, undergrowth | what grows beneath and because of the trees | Scatter | lib | shown: `around`, `examples/basin` |
| riparian growth, reeds | plants that follow water: the band along it with its own moisture-loving growth | Scatter | lib | partly: `examples/glade` |
| leaf litter, forest floor | ground material caused by the trees above | Derivation (local) | lib | none |
| deadfall, snag, stump, log | dead wood standing or lying | Scatter | lib, pack | partly: `lib/packs/starter.toml` |
| disturbance: burn scar, clear-cut | growth removed by an event (fire, storm, logging, flood), which resets succession over a patch | Area partition | lib | partly: `vegetation.clearing` |
| ecosystem simulation | placement by what could survive there | Simulation | lib | none |
| ground cover, grass | growth too fine to place as objects | Scatter | loader, engine | partly: `examples/vale` |
| exclusion, clearing | growth kept off roads, buildings and paths | Derivation (local) | core | shown: `rules.md 7`, `vegetation.clearing` |
| rocks, boulders | loose stone placed like growth | Scatter | lib | shown: `examples/example` |
| scree | a slope deposit caused by the cliff above | Scatter | lib | shown: `examples/example` |
| interactive growth | cuttable, burnable, shakeable, bending underfoot | Runtime | pack, game | none |
| harvestable, resource node | a plant or rock the player can take from | Markup | game, carries | partly: `build.md 9` |
| region-exclusive resource | a gatherable found only in one region | Markup | game, carries | none |
| wind | what makes growth move; needs a world direction and per-asset facts | Runtime | engine, pack | none |
| seasons | the same world in another state | Overlay | core | shown: `[change.<name>]`, `examples/vale` |

### Biomes: classes of land

A biome is an area class. Two established patterns hold it: exactly one label per cell, or a
share of every biome per cell.

| Term | Meaning | Archetype | Whose | Now |
|---|---|---|---|---|
| biome map (boolean) | exactly one biome label per cell; exclusive, readable, easy to debug | Area partition | core | none |
| biome weights (gradient) | a share of every biome per cell, summing to one; smooth but ambiguous when several overlap | Field | core | partly: `type` |
| hardening | turning weights into labels by taking the strongest at each cell | Derivation (local) | core | shown: `lib/map/export.py` |
| main biome, sub-biome | a coarse painted class, refined by rule into finer classes within it (the talk on Far Cry 5 (?)) | Area partition | lib | none |
| rule classification | label chosen by thresholds on abiotic fields (Köppen climate classes, Whittaker) | Derivation (local) | lib | partly: `lib/land/ground.md` |
| nearest-seed partition (Voronoi) | each cell takes the label of its closest seed; borders roughened by warping | Area partition | core | none |
| parameter-space lookup | several noise fields give a cell's climate; it takes the biome nearest in that climate space | Derivation (local) | lib | none |
| majority filter | each cell takes the commonest label around it; removes specks, smooths borders | Derivation (focal) | core | none |
| border dithering | cells near a border pick a side at random by weight, so two classes intermix instead of meeting at a line | Meeting | core | none |
| biotic layer | the living result decided from abiotic data: which species grow where | Area partition | lib | none |
| viability | a per-species score over abiotic fields; the best-scoring species win a location | Derivation (local) | lib | partly: `density` |
| Whittaker diagram | biome chosen from temperature and rainfall | Derivation (local) | lib | none |
| biome recipe | the materials and species a biome expands into | Recipe | lib | none |

### Scatter: where and how each thing stands

Fields decide more than whether a thing is placed: its size, lean, aim and variant too. The
plant behaviour behind several rows is real: growth corrects towards vertical and towards light,
and creeping soil bends young trunks.

| Term | Meaning | Archetype | Whose | Now |
|---|---|---|---|---|
| attribute modulation | abiotic fields set not only whether a thing is placed but its size, density, age and variant | Scatter | core | partly: `density` |
| krummholz | stunted, twisted trees in the band just below the tree line | Scatter | lib | none |
| flagging | trees shaped one-sided by a prevailing wind | Scatter | lib | none |
| ecological ordering | large plants claim space first; smaller ones fill what remains | Scatter | core | none |
| distance field | distance from every location to the nearest feature (water, road, building) | Derivation (global) | core | shown: `dist()`, `sdist()` |
| direction field | a direction at every location, usually the downhill of a distance field: "towards the water" | Derivation (global) | core | none |
| instance orientation | each instance's rotation: random spin, or read from a direction field | Scatter | core | partly: `rules.md 7` |
| up alignment | whether an instance stands along world-up (trees) or the ground normal (rocks, low plants), or a blend | Scatter | pack | partly: `rules.md 6` |
| lean | a tilt away from upright along a direction field: towards light or water, away from wind | Scatter | core | none |
| variant selection | which model of a species is used at a location, chosen by a field (a bent trunk where it is steep) | Scatter | core | partly: `assets`, `weight` |
| basal sweep ("pistol butt") | a trunk curved at the base and upright above, the curve bowing downhill | Asset | pack | none |
| creep | slow downhill movement of soil or snowpack that tips young trees over | Simulation | lib | none |
| gravitropism | plants correct their growth to vertical, whatever the slope | Scatter | lib | none |
| phototropism | plants grow towards light, so growth at an edge leans into the open side | Scatter | lib | none |
| fog / atmosphere map | density derived from valleys and water, handed to rendering (?) | Derivation (global) | lib | none |

### Forest structure and time

The theory behind layered scatter: a forest is stacked layers, and a stand passes through stages
after each disturbance.

| Term | Meaning | Archetype | Whose | Now |
|---|---|---|---|---|
| vertical stratification | a forest is stacked layers: overstory, midstory, woody understory, herb layer | Scatter | lib | none |
| canopy closure | crowns meet and shade the ground; the layer above becomes a light mask for the layer below | Derivation (focal) | lib | partly: `vegetation.wood` |
| succession | a plant community changes in a predictable sequence over time after a reset | Simulation | lib | none |
| stand stages | initiation, stem exclusion, understory reinitiation, old growth (Oliver and Larson 1990) | Simulation | lib | none |
| self-thinning | as trees grow, crowding kills the weaker; fewer, larger stems remain | Simulation | lib | none |
| gap dynamics | a fallen tree opens a light gap where young growth restarts | Simulation | lib | none |
| stand age | time since the last disturbance; a per-patch value that selects density, size and layers | Field | lib | none |

## 5. Land use and farming

From general knowledge (2026-11-12), re-read against the first cut on 2026-11-18.

| Term | Meaning | Archetype | Whose | Now |
|---|---|---|---|---|
| land use, land cover | what each piece of land is used for | Area partition | core | none |
| field, field pattern, strip, furrow | farmland divided into worked pieces | Area partition | lib | partly: `examples/example` |
| field boundary: fence, dry-stone wall, hedgerow, ditch | what divides one field from the next | Linear placement | lib | partly: `[run.<id>]` |
| pasture, paddock, corral, pen | enclosed or open ground for stock | Area partition | lib | partly: `town.enclosure` |
| crop, orchard, row planting | growth planted in rows by people | Linear placement | lib | shown: `examples/example` |
| stile, cattle grid, field gate | where a way passes through a boundary | Meeting | lib | partly: `town.gate` |
