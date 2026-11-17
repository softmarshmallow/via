# Conventions and standards that define a reusable 3D asset (modular building pieces)

Reading notes: "fetched" means the page was opened in this pass. "Search summary" means the claim was seen only in a search result digest and should be re-verified before it is relied on. Anything I know from general practice but did not confirm against a source in this pass is listed under Gaps, not under Cited Findings.

## 1. Axes and units: what each format and engine defines as up, front and unit of length

### Takeaway
Only glTF 2.0 formally specifies all three of up, front and unit (+Y up, +Z front, metres, right-handed). OpenUSD and FBX declare up axis and unit as per-file metadata but leave "front" undefined or tool-specific; engines fix their own world axes, and "which way an asset faces" is a convention in every engine except where it inherits glTF's rule (Godot documents it explicitly).

### Cited Findings

| System | Handedness | Up | Front / forward | Unit of length | Status of "front" |
|---|---|---|---|---|---|
| glTF 2.0 | Right | +Y | +Z ("the front of a glTF asset faces +Z"), -X is right | metres | Normative in the spec |
| OpenUSD | Right | `upAxis` = "Y" or "Z" per stage; fallback "Y" | Not defined | `metersPerUnit` per stage; fallback 0.01 (cm) | Not specified |
| FBX (SDK) | Right by default; declared per file | Y by default; declared per file | Front axis is a declared field of the axis system (parity) | cm by default; declared per file | Declared, but tool-dependent |
| Unreal Engine | Left | +Z | +X forward, +Y right | cm | Engine world convention |
| Unity | (see Gaps) | (see Gaps) | (see Gaps) | 1 unit = 1 m expected by physics | Engine convention |
| Godot 4 | Right | +Y | Camera looks along -Z; oriented assets face +Z | 1 unit = 1 m | Documented convention, matches glTF |
| Blender | (Z up, see Gaps) | Z | -Y is front, +Y is rear for an asset | (see Gaps) | Convention; exporter converts |

- glTF: "glTF uses a right-handed coordinate system", "glTF defines +Y as up, +Z as forward, and -X as right; the front of a glTF asset faces +Z" (section 3.4). "The units for all linear distances are meters." "All angles are in radians." "Positive rotation is counterclockwise." — [glTF 2.0 Specification](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html)
- OpenUSD up axis: legal values of the stage metadatum `upAxis` are "Y" and "Z"; "In a generic installation of USD, the fallback will be 'Y'"; "UsdGeom stipulates a right-handed coordinate system"; the declaration lives in the stage root layer and "must hold true for all referenced geometry and assets"; the page suggests a set-construction tool apply a corrective rotation when a referenced asset's up axis differs. The page defines no front axis. — [UsdGeom Stage Up Axis](https://openusd.org/release/api/group___usd_geom_up_axis__group.html)
- OpenUSD units: linear measure is stage metadata `metersPerUnit`; unauthored falls back to 0.01 (centimetres). USD does not auto-correct: "it is the assembler's responsibility to apply suitable correctives to the referenced data." Meters-per-unit was chosen so imperial units encode exactly (1 ft = 0.3048 m). — [UsdGeom Linear Units](https://openusd.org/release/api/group___usd_geom_linear_units__group.html)
- FBX: "Objects in the FBX SDK are always created in the right handed, Y-Up axis system"; the implicit unit is centimetres. `ConvertScene()` on an axis system acts on node transforms (pre-rotation) and animations; unit conversion acts on node scale and animations; "calls to ConvertScene() do not change the vertex values of meshes." — [Autodesk FBX SDK: Scene Axis and Unit Conversion](https://help.autodesk.com/cloudhelp/2016/ENU/FBX-Developer-Help/files/GUID-CC93340E-C4A1-49EE-B048-E898F856CFBF.htm)
- FBX predefined axis systems (search summary only, not fetched): MayaYUp is UpVector +Y, FrontVector +Z, right-handed; the header also lists MayaZUp, Max, Motionbuilder, OpenGL, DirectX and Lightwave. — [FbxAxisSystem Class Reference](https://help.autodesk.com/cloudhelp/2016/ENU/FBX-Developer-Help/cpp_ref/class_fbx_axis_system.html)
- Unreal: the editor coordinate system is "left-handed and uses a Z-up axis"; X is forward/backward with positive forward; Y is left/right with positive right; Z positive is up. The page warns that Maya, Houdini, ZBrush, Blender and 3ds Max can use different up axes and handedness so imports may need adjusting. — [Unreal Engine: Coordinate System and Spaces](https://dev.epicgames.com/documentation/en-us/unreal-engine/coordinate-system-and-spaces-in-unreal-engine)
- Unreal default distance unit is centimetres. — [Unreal Engine: Units of Measurement](https://dev.epicgames.com/documentation/en-us/unreal-engine/units-of-measurement-in-unreal-engine)
- Unity: "Unity's physics system expects 1 meter in the game world to be 1 unit in the imported file." The model importer has Scale Factor, Convert Units ("convert the Model scaling defined in the Model file to Unity's scale") and Bake Axis Conversion ("bake the results of axis conversion directly into your application's asset data"; when off, Unity compensates on the root GameObject's Transform). — [Unity Manual 2022.3: Model tab](https://docs.unity3d.com/2022.3/Documentation/Manual/FBXImporter-Model.html)
- Godot: "Godot uses the metric system for everything in 3D, with 1 unit being equal to 1 meter"; "Godot uses a right-handed coordinate system"; Y is up, X is sides, Z is front/back. — [Godot: Introduction to 3D](https://docs.godotengine.org/en/stable/tutorials/3d/introduction_to_3d.html)
- Godot on front: "Godot uses a right-handed, Y-is-up coordinate system, with the -Z axis as the camera's forward direction." "The convention for 3D assets is to face the opposite direction as the camera", so oriented assets have +Z as front. "In Blender, this means that +Y is rear and -Y is front for an asset." Use `use_model_front` on `look_at` and the `Vector3.MODEL_*` constants for asset-local maths. For non-oriented assets (maps, terrain) +X is east, +Z is south, -Z is north. — [Godot: Model export considerations](https://docs.godotengine.org/en/stable/tutorials/assets_pipeline/importing_3d_scenes/model_export_considerations.html)
- Blender glTF exporter has a "+Y Up" transform option: "Export using glTF convention, +Y up." — [Blender 4.2 Manual: glTF 2.0](https://docs.blender.org/manual/en/4.2/addons/import_export/scene_gltf2.html)
- NVIDIA SimReady Foundation makes the USD options mandatory and narrows them: un-001 "Stage must specify upAxis", un-002 "Stage must specify metersPerUnit", un-006 "Stage must specify upAxis = 'Z'", un-007 "Stage must specify metersPerUnit = 1.0", plus "Stage must specify kilogramsPerUnit ... if physics objects are present" and "Must apply corrective transforms for different units". — [SimReady Foundation: Units requirements](https://nvidia.github.io/simready-foundation/latest/capabilities/core/units/requirements.html)

### Inferences
- A consuming tool can trust front and unit from a glTF file without inspecting it, must read `upAxis` and `metersPerUnit` from a USD stage and still cannot learn front from it, and must read the axis system and system unit from an FBX and treat front as "whatever the exporting tool meant".
- Known failure modes follow directly from the cited rules: (a) USD assets referenced across stages with different `upAxis` or `metersPerUnit` come in rotated 90 degrees or scaled by 100 because composition applies no corrective; (b) FBX axis or unit conversion lands as a root-node rotation or scale rather than in vertices, so a piece can look right but carry a non-identity root transform (Unity's Bake Axis Conversion exists to deal with exactly this); (c) cm-versus-m mismatch between Unreal or FBX defaults (cm) and glTF, Unity, Godot (m) gives 100x or 0.01x errors; (d) handedness flips between right-handed formats and left-handed Unreal mirror one axis, which matters for asymmetric kit pieces and for left or right variants.
- Blender's -Y front becomes glTF +Z front under the exporter's +Y-up conversion, which is why Godot's doc pairs the two conventions.
- SimReady (Z up, 1 m) and glTF (Y up, 1 m) agree on unit but not on up axis; USD's own fallback (Y up, cm) agrees with neither on unit.

### Gaps
- Unity's world axes (commonly described as left-handed, Y up, +Z forward) were not confirmed from a Unity primary page in this pass; the fetched importer page only says a model may use "a different axis system than Unity".
- Blender's native Z-up, right-handed world and its unit system were not fetched from the Blender manual; only the exporter option and Godot's statement about Blender's -Y front were confirmed.
- The exact up/front/handedness of each FBX predefined axis system (Max, OpenGL, DirectX, Lightwave, Motionbuilder) and the meaning of front-vector parity were not confirmed; the 2020 class reference URL returned 404.
- Whether Unreal 5.x documentation has changed its axis naming or FBX import defaults was not found on the fetched page.
- glTF's statements that time is in seconds and that cameras and lights look along local -Z were not reached in the fetched portion of the spec (the page is about 826k characters; only the first 200k were read).

## 2. Pivot or origin conventions for modular kit pieces

### Takeaway
No file format constrains where an asset's origin sits; pivot placement is purely a kit-level convention, and published guides disagree between "corner on the floor" (Epic's FBX pipeline doc and Unreal-oriented guides) and "centre on the ground plane" (Bethesda). The shared rules are: pivot on the ground plane, pivot on the grid, same rule for every piece, and never change it after placement begins.

### Cited Findings
- Epic: on import the pivot is "always located at the origin (0,0,0) when exporting from a 3D modeling application"; meshes should be built at the origin, typically with the origin at one corner of the mesh, so they snap correctly in the editor. — [Unreal Engine: FBX Static Mesh Pipeline](https://dev.epicgames.com/documentation/en-us/unreal-engine/fbx-static-mesh-pipeline-in-unreal-engine)
- Bethesda (Burgess and Purkeypile, GDC 2013): "Generally, the pivots of our kit pieces are at the ground plane of the kit and in the center." Exceptions: a pipe kit may pivot at the pipe's edge so it hinges; a platform kit may pivot at its base. "Changing pivots later can be a huge problem, requiring manual updates to hundreds of instances." — [Game Developer: Skyrim's Modular Approach to Level Design](https://gamedeveloper.com/design/skyrim-s-modular-approach-to-level-design)
- Bethesda also describes "Snap to Reference", where the editor treats any object as the origin for snapping, and a "pivot-and-flange" system in the Ratway kit where a full-curvature archway hides gaps created by rotated pieces. — [Game Developer: Skyrim's Modular Approach to Level Design](https://gamedeveloper.com/design/skyrim-s-modular-approach-to-level-design)
- A third-party UE5 kit-workflow blog (search summary only; vendor blog, not an authority) calls a corner of the piece's grid footprint on the floor plane the "near-universal choice" and says corner pivots with grid snap set to the module size or a clean divisor (50 or 100) make pieces land flush by construction; it recommends grey proxy pieces with the exact final dimensions and pivot. — [StraySpark: modular kit workflow](https://www.strayspark.studio/blog/modular-kit-workflow-greybox-to-final-ue5)
- Godot advises applying object transforms before export. — [Godot: Model export considerations](https://docs.godotengine.org/en/stable/tutorials/assets_pipeline/importing_3d_scenes/model_export_considerations.html)
- Epic requires LODs to be "aligned and occupying the same space with the same pivot point." — [Unreal Engine: FBX Static Mesh Pipeline](https://dev.epicgames.com/documentation/en-us/unreal-engine/fbx-static-mesh-pipeline-in-unreal-engine)

### Inferences
- Corner pivot suits pieces placed by translation on a grid (edges meet when positions are multiples of the module); centre pivot suits pieces that are rotated in 90-degree steps in place (a square footprint stays in its cell). Bethesda's footprint-centred pivot goes with their habit of snapping at half the footprint size (see section 3).
- Since the convention is not machine-declared anywhere, a consuming tool has to either be told the kit's pivot rule or infer it from the bounding box relative to the origin (origin at bbox min corner, at bbox floor centre, or elsewhere).

### Gaps
- The Polycount wiki "Modular environments" page could not be fetched (connection closed), so its pivot advice is not cited.
- No primary Unity or Godot document recommending a pivot location for kit pieces was found; Godot's GridMap page mentions a "Center Y" setting but does not state where an item's origin sits in the cell.

## 3. Grid and module sizes used in practice

### Takeaway
Module sizes are per-kit choices, not standards. Two families appear in published sources: power-of-two footprints in engine units (Bethesda: 512 and 256, snap at half the footprint) and metric round numbers in centimetres for Unreal (100 cm grid, walls 400 to 600 cm wide, 300 cm high in one guide). The only rule stated as near-universal is that sub-kit footprints must be integer multiples of one another.

### Cited Findings
- Bethesda: "The footprint is the foundation of the entire kit"; it is the full bounds of the piece, not the walkable space. A 512x512x512 room tiles with a 256x256x256 hallway, whereas a 384x384x384 room "will eventually create gaps and/or overlaps." "The most common footprints for kits are equilateral." "Level designers tend to build on a grid snap setting of one-half the size of the footprint." Keep snap sizes as large as possible. — [Game Developer: Skyrim's Modular Approach to Level Design](https://gamedeveloper.com/design/skyrim-s-modular-approach-to-level-design)
- Bethesda scale: "a character is usually about 128 units, or six feet, tall"; the environment is Z-up; Unreal units are cited as similar. Do not build to the edge of the footprint or outside it. Hallways usually tile on one axis and rooms on two; decide floor thickness in advance for vertical stacking. Narrowest traversable space is "a minimum of two character widths"; 30 to 45 degree slopes are suggested. — [Game Developer: Skyrim's Modular Approach to Level Design](https://gamedeveloper.com/design/skyrim-s-modular-approach-to-level-design)
- Epic marketplace rule as quoted by the Allar style guide (section 4.3): modular socketless assets must "snap cleanly when the grid is set to 10 units or bigger." — [Allar UE5 Style Guide](https://github.com/Allar/ue5-style-guide)
- Unreal forum reply (search summary only; community post): modular assets are commonly built on a 100 cm grid, with a wall for example 400 to 600 cm wide. — [Unreal Engine Forums: first time modular environment help](https://forums.unrealengine.com/t/first-time-modular-environment-help/1268900)
- Vendor blog example sizes (search summary only): 400 x 400 cm floor tile, wall 400 cm wide by 300 cm high, 200 cm half-wall, 50 x 50 x 300 cm pillar to hide seams, stair rise equal to a whole module. — [StraySpark: modular kit workflow](https://www.strayspark.studio/blog/modular-kit-workflow-greybox-to-final-ue5)
- Godot GridMap: the cell size is a property set "to the size of your meshes"; no standard size is prescribed. — [Godot: Using GridMaps](https://docs.godotengine.org/en/stable/tutorials/3d/using_gridmaps.html)

### Inferences
- From Bethesda's 128 units for about six feet, one unit is roughly 1.4 cm, so a 256 footprint is roughly 3.6 m and a 512 footprint roughly 7.3 m. This is my arithmetic, not a figure from the talk.
- Module size is never carried in the file by any format surveyed; it lives in the kit's documentation, in piece names (sometimes), or has to be recovered from bounding boxes.

### Gaps
- No primary source was fetched for Unreal's power-of-two grid habit (legacy snap values such as 16, 32, 64 ... units) or for the current editor default snap steps.
- Published module sizes of widely used commercial kits (for example Kenney, Synty, KitBash3D) were not researched in this pass.
- The Polycount wiki page was unreachable, so its grid recommendations are not cited.
- The original talk transcript on the author's blog timed out; all Bethesda facts come from the Game Developer repost.

## 4. Connection or attachment points

### Takeaway
Every ecosystem has a way to store a named transform on an asset (Unreal socket, Unity child transform, Godot node, glTF node, USD Xform, Blender empty), but none standardises its meaning. "A door goes here" or "this end joins another wall" is carried by naming convention or custom metadata. The only formal, engine-recognised name prefixes are Unreal's `SOCKET_` and collision prefixes and Godot's import suffixes.

### Cited Findings

| System | Mechanism | Declared how | Semantics standardised? |
|---|---|---|---|
| Unreal | Static mesh socket | Authored in Socket Manager, or imported from FBX helper named `SOCKET_[RenderMeshName]_##` | Name only; meaning is project convention |
| Godot | Scene nodes, import suffixes, GridMap MeshLibrary items | Node name suffixes; MeshLibrary scene layout | Suffixes cover collision, nav, occluder, not connections |
| glTF | Any node (name + transform), plus `extras` | Free-form | No |
| OpenUSD | Prims with `kind = subcomponent`, custom properties | Kind metadata | Partially: "important sub part" |
| Blender | Empties, custom properties | Exported as nodes and `extras` | No |

- Unreal sockets: "Sockets are attach points you can place on your Static Mesh"; created in the Static Mesh Editor's Socket Manager, each has a name and a relative location, rotation and scale; actors are attached via Attach To and choosing a socket; attached objects do not auto-snap until their relative transform is reset. — [Unreal Engine: Using Sockets with Static Meshes](https://dev.epicgames.com/documentation/en-us/unreal-engine/using-sockets-with-static-meshes-in-unreal-engine)
- Unreal FBX socket naming: `SOCKET_[RenderMeshName]_##` applied to dummy or helper objects in the modelling tool; each FBX with sockets should contain only one render mesh. — [Unreal Engine: FBX Static Mesh Pipeline](https://dev.epicgames.com/documentation/en-us/unreal-engine/fbx-static-mesh-pipeline-in-unreal-engine)
- Allar's style guide has no socket naming rule but distinguishes "modular socketless assets", which must snap to the project grid, implying socket-based kits are the alternative. — [Allar UE5 Style Guide](https://github.com/Allar/ue5-style-guide)
- Godot import suffixes (work with `-`, `$` or `_`, case-insensitive): `-noimp` (skip), `-col` (trimesh static collision child), `-convcol` (convex), `-colonly` (replace mesh with StaticBody3D collision), `-convcolonly`, `-occ` and `-occonly` (occluder), `-navmesh`, `-vehicle`, `-wheel`, `-rigid`, `-loop`/`-cycle` (animations); material hints `-alpha`, `-vcol`. Blender empties named `-colonly` become primitive shapes by draw type (Collada only): single arrow to SeparationRayShape3D, cube to BoxShape3D, image to WorldBoundaryShape3D, sphere to SphereShape3D. — [Godot: Node type customization using name suffixes](https://docs.godotengine.org/en/stable/tutorials/assets_pipeline/importing_3d_scenes/node_type_customization.html)
- Godot GridMap: a MeshLibrary is built from a scene whose Node3D root has MeshInstance3D children, one per item; each item may have up to one StaticBody3D child (with CollisionShape3D children) and up to one NavigationRegion3D child; "Other node types placed as children will not be recognized and exported." GridMap is designed to place meshes with collision and navigation, not arbitrary nodes. — [Godot: Using GridMaps](https://docs.godotengine.org/en/stable/tutorials/3d/using_gridmaps.html)
- glTF: any top-level object may have a `name`, and names "are not guaranteed to be unique"; a node is "an object defining the hierarchy relations and the local transform of its content." — [glTF 2.0 Specification](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html)
- Blender: custom properties export as glTF `extras` when the option is enabled and always import. — [Blender 4.2 Manual: glTF 2.0](https://docs.blender.org/manual/en/4.2/addons/import_export/scene_gltf2.html)
- OpenUSD: a subcomponent "just identifies some prim inside a component that may be 'important'", with articulation points such as a DoorAssembly Xform given as the example. — [OpenUSD Glossary](https://openusd.org/release/glossary.html)
- Bethesda uses helper markers to show how a kit should snap, and encodes connectivity in piece names (`1Way`, `2way`, `3way`, `4way` for hallway junctions; Cor/In/Mid for corner, inside, tiling). — [Game Developer: Skyrim's Modular Approach to Level Design](https://gamedeveloper.com/design/skyrim-s-modular-approach-to-level-design)

### Inferences
- A cross-source tool has two portable carriers for connection points: an empty named node (survives FBX, glTF and USD) and key-value metadata on that node (glTF `extras`, USD custom attributes or `customData`, FBX user properties). The vocabulary on top (door, wall end, facing direction, compatible neighbour) has to be defined by the tool.
- Because glTF names are not unique and Unreal's socket prefix embeds the render mesh name, a connection-point scheme should key on a prefix plus a typed tag rather than on the full name.
- Most grid kits carry no explicit connectors at all: adjacency is implied by the footprint and pivot rule, and by role tokens in names.

### Gaps
- Unity's convention (empty child GameObjects in a prefab as anchors) and Godot's Marker3D were not confirmed from primary docs in this pass.
- No published, widely adopted naming convention for typed connectors on building kits (for example a door or wall-end prefix) was found; I found only engine-level prefixes.
- The glTF spec text on `extras` was not reached in the fetched portion; its use is confirmed only via the Blender exporter documentation.

## 5. Metadata carried with an asset

### Takeaway
OpenUSD has the richest standard vocabulary for declared asset metadata (stage metrics, `kind` and model hierarchy, `assetInfo`, `customData`, variant sets, purposes, extents hints, default prim), and SimReady turns parts of it into testable requirements. glTF's standard metadata is thin: free-form `extras`, XMP packets for provenance and licensing, material variants, and newer interactivity and visibility extensions; physics is still a draft.

### Cited Findings

OpenUSD
- `kind` is "a reserved, prim-level metadatum whose authored value is a simple string token". A model is "a scenegraph annotation ascribable to prims by setting their kind metadata". "The model hierarchy defines a contiguous set of prims descending from a root prim on a stage, all of which are models." Components are leaf models that cannot contain other models; assemblies are "group models ... that aggregate other models into meaningful collections". — [OpenUSD Glossary](https://openusd.org/release/glossary.html)
- `assetInfo` is "a metadata dictionary that can be applied to any UsdPrim or UsdProperty" with fields `identifier`, `name`, `version` and `payloadAssetDependencies`; it is advisory and not consumed by USD core composition. `customData` is "a special, dictionary-valued metadatum" for user metadata that needs no configuration. — [OpenUSD Glossary](https://openusd.org/release/glossary.html)
- A VariantSet lets a creator "package a discrete set of alternatives, between which a downstream consumer is able to non-destructively switch". Purposes are `default`, `render` (final-quality render only), `proxy` (lightweight preview) and `guide` (shown only on request). A payload is a deferred, loadable reference. — [OpenUSD Glossary](https://openusd.org/release/glossary.html)
- `upAxis` and `metersPerUnit` are stage-level metadata in the root layer. — [UsdGeom Stage Up Axis](https://openusd.org/release/api/group___usd_geom_up_axis__group.html); [UsdGeom Linear Units](https://openusd.org/release/api/group___usd_geom_linear_units__group.html)
- NVIDIA's "Principles of Scalable Asset Structure in OpenUSD" names four principles (legibility, modularity, performance, navigability). The root layer is the asset interface layer; set `defaultPrim` as the entry point; use the reference-then-payload pattern and loft cheap fields above the payload; publish `extentsHint` (via GeomModelAPI) so bounds are known before the payload loads; "All ancestors of a component model must have their kind metadata set to group or a subkind of group"; an assembly is an aggregate such as a neighbourhood or city; keep the model hierarchy shallow; prefer custom properties or schemas over custom kinds; prefix asset-level primvars with `asset_`; variant selections create new instancing prototypes whereas primvars do not; put proxy and render geometry in separate scopes; class prims under an `asset_classes` scope serve as `inherits` targets. — [NVIDIA: Principles of Scalable Asset Structure in OpenUSD](https://docs.omniverse.nvidia.com/usd/latest/learn-openusd/independent/asset-structure-principles.html)
- SimReady Foundation organises requirements into capabilities including units, naming and paths, hierarchy, semantic labels, physics rigid bodies and colliders, geometry and materials, each with validator IDs (for example un-001 to un-007 for units). — [SimReady Foundation: Units requirements](https://nvidia.github.io/simready-foundation/latest/capabilities/core/units/requirements.html)
- SimReady overview (search summary only): the spec covers semantic labelling, non-visual sensor attributes, rigid-body physics and mass; it is built from profiles, features and capabilities; the initial spec covers static props. — [NVIDIA: The SimReady Specification](https://docs.omniverse.nvidia.com/simready/latest/overview/simready-spec.html)

glTF
- Khronos extension registry status as listed: KHR_xmp_json_ld (ratified), KHR_materials_variants (ratified), KHR_interactivity (ratified), KHR_node_visibility (ratified), EXT_mesh_gpu_instancing (ratified), KHR_physics_rigid_bodies (review draft), KHR_collision_shapes (review draft), MSFT_lod (vendor). — [Khronos glTF extensions registry](https://github.com/KhronosGroup/glTF/blob/main/extensions/README.md)
- KHR_xmp_json_ld: packets live in a root `extensions.KHR_xmp_json_ld.packets` array as JSON-LD with an `@context`; `asset`, `scene`, `node`, `mesh`, `material`, `image` and `animation` objects can reference a packet by index; typical content is Dublin Core and XMP namespaces (title, creator, rights, dates); metadata has no normative effect on rendering; metadata on `asset` applies to the whole file and object-level metadata takes precedence. — [KHR_xmp_json_ld](https://github.com/KhronosGroup/glTF/blob/main/extensions/2.0/Khronos/KHR_xmp_json_ld/README.md)
- Extension-defined attribute names take the form `<extension>:<attribute>`. — [glTF 2.0 Specification source](https://raw.githubusercontent.com/KhronosGroup/glTF/main/specification/2.0/Specification.adoc)
- Blender round-trips custom properties through `extras`. — [Blender 4.2 Manual: glTF 2.0](https://docs.blender.org/manual/en/4.2/addons/import_export/scene_gltf2.html)

### Inferences
- For a building kit, USD's vocabulary maps cleanly: one kit piece is a `component`, a building is an `assembly` (with `group` ancestors), a door hinge or connector frame is a `subcomponent`, style or damage options are variant sets, module size and role would be custom properties or `customData`, and footprint can be published as `extentsHint`.
- In glTF the same information has to go in `extras` (or a private extension); XMP covers who made it and under what licence, not what the piece is or how it connects.
- Nothing in either standard declares module size, front face, role (wall, floor, corner) or connector type. That layer is always a private contract.

### Gaps
- I did not locate a separate Alliance for OpenUSD (AOUSD) "asset structure guidelines" document; the asset-structure guidance found is NVIDIA's Learn OpenUSD page. Whether AOUSD has published its own version is unconfirmed.
- The glossary entries for `defaultPrim`, `extent` and `extentsHint` were not reached; those are cited only via the NVIDIA guidance page.
- SimReady's exact semantic-label schema and physics attribute requirements were not fetched (only the units table was).
- OMI_* glTF extensions and KHR_implicit_shapes were not present on the registry page as read; their status is unknown from this pass.
- FBX user-defined properties as a metadata carrier were not researched.

## 6. Level-of-detail, collision and bounds conventions

### Takeaway
LOD and collision are carried by naming conventions that differ per engine (Unreal `UCX_`/`UBX_`/`UCP_`/`USP_` prefixes, Unity `_LOD0..n` suffixes, Godot `-col` family suffixes); neither core glTF nor FBX standardises them. USD is the only format with a first-class bounds hint and a proxy/render split.

### Cited Findings
- Unreal collision prefixes, matched to the render mesh by name: `UBX_[RenderMeshName]_##` (box; "must be created using a regular rectangular 3D object"), `UCP_[RenderMeshName]_##` (capsule), `USP_[RenderMeshName]_##` (sphere), `UCX_[RenderMeshName]_##` (convex; "can be any completely closed convex 3D shape"). Multiple hulls take numeric suffixes (UCX_Tree_01_00, UCX_Tree_01_01) and work best when they do not intersect. Spheres and boxes do not work on non-uniformly scaled meshes, so UCX is generally preferred. When several meshes with custom collision are in one file only the first mesh's collision is imported. — [Unreal Engine: FBX Static Mesh Pipeline](https://dev.epicgames.com/documentation/en-us/unreal-engine/fbx-static-mesh-pipeline-in-unreal-engine)
- Unreal LODs: must be "aligned and occupying the same space with the same pivot point"; each LOD may use different materials; imported via the Import LODs option; the pipeline uses FBX 2020.2. — [Unreal Engine: FBX Static Mesh Pipeline](https://dev.epicgames.com/documentation/en-us/unreal-engine/fbx-static-mesh-pipeline-in-unreal-engine)
- Allar style guide section 4: all meshes must have UVs and non-overlapping lightmap UVs; "any mesh that can be seen at varying distances should have proper LODs"; "all meshes should have proper collision defined"; "all assets should be scaled correctly to their project", with in-engine scale "treated as a scale override, not a scale correction." — [Allar UE5 Style Guide](https://github.com/Allar/ue5-style-guide)
- Unity LOD naming (search summary of legacy manual pages; the current page 404'd): meshes whose names end in `_LOD0`, `_LOD1`, `_LOD2` ... get an LOD Group created automatically on import; LOD 0 is the most detailed. — [Unity Manual 2018.2: Level of Detail](https://docs.unity3d.com/2018.2/Documentation/Manual/LevelOfDetail.html)
- Unity: Generate Colliders attaches mesh colliders to imported meshes automatically and "should be avoided for geometry you are moving." — [Unity Manual 2022.3: Model tab](https://docs.unity3d.com/2022.3/Documentation/Manual/FBXImporter-Model.html)
- Godot collision, occluder and navmesh suffixes: see section 4. — [Godot: Node type customization using name suffixes](https://docs.godotengine.org/en/stable/tutorials/assets_pipeline/importing_3d_scenes/node_type_customization.html)
- Godot MeshLibrary items carry at most one static body and one navigation region; lightmap UV2 is auto-generated at 0.1 units texel size if missing. — [Godot: Using GridMaps](https://docs.godotengine.org/en/stable/tutorials/3d/using_gridmaps.html)
- Godot export advice: triangulate before export (N-gon results "may be unpredictable or incorrect") and apply transforms. — [Godot: Model export considerations](https://docs.godotengine.org/en/stable/tutorials/assets_pipeline/importing_3d_scenes/model_export_considerations.html)
- glTF: LOD exists only as the vendor extension MSFT_lod; physics shapes are review drafts (KHR_collision_shapes, KHR_physics_rigid_bodies). — [Khronos glTF extensions registry](https://github.com/KhronosGroup/glTF/blob/main/extensions/README.md)
- USD: `extentsHint` advertises bounds before payload load; `proxy` and `render` purposes separate stand-in from full geometry. — [NVIDIA: Principles of Scalable Asset Structure in OpenUSD](https://docs.omniverse.nvidia.com/usd/latest/learn-openusd/independent/asset-structure-principles.html)
- Bethesda distinguishes the footprint (full bounds, the tiling contract) from traversable space, and says not to build to or beyond the footprint edge. — [Game Developer: Skyrim's Modular Approach to Level Design](https://gamedeveloper.com/design/skyrim-s-modular-approach-to-level-design)

### Inferences
- For modular pieces the important bound is the footprint (the grid cell the piece claims), which is generally larger than the mesh's bounding box and is declared by no standard field. USD `extentsHint` is a geometric bound, not a footprint.
- A cross-engine kit either ships collision as specially named meshes per target engine or lets each engine generate it; there is no single naming that all three engines recognise.

### Gaps
- Unreal's FBX LOD naming (LOD Group node, `_LOD0` suffix behaviour) is not described on the fetched page.
- Typical LOD counts, triangle budgets and collision complexity expected of marketplace kits (for example Fab listing requirements) were not researched.
- Godot's automatic mesh LOD generation on import was not confirmed in this pass.

## 7. Asset naming conventions

### Takeaway
The widely cited Unreal style guide encodes asset type (prefix), variant and texture channel (suffix), not module size or role. Role and connectivity encoding appears in kit-specific schemes such as Bethesda's; module size in names was not found in any source read.

### Cited Findings
- Allar pattern: `Prefix_BaseAssetName_Variant_Suffix`; variants are descriptive (`Bob_Evil`) or two-digit numbers from `01` (`Rock_01`). Prefixes: Static Mesh `S_` ("Many use SM_. We use S_."), Skeletal Mesh `SK_`, Material `M_`, Material Instance `MI_`, Blueprint `BP_`, Texture `T_`. Texture suffixes: `_D` base colour, `_N` normal, `_R` roughness, `_A` alpha, `_O` ambient occlusion, `_E` emissive, `_M` mask or metallic, `_S` specular; packed textures stack letters (`_ERO`). Nothing in the guide encodes module size. — [Allar UE5 Style Guide](https://github.com/Allar/ue5-style-guide)
- Bethesda example `UtlBayCorInMidPRTT01L01`: Utl (Utility kit), Bay (sub-kit), Cor (corner), In (inside), Mid (tiling piece), PRTT01 (the authors themselves say "No clue"), L (mirrored), 01 (visual variant). Hallway pieces are named by exit count (`1Way`, `2way`, `3way`, `4way`). "The key to good naming is balancing brevity with readability." Use the same conventions across all kits. — [Game Developer: Skyrim's Modular Approach to Level Design](https://gamedeveloper.com/design/skyrim-s-modular-approach-to-level-design)
- Engine-reserved name tokens: Unreal `UBX_`, `UCP_`, `USP_`, `UCX_`, `SOCKET_` prefixes; Godot `-col`, `-colonly`, `-convcol`, `-convcolonly`, `-occ`, `-occonly`, `-navmesh`, `-noimp`, `-rigid`, `-vehicle`, `-wheel` suffixes; Unity `_LOD<n>` suffix. — [Unreal Engine: FBX Static Mesh Pipeline](https://dev.epicgames.com/documentation/en-us/unreal-engine/fbx-static-mesh-pipeline-in-unreal-engine); [Godot: Node type customization](https://docs.godotengine.org/en/stable/tutorials/assets_pipeline/importing_3d_scenes/node_type_customization.html); [Unity Manual 2018.2: Level of Detail](https://docs.unity3d.com/2018.2/Documentation/Manual/LevelOfDetail.html)
- NVIDIA's USD guidance treats legibility of prim, property and resource identifiers as a first principle, and SimReady has a "Naming and Paths" capability with its own requirements. — [NVIDIA: Principles of Scalable Asset Structure in OpenUSD](https://docs.omniverse.nvidia.com/usd/latest/learn-openusd/independent/asset-structure-principles.html); [SimReady Foundation](https://nvidia.github.io/simready-foundation/latest/capabilities/core/units/requirements.html)

### Inferences
- Names are a lossy and kit-specific channel: even Bethesda's own authors could not decode one token of their example. A consuming tool should treat names as hints and require role, size and connectivity as explicit metadata.
- Reserved engine tokens collide with free naming: a piece whose name happens to end in `_col` or start with `UCX_` will be reinterpreted by Godot or Unreal on import.

### Gaps
- SimReady's specific naming and path rules were not fetched.
- No survey of how commercial kits name pieces by dimension (for example `Wall_4x3`) was done; whether such size-in-name schemes are common is unverified.

## What is standardised versus convention (summary of the above)

### Takeaway
Standardised and machine-readable: glTF axes, front and unit; USD `upAxis`, `metersPerUnit`, `kind`, `assetInfo`, variant sets, purposes, extents hints; FBX axis system and unit; engine-reserved name tokens for collision, LOD and sockets. Convention only: front in USD and FBX, pivot location, module or footprint size, connector meaning, role, and nearly all naming.

### Cited Findings
- See the sources cited in sections 1 to 7; this section adds no new facts.

### Inferences
- The contract a multi-source tool must define for itself is exactly the convention-only layer: pivot rule, footprint and module size, front face, typed connection points, and role vocabulary. The standard fields give it somewhere to store those (glTF `extras`, USD custom properties or `customData`), not a schema.

### Gaps
- Sources that failed to load in this pass and should be retried if the report depends on them: the Polycount wiki "Modular environments" page, the original Burgess transcript, the Autodesk FbxAxisSystem class reference, the current Unity LOD import page, and the latter part of the glTF specification.
