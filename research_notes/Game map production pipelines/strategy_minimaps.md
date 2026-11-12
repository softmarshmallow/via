# Strategy / RTS / MOBA minimaps: what they show, how the image is produced, how it stays in sync

Games covered: Warcraft III, StarCraft II, Dota 2, League of Legends, Company of Heroes 2/3, with partial coverage of Age of Empires II and IV.

Provenance key used throughout (read this before citing anything):
- **[fetched]** = I retrieved the page and the claim is in its text.
- **[snippet]** = the claim comes from a search-engine summary of the page; the page itself returned 403/504/redirect when fetched, so wording and context are unverified. Treat as "probably right, check before quoting".
- Anything not carrying a source link is in an Inferences or Gaps block.

Blocked sources worth a manual visit: the Valve Developer Wiki minimap page (403), all Hive Workshop threads (403), SC2Mapster forum threads (now redirect to CurseForge), Liquipedia (403), technology.riotgames.com (504; the riotgames.com mirrors worked).

## 1. What does the minimap show in each game?

### Takeaway
Sourced detail on minimap *content* is thin; documentation concentrates on the background image and the icon overlay. What is documented shows a consistent two-layer split: a static terrain picture plus dynamic markers (start locations, resources, neutral camps, units, wards) and a fog layer on top.

### Cited Findings
- **Warcraft III**: the map archive carries a separate minimap-overlay file, `war3map.mmp`, holding icons for start locations, gold mines and creep camps, each with a position, colour and transparency on a 256x256 canvas. A community API enumerates icon types Gold = 0, NeutralBuilding = 1, StartLoc = 2. Icons appear clipped about 8 px from the border (usable range roughly 8 to 247), described as an observation, not documented behaviour. [snippet] — [Hive Workshop, Lua MMP API](https://www.hiveworkshop.com/threads/lua-mmp-api.370218/)
- **Warcraft III**: the gowarcraft3 library exposes `MenuMinimap` (background plus icons) and `MinimapIcons` (icon layer only), confirming the background and the icon overlay are stored separately. [snippet] — [gowarcraft3 w3m package](https://pkg.go.dev/github.com/nielsad/gowarcraft3/file/w3m)
- **Warcraft III**: the same generated image doubles as the lobby/loading "poster"; the game looks for `war3mapPreview.tga`, then `war3mapMap.tga`, then `war3mapMap.blp`. [snippet] — [Hive Workshop, parsing metadata from w3m/w3x](https://www.hiveworkshop.com/threads/parsing-metadata-from-w3m-w3x-w3n.322007)
- **League of Legends**: the minimap is darkened by the same fog-of-war texture as the 3D scene ("apply it to darken appropriate areas of the game and minimap"). The HUD costs about 1,000 triangles per frame, "nearly 300 for the minimap", which implies the minimap is drawn as textured quads (background plus per-icon sprites), not as a second 3D render of the scene. [fetched] — [Riot, A Trip Down the LoL Graphics Pipeline](https://www.riotgames.com/en/news/trip-down-lol-graphics-pipeline)
- **League of Legends**: wards are minimap-visible; a 2014 Summoner's Rift update PBE note adjusted minimap brightness specifically so wards were easier to see. [snippet] — [Surrender at 20, SR update PBE notes](https://www.surrenderat20.net/2014/06/update-to-summoners-rift-headed-to-pbe.html)
- **Dota 2**: Valve's docs separate the static image from "dynamic icons"; the per-map settings file affects the dynamic icons (their scale/position mapping), not the image. [snippet] — [Valve Developer Wiki, Dota 2 Workshop Tools / Minimap](https://developer.valvesoftware.com/wiki/Dota_2_Workshop_Tools/Level_Design/Dota/Minimap)
- **Dota 2**: a "Simple" minimap variant exists as a separate shipped image asset alongside the normal one (assets for 7.00, pre-7.23, 7.29, 7.33). [snippet] — [Liquipedia, Minimap](https://liquipedia.net/dota2/Minimap); [Liquipedia asset, 7.00 simple map](https://liquipedia.net/commons/File:Gamemap_7.00_simplemap_dota2_gameasset.png)
- **Age of Empires II**: community tools reproduce the minimap from a fixed terrain-id-to-colour table (`terrainColors.json`), i.e. one colour per tile terrain class. [snippet] — [Marfullsen/AoE2-minimap-generator](https://www.github.com/Marfullsen/AoE2-minimap-generator)
- **Age of Empires II**: each map tile stores `terrain_id`, `elevation` and `layer`. [snippet] — [AoE2ScenarioParser map cheatsheet](https://ksneijders.github.io/AoE2ScenarioParser/cheatsheets/map)
- **Age of Empires II**: players disagree in an official-forum thread on whether the minimap encodes elevation: one says it shows only terrain types and some resources (gold, stone), another says elevation appears as shading. Neither is sourced. [snippet] — [AoE forum, Simplify minimap colors](https://forums.ageofempires.com/t/simplify-minimap-colors/120854)
- **Age of Empires IV**: the Content Editor has a Minimap panel showing "an active minimap of your map, as it would appear in game". [snippet] — [AoE support, Content Editor Basics](https://support.ageofempires.com/hc/en-us/articles/4498212029076-Content-Editor-Basics)

### Inferences
- The common content model across these games is: (a) static background, (b) static-ish overlay icons authored with the map (start locations, resource nodes, neutral camps, capture points), (c) live team-coloured unit/building markers, (d) fog/exploration mask, (e) transient pings and the camera frustum box. Only (a), (b) and (d) are directly evidenced above; (c) and (e) are common knowledge of these games but I did not retrieve a citable primary source for them.
- LoL drawing the minimap in about 300 triangles strongly suggests background quad + icon quads + fog quad.

### Gaps
- No primary source retrieved that enumerates the full minimap content list (units, buildings, pings, camera frustum) for StarCraft II, Warcraft III, Dota 2 or LoL. These are observable in-game but uncited here.
- Whether AoE II's minimap shades by elevation: unresolved, conflicting forum claims only.
- Civilization VI and Total War were not researched (scope cut for depth).

## 2. How is the minimap background produced (auto from terrain, orthographic capture, or hand-painted)?

### Takeaway
Three production models are evidenced: tile-colour lookup (AoE II), automatic render/capture by the editor (Warcraft III, StarCraft II, Dota 2, CoH, AoE IV), and a captured image that is then hand-edited or replaced by stylised art (Dota 2, CoH 2/3). The belief that Dota 2 is *purely* hand-painted needs correcting: Valve documents a generated top-down capture that may then be edited; for League of Legends I found no source confirming hand-painting either way.

### Cited Findings
- **Dota 2**: "The minimap in Dota 2 is an image generated from a top-down perspective, high above the map." It can be edited directly, but it is static and must be re-exported after geometry changes. [snippet] — [Valve Developer Wiki, Minimap](https://developer.valvesoftware.com/wiki/Dota_2_Workshop_Tools/Level_Design/Dota/Minimap)
- **Dota 2**: capture volume is defined by two `dota_minimap_boundary` entities (XY bounds; Z distance between them sets capture depth); image is produced with the console command `dota_minimap_create`; output goes to `materials/overviews`. [snippet] — [Valve Developer Wiki, Minimap](https://developer.valvesoftware.com/wiki/Dota_2_Workshop_Tools/Level_Design/Dota/Minimap)
- **Dota 2 (legacy Source 1 flow)**: image file plus a key-value file named after the map that defines minimap bounds; image converted to `.vtf` with VTFEdit. [snippet] — [Valve Developer Wiki, Legacy Level Design](https://developer.valvesoftware.com/wiki/Dota_2_Workshop_Tools/Legacy/Level_Design); [Valve Developer Wiki, Dota 2 Maps](https://developer.valvesoftware.com/wiki/Dota_2_Maps)
- **Warcraft III**: World Editor writes `war3mapMap.blp` into the map archive itself; a BLP-format reference singles out the "WorldEdit generated war3mapMap.blp (mini-map) file" as using its own compression setting. [snippet] — [Hive Workshop, BLP specifications](https://www.hiveworkshop.com/threads/blp-specifications-wc3.279306)
- **StarCraft II**: the map contains a reserved `Minimap.tga` (visible in the Import Manager with "Show Reserved"), generated by the editor. A poster assumed it is rendered using the tileset's light source and possibly the light map; one user saw the minimap go black except for water with a custom tileset. All forum-level, uncertain. [snippet] — [SC2Mapster, black terrain minimap in game](https://www.sc2mapster.com/forums/development/terrain/147238-black-terrain-minimap-in-game); [SC2Mapster, minimap broken](https://sc2mapster.com/forums/development/miscellaneous-development/172086-minimap-broken-o-o)
- **StarCraft II**: the Terrain Editor also has a manual "Data -> Export map image" used by modders to build scripted/custom minimaps; a tutorial recommends an image at least 4x the map size. [snippet] — [SC2Mapster tutorial thread](https://www.sc2mapster.com/forums/tutorials/179217-data-skin-tutorial/previous)
- **Company of Heroes 2**: per a message relayed from Relic's WorldBuilder team, a custom minimap is supplied as `<MyScenario>_mm_low.tga` and a custom lobby preview as `<MyScenario>_mm_preview.tga` (usually the same picture). If the image has a border, `minimap_size` in the scenario's `.options` file is the pixel size of the playable area (image size minus border). [fetched] — [coh2.org, Worldbuilder basic information by Relic](https://www.coh2.org/topic/10593/worldbuilder-basic-information-by-relic/page/1)
- **Company of Heroes 2**: forum replies state the WorldBuilder-created `_mm.tga` is typically 768x768 for square maps, with the preview at 384x384, and that modders export the WorldBuilder image and repaint it in GIMP. [snippet] — [coh2.org thread replies](https://www.coh2.org/topic/10593/title/post/95599)
- **Company of Heroes 3**: the editor emits a `_mm_generated` TGA in the map folder; guides advise copying it as the base for a hand-drawn version, using an orthographic top-down view of the map as reference; a "Stylized Minimap" option in scenario properties selects a stylised pipeline instead of a hand-supplied TGA. The in-game minimap is reported to crop the bottom-right slightly. [snippet] — [gamenguides, custom minimap for CoH3](https://www.gamenguides.com/how-to-create-a-custom-minimap-for-company-of-heroes-3); [gameplay.tips, CoH3 proper minimap](https://gameplay.tips/guides/company-of-heroes-3-how-to-give-your-map-a-proper-minimap-essence-editor.html)
- **Age of Empires IV**: the editor writes a minimap TGA into the local map folder when the map is built; a bug report describes it sometimes coming out empty or purple and only a few KB, with the workaround of screen-grabbing the editor minimap, editing it and saving as TGA. A staff member acknowledged the report. [fetched] — [AoE forum, Essence Editor minimap TGA on Crafted Maps](https://forums.ageofempires.com/t/essensce-editor-minimap-tga-file-on-crafted-maps/226576)
- **Age of Empires II**: minimap colours appear to be data-driven per terrain; modders located palette files that seem to hold minimap colours but could not confirm the game reads them. [snippet] — [AoE forum, How can I mod the mini-map colors](https://forums.ageofempires.com/t/how-can-i-mod-the-mini-map-colors-design/85692)
- **League of Legends**: Riot described the 2014 Summoner's Rift update as a "handcrafted art style"; this refers to the map, not specifically the minimap. [snippet] — [Riot dev blog archive, Updating Summoner's Rift](https://archive.runeterra.net/na/riot-official/97BoAHLR)

### Inferences
- Dota 2's shipped minimap is best described as "captured, then art-directed": Valve's tooling produces an orthographic-style capture, and the existence of a separately shipped "Simple" image shows that final assets are authored images rather than live renders. How much hand-painting Valve applies to the official map is not documented in anything I retrieved.
- LoL's minimap is almost certainly a static authored texture (it is drawn as HUD quads and only darkened by fog), but "hand-painted" is unconfirmed.
- The Relic lineage (CoH2, CoH3, AoE IV all on Essence) shares one pattern: editor auto-generates a TGA, and a same-named override file or a stylised pipeline replaces it.
- AoE II is the only game here where the minimap is plausibly computed at runtime straight from tile data rather than from a baked image; consistent with random-map generation, where no editor step exists to bake one.

### Gaps
- No Blizzard documentation found describing how World Editor or the SC2 editor renders the minimap image (top-down render vs texture-colour average per cell). The lighting hypothesis for SC2 is one forum poster's guess.
- Resolution of SC2's `Minimap.tga` and confirmation of 256x256 for `war3mapMap.blp` (256x256 is evidenced only for the `.mmp` icon canvas).
- No source confirming or refuting that Riot hand-paints the LoL minimap. No GDC talk on minimap production was found for any of these games.
- I could not read the Valve wiki page in full; the option name "Create minimap on load" and the settings-file path `resource/overviews` come from a search summary.

## 3. How do map editors generate the minimap for a custom map? Is it automatic on save?

### Takeaway
Blizzard's editors regenerate the minimap automatically on every save and overwrite the reserved file; Dota 2 uses an explicit console command (with an optional auto-create-on-load mode); Relic editors generate on save/export/build. In every case a hand-made override must use a different filename or mechanism, or it gets clobbered.

### Cited Findings
- **Warcraft III**: importing a custom image as `war3mapMap.blp` fails because World Editor replaces that file each time the map is saved; the workaround is a differently named import plus `BlzChangeMinimapTerrainTex` in script. One later thread reports both the old rename trick and the script route failing in Reforged. [snippet] — [Hive Workshop, How to change in-game minimap properly](https://www.hiveworkshop.com/threads/how-to-change-in-game-minimap-properly.348865/); [Hive Workshop, in-game minimap no longer changing](https://www.hiveworkshop.com/threads/in-game-minimap-no-longer-changing.356773/latest)
- **Warcraft III**: stripping `war3map.mmp` with a map optimiser is reported (single user) to cause minimap problems. [snippet] — [Hive Workshop, wc3mapoptimizer and preview trick](https://www.hiveworkshop.com/threads/100-file-size-loss-after-using-wc3mapoptimizer-and-doing-the-preview-trick.359153)
- **StarCraft II**: "when a map is saved, the minimap is generated according to the map bounds"; changing camera bounds and saving triggers regeneration, though not always reliably. Reported fixes for a black minimap: restart the editor, generate a light map from the Terrain Editor's Data menu then save, or make a small terrain change and save. [snippet] — [SC2Mapster, black terrain minimap in game](https://www.sc2mapster.com/forums/development/terrain/147238-black-terrain-minimap-in-game); [SC2Mapster, minimap broken](https://sc2mapster.com/forums/development/miscellaneous-development/172086-minimap-broken-o-o)
- **StarCraft II**: Map Info lets you set custom Small/Large preview images (DDS or TGA); setting the Large image was observed to also replace the in-game minimap background. Directly overriding reserved `Minimap.tga` gave one user a 16x16 empty image. [snippet] — [SC2Mapster forum thread](https://www.sc2mapster.com/forums/miscellaneous-development/172211-issue-with-command-cards/next)
- **Dota 2**: workflow is place two `dota_minimap_boundary` entities, compile, launch the map, run `dota_minimap_create`. An auto-generation option ("Create minimap on load") exists and will overwrite an existing, possibly hand-edited, minimap image. [snippet] — [Valve Developer Wiki, Minimap](https://developer.valvesoftware.com/wiki/Dota_2_Workshop_Tools/Level_Design/Dota/Minimap)
- **Company of Heroes 2**: saving a scenario generates several files; custom files are bundled via File -> Export Package. [fetched] — [coh2.org, Worldbuilder basic information by Relic](https://www.coh2.org/topic/10593/worldbuilder-basic-information-by-relic/page/1). A forum reply adds that the WorldBuilder-created `_mm.tga` is overwritten on export, which is why overrides use `_mm_low.tga`. [snippet] — [coh2.org thread replies](https://www.coh2.org/topic/10593/title/post/95599)
- **Age of Empires IV**: minimap TGA is (re)produced when the crafted map is compiled; "recompile several times" is the suggested retry when it comes out empty. [fetched] — [AoE forum bug report](https://forums.ageofempires.com/t/essensce-editor-minimap-tga-file-on-crafted-maps/226576)

### Inferences
- "Regenerate on every save, into a reserved filename" is the dominant design. Its recurring failure mode, across three studios, is clobbering hand-made art or silently producing a blank image. A derived-artifact design should keep generated and authored images under distinct names with an explicit precedence rule (as CoH2 and Warcraft III's poster lookup order do).
- Dota 2's capture requires the map to be compiled and running in-engine, so the minimap is a function of the rendered world, not of source map data; this is why it can go stale after geometry edits.

### Gaps
- Age of Empires II scenario editor: no source found describing any minimap bake step (consistent with runtime generation, but unconfirmed).
- Whether SC2 generation happens strictly on save or also on publish: not established.

## 4. How are fog of war and exploration state represented on the minimap?

### Takeaway
The only engineering-level source found is Riot's: a low-resolution CPU visibility grid (128x128) upscaled to a 512x512 texture, blurred, and used as a darkening multiplier on both the world and the minimap. No comparable published description was found for the Blizzard, Valve or Relic games.

### Cited Findings
- LoL fog is "maintained by the CPU as a 128x128 grid which is then upscaled to a 512x512 square texture", then blurred and applied "to darken appropriate areas of the game and minimap". Fog and shadow textures are prepared before any scene drawing. [fetched] — [Riot, A Trip Down the LoL Graphics Pipeline](https://www.riotgames.com/en/news/trip-down-lol-graphics-pipeline)
- The 128x128 grid covers the entire map; raising it to 256x256 was rejected because it would have "quadrupled CPU cost". [fetched] — [Riot, A Story of Fog and War](https://www.riotgames.com/en/news/story-fog-and-war)
- Upscale method: each source cell becomes a 4x4 block chosen from 16 patterns based on a 2x2 neighbourhood, with grey anti-aliasing pixels at edges, followed by a blur on the 512x512 result; the earlier pipeline used a non-separable Gaussian blur on the 128x128 data. [fetched] — [Riot, A Story of Fog and War](https://www.riotgames.com/en/news/story-fog-and-war)
- Visibility is stored as a simple per-cell flag rather than a gradient; the old code multiplied visible areas by about 0.9 instead of 1.0, a bug fixed in the rework. [fetched] — [Riot, A Story of Fog and War](https://www.riotgames.com/en/news/story-fog-and-war)

### Inferences
- The minimap fog is a second consumer of the same mask texture as the world fog, not separately computed state. This is the cheapest way to guarantee the two never disagree.
- LoL has only two states (visible / not visible) because the map layout is always known. RTS games with an explored-but-not-visible state (Warcraft III, StarCraft II, AoE) need at least three states per cell (unexplored, explored, visible); that is standard genre behaviour but I have no citable engine source for the storage format.

### Gaps
- No documented fog/exploration representation for Warcraft III, StarCraft II, Dota 2, Company of Heroes or Age of Empires minimaps (grid resolution, number of states, how last-seen buildings are remembered).
- Riot's articles do not say what sampling or resolution the minimap uses when applying the fog texture.

## 5. What published talks, docs or dev blogs describe these systems?

### Takeaway
Primary documentation exists for Dota 2 (Valve wiki), LoL fog (two Riot engineering posts) and CoH2 file conventions (Relic team message); Blizzard's editors are documented almost entirely by community wikis and forums. I found no GDC talk dedicated to minimap production.

### Cited Findings
- Valve Developer Wiki, "Dota 2 Workshop Tools / Level Design / Dota / Minimap": official, covers generation, entities, console command, overview settings. (403 to automated fetch.) — [link](https://developer.valvesoftware.com/wiki/Dota_2_Workshop_Tools/Level_Design/Dota/Minimap)
- Riot Games Technology blog, "A Story of Fog and War" (Jaewon Jung): fog grid, upscaling, blur. — [link](https://www.riotgames.com/en/news/story-fog-and-war); original at [technology.riotgames.com/node/39](https://technology.riotgames.com/node/39)
- Riot Games Technology blog, "A Trip Down the LoL Graphics Pipeline" (Tony Albrecht): frame breakdown including fog texture and HUD/minimap triangle counts. — [link](https://www.riotgames.com/en/news/trip-down-lol-graphics-pipeline)
- Relic WorldBuilder team message relayed on coh2.org: minimap/preview file naming and `minimap_size`. — [link](https://www.coh2.org/topic/10593/worldbuilder-basic-information-by-relic/page/1)
- Age of Empires IV official Content Editor guides (Minimap panel; terrain layout tool). — [Content Editor Basics](https://support.ageofempires.com/hc/en-us/articles/4498212029076-Content-Editor-Basics); [Terrain Layout Tool](https://support.ageofempires.com/hc/en-us/articles/4544564998420-Using-the-Terrain-Layout-Tool)
- Community format references for Warcraft III: Hive Workshop wiki `war3map.w3e` terrain spec and BLP spec; gowarcraft3 Go library. — [w3e wiki](https://wiki.hiveworkshop.com/index.php/War3map.w3e_-_Terrain); [BLP spec](https://www.hiveworkshop.com/threads/blp-specifications-wc3.279306); [gowarcraft3](https://pkg.go.dev/github.com/nielsad/gowarcraft3/file/w3m)
- Community SC2 editor documentation (not Blizzard-authored). — [s2editor-guides](https://s2editor-guides.readthedocs.io/Classic_Tutorials/01_Terrain_Module/1/)
- Microsoft patent on random map generation in a strategy game (Age of Empires lineage); surfaced in search, not read, relevance to minimaps unverified. — [US7589742B2](https://patents.google.com/patent/US7589742)

### Inferences
- Minimap generation is treated by studios as editor plumbing rather than a publishable technique; the richest information comes from file-format reverse engineering and modder bug reports.

### Gaps
- No GDC Vault talk located on minimap production for any covered game (not exhaustively searched: one-pass study).
- No official Blizzard documentation located for `war3mapMap.blp`, `war3map.mmp` or SC2 `Minimap.tga`.

## 6. What data does such a minimap need?

### Takeaway
The shipped systems reduce to five inputs: a background image (baked or derived from per-cell terrain class), a world-to-minimap transform defined by playable bounds, a list of authored static icons, live entity positions with team colour, and a low-resolution visibility mask shared with the main view.

### Cited Findings
- **World-to-map transform from bounds**: Dota 2 uses two boundary entities for XY extent plus a per-map settings file for icon scale/position [snippet] — [Valve wiki](https://developer.valvesoftware.com/wiki/Dota_2_Workshop_Tools/Level_Design/Dota/Minimap). SC2 generates the minimap "according to the map bounds" [snippet] — [SC2Mapster](https://www.sc2mapster.com/forums/development/terrain/147238-black-terrain-minimap-in-game). CoH2 stores `minimap_size` (playable-area pixels, excluding border) in `.options` [fetched] — [coh2.org](https://www.coh2.org/topic/10593/worldbuilder-basic-information-by-relic/page/1).
- **Static icon layer stored as data, separate from the image**: Warcraft III `war3map.mmp` with per-icon type, position, colour, transparency on a 256x256 canvas. [snippet] — [Hive Workshop, Lua MMP API](https://www.hiveworkshop.com/threads/lua-mmp-api.370218/)
- **Terrain class per cell to colour**: AoE II tiles carry `terrain_id` and `elevation`; community renderers use a fixed terrain-colour table. [snippet] — [AoE2ScenarioParser](https://ksneijders.github.io/AoE2ScenarioParser/cheatsheets/map); [AoE2 minimap generator](https://www.github.com/Marfullsen/AoE2-minimap-generator)
- **Fog mask**: LoL 128x128 CPU grid, upscaled to 512x512, blurred, shared between world and minimap. [fetched] — [Riot graphics pipeline](https://www.riotgames.com/en/news/trip-down-lol-graphics-pipeline)
- **Readability is a colour-budget problem**: an AoE II DE mod rebalanced all map and player colours against each other for unit/resource legibility; players ask for near-identical terrains to share one colour so woodlines and elevation stand out. [snippet] — [0xDB AoE II DE mods](https://dennisbusch-de.itch.io/0xdb-age-of-empires-ii-de-mods); [AoE forum](https://forums.ageofempires.com/t/simplify-minimap-colors/120854)
- **Capture depth matters for captured backgrounds**: Dota 2's Z distance between boundary entities sets what the top-down capture includes. [snippet] — [Valve wiki](https://developer.valvesoftware.com/wiki/Dota_2_Workshop_Tools/Level_Design/Dota/Minimap)

### Inferences
- Minimum data contract for a functional strategy minimap, synthesised from the above (not stated as a list by any single source):
  1. Background raster: either baked image, or per-cell terrain class (+ optional height for shading, water flag) with a small, deliberately coarse colour table.
  2. Playable-bounds rectangle and resulting affine world-to-minimap transform, stored next to the image so icons and image cannot drift apart.
  3. Authored static markers: start locations, resource nodes, neutral objectives, each with type/position/colour.
  4. Live markers: entity position, owner/team colour, marker class (unit, building, hero), visibility-filtered.
  5. Visibility/exploration mask at coarse resolution (order of 128x128 for a MOBA map), reused by the main view.
  6. Transient overlays: pings, camera frustum quad.
- Pathability and cliff level are not evidenced as direct minimap inputs in any source I retrieved; in the Blizzard editors they would enter only implicitly through the rendered terrain image.
- A coarse palette (few terrain colours, reserved hues for teams and resources) is a functional requirement, not a style choice: background colours must not collide with marker colours.
- Sync risk is highest for baked/captured images (Dota 2 static image must be re-exported; SC2 and AoE IV regenerate unreliably); it is lowest where the minimap is computed from the same cell data the game simulates on (AoE II model).

### Gaps
- No source gives the actual colour-derivation rule in the Blizzard editors (render vs per-tile average), nor whether height or pathing are explicitly encoded.
- Exact schema of Dota 2's overview settings file (key names such as scale/pos) not retrieved because the Valve wiki page blocked fetching.
- No sourced numbers on minimap update rate for live markers in any covered game.
