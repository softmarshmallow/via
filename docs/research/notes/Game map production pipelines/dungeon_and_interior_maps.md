# Dungeon, procedural-level, metroidvania and multi-floor interior maps in shipped games

Evidence grades used in these notes:

- **[code]**: read from reverse-engineered source or a RAM/disassembly map of the shipped game.
- **[dev]**: stated by a developer of the game (article, interview, official post).
- **[modding doc]**: community documentation of the shipped game's data files.
- **[press/wiki]**: observed behaviour described by press, guides or wikis. This describes what the map does, not how it is built.
- **[snippet]**: I only saw a search-result summary of the page, not the page itself. Treat as a lead.

Overall caveat: primary sources are good for older and reverse-engineered games (Diablo, Super Metroid, Isaac) and for level generation (Dead Cells, Isaac). I found **no developer-authored description of how the map itself is produced** for any Group B game (Resident Evil 2 Remake, Prey, Control, Baldur's Gate 3, Deus Ex: Mankind Divided, Dishonored), nor for Hollow Knight, Hades, Diablo III/IV, Metroid Dread or Dead Cells' minimap. For those, only player-visible behaviour is documented here.

## Group A: what does the automap show, and is it a room graph, a cell grid or an outline of walkable space? Generated or authored?

### Takeaway
Three distinct models appear in shipped games: (1) a per-tile lookup from level tiles to hand-authored automap glyphs (Diablo, Diablo II), (2) a coarse screen-sized cell grid with one explored bit per cell over a hand-authored area map (Super Metroid), and (3) a room-per-cell grid where the generator's floor plan is itself the map (Binding of Isaac). In every documented case the map is derived from the same data the level is built from, plus a small authored lookup table or tilemap; none is drawn independently of level data.

### Cited Findings

**Diablo (1996), from the Devilution reverse-engineered source [code]**
- The automap is a per-dungeon-cell grid. `automapview[DMAXX][DMAXY]` is a boolean array recording which dungeon cells have been explored. — [devilution automap.cpp](https://github.com/diasurgical/devilution/blob/master/Source/automap.cpp)
- A lookup table `automaptype[512]` (16-bit words) maps each level tile ID to an automap shape. The low nibble is a shape type (column, vertical wall, horizontal wall, both, cave variants), and the high byte holds flag bits: `MAPFLAG_VERTDOOR` 0x01, `MAPFLAG_HORZDOOR` 0x02, `MAPFLAG_VERTARCH` 0x04, `MAPFLAG_HORZARCH` 0x08, `MAPFLAG_VERTGRATE` 0x10, `MAPFLAG_HORZGRATE` 0x20, `MAPFLAG_DIRT` 0x40, `MAPFLAG_STAIRS` 0x80. — [devilution automap.cpp](https://github.com/diasurgical/devilution/blob/master/Source/automap.cpp)
- The table is authored per tileset, not per level: `InitAutomap()` loads one `.AMP` file per dungeon type (`Levels\L1Data\L1.AMP` cathedral, `L2.AMP` catacombs, `L3.AMP` caves, `L4.AMP` hell), then clears `automapview`. Because levels are procedurally assembled from that tileset, the map follows automatically. — [devilution automap.cpp](https://github.com/diasurgical/devilution/blob/master/Source/automap.cpp)
- Drawing is vector-style: `DrawAutomap()` walks the isometric grid and calls `DrawAutomapTile()` for every explored cell with a non-zero type; all geometry is drawn with `DrawLine()`, with floor "dirt" dots as single pixels. Doors and stairs use a bright colour, walls and grates a dim one. The player is a three-line arrow (`DrawAutomapPlr()`); Hellfire adds item diamonds by scanning a 16x16 window around the player. In town only the level-name text is drawn. — [devilution automap.cpp](https://github.com/diasurgical/devilution/blob/master/Source/automap.cpp)
- So the Diablo automap shows walls, doors, arches, grates, stairs and floor texture hints. It is an outline of walls on a cell grid, not a room graph.

**Diablo II (2000) [modding doc]**
- The same idea, table-driven: `AutoMap.txt` maps level tiles to automap image frames. Columns: `LevelName` (act and level type), `TileName` (tile orientation codes such as `fl` base floor, `wl` base left wall), `Style` (group ID, the tile main index), `StartSequence` / `EndSequence` (sub-index range), and `Cel1`..`Cel4`, each of which "Determines the unique image frame to use from the MaxiMap.dc6 file". Several Cel entries give random visual variety. — [Diablo II Data File Guide](https://d2runewizard.com/game-files/guide.html)
- A Phrozen Keep modding guide (seen via a Scribd mirror) states the automap is tied to the tiles a map is composed of, each tile can have an automap icon, neighbouring icons combine into the picture, and tiles with no icon leave blanks (in Act 1 wilderness mostly only paths and level borders appear). The glyph sheets are `MaxiMap.dc6` and `MaxiMapS.dc6`. [snippet] — [AutoMap guide mirror](https://www.scribd.com/document/775854992/AutoMap)
- So Diablo II replaces Diablo's line primitives with small pre-drawn sprites per tile, still keyed from tile identity.

**Super Metroid (1994) [code]**
- The map is a coarse grid of map tiles that correspond to screen-sized scroll blocks of the level. — [Metroid Wiki, Map](https://metroid.fandom.com/wiki/Map)
- Explored state for the current area is 100h bytes at `$07F7..08F6`, one bit per map tile, laid out like a 64x32 1bpp tilemap (two 32x32 pages side by side), most significant bit leftmost; the first row is padding. — [Super Metroid RAM map, P. Johnston](https://patrickjohnston.org/ASM/Lists/Super%20Metroid/RAM%20map.asm)
- Each room carries its own position on the area map: `$07A1` room X co-ordinate (on map) and `$07A3` room Y co-ordinate (on map), with `$079F` as the area index (Crateria, Brinstar, Norfair, Wrecked Ship, Maridia, Tourian, Ceres, Debug). — [Super Metroid RAM map](https://patrickjohnston.org/ASM/Lists/Super%20Metroid/RAM%20map.asm)
- Map-station data is shown in blue and turns pink once the player enters the matching scroll block. — [Metroid Wiki, Map](https://metroid.fandom.com/wiki/Map)

**The Binding of Isaac (2011) and Rebirth (2014) [dev-assisted reverse engineering]**
- The floor is generated on a 9x8 grid of cells, one room per cell, starting at cell 35. Room count is `random(2) + 5 + level * 2.6`. Growth is breadth-first from the start; a neighbour is rejected if occupied, if it already has more than one filled neighbour, if the quota is met, or on a 50% roll. Dead ends are recorded and used for boss and special rooms. — [Boris the Brave, Dungeon Generation in Binding of Isaac](https://www.boristhebrave.com/2020/09/12/dungeon-generation-in-binding-of-isaac/)
- Adjacent rooms always connect by a door at the centre of the shared edge, and every room template is designed to be enterable from all four sides. Room interiors come from hand-made pools (174 normal rooms for the Basement). — [Boris the Brave](https://www.boristhebrave.com/2020/09/12/dungeon-generation-in-binding-of-isaac/)
- Rebirth adds large rooms (2x2, 2x1, L-shaped, narrow corridors) that occupy several grid cells and have up to 8 exits; the generator then iterates over exits rather than four directions. — [Boris the Brave](https://www.boristhebrave.com/2020/09/12/dungeon-generation-in-binding-of-isaac/)
- The author's sources were a community decompilation plus answers from Florian Himsl (original programmer) and Simon Parzer (Rebirth developer). — [Boris the Brave](https://www.boristhebrave.com/2020/09/12/dungeon-generation-in-binding-of-isaac/)
- The Rebirth modding API exposes per-room `DisplayFlags` controlling minimap appearance (invisible, visible, shadow for secret or locked rooms). [snippet; the page returned HTTP 429 when fetched] — [IsaacDocs summary, Room and Level System](https://deepwiki.com/wofsauge/IsaacDocs/3-room-and-level-system)

**Dead Cells (2018) [dev]**
- Levels are assembled from hand-designed room templates ("tiles"), each with "a specific layout of platforms designed for a specific purpose", authored in CastleDB and tagged by biome, purpose and entrance/exit count. Each biome has a designer-made concept graph of room nodes; "For each node, the algorithm tries a random room" until one satisfies the graph's constraints. — [S. Benard, The Level Design of Dead Cells: a hybrid approach](https://deepnight.net/tutorial/the-level-design-of-dead-cells-a-hybrid-approach/); mirror at [Game Developer](https://www.gamedeveloper.com/design/building-the-level-design-of-a-procedurally-generated-metroidvania-a-hybrid-approach-)
- The article says nothing about the minimap. — [same source](https://deepnight.net/tutorial/the-level-design-of-dead-cells-a-hybrid-approach/)

**Hollow Knight (2017) [dev for intent; press/wiki for behaviour]**
- Team Cherry dropped an early randomly-generated layout because "it was a headache", and built the hand-authored world outward from the idea of the world. — [Game Developer, How the Hollow Knight devs mapped out their metroidvania](https://www.gamedeveloper.com/design/how-the-i-hollow-knight-i-devs-mapped-out-their-metroidvania-)
- Ari Gibson is reported as saying the map could not work "in a Metroid-style way" because they wanted players to feel like explorers of uncharted land, and that the mapping system went through many discarded versions. [snippet; the PC Gamer page body was truncated when fetched] — [PC Gamer, How to design a great Metroidvania map](https://www.pcgamer.com/how-to-design-a-great-metroidvania-map/)
- Behaviour: an incomplete area map must be bought from Cornifer (or later from Iselda); the Quill is required to add rooms; newly explored rooms are added only when resting at a bench; pins for benches and other features are bought separately. — [Hollow Knight Wiki, Map and Quill](https://hollowknight.wiki/w/Map); [Hollow Knight Wiki, Bench](https://hollowknight.wiki/w/Bench_%28Hollow_Knight%29)

**Metroid Dread (2021) [official/press]**
- The map moved from simple cells to detailed outlines of the rooms; it shows item types at all times (dimmed once collected), glowing spots hinting at hidden items, colour-coded doors, a function to highlight all doors a given ability opens, and up to six player-placed markers. — [GamingBolt summary of Nintendo's Dread Report](https://gamingbolt.com/metroid-dread-map-improvements-include-markers-icon-highlighting-hints-and-more); [Nintendo, Metroid Dread Report Vol. 3](https://metroid.nintendo.com/dread/news/metroid-dread-report-vol-3)

**Path of Exile and Diablo IV [snippet only]**
- Path of Exile's Fall of Oriath update (2017) added an optional walkability layer to the minimap marking crossable and non-crossable ground, plus an exploration line marking walkable ground not yet visited. [snippet; page fetch failed] — [Future Game Releases, PoE minimap preview](https://www.futuregamereleases.com/2017/05/path-of-exile-the-fall-of-oriath-minimap-preview/)
- Diablo IV launched with only a corner minimap and no Diablo II-style overlay automap; Blizzard was reported as having "no plans at the moment", and a German outlet reports the overlay returning with the Lord of Hatred expansion. [snippet] — [MP1st](https://mp1st.com/news/diablo-4-tab-map-overlay-blizzard-has-no-plans-at-the-moment); [GameStar](https://www.gamestar.de/artikel/diablo-4-automap-overlay-lord-of-hatred,3452089.html)

### Inferences
- Summary classification (first three are documented, the rest inferred from behaviour):
  - Diablo, Diablo II: cell grid of wall/door glyphs, keyed from tile ID. Outline of walls, not rooms.
  - Super Metroid: coarse cell grid (one cell per screen), hand-authored area tilemap, bit per cell.
  - Isaac: room grid; the generator's floor plan is the minimap. Effectively a room graph embedded in a grid.
  - Dead Cells: the level is a tile grid assembled from room templates, so an explored-tile minimap can be rendered straight from the collision grid. Inference; not stated by Motion Twin.
  - Hollow Knight: per-room map sprites revealed by room. Inference from behaviour (rooms appear whole after a bench rest); I could not confirm the internal representation. My unverified recollection of the decompiled game is a `GameMap` object with one sprite per scene and `scenesVisited` / `scenesMapped` lists in save data, but the search did not confirm these names.
  - Path of Exile: outline of walkable space, presumably from the same walkability grid used for pathing. Inference.
- The decisive design variable is map resolution relative to level resolution: per collision tile (Diablo), per screen (Super Metroid), per room (Isaac, Hollow Knight). Coarser maps need less storage and less authoring but cannot show intra-room detail.
- In tile-keyed systems the authored cost is per tileset (one `.AMP` file or one block of `AutoMap.txt` rows), independent of the number of levels. That is the property that makes them work with procedural generation.

### Gaps
- Hades: no source found on its room-to-room presentation or any map; it has no persistent automap that I could document. Not researched further.
- Diablo III, Diablo IV: no developer source found on how the minimap is produced (navmesh, scene tiles or baked textures).
- Castlevania: Symphony of the Night: not researched in this pass.
- Dead Cells minimap, Hollow Knight map internals, Metroid Dread map generation (whether outlines are derived from collision): no primary source found.
- I did not verify the Diablo II claims against game binaries; they rest on community file guides.

## Group B: how are floors represented and switched, what is marked, and is the plan hand-drawn or derived from geometry?

### Takeaway
I could document player-visible behaviour for Resident Evil 2 Remake, Prey and Control, but found no developer statement for any Group B game on whether the floor plans are hand-drawn or derived from geometry. The clearest lesson is negative: Control shipped with a single-layer map of a stacked building and was criticised for it, while Resident Evil 2 Remake's per-room state colouring and door/lock annotations were singled out as the model to copy.

### Cited Findings

**Resident Evil 2 Remake (2019) [press]**
- Room fill colour encodes completion: red for rooms not fully explored, blue for rooms cleared of items and collectibles. — [PCGamesN, Resident Evil 2 Remake's map](https://www.pcgamesn.com/resident-evil-2/resident-evil-2-remake-capcom-map)
- Items the player has seen but not picked up are recorded as icons identifying the item at its location (example: gunpowder in the east office). — [PCGamesN](https://www.pcgamesn.com/resident-evil-2/resident-evil-2-remake-capcom-map)
- Doors record their lock type once encountered (for example the spade emblem), not just locked or unlocked; unsolved puzzles, safes and padlocks are also annotated for return visits. — [PCGamesN](https://www.pcgamesn.com/resident-evil-2/resident-evil-2-remake-capcom-map)
- The building is presented as separate floors (the article discusses the police station's first, second and third floors, then sewers and laboratory as separate areas). — [PCGamesN](https://www.pcgamesn.com/resident-evil-2/resident-evil-2-remake-capcom-map)
- The article contains no Capcom quotes about the map; the assessment is the journalist's. — [PCGamesN](https://www.pcgamesn.com/resident-evil-2/resident-evil-2-remake-capcom-map)

**Prey (2017) [guide/press, snippet level]**
- The map shows the player's location and current floor, a map of the current area, and a station-level map showing how sections connect. Each area starts dark and reveals only explored parts; a security-station terminal can download the complete area map. Tracked crew members and optional objectives appear as markers. [snippet] — [Prima Games Prey eGuide, Talos I equipment orientation](https://primagames.com/eguides/prey-eguide/introduction/talos-i-equipment-orientation); [Prima Games, gameplay fundamentals](https://primagames.com/eguides/prey-eguide/introduction/gameplay-fundamentals)
- Talos I is split into separately loaded areas. [snippet] — [PowerPyx Prey review](https://www.powerpyx.com/prey-ps4-review/)

**Control (2019) [press, counter-example]**
- A launch review states the FBC headquarters has multiple floors "but your map doesn't distinguish between them". — [We Got This Covered, Control review](https://wegotthiscovered.com/gaming/control-review/)
- Remedy acknowledged map problems after launch and said it was working to improve how the in-game map loads so that all relevant information displays immediately and correctly. — [Stevivor, Control map fix incoming](https://stevivor.com/news/control-map-fix-incoming-says-remedy); [DSOGaming](https://www.dsogaming.com/news/remedy-will-bring-rtx-improvements-motion-blur-option-and-better-in-game-map-to-control)

**The Witcher 3 (2015), a documented geometry-derived interior map [snippet]**
- CD Projekt Red's REDkit documentation has a "HOW-TO: Generate minimaps" page; the search summary says the editor generates minimap images for building interiors from navmesh masks, and that caves use the world navmesh directly. The page body was truncated when fetched, so the procedure is unconfirmed. — [REDkit wiki, HOW-TO: Generate minimaps](https://cdprojektred.atlassian.net/wiki/spaces/W3REDkit/pages/51904544/HOW-TO+Generate+minimaps)

### Inferences
- Resident Evil 2 Remake's map implies a data model with: rooms as first-class polygons with a three-state status (unvisited, visited with items remaining, cleared); doors as entities with a lock type and a discovered flag; and item pickups registered with a "seen" flag distinct from "collected". The "cleared" colour requires every pickup to be assigned to exactly one room. This is inference from behaviour; Capcom has not, to my knowledge, described it.
- Room shapes that exactly match clean architectural floor plans, with consistent door symbols, suggest authored 2D room polygons (or polygons traced from the level) rather than a raw projection of collision. Inference only.
- Control's failure mode suggests that a single top-down projection of a vertically stacked building is unreadable without either floor slicing or a height-based filter, and that streaming the map texture late is itself a usability bug.
- The Witcher 3 lead, if confirmed, would be the only documented shipped pipeline in this study where interior plans are generated from navmesh rather than drawn.

### Gaps
- No developer source found for: Resident Evil 2 Remake, Prey, Control, Deus Ex: Mankind Divided, Dishonored, Persona 5, Baldur's Gate 3. Searches for the latter four returned nothing relevant; they are not covered.
- How floors are switched (automatic by player height versus manual paging) is not documented in any source I read. My unverified recollection is that Resident Evil 2 Remake and Prey both follow the player's current floor and allow manual paging between floors.
- Whether any of these plans are hand-drawn or derived from geometry is unconfirmed for every Group B game.
- The REDkit minimap page should be read directly in a browser; it is the most promising unexplored primary source.

## For procedurally generated levels, how is the map generated at run time from the generated layout?

### Takeaway
In the documented cases the map is not generated as a separate step. The generator's own output (a tile grid or a room grid) is the map's geometry, and a small authored table translates level elements into map glyphs.

### Cited Findings
- Diablo: on level load, `InitAutomap()` loads the tileset's `.AMP` table and clears explored state; at draw time each explored dungeon cell is looked up through `automaptype` and drawn as lines. No per-level map asset exists. — [devilution automap.cpp](https://github.com/diasurgical/devilution/blob/master/Source/automap.cpp)
- `GetAutomapType()` applies neighbour-dependent fix-ups at draw time (a type-7 tile becomes a column when its left neighbour has a horizontal arch and its upper neighbour a vertical arch), so some context sensitivity is resolved in code, not in the table. — [devilution automap.cpp](https://github.com/diasurgical/devilution/blob/master/Source/automap.cpp)
- Diablo II: `AutoMap.txt` rows match on level type, tile orientation, style and sequence, then pick one of up to four `MaxiMap.dc6` frames at random for variety. — [Diablo II Data File Guide](https://d2runewizard.com/game-files/guide.html)
- Isaac: the floor plan is a 9x8 grid with one room per cell and doors implied by adjacency, so the generator's occupancy grid already contains everything a room-level minimap needs; special-room types are assigned to dead ends during generation. — [Boris the Brave](https://www.boristhebrave.com/2020/09/12/dungeon-generation-in-binding-of-isaac/)
- Rebirth's multi-cell rooms mean one room can cover several grid cells with doors on specific cells, and room selection considers which neighbours exist so that door-less variants are only used where no door is needed. — [Boris the Brave](https://www.boristhebrave.com/2020/09/12/dungeon-generation-in-binding-of-isaac/)
- Dead Cells: the generator fills a designer-authored graph with hand-made room templates that must match entrance and exit constraints. In a comment, Benard says the shipped algorithm was changed from brute-force retry to matching tile connections. — [Deepnight, Level Design of Dead Cells](https://deepnight.net/tutorial/the-level-design-of-dead-cells-a-hybrid-approach/)

### Inferences
- Two run-time strategies cover the documented cases: (a) tile-to-glyph lookup at draw time, and (b) drawing the generator's room-occupancy grid directly with a per-room type icon. Both need zero per-level authoring.
- A template-based generator (Dead Cells, Isaac) could alternatively ship a pre-authored minimap fragment per room template and stamp it at the room's placement. I found no source confirming any game does this.
- Neighbour-dependent fix-ups (Diablo's arch-to-column rule) are a recurring cost of tile-keyed maps: a tile's correct glyph can depend on adjacent tiles.

### Gaps
- No primary source on how Dead Cells, Hades, Diablo III/IV or Path of Exile build minimap geometry from generated layouts.

## How is discovery or fog-of-war state stored and revealed (per room, per cell, per radius)?

### Takeaway
Documented storage is always a flat bitmap or boolean grid at the map's own resolution: per dungeon cell in Diablo, per screen-sized map tile in Super Metroid (256 bytes per area). Reveal is triggered by the player occupying a cell, with small neighbour rules for walls; "map item" pickups are stored as a separate flag, not by setting the explored bits.

### Cited Findings
- Diablo: `SetAutomapView(x, y)` converts a world position to a map cell with `(x - 16) >> 1`, sets `automapview[xx][yy] = TRUE`, then reveals specific direct neighbours depending on the cell's shape type, so that the wall bounding a floor cell appears with it. It is neighbour logic, not a radius. Its caller is outside the file read. — [devilution automap.cpp](https://github.com/diasurgical/devilution/blob/master/Source/automap.cpp)
- Diablo: explored state is reset on level init (`automapview` cleared and `BFLAG_EXPLORED` removed from `dFlags`). — [devilution automap.cpp](https://github.com/diasurgical/devilution/blob/master/Source/automap.cpp)
- Super Metroid: current-area explored tiles are 100h bytes at `$07F7..08F6`, one bit per map tile in a 64x32 layout. Per-area copies are kept at `$7E:CD52..D551`, 256 bytes for each of eight areas (Crateria, Brinstar, Norfair, Wrecked Ship, Maridia, Tourian, Ceres, Debug). — [Super Metroid RAM map](https://patrickjohnston.org/ASM/Lists/Super%20Metroid/RAM%20map.asm)
- Super Metroid: map-station acquisition is a separate one-byte-per-area array at `$7E:D908..0F`, mirrored for the current area at `$0789`. — [Super Metroid RAM map](https://patrickjohnston.org/ASM/Lists/Super%20Metroid/RAM%20map.asm)
- Super Metroid: the save file stores a compacted form, "Exported map data" at `$7E:D91C..DE1B`, with per-area lengths of 74, 72, 76, 18, 66 and 21 bytes for Crateria through Tourian. — [Super Metroid RAM map](https://patrickjohnston.org/ASM/Lists/Super%20Metroid/RAM%20map.asm)
- Super Metroid: station-revealed tiles draw in blue, visited tiles in pink. — [Metroid Wiki, Map](https://metroid.fandom.com/wiki/Map)
- Hollow Knight: reveal is deferred. Visited rooms are added to the drawn map only on resting at a bench, and only if the area map and the Quill are owned. — [Hollow Knight Wiki, Map and Quill](https://hollowknight.wiki/w/Map); [Hollow Knight Wiki, Bench](https://hollowknight.wiki/w/Bench_%28Hollow_Knight%29)
- Resident Evil 2 Remake: per-room state (red not fully explored, blue cleared) plus per-item "seen but not taken" icons and per-door lock annotations recorded on first encounter. — [PCGamesN](https://www.pcgamesn.com/resident-evil-2/resident-evil-2-remake-capcom-map)
- Prey: areas start dark and reveal as explored; a terminal download reveals the whole area map at once. [snippet] — [Prima Games Prey eGuide](https://primagames.com/eguides/prey-eguide/introduction/talos-i-equipment-orientation)
- Isaac Rebirth: per-room `DisplayFlags` distinguish invisible, visible and shadow states. [snippet] — [IsaacDocs summary](https://deepwiki.com/wofsauge/IsaacDocs/3-room-and-level-system)

### Inferences
- The Super Metroid save sizes (74, 72, 76... bytes versus 256 in RAM) indicate the save keeps only the bytes of the bitmap that can ever be set for that area, that is, cells where the authored area map has a tile. This is my reading of the sizes; the routine at `$81:8131` would confirm it.
- A consistent pattern across Super Metroid, Hollow Knight and Prey: "the player has the chart" and "the player has been there" are two separate pieces of state, combined at draw time (blue versus pink; purchased outline versus quill-added rooms; downloaded versus explored).
- Hollow Knight's deferred reveal implies two per-room sets: visited, and committed to the map at the last bench. Inference from behaviour.
- Reveal granularity follows map granularity: cell on occupancy (Diablo), screen cell on entry (Super Metroid), whole room on entry or on commit (Isaac, Hollow Knight, Resident Evil 2 Remake). I found no documented shipped example of radius-based reveal in this pass, though Diablo II and Path of Exile behave that way in play (unconfirmed).

### Gaps
- Diablo II, Diablo III/IV, Path of Exile reveal radius and storage: no source read.
- Hollow Knight save-data field names: unconfirmed.
- Exact meaning of Isaac `DisplayFlags` bits: only seen in a search summary.

## What published talks, postmortems, dev blogs or documentation describe these map systems?

### Takeaway
Very little developer-authored material exists on player-facing map systems specifically. The usable primary sources are reverse-engineered code and modding documentation; developer articles cover level generation and world layout, not the map screen.

### Cited Findings
- Diablo automap, complete implementation in reverse-engineered source. — [devilution automap.cpp](https://github.com/diasurgical/devilution/blob/master/Source/automap.cpp)
- Diablo II `AutoMap.txt` column reference. — [Diablo II Data File Guide](https://d2runewizard.com/game-files/guide.html); related Phrozen Keep guides at [d2mods.info Levels.txt guide](https://www.d2mods.info/forum/kb/viewarticle?a=232) and [LvlPrest.txt guide](https://www.d2mods.info/forum/kb/viewarticle?a=357) (the latter two not read)
- Super Metroid annotated RAM map with all map-related addresses. — [P. Johnston RAM map](https://patrickjohnston.org/ASM/Lists/Super%20Metroid/RAM%20map.asm)
- Binding of Isaac generation analysis written with input from the original and Rebirth programmers. — [Boris the Brave](https://www.boristhebrave.com/2020/09/12/dungeon-generation-in-binding-of-isaac/); community pseudo-code at [Isaac Rebirth wiki, Level Generation](https://bindingofisaacrebirth.wiki.gg/wiki/Level_Generation) (not read in full)
- Dead Cells level generation by its lead designer (2017), covering templates, biome graphs and CastleDB. — [Deepnight](https://deepnight.net/tutorial/the-level-design-of-dead-cells-a-hybrid-approach/)
- Hollow Knight world and map design interview with Ari Gibson and William Pellen (PC Gamer, summarised by Game Developer, 4 August 2017). — [Game Developer](https://www.gamedeveloper.com/design/how-the-i-hollow-knight-i-devs-mapped-out-their-metroidvania-); [PC Gamer original](https://www.pcgamer.com/how-to-design-a-great-metroidvania-map/)
- Metroid Dread map features described by Nintendo. — [Metroid Dread Report Vol. 3](https://metroid.nintendo.com/dread/news/metroid-dread-report-vol-3)
- The Witcher 3 REDkit minimap generation how-to (not successfully read). — [REDkit wiki](https://cdprojektred.atlassian.net/wiki/spaces/W3REDkit/pages/51904544/HOW-TO+Generate+minimaps)
- Resident Evil 2 Remake design analyses exist but are third-party. — [Game Developer, A design examination on Resident Evil 2 Remake](https://www.gamedeveloper.com/design/a-design-examination-on-resident-evil-2-remake) (not read); [PC Gamer, The making of the Raccoon City Police Department](https://www.pcgamer.com/the-making-of-resident-evil-2-remakes-raccoon-city-police-department/) (not read; search summary indicates it covers the building, not the map)

### Inferences
- The absence of talks is itself a finding: map screens appear to be treated as UI implementation detail and rarely get a GDC slot. The best route to ground truth for modern games is modding tools and data-mined assets, not talks.

### Gaps
- I found no GDC Vault talk dedicated to an automap or interior map system for any of the named games. I did not search GDC Vault directly; this is absence in general web search, not proof of absence.
- No 80.lv or Game Developer technical write-up on a Group B map system was found.

## What data does such a map system need?

### Takeaway
The documented systems need surprisingly little: a geometry source shared with the level, a small authored lookup from level elements to map glyphs, an explored bitmap at map resolution, and separate flags for "map obtained". Richer interior maps (Resident Evil 2 Remake) add per-room status, per-door lock state and per-item seen/collected flags.

### Cited Findings
- Tile-to-glyph table per tileset, with shape type and flags for door, arch, grate, stairs and floor texture. — [devilution automap.cpp](https://github.com/diasurgical/devilution/blob/master/Source/automap.cpp); [Diablo II Data File Guide](https://d2runewizard.com/game-files/guide.html)
- Explored grid at level-cell resolution, reset per level. — [devilution automap.cpp](https://github.com/diasurgical/devilution/blob/master/Source/automap.cpp)
- Per-room map position (X, Y on the area map) and area index, so that a room's screens can be mapped to cells of the area map. — [Super Metroid RAM map](https://patrickjohnston.org/ASM/Lists/Super%20Metroid/RAM%20map.asm)
- Per-area explored bitmap (256 bytes per area), per-area "map obtained" byte, and a compacted save representation. — [Super Metroid RAM map](https://patrickjohnston.org/ASM/Lists/Super%20Metroid/RAM%20map.asm)
- Room occupancy grid with room type per cell, multi-cell room shapes, and door positions implied by adjacency or listed per exit. — [Boris the Brave](https://www.boristhebrave.com/2020/09/12/dungeon-generation-in-binding-of-isaac/)
- Room templates tagged with entrance/exit count, purpose and biome, placed according to a graph. — [Deepnight](https://deepnight.net/tutorial/the-level-design-of-dead-cells-a-hybrid-approach/)
- Per-room completion state, per-item seen-not-collected markers with item identity, per-door lock type recorded on encounter, and puzzle/safe reminders. — [PCGamesN](https://www.pcgamesn.com/resident-evil-2/resident-evil-2-remake-capcom-map)
- Player-placed markers (up to six), item-type icons with collected state, door type colouring and an ability-to-door highlight query. — [GamingBolt](https://gamingbolt.com/metroid-dread-map-improvements-include-markers-icon-highlighting-hints-and-more)
- Purchased pin categories and a deferred "commit visited rooms" step. — [Hollow Knight Wiki, Map and Quill](https://hollowknight.wiki/w/Map)
- Current floor indicator, per-area map, and a higher-level connectivity map between areas. [snippet] — [Prima Games Prey eGuide](https://primagames.com/eguides/prey-eguide/introduction/talos-i-equipment-orientation)

### Inferences
A consolidated schema implied by the above (synthesis, not from any single source):

- **Static, per level or per generated layout**
  - Geometry at map resolution: either a cell grid with a glyph type per cell, or room polygons.
  - Room list: id, footprint (cells or polygon), room type, floor index.
  - Adjacency / door list: door id, the two rooms it joins, position, door kind (normal, locked-by-key-type, one-way, ability-gated, secret).
  - Floor table: floor index, name, and a height range so the current floor can be picked from the player's elevation. No source read documents height ranges explicitly; Control's criticised single-layer map is the evidence that floor separation is needed.
  - Static icon list: save points, stairs and elevators (with the floor they lead to), shops, fast travel.
  - Glyph lookup table per tileset (for tile-keyed maps).
- **Dynamic, saved per playthrough**
  - Explored state at map resolution: bit per cell, or flag per room.
  - "Chart obtained" flag per area or floor, kept separate from explored state.
  - Per-room status beyond visited: items remaining versus cleared.
  - Per-door state: seen, lock type known, unlocked.
  - Per-item state: seen, collected.
  - Player markers and tracked objectives.
- **Sync guarantee**
  - In every documented case the map stays in sync because it reads the same tile, room or grid data the level is built from, at run time or at build time. The only authored map-specific data is small and reusable (glyph tables, per-room map coordinates). A hand-drawn plan with no link to level data would need a manual update on every level change; I found no documented shipped example of that workflow, though it may well exist in Group B games.

### Gaps
- No source documents floor height ranges, stair/elevator link records or how the current floor is chosen in any shipped multi-floor game.
- No source documents how room polygons are produced for Resident Evil 2 Remake-style maps (traced by artists, exported from level editor volumes, or derived from navmesh).
- Icon and marker data formats are undocumented in everything I read; the list above is derived from visible behaviour.
