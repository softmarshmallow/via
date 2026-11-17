# Atmosphere, delivery and production

Sections 14 to 16 and 18 of the [ledger](README.md). The row shape and the marks are explained
in the README; the archetypes in [archetypes.md](archetypes.md).

## 14. Atmosphere, light and sound

From general knowledge (2026-11-12), re-read against the first cut on 2026-11-18.

| Term | Meaning | Archetype | Whose | Now |
|---|---|---|---|---|
| time of day | the sun's place and colour through a cycle; also gates content | Runtime | engine, game | none |
| weather | rain, snow, storm; also changes ground and mechanics | Runtime | engine, game, carries | none |
| weather zone, local climate | weather fixed or weighted per area | Markup | carries, game | none |
| temperature zone | an area that drains the player without protection | Markup | game, carries | none |
| sky, clouds, cloud shadows | the dome above, and its moving shade | Runtime | engine | none |
| fog, haze, aerial perspective | air that is seen; far things fading to its colour | Runtime | engine | none |
| dynamic bounced light | bounced light computed live, with a moving sun | Runtime | engine | none |
| baked light | light computed ahead; rare with a moving sun | Runtime | engine, pack | none |
| light and reflection samples | probes placed in the world | Delivery | loader | none |
| placed light | a lamp that is a light | Placement | lib, loader | none |
| street lighting, lamp spacing | lamps placed for even night coverage | Linear placement | lib | partly: `[run.<id>]` |
| night readability | moon, stars and exposure keeping the dark world legible | Runtime | engine | none |
| shadow method, shadow distance | how far and how sharply shadows are drawn | Runtime | engine | none |
| rain shelter | where rain and snow do not reach | Derivation (focal) | loader, engine | none |
| ambient particles | dust, pollen, leaves, embers, fireflies | Runtime | loader, game | none |
| picture-change region | a region that changes the final image | Markup | game, loader | none |
| colour script, regional palette | the planned colour and mood of each region | Guidance | game | none |
| art style | the style everything shares; what art direction decides | Practice | game | none |

### A stylised sky stored as masks

How a stylised world stores an authored look as data the engine can relight, shown for clouds
(the *Genshin Impact* GDC 2021 talk, `docs/references.md`).

| Term | Meaning | Archetype | Whose | Now |
|---|---|---|---|---|
| billboard | a flat picture placed in the world that always faces the viewer | Asset | pack, engine | none |
| sprite sheet / flipbook | a grid of variants or animation frames stored in one image | Asset | pack | none |
| mask, not picture | the artist paints where light, shadow and edge fall; the engine supplies the colours at run time | Asset | pack, engine | none |
| shading mask | painted light and dark areas of a form | Asset | pack | none |
| rim mask | the outline that catches light from behind | Asset | pack | none |
| threshold gradient | a painted ramp; raising a cut-off across it makes a shape grow or dissolve | Asset | pack | none |
| volumetric cloud | the realistic alternative: clouds as a simulated density in space, lit physically | Field | engine | none |

## 15. Seeing far and running fast

From general knowledge (2026-11-12), re-read against the first cut on 2026-11-18.

| Term | Meaning | Archetype | Whose | Now |
|---|---|---|---|---|
| instancing | drawing many copies of one mesh as one | Delivery | loader | shown: `build.md 8`, `engines/godot` |
| batching | merging different things into fewer draws | Delivery | loader, engine | none |
| merged far proxy | many far objects replaced by one simple mesh | Delivery | engine, loader | none |
| far proxy build | the offline step that makes those | Delivery | loader, engine | none |
| always-visible landmark | a far landmark kept beyond streaming range | Delivery | loader, pack | partly: `engines/godot` |
| far terrain | simplified ground beyond streaming range | Delivery | loader, engine | none |
| far lights, city glow | distant lights faked cheaply | Delivery | loader, engine | none |
| automatic geometric detail | detail chosen per cluster, replacing hand-made copies | Runtime | engine, pack | none |
| culling: view, occlusion, distance | not drawing what is not seen | Runtime | engine | none |
| cull distance, fade range | where a thing stops being drawn | Delivery | pack, loader | none |
| detail switch size | when each simpler copy takes over | Delivery | pack, loader | none |
| pop-in | a thing appearing suddenly, from loading or a detail switch | Runtime | engine, loader | none |
| overdraw | the cost of many see-through layers, chiefly growth | Runtime | pack, engine | none |
| texture memory, blurry textures | what happens when textures exceed their budget | Runtime | engine | none |
| shader stutter | a hitch when a material is first drawn | Runtime | engine | none |
| interior streaming, rooms and portals | loading and hiding interiors apart from the street | Delivery | loader, engine | none |
| teleport preload | world loaded and solid before the player lands | Runtime | loader | none |
| region bundle | assets shipped per region and fetched when needed | Delivery | pack, loader | none |
| quality tier, platform profile | the same world at different costs per machine | Delivery | game, loader | none |
| budget | allowed instances, triangles, draws and memory | Measure | core, engine | shown: `[budget]` |
| profiling | measuring frame time, memory and loads against budget | Measure | loader, engine | partly: `engines/godot` |
| runtime virtual texturing | terrain material cached so objects can blend into it | Runtime | engine | none |

## 16. The player's map

From general knowledge (2026-11-12), re-read against the first cut on 2026-11-18.

| Term | Meaning | Archetype | Whose | Now |
|---|---|---|---|---|
| minimap | the small map in the corner | Delivery | core, game | shown: `lib/map/export.py`, `build.md 9` |
| world map | the full map screen | Delivery | core, game | partly: `lib/map/export.py` |
| map marker, icon | what is shown on it | Markup | core | shown: `map.poi` |
| place name, label | names of places, as translatable data | Markup | core | partly: `map.area` |
| contours, relief shading | how the map shows the shape of land | Derivation (focal) | core | none |
| map reveal region | the piece of map one unlock uncovers | Markup | core | none |
| layered map | a separate map per vertical layer | Delivery | core | partly: `lib/map/export.py` |
| floor map | one map per storey indoors | Delivery | core | shown: `lib/map/export.py` |
| exploration progress | share of a region's finds already taken | Runtime | core, game | none |
| search area | a fuzzy region in place of an exact marker | Markup | game | none |
| detector, compass | a pointer to a goal or the nearest find | Runtime | game, core | none |

## 18. Making it: pipeline and people

From general knowledge (2026-11-12), re-read against the first cut on 2026-11-18.

| Term | Meaning | Archetype | Whose | Now |
|---|---|---|---|---|
| source of truth | the one place a fact is edited | Principle | core | shown: `documents.md 1` |
| hand-authored, procedural, hybrid | made by a person, by a rule, or both | Principle | core | shown: `authority.md 3` |
| non-destructive | an edit that does not bake away what came before: change an input and what follows is made again | Principle | core | shown: `graph.md 7` |
| override, pin, lock | a hand decision that survives regeneration and beats the rule | Principle | core | shown: `[pin.<id>]`, `[clear.<id>]`, `[lock.<id>]`, `identity.md 4` |
| seed, determinism | the same input giving the same world | Principle | core | shown: `seed`, `numbers.md 9` |
| variant | the same world in another condition | Overlay | core | shown: `[change.<name>]` |
| layer | a separately owned document of one world | Overlay | core | shown: `[layer.<id>]` |
| merge, conflict, file locking | two people's changes meeting; unmergeable files held by one | Practice | core | partly: `documents.md 5` |
| diff, blast radius | what a change touched | Measure | core | shown: `identity.md 5` |
| validation, lint | automatic checks on the world | Measure | core | shown: `checks`, `lib/review/checks.py` |
| asset validation | checks on assets when saved or submitted | Measure | pack, loader | shown: `assets.md 8` |
| review, capture | looking at a change before accepting it | Practice | core, loader | shown: `review.bookmark`, `engines/godot/sheet.sh` |
| automated fly-through, nightly build | a scripted flight that catches breakage | Practice | core, loader | partly: `engines/godot/walk.gd` |
| headless build | running the build with no editor open | Practice | core, loader | shown: `build.md 14` |
| compile, bake, cook, package | make the world; precompute light and navigation; convert per platform; bundle | Delivery | core, engine | partly: `build.md 2` |
| shared build cache | built results shared so not everyone rebuilds | Delivery | core, engine | none |
| references, renames | how a moved or renamed asset keeps its users | Principle | engine, loader | none |
| unused assets, dependency size | finding what nothing uses and what costs most | Measure | pack | shown: `assets.md 8` |
| editor-only versus shipped data | markup that helps authoring and must not ship | Delivery | core, loader | none |
| editor extension | how our loader is delivered into each engine | Delivery | loader | partly: `engines/godot/addons` |
| generator, step | user code that makes world | Practice | core | shown: `[step.<id>]`, `protocol.md 1` |
| placement graph | a node graph of placement rules, in the engine or a tool | Practice | engine, ? | none |
| geometry along a line | a mesh swept or lofted along a path: a cross-section carried along a curve | Profile | core | partly: `build.md 10` |
| real-world data | measured terrain and land cover used as input | Field | core | partly: `fields.md 4` |
| reference, mood board | the real places a region is modelled on | Practice | game | none |
| import, migration | bringing an existing world in | Practice | ? | none |
| tiled | tiles compute independently and meet without seams | Principle | core | shown: `chunk`, `build.md 5` |
| resolution independence | defined in world units; erosion breaks this in practice | Principle | core | partly: `numbers.md 4` |

### The causal order of a landscape

Each stage consumes masks the earlier ones emit.

1. World design: points of interest, critical path, blockout.
2. Terrain: base landform.
3. Terrain analysis: abiotic data.
4. Water.
5. Roads.
6. Blocks and lots.
7. Streetscape.
8. Buildings.
9. Cliffs, biomes, scatter.
10. Atmosphere and other derived consumers.
11. Engine delivery.

Height writers: terrain, water, roads, pads. Everything else only reads height.

Surface writers: almost everything. Biomes, scatter, roads and water all write
the ground's material back after they are placed.

### Tool mental models

What each tool's design says a world is. Read in 2026-11 for the model, not for ease of use.

| Tool | Central idea |
|---|---|
| World Machine, Gaea | A graph over one type (heightfield plus masks); nodes named after landforms and processes; preview at low resolution, build at high. |
| World Creator, Unreal edit layers | A layer stack; order is the only structure. |
| Houdini | A graph over general attribute-carrying geometry; reusable assets (HDA), batch cooking per tile (PDG). |
| CityEngine | A grammar; a lot is rewritten into a building. |
| Unreal PCG | Points with attributes flowing through filters. |

#### How tools treat roads

Three stances a tool can take towards a domain concept (all from memory (?)):

| Stance | Meaning |
|---|---|
| Built-in concept | The tool knows what a road is: lanes, junctions and markings exist as such. |
| Skeleton plus rules | The tool owns the network, detects junctions and hands out typed pieces; the user's rules decide what each piece becomes. |
| Primitive toolkit | The tool knows only curves, sweeps and copies; a road is something the user builds and packages. |

| Tool | Stance | What it actually gives |
|---|---|---|
| Houdini | Primitive toolkit | Curves with attributes, sweep, copy-to-points, terrain projection. Road tools are packaged assets; SideFX Labs ships some as examples, not as core. |
| Blender geometry nodes | Primitive toolkit | The same idea, smaller. |
| Unreal landscape splines | Thin built-in | Points and segments with width and falloff that reshape and paint terrain and carry meshes along. No lanes, markings or junction logic. |
| Unreal PCG, Unity splines | Primitive toolkit | Sample a curve, place things. Full road tools are third-party add-ons. |
| CityEngine | Skeleton plus rules | A street graph with widths; it generates typed shapes (street, sidewalk, junction, crossing, roundabout) and runs user grammar rules on each. |
| Civil design software | Built-in, extensible | Alignment, profile, assembly, corridor; the cross-section parts are programmable. |
| RoadRunner | Built-in | Lanes, markings, junctions, signals, bridges; exports to the driving-simulation standard. |
| City-builder games | Built-in | A road type is a packaged asset: lanes, props, node and segment pieces. |
| World Machine, Gaea | None | A road can only be a mask. |
