# World structure

Section 1 of the [ledger](README.md): what a world is made of above the level of any one
landscape. The row shape and the marks are explained in the README; the archetypes in
[archetypes.md](archetypes.md).

## 1. World structure and scale

From general knowledge (2026-11-12), re-read against the first cut on 2026-11-18.

| Term | Meaning | Archetype | Whose | Now |
|---|---|---|---|---|
| open world | one continuous map the player crosses freely; teleports and instances may still load | Guidance | game | none |
| region, zone, area | a named part of the world with its own character and rules | Area partition | core | partly: `map.area` |
| sub-area, locale, named place | a named spot inside a region; shows a title on entry | Markup | core | partly: `map.area` |
| biome | a kind of land: its ground, growth and climate together; chosen by rules over height, slope and moisture, a painted override winning | Area partition | lib | none |
| landmark | something seen from afar that the player steers by | Guidance | core | partly: `authority.md 3` |
| point of interest (POI) | a place worth going to, marked for map and design; what is around it is arranged to it | Markup | lib | shown: `map.poi` |
| vista, composition, focal point, reveal | a view arranged on purpose | Guidance | game | none |
| sightline, viewshed | what can be seen from a point | Measure | core | none |
| silhouette, skyline | the outline of land and landmarks against the sky | Guidance | core, game | none |
| world scale compression | distances shortened so travel stays interesting: a mountain minutes high, a capital of a few streets, proportions to the player kept | Guidance | game, core | none |
| map layer: underground, surface, sky | stacked playable strata over one map footprint: underground, surface, sky islands, underwater | Area partition | core | partly: `lib/map/export.py` |
| layered levels, stacked spaces | several walkable levels over one ground point | Representation switch | core | shown: `surfaces.md 1`, `examples/station` |
| instance, instanced dungeon, domain, sub-map | a separate map entered through a door, with its own load | Area partition | core, game | partly: `composition.md 2` |
| region gate, hazard barrier | what keeps the player out: a wall, a level, a deadly cold | Guidance | game, carries | none |
| world bounds, playable area | where the player may go, and what stops them | Markup | game, core | none |
| backdrop, distant land | what lies beyond the playable world | Delivery | loader | none |
| authoring tile, chunk | the unit we compile and hand over | Delivery | core | shown: `chunk`, `build.md 5` |
| runtime cell, streaming cell | the unit the engine loads; rarely the same as our tile | Delivery | engine, loader | partly: `engines/godot` |
| world streaming | loading and unloading parts of the world as the player moves | Runtime | engine, loader | shown: `engines/godot` |
| streaming source, loading range | the point cells load around, and how far | Runtime | loader, game | shown: `engines/godot` |
| always loaded versus streamed | whether a thing stays resident or comes and goes with its cell | Delivery | loader | none |
| cell straddler | a thing larger than a cell | Delivery | core, loader | partly: `build.md 5` |
| streaming hitch, late load | a stall or missing ground when loading lags the player | Runtime | engine, game | none |
| traversal speed budget | the fastest the player moves that loading can keep up with | Measure | game, core | none |
| loading corridor | a narrow passage that hides a load | Guidance | core, loader | none |
| placed level, level instance | a piece of world kept as its own file and placed in another | Placement | core | shown: `[import.<id>]`, `composition.md 2` |
| switchable set, data layer | a named set of things turned on or off together | Overlay | loader | none |
| world precision | keeping positions exact far from the origin | Runtime | engine | none |
| scale, units | how big a metre, a door and a character are | Principle | core, pack | partly: `numbers.md 1`, `review.bookmark` |

### Themed regions

What a themed-region open world is made of. Read on 2026-11-13 from the published world map of
one stylised open-world game (*Genshin Impact*) and its developers' GDC 2021 talk
(`docs/references.md`). The rows are general concepts, not a description of that map.

| Term | Meaning | Archetype | Whose | Now |
|---|---|---|---|---|
| themed region | a large area with one identity: its own biome, culture, palette, music and landmark; chosen by the author, not by climate | Area partition | lib | partly: `map.area` |
| region seam | the narrow band where two unrelated regions meet and must be made to pass for neighbours | Meeting | core | partly: `composition.md 6` |
| incremental world | regions are added over years next to finished ones, which must not change | Principle | core | partly: `composition.md 2` |
| world edge | how the playable area ends: open sea, a cliff wall, fog, or a soft margin of water round every landmass | Meeting | lib | none |
| connective sea | water as the filler between landmasses: a boundary, a route and empty space at once | Area partition | lib | partly: `examples/strait` |
| region landmark | one dominant form per region, visible from most of it, that the layout is arranged around | Guidance | game | none |
| portal | the entrance joining two spaces that are not physically continuous | Meeting | core | partly: `[import.<id>]`, `on` |
| culture kit | one region's set of building parts, props and motifs; the same town structure dressed differently per region | Asset | pack | partly: `pack`, `assets.md 7` |
| illustrated map | the in-game map as a drawn picture derived from the world, not a top-down photo of it | Delivery | game | partly: `lib/map/export.py` |
| barrier seam | two regions separated by a line of rock or cliff, so the change happens out of sight | Meeting | lib | partly: `examples/border` |
| gradient seam | two regions blended over a short band of mixed ground | Meeting | core | shown: `blend`, `examples/border` |

### Read from close views of a stylised map

Read from four close views of the same map. These rows rest on reading pictures, not on a source
that names them.

**Terrain as a play surface**

| Term | Meaning | Archetype | Whose | Now |
|---|---|---|---|---|
| shelf terrain | ground built as near-flat shelves separated by cliff bands, not as a continuous slope; shelves are for walking, bands for climbing | Area partition | lib | none |
| cliff band | the steep face between two shelves; its height is a traversal cost | Profile | lib | partly: `land.cliff` |
| setpiece landform | a unique, fully specified form at the scale of a district: a spiral pit, a crater, a radial lava dome | Placement | lib | none |
| filler terrain | the ordinary ground between setpieces that only has to be plausible and walkable | Field | lib | partly: `lib/land/uplands.py` |
| empty expanse | a deliberately bare area (sand sea, open water) that makes what stands in it read from far away | Guidance | game | none |
| pocket | a small rich spot inside an expanse: an oasis, an islet, a hot spring | Placement | lib | none |
| basin | ground stepping down on all sides to a centre, which holds the place that matters | Guidance | game | none |
| shore type | beach, cliff shore or marsh edge; decides where land and water may be crossed | Meeting | lib | none |
| shallows | the lighter band of water near land: wadeable, and a visual margin | Profile | lib | none |
| interleaved wetland | land and water in fine alternation: delta, marsh, stilt ground | Area partition | lib | none |
| isthmus, causeway | a single narrow land or built link to an otherwise separate mass | Network (undirected) | lib | none |

**Settlement and ruin form**

| Term | Meaning | Archetype | Whose | Now |
|---|---|---|---|---|
| enclosed city | a settlement with a hard boundary (wall, island shore) and counted entrances | Area partition | lib | partly: `town.enclosure`, `town.gate` |
| approach | the one staged route into a place: road junction, bridge, gate | Guidance | lib | partly: `land.approach` |
| ascending axis | the main street climbs from gate through square to the chief building at the highest point | Guidance | lib | none |
| precinct | a walled or raised part of a city given to one function: cathedral, palace, harbour | Area partition | lib | partly: `town.enclosure` |
| hamlet | a handful of buildings beside a road, with no streets of its own | Recipe | lib | partly: `examples/example` |
| monument complex | a dominant structure with lesser ones laid out on its axes | Grammar | lib | none |
| ruin field | fragments scattered around a core, thinning with distance | Scatter | lib | none |
| radial settlement | buildings arranged around a centre, often on a basin's terraces | Grammar | lib | none |

**Paths**

| Term | Meaning | Archetype | Whose | Now |
|---|---|---|---|---|
| sparse path tree | a few trunk paths with spurs; little choice of route, almost no loops | Network (undirected) | lib | none |
| spur | a dead-end branch that exists to reach one place | Shape | lib | partly: `ways.trail` |
| path-free ground | areas reached only by climbing, gliding or swimming, with no path drawn at all | Guidance | game | none |

**Map drawing**

| Term | Meaning | Archetype | Whose | Now |
|---|---|---|---|---|
| height tint | the map coloured by elevation or depth band | Derivation (local) | lib | none |
| edge line | the map's drawn outline along each cliff band, which is what makes shelves readable | Derivation (global) | lib | none |

### A stylised reading: what stands in place of what

A new genre is read as a table of substitutions over the same archetypes. This one is from eight
screenshots of the same game: how each realistic concept is restated when the aim is a readable
picture, not a plausible landscape. Each meaning ends with the realistic concept it stands in
place of.

| Term | Meaning | Archetype | Whose | Now |
|---|---|---|---|---|
| top-and-side rule | flat tops are grass, steep sides are rock, with a crisp line between; nothing else decides ground cover; in place of abiotic data and viability | Area partition | lib | partly: `lib/land/ground.md` |
| undercut shelf | a shelf that overhangs its own base, or floats free; in place of cliff as a steep heightfield region | Representation switch | lib | none |
| faceted rock | cliffs built from large flat planes with hard edges, as if chiselled; in place of erosion detail | Asset | pack | none |
| painted strata | rock layering shown as broad colour bands on the faces; in place of strata function and band displacement | Derivation (local) | lib | none |
| accent tree | a few trees placed singly or in small clumps where they help the picture, often on a shelf's edge; in place of density-driven scatter | Placement | lib | shown: `[instance.<id>]` |
| canopy colour code | tree colour (orange, yellow, teal) chosen to identify a region or mark a place; in place of species by ecology | Recipe | lib | partly: `tint` |
| blade grass, ground-tinted | tall soft grass that takes the ground's colour, so fields read as one surface; in place of layered understory | Scatter | loader | none |
| flat water with depth ramp | water coloured by depth from pale shallows to deep blue, with a light line at the shore; in place of flow and wetness simulation | Derivation (local) | loader | none |
| stepped water | each shelf holds a level pool; water passes between shelves only as a waterfall; in place of rivers obeying continuous downhill flow | Network (directed) | lib | partly: `water.pond`, `water.weir` |
| soft trail | a dirt path with no edge, fading into grass; in place of road cross-section | Profile | lib | shown: `ways.trail` |
| building shelf | each building or group stands on its own flat shelf, held by a retaining wall where needed; in place of graded pad | Profile | lib | shown: `land.pad`, `town.terrace` |
| connector | the built way across a cliff band or gap: stairs, ramp, bridge, rope bridge, ladder; in place of road grade limits | Meeting | lib | partly: `building.flight`, `ways.ramp` |
| stacked city | a city climbing shelves, districts one above another, joined by connectors; in place of blocks and lots on a street grid | Area partition | lib | none |
| niche siting | a settlement set in a recess between cliffs, which serve as its walls and its backdrop; in place of siting by trade and defence | Guidance | game | none |
| rock within town | natural outcrops left standing between buildings and built around; in place of cleared, levelled sites | Placement | lib | none |
| terraced field | farmland as stepped plots with stone edges, each filled with one crop colour; in place of field systems on graded land | Area partition | lib | none |
| skyline landmark | tall thin forms (spire, windmill, tower) that rise above the settlement's outline to identify it from afar; in place of landmark by size alone | Guidance | game | none |
| beacon | a light or marker visible across the region that points to a destination; in place of map marker | Guidance | game | none |
| palette by time | the whole scene re-coloured through the day; shapes stay, colours swap; in place of physically based lighting | Runtime | engine | none |
