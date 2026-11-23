# Names

Sections 20 and 25 of the [ledger](README.md): what each engine calls a thing, and this
project's own words.

## 20. One thing, three names

The same thing is called differently by each engine, and several common words belong to an engine
we are not targeting. First written from memory; checked against the engines' own documentation on
2026-11-17 (M7). **Checked** names the source below that confirms the row, says "corrected" where
the row was changed, or "not checked" where no primary source was read. Engines change: a status
(experimental, production-ready) is as of that date.

| General term here | Unreal | Godot | Note | Checked |
|---|---|---|---|---|
| runtime cell | grid cell of a runtime grid (World Partition) | none built in | Godot has no built-in world streaming | U1 |
| placed level | Level Instance, Packed Level Actor | instanced scene | "sublevel" is level streaming's word (a persistent level with sublevels), still used in a world without World Partition; World Partition removes the need for it | U1, U2; corrected |
| switchable set | Data Layer (a Data Layer Asset per project, a Data Layer Instance per world) | none built in | | U3 |
| world precision | Large World Coordinates | large world coordinates: an engine built with `precision=double` | Godot uses the same name; official Godot builds do not have it, so a custom build is needed. Origin shifting is the older way | G1; corrected |
| terrain | Landscape | none built in; a plugin | | not checked |
| weight map | layer weightmap | a plugin's control map | "splat map" is said beyond Unity; who uses it is not checked | not checked |
| terrain hole | the Landscape Visibility tool, with a Landscape Visibility Mask node and a hole material | plugin-specific | | U4; corrected |
| stamp | Landscape Blueprint Brush (Landmass plugin, on edit layers), Landscape Patch (a texture patch on a patch edit layer) | plugin-specific | | U5, U6; corrected |
| mesh-to-terrain blend | Runtime Virtual Texture | none built in | | not checked |
| terrain texture streaming | Streaming Virtual Texture | none built in | | not checked |
| displacement | Nanite Tessellation (dynamic displacement by a map or a material) | none | that Unreal 5 dropped hardware tessellation is not checked | U7; corrected |
| assembly, saved group | Blueprint, Packed Level Actor | scene | "prefab" is Unity's word | not checked |
| instancing | Instanced Static Mesh, Hierarchical Instanced Static Mesh | MultiMesh | | not checked |
| tile grid of pieces | none direct | GridMap with a MeshLibrary | | not checked |
| merged far proxy | HLOD | visibility ranges (the docs call them HLOD), with artist-made far meshes; no automatic merging | | U1, G2 |
| automatic geometric detail | Nanite | mesh LOD, generated on import of a 3D scene (meshoptimizer) | | G3 |
| cull distance | cull distance | visibility range | | not checked |
| occluder | none authored | OccluderInstance3D | | not checked |
| navigation mesh | NavMesh, navigation invokers | NavigationRegion3D | | not checked |
| navigation link | Nav Link Proxy | NavigationLink3D (marked experimental in the class reference) | "off-mesh link" is Recast's and Unity's word | G4 |
| surface type | Physical Material with a surface type | custom metadata | Godot's PhysicsMaterial has friction, rough, bounce and absorbent only | G5 |
| trigger, blocking, out-of-bounds region | Trigger Volume, Blocking Volume, Kill Z (a World Settings height) and Kill Z Volume | Area3D, a static body | | U8 |
| picture-change region | Post Process Volume | WorldEnvironment; no regions | | not checked |
| sound region | Audio Volume | Area3D with an audio bus | | not checked |
| reflection sample | Reflection Capture | ReflectionProbe | Unity calls it a Reflection Probe too; a Unity Light Probe stores indirect light, not reflections | X1; corrected |
| dynamic bounced light | Lumen | SDFGI, VoxelGI | | not checked |
| placement graph | PCG framework (production-ready from 5.7) | none built in | | U9; corrected |
| crowd and traffic | Mass (MassAI, MassCrowd), ZoneGraph | none built in | the names hold; their status is not confirmed from a primary source (forum posts say the Mass AI plugins are experimental) | U10; names only |
| water body | Water plugin's water bodies (ocean, lake, river, custom, island) | none built in | the Water plugin is listed as experimental | U11, U12; corrected |
| scenario point | Smart Object | none built in | | U13 |
| free labels | Gameplay Tags | groups, metadata | | not checked |
| one file per placed thing | One File Per Actor (editor only; a cooked build embeds actors in their level) | each scene is a text file | | U14 |
| asset rename tracking | redirectors | resource UIDs (`uid://`; scripts and shaders have `.uid` files from 4.4) | | G6 |
| import cache | Derived Data Cache | the `.godot/imported` folder | | not checked |
| cook | cook | export | | not checked |
| editor extension | editor plugin, editor utility | EditorPlugin, GDExtension, `@tool` | | not checked |

Sources (read 2026-11-17, the page itself or a search's summary of it; U10 is a forum thread, secondary):

- U1 World Partition: https://dev.epicgames.com/documentation/en-us/unreal-engine/world-partition-in-unreal-engine
- U2 Level streaming: https://dev.epicgames.com/documentation/unreal-engine/level-streaming-overview-in-unreal-engine
- U3 Data Layers: https://dev.epicgames.com/documentation/unreal-engine/world-partition---data-layers-in-unreal-engine
- U4 Landscape Visibility tool: https://dev.epicgames.com/documentation/en-us/unreal-engine/landscape-visibility-tool-in-unreal-engine
- U5 Landscape Blueprint Brushes: https://dev.epicgames.com/documentation/unreal-engine/landscape-blueprint-brushes-in-unreal-engine
- U6 Landscape Patch: https://dev.epicgames.com/documentation/unreal-engine/landscape-patch-system
- U7 Nanite (its Tessellation section): https://dev.epicgames.com/documentation/en-us/unreal-engine/nanite-virtualized-geometry-in-unreal-engine
- U8 Kill Z Volume: https://dev.epicgames.com/documentation/unreal-engine/API/Runtime/Engine/AKillZVolume
- U9 Unreal Engine 5.7 release: https://www.unrealengine.com/news/unreal-engine-5-7-is-now-available
- U10 Mass, Epic staff on the forum: https://forums.unrealengine.com/t/mass-entity-roadmap-vision-and-more-questions/2527030
- U11 Water plugin index: https://dev.epicgames.com/documentation/unreal-engine/API/PluginIndex/Water
- U12 Water body actors: https://dev.epicgames.com/documentation/unreal-engine/water-body-actors-in-unreal-engine
- U13 Smart Objects: https://dev.epicgames.com/documentation/en-us/unreal-engine/smart-objects-in-unreal-engine---overview
- U14 One File Per Actor: https://dev.epicgames.com/documentation/en-us/unreal-engine/one-file-per-actor-in-unreal-engine
- G1 Large world coordinates: https://docs.godotengine.org/en/stable/tutorials/physics/large_world_coordinates.html
- G2 Visibility ranges (HLOD): https://docs.godotengine.org/en/stable/tutorials/3d/visibility_ranges.html
- G3 Mesh level of detail: https://docs.godotengine.org/en/stable/tutorials/3d/mesh_lod.html
- G4 NavigationLink3D: https://docs.godotengine.org/en/latest/classes/class_navigationlink3d.html
- G5 PhysicsMaterial: https://docs.godotengine.org/en/stable/classes/class_physicsmaterial.html
- G6 UID changes in 4.4: https://godotengine.org/article/uid-changes-coming-to-godot-4-4/
- X1 Unity Light Probes: https://docs.unity.com/en-us/engine/current/manual/lighting-overview/direct-and-indirect-lighting/light-probes/light-probes

## 25. This project's own words

The first cut's own words, in three tables: the nouns the engine knows, the kinds our libraries
ship and the steps those libraries run. Each names the archetypes it is made of
([archetypes.md](archetypes.md)). The mechanisms between the nouns and the kinds are in
[`docs/layers.md`](../layers.md), "The engine's form of each archetype".
`python3 tools/layers_check.py` holds the kinds and the steps equal to `lib/`.

### The eleven nouns

From [`docs/doctrine.md`](../doctrine.md), section 3.

| Noun | What it is | Archetypes |
|---|---|---|
| frame | Where coordinates are measured from | none: the setting of every Field and Shape |
| field | A value at every point | Field |
| feature | A point, a path or a region with an id, a kind and attributes | Shape |
| kind | What a feature does to fields, written as data | Profile, Representation switch, Meeting: its effects, its surfaces and its mounts |
| authority | How firmly a statement holds: derived, shaped, anchored | Principle: who gives way |
| asset | An id a pack binds to a shape and its facts | Asset |
| instance | One asset in one place | Placement |
| rule | Anything that lowers to plain instances or features: set, run, scatter | Recipe, Linear placement, Scatter |
| layer | One document of a world, with an owner | Overlay |
| change | A named difference over a world: a variant, an overlay, a hand edit | Overlay |
| step | The project's own code, run at a declared point | whichever its code realises: Simulation, Grammar, Measure, Delivery |

### The kinds our libraries ship

"Made of" is read from each library's `kinds.toml`: the kind's authority, how many effects it
has and on which fields, its surfaces, its mounts, and whether it is an exit or an output.

| Kind | Library | Archetypes | Made of |
|---|---|---|---|
| `building` | `building` | Grammar | anchored |
| `row` | `building` | Linear placement, Grammar | anchored |
| `court` | `building` | Grammar | anchored |
| `storey` | `building` | Grammar, Representation switch | anchored; surface floor with a cut |
| `footing` | `building` | Grammar, Representation switch | anchored; surface top |
| `platform` | `building` | Grammar, Representation switch | anchored; surface floor |
| `yard` | `building` | Grammar | anchored |
| `room` | `building` | Grammar | anchored |
| `doorway` | `building` | Meeting | 1 effect on role:surface |
| `flight` | `building` | Meeting, Representation switch | surface tread |
| `footpath` | `building` | Meeting | attributes only |
| `roof` | `building` | Grammar, Representation switch | anchored; surface garden |
| `pad` | `land` | Profile | shaped; 2 effects on role:keep_out, role:surface |
| `firm_pad` | `land` | Profile | anchored; 2 effects on role:keep_out, role:surface |
| `approach` | `land` | Profile, Meeting | anchored; 1 effect on role:surface |
| `cliff` | `land` | Profile | shaped; 1 effect on role:surface |
| `sheer_cliff` | `land` | Profile, Representation switch | shaped; 1 effect on role:surface; surface face with a cut |
| `cave` | `land` | Representation switch, Meeting | anchored; 1 effect on role:surface; surface floor with a cut; 4 mounts |
| `poi` | `map` | Markup | output |
| `spawn_zone` | `map` | Markup | output |
| `area` | `map` | Markup | output |
| `bookmark` | `review` | Markup | output |
| `terrain_lake` | `terrain` | Simulation | anchored; 1 effect on role:keep_out |
| `terrain_river` | `terrain` | Simulation | anchored |
| `terrain_hold` | `terrain` | Simulation | anchored |
| `terrain_course` | `terrain` | Network (directed), Profile | 3 effects on role:keep_out, role:surface, tag:water_level |
| `main_street` | `town` | Profile, Network (undirected) | shaped; 2 effects on role:keep_out, role:surface |
| `street` | `town` | Profile, Network (undirected) | shaped; 2 effects on role:keep_out, role:surface |
| `terrace` | `town` | Profile | shaped; 1 effect on role:surface |
| `block` | `town` | Area partition | derived |
| `lot` | `town` | Area partition | derived |
| `ward` | `town` | Area partition | derived |
| `waterfront` | `town` | Meeting, Profile | anchored; 3 effects on role:keep_out, role:surface |
| `enclosure` | `town` | Profile | anchored; 2 effects on role:surface |
| `gate` | `town` | Meeting | attributes only |
| `entry` | `town` | Meeting | exit |
| `wood` | `vegetation` | Area partition | 1 effect on tag:cover |
| `clearing` | `vegetation` | Area partition | 1 effect on role:keep_out |
| `green` | `vegetation` | Area partition | 1 effect on tag:cover |
| `beck` | `water` | Network (directed), Profile | shaped; 3 effects on role:keep_out, role:surface, tag:water_level |
| `lake` | `water` | Derivation (global) | 2 effects on role:keep_out, tag:water_level |
| `tarn` | `water` | Derivation (global) | attributes only |
| `pond` | `water` | Profile | anchored; 3 effects on role:keep_out, role:surface, tag:water_level |
| `weir` | `water` | Meeting, Profile | anchored; 3 effects on role:keep_out, role:surface, tag:water_level |
| `lane` | `ways` | Profile | shaped; 2 effects on role:keep_out, role:surface |
| `ford` | `ways` | Meeting | anchored; 1 effect on role:surface |
| `road` | `ways` | Profile | shaped; 2 effects on role:keep_out, role:surface |
| `trail` | `ways` | Profile | shaped; 2 effects on role:keep_out, role:surface |
| `crossing` | `ways` | Meeting, Representation switch | anchored; 3 effects on role:keep_out, role:surface; surface deck |
| `freeway` | `ways` | Profile | shaped; 2 effects on role:keep_out, role:surface |
| `interchange` | `ways` | Meeting | anchored; 2 effects on role:keep_out, role:surface |
| `viaduct` | `ways` | Representation switch | anchored; 2 effects on role:keep_out, role:surface; surface deck |
| `ramp` | `ways` | Representation switch | anchored; 1 effect on role:surface; surface deck |
| `slipway` | `ways` | Meeting | anchored; 2 effects on role:keep_out, role:surface; exit; output |
| `ferry` | `ways` | Network (undirected) | output |

### The steps our libraries run

A library declares no step; a world restates the step's table (`spec/protocol.md` 1).

| Step | When | Archetypes | What it does |
|---|---|---|---|
| `lib/building/lay.py` | generate | Grammar | One building becomes storeys, their footings, rooms, flights, doorways, footpaths, wall runs, floor tiles and a roof |
| `lib/building/row.py` | generate | Linear placement, Grammar | Every row becomes its buildings: each lot sized in the kit's bays, wall to wall, told what each side faces |
| `lib/building/court.py` | generate | Grammar | Every court becomes its buildings: the yard and each hall sized in the kit's bays about one axis, a wing written once and returned as two mirror images |
| `lib/building/check.py` | lint | Measure | Whether every storey and room can be reached, and stairs, doors and kit are within their limits |
| `lib/land/uplands.py` | generate | Field | A hillside made from noise and the water that runs off it |
| `lib/land/weather.py` | generate | Simulation | Ground steeper than its angle of repose sheds downhill |
| `lib/map/export.py` | export | Delivery | The player map as data |
| `lib/review/checks.py` | lint | Measure | The measures a world is judged by beside the eye |
| `lib/review/form.py` | lint | Measure | How a town's ways are shaped, as numbers |
| `lib/review/near.py` | lint | Measure | Whether each chosen instance stands within reach of a way |
| `lib/review/reach.py` | lint | Measure | Whether a declared body can walk between the places a world states |
| `lib/town/houses.py` | generate | Recipe | One house on each lot, its front to the street |
| `lib/town/kit.py` | generate | Linear placement | A kit of pieces laid along a path, with ends, corners, fillers and steps |
| `lib/town/streets.py` | generate | Network (undirected) | Local streets inside a district, laid on the land |
| `lib/vegetation/clumps.py` | generate | Scatter | Things placed in groups, as a scatter's `clump` does in the engine |
| `lib/water/fill.py` | generate | Derivation (global) | Each tarn becomes a lake filled to where its hollow would run over |
| `lib/ways/check.py` | lint | Measure | Whether water passes beneath each deck, a way meets each end level, and every end is joined |
| `lib/ways/parapet.py` | generate | Linear placement | A run of posts along each edge of a crossing's deck |
| `crates/via-terrain` | generate | Simulation | `lib/terrain`'s program: the land settled under a few stated facts |

### Words coined before the first cut

Written against the prototype on 2026-11-12, with the nearest word a practitioner would use.
`spec/vocabulary.md` section 6 maps each to its word in the first cut, and section 8 lists the
names that still collide. If we keep one of ours, it should be because no existing word fits.

| Ours | Meaning here | Nearest existing word | Keep? |
|---|---|---|---|
| field | a value at every point of the world | mask, map, layer, attribute | yes: it covers all of them |
| feature | a named point, path or region with attributes | spline, volume, marker, actor | yes |
| kind | what a feature does to fields, as data | brush, modifier, stamp, node | open: "kind" says little |
| effect | one change a kind makes to one field | operation, modifier | yes |
| authority: derived, shaped, anchored | who gives way where two things claim the same ground | priority, layer order, lock | yes: nothing else says who yields |
| order | who is applied last | layer order, stack order | merge with authority or explain apart |
| derived feature | a shape worked out from a field, such as a shoreline | procedural mask, generated spline | yes |
| instance | one asset at one place | actor, node, placement | yes |
| pack | the binding of asset names to files and facts | asset library, catalogue, manifest | open: "pack" also means a bought bundle |
| taxon, labels | where an asset is filed, and marks that cut across | category, tag | yes |
| producer | a rule that expands to instances | spawner, scatter, generator | open: six of them overlap |
| kit run | pieces of one width repeated along a line | modular run, spline mesh, array | open |
| set | a group placed as one | prefab, assembly | rename: "assembly" is clearer |
| dress, flank | small things put on or beside a host | set dressing, clutter | fold into one placement rule |
| layer | a separately owned document of one world | sublevel, data layer, layer | yes, but it collides with "terrain layer" |
| variant | a named change merged over a world | state, season, data layer | yes |
| overlay | a change kept in its own file | patch, override, changelist | yes |
| part | a world placed inside another | level instance, sublevel | rename: collides with "building part" |
| seam, portal (of a part) | how a placed world meets its host | blend, doorway | "portal" collides with culling portals |
| step | a point where outside code joins the build | hook, node, plugin, generator | yes |
| deck | a way that leaves the ground | bridge, viaduct, ramp | open: a domain word in the core |
| level (of a building) | a named floor height | storey, floor | rename: collides with level as a map |
| bookmark | a named view for review | camera bookmark, shot | yes |
| diagnostic | a coded statement about the world | validation message, lint | yes |
| reach | whether a walker can get there | reachability, navigability | widen beyond walking |
| build | the folder the compiler writes | cooked data, export, bake | open: "build" also means the whole game |
| zoo | browsing a pack by itself | asset browser, content browser | yes |
