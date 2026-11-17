# City data standards: vocabulary and structure for the layer above individual assets

Verification legend used throughout:
- **[read]** = the cited primary page was fetched and read in this pass.
- **[summary]** = seen only in a search-result summary; not opened.
- **[recalled]** = stated from prior knowledge of the cited standard; the primary document could not be fetched in this pass (size limit, 403 or 404). Treat as needing verification before it is quoted as fact.

## OpenStreetMap: how streets, buildings and land use are tagged, the Simple 3D Buildings schema, and what it does not capture

### Takeaway
OSM is a flat tag vocabulary on three primitives (node, way, relation): streets are centreline ways classed by `highway=*`, buildings are footprint polygons with optional height, level and roof tags, and Simple 3D Buildings adds `building:part` volumes inside a footprint. It has no block, no parcel, no storey object and no facade or interior structure, and its attributes have no enforced schema.

### Cited Findings

Street tags [read]:

| Group | `highway=*` values |
|---|---|
| Roads by importance | motorway, trunk, primary, secondary, tertiary, unclassified, residential, service |
| Link roads | motorway_link, trunk_link, primary_link, secondary_link, tertiary_link |
| Special | living_street, pedestrian, track, bus_guideway, busway, escape, raceway, road |
| Paths | footway, bridleway, steps, corridor, path, via_ferrata, cycleway |

- `highway=*` is "the main key used for identifying any kind of road, street or path"; commonly combined keys on the page include `lanes`, `width`, `surface`, `oneway`, `maxspeed`, `sidewalk`, `junction=roundabout`, `incline`, `lit`, `smoothness`, `tracktype` — [OSM wiki Key:highway](https://wiki.openstreetmap.org/wiki/Key:highway)
- Streets are ways (ordered node lists); areas are supported only for a few values, for example a pedestrian square as a closed way with `highway=pedestrian` + `area=yes` — [OSM wiki Key:highway](https://wiki.openstreetmap.org/wiki/Key:highway)

Building and Simple 3D Buildings tags [read] — all from [OSM wiki Simple 3D Buildings](https://wiki.openstreetmap.org/wiki/Simple_3D_Buildings):

| Tag | Meaning |
|---|---|
| `building=*` | The outline (footprint), a closed way or multipolygon. Carries address, name, overall height. |
| `building:part=*` | A section with different physical characteristics (height, roof, material). Usually `yes`. |
| relation `type=building` | Optional. Roles `outline` and `part`. Without it, all parts inside the outline belong to the building. |
| `height` | Ground contact to top of roof, excluding antennas, spires, rooftop equipment. |
| `min_height` | Height of empty space below the part; `height` is still measured from ground. |
| `building:levels` | Above-ground floors, excluding roof levels. |
| `building:min_level` | Levels skipped below a part (analogue of `min_height`). |
| `roof:shape` | flat, skillion, gabled, half-hipped, hipped, pyramidal, gambrel, mansard, dome, onion, round, saltbox |
| `roof:height` / `roof:angle` | Roof height, or inclination in degrees as an alternative. Facade height = `height` − `roof:height`. |
| `roof:levels` | Floors inside the roof, not counted in `building:levels`. |
| `roof:orientation` | `along` or `across`; ridge defaults to parallel with the longest side. |
| `roof:direction` | Direction the main roof face looks. |
| `building:material`, `building:colour`, `roof:material`, `roof:colour` | Facade and roof surface. |

- When a building has any `building:part`, the outline is not used for 3D rendering — [OSM wiki Simple 3D Buildings](https://wiki.openstreetmap.org/wiki/Simple_3D_Buildings)
- The page states the scheme is limited and "not all details can be described by it"; overlapping 3D volumes should be avoided; non-listed `roof:shape` values exist in the database but are not standardised; the page gives no default height per level — [OSM wiki Simple 3D Buildings](https://wiki.openstreetmap.org/wiki/Simple_3D_Buildings)

Land use and amenity [read] — from [OSM wiki Key:landuse](https://wiki.openstreetmap.org/wiki/Key:landuse):

| Group | `landuse=*` values |
|---|---|
| Developed | residential, commercial (offices, low foot traffic), retail (shops, high foot traffic), industrial, institutional (flagged ambiguous), education, construction, fairground |
| Rural | farmland, orchard, vineyard, meadow, allotments |
| Traffic | railway, highway, depot, garages, port |
| Other | cemetery, military, quarry, recreation_ground, religious, greenfield, brownfield |

- `landuse` is mapped as areas (or multipolygons), not on ways; more specific use is "often implied" by feature tags such as `amenity=school`, `amenity=hospital`, `leisure=park` — [OSM wiki Key:landuse](https://wiki.openstreetmap.org/wiki/Key:landuse)
- The landuse page does not mention parcels, lots, cadastre or city blocks — [OSM wiki Key:landuse](https://wiki.openstreetmap.org/wiki/Key:landuse)
- Game-tool authors note the data-quality side: the Unreal StreetMap plugin README warns that building heights are often missing or incorrect in OSM data — [ue4plugins/StreetMap](https://github.com/ue4plugins/StreetMap)

### Inferences
- OSM separates three independent descriptions of "use": zone (`landuse` polygon), building type (`building=house|apartments|retail|…`), and function at a point or polygon (`amenity`, `shop`, `office`). A game authoring vocabulary probably wants the same split: zone use, building type, and occupant or function.
- OSM's building model is exactly two levels (outline, parts) with extrusion plus a roof primitive. This is the minimal "massing" vocabulary that many renderers already implement.
- What OSM does not capture, as far as the pages read show: block and parcel polygons, a storey as an object (only a count), facade composition (windows, doors, bays), interiors, street cross-section as geometry (width and lanes are optional scalar tags on a centreline), junction geometry, and any required attribute set.

### Gaps
- `amenity=*`, `shop=*`, `addr:*`, `place=city_block`, `place=plot`, `area:highway=*`, and `name`/`layer`/`bridge`/`tunnel` were not confirmed on the pages read (Key:highway was truncated at 100k characters; Key:amenity was not fetched). Their existence is [recalled]; see [OSM wiki Map features](https://wiki.openstreetmap.org/wiki/Map_features).
- No measured figure was collected for how often `height`, `building:levels` or `roof:shape` are actually present (taginfo was not consulted).

## CityGML 3.0 and CityJSON: levels of detail, thematic modules, building decomposition, implicit geometry

### Takeaway
CityGML 3.0 is a typed conceptual model with 17 modules; CityJSON 2.0 is its JSON encoding of a subset, and is the easiest place to read the concrete object hierarchy: a `Building` owns `BuildingPart`, `BuildingStorey`, `BuildingUnit`, `BuildingRoom`, installations and furniture, with surfaces labelled `RoofSurface`, `WallSurface`, `GroundSurface`, `Window`, `Door`. Reusable props are "geometry templates" placed by a reference point plus a 4x4 matrix.

### Cited Findings

Modules [read] — CityGML 3.0 Users Guide table of contents lists: Core, Appearance, City Furniture, City Object Group, Dynamizer, Generics, Land Use, Point Cloud, Relief, Transportation, Vegetation, Versioning, Water Body, Construction, Bridge, Building, Tunnel — [OGC CityGML 3.0 Users Guide 20-066](https://docs.ogc.org/guides/20-066.html)
- "All three modules Building, Bridge, and Tunnel model civil structures"; Construction holds the concepts common to the three — [OGC CityGML 3.0 Users Guide 20-066](https://docs.ogc.org/guides/20-066.html)

CityJSON 2.0.1 object hierarchy [read] — all from [CityJSON 2.0.1 specification](https://www.cityjson.org/specs/2.0.1/):

| 1st-level object (stands alone) | 2nd-level objects (need `parents`) |
|---|---|
| `Building` | `BuildingPart`, `BuildingStorey`, `BuildingUnit`, `BuildingRoom`, `BuildingInstallation`, `BuildingConstructiveElement`, `BuildingFurniture` |
| `Bridge` | `BridgePart`, `BridgeInstallation`, `BridgeConstructiveElement`, `BridgeRoom`, `BridgeFurniture` |
| `Tunnel` | `TunnelPart`, `TunnelInstallation`, `TunnelConstructiveElement`, `TunnelHollowSpace`, `TunnelFurniture` |
| `Road`, `Railway`, `Waterway`, `TransportSquare` | a `Road` may be a child of a `Road` |
| `LandUse`, `PlantCover`, `SolitaryVegetationObject`, `CityFurniture`, `WaterBody`, `TINRelief`, `OtherConstruction`, `GenericCityObject`, `CityObjectGroup` | — |

- Installations, constructive elements, furniture and storeys may have a `Building`, `BuildingPart` or `BuildingRoom` as parent — [CityJSON 2.0.1 specification](https://www.cityjson.org/specs/2.0.1/)
- Relations are expressed only as `parents` / `children` id lists, plus `CityObjectGroup` with `children` and optional `children_roles` — [CityJSON 2.0.1 specification](https://www.cityjson.org/specs/2.0.1/)

Semantic surfaces [read] — from [CityJSON 2.0.1 specification](https://www.cityjson.org/specs/2.0.1/):

| Applies to | Surface types |
|---|---|
| Building family | `RoofSurface`, `GroundSurface`, `WallSurface`, `ClosureSurface`, `OuterCeilingSurface`, `OuterFloorSurface`, `Window`, `Door`, `InteriorWallSurface`, `CeilingSurface`, `FloorSurface` |
| Road, Railway, TransportSquare | `TrafficArea`, `AuxiliaryTrafficArea`, `TransportationMarking`, `TransportationHole` |
| WaterBody | `WaterSurface`, `WaterGroundSurface`, `WaterClosureSurface` |

- Custom semantic types must start with `+` — [CityJSON 2.0.1 specification](https://www.cityjson.org/specs/2.0.1/)

Geometry and LoD [read]:
- Eight geometry types: `MultiPoint`, `MultiLineString`, `MultiSurface`, `CompositeSurface`, `Solid`, `MultiSolid`, `CompositeSolid`, `GeometryInstance`; only linear and planar primitives — [CityJSON 2.0.1 specification](https://www.cityjson.org/specs/2.0.1/)
- Every geometry carries `lod` as a string, either a single digit (CityGML style, "2") or "X.Y" (TU Delft refined LoDs, "2.2"); one object can hold several geometries at different LoDs — [CityJSON 2.0.1 specification](https://www.cityjson.org/specs/2.0.1/)
- The CityJSON compatibility page describes the TU Delft LoDs as refining "the five CityGML LoDs" and applying to buildings only — [CityJSON: CityGML 3.0 compatibility](https://www.cityjson.org/citygml/v30/)

Implicit geometry [read]:
- Templates live in a top-level `geometry-templates` object with a `templates` array and their own `vertices-templates` in local coordinates. A `GeometryInstance` has `template` (index), `boundaries` (one vertex index = the reference point), and `transformationMatrix` (16 values, 4x4, row order) — [CityJSON 2.0.1 specification](https://www.cityjson.org/specs/2.0.1/)
- CityGML implicit geometries are "supported as Geometry Templates" — [CityJSON: CityGML 3.0 compatibility](https://www.cityjson.org/citygml/v30/)

Attributes [read]:
- CityJSON prescribes no attribute names or values; `attributes` may hold any JSON. `Building`, `BuildingPart`, `BuildingUnit`, `Bridge`, `BridgePart` may carry an `address` array — [CityJSON 2.0.1 specification](https://www.cityjson.org/specs/2.0.1/)

What CityJSON drops from CityGML 3.0 [read]:
- Transportation omits `Section`, `Intersection` and `Track`; no curves or parametric surfaces; ADEs replaced by Extensions without inheritance — [CityJSON 2.0.1 specification](https://www.cityjson.org/specs/2.0.1/)
- Versioning module unsupported; only `TINRelief` for terrain; no multiple CRSs; no `relativeToTerrain` / `relativeToWater`; no Terrain Intersection Curve — [CityJSON: CityGML 3.0 compatibility](https://www.cityjson.org/citygml/v30/)

CityGML 3.0 concepts not re-read this pass [recalled] — verify against [OGC 20-010 CityGML 3.0 Conceptual Model](https://docs.ogc.org/is/20-010/20-010.html):

| Concept | Recalled content |
|---|---|
| LoD | LOD0 = planar/footprint, LOD1 = block (extruded prism), LOD2 = generalised shape with roof form and thematic surfaces, LOD3 = detailed shape with openings. 3.0 removed LOD4; interiors may be modelled at any LoD. |
| Space concept | Every feature is a `Space` (`OccupiedSpace` such as a building or tree, `UnoccupiedSpace` such as a room or traffic space) or a `SpaceBoundary` (thematic surface). |
| Building attributes | `class`, `function`, `usage` (code lists), `roofType`, `storeysAboveGround`, `storeysBelowGround`, `storeyHeightsAboveGround`, `storeyHeightsBelowGround`; heights via Construction `height`. |
| New in 3.0 Building | `Storey` and `BuildingUnit` as logical subdivisions; `BuildingRoom` replaces `Room`. |
| Transportation | `Road`/`Railway`/`Track`/`Waterway`/`Square` composed of `Section` and `Intersection`; each holds `TrafficSpace`, `AuxiliaryTrafficSpace`, `ClearanceSpace`; `granularity` = `area`, `way` or `lane`; optional linear (centreline) representation. |
| ImplicitGeometry | A prototype geometry (inline or external file reference) + 4x4 transformation matrix + reference point; typical for trees, lamps, signs, furniture. |
| Vegetation | `SolitaryVegetationObject` (single tree, with species, height, trunkDiameter, crownDiameter) vs `PlantCover` (area or volume of vegetation). |

### Inferences
- The stable core across CityGML 2, CityGML 3 and CityJSON is: Building → BuildingPart, plus the three boundary labels RoofSurface / WallSurface / GroundSurface. Storey, Unit and Room are logical children added in 3.0 and are optional.
- CityJSON shows that the whole hierarchy can be carried by two fields (`type`, `parents`/`children`) plus free attributes; the standard value is the type vocabulary and the surface labels, not the attribute schema.
- The template-plus-matrix instancing model is the same contract a game uses for modular props; CityGML limits it to whole objects (trees, lamps), not to building pieces.
- The `+` prefix rule for custom semantics is a useful convention to borrow: standard terms unprefixed, local terms marked.

### Gaps
- The CityGML 3.0 Conceptual Model (20-010) exceeded the fetch size limit, and three offsets into the Users Guide returned only embedded image data or chapter 7 introductions. All LoD wording, space-concept definitions, Building attributes and Transportation granularity above are [recalled], not quoted.
- Whether every CityJSON-listed module is "fully" supported could not be confirmed: the compatibility page's colour coding did not survive extraction.
- The Kutzner, Chaturvedi and Kolbe 2020 overview paper ([doi 10.1007/s41064-020-00095-z](https://link.springer.com/article/10.1007/s41064-020-00095-z)) redirected to a login page and was not read.

## IFC: where its site, building, storey and space hierarchy overlaps

### Takeaway
IFC's spatial structure is a strict containment tree (project, site, building, storey, space) for a single construction project; it overlaps with CityGML 3.0 at Building, Storey and Room/Space, but has no street, block or parcel concept of its own beyond the site.

### Cited Findings
- No IFC page could be read in this pass: both buildingSMART URLs returned HTTP 403 ([IfcSpatialStructureElement, IFC 4.3](https://standards.buildingsmart.org/IFC/RELEASE/IFC4_3/HTML/lexical/IfcSpatialStructureElement.htm), [IfcBuildingStorey, IFC 4.3](https://standards.buildingsmart.org/IFC/DEV/IFC4_3/HTML/lexical/IfcBuildingStorey.html)).
- [recalled] Spatial composition order is `IfcProject` → `IfcSite` → `IfcBuilding` → `IfcBuildingStorey` → `IfcSpace`, linked by `IfcRelAggregates`; physical elements (walls, slabs, doors) are assigned to one spatial level by `IfcRelContainedInSpatialStructure`; `IfcBuildingStorey` carries an `Elevation`. IFC 4.3 generalises `IfcBuilding` under `IfcFacility` to cover roads, railways and bridges — verify at [buildingSMART IFC 4.3 documentation](https://standards.buildingsmart.org/IFC/RELEASE/IFC4_3/HTML/lexical/IfcSpatialStructureElement.htm).

### Inferences
- Overlap table (IFC side [recalled]; CityJSON side [read]):

| Level | OSM | CityGML 3.0 / CityJSON | IFC |
|---|---|---|---|
| Plot | none | `LandUse` polygon (not a legal parcel) | `IfcSite` |
| Building | `building=*` outline | `Building` | `IfcBuilding` |
| Massing piece | `building:part` | `BuildingPart` | none (aggregated buildings) |
| Storey | `building:levels` count | `BuildingStorey` | `IfcBuildingStorey` |
| Room | none (indoor tagging is separate) | `BuildingRoom` | `IfcSpace` |
| Unit | none | `BuildingUnit` | `IfcZone` (grouping) |

- IFC is the only one of the three where the storey is the primary container for elements; in CityGML and OSM the storey is optional or a number.

### Gaps
- No IFC claim here is verified against a primary page in this pass.

## Street network representations: centreline graphs, OpenDRIVE, and deriving blocks and parcels from a street graph

### Takeaway
Three levels exist: an attributed centreline graph (OSM, Parish and Müller), a reference line with lanes offset from it (OpenDRIVE), and surface polygons (CityGML TrafficArea). In procedural city research blocks are the faces enclosed by the street graph, and lots come from recursive subdivision of each block with a street-access test.

### Cited Findings

OpenDRIVE [read] — from [ASAM OpenDRIVE standard page](https://www.asam.net/standards/detail/opendrive/):
- XML files with extension `.xodr` (compressed `.xodrz`); current version 1.9.0, released 19 May 2026.
- "The ASAM OpenDRIVE road network is modelled along the reference line"; roads, lanes and elevation profiles attach to it.
- Objects and signals are placed either relative to the reference line (s/t coordinates) or in global coordinates.
- In junctions every possible connection between entry roads is its own connecting road; connecting roads are the only roads whose surfaces overlap; lanes are linked across roads.
- The page states that static 3D roadside objects such as trees and buildings need separate description formats; OpenCRG covers surface profiles and OpenSCENARIO covers dynamic content.

OpenDRIVE lane detail [recalled] — verify at [ASAM OpenDRIVE 1.9.0 specification](https://publications.pages.asam.net/standards/ASAM_OpenDRIVE/ASAM_OpenDRIVE_Specification/v1.9.0/specification/index.html):
- Reference-line primitives: line, spiral (clothoid), arc, cubic polynomial, parametric cubic. Roads are cut into lane sections; the centre lane has id 0, left lanes positive, right lanes negative; each lane has a `type` (driving, shoulder, sidewalk/walking, biking, parking, border, curb, median, and others), a width or border polynomial, road marks, speed and access rules.

Parish and Müller 2001 [read, full PDF] — all from [Parish and Müller, "Procedural Modeling of Cities", SIGGRAPH 2001](https://cgl.ethz.ch/Downloads/Publications/Papers/2001/p_Par01.pdf):

| Stage | Method | Output data structure |
|---|---|---|
| Roadmap creation | Extended L-system | Roadmap (graph) |
| Division into lots | Subdivision | Allotments (polygons) |
| Building generation | L-system | Buildings (strings) |
| Geometry | Parser | Polygons |

- Inputs are 2D image maps: elevation; land/water/vegetation; population density; zone maps (residential, commercial or mixed); street-pattern control maps; height maps (maximal house height).
- Two road classes: highways connect population-density peaks; streets fill the areas between highways following the local pattern.
- Named street patterns: Basic (no superimposed pattern), New York (rectangular raster, with maximal block length and width), Paris (radial to centre), San Francisco (follow elevation minimum or maximum).
- The L-system proposes an "ideal successor"; `globalGoals` sets parameters (patterns, population density) and `localConstraints` adjusts or rejects them (land/water/park boundaries, elevation, crossings). A crossing creates a new node and two edges in the street graph.
- Local constraint repairs: prune or rotate a segment to stay in legal area; let highways cross illegal area up to a set length and flag the segment for a bridge; snap to an existing crossing or extend to form an intersection.
- "The populated area of the city is subdivided into many small areas which we call blocks"; figure 10 caption: blocks are "created by scaling from street crossings".
- Lots: most are assumed convex and rectangular; a block is divided by "a simple, recursive algorithm that divides the longest edges that are approximately parallel until the subdivided lots are under a threshold area specified by the user"; concave allotments are forbidden.
- "All allotments that are too small or do not have direct access to a street are discarded."
- One building per allotment; three building types (skyscrapers, commercial buildings, residential houses) chosen by zoning image maps; buildings are made by extruding and transforming the ground plan with templates for roofs and antennae; the L-system axiom is the bounding box, which gives LoD steps for free.
- Results: a virtual Manhattan of about 13,000 buildings; street graph in under ten seconds; lots and buildings about ten minutes.

Vanegas et al. 2012 [summary only]:
- "Procedural Generation of Parcels in Urban Modeling" (Vanegas, Kelly, Weber, Halatsch, Aliaga, Müller; Computer Graphics Forum 31(2), Eurographics 2012) partitions the interior of city blocks into parcels using user-specified subdivision attributes and style parameters, with two methods: skeleton-based and OBB-based — [Eurographics Digital Library record](https://diglib.eg.org/handle/10.1111/v31i2pp681-690); [Purdue CGVLAB publication page](https://cs.purdue.edu/cgvlab/www/publications/vanegas2012procedural)
- A secondary description says the OBB method recursively splits the block along its oriented bounding box, favours regular parcels aligned to an adjacent street and tries to guarantee street access; the skeleton method uses the straight skeleton of the block polygon — search summary of the same records; full text at [Purdue PDF](https://www.cs.purdue.edu/cgvlab/papers/aliaga/eg2012.pdf) (not readable in this pass: size limit).

### Inferences
- The relations that recur: street segment (edge) meets street segment at a junction (node); a block is a face of the planar street graph, inset by street width; a lot is a subdivision cell of a block; a lot has "street access" (a frontage edge on the block boundary); a building sits on one lot.
- "Frontage" is the key derived relation: both Parish and Müller (discard lots without direct street access) and the Vanegas summary (ensure street access) treat it as a validity rule, so a vocabulary should make frontage explicit on the lot rather than leave it implied by geometry.
- Lane-level description (OpenDRIVE) hangs off the same centreline as the graph: lanes are lateral offsets from the reference line, so a centreline graph with a per-edge cross-section profile is a strict simplification of OpenDRIVE, not a different model.
- Junctions are where the representations diverge most: a node in OSM and Parish and Müller, a set of overlapping connecting roads in OpenDRIVE, an `Intersection` area in CityGML 3.0.

### Gaps
- Vanegas et al. 2012 was not read in full; parameter names (parcel area, minimum width, split irregularity), front/side/back edge labelling, and when each method applies (commonly described as skeleton for blocks with interior, OBB for regular blocks) are not verified.
- OpenDRIVE lane sections, lane types and geometry primitives are [recalled]; the specification index returned a redirect and the lanes chapter URL a 404.
- Other block-extraction literature (minimal cycle basis, tensor-field street generation by Chen et al. 2008, Esri CityEngine's block parameters) was not covered.

## 3D Tiles and glTF: per-feature metadata

### Takeaway
3D Tiles 1.1 and glTF define how to attach typed metadata to things (tileset, tile, content, feature, vertex, texel) but define no city vocabulary; property names such as height come from whoever produced the data.

### Cited Findings
- The 3D Metadata model consists of a schema (classes and enums), a class (set of properties), a property (type, component type, optional array, normalisation, offset/scale, min/max, required, noData, default, and an optional `semantic` identifier), and an entity (instance of a class) — [3D Tiles Metadata specification](https://github.com/CesiumGS/3d-tiles/blob/main/specification/Metadata/README.adoc)
- Types: SCALAR, VEC2 to VEC4, MAT2 to MAT4, STRING, BOOLEAN, ENUM; storage as a binary table (entities as rows, properties as columns) or JSON — [3D Tiles Metadata specification](https://github.com/CesiumGS/3d-tiles/blob/main/specification/Metadata/README.adoc)
- Granularity: 3D Tiles 1.1 attaches metadata to tilesets, tiles and contents; the glTF extension `EXT_structural_metadata` attaches it to vertices, texels and features — [3D Tiles Metadata specification](https://github.com/CesiumGS/3d-tiles/blob/main/specification/Metadata/README.adoc)
- The model is schema-agnostic; a "building" class with a height property appears only as an illustration — [3D Tiles Metadata specification](https://github.com/CesiumGS/3d-tiles/blob/main/specification/Metadata/README.adoc)
- [summary] Cesium OSM Buildings is a global 3D Tiles tileset derived from OSM with over 350 million buildings and per-building metadata (name, height, address, opening hours, material of building parts), i.e. OSM tags passed through as feature properties — [Cesium OSM Buildings](https://cesium.com/platform/cesium-ion/content/cesium-osm-buildings/)

### Inferences
- For delivery formats the feature is "a building" identified by a feature id on vertices, with a row in a property table. That is the same shape as OSM (object + tag bag) and CityJSON (object + `attributes`): identity plus free properties, vocabulary supplied elsewhere.
- The useful borrowable part is the property definition structure (type, required, noData, default, enum, semantic), not any term list.

### Gaps
- `EXT_mesh_features` (feature ids) and property tables/textures were not confirmed on the page read; [recalled].
- Exact property names in Cesium OSM Buildings (for example whether `building:levels` is exposed) were not confirmed.
- Esri's I3S scene-layer attribute model was not read from a primary source.

## Use in games and game tools

### Takeaway
Shipped and tooling use is almost entirely "footprint polygon + height + a few class tags, then procedural dressing": the importers read OSM ways and tags, extrude footprints, and class roads; none of the documented pipelines found consumes CityGML semantics, and richer structure (lots, storeys, facades) is generated, not imported.

### Cited Findings

| Tool or game | What it takes from map data | What it generates itself | Source |
|---|---|---|---|
| Microsoft Flight Simulator (2020), Blackshark.ai | Bing Maps 2D imagery; photogrammetry for about 400 cities; "existing map data" and GIS heights where available | Footprints detected by computer vision; height and building characteristics estimated by ML (shadows and rooftop features as likely cues); typical height per building type per area as a fallback; about 1.5 billion buildings; windows and lights added procedurally | [read] [TechCrunch, 17 Aug 2020](https://techcrunch.com/2020/08/17/meet-the-startup-that-helped-microsoft-build-the-world-of-flight-simulator) |
| Houdini Labs OSM Import | Parses OSM XML to curves and polygons; "All of the OSM Tags are parsed and stored as primitive attributes"; optional marker points (parking meters, stop signs); building colour tag to `Cd` | Companion nodes OSM Buildings (generates buildings) and OSM Filter (isolates layers) | [read] [SideFX Labs OSM Import](https://www.sidefx.com/docs/houdini/nodes/sop/labs--osm_import.html) |
| Unreal StreetMap plugin | OSM XML to a Street Map asset; roads with connectivity; projected to a flat plane | Roads as coloured quad strips; simple building meshes with triangulated roofs; no collision, navmesh or UVs; coordinates not retained | [read] [ue4plugins/StreetMap](https://github.com/ue4plugins/StreetMap) |
| Cesium (Unreal, Unity, CesiumJS) | Cesium OSM Buildings tileset with per-building OSM-derived properties, filterable by tag (for example `building` = residential or apartments) | Styling and filtering by property | [summary] [Cesium OSM Buildings](https://cesium.com/platform/cesium-ion/content/cesium-osm-buildings/) |
| ArcGIS Maps SDK for Unreal Engine | Esri-hosted global OSM 3D Buildings layer, added as a 3D Object Scene Layer; identify API (v2.2) returns feature attributes; separate Building Scene Layer type with attribute filters (v1.4) | Material by attribute | [summary] [Esri blog: Maps SDK 2.2 for Unreal Engine](https://www.esri.com/arcgis-blog/products/unreal-engine/announcements/announcing-arcgis-maps-sdk-2-2-for-unreal-engine) |
| Cities: Skylines mods (Cimtographer, OSM Import, Cities-Skylines-Mapper) | Road networks from OSM with names and road-type mapping; Cimtographer also exports a city as OSM | No confirmed import of buildings or zoning; scale and elevation problems reported | [summary] [rdiekema/Cities-Skylines-Mapper](https://github.com/rdiekema/Cities-Skylines-Mapper); [PC Gamer on Cimtographer](https://pcgamer.com/cities-skylines-mod-lets-you-export-your-city-as-a-real-world-map) |
| Pixela Labs (Unreal Fest 2023 talk) | OSM, preprocessed in Rhino, into Houdini | Procedural generation into Unreal with World Partition | [summary] [Unreal forum: Creating 25 Square Miles of Fun with World Partition](https://forums.unrealengine.com/t/talks-and-demos-creating-25-square-miles-of-fun-with-world-partition-unreal-fest-2023/1684752) |
| City Bus Manager | Real OSM data to simulate public transport in real cities | — | [summary] via search result citing The Verge; [OSM wiki Game](https://wiki.openstreetmap.org/wiki/Game) lists further OSM-based games |

- Blackshark's CEO-level statements in the article concede statistical error: "it might be that 20% of the buildings are off" — [TechCrunch, 17 Aug 2020](https://techcrunch.com/2020/08/17/meet-the-startup-that-helped-microsoft-build-the-world-of-flight-simulator)
- The TechCrunch article does not mention OpenStreetMap or regional architecture styles — [TechCrunch, 17 Aug 2020](https://techcrunch.com/2020/08/17/meet-the-startup-that-helped-microsoft-build-the-world-of-flight-simulator)

### Inferences
- Every importer found reduces a building to (footprint, height or levels, type tag, optional roof shape, optional colour or material). This is the de facto interchange record between map data and game geometry.
- Roads reduce to (polyline, class, optional lanes/width/name/oneway). Junction geometry, sidewalks and kerbs are always generated downstream.
- No pipeline found imports blocks or lots; where they are needed they are derived from the street graph, as in the procedural papers.
- The Flight Simulator approach shows the fallback order that matters when attributes are missing: measured height, else inferred height, else typical height for the building type in that region. A text authoring format can use the same order: explicit value, else rule, else type default.

### Gaps
- No Asobo or Blackshark primary technical write-up was found; the role of OSM in Flight Simulator is asserted only in forum posts and is unconfirmed. Roof-type determination was not documented in any source read.
- No shipped commercial game with a published talk on building its city directly from OSM or CityGML was confirmed in this pass. Candidates not checked: The Crew, Forza Horizon, Watch Dogs, Marvel's Spider-Man (the last is known for Houdini-based procedural tooling, not confirmed as map-data driven).
- Details for Cesium, Esri, Cities: Skylines, Pixela Labs and City Bus Manager are from search summaries only.
- No game tool that ingests CityGML or CityJSON semantics directly was found (Esri CityEngine and FME workflows were not searched).

## The smallest common vocabulary for a town's structure

### Takeaway
Across OSM, CityGML/CityJSON, IFC, OpenDRIVE and the procedural papers, about a dozen nouns and half a dozen relations recur; blocks and lots are absent from the mapping standards and come only from the procedural literature and cadastral practice.

### Cited Findings
Agreement table (each cell is the term the source uses; "—" means absent). Sources and verification status are those of the sections above.

| Concept | OSM | CityGML 3.0 / CityJSON | IFC | OpenDRIVE | Parish and Müller |
|---|---|---|---|---|---|
| Street segment | way `highway=*` | `Road` / `Section` | (4.3: road facility) | road + reference line | street / highway segment |
| Junction | shared node | `Intersection` | — | junction + connecting roads | crossing (graph node) |
| Street class | `highway` value | `class` / `function` | — | road type | highway vs street |
| Lane / cross-section | `lanes`, `width`, `sidewalk` tags | `TrafficArea`, `AuxiliaryTrafficArea`, granularity lane | — | lane, lane section, lane type | — |
| Block | — | — | — | — | block |
| Lot / parcel | — | — (`LandUse` is use, not ownership) | `IfcSite` | — | lot / allotment |
| Land use / zone | `landuse=*` | `LandUse` | — | — | zone map (residential, commercial, mixed) |
| Building | `building=*` outline | `Building` | `IfcBuilding` | — | building |
| Building part | `building:part` | `BuildingPart` | — | — | (extrusion steps) |
| Storey | `building:levels` (count) | `BuildingStorey`, storeysAboveGround | `IfcBuildingStorey` | — | — |
| Height | `height`, `min_height` | height attribute | storey elevation | — | height map (max height) |
| Roof | `roof:shape`, `roof:height` | `RoofSurface`, roofType | roof element | — | roof template |
| Wall / ground | (facade = height − roof height) | `WallSurface`, `GroundSurface` | wall, slab | — | facade |
| Opening | — | `Window`, `Door` | window, door | — | facade grid cell |
| Use / function | `building` value, `amenity`, `shop` | `function`, `usage` | space/zone type | — | building type (skyscraper, commercial, residential) |
| Instanced prop | point feature (`natural=tree`, `highway=street_lamp`) | `CityFurniture`, `SolitaryVegetationObject` via template + matrix | type/occurrence | object, signal | — |
| Area vegetation | `landuse=forest`, `natural=wood` | `PlantCover` | — | — | vegetation map |

### Inferences
Smallest shared set (appearing in at least three of the five columns):

| Noun | Minimum attributes | Relations |
|---|---|---|
| Street segment | class, width or lane count, name | connects two junctions; bounds blocks on each side |
| Junction | — | joins N street segments |
| Land-use zone | use (residential, commercial, retail, industrial, …) | covers blocks or lots |
| Building | footprint, height or levels, type/use | stands on a lot; fronts a street |
| Building part | footprint, height, min height, roof | part of a building |
| Storey | index or count, height | part of a building or part |
| Roof | shape, height, orientation | tops a building part |
| Wall / facade surface | material, colour | bounds a building part |
| Opening (window, door) | — | in a wall surface |
| Prop instance | prototype id, position, transform | placed in a street or lot |

Additions needed that the mapping standards do not supply:
- **Block** (face of the street graph) and **lot** (subdivision of a block, with a frontage edge). Only the procedural literature names them; OSM and CityGML have no equivalent object.
- **Frontage / street access** as an explicit lot-to-street relation.

Structural conventions several standards share and that a text format can copy:
- Two-level building massing (outline, then parts that replace it) — OSM and CityGML agree.
- Height measured from ground to roof top, roof height separate, facade height derived — OSM.
- Levels and roof levels counted separately — OSM (`building:levels`, `roof:levels`).
- Use described at three scales: zone, building type, occupant function — OSM and CityGML (`class`, `function`, `usage`).
- LoD as a property of the geometry, not of the object; one object may carry several — CityJSON.
- Instancing as prototype + reference point + 4x4 matrix — CityGML/CityJSON.
- Free attribute bag with a typed schema optional on top — OSM tags, CityJSON `attributes`, 3D Tiles property tables.
- Marked custom terms (`+` prefix) — CityJSON.

### Gaps
- The agreement table mixes [read] cells (OSM, CityJSON, Parish and Müller, OpenDRIVE overview) with [recalled] cells (CityGML 3.0 attributes and Transportation classes, all IFC cells, OpenDRIVE lane cells). It should be checked against the primary standards before terms are adopted verbatim.
- Cadastral standards that do define parcels (ISO 19152 LADM, INSPIRE Cadastral Parcels) and Overture Maps' schema (buildings, building parts, segments, connectors) were out of scope for this pass and would likely fill the block/lot and segment/connector rows.
