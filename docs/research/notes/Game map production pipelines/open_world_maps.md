# Player-facing maps in shipped open-world games: content, production, and sync

Scope note: single-pass study, about 25 tool calls. Games covered: The Legend of Zelda: Breath of the Wild (BotW), The Witcher 3 (W3), Elden Ring, Assassin's Creed Mirage (with Valhalla as its stated baseline), Ghost of Tsushima, plus brief Horizon Zero Dawn notes. Evidence quality is very uneven: BotW and W3 have developer-originated documentation of the pipeline; Elden Ring, Ghost of Tsushima and Horizon have essentially none that I could find, only descriptions of the shipped result.

Source-access caveats (apply throughout):
- The BotW CEDEC material was read via an English translation/summary (Matt Walker's thread, mirrored in a GitHub gist) of Japanese press reports of the talks. It is third-hand: talk -> Japanese press report -> English summary. I read the gist text directly; I did not read the Japanese originals or the slides.
- The CD Projekt RED REDkit "Generate minimaps" page is JavaScript-rendered and could not be fetched. Everything attributed to it comes from search-engine extracts of that page, not a full read.
- The AC Mirage ArtStation page returned HTTP 403. Everything attributed to it comes from search-engine extracts.

## 1. What do the world map and minimap show in each game?

### Takeaway
All five world maps show the same core layers (terrain form, water, roads, settlements/POI icons, region labels, a discovery mask, player-placed markers), but differ sharply in cartographic idiom: BotW is a contour/hypsometric "topographic" map, Elden Ring and Mirage are illustrated "parchment" maps, W3 is a painted map with an auto-captured minimap. Two of the five (Elden Ring, Ghost of Tsushima) deliberately ship without a minimap.

### Cited Findings

BotW
- Base map shows contour lines, elevation tinted in shades of brown (darker = lower, lighter = peaks), bathymetric water colouring, buildings, roads and individual tree canopies. — [Stamen, "Cartographers play video games: BotW"](https://stamen.com/cartographers-play-video-games-a-review-of-the-map-in-the-legend-of-zelda-breath-of-the-wild/)
- The map is static with respect to runtime world changes: felled trees stay on the map; the reviewers write that "the map doesn't reflect the current state of the map in real-time." — [Stamen BotW](https://stamen.com/cartographers-play-video-games-a-review-of-the-map-in-the-legend-of-zelda-breath-of-the-wild/)
- Labels are zoom-dependent: zoomed out, "only labels for major regions are shown"; smaller regions and features appear on zooming in. Zoom is "docked to certain levels instead of a more freeform zooming." — [Stamen BotW](https://stamen.com/cartographers-play-video-games-a-review-of-the-map-in-the-legend-of-zelda-breath-of-the-wild/)
- Discovery is per region and binary for the base map: undiscovered regions are covered by a patterned overlay until a Sheikah Tower is activated; some place names appear only after visiting. Shrine/tower icons have three states (unseen, discovered, completed). — [Stamen BotW](https://stamen.com/cartographers-play-video-games-a-review-of-the-map-in-the-legend-of-zelda-breath-of-the-wild/)
- Player annotations: up to 100 stamps, pins that also appear docked to the minimap edge and vanish on arrival, a death marker, and (DLC) Hero's Path, a replayable trace of the whole route walked. — [Stamen BotW](https://stamen.com/cartographers-play-video-games-a-review-of-the-map-in-the-legend-of-zelda-breath-of-the-wild/)
- Minimap pans with the player and zooms in automatically on entering a village to show shops and inns. — [Stamen BotW](https://stamen.com/cartographers-play-video-games-a-review-of-the-map-in-the-legend-of-zelda-breath-of-the-wild/)
- The map is split into 120 dynamically loaded sections with 4 zoom levels, and unopened areas are drawn in separate colours (per the CEDEC 2017 UI talk summary). — [Matt Walker translation of CEDEC 2017 reports, gist mirror](https://gist.github.com/idbrii/e39fe96279aa1670319bfa521d907399)

The Witcher 3
- The minimap is built from tile images, "each image being a screenshot which represents a tile of the world from a bird's-eye view." — [CDPR REDkit wiki, "HOW-TO: Generate minimaps" (search extract)](https://cdprojektred.atlassian.net/wiki/spaces/W3REDkit/pages/51904544/HOW-TO+Generate+minimaps)
- The generator distinguishes exteriors (water, roads, terrain, forest) from interiors (buildings, caves), with masks/folders deciding which layers get tiles. — [CDPR REDkit wiki (search extract)](https://cdprojektred.atlassian.net/wiki/spaces/W3REDkit/pages/51904544/HOW-TO+Generate+minimaps)
- Points of interest on the map were used by the designers as the device for pulling players across the world; the story set each area's size and tone. — [80.lv, "World Building of Witcher 3"](https://80.lv/articles/world-building-of-witcher-3/)

Elden Ring
- Terrain is drawn with hachure marks and coloured land cover in a black-ink-and-watercolour look; land cover hints at hazards such as poison swamps. Brown isolines exist but the reviewers judged them inaccurate and inconsistent. — [Stamen, "Cartographers play video games: Elden Ring"](https://stamen.com/cartographers-play-video-games-a-review-of-the-map-in-elden-ring/)
- Two stacked discovery effects: a cloudy fog-of-war overlay, and beneath it a low-detail base that still shows main roads and where map-fragment pillars are; collecting a fragment fills in the detailed illustration for that area. — [Stamen Elden Ring](https://stamen.com/cartographers-play-video-games-a-review-of-the-map-in-elden-ring/)
- Miyazaki's stated intent: fragments preserve a sense of discovery, and they are easy to obtain because exploring with a map is fun. — [PC Gamer](https://pcgamer.com/elden-rings-open-world-took-a-lot-more-effort-than-previous-souls-games)
- Icons: sites of grace (oriented to show the guidance direction), player beacons, and in the expansion up to 100 player markers in 10 symbols. Overworld and underground are separate toggled map layers. — [Stamen Elden Ring](https://stamen.com/cartographers-play-video-games-a-review-of-the-map-in-elden-ring/)
- No minimap and no dungeon maps; the map is a full-screen menu view that does not pause the game. — [Stamen Elden Ring](https://stamen.com/cartographers-play-video-games-a-review-of-the-map-in-elden-ring/)

Assassin's Creed Mirage
- The world map artist states they helped define the look and feel of the Mirage world map, contrasting it with Valhalla's map, which they describe as a flat two-dimensional design. The portfolio shows fogged and unfogged states, and credits separate POI drawings to Edouard Noisette. — [ArtStation, "Assassin's Creed Mirage - World Map" (search extract; page returned 403)](https://therareaquaticpyro.artstation.com/projects/AlD4EN)

Ghost of Tsushima
- An early design decision was to not use UI elements other open-world games rely on, such as minimaps. — [Game Developer, "A taxonomy of Weenies"](https://gamedeveloper.com/design/a-taxonomy-of-weenies-the-landmarks-that-define-i-ghost-of-tsushima-i-)
- Navigation is by a guiding wind toward the waypoint picked on the map instead of on-screen icons; the lead VFX artist says the aim was for players to navigate by thinking instead of following game UI. — [PlayStation Blog, VFX of Ghost of Tsushima](https://blog.playstation.com/?p=345372)
- The world map starts covered in fog that lifts only where the player has personally travelled (press description, not developer documentation). — [GamingBible](https://www.gamingbible.co.uk/features/games-why-zelda-breath-of-the-wild-has-the-greatest-gaming-map-ever-20200807)

Horizon Zero Dawn (brief)
- UI is split into a holographic style presented by the in-world Focus device and a chalk/cave-drawing style; icons follow the tribal cave-painting idiom (player-side critique, not developer documentation). — [Medium UX critique](https://medium.com/@mrgnarchr/horizon-zero-dawns-use-of-diegetic-and-non-diegetic-uis-73f627c21988)
- An "Indiana Jones style" fast-travel map was built and cut after playtest feedback. — [GamingBolt](https://gamingbolt.com/horizon-zero-dawn-had-an-indiana-jones-style-map-for-fast-travel-but-it-was-cut-out)

### Inferences
- The layer set is nearly constant across games (terrain, water, roads, settlement/POI icons, region labels, discovery mask, player markers). What varies is the rendering idiom and how much of it is art versus data.
- Discovery state comes in at least three distinct granularities in this sample: per-region unlock (BotW towers), per-region unlock on top of a permanently visible low-detail base (Elden Ring fragments), and continuous reveal along the player's path (Ghost of Tsushima).
- Zoom-dependent labelling (BotW) and discrete zoom stops are a map-generalisation decision, the same one a web map makes; the 4 zoom levels in the CEDEC summary match the "docked" zoom the Stamen reviewers observed.

### Gaps
- Minimap rotation (north-up versus heading-up) and zoom behaviour were not documented in any source I read, for any of the games. The Stamen BotW review explicitly does not discuss rotation.
- I did not find a source describing W3's world map content layer by layer (the hub/world map as opposed to the minimap tiles).
- Mirage's ArtStation page could not be opened, so its layer content is known only from a search extract.
- Tears of the Kingdom (sky/surface/depths layers) and Horizon Forbidden West were not researched.
- Whether Horizon Zero Dawn has a compass in place of a minimap was not confirmed by any source in this pass.

## 2. How is the map image produced?

### Takeaway
The only two pipelines with developer-originated documentation are both generated or captured from level data: BotW's map textures were procedurally generated (CEDEC 2017 UI talk), and W3's minimap tiles are top-down editor captures (REDkit docs). For the illustrated-looking maps (Elden Ring, Mirage, Ghost of Tsushima) I found no production documentation; "hand-painted" is how reviewers describe the look, not a confirmed process.

### Cited Findings
- BotW: needing 2,344 distinct map screens (120 sections x 4 zooms, plus per-area unopened variants) "would have been impossible manually, so the map was procedurally generated." Speakers at that session were UI programmer Shigehisa Kitayama and UI lead designer Takahiro Hase. — [Matt Walker translation, gist mirror](https://gist.github.com/idbrii/e39fe96279aa1670319bfa521d907399); speakers per [GAME Watch report of the session](https://game.watch.impress.co.jp/docs/news/1078/846/amp.index.html)
- BotW had only 2 UI designers, which the summary gives as the reason for leaning on programmer-built, data-driven tooling. — [gist mirror](https://gist.github.com/idbrii/e39fe96279aa1670319bfa521d907399)
- W3: minimap tiles are bird's-eye screenshots produced by a Minimap Generator in the editor's Tools menu; it captures the world's current loaded state, so every layer that should appear must be loaded first. — [CDPR REDkit wiki (search extract)](https://cdprojektred.atlassian.net/wiki/spaces/W3REDkit/pages/51904544/HOW-TO+Generate+minimaps)
- W3: CDPR's own docs warn that the raw output is not shippable as-is: making detailed, accurate minimaps "can be challenging" and needs "a lot of external adjustments" to the raw generated images. — [CDPR REDkit wiki (search extract)](https://cdprojektred.atlassian.net/wiki/spaces/W3REDkit/pages/51904544/HOW-TO+Generate+minimaps)
- W3: shipped minimap tiles live under the game depot at gameplay/gui_new/minimaps. — [CDPR REDkit wiki (search extract)](https://cdprojektred.atlassian.net/wiki/spaces/W3REDkit/pages/51904544/HOW-TO+Generate+minimaps)
- W3 predecessor: The Witcher 2 REDkit map tutorial describes hand-made DDS map images dropped into a folder, not a generator. — [Witcher 2 REDkit wiki, map](https://redkitwiki.cdprojektred.com/map.htm)
- Elden Ring: the Stamen reviewers call the look "hand-drawn" and "hand-illustrated" but the article contains no developer statement on production. — [Stamen Elden Ring](https://stamen.com/cartographers-play-video-games-a-review-of-the-map-in-elden-ring/)
- AC Mirage: a named individual artist is credited with the world map look, and another with POI drawings, which establishes that the map was authored art with illustrated POIs. — [ArtStation (search extract)](https://therareaquaticpyro.artstation.com/projects/AlD4EN)

### Inferences
- W3 is a hybrid by its own documentation: captured base plus manual external adjustment. Whether the adjustment is paint-over, colour grading or masking is not stated in what I could read.
- The Witcher 2 -> Witcher 3 change (hand-made DDS maps -> generator) suggests the move to automation was driven by world size, the same argument Nintendo gives explicitly ("impossible manually").
- BotW's clean contour/hypsometric style is what a height-field-derived map naturally looks like, and is far easier to regenerate than an illustrated style. The illustrated maps (Elden Ring, Mirage) trade regenerability for character. This is my reading; no source states the trade-off.
- For Elden Ring, the inaccurate, inconsistent isolines the Stamen reviewers noticed are weak evidence for hand illustration over derivation from height data. Inference only.

### Gaps
- BotW: what "procedurally generated" consumed (height field, material/splat data, road and object placement) and how contours, tree canopies and buildings were rasterised is not described in the summary I read. The slides themselves were not examined.
- W3: whether the large painted world/hub map is a separate hand-painted asset, a paint-over of captures, or derived from the tiles is unconfirmed. A Cook and Becker "World Map" print is credited to an illustrator, but the listing does not say it is the in-game map: [Cook and Becker](https://www.candb.com/en/artwork/1976/the-witcher-3-world-map.html).
- W3 generator specifics (tile pixel size, zoom levels, mask format) are on the REDkit page but I could not read the body.
- Elden Ring, Ghost of Tsushima, Horizon: no developer source on map image production found. Searches of GDC-related coverage, PlayStation Blog and ArtStation for these returned only world-building and environment-art material.
- Mirage: whether the artist painted over a heightmap or engine capture is not known; the page was inaccessible.
- No orthographic-capture-plus-paint-over workflow was found documented step by step for any of the five games.

## 3. How is the map kept in sync when the level changes during production?

### Takeaway
Exactly one documented automation was found: BotW regenerated its map textures for every section every night. W3 has a re-runnable capture tool but with acknowledged manual clean-up afterwards. For every hand-illustrated map, I found nothing on how changes were propagated.

### Cited Findings
- BotW: "textures would be generated for each section every night." — [Matt Walker translation, gist mirror](https://gist.github.com/idbrii/e39fe96279aa1670319bfa521d907399)
- BotW used Jenkins for other content automation (random placement of environmental sound emitters by rule) and automatically merged and placed shrine model parts so polishing could continue late; a build-automation culture existed around the map job. — [gist mirror](https://gist.github.com/idbrii/e39fe96279aa1670319bfa521d907399)
- BotW's UI workflow was changed so designers could preview screens over the running game instead of asset -> viewer -> build -> in-game check. — [gist mirror](https://gist.github.com/idbrii/e39fe96279aa1670319bfa521d907399)
- BotW internally plotted field-related task and QA data on the game map ("field task view") to see completion rates, and logged player coordinates with bug reports: the map doubled as a production dashboard. — [gist mirror](https://gist.github.com/idbrii/e39fe96279aa1670319bfa521d907399)
- W3: the generator captures whatever is currently loaded, so a re-capture after level edits is a tool run, but the docs say substantial external adjustment of raw images follows. — [CDPR REDkit wiki (search extract)](https://cdprojektred.atlassian.net/wiki/spaces/W3REDkit/pages/51904544/HOW-TO+Generate+minimaps)
- REDkit's community changelog records a fix for minimap generation stretching output images and notes an editor map file (editormap.json) emitted by the tool. — [REDkit community wiki changelog](https://wiki.redmodding.org/redkit-community-wiki/redkit-changelog)

### Inferences
- The sync cost is set by the production method: a fully generated map (BotW) syncs for free on a nightly job; a captured-then-adjusted map (W3) needs a re-capture plus repeated manual work unless the adjustments are scripted; an illustrated map (Elden Ring, Mirage) needs an artist to redraw the changed area. Only the first two are documented; the third is inference.
- Illustrated maps are probably painted late, once layout is locked, and over some captured or height-derived reference. This is common-sense inference and I found no source confirming it for any game here.
- A map whose discovery variants multiply the image count (BotW's 2,344 screens) makes hand maintenance infeasible independent of art style; computing the mask at runtime instead of baking variants would avoid that multiplication. Inference.

### Gaps
- No source describes how Elden Ring, Mirage, Ghost of Tsushima or Horizon kept the map current during production.
- For W3 I could not determine whether the post-capture adjustments were automated or redone by hand at each re-capture, nor how often re-captures happened.
- No source gives the BotW nightly job's runtime, failure handling, or whether artists could override generated output for specific tiles.

## 4. What published talks, postmortems, blogs or docs describe these pipelines?

### Takeaway
Primary documentation of map-image pipelines is scarce. The two useful primary sources are Nintendo's CEDEC 2017 UI session and CD Projekt RED's REDkit documentation; the best secondary sources on what shipped maps show are Stamen's "Cartographers play video games" reviews. No GDC talk dedicated to a world-map or minimap pipeline was found.

### Cited Findings
Directly about map pipelines
- CEDEC 2017, Nintendo UI session on BotW (Kitayama, Hase): map sectioning, zoom levels, procedural generation, nightly rebuild. Japanese press report: [GAME Watch](https://game.watch.impress.co.jp/docs/news/1078/846/amp.index.html). English summary: [gist mirror of Matt Walker's translation](https://gist.github.com/idbrii/e39fe96279aa1670319bfa521d907399).
- CD Projekt RED, The Witcher 3 REDkit documentation, "HOW-TO: Generate minimaps": [page](https://cdprojektred.atlassian.net/wiki/spaces/W3REDkit/pages/51904544/HOW-TO+Generate+minimaps); space overview: [REDkit wiki](https://cdprojektred.atlassian.net/wiki/spaces/W3REDkit/overview).
- ArtStation, AC Mirage world map by the map artist (fogged/unfogged views, POI drawings): [project](https://therareaquaticpyro.artstation.com/projects/AlD4EN).

About what the shipped maps show (cartographic critique, not production)
- Stamen on BotW: [review](https://stamen.com/cartographers-play-video-games-a-review-of-the-map-in-the-legend-of-zelda-breath-of-the-wild/). Stamen on Elden Ring: [review](https://stamen.com/cartographers-play-video-games-a-review-of-the-map-in-elden-ring/).

Adjacent (world building and navigation design, not map imagery)
- CEDEC 2017 BotW field level design session (Fujibayashi, Yonezu), with player-path heat maps over the field: [Famitsu](https://www.famitsu.com/news/201708/31140870.html); summary in the [gist mirror](https://gist.github.com/idbrii/e39fe96279aa1670319bfa521d907399).
- BotW prototype used a 2D Twilight Princess map with Kyoto terrain pasted around it for scale: [Nintendo Everything](https://nintendoeverything.com/nintendo-on-the-behind-the-scenes-creation-of-zelda-breath-of-the-wild).
- Ghost of Tsushima landmark ("weenie") design talk, including the no-minimap decision: [Game Developer](https://gamedeveloper.com/design/a-taxonomy-of-weenies-the-landmarks-that-define-i-ghost-of-tsushima-i-).
- W3 world building, POI-driven layout: [80.lv](https://80.lv/articles/world-building-of-witcher-3/).
- Guerrilla GDC talks cover procedural placement and tools pipeline but not the player map: [80.lv on procedural placement](https://80.lv/articles/real-time-procedural-placement-in-horizon-zero-dawn/?amp=1), [Game Developer on the tools pipeline talk](https://gamedeveloper.com/production/video-creating-a-tools-pipeline-for-i-horizon-zero-dawn-i-).
- AC Odyssey team made deliberate cartographic generalisations of Greece for playability: [Ubisoft News](https://news.ubisoft.com/en-us/article/348038/Assassins-Creed-Odyssey-The-Game-That-Greece-Built).

### Inferences
- Map production is treated as a UI/art sub-task and rarely gets its own talk; where it is documented it appears as one slide in a UI talk (Nintendo) or as modding documentation (CDPR). Modding and datamining documentation is therefore a more productive vein than conference talks.

### Gaps
- No GDC Vault talk specifically on world-map or minimap production was found for any of these games. I did not search GDC Vault directly, only through general web search.
- The zeldamods wiki page for BotW's map textures (Content/UI/MapTex) is a stub listing only MainFieldArea.byaml and a MainField directory, with no format detail: [zeldamods](https://zeldamods.org/wiki/Content/UI/MapTex).
- Not searched: Genshin Impact, Red Dead Redemption 2, Horizon Forbidden West, Tears of the Kingdom, artbooks ("Creating a Champion"), game credits for map-artist names.

## 5. What data and metadata does such a map system need?

### Takeaway
The documented systems imply a small, stable schema: a tiled multi-zoom raster pyramid in world-aligned sections, a region/area definition table driving discovery state, per-zoom label rules, a POI/icon list with per-icon state, and player annotation storage. Most of the specifics below are inferred from behaviour; only the tiling and zoom facts are stated by developers.

### Cited Findings
- Tiles and zoom pyramid: BotW uses 120 dynamically loaded sections at 4 zoom levels. — [gist mirror](https://gist.github.com/idbrii/e39fe96279aa1670319bfa521d907399)
- Per-area discovery variants: BotW bakes separate colourings for unopened areas, which is what drives the count to 2,344 screens. — [gist mirror](https://gist.github.com/idbrii/e39fe96279aa1670319bfa521d907399)
- Area definition data ships alongside the map textures in BotW (UI/MapTex/MainFieldArea.byaml next to UI/MapTex/MainField). — [zeldamods MapTex](https://zeldamods.org/wiki/Content/UI/MapTex)
- World-aligned tile grid: W3 minimap tiles each represent "a tile of the world" seen from above. — [CDPR REDkit wiki (search extract)](https://cdprojektred.atlassian.net/wiki/spaces/W3REDkit/pages/51904544/HOW-TO+Generate+minimaps)
- Layer classes and masks: W3 separates exterior layers (water, roads, terrain, forest) from interior ones (buildings, caves) with per-mask folders. — [CDPR REDkit wiki (search extract)](https://cdprojektred.atlassian.net/wiki/spaces/W3REDkit/pages/51904544/HOW-TO+Generate+minimaps)
- Label rules by zoom: region labels at far zoom, finer names nearer; some names gated on visiting. — [Stamen BotW](https://stamen.com/cartographers-play-video-games-a-review-of-the-map-in-the-legend-of-zelda-breath-of-the-wild/)
- Icon state machine: three states per shrine/tower icon; grace icons carry an orientation. — [Stamen BotW](https://stamen.com/cartographers-play-video-games-a-review-of-the-map-in-the-legend-of-zelda-breath-of-the-wild/); [Stamen Elden Ring](https://stamen.com/cartographers-play-video-games-a-review-of-the-map-in-elden-ring/)
- Player annotations with caps: 100 stamps (BotW); 100 markers in 10 symbols (Elden Ring expansion); a recorded path history (BotW Hero's Path). — [Stamen BotW](https://stamen.com/cartographers-play-video-games-a-review-of-the-map-in-the-legend-of-zelda-breath-of-the-wild/); [Stamen Elden Ring](https://stamen.com/cartographers-play-video-games-a-review-of-the-map-in-elden-ring/)
- Multiple map layers for stacked spaces: Elden Ring toggles overworld and underground maps; beacons do not carry across layers. — [Stamen Elden Ring](https://stamen.com/cartographers-play-video-games-a-review-of-the-map-in-elden-ring/)
- Two-tier discovery data: a low-detail always-available base plus fragment-unlocked detail and a separate fog overlay. — [Stamen Elden Ring](https://stamen.com/cartographers-play-video-games-a-review-of-the-map-in-elden-ring/)
- Minimap context switching: BotW's minimap changes zoom on entering a village and shows shops and inns, implying settlement trigger volumes and a finer-scale POI set. — [Stamen BotW](https://stamen.com/cartographers-play-video-games-a-review-of-the-map-in-the-legend-of-zelda-breath-of-the-wild/)
- Production-side reuse: the same world-to-map transform lets bug reports and task data be plotted on the map. — [gist mirror](https://gist.github.com/idbrii/e39fe96279aa1670319bfa521d907399)

### Inferences
Synthesised requirements list (inferred from the cited behaviour; not stated as a schema by any developer):
- World-to-map transform: an affine mapping from world XZ to map pixels per map layer, plus the tile grid origin and tile world size. Required by every feature above (player arrow, pins, POIs, bug plotting) but never spelled out in the sources.
- Raster pyramid: tiles keyed by (layer, zoom, x, y); streaming by section.
- Region/area polygons or a tile-to-area index: drives discovery reveal, region labels, and (in BotW) baked unopened variants.
- Discovery state in the save file: a per-region bitset (BotW, Elden Ring) or a low-resolution visited mask (Ghost of Tsushima style path reveal).
- Label table: text, anchor position, feature class, min/max zoom, optional "visited" gate.
- POI table: id, type/icon, world position, optional orientation, state enum, visibility conditions, which map scale/layer it appears on.
- Player annotation store: capped lists of markers; optional path polyline history.
- Layer set: surface/underground or interior/exterior, each with its own imagery and possibly its own transform.
- Minimap parameters: zoom per context (field versus settlement), icon subset, edge-docking rule for off-screen targets.
- If discovery variants are baked into tiles (BotW), image count multiplies by area state; computing the mask at runtime over one base pyramid avoids that.

### Gaps
- No source gives an actual file format or schema for any of these (tile resolution, coordinate constants, area file contents). The zeldamods page that would hold BotW's is a stub.
- No source states how road networks are represented for the map (baked into the raster versus drawn from spline data at runtime) in any of the five games.
- Minimap rotation mode and how minimap imagery relates to world-map tiles (same pyramid or separate asset) is undocumented in what I read. In W3 the documented capture tool is for the minimap; the relationship to the world map is unconfirmed.
