# References

Prior work, standards and tools worth studying, with one line on why each matters to this project.
We study these to challenge our design, not to mirror them.

Links were written from memory on 2026-11-11 and have not been re-checked. Verify a link before
citing it anywhere else. Add to this list freely; keep each entry to a name, a link and a reason.

## World building and procedural environments

- [Houdini](https://www.sidefx.com/products/houdini/world-building/): procedural terrain, vegetation,
  roads, settlements and reusable world-building tools. Authority there is the order of nodes.
- [Unreal Engine PCG](https://dev.epicgames.com/documentation/en-us/unreal-engine/procedural-content-generation-framework-in-unreal-engine):
  rule-driven environment assembly, from individual assets to entire worlds.
- [Unreal PCG Biome Core](https://dev.epicgames.com/documentation/en-us/unreal-engine/procedural-content-generation-pcg-biome-core-and-sample-plugins-quick-start-guide-in-unreal-engine):
  data-driven biome definitions, assets and generators; experimental plugin.
- [Dungeon Architect](https://docs.dungeonarchitect.dev/unreal/start-here/how-it-works/): separates
  spatial layouts from visual themes and asset dressing. Close to our world-versus-pack split.
- [Gaia](https://www.procedural-worlds.com/products/indie/gaia/): Unity environment creation using
  terrain, biomes and asset spawning.
- [GeNa Pro](https://www.procedural-worlds.com/products/professional/gena-pro/): Unity procedural
  level dressing and environment placement.
- [ArcGIS CityEngine / CGA](https://doc.arcgis.com/en/cityengine/latest/help/help-cga-modeling-overview.htm):
  rule-based architectural and urban generation. Has both "align terrain to shapes" and "align shapes
  to terrain", the two directions we call authority.
- [Frostbite terrain tools](https://www.ea.com/ea-play/news/procedural-terrain-in-ea-sports-pga-tour):
  studio reference for interconnected terrain, material and placement rules; internal tooling.
- [Scenic](https://docs.scenic-lang.org/en/3.x/tutorials/fundamentals.html): textual spatial
  relationships, distributions and constraints, with hard and soft requirements.
- [Infinigen](https://infinigen.org/): open-source procedural natural environments on Blender, and
  an indoor variant with a constraint language for room and furniture layout.
- [Scenethesis](https://arxiv.org/abs/2507.18625): constraint-aware scene language for agent-driven
  generation and editing.
- [Holodeck 2.0](https://arxiv.org/abs/2508.05899): language-guided world generation and iterative
  editing.
- [Terrain3D](https://github.com/TokisanGames/Terrain3D): Godot terrain and foliage backend; the kind
  of infrastructure that sits beneath an authoring framework.

## Scene description and composition

Relevant to overlays, variants, parts and the handoff to an engine.

- [OpenUSD](https://openusd.org/): layered scene description with references, payloads, variants and
  instancing. The most developed answer to "compose one scene from many files with overrides", which
  is what our overlays and parts are reaching for.
- [glTF](https://www.khronos.org/gltf/): the asset interchange format our packs already read. Its
  extension mechanism is a model for carrying declared facts beside geometry.
- [Unreal level instances](https://dev.epicgames.com/documentation/en-us/unreal-engine/level-instancing-in-unreal-engine):
  a level placed inside another level as a unit. Prior art for parts.

## Buildings and interiors

### Standards and their vocabulary

- [IFC (Industry Foundation Classes)](https://www.buildingsmart.org/standards/bsi-standards/industry-foundation-classes/),
  ISO 16739: the building-information standard. Defines storey, space, wall, slab, door, window,
  stair, opening and the relations between them. Study the vocabulary and the relations (what
  contains what, what bounds what, what voids what) before naming anything ourselves.
- [IFC 4.3 documentation](https://ifc43-docs.standards.buildingsmart.org/): the browsable schema.
- [IfcOpenShell](https://ifcopenshell.org/) and [Bonsai](https://bonsaibim.org/): open tooling that
  reads and authors IFC. A way to see the model in use without commercial software.
- [CityGML](https://www.ogc.org/standards/citygml/) and [CityJSON](https://www.cityjson.org/):
  buildings at levels of detail, from a footprint to full interiors. Relevant to the boundary
  between a building's footprint on our map and what is inside it.
- [IndoorGML](https://www.ogc.org/standards/indoorgml/): indoor space as cells and a connectivity
  graph, made for navigation. Directly relevant to reachability indoors.

### Game practice

- [Skyrim's modular level design](http://blog.joelburgess.com/2013/04/skyrims-modular-level-design-gdc-2013.html)
  (Burgess and Purkeypile, GDC 2013): how kits, grids, footprints and pivots are agreed so pieces
  snap. The clearest account of the kitbash contract.
- [Godot GridMap](https://docs.godotengine.org/en/stable/tutorials/3d/using_gridmaps.html): placing
  mesh-library pieces on a 3D grid. The simplest engine-side kit target.
- [TrenchBroom](https://trenchbroom.github.io/) and the Quake `.map` format: levels as plain text
  brushes and entities, in use for decades. Prior art for a text-first level source.
- [LDtk](https://ldtk.io/) and [Tiled](https://www.mapeditor.org/): 2D level editors whose files carry
  typed entity definitions, enums and layers. Useful for how they declare user-defined data.

### Research

- Müller et al., "Procedural Modeling of Buildings" (SIGGRAPH 2006): shape grammars for facades and
  massing; the origin of CGA.
- Merrell et al., "Interactive Furniture Layout Using Interior Design Guidelines" (SIGGRAPH 2011) and
  Yu et al., "Make it Home" (SIGGRAPH 2011): furniture arrangement as optimisation over clearance,
  circulation and grouping rules. Sources for dressing rules that are not invented by us.
- [ProcTHOR](https://procthor.allenai.org/): procedural houses from room specifications down to
  placed objects, at scale.
- [WaveFunctionCollapse](https://github.com/mxgmn/WaveFunctionCollapse): assembling tiles from
  adjacency constraints. Relevant to what a kit's sockets must declare.

### Dimensions

- Neufert, "Architects' Data": standard room, door, stair and furniture dimensions.
- The stair rule of thumb, twice the riser plus the going is about 63 cm (Blondel): the source for a
  "stair too steep" check, in place of thresholds we make up.

## Navigation and reachability

- [Recast and Detour](https://github.com/recastnavigation/recastnavigation): the standard navigation
  mesh builder. Its agent parameters (radius, height, maximum slope, maximum climb) are exactly a
  movement profile, and its voxel approach handles overhangs and multiple floors, which our
  top-surface reach check does not.
- Space syntax: analysis of how rooms and streets connect, as graphs. Background for measuring a
  layout's connectivity and depth from an entrance.

## Player-facing maps (minimap and world map)

From a one-pass study on 2026-11-11; the full report is `docs/research/Game map production pipelines.md`.
Entries marked (unread) were seen only in search summaries.

- [Breath of the Wild UI session, CEDEC 2017, English summary](https://gist.github.com/idbrii/e39fe96279aa1670319bfa521d907399):
  the map was procedurally generated and rebuilt every night, 120 sections at 4 zoom levels, 2,344
  screens, because two UI designers could not maintain it by hand. The strongest precedent for
  deriving the player map from level data. Third-hand.
- [REDkit: how to generate minimaps](https://cdprojektred.atlassian.net/wiki/spaces/W3REDkit/pages/51904544/HOW-TO+Generate+minimaps)
  (unread): The Witcher 3's minimap is captured per world tile by an editor tool, then adjusted by
  hand. Reportedly builds interior maps from navigation masks.
- [Cartographers review the Breath of the Wild map](https://stamen.com/cartographers-play-video-games-a-review-of-the-map-in-the-legend-of-zelda-breath-of-the-wild/)
  and [the Elden Ring map](https://stamen.com/cartographers-play-video-games-a-review-of-the-map-in-elden-ring/):
  what a good game map shows, judged by mapmakers.
- Devilution `automap.cpp` (the reconstructed Diablo source): the automap is a boolean grid plus a
  per-tileset table from tile to wall glyph, so generated levels need no map asset.
- Dungeon Architect's Dungeon Canvas (docs.dungeonarchitect.dev): a minimap drawn from the layout
  data through styled layers, with fog of war and icons. The one out-of-the-box system that styles
  a map from data, limited to its own dungeons.
- Valve's Dota 2 workshop tools (unread): the minimap is a top-down capture between two boundary
  entities, then edited by hand; regenerating overwrites the edits.
- [Affolter et al., "Generative AI in Map-Making"](https://arxiv.org/abs/2508.18959): the one
  rigorous study of image models conditioned on map data. Labels come out as gibberish unless text
  is masked and added separately; alignment, blur and tile seams are open problems.

## City building and asset kits

From a one-pass study on 2026-11-12; the full report is `docs/research/Modular city asset contracts.md`,
which marks what was read, seen only in summaries, or recalled.

- Burgess and Purkeypile on Skyrim's kits (listed above) and the Fallout 4 follow-up (GDC 2016):
  the only source with an explicit kit contract. Footprints that are multiples of each other, snap
  at half the footprint, one pivot rule, names that encode kit, role and variant, and loopback,
  stack and gap tests.
- [glTF 2.0](https://www.khronos.org/gltf/): the one format that fixes up (+Y), front (+Z) and
  unit (metres). Everything else about a kit piece is convention.
- Unreal's PCG shape grammar: a module is a symbol, a size and a scalable flag; the grammar fills
  one dimension. Evidence that a one-dimensional pattern plus stacking covers fences and facades.
- Watch Dogs 2 (80.lv interview with an environment artist): each edge of a building footprint
  spawns a "kit recipe", and road splines carry prop recipes. The closest studio practice to our
  kit runs.
- Ghost Recon Wildlands (80.lv): settlements generated in Houdini, hand edits lost on
  regeneration unless the region is locked first.
- [CityJSON](https://www.cityjson.org/) and OpenStreetMap's Simple 3D Buildings: the shared
  vocabulary for building, part, storey, roof and opening. Neither has block, lot or frontage;
  those come from Parish and Müller, "Procedural Modeling of Cities" (2001).

## Engine and SDK setup

- [Grida FX](https://github.com/gridaco/fx): a different tool (a workflow engine for generated
  assets), studied on 2026-11-12 for its setup only. A Rust engine with Python and TypeScript SDKs
  that never link the engine: each SDK ships the binary, drives it as a command, and hosts user
  code in a separate process the engine talks to over JSON-RPC on stdio. A language-neutral `spec/`
  with schemas and test vectors is normative, and conformance cases test only through the command
  line. Lessons: all engine logic stays in the engine; no Windows support because of Unix process
  handling; byte-identical output broke on a Python imaging library across platforms.

## Measurements

- Shoreline development index, D = L / (2 * sqrt(pi * A)), from limnology (Hutchinson, "A Treatise on
  Limnology", 1957). Used by the spike to say how intricate a lake's shore is.

## Still to find

- A standard or common practice for vegetation and plant modelling (species, growth forms, placement
  ecology), for the vegetation domain.
- References for hydrology and erosion as producers.
- How studios describe movement profiles and traversal rules for level validation.
