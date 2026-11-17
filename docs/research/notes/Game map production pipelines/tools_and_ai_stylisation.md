# Tools for minimap / world-map generation, and AI stylisation of data-generated maps

Research date: 2026-11-11. Single-pass, bounded study. Evidence grades used below:
- **[fetched]** = I read the page itself.
- **[snippet]** = the claim comes from a search-result summary of the page; I did not read the page body. Treat as lower confidence.
- Anything under "Inferences" is my reasoning, not documented.

---

# PART 1: TOOLS

## 1. Dungeon Architect (Code Respawn): what does the minimap feature actually do?

### Takeaway
In the current Unreal version (docs say "Applies to Dungeon Architect 3.7") the minimap is the **Dungeon Canvas** framework: a material-driven 2D map drawn from the dungeon's **layout** (a "Layout Texture" plus SDF-driven material layers for outline, border, fill and fog), with fog of war, tracked-actor icons, a pan/zoom UI widget and multiplayer sharing modes. It is not a top-down scene capture. The Unity version has an older, simpler equivalent (`GridFlowMinimap`) that renders the Grid Flow tile layout into a Render Texture. A separate Unreal "3D minimap" for Snap dungeons re-builds the dungeon layout a second time with simplified modules.

### Cited Findings

**Unreal: Dungeon Canvas (current system)**
- Grid Flow dungeons draw their minimap through the Dungeon Canvas framework "like every other builder". The older framework (a Grid Flow Mini Map actor plus a `DungeonMiniMapTrackedObject` component) was removed in version 2.32.0 and replaced by Dungeon Canvas; old assets are redirected automatically to the `Dungeon Canvas Item` component. Page header: applies to Dungeon Architect 3.7, Unreal Engine (no engine version named). [fetched] — [DA Unreal: Grid Flow Mini Map](https://docs.dungeonarchitect.dev/unreal/grid-flow/gridflow-mini-map/)
- Setup: add a `Dungeon Canvas` component to the Dungeon actor. Stated purpose: "Design beautiful minimaps for your levels using a Dungeon Canvas." Each dungeon actor's canvas component holds a default canvas theme reference used by the UI widgets; the theme can be overridden per widget to show several minimaps at once. Samples ship under `Dungeon Architect Content > Showcase > Samples > Features > Canvas > CanvasExamples`. [fetched] — [DA Unreal: Canvas Overview](https://docs.dungeonarchitect.dev/unreal/canvas/canvas-overview/)
- Appearance is authored in a **Canvas Theme Editor** as a stack of material layers. The documented example stack is: Layout Texture, Shadow, Outline, Border Decoration, Background Texture. The Layout Texture layer "will draw the layout on to the material". An image caption on the page describes "Stacking SDF driven layers" for "outlines, borders, fills and fog". Themes are assets created via `Dungeon Architect > Dungeon Canvas > Dungeon Canvas Material Theme`. [fetched] — [DA Unreal: Canvas Theme](https://docs.dungeonarchitect.dev/unreal/canvas/canvas-theme/)
- Layers are standard Unreal Material Layer assets; the final material is built by stacking them; users can add a `Custom Material Layer` and expose their own parameters. Reference example `ML_IconOverlay` lives at `DungeonArchitect Content > Core/Features/DungeonCanvas/Materials/Layers/Functions/`. [fetched] — [DA Unreal: Canvas Material Layers](https://docs.dungeonarchitect.dev/unreal/canvas/canvas-material-layers/)
- Theme settings include: a preview dungeon with switchable **builder type** ("to visualize how your theme looks with different dungeon types") and a Randomize Dungeon button; `Layout Draw Margin Percent` (how much of the background the dungeon fills; example value 50); `Effects` (custom advanced effects with user-managed textures); `Material Template Canvas` (master canvas material); `Material Template Fog of War` ("Provide your own custom fog of war material"). Defaults are not stated on the page. [fetched] — [DA Unreal: Canvas Settings](https://docs.dungeonarchitect.dev/unreal/canvas/canvas-settings/)
- Icons: the theme has an `Overlay Icons` list (name, image, tint). A `Dungeon Canvas Item` component on an actor selects an entry by `Icon Name`; `Override Icon Tint` recolours per actor; a `Fallback Icon` is drawn if the name is not in the theme. [fetched] — [DA Unreal: Canvas Theme](https://docs.dungeonarchitect.dev/unreal/canvas/canvas-theme/)
- Fog of war / discovery: any actor with a `Dungeon Canvas Item` can show an icon and reveal fog around itself (players, bots, or world objects such as shrines or watch towers). Settings: `Item Type` (e.g. `World Object`), `Explore Fog Of War`, `Light Radius` (default 2000 world units), `Num Shadow Samples` (default 1; jittered samples soften shadow edges), `Shadow Jitter Distance` (default 30). `Fog Of War Explorer Enabled` toggles at runtime; `Add Fog Of War Explorer` on the canvas component registers an actor with no item component. [fetched] — [DA Unreal: Fog of War & Multiplayer](https://docs.dungeonarchitect.dev/unreal/canvas/canvas-fog-of-war/)
- Multiplayer: `Fog Of War Share Mode` on the `Dungeon Canvas` component has three modes: Free For All (own exploration only), Team (shared by `Team Id`), All Players. World-object items ignore the share mode. `Get Local Player Team Id` can be overridden for custom team systems. [fetched] — [DA Unreal: Fog of War & Multiplayer](https://docs.dungeonarchitect.dev/unreal/canvas/canvas-fog-of-war/)
- The doc set also has pages for the interactive pan/zoom UI widget and player setup (I did not fetch these): [Canvas UI Widget](https://docs.dungeonarchitect.dev/unreal/canvas/canvas-ui-widget/), [Canvas Player Setup](https://docs.dungeonarchitect.dev/unreal/canvas/canvas-player-setup/). Their existence and scope are stated in the link list on [Grid Flow Mini Map](https://docs.dungeonarchitect.dev/unreal/grid-flow/gridflow-mini-map/) [fetched].

**Unreal: 3D minimap for Snap dungeons**
- A separate technique builds a 3D minimap by building the snap dungeon again from the existing layout using a different module theme (e.g. a theme named `minimap` in the module database with a simplified module per entry). It can sit in the same world as a hologram-style preview, or in a separate world rendered to a render texture for the HUD. Minimap modules have no connections defined because the minimap is built from the existing dungeon layout. A `Connection Marker Prefix` on the `Build Snap Themed Dungeon` node (e.g. `Minimap`) makes the builder look up `MinimapDoor` / `MinimapWall` instead of `Door` / `Wall`. The page does not mention fog of war or tracked actors. [fetched] — [DA Unreal: Snap Flow 3D Minimap](https://docs.dungeonarchitect.dev/unreal/snap-flow/snapflow-minimap3d/)

**Unity: GridFlowMinimap**
- The Unity minimap is a 2D map with fog of war for the Grid Flow builder; the `DungeonGridFlow` prefab ships pre-configured with a `GridFlowMinimap` component. [fetched] — [DA Unity: Grid Flow Mini Map](https://docs.dungeonarchitect.dev/unity/grid-flow/gridflow-mini-map/)
- It renders into a user-assigned Render Texture (`Minimap Texture`), recommended 512x512, no depth buffer, Point filter "for sharp tile edges". The texture can be shown on a UI `RawImage` or on a mesh (the sample uses an in-world tablet with shader `UI/Unlit/Transparent`). [fetched] — [DA Unity: Grid Flow Mini Map](https://docs.dungeonarchitect.dev/unity/grid-flow/gridflow-mini-map/)
- Settings: `Update Frequency` (can run below frame rate), `Enable Fog of War`, `See Through Walls` (if off, unexplored areas behind a wall stay hidden; only applies with fog on), `Icons` (overlaid on special tiles), `Init Mode` for the layout texture (On Dungeon Rebuild / On Play / Manual from script). [fetched] — [DA Unity: Grid Flow Mini Map](https://docs.dungeonarchitect.dev/unity/grid-flow/gridflow-mini-map/)
- Tracked objects: add `GridFlowMinimapTrackedObject` to a prefab; options include "Explores Fog of War" (recommended only for the player and other relevant objects), icon, colour, scale, and icon rotation following the object's Y rotation; greyscale icons can be tinted (e.g. red and blue keys). [fetched] — [DA Unity: Grid Flow Mini Map](https://docs.dungeonarchitect.dev/unity/grid-flow/gridflow-mini-map/)
- An older indexed copy of this page is titled "Dungeon Architect for Unity 1.20 Help". [snippet] — [raw GitHub copy (now 404)](https://raw.githubusercontent.com/coderespawn/docs.dungeonarchitect.dev/main/unity/gridflow-mini-map.html)

**History**
- In January 2019 the developer described a procedural minimap for Unreal in which "a lower resolution texture is used for fog of war and passed to the material", with static and dynamic markers. [snippet; forum/Reddit post, not primary docs] — [r/unrealengine mirror](https://redlib.hbubli.cc/r/unrealengine/comments/al1996/procedural_minimaps_dungeon_architect)

### Inferences
- The map is generated from the **dungeon model/layout data**, not from a top-down camera capture: the docs speak only of a "Layout Texture", SDF-driven layers and material templates, and the Unity version asks for a depth-less, point-filtered render texture "for sharp tile edges". No page mentions a scene capture. The exact pipeline (how the layout texture and SDF are rasterised, at what resolution) is **not documented** on the pages I read.
- Because the look is a material stack over layout/SDF textures, Dungeon Canvas is effectively a **procedural shader-styled schematic map**: style is authored once as a theme and applies to any generated layout with exact registration to the data. This is the closest out-of-the-box precedent found for "stylised map generated from world data without an artist painting each one".
- The Unity feature is tied to the Grid Flow builder (tile grid). Whether Unity has any Canvas-equivalent for other builders is not shown in the sitemap (only one Unity minimap page exists).

### Gaps
- Could not confirm which Unreal Engine versions DA 3.7 supports, or the exact list of builders supported by Dungeon Canvas (the theme editor's "builder type" switch implies several).
- Did not read the Canvas UI Widget and Player Setup pages, so pan/zoom behaviour, full-screen world-map mode and widget settings are unconfirmed.
- No documentation found on export of the canvas as a static image file.

---

## 2. Unreal Engine: World Partition minimap builder and in-game minimap approaches

### Takeaway
The World Partition MiniMap Builder is an **editor-facing** tool: it bakes a top-down image of the world for display in the World Partition editor window, and is not a player-facing minimap system. I found no built-in player-facing minimap in Unreal Engine; common practice is a `SceneCaptureComponent2D` in orthographic mode writing to a render target shown in UMG, or a pre-baked top-down texture, or a hybrid.

### Cited Findings
- Official docs (page titled for Unreal Engine 5.9): the World Partition MiniMap Builder "generates or updates the minimap that displays in the WorldPartition Editor window." It runs as `WorldPartitionBuilderCommandlet` with `-Builder=WorldPartitionMiniMapBuilder` and `-AllowCommandletRendering`. The page lists no further arguments and does not say where the output is stored. [fetched] — [World Partition Builder Commandlet Reference](https://dev.epicgames.com/documentation/en-us/unreal-engine/world-partition-builder-commandlet-reference)
- It is also available from the editor menu: Build > Build World Partition Editor Minimap. [snippet] — [World Partition Builder Commandlet Reference](https://dev.epicgames.com/documentation/en-us/unreal-engine/world-partition-builder-commandlet-reference)
- The result lives in an `AWorldPartitionMiniMap` actor, described in the API as a mini map to preview the world in the World Partition window, with its texture and world bounds exposed as read-only properties. [snippet] — [AWorldPartitionMiniMap API](https://dev.epicgames.com/documentation/unreal-engine/API/Runtime/Engine/AWorldPartitionMiniMap?lang=en-US); [Python API: WorldPartitionMiniMap](https://dev.epicgames.com/documentation/en-us/unreal-engine/python-api/class/WorldPartitionMiniMap)
- Reliability: forum users report the build crashing (UE 5.3.2 Linux) or doing nothing (UE 5.4.4 new project). Individual reports, no official confirmation. [snippet; forum hearsay] — [Has 'Building the world partition minimap' ever worked?](https://forums.unrealengine.com/t/has-building-the-world-partition-minimap-ever-worked/2726974); [Building World Partition Editor Mini Map](https://forums.unrealengine.com/t/building-world-partition-editor-mini-map/2845063)
- There is a third-party tutorial on reusing the World Partition editor minimap as a game minimap. [snippet; I did not read it] — [80.lv: Create Minimap from UE5's World Partition](https://80.lv/articles/tutorial-create-minimap-from-ue5-s-world-partition)
- `USceneCaptureComponent2D` is documented as capturing a "snapshot" of the scene from a single plane and feeding it to a render target. [snippet] — [USceneCaptureComponent2D API](https://dev.epicgames.com/documentation/unreal-engine/API/Runtime/Engine/USceneCaptureComponent2D)
- Typical live-capture recipe (UE5 practitioner guide): 512 render target, Scene Capture Component 2D above the player pointing down, Projection Type = Orthographic, Ortho Width = 3000, disable Capture Every Frame / Capture on Movement where possible, lower resolution and turn off unneeded show flags (fog, particles); the render target is assigned directly to a UMG Image brush. [snippet] — [uhiyama-lab: UE5 minimaps with render targets](https://uhiyama-lab.com/en/notes/ue/render-target-minimap-camera/)
- An Epic Developer Community tutorial covers the same pattern (Render Target 2D for UI, procedural circular mask material, orthographic scene capture). [snippet] — [Community Tutorial: How to Create an Optimized Minimap in Unreal Engine](https://forums.unrealengine.com/t/community-tutorial-how-to-create-an-optimized-minimap-in-unreal-engine/2719592)
- A 2026 vendor blog (StraySpark, selling a UE 5.8 minimap plugin, so not neutral) compares four approaches: live scene capture, baked texture (offline top-down render plus the stored world rectangle it covers), canvas-only vector drawing, and hybrid (baked base + live overlay). Claims: "A single texture caps at 16,384 px a side" (about 8.2 km at 50 world units per pixel), so larger worlds need a tile pyramid; scene capture is "frequently the single largest UI cost in a project" on console and mobile (no timings given); live scene capture cannot show unloaded World Partition cells because "Unloaded World Partition cells contain no actors, so they render as nothing"; default sky spheres inflate automatic bounds; bakes should be automated (e.g. a commandlet in CI). [fetched] — [StraySpark: UE5 minimap, scene capture vs baked map](https://www.strayspark.studio/blog/ue5-minimap-scene-capture-vs-baked-map-2026)
- Older forum threads state scene capture was perspective-only; newer guides show the orthographic option, so the older posts are out of date. [snippet] — [How do you make a scene capture 2d orthographic](https://forums.unrealengine.com/t/281242)

### Inferences
- "No built-in player-facing minimap in UE" is an inference from absence: the official docs describe the WP minimap only as an editor preview, and every tutorial found builds the minimap from components. I did not find an Epic statement saying so explicitly.
- For a streamed open world, a live capture is structurally unable to show the whole world, which pushes large-world games toward a baked or data-drawn map. This matches the vendor blog's argument but is the vendor's framing.

### Gaps
- Could not confirm from primary docs the resolution/format of the WP minimap texture, whether it is virtual-textured, or whether it is cooked into shipping builds.
- No primary source checked for sample projects (e.g. Lyra, City Sample) and what they use for minimaps.
- The 16,384 px limit is a vendor-blog claim, not checked against Epic docs.

---

## 3. Unity and Godot: common practice and widely used packages

### Takeaway
Neither engine ships a minimap system; both document the building block (render a second camera to a texture) and leave the minimap to the developer or to store packages. Unity practice is a second orthographic top-down camera rendering to a Render Texture shown in a UI RawImage; Godot practice is a camera inside a `SubViewport` whose `ViewportTexture` is drawn in the UI.

### Cited Findings
- Unity manual (Unity 6.3 LTS): the documented workflow is to create a Render Texture, add a second camera and assign the texture as its target; the result updates in real time. [snippet] — [Unity Manual: Output to a Render Texture](https://docs.unity3d.com/Manual/output-to-render-texture.html)
- Practitioner tutorials consistently use an orthographic second camera (no perspective distortion), displayed on a UI RawImage and masked to a circle. [snippet] — [Game Developer: Implementing a minimap in Unity](https://www.gamedeveloper.com/programming/implementing-a-minimap-in-unity-tutorial-); [uhiyama-lab: Unity Render Texture basics](https://uhiyama-lab.com/en/notes/unity/unity-rendertexture-minimap/); [Medium: Render Textures part 2, in-game 2D mini-map](https://medium.com/geekculture/unity-features-101-render-textures-part-2-in-game-2d-mini-map-e5be2cb25ce2)
- Unity Asset Store packages that surfaced: **UGUI MiniMap** (described as a complete MiniMap/WorldMap solution on uGUI; listing showed v2.8.1, 19 March 2026, $7.50 on sale from $15), plus MiniMap Optimized, Map and Minimap, Fast RTS Minimap, Minimap and Generator. Only names, features and prices were visible; no usage statistics. [snippet] — [UGUI MiniMap](https://assetstore.unity.com/packages/tools/gui/ugui-minimap-32874); [Fast RTS Minimap](https://assetstore.unity.com/packages/tools/gui/fast-rts-minimap-237367)
- Open-source Unity options that surfaced: DMMap ("Procedural, Multilayer & Vector-like maps", now free and open source) and MinimapTools. [snippet] — [DMMap thread](https://discussions.unity.com/t/dmmap-minimap-system-2-0-procedural-multilayer-vector-like-maps-ugui-now-free-and-open-source/546286?page=6); [MinimapTools on GitHub](https://github.com/nicholas-maltbie/MinimapTools)
- Godot official docs (Viewports tutorial): content rendered to a SubViewport is not visible in the scene editor; "To display the contents, you have to draw the SubViewport's ViewportTexture somewhere." Only one camera can be active per Viewport. A SubViewport fills its parent SubViewportContainer when Stretch is enabled. [snippet] — [Godot docs 4.1: Using Viewports](https://docs.godotengine.org/en/4.1/tutorials/rendering/viewports.html)
- Godot Viewport class reference: a viewport can optionally have its own 2D or 3D world so it does not share what it draws with other viewports. [snippet] — [Godot docs: Viewport class](https://docs.godotengine.org/en/4.2/classes/class_viewport.html)
- Godot community pattern: an overhead camera inside the SubViewport follows the player each frame; icons for enemies/items are created and updated in the minimap; the minimap camera should render only the layers that belong on the map. [snippet] — [uhiyama-lab: Godot SubViewport techniques](https://uhiyama-lab.com/en/notes/godot/subviewport-techniques/)

### Inferences
- Across UE, Unity and Godot the engine-level primitive is identical (secondary camera to texture). None of the three offers a data-driven, stylised map generator out of the box; that capability appears only in add-ons such as Dungeon Architect's Canvas.
- "Widely used" cannot be established from what I found; UGUI MiniMap is the package that surfaced most prominently, but I have no download or review counts.

### Gaps
- No official Godot minimap tutorial was found; the Godot Asset Library was not searched for minimap add-ons.
- Baked top-down texture practice in Unity/Godot was not separately sourced (it is the same idea as the UE baked approach in section 2).
- The package names "Bitmap2Material" and "MT Minimap" from my search query did not appear; ignore them.

---

## 4. Other procedural / world-building tools: terrain producers vs stylised-map producers

### Takeaway
The tools split cleanly. Terrain tools (World Machine, Gaea, World Creator) output heightmaps, masks/splat maps and a colour/texture map of the terrain surface, i.e. data and a satellite-like image, not a cartographic map. Map tools (Azgaar's Fantasy Map Generator, Wonderdraft, Inkarnate) output a stylised 2D map; of these only Azgaar is a simulation-style generator that also holds map data and can export a heightmap. No tool found does both ends: turn an arbitrary game world's data into an illustrated map.

### Cited Findings

**Terrain producers**
- World Creator's export presets include Height Map, Splat Map, Color Map, Normal Map, Mesh, Roughness, Metalness, Spline, **Topo Map**, Heat Map, Simulation, AO Map and Unity Mask Map. The colour map is the blend of all splat layers into one texture. Heightmaps should be at least 16-bit. The bridge output is a raw heightmap, PNG texture map, TGA splat map and an XML with resolution, size and min/max height. [snippet] — [World Creator docs: Conventional Export](https://docs.world-creator.com/reference/export/conventional-export); [World Creator docs: Bridge Tools](https://docs.world-creator.com/reference/export/bridge-tools)
- World Machine exports only through Output devices in the device network; RAW16 heightfields for Unity; splat maps via a Splat Converter; note the Y-axis flip relative to Unity. [snippet] — [World Machine help: How do I export my terrain and textures](https://help.world-machine.com/topic/how-do-i-export-my-terrain-and-textures/); [World Machine help: Export to Unity](https://help.world-machine.com/topic/export-to-unity/)
- Gaea: any node can be marked for export and is saved on build; colour maps can be saved in RGB, sRGB or scRGB. The **SatMap** node colours terrain from built-in colour ramps derived from satellite data (works like CLUTer with built-in maps). [snippet] — [Gaea docs: SatMaps](https://docs.quadspinner.com/Reference/Color/SatMaps.html); [Gaea docs: Build Manager](https://docs.quadspinner.com/Guide/Build/Manager.html)

**Stylised-map producers**
- Azgaar's Fantasy Map Generator: exports the full map as SVG, or the currently displayed fragment as PNG; heightmap export exists via Tools > Heightmap > Export as image. [snippet; the heightmap menu path is from a third-party guide] — [Azgaar FMG wiki Q&A (mirror)](https://git.korz.dev/backups/Fantasy-Map-Generator/wiki/Q%26A); [Exporting and importing Azgaar maps into UE5](https://anaryl.substack.com/p/exporting-and-importing-azgaar-maps)
- Wonderdraft: desktop tool, one-time purchase (about $30 per reviews); generates landmasses/coastlines or lets the user paint them, with painting, stamping (symbols) and labelling tools and custom assets. [snippet; review sites, not vendor docs] — [PC Gamer on Wonderdraft](https://pcgamer.com/create-the-fantasy-map-of-your-dreams-with-wonderdraft); [Arcane Eye: DnD map makers](https://arcaneeye.com/dm-tools-5e/dnd-map-makers/)
- Inkarnate: browser-based, several map styles and a large asset library, free tier plus Pro subscription. A user feature request asks for heightmap export, indicating it was not offered at the time of that post. [snippet] — [LegendKeeper: map-making software](https://www.legendkeeper.com/map-making-software/); [Inkarnate feedback: Export heightmaps](https://feedback.inkarnate.com/feature-requests/p/export-heightmaps)
- Interop is lossy: a user report describes a Wonderdraft heightmap imported into Azgaar with coastlines thrown off. [snippet; Reddit hearsay] — [r/FantasyMapGenerator thread (mirror)](https://pol1.lr.ggtyler.dev/r/FantasyMapGenerator/comments/1jwmbsk/any_recommended_settings_when_exporting_a)

### Inferences
- Classification (mine, based on the above):
  - *Terrain out, no cartographic map*: World Machine, Gaea, World Creator (and, by reputation, Houdini heightfields, Gaia, MapMagic; unsourced here).
  - *Stylised map out, hand-authored*: Wonderdraft, Inkarnate. The map is the product; there is no world behind it.
  - *Stylised map out, generated from simulated data*: Azgaar FMG. Its styling is rule-based vector cartography (SVG), which is why the map is exactly registered to its own data.
  - *Stylised map from game-level data, in engine*: Dungeon Architect Canvas (section 1), limited to its own dungeon layouts.
- World Creator's "Topo Map" export is the only terrain-tool output found that is cartographic rather than photographic in intent; I did not confirm what it looks like.

### Gaps
- **Houdini terrain tools, Gaia and MapMagic were not sourced at all.** I found no documentation in this pass stating that any of them produce a map image beyond heightfield/mask/texture outputs; do not cite them without a follow-up.
- Wonderdraft and Inkarnate facts come from reviews, not vendor documentation; Wonderdraft's layered export and Inkarnate's current heightmap-export status are unconfirmed.
- Azgaar details come from a wiki mirror and a third-party guide, not the canonical GitHub wiki.

---

# PART 2: AI STYLISATION

## 5. Documented cases of image models stylising a generated/captured top-down map

### Takeaway
There is one strong, directly relevant published case: Affolter et al. (ETH Zurich cartography group, arXiv 2508.18959, 2025) train ControlNets on Stable Diffusion 1.5 to render colour-coded vector data into real cartographic styles, with code, checkpoints and a cartographer user study. Everything else found is adjacent (terrain or isometric generation, 3D-scene cartoon stylisation) or a practitioner demo. I found **no documented shipped game** that uses a diffusion model to produce its player-facing map from game data.

### Cited Findings
- "Generative AI in Map-Making: A Technical Exploration and Its Implications for Cartographers", Claudio Affolter, Sidi Wu, Yizi Chen, Lorenz Hurni; arXiv 2508.18959, submitted 26 Aug 2025, revised 14 Nov 2025. Abstract position: image diffusion models struggle with accurate maps because of limited control over spatial composition and semantic layout; the authors integrate vector data to guide generation in styles given by text prompts, wrap it in a web application, and run a user study with professional cartographers on fidelity, usability and implications. [fetched] — [arXiv:2508.18959](https://arxiv.org/abs/2508.18959)
- Implementation: Stable Diffusion 1.5 + ControlNet; training triples of (target raster map tile, source vector image, prompt); tiles 512x512; each vector class is rendered in a unique RGB colour (Building, Railway, Lake, Forest, etc.). Four checkpoints released: Swisstopo, Old National, Siegfried, and a Combined model for all three styles used in the web app. [fetched] — [GitHub: claudaff/generative-ai-mapmaking](https://github.com/claudaff/generative-ai-mapmaking); [Hugging Face: claudaff/Cartographic-ControlNet](https://huggingface.co/claudaff/Cartographic-ControlNet/blob/main/README.md)
- A third-party summary says cartographers rated outputs close to reference Swisstopo tiles and that stitching and topology remain open issues. [snippet; secondary source, unverified against the paper] — [Moonlight review](https://www.themoonlight.io/review/generative-ai-in-map-making-a-technical-exploration-and-its-implications-for-cartographers)
- Practitioner demo (Feb 2023): Reddit user "stassius" built rough colour-coded layouts in Houdini, generated images with Stable Diffusion + Multi-ControlNet, and projected them back onto the Houdini models to get isometric in-game maps. The 80.lv write-up does not name the ControlNet types and reports no limitations. [fetched] — [80.lv: Houdini, Stable Diffusion & Multi-ControlNet isometric maps](https://80.lv/articles/using-houdini-stable-diffusion-multi-controlnet-to-create-isometric-maps)
- A game-specific ControlNet exists on Hugging Face: an SD 1.5 model that turns annotated terrain renders from the game Under Fire into photorealistic aerial terrain (the reverse direction of stylisation: data render to realistic image). [snippet] — [tstruk/under-fire-terrain-controlnet](https://huggingface.co/tstruk/under-fire-terrain-controlnet)
- An ISPRS IJGI (MDPI, 2025) paper combines DDIM inversion, ControlNet and MultiDiffusion for cartoon style transfer of real-world 3D scenes, and notes that LDM-based style transfer often relies on complex prompts or per-input tuning. [snippet; page returned 403, title and details unverified] — [MDPI IJGI 14(8) 303](https://www.mdpi.com/2220-9964/14/8/303)
- Adjacent generation work (not stylisation of an existing map): Earthbender (MIG 2025) extends Stable Diffusion with a ControlNet taking Canny edges plus colour channels (red mountains, green lakes, blue for carving roads/rivers) to generate heightmaps from sketches; TerraFusion (arXiv 2505.04050) argues two-stage heightmap-then-texture pipelines accumulate error and generates both jointly. [snippet] — [Earthbender](https://danial-barazandeh.github.io/Earthbender); [TerraFusion](https://arxiv.org/html/2505.04050v1)
- ControlMap (arXiv) uses latent diffusion + ControlNet to generate HD driving maps with city-level style conditioning. [snippet; relevance marginal, ID not verified] — [arXiv:2606.15930](https://arxiv.org/abs/2606.15930)

### Inferences
- The Affolter et al. setup is structurally the same problem as "stylise a game-data map": a colour-coded class raster as the control image, style in the prompt/checkpoint. The key difference is that they had thousands of perfectly aligned (vector, hand-made map) pairs from a national mapping agency for supervised ControlNet training; a game project has no such paired corpus for its target art style.
- The absence of documented production use in games is an absence-of-evidence finding from a short search, not proof that no studio does it.

### Gaps
- Did not read the full paper PDF, so the user-study numbers, the stated "further technical improvements" and any explicit failure analysis are not captured.
- The Reddit thread behind the 80.lv demo (which holds the actual prompts and ControlNet types) was not read.
- No dev blog or GDC talk on AI-stylised game maps was found in this pass.

---

## 6. Conditioning methods used to keep geometry in place

### Takeaway
The documented methods are: (a) a ControlNet trained on a colour-coded semantic/vector raster (strongest evidence, Affolter et al.); (b) stacked generic ControlNets (edge, depth, segmentation) with per-control weights; (c) inversion-based image-to-image (DDIM inversion) so the output starts from the source image's latent; (d) tile-based generation (MultiDiffusion, Tiled Diffusion, ControlNet Tile) for outputs larger than the model's native resolution.

### Cited Findings
- Semantic colour-map ControlNet: vector layers rasterised with one RGB colour per class, same extent as the 512x512 target tile, as the ControlNet condition on SD 1.5; style selected by prompt. The authors note "The better the alignment between target and source, the better the resulting model." [fetched] — [GitHub: claudaff/generative-ai-mapmaking](https://github.com/claudaff/generative-ai-mapmaking)
- ControlNet mechanism: clones the U-Net encoder blocks of Stable Diffusion into a trainable branch that processes the conditioning input while the original blocks stay frozen. [snippet; secondary summary] — [Moonlight review](https://www.themoonlight.io/review/generative-ai-in-map-making-a-technical-exploration-and-its-implications-for-cartographers)
- Multi-ControlNet: the diffusers library supports several conditionings at once with per-control weights, e.g. `controlnet_conditioning_scale=[1.0, 0.8]`. [snippet] — [Diffusers docs v0.16: ControlNet](https://huggingface.co/docs/diffusers/v0.16.0/api/pipelines/stable_diffusion/controlnet)
- Multi-channel control: Earthbender feeds Canny edges for overall structure plus colour channels for semantic classes into one ControlNet branch. [snippet] — [Earthbender](https://danial-barazandeh.github.io/Earthbender)
- Inversion + ControlNet + MultiDiffusion combined for stylising large geospatial scenes. [snippet; unverified, page 403] — [MDPI IJGI 14(8) 303](https://www.mdpi.com/2220-9964/14/8/303)
- Tiled generation: Tiled Diffusion splits denoising into overlapping tiles (overlap and tile size trade off against speed and VRAM); the official MultiDiffusion code uses large overlaps so each edge is covered by several tiles; a ControlNet Tile step is used so neighbouring tiles do not disagree on fine detail. [snippet] — [ComfyUI-TiledDiffusion](https://github.com/shiimizu/ComfyUI-TiledDiffusion); [InvokeAI PR 6635](https://github.com/invoke-ai/InvokeAI/pull/6635)
- General caution from style-transfer literature: an edge map "tends to strongly preserve the structure of content images so that the structural styles are not effectively conveyed", i.e. strong edge conditioning fights stylisation. [snippet; exact paper not pinned down among the results] — [DreamStyler, arXiv:2309.06933](https://arxiv.org/pdf/2309.06933)

### Inferences
- There is a direct trade-off: the tighter the structural conditioning (edges, high control weight, low denoise), the less the model can restyle shapes (e.g. hand-drawn wobble, exaggerated mountains); the looser it is, the more geometry drifts. Nothing found resolves this; the cartographic paper sidesteps it by training on real paired maps so the "style" includes the correct generalisation.
- Plain image-to-image with low denoise strength as a geometry-preserving method is standard practitioner knowledge, but I did not find a map-specific source documenting it in this pass.

### Gaps
- No source found comparing conditioning types (edge vs depth vs segmentation vs line-art) specifically for maps.
- No map-specific evidence found for newer instruction-based image-editing models; not searched.

---

## 7. Known failure modes

### Takeaway
Documented failures are: hallucinated illegible text/labels (well documented, with a mitigation), quality collapse when control data and target style are misaligned, blur at small scales, the need to generate several seeds and filter, and visible seams/patchwork between tiles. Geometry drift of specific features (coastlines, roads) and inconsistent style across tiles are widely assumed but I found only indirect documentation.

### Cited Findings
- Fake text: if labels are not masked in training data, "the generated map tiles will be subject to fake labels" (illegible made-up letters). Mitigation: "It is important to mask all map labels in the raster data AND vector data"; the authors detect labels with keras-ocr for historical maps and use annotation vectors for modern ones; rare classes should also be masked. The implication is that labels are removed from generation and must be added by another process. [fetched] — [GitHub: claudaff/generative-ai-mapmaking](https://github.com/claudaff/generative-ai-mapmaking)
- Misalignment: historical maps without matching vector data can yield "poor quality" tiles; the model card material says historical styles need significant post-processing due to misalignment between modern vector data and old map references. [fetched for the first clause; snippet for the second] — [GitHub: claudaff/generative-ai-mapmaking](https://github.com/claudaff/generative-ai-mapmaking); [Hugging Face model card](https://huggingface.co/claudaff/Cartographic-ControlNet/blob/main/README.md)
- Scale/detail: tiles were at 1:5000; smaller scales such as 1:25000 "will likely be blurry". [fetched] — [GitHub: claudaff/generative-ai-mapmaking](https://github.com/claudaff/generative-ai-mapmaking). Note: a secondary summary says the styles are 1:25,000 maps, which conflicts on its face; possibly map series scale vs training tile scale. Unresolved. — [Moonlight review](https://www.themoonlight.io/review/generative-ai-in-map-making-a-technical-exploration-and-its-implications-for-cartographers)
- Non-determinism: the authors generate multiple seeds per tile and select the best automatically, using a segmentation model and the standard deviation of background pixel values. [fetched] — [GitHub: claudaff/generative-ai-mapmaking](https://github.com/claudaff/generative-ai-mapmaking)
- Stitching and topology are reported as open issues. [snippet; secondary] — [Moonlight review](https://www.themoonlight.io/review/generative-ai-in-map-making-a-technical-exploration-and-its-implications-for-cartographers). The repo itself only says tiles "can, if needed, be stitched together" without a method. [fetched] — [GitHub: claudaff/generative-ai-mapmaking](https://github.com/claudaff/generative-ai-mapmaking)
- Tile seams: without blending, a sharp cut-off caused visible seams "particularly on smooth single-color regions"; fixes were gradient blending between tiles and jittering tile positions per step. [snippet] — [InvokeAI PR 6635](https://github.com/invoke-ai/InvokeAI/pull/6635)
- Pixel-space tile fusion with masks (e.g. Gaussian) can leave visible seams; one project moves tiling into the latent denoising loop instead. Tile processing is also slow because tiles are processed sequentially. [snippet; project README claim] — [MoD ControlNet Tile Upscaler for SDXL](https://elismasilva-mod-control-tile-upscaler-sdxl.hf.space/)
- Diagnosing seams: set `tile_overlap` to 0 and `denoise` to 1 to expose them, then raise overlap; a colour-fix node is suggested for colour mismatch between tiles. [snippet] — [ComfyUI-TiledDiffusion](https://github.com/shiimizu/ComfyUI-TiledDiffusion)
- Two-stage pipelines accumulate error across stages. [snippet] — [TerraFusion, arXiv:2505.04050](https://arxiv.org/html/2505.04050v1)

### Inferences
- Smooth single-colour regions being the worst case for seams is bad news for maps specifically: sea, plains and parchment backgrounds are exactly such regions.
- "Colour fix between tiles" being a standard node implies per-tile colour/style inconsistency is a known practical problem, though I found no measurement of it.
- For a game map, an invented feature is a functional bug, not just an aesthetic one (a painted bridge or path that does not exist misleads the player). None of the sources discusses this consequence; it is my framing.
- The label-masking result suggests the robust design is to keep text, icons and probably roads as a separate data-drawn vector overlay on top of a stylised base, rather than asking the model to render them.

### Gaps
- No source found that directly documents and quantifies coastline or road displacement, or invented features, in diffusion-stylised maps.
- No source found on style consistency across independently generated tiles for maps specifically.

---

## 8. Is there a published way to measure registration between stylised output and source data?

### Takeaway
Not for stylised maps, as far as this pass found. The only map-specific measures are pixel MSE against a ground-truth map tile (which requires a paired reference and is not a registration metric) and an implied segmentation-based check used for seed selection. General style-transfer papers use perceptual content-preservation scores (LPIPS, SSIM, CFSD, DSD), none of which measures positional accuracy of features.

### Cited Findings
- Affolter et al. validation: MSE between ground-truth target tile and generated tile on a 100-tile validation set, with the warning that this "only makes sense when training is done using perfectly corresponding vector data!". Test-set evaluation is qualitative. [fetched] — [GitHub: claudaff/generative-ai-mapmaking](https://github.com/claudaff/generative-ai-mapmaking)
- The same project selects among seeds using a segmentation model plus the standard deviation of background pixel values, i.e. it segments the generated tile and scores it. [fetched] — [GitHub: claudaff/generative-ai-mapmaking](https://github.com/claudaff/generative-ai-mapmaking)
- The paper's headline evaluation is a user study with professional cartographers on fidelity and usability. [fetched] — [arXiv:2508.18959](https://arxiv.org/abs/2508.18959)
- General diffusion style-transfer papers measure content/structure preservation with LPIPS, SSIM and CFSD between stylised and source images; one multi-view paper uses DSD (lower is better structure preservation). [snippet] — [SCAdapter, arXiv:2512.12963](https://arxiv.org/pdf/2512.12963); [Multi-view consistent style transfer, arXiv:2411.10130](https://arxiv.org/pdf/2411.10130)
- One paper (DiffArtist) argues existing metrics are obsolete for structural stylisation and proposes an MLLM-based evaluator. [snippet] — [arXiv:2407.15842](https://arxiv.org/html/2407.15842v4)
- A targeted search for IoU / edge-alignment / geometric-fidelity metrics for stylised maps returned nothing map-specific. [my search result, 2026-11-11]

### Inferences
- A registration measure could be assembled from standard parts, though I found no publication doing so for stylised maps: re-segment the stylised image into classes (water/land/road) and compute per-class IoU against the source raster; or extract coastline/road centrelines from both and report distance statistics (mean / 95th percentile / Hausdorff, in pixels or metres); plus a count of unmatched features to capture inventions and deletions. The Affolter seed-selection step (segment the output, score it) is the nearest published precedent for the first half of this.
- Perceptual metrics such as LPIPS/SSIM would penalise the intended style change and would not notice a coastline shifted by a few pixels, so they are a poor fit for this purpose.

### Gaps
- Did not search the remote-sensing map-translation literature (aerial-to-map GAN work such as pix2pix-style map generation) or the map-generalisation literature, which may contain positional-accuracy metrics that transfer. This is the most promising follow-up for the measurement question.
- Did not read the full Affolter et al. paper for any quantitative fidelity metric beyond what the repo README states.
