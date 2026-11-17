# How game studios build cities and towns for shipped 3D games: workflows, roles, kits

Reading notes: one bounded pass (about 27 tool calls). Page contents were read through a fetch tool that summarises pages, so short quoted phrases are as reported by that tool and should be checked against the source before being quoted verbatim. Items marked **[search summary only]** were seen only in a search-result summary, not on the fetched page. Items marked **[secondary]** come from third-party notes on a talk, not the talk itself. GDC Vault videos and slide decks themselves were not watched or read in full.

Cases covered: Skyrim / Fallout 3 / Fallout 4 (Bethesda), Watch Dogs 2, Ghost Recon Wildlands, Far Cry 5, Cyberpunk 2077, The Witcher 3, Marvel's Spider-Man, Assassin's Creed Unity (weak). Not covered for lack of budget: Horizon, Cities: Skylines, Kingdom Come: Deliverance, AC Origins/Odyssey, Watch Dogs Legion.

## 1. Per game: hand-placed from a kit, procedural then hand-finished, or both? Which tool?

### Takeaway
No well-sourced case is fully procedural for settlements: the spectrum runs from fully hand-placed (Cyberpunk 2077, Witcher 3, Bethesda interiors) through "tool assembles kit pieces along artist-drawn footprints and splines" (Watch Dogs 2) to "procedural base layer then manual detailing" (Ghost Recon Wildlands settlements, Spider-Man's Manhattan). Far Cry 5 used Houdini heavily for nature and linear features, while buildings and vehicles stayed hand-placed.

### Cited Findings

**Bethesda: Skyrim / Fallout 3 / Fallout 4 (hand-placed from modular kits, in-house editor)**
- Level designers hand-assemble levels from artist-made modular kits, snapping pieces on a grid in the studio's editor; the method is documented in a GDC 2013 talk by Joel Burgess (level design) and Nate Purkeypile (environment art) — [Game Developer transcript](https://www.gamedeveloper.com/design/skyrim-s-modular-approach-to-level-design)
- Skyrim team of about 90; only 10 worked on dungeons: 8 level designers and 2 full-time kit artists — [Game Developer transcript](https://www.gamedeveloper.com/design/skyrim-s-modular-approach-to-level-design)
- 7 kits built Skyrim's dungeon content; those kits produced over 400 cells in about two and a half years — [Game Developer transcript](https://www.gamedeveloper.com/design/skyrim-s-modular-approach-to-level-design)
- Skyrim world scope as stated in the talk: 16 sq mile overworld, 5 major cities, 2 hidden worldspaces, 300+ dungeons, 140+ points of interest, 37 towns, farms and villages — [Game Developer transcript](https://www.gamedeveloper.com/design/skyrim-s-modular-approach-to-level-design)
- Fallout 4 "family of kits" lists 14 kits: Retail, Institutional, Church, High-Tech, Industrial, Public Space, Steam Tunnel, Cave, House, The Institute, Neo-Classical, Office, Vault, Quarry — [GDC 2016 slides](https://www.slideshare.net/JoelBurgess/gdc-2016-modular-level-design-of-fallout-4)
- Fallout 4 moved from pieces exported whole from 3ds Max ("lots of redundant work") to finer-grained pieces assembled in the editor into "pack-ins"; one example layout went from 20 objects the old way to 123 objects the new way — [GDC 2016 slides](https://www.slideshare.net/JoelBurgess/gdc-2016-modular-level-design-of-fallout-4)

**Watch Dogs 2 (kit pieces assembled by an in-editor building tool on artist footprints; hand layout)**
- Planning started from a 2D city map (gameplay locations, roads, district types); grey 3D blocks on that map were later swapped for detailed buildings. Art staff used Google Earth "almost every day" to plan whole streets — [80.lv, Lincoln Hughes interview](https://80.lv/articles/building-urban-playgrounds-for-video-games)
- The building tool takes top-down footprint shapes; each edge of a shape spawns a "kit recipe", and added cuts and details create variation — [80.lv, Lincoln Hughes interview](https://80.lv/articles/building-urban-playgrounds-for-video-games)
- Roads are splines drawn on terrain; each spline carries width and material from recipes, and recipes spawn props procedurally (garbage cans at every intersection, randomised road decals, street lights about every 15 m). Moving the spline moves its props — [80.lv, Lincoln Hughes interview](https://80.lv/articles/building-urban-playgrounds-for-video-games)
- Ubisoft has said the game represents 36 architectural styles **[search summary only]** — [Game Developer](https://gamedeveloper.com/design/using-san-francisco-as-the-framework-for-i-watch-dogs-2-i-)

**Ghost Recon Wildlands (procedural base in Houdini, then hand detailing; some locations fully manual)**
- Houdini was used for terrain manipulation (roads, rivers), object placement rules (decals, forest, settlements, buildings) and asset creation; Houdini Engine was integrated into the in-house editor so artists run Houdini-built tools directly — [80.lv, Martinez and Delassus interview](https://80.lv/articles/procedural-technology-in-ghost-recon-wildlands/)
- Settlements used a "full procedural approach" giving artists control over patterns and diversity; artists then add manual detail — [80.lv](https://80.lv/articles/procedural-technology-in-ghost-recon-wildlands/)
- A generic building tool enforces metrics (window and door sizes, floor height), handles UVs and generates dirt strips where walls meet ground; windows, doors and props are added manually — [80.lv](https://80.lv/articles/procedural-technology-in-ghost-recon-wildlands/)
- Some locations are fully hand-built; for example a boss location inside a village is designed manually and a tool then builds the village around it — [80.lv](https://80.lv/articles/procedural-technology-in-ghost-recon-wildlands/)
- Staffing: never more than 15 level artists in Paris, 4 Houdini artists; world-building teams in Paris, Bucharest and Montpellier — [80.lv](https://80.lv/articles/procedural-technology-in-ghost-recon-wildlands/)
- Scale: 11 biomes; terrain 64k x 64k, at 25 cm per pixel initially and later 50 cm; World Machine recompute every 3 days over 80 computers — [80.lv](https://80.lv/articles/procedural-technology-in-ghost-recon-wildlands/)
- Commonly cited map figures: 800 km of road, 150 km of railway, 120 unique settlements, 7 million trees/bushes/rocks **[search summary only; original page not fetched]** — [USgamer](https://www.usgamer.net/articles/ghost-recon-wildlands-building-bolivia-by-brick-and-biome?page=2)
- Road builder described as a customised pathfinding (least-resistance) algorithm between two points **[search summary only]** — [World Machine forum](https://forum.world-machine.com/t/wm-used-in-ghost-recon-wildlands/4435)

**Far Cry 5 (procedural nature and linear features via Houdini Engine in Dunia; buildings hand-placed)**
- Tool set: freshwater (lakes, rivers, streams, waterfalls from artist splines), fences and power lines along splines, cliffs from terrain slope, biome tool (vegetation scatter), fog density map, world-map terrain **[secondary]** — [Christian Mills notes on Carrier's talk](https://christianjmills.com/posts/procedural-tools-far-cry-5-notes/index.html)
- Artists lay roads as splines; road generation clears vegetation, but houses and vehicles are still placed manually **[secondary]** — [Christian Mills notes](https://christianjmills.com/posts/procedural-tools-far-cry-5-notes/index.html)
- Houdini Engine runs inside the Dunia editor; a Python script passes world info, the terrain sector and splines with metadata into Houdini; smallest generation unit is a 64 x 64 m sector **[secondary]** — [Christian Mills notes](https://christianjmills.com/posts/procedural-tools-far-cry-5-notes/index.html)
- Nightly builds on build machines refresh the whole world each day; on-demand baking in the editor for loaded content, sectors under the camera or in the frustum **[secondary]** — [Christian Mills notes](https://christianjmills.com/posts/procedural-tools-far-cry-5-notes/index.html)
- World of about 100 sq km of wilderness; premise that not every square metre gets a level artist's attention — [80.lv summary of Carrier's talk](https://80.lv/articles/houdini-procedural-world-generation-of-far-cry-5/)
- Ubisoft's own article says the map is by and large handcrafted, with procedural placement for natural assets such as foliage and boulders **[search summary only]** — [Ubisoft News](https://news.ubisoft.com/en-us/article/16TjVZmAtD85EWcvHtxHXL/far-cry-5-creating-curiosity-in-a-familiar-world)

**Cyberpunk 2077 (hand-built; one spline road tool)**
- Environment Art Director Kacper Niepokólczycki says everything was hand-crafted, from buildings and adverts to trash and paper on the sidewalk; the team tested procedural tools but was "never happy with the results" — [80.lv interview](https://80.lv/articles/interview-how-cyberpunk-2077-s-night-city-was-built-almost-entirely-by-hand/)
- The nearest thing to procedural was a road tool: artists draw a spline, the tool places road, bridge and overpass meshes plus parameter-driven decoration, and artists then adjust by hand — [80.lv interview](https://80.lv/articles/interview-how-cyberpunk-2077-s-night-city-was-built-almost-entirely-by-hand/)
- REDengine 4 used a prefab-based hierarchy to build and assemble the world **[search summary only, from the GDC Vault session overview]** — [GDC Vault: Building Night City, The Technology](https://gdcvault.com/play/1028981/Building-Night-City-The-Technology)

**The Witcher 3 (hand-placed; brush-painted vegetation)**
- Cities, villages and caves start as grey boxes, then get proper meshes, decoration, paths and foliage; meshes were deliberately not placed on a grid and kept slightly imperfect to look hand-built — [80.lv, Michał Janiszewski](https://80.lv/articles/world-building-of-witcher-3/)
- Vegetation was painted with brushes in the editor from regional SpeedTree libraries (Novigrad, Skellige, Toussaint), with transitions hand-placed and trees moved for composition and combat — [80.lv](https://80.lv/articles/world-building-of-witcher-3/)
- Texture arrays of tileable textures were used per district to cut unique textures, because city crowds consume texture memory — [80.lv](https://80.lv/articles/world-building-of-witcher-3/)
- Dungeons used "more or less modular" sets of small wall pieces at varying angles, not finished rooms **[search summary only]** — [NeoGAF transcription of GameStar interview](https://m.neogaf.com/threads/new-the-witcher-3-information-from-gamestar.1014355/page-8)

**Marvel's Spider-Man (Houdini-driven procedural systems with hand polish)**
- David Santiago (Principal Technical Artist, Insomniac) presented "Procedurally Crafting Manhattan" at GDC 2019; the systems were designed around iteration and dependencies and ended up authoring, modifying and monitoring more content than planned — [Game Developer](https://www.gamedeveloper.com/programming/video-procedurally-creating-manhattan-for-i-marvel-s-spider-man-i-)
- SideFX's summary of his Houdini HIVE talk lists Houdini use for ground, buildings, traffic, vignettes and crimes, props, impostors, audio and lighting **[search summary only]** — [SideFX GDC 2019](https://www.sidefx.com/community/houdinihive_gdc2019/)
- Environment generated offline; crimes and vignettes placed by systems that search the generated city for suitable locations; designers polish output by hand **[secondary]** — [Medium summary](https://medium.com/@jahani.rana/procedural-elements-in-marvels-spiderman-2885ce2b387d); [SIGGRAPH 2019 talk abstract](https://history.siggraph.org/learning/procedural-system-assisted-authoring-of-open-world-content-for-marvels-spider-man-by-santiago/)

**Assassin's Creed Unity (generator then hand edits; weakly sourced)**
- A fan analysis of IGN's making-of video describes a "House Block Generator": developers sketch a street shape, assign it a theme (probably a district), and the generator fills it with buildings; they can then reassign themes, regenerate buildings and streets, and hand-modify rooftops for parkour **[secondary, fan reading of video screenshots]** — [Access the Animus](https://accesstheanimus.com/Making_of_hidden_secrets_part2.html)

### Inferences
- The more a game depends on close-range first-person or dense narrative spaces (Cyberpunk, Witcher 3, Bethesda interiors), the more hand placement dominates; the more it must cover area (Wildlands, Far Cry 5) the more procedural layers appear, but mostly for nature and linear infrastructure, not for building design.
- Where buildings are tool-assembled (Watch Dogs 2, Wildlands, AC Unity), the tool input is a footprint or street shape plus a style/theme tag, and the tool output is an arrangement of artist-made kit pieces. The kit is still hand-authored in every case.
- "Recipe" (Watch Dogs 2) and "theme" (AC Unity) play the same role: a named rule set mapping an edge or lot to kit pieces.

### Gaps
- Spider-Man: no numbers (blocks, buildings, tiles, team size) or building construction details recovered; the GDC video ([YouTube ID 4aw9uyj9MAE](https://www.youtube.com/watch?v=4aw9uyj9MAE)) and slides were not read. A Chinese translation of the talk exists ([CSDN](https://blog.csdn.net/u013412391/article/details/115217431)) but the fetch failed.
- AC Unity / Origins / Odyssey: no primary Ubisoft source on city or building generation found in this pass.
- Novigrad: no source found on building kit structure or building counts. A claim of "well over a hundred" open buildings exists only in a forum transcription seen in a search summary.
- Cyberpunk 2077: no building count or city team size found; the prefab hierarchy talk was not read.
- Far Cry 5 details rest on third-party notes, not the talk itself.

## 2. What does an environment artist deliver for a building kit?

### Takeaway
A kit is a set of snapping pieces built to one agreed footprint, with consistent pivots, a decodable naming scheme and numbered visual variants, delivered in stages (proof, graybox, build-out, polish) and stress-tested by level designers before being arted up. Bethesda's talks are the most concrete published contract; unit numbers there are 128 / 256 / 512 engine units.

### Cited Findings
- Footprint: a 512x512x512 room tiles with a 256x256x256 hallway; a 384 room creates gaps or overlaps. Sub-kit footprints should be multiples of each other — [Game Developer transcript](https://www.gamedeveloper.com/design/skyrim-s-modular-approach-to-level-design)
- Level designers typically work with grid snap at half the footprint size — [Game Developer transcript](https://www.gamedeveloper.com/design/skyrim-s-modular-approach-to-level-design)
- Scale reference: a character is about 128 units (six feet); narrowest traversable space at least two character widths; doorframes share one uniform dimension across kits — [Game Developer transcript](https://www.gamedeveloper.com/design/skyrim-s-modular-approach-to-level-design)
- Slopes: AI handled roughly 60 degree inclines; kits may use 30 to 45 degrees — [Game Developer transcript](https://www.gamedeveloper.com/design/skyrim-s-modular-approach-to-level-design)
- Tiling: avoid tiling on all axes; hallways usually tile on one axis, rooms on two. Floor thickness must be fixed up front to avoid co-planar faces when stacking — [Game Developer transcript](https://www.gamedeveloper.com/design/skyrim-s-modular-approach-to-level-design)
- Pivots: usually at ground plane and centred, with exceptions (pipe kit at pipe edge, platform kit at base). Fallout 4 talk: "Pick a pivot, and stick with it" — [Game Developer transcript](https://www.gamedeveloper.com/design/skyrim-s-modular-approach-to-level-design); [GDC 2016 slides](https://www.slideshare.net/JoelBurgess/gdc-2016-modular-level-design-of-fallout-4)
- Naming: Fallout 3 piece "UtlBayCorInMidPRTT01L01" decodes as kit (Utl), sub-kit (Bay), type (Cor = corner), In (inside), Mid (tiling piece), a hard-to-read token, L (mirrored left), 01 (visual variant). The unreadable token led to the kit being underused — [Game Developer transcript](https://www.gamedeveloper.com/design/skyrim-s-modular-approach-to-level-design)
- Naming: hallway pieces named 1Way, 2Way, 3Way, 4Way so similar pieces sort together; 01/02/03 suffix for visual variants; keep conventions consistent across kits — [Game Developer transcript](https://www.gamedeveloper.com/design/skyrim-s-modular-approach-to-level-design)
- Fallout 4 example names: BldWoodPSmHall2Way01, BldWoodPSmWall01, BldWoodPSmWallCorner01, BldWoodPSmFlrOnly01, BldWoodPSmCeilOnly01 — [GDC 2016 slides](https://www.slideshare.net/JoelBurgess/gdc-2016-modular-level-design-of-fallout-4)
- Standard sub-kits in Fallout 4's building kit: rooms, halls, stairs, railings, platforms — [GDC 2016 slides](https://www.slideshare.net/JoelBurgess/gdc-2016-modular-level-design-of-fallout-4)
- Piece counts: a Skyrim cave "small hallway" sub-kit has about 50 pieces; the Ratway hall sub-kit has 7; a simple Fallout 3 pipe kit has 4. Cave kit has 7 sub-kits and is used 200+ times; Ratway has 3 sub-kits and is used twice — [Game Developer transcript](https://www.gamedeveloper.com/design/skyrim-s-modular-approach-to-level-design)
- Core module types listed by the Level Design Book: floor, ceiling, wall, single doorway, double doorway, window, corner; plus platforms and glue pieces, and for caves flanges, arches and shells — [The Level Design Book](https://book.leveldesignbook.com/process/blockout/metrics/modular)
- Staged delivery: Concept; Proof (near-zero detail pieces, 1 to 3 weeks, settles footprint, kit logic and naming); Graybox (functional rough pieces, 1 to 4 weeks, primary sub-kit first, stress-tested in real layouts); Build-out (real geometry and textures, a few months, one finished piece before variants); Polish (ongoing fixes, resist bloat) — [Game Developer transcript](https://www.gamedeveloper.com/design/skyrim-s-modular-approach-to-level-design)
- Variants last: "DO NOT MAKE VARIANTS YET, just build basic pieces first"; texture variants are cheap because geometry need not be retested — [The Level Design Book](https://book.leveldesignbook.com/process/blockout/metrics/modular)
- Special pieces: glue and platform kits bridge sub-kits and styles; a "De-Twist" piece handles flipped meetups in directional kits; editor-only "helper markers" can be shipped with a kit — [Game Developer transcript](https://www.gamedeveloper.com/design/skyrim-s-modular-approach-to-level-design)
- Acceptance tests: loopback (kit closes on itself), stack (floors have thickness), gap (enough glue pieces for off-angle construction) — [The Level Design Book](https://book.leveldesignbook.com/process/blockout/metrics/modular); [Game Developer transcript](https://www.gamedeveloper.com/design/skyrim-s-modular-approach-to-level-design)
- For tool-assembled exteriors (Wildlands), the tool, not the artist, enforces metrics such as window and door size and floor height — [80.lv](https://80.lv/articles/procedural-technology-in-ghost-recon-wildlands/)
- Watch Dogs 2: kits were chosen under memory budgets by picking the most common local architectural styles — [80.lv](https://80.lv/articles/building-urban-playgrounds-for-video-games)

### Inferences
- The kit "contract" has four parts that must be frozen early because they are expensive to change: footprint, pivot, snap rules and names. Geometry and textures can change freely behind them.
- Variant numbering as a trailing suffix means variants are interchangeable by construction: same footprint, same pivot, different look.
- Published kit contracts are mostly for interiors and dungeons. Exterior facade kits (wall, window, door, roof, trim, corner per storey) are described only indirectly via building tools.

### Gaps
- No published piece list or module size found for an exterior city facade kit (Watch Dogs 2, AC Unity, Spider-Man, Night City).
- Damage or dressing set conventions: nothing specific found beyond Fallout 4's note that finer granularity gives more destruction options.
- Unreal's modular level design page (Lee Perry, 2002) returned HTTP 403 and was not read: https://docs.unrealengine.com/udk/Three/ModularLevelDesign.html
- Second half of the Fallout 4 slide deck (about slides 100 to 274) was not read.

## 3. Layers of a city in production: what they are, who owns them, in what order

### Takeaway
Published workflows agree on the order: story or 2D map, then roads and district types, then greybox massing with gameplay metrics, then kit buildings replacing the grey blocks, then props and dressing in layers, with optimisation last. Procedural pipelines formalise the same order as a dependency chain where each layer exports masks for the next.

### Cited Findings
- Watch Dogs 2 order: research and story planning; 2D city map (gameplay locations, roads, district types); minimal greybox with gameplay metrics; detailing starting with large gameplay-relevant props; final layered detail pass usually in the last few months — [80.lv](https://80.lv/articles/building-urban-playgrounds-for-video-games)
- Watch Dogs 2 role split: content team models and textures modular building pieces; tech artists integrate them into the editor's building tool; level artists pick the style per neighbourhood and swap greybox for the matching kits — [80.lv](https://80.lv/articles/building-urban-playgrounds-for-video-games)
- Watch Dogs 2 street furniture is owned by the road spline through recipes, not placed individually — [80.lv](https://80.lv/articles/building-urban-playgrounds-for-video-games)
- Wildlands layer order: terrain, then rivers, roads, railways, forest, settlements, mines; each layer gives "the base for the next layer" — [80.lv](https://80.lv/articles/procedural-technology-in-ghost-recon-wildlands/)
- Far Cry 5 tools run sequentially (freshwater, cliffs, biomes, and so on), earlier tools exporting masks that later tools consume; roads act as exclusion masks **[secondary]** — [Christian Mills notes](https://christianjmills.com/posts/procedural-tools-far-cry-5-notes/index.html)
- Witcher 3 order: story sets dimensions and points of interest; grey boxes set city size; Level, Quest and Living World design teams decide shop and inn counts; meshes, decoration, paths, foliage; artists make near-final "beautiful corners"; open-space polish and cutscene support; optimisation last. Village planning starts from population and number of enterable houses — [80.lv](https://80.lv/articles/world-building-of-witcher-3/)
- Cyberpunk 2077: open world is art-driven (level artists sketch large shapes per district, art directors set look); quest locations are level-design-driven (LD blockout first, then art). Environment Art and Level Design are paired from the start — [80.lv](https://80.lv/articles/interview-how-cyberpunk-2077-s-night-city-was-built-almost-entirely-by-hand/)
- Bethesda level passes: Pass 0 Concept, Pass 1 Layout, Pass 2 Gameplay, Pass 3 Content Complete, Pass 4 Polish; kits are front-loaded in the art schedule so the core kit is usable before layouts rely on it — [GDC 2016 slides](https://www.slideshare.net/JoelBurgess/gdc-2016-modular-level-design-of-fallout-4)
- Spider-Man: runtime content (crimes, vignettes) depends on the offline city layout; traffic, pedestrians, props, impostors, lighting probes and audio are further dependent layers — [SIGGRAPH abstract](https://history.siggraph.org/learning/procedural-system-assisted-authoring-of-open-world-content-for-marvels-spider-man-by-santiago/); [SideFX GDC 2019](https://www.sidefx.com/community/houdinihive_gdc2019/) **[SideFX list from search summary only]**

### Inferences
- A consistent ownership pattern: designers and level artists own layout (roads, footprints, greybox); a separate content or kit team owns pieces; technical artists own the tool that binds pieces to layout; level artists own final dressing.
- Roads come before buildings everywhere, and are represented as splines carrying attributes (width, material, prop recipe). Buildings attach to footprints or street edges. This makes roads the upstream dependency whose changes ripple furthest.
- None of the sources describes an explicit "lot or parcel" layer; the unit below the block is a footprint shape or a street edge.

### Gaps
- Who owns traffic and crowd data, and when it is authored, is not described in the sources read.
- Interiors as a city layer (versus separate cells) not covered, apart from Witcher 3 counting enterable houses.
- No source read gives an explicit blocks-to-lots subdivision step for a shipped game.

## 4. What goes wrong and what is painful

### Takeaway
The recurring failures are (a) contract changes to kit pieces (pivot, footprint, snap) that touch hundreds of placed instances, (b) hand edits destroyed when an upstream procedural layer is regenerated, and (c) art changes that silently break gameplay markup. Studios respond by freezing kit contracts early, locking regions, and making tools preserve hand work that still passes its rules.

### Cited Findings
- Changing a pivot late can require manual updates to hundreds of instances; fixing one piece affects every placed copy, so footprint, snap and pivot changes must be anticipated — [Game Developer transcript](https://www.gamedeveloper.com/design/skyrim-s-modular-approach-to-level-design)
- Footprint violations: alcoves extending past the footprint block adjacent hallways — [Game Developer transcript](https://www.gamedeveloper.com/design/skyrim-s-modular-approach-to-level-design)
- Patch-up pieces (for example a half-length hallway) bloat the kit and add non-obvious rules; fix the core piece instead — [Game Developer transcript](https://www.gamedeveloper.com/design/skyrim-s-modular-approach-to-level-design)
- Stacking pieces with no floor thickness gives co-planar faces, z-fighting and optimisation problems — [Game Developer transcript](https://www.gamedeveloper.com/design/skyrim-s-modular-approach-to-level-design)
- Rotated or off-grid placement (snap-to-reference, pivot-and-flange) creates gaps that must be bridged and is prone to tiny holes — [Game Developer transcript](https://www.gamedeveloper.com/design/skyrim-s-modular-approach-to-level-design)
- Handoff breakage: art changes after design markup broke gameplay, for example cover turned into thin poles, or a wall turned into see-through chain-link — [Game Developer transcript](https://www.gamedeveloper.com/design/skyrim-s-modular-approach-to-level-design)
- Art fatigue: repeated detail elements are noticed more than repeated architecture; Oblivion's reuse led to "cookie-cutter" dungeons — [Game Developer transcript](https://www.gamedeveloper.com/design/skyrim-s-modular-approach-to-level-design)
- Kit-bashing across kits can cause bad intersections and lighting issues — [Game Developer transcript](https://www.gamedeveloper.com/design/skyrim-s-modular-approach-to-level-design)
- Each kit carries overhead: lead time, onboarding, iteration, maintenance — [GDC 2016 slides](https://www.slideshare.net/JoelBurgess/gdc-2016-modular-level-design-of-fallout-4)
- Wildlands: procedural and manual work "don't mix very well"; manual edits are lost at the next update unless the procedural data is locked first, and workarounds were "not trivial" — [80.lv](https://80.lv/articles/procedural-technology-in-ghost-recon-wildlands/)
- Wildlands: early mesh-tile pipeline needed round trips between Houdini and editor and only one person could iterate on it; World Machine output was not fully deterministic, tile seams fixed about 90% by pre-eroding at low resolution — [80.lv](https://80.lv/articles/procedural-technology-in-ghost-recon-wildlands/)
- Far Cry 5: constant terrain iteration made manual placement and repainting inconsistent and tedious, and locking terrain early was unrealistic; determinism across build machines was required for seamless junctions and nav mesh **[secondary]** — [Christian Mills notes](https://christianjmills.com/posts/procedural-tools-far-cry-5-notes/index.html)
- Far Cry 5 lesson: too much automation causes problems, too much manual control is slow; users sometimes preferred manual control (riverbed carving) **[secondary]** — [Christian Mills notes](https://christianjmills.com/posts/procedural-tools-far-cry-5-notes/index.html)
- Spider-Man: when roads or buildings change, crimes and vignettes in that area must be re-placed even if already hand-adjusted; the follow-up system keeps hand-authored work that still meets specifications, places components with higher fidelity, and flags items needing attention — [SIGGRAPH 2019 abstract](https://history.siggraph.org/learning/procedural-system-assisted-authoring-of-open-world-content-for-marvels-spider-man-by-santiago/)
- Spider-Man: dependencies between systems drove the need to modularise generated content **[secondary]** — [Medium summary](https://medium.com/@jahani.rana/procedural-elements-in-marvels-spiderman-2885ce2b387d)
- Witcher 3: story revisions changed layouts (a mountain blocking a quest location was replaced with a lake and village); artists told not to get attached to work — [80.lv](https://80.lv/articles/world-building-of-witcher-3/)
- Cyberpunk 2077: some locations were rebuilt entirely after narrative or gameplay changes; some areas became too noisy, leading to colour, shape and negative-space guidelines for Phantom Liberty; Witcher-era metrics did not work in first person — [80.lv](https://80.lv/articles/interview-how-cyberpunk-2077-s-night-city-was-built-almost-entirely-by-hand/)
- Watch Dogs 2: everything can change during production (gameplay, tools, art direction, story); procedural systems exist to avoid manually editing thousands of meshes — [80.lv](https://80.lv/articles/building-urban-playgrounds-for-video-games)

### Inferences
- Instance-based kits make geometry revisions free (all instances update) but make contract revisions (pivot, footprint, snap) catastrophic. This is why Bethesda spends weeks on untextured proof pieces first.
- Three published strategies for hand work versus regeneration: lock the region (Wildlands), keep regeneration to layers that carry no hand work and place buildings by hand (Far Cry 5), or re-validate hand work against rules and keep what still passes (Spider-Man).
- Binding props to their parent spline or footprint (Watch Dogs 2 recipes) removes a class of re-dressing work after layout changes, at the cost of less bespoke dressing.

### Gaps
- No source read describes a versioning or migration mechanism for a revised kit piece across an already-built city.
- No quantified cost of a layout change (hours of re-dressing) found.
- The mechanism by which Spider-Man tests hand work against "specifications" is not explained in the abstract.

## 5. Published talks, postmortems and articles describing these pipelines

### Takeaway
The strongest primary sources are Bethesda's two GDC modular talks (text and slides freely available), the 80.lv interview set for Ubisoft and CD Projekt Red titles, and the GDC 2018/2019 Houdini talks for Far Cry 5 and Spider-Man (video only).

### Cited Findings
- Joel Burgess and Nate Purkeypile, "Skyrim's Modular Approach to Level Design", GDC 2013 — [transcript](https://www.gamedeveloper.com/design/skyrim-s-modular-approach-to-level-design); [slides](https://www.slideshare.net/JoelBurgess/gdc2013-kit-buildingfinal)
- Burgess and Purkeypile, "The Modular Level Design of Fallout 4", GDC 2016, 274 slides — [slides](https://www.slideshare.net/JoelBurgess/gdc-2016-modular-level-design-of-fallout-4); [video](https://www.youtube.com/watch?v=QBAM27YbKZg)
- The Level Design Book, modular kit design page, with references — [book page](https://book.leveldesignbook.com/process/blockout/metrics/modular)
- Lee Perry, "Modular Level and Component Design", Game Developer magazine Nov 2002, hosted in Unreal UDK docs (not read, 403) — [UDK docs](https://docs.unrealengine.com/udk/Three/ModularLevelDesign.html)
- Lincoln Hughes on Watch Dogs 2 urban environments — [80.lv](https://80.lv/articles/building-urban-playgrounds-for-video-games)
- Benoit Martinez and Vincent Delassus on Ghost Recon Wildlands procedural technology (2017) — [80.lv](https://80.lv/articles/procedural-technology-in-ghost-recon-wildlands/); related [80.lv world building](https://80.lv/articles/procedural-world-building-in-ghost-recon-wildlands/); GDC 2017 "Ghost Recon Wildlands: Terrain Tools and Technology" by Guillaume Werle and Benoit Martinez — [GDC Vault](https://gdcvault.com/play/1024029/) (not read)
- Etienne Carrier, "Procedural World Generation of Far Cry 5", GDC 2018 — [GDC Vault](https://gdcvault.com/play/1025557/Procedural-World-Generation-of-Far); [SideFX talk page](https://www.sidefx.com/learn/talks/procedural-world-generation-far-cry-5/); [third-party notes](https://christianjmills.com/posts/procedural-tools-far-cry-5-notes/index.html)
- David Santiago, "Procedurally Crafting Manhattan for Marvel's Spider-Man", GDC 2019 — [Game Developer](https://www.gamedeveloper.com/programming/video-procedurally-creating-manhattan-for-i-marvel-s-spider-man-i-); [video](https://www.youtube.com/watch?v=4aw9uyj9MAE); SIGGRAPH 2019 talk "Procedural System Assisted Authoring of Open-World Content for Marvel's Spider-Man" — [ACM](https://dl.acm.org/doi/10.1145/3306307.3328158)
- Elan Ruskin, "Marvel's Spider-Man: A Technical Postmortem", GDC 2019 — [80.lv summary](https://80.lv/articles/marvels-spider-man-a-technical-postmortem)
- Kacper Niepokólczycki and Jakub Knapik on Night City — [80.lv](https://80.lv/articles/interview-how-cyberpunk-2077-s-night-city-was-built-almost-entirely-by-hand/); GDC talks "Art Direction Summit: Building Night City" — [GDC Vault](https://gdcvault.com/play/1027571/Art-Direction-Summit-Building-Night) and "Building Night City: The Technology of Cyberpunk 2077" — [GDC Vault](https://gdcvault.com/play/1028981/Building-Night-City-The-Technology) (neither read)
- Michał Janiszewski on Witcher 3 world building — [80.lv](https://80.lv/articles/world-building-of-witcher-3/)
- AC Unity generator, fan analysis of IGN making-of — [Access the Animus](https://accesstheanimus.com/Making_of_hidden_secrets_part2.html)

### Inferences
- For a kit contract, Bethesda's two talks are the only sources found with explicit numbers and names; everything else describes workflow at interview level.

### Gaps
- GDC Vault video content was not accessible to the fetch tool; talk details beyond transcripts, slides and third-party notes are unverified.
- No SideFX customer story page was fetched directly.
- Kingdom Come: Deliverance, Horizon, Cities: Skylines, AC Origins/Odyssey and Watch Dogs Legion were not researched.
