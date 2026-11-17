# References

Prior work, standards and tools worth studying, with one line on why each matters to this project.
We study these to challenge our design, not to mirror them.

Links in the earlier sections were written from memory on 2026-11-11 and have not been re-checked:
verify one before citing it anywhere else. A section added later says when and how it was checked.
Add to this list freely; keep each entry to a name, a link and a reason.

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

## Sources for the archetypes

What `docs/ledger/archetypes.md` and the ledger's rows cite by author and year. Checked on
2026-11-18 against each publisher's record: the page itself, or the record deposited for the DOI
where the page refused a script. A claim checked only on a secondary page says so.

Fields, shapes and derivation:

- Couclelis, H. (1992). People manipulate objects (but cultivate fields): beyond the raster-vector
  debate in GIS. In *Theories and Methods of Spatio-Temporal Reasoning in Geographic Space*, LNCS
  639, Springer, 65-77. https://doi.org/10.1007/3-540-55966-3_3 : the object view and the field
  view, the ledger's two families of data type.
- Tomlin, C. D. (1990). *Geographic Information Systems and Cartographic Modeling*. Prentice Hall.
  https://archive.org/details/geographicinform00toml : map algebra, whose local, focal and zonal
  operations are three of Derivation's four classes. The global class is a later addition; one
  textbook credits it to O'Sullivan and Unwin (2010).
- O'Callaghan, J. F. and Mark, D. M. (1984). The extraction of drainage networks from digital
  elevation data. *Computer Vision, Graphics, and Image Processing* 28(3), 323-344.
  https://doi.org/10.1016/S0734-189X(84)80011-0 : flow routed downhill, a global derivation.
- Barnes, R., Lehman, C. and Mulla, D. (2014). Priority-flood: an optimal depression-filling and
  watershed-labeling algorithm for digital elevation models. *Computers & Geosciences* 62, 117-127.
  https://doi.org/10.1016/j.cageo.2013.04.024 : filling a hollow until it spills.
- Strahler, A. N. (1957). Quantitative analysis of watershed geomorphology. *Transactions, American
  Geophysical Union* 38(6), 913-920. https://doi.org/10.1029/TR038i006p00913 : stream order.

Terrain:

- Galin, E., Guérin, E., Peytavie, A., Cordonnier, G., Cani, M.-P., Benes, B. and Gain, J. (2019).
  A Review of Digital Terrain Modeling. *Computer Graphics Forum* 38(2), 553-577.
  https://doi.org/10.1111/cgf.13657 : the families of terrain method (procedural, simulation,
  example-based); "feature-based construction" is its name for building terrain from features.
- Musgrave, F. K., Kolb, C. E. and Mace, R. S. (1989). The synthesis and rendering of eroded fractal
  terrains. *Proc. SIGGRAPH '89*, 41-50. https://doi.org/10.1145/74333.74337 : noise terrain and
  erosion as simulation.
- Mei, X., Decaudin, P. and Hu, B.-G. (2007). Fast hydraulic erosion simulation and visualization on
  GPU. *Proc. Pacific Graphics 2007*, 47-56. https://doi.org/10.1109/PG.2007.15
- Cordonnier, G., Braun, J., Cani, M.-P., Benes, B., Galin, É., Peytavie, A. and Guérin, É. (2016).
  Large scale terrain generation from tectonic uplift and fluvial erosion. *Computer Graphics Forum*
  35(2), 165-175. https://doi.org/10.1111/cgf.12820
- Génevaux, J.-D., Galin, É., Guérin, E., Peytavie, A. and Benes, B. (2013). Terrain generation using
  procedural models based on hydrology. *ACM Transactions on Graphics* 32(4).
  https://doi.org/10.1145/2461912.2461996 : the river network laid out first, the terrain built
  round it.
- Génevaux, J.-D., Galin, E., Peytavie, A., Guérin, E., Briquet, C., Grosbellet, F. and Benes, B.
  (2015). Terrain modelling from feature primitives. *Computer Graphics Forum* 34(6), 198-210.
  https://doi.org/10.1111/cgf.12530 : a construction tree of primitives and operators.
- Hnaidi, H., Guérin, E., Akkouche, S., Peytavie, A. and Galin, E. (2010). Feature based terrain
  generation using diffusion equation. *Computer Graphics Forum* 29(7), 2179-2186.
  https://doi.org/10.1111/j.1467-8659.2010.01806.x : feature curves with side profiles.
- Peytavie, A., Galin, E., Grosjean, J. and Mérillou, S. (2009). Arches: a framework for modeling
  complex terrains. *Computer Graphics Forum* 28(2), 457-467.
  https://doi.org/10.1111/j.1467-8659.2009.01385.x : a grid of material stacks that may hold air,
  so rock can stand over a gap.
- Guérin, E., Digne, J., Galin, E. and Peytavie, A. (2016). Sparse representation of terrains for
  procedural modeling. *Computer Graphics Forum* 35(2), 177-187. https://doi.org/10.1111/cgf.12821
- Zhou, H., Sun, J., Turk, G. and Rehg, J. M. (2007). Terrain synthesis from digital elevation
  models. *IEEE Transactions on Visualization and Computer Graphics* 13(4), 834-848.
  https://doi.org/10.1109/TVCG.2007.1027
- Guérin, É., Digne, J., Galin, É., Peytavie, A., Wolf, C., Benes, B. and Martinez, B. (2017).
  Interactive example-based terrain authoring with conditional generative adversarial networks.
  *ACM Transactions on Graphics* 36(6), article 228. https://doi.org/10.1145/3130800.3130804
- Lorensen, W. E. and Cline, H. E. (1987). Marching cubes: a high resolution 3D surface construction
  algorithm. *Proc. SIGGRAPH '87*, 163-169. https://doi.org/10.1145/37401.37422
- Ju, T., Losasso, F., Schaefer, S. and Warren, J. (2002). Dual contouring of Hermite data. *ACM
  Transactions on Graphics* 21(3), 339-346. https://doi.org/10.1145/566654.566586

Networks, partitions and grammars:

- Galin, E., Peytavie, A., Maréchal, N. and Guérin, E. (2010). Procedural generation of roads.
  *Computer Graphics Forum* 29(2), 429-438. https://doi.org/10.1111/j.1467-8659.2009.01612.x : a
  road as the least-cost path over the land.
- Parish, Y. I. H. and Müller, P. (2001). Procedural modeling of cities. *Proc. SIGGRAPH 2001*,
  301-308. https://doi.org/10.1145/383259.383292 : streets grown by an extended L-system.
- Chen, G., Esch, G., Wonka, P., Müller, P. and Zhang, E. (2008). Interactive procedural street
  modeling. *ACM Transactions on Graphics* 27(3), article 103.
  https://doi.org/10.1145/1360612.1360702 : streets traced along a tensor field.
- ASAM OpenDRIVE 1.9.0 (2026). Static road network description.
  https://www.asam.net/standards/detail/opendrive/ : a road as a reference line, lanes in lane
  sections, and junctions.
- ISO 19148:2021. Geographic information: linear referencing.
  https://www.iso.org/standard/75150.html : a place named by a distance measured along a line,
  with an offset to the side.
- Vanegas, C. A., Kelly, T., Weber, B., Halatsch, J., Aliaga, D. G. and Müller, P. (2012).
  Procedural generation of parcels in urban modeling. *Computer Graphics Forum* 31(2pt3), 681-690.
  https://doi.org/10.1111/j.1467-8659.2012.03047.x : lots by straight skeleton and by oriented
  bounding box.
- Aichholzer, O., Aurenhammer, F., Alberts, D. and Gärtner, B. (1995). A novel type of skeleton for
  polygons. *Journal of Universal Computer Science* 1(12), 752-761.
  https://www.jucs.org/jucs_1_12/a_novel_type_of.html : the straight skeleton.
- Whittaker, R. H. (1975). *Communities and Ecosystems* (2nd ed.). Macmillan.
  https://archive.org/details/communitiesecos00whit : biomes by yearly temperature and rainfall
  (the claim checked on a secondary page).
- Stiny, G. and Gips, J. (1972). Shape grammars and the generative specification of painting and
  sculpture. In *Information Processing 71*, North-Holland, 1460-1465.
  https://en.wikipedia.org/wiki/Shape_grammar
- Lindenmayer, A. (1968). Mathematical models for cellular interactions in development, I and II.
  *Journal of Theoretical Biology* 18(3), 280-315. https://doi.org/10.1016/0022-5193(68)90079-9
- Wonka, P., Wimmer, M., Sillion, F. and Ribarsky, W. (2003). Instant architecture. *ACM
  Transactions on Graphics* 22(3), 669-677. https://doi.org/10.1145/882262.882324 : split grammars.
- Müller, P., Wonka, P., Haegler, S., Ulmer, A. and Van Gool, L. (2006). Procedural modeling of
  buildings. *ACM Transactions on Graphics* 25(3), 614-623. https://doi.org/10.1145/1141911.1141931 :
  CGA shape.
- Merrell, P. (2007). Example-based model synthesis. *Proc. I3D '07*, 105-112.
  https://doi.org/10.1145/1230100.1230119 ; and Gumin, M. (2016). WaveFunctionCollapse.
  https://github.com/mxgmn/WaveFunctionCollapse : filling a grid under adjacency rules.
- Smelik, R. M., Tutenel, T., Bidarra, R. and Benes, B. (2014). A survey on procedural modelling for
  virtual worlds. *Computer Graphics Forum* 33(6), 31-50. https://doi.org/10.1111/cgf.12276

Placement:

- Illian, J., Penttinen, A., Stoyan, H. and Stoyan, D. (2008). *Statistical Analysis and Modelling of
  Spatial Point Patterns*. Wiley. https://doi.org/10.1002/9780470725160 : the marked point process,
  the statistics name for scattered points that each carry values.
- Cook, R. L. (1986). Stochastic sampling in computer graphics. *ACM Transactions on Graphics* 5(1),
  51-72. https://doi.org/10.1145/7529.8927 ; and Bridson, R. (2007). Fast Poisson disk sampling in
  arbitrary dimensions. *ACM SIGGRAPH 2007 Sketches*, article 22.
  https://doi.org/10.1145/1278780.1278807 : random points kept a least distance apart.
- Deussen, O., Hanrahan, P., Lintermann, B., Měch, R., Pharr, M. and Prusinkiewicz, P. (1998).
  Realistic modeling and rendering of plant ecosystems. *Proc. SIGGRAPH '98*, 275-286.
  https://doi.org/10.1145/280814.280898 : plants placed by competition for room.
- Oliver, C. D. and Larson, B. C. (1990). *Forest Stand Dynamics*. McGraw-Hill; update edition,
  Wiley, 1996. https://elischolar.library.yale.edu/fes_pubs/1/ : the four stages of a stand (the
  stages checked in a silviculture textbook).
- Sutherland, I. E. (1963). *Sketchpad: a man-machine graphical communication system*. PhD thesis,
  MIT. https://www.cl.cam.ac.uk/techreports/UCAM-CL-TR-574.html : constraints between drawn parts.

Guidance and measures:

- Lynch, K. (1960). *The Image of the City*. MIT Press. : paths, edges, districts, nodes, landmarks.
- Cullen, G. (1961). *Townscape*. Architectural Press. : serial vision.
- Appleton, J. (1975). *The Experience of Landscape*. Wiley. : prospect and refuge.
- Kaplan, R. and Kaplan, S. (1989). *The Experience of Nature: a Psychological Perspective*.
  Cambridge University Press. https://archive.org/details/experienceofnatu00kapl : mystery, one of
  four informational factors with coherence, complexity and legibility.
- Benedikt, M. L. (1979). To take hold of space: isovists and isovist fields. *Environment and
  Planning B* 6(1), 47-65. https://doi.org/10.1068/b060047 : the isovist, everything visible from a
  point.
- Hillier, B. and Hanson, J. (1984). *The Social Logic of Space*. Cambridge University Press.
  https://doi.org/10.1017/CBO9780511597237 : space syntax.
- Mansfield, S. (2011). *Japan's Master Gardens*. Tuttle.
  https://archive.org/details/japansmastergard0000mans : *miegakure*, hide and reveal in the stroll
  garden.
- Frederick, M. (2007). *101 Things I Learned in Architecture School*. MIT Press. : denial and reward
  (the lesson seen only on a page quoting the book).

Delivery, overlays and what runs in the game:

- Clark, J. H. (1976). Hierarchical geometric models for visible surface algorithms.
  *Communications of the ACM* 19(10), 547-554. https://doi.org/10.1145/360349.360354 : a model held
  at several levels of detail.
- OpenUSD glossary, "Composition Arcs". https://openusd.org/release/glossary.html : sublayers,
  references and variant sets, with a stated order of strength.
- Reynolds, C. W. (1987). Flocks, herds and schools: a distributed behavioral model. *Computer
  Graphics* 21(4), 25-34. https://doi.org/10.1145/37401.37406 : flocking from three rules per
  member.
- Sunshine-Hill, B. and Badler, N. (2010). Perceptually realistic behavior through alibi generation.
  *Proc. AIIDE* 6(1), 83-88. https://doi.org/10.1609/aiide.v6i1.12389

## Studio talks on procedural worlds

Titles, speakers and addresses checked on 2026-11-18.

- Carrier, E. (2018). Procedural World Generation of 'Far Cry 5'. GDC 2018.
  https://www.gdcvault.com/play/1025215/Procedural-World-Generation-of-Far : fresh water, roads,
  fences and power lines, cliffs, biomes and fog as tools over one terrain; read slide by slide for
  the ledger's sections on terrain analysis, biomes and scatter.
- van Muijden, J. (2017). GPU-Based Run-Time Procedural Placement in 'Horizon: Zero Dawn'. GDC 2017.
  https://www.gdcvault.com/play/1024120/GPU-Based-Procedural-Placement-in : placement by density
  maps.
- Werle, G. and Martinez, B. (2017). 'Ghost Recon Wildlands': Terrain Tools and Technology. GDC
  2017. https://www.gdcvault.com/play/1024029/-Ghost-Recon-Wildlands-Terrain
- Santiago, D. (2019). Procedurally Crafting Manhattan for 'Marvel's Spider-Man'. GDC 2019.
  https://www.gdcvault.com/play/1025765/Procedurally-Crafting-Manhattan-for-Marvel
- Pohlmann, M. (2021). Samurai Landscapes: Building and Rendering Tsushima Island on PS4. GDC 2021.
  https://www.gdcvault.com/play/1027352/Samurai-Landscapes-Building-and-Rendering

## Stylised and guidance-led worlds

That each exists was checked by search on 2026-11-13; the ledger read the first three.

Guidance and world layout:

- "Genshin Impact: Crafting an Anime-Style Open World", GDC 2021:
  https://gdcvault.com/play/1027538/-Genshin-Impact-Crafting-an : the source for the ledger's
  themed regions and its stylised sky.
- *Breath of the Wild* field level design, CEDEC 2017, an English translation of the slides:
  https://gist.github.com/idbrii/e39fe96279aa1670319bfa521d907399
- "Open world level design: spatial composition", a reading of that talk:
  https://www.blog.radiator.debacle.us/2017/10/open-world-level-design-spatial.html
- "How Nintendo Solved Zelda's Open World Problem", Game Maker's Toolkit:
  https://gmtk.substack.com/p/how-nintendo-solved-zeldas-open-world
- "Exploration in Ghost of Tsushima: Letting the Island Guide You", GDC 2021:
  https://gdcvault.com/play/1027399/Exploration-in-Ghost-of-Tsushima
- "A taxonomy of weenies: the landmarks that define Ghost of Tsushima":
  https://gamedeveloper.com/design/a-taxonomy-of-weenies-the-landmarks-that-define-i-ghost-of-tsushima-i-
- "Everything I Learned About Level Design I Learned from Disneyland", Scott Rogers, GDC 2009;
  slides: https://boingboing.net/2009/03/27/gamedesign-lessons-f.html

Small teams and small worlds:

- "Crafting a Tiny Open World: A Short Hike Postmortem", GDC 2020:
  https://gdcvault.com/play/1026613/Independent-Games-Summit-Crafting-A
- "Using a 'lemon-shaped' structure to create the open world of Sable":
  https://www.gamedeveloper.com/design/using-a-lemon-shaped-structure-to-create-the-sanguine-open-world-of-i-sable-i-
- "Organic Towns from Square Tiles", Oskar Stålberg, IndieCade Europe 2019:
  https://www.youtube.com/watch?v=1hqt8JkYRdI
- "How Townscaper works: a story four games in the making":
  https://www.gamedeveloper.com/game-platforms/how-townscaper-works-a-story-four-games-in-the-making

Stylised nature, technique:

- "Procedural Grass in Ghost of Tsushima", GDC 2021: https://gdcvault.com/play/1027214
- "Blowing from the West: Simulating Wind in Ghost of Tsushima", GDC 2021:
  https://gdcvault.com/play/1027350/Blowing-from-the-West-Simulating

Working projects to open:

- SideFX Project Skylark, a stylised low-poly Houdini and Unreal project aimed at a small team, and
  Project Pegasus, an open-world one: https://www.sidefx.com/learn-main-menu/tech-demos/
- Unreal's Electric Dreams environment, the reference project for PCG:
  https://www.unrealengine.com/electric-dreams-environment

## Cookbooks

Catalogue-style references: many small named entries, each a pattern, a node or a recipe. Kept as
a standing source of examples to reproduce and of names to file in the ledger. Addresses checked
on 2026-11-18.

Design patterns (what to build and why):

- The Level Design Book: level design patterns for layout, blockout and wayfinding, with a list
  of talks. https://book.leveldesignbook.com
- Alexander, C., Ishikawa, S. and Silverstein, M. (1977). *A Pattern Language*: 253 patterns for
  regions, towns, streets and buildings, each linked to larger and smaller ones. Print.
- Cullen, G. (1961). *Townscape*: named visual effects of moving through a town. Print.
- Totten, C. *An Architectural Approach to Level Design*: architectural patterns of space applied to
  games. Print.
- NACTO, Urban Street Design Guide: street cross-sections and every element in them: kerbs,
  crossings, bollards, tactile paving. https://nacto.org/publication/urban-street-design-guide/
- OpenStreetMap, Map Features: the working vocabulary of real-world features and their attributes.
  https://wiki.openstreetmap.org/wiki/Map_features

Technique recipes (how to compute it):

- Red Blob Games: interactive explanations of grids, pathfinding, noise terrain and polygon map
  generation. https://www.redblobgames.com/
- The Book of Shaders: noise, fractal layering, cellular patterns, shaping functions.
  https://thebookofshaders.com/
- Inigo Quilez, articles: distance functions, noise, domain warping, smooth blending.
  https://iquilezles.org/articles/
- Shaker, N., Togelius, J. and Nelson, M. J. (2016). *Procedural Content Generation in Games*.
  Springer: generation methods by kind of content. https://www.pcgbook.com/

Tool node references (each tool's vocabulary, one entry per operation):

- Gaea documentation: terrain operations named after landforms and processes.
  https://docs.gaea.app/
- Houdini documentation, its height field and Labs nodes: terrain, scatter and game-pipeline
  operations. https://www.sidefx.com/docs/houdini/
- Unreal Engine documentation, its PCG node reference: point sampling, filtering and spawning.
  https://dev.epicgames.com/documentation/unreal-engine
- ArcGIS CityEngine help, its rule reference: shape-grammar operations for buildings and streets.
  https://doc.arcgis.com/en/cityengine/latest/help/cityengine-help-intro.htm

## Still to find

- A standard or common practice for vegetation and plant modelling (species, growth forms, placement
  ecology), for the vegetation domain.
- How studios describe movement profiles and traversal rules for level validation.
