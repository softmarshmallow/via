# Ways, settlements and buildings

Sections 6 to 10 of the [ledger](README.md). The row shape and the marks are explained in the
README; the archetypes in [archetypes.md](archetypes.md).

## 6. Ways: roads, streets, paths

From general knowledge (2026-11-12), re-read against the first cut on 2026-11-18. The
subsections were read from sources in 2026-11.

| Term | Meaning | Archetype | Whose | Now |
|---|---|---|---|---|
| road network, street graph | ways and where they meet | Network (undirected) | lib | shown: `[network.<id>]`, `lib/town/streets.py` |
| street hierarchy, way class | which ways are main, minor or mere tracks | Network (undirected) | lib | shown: `town.main_street`, `town.street`, `ways.freeway` |
| street typology | avenue, boulevard, one-way, dead end: named kinds | Profile | lib | none |
| centreline, spline | the line a way follows | Shape | core | shown: `path`, `smooth` |
| cross-section, profile | what a way is across its width: carriageway, shoulder, kerb and footway as offsets from the centreline | Profile | lib | shown: `[kind.<k>.effect.<id>]`, `authority.md 2` |
| carriageway, roadway | the part vehicles use | Profile | lib | shown: `town.street` |
| lane configuration | how many lanes, which direction, how wide | Profile | lib | none |
| turn lane, slip lane, taper | a lane added before a junction for turning, widening in over a set length | Profile | lib | none |
| lane connectivity, turn restrictions | which entry lane leads to which exit lane | Network (directed) | lib | none |
| lane taper, lane drop | where a road gains or loses a lane | Profile | lib | none |
| bike lane, bus lane | lanes reserved for one kind of user | Profile | lib | none |
| sidewalk, pavement | where people walk beside a road: the strip between carriageway and lots | Profile | lib | shown: `town.street` |
| sidewalk zones, tree pit | the strips a sidewalk is divided into | Profile | lib | none |
| kerb, curb | the step between road and sidewalk | Profile | lib | partly: `town.street` |
| kerb ramp, dropped kerb, tactile paving | the kerb lowered where people cross, with the paving that marks it underfoot | Meeting | lib | none |
| driveway apron, curb cut | the kerb lowered for a vehicle entrance | Meeting | lib | none |
| corner radius, kerb return, bulb-out | the kerb's curve round a junction corner | Meeting | lib | none |
| median, verge, shoulder | the strips between and beside | Profile | lib | partly: `ways.road` |
| gutter, storm drain, road crown | how a street sheds water | Profile | lib | partly: `ways.road` |
| grade, gradient | how steep a way climbs, and the most a route may | Shape | lib | shown: `smoothed_along()` |
| camber, banking, superelevation | a way's tilt across its width; through a bend, towards the inside | Profile | lib | partly: `ways.road` |
| intersection, junction | where ways meet at one level; it needs geometry of its own | Meeting | lib | partly: `geometry.md 8` |
| stop line, right of way | who yields, and where they wait | Markup | lib, game | none |
| traffic signal, phasing | lights, and which movements go together | Runtime | game, lib | none |
| traffic island, refuge | raised islands that split movements | Meeting | lib | none |
| roundabout | a junction round an island | Meeting | lib | none |
| crossing, crosswalk | where people cross a road | Meeting | lib | none |
| road marking | paint on the surface: centre line, edge line, stop line, crossing, arrows | Linear placement | lib | none |
| traffic sign, signal, street name sign, wayfinding | objects placed at stations along a way, with a side offset, facing a particular approach | Linear placement | lib, pack | none |
| bus stop, taxi rank, loading zone | kerbside places where vehicles stop | Markup | lib | none |
| on-street parking, bay, meter | marked parking along the kerb | Linear placement | lib | none |
| car park, parking garage | off-street parking: aisles, bays, ramps | Grammar | lib | none |
| alley, service lane, loading dock | a narrow way behind buildings | Network (undirected) | lib | none |
| highway, freeway, motorway | a fast road with no junctions at grade | Network (undirected) | lib | shown: `ways.freeway` |
| on-ramp, off-ramp, merge lane, gore | where traffic joins or leaves a highway | Meeting | lib | partly: `ways.ramp` |
| ramp, slip road | a way between two levels | Network (undirected) | lib | shown: `ways.ramp` |
| interchange types | diamond, trumpet, stack, cloverleaf: standard ramp layouts | Meeting | lib | partly: `ways.interchange` |
| frontage road, service road | a parallel road taking local access | Network (undirected) | lib | none |
| crash barrier, guardrail, noise wall | protection at a highway's edge and middle: a continuous restraint, required where a drop or hazard lies beyond | Linear placement | lib, pack | partly: `lib/ways/parapet.py` |
| gantry, toll plaza | structures spanning a highway | Linear placement | lib, pack | none |
| bridge, viaduct | a way carried over a gap | Representation switch | lib | shown: `ways.crossing`, `ways.viaduct` |
| bridge types | girder, arch, truss, suspension: form chosen by span | Recipe | lib, pack | none |
| pier, abutment, deck, parapet, expansion joint | the parts of a bridge | Recipe | lib | partly: `ways.crossing`, `lib/ways/parapet.py` |
| overpass, flyover, underpass, grade separation | one way over another: two routes crossing at different levels without meeting | Meeting | lib | shown: `ways.viaduct`, `ways.interchange` |
| tunnel, tunnel portal | a way through the ground, and its mouth | Representation switch | lib | none |
| embankment, cutting | ground raised or cut to carry a way | Profile | lib | shown: `ways.road`, `ways.freeway` |
| retaining wall | a wall holding ground at two levels beside a way or lot, where a slope would take too much room | Linear placement | lib | partly: `town.waterfront` |
| culvert, ditch | where water passes under or beside a way | Meeting | lib | none |
| switchback, hairpin | a way zigzagging up a slope | Shape | lib | none |
| wagon road, dirt road, two-track | an unsurfaced vehicle way | Profile | lib | shown: `ways.lane` |
| ruts, wheel tracks, wear | what a way's traffic leaves in its surface: grooves, patches, cracks and stains | Profile | lib | none |
| footpath, trail | a way for walkers, made or unmade: a centreline with a soft profile, worn ground and thinned growth | Profile | lib | shown: `ways.trail` |
| desire line | an unplanned shortcut worn by feet | Guidance | lib | none |
| game trail | a narrow way worn by animals | Profile | lib | none |
| footbridge, stepping stones, rope bridge | light crossings for people only | Representation switch | lib | shown: `ways.crossing`, `examples/example` |
| waymarker, signpost, cairn | objects that say where a way goes | Linear placement | lib | none |
| street furniture | lamps, benches, bins, bollards along a way: elements repeated at a spacing, with room kept round each | Linear placement | lib | shown: `[run.<id>]` |
| manhole, hydrant, utility cover | utility access in road and sidewalk | Linear placement | lib, pack | none |
| utility pole, overhead wires, fence line | poles or posts at stations along a route, with wire or rails spanning between | Linear placement | lib | partly: `span` |
| waterway, ferry route | a route on water | Network (undirected) | lib | shown: `ways.ferry`, `examples/strait` |

### Routes

Finding and shaping a route.

| Term | Meaning | Archetype | Whose | Now |
|---|---|---|---|---|
| least-cost path | route between places minimising slope and other costs (Galin et al. 2010) | Network (undirected) | core | shown: `route`, `derived.md 3` |
| clothoid | curve whose curvature changes linearly; the standard road transition | Shape | core | none |
| L-system streets | streets grown by rewriting rules under local constraints (Parish and Müller 2001) | Grammar | lib | none |
| tensor-field streets | streets traced along two perpendicular direction fields (Chen et al. 2008) | Derivation (global) | lib | none |
| linear referencing | addressing a spot as distance along a curve plus sideways offset | Linear placement | core | shown: `on`, `t`, `offset` |

### Road realisation (corridor modelling)

Turning an abstract route into built form: surface, markings, edges and structures. Game tools
call it a spline road tool; civil engineering calls it corridor modelling, and the
driving-simulation standard OpenDRIVE is the most complete formal description.

**Geometry of the route**

| Term | Meaning | Archetype | Whose | Now |
|---|---|---|---|---|
| horizontal alignment | the route seen from above: straights, arcs and transition curves | Shape | core | partly: `path`, `smooth` |
| vertical alignment (profile) | the route's height along its length: grades joined by smooth crests and sags | Shape | lib | shown: `smoothed_along()` |
| assembly | the full cross-section as an ordered set of parts: lanes, shoulder, kerb, footway, ditch | Profile | lib | partly: `town.street` |
| lane section | a stretch over which the number and widths of lanes stay the same; lanes are added or dropped between sections | Profile | lib | none |
| daylighting | the side slope that runs from the road's edge out to where it meets natural ground | Meeting | lib | shown: `ways.road` |
| corridor | the result of sweeping the assembly along both alignments | Profile | lib | partly: `ways.freeway` |

**Surface and markings**

| Term | Meaning | Archetype | Whose | Now |
|---|---|---|---|---|
| edge blend | the road surface fading into the ground at its sides, not ending at a hard line | Meeting | lib | shown: `ways.road`, `lib/land/ground.md` |
| length-wise mapping | texture laid by distance along the road, so it neither stretches nor swims in bends | Linear placement | loader | none |
| dash pattern | a marking's repeat of stroke and gap, measured along the lane | Linear placement | lib | none |
| marking rule | which marking a lane boundary gets, from the lanes either side (same direction, opposing, may or may not overtake) | Recipe | lib | none |

**Edges and furniture**

| Term | Meaning | Archetype | Whose | Now |
|---|---|---|---|---|
| warrant | the condition that makes an element required: a guardrail where the drop exceeds a height, a sign before a bend | Recipe | lib | partly: `lib/ways/parapet.py` |

**Structures**

| Term | Meaning | Archetype | Whose | Now |
|---|---|---|---|---|
| structure switch | where the road's height departs too far from the ground, earthwork gives way to a bridge, tunnel or wall | Representation switch | lib | partly: `ways.crossing`, `ways.viaduct` |
| span rule | how far a deck may run between supports, which sets pier spacing | Linear placement | lib | none |

**Junctions**

| Term | Meaning | Archetype | Whose | Now |
|---|---|---|---|---|
| junction area | the shared surface where assemblies meet and no single cross-section applies | Meeting | lib | partly: `ways.interchange` |

## 7. Rail and transit

From general knowledge (2026-11-12), re-read against the first cut on 2026-11-18.

| Term | Meaning | Archetype | Whose | Now |
|---|---|---|---|---|
| railway, track | rail on its own right of way | Network (undirected) | lib | none |
| ballast, sleepers, siding, turnout | the parts of track and its support | Linear placement | lib, pack | none |
| curve radius, ruling grade | limits rail places on a route | Shape | lib | none |
| level crossing | where road and rail meet at one level | Meeting | lib | none |
| rail bridge, trestle | rail carried over a gap | Representation switch | lib | none |
| elevated rail, monorail | track on columns above the street | Representation switch | lib | none |
| subway, station entrance, concourse | underground rail and its way to the street | Representation switch | lib | none |
| tram, light rail, catenary | rail sharing the street, with overhead wire | Profile | lib | none |
| station, platform | where trains stop | Recipe | lib | none |
| transit route, timetable | scheduled vehicles on fixed routes | Runtime | game, lib | none |

## 8. Settlements and city structure

From general knowledge (2026-11-12), re-read against the first cut on 2026-11-18.

| Term | Meaning | Archetype | Whose | Now |
|---|---|---|---|---|
| settlement hierarchy | hamlet, village, town, city, and their typical form | Recipe | lib | partly: `lib/town/kinds.toml` |
| settlement siting | why a place stands here: water, flat ground, a crossing | Guidance | lib | none |
| district, neighbourhood | an area of one character | Area partition | lib, game | shown: `town.ward` |
| zoning, land use class | the per-lot rule that picks the building type | Area partition | lib | partly: `lib/town/houses.py` |
| density, height limit, skyline | how much building a lot may carry | Field | lib | none |
| block | the land enclosed by streets: a face of the road network | Area partition | lib | shown: `town.block`, `[network.<id>]` |
| lot, parcel, plot | one holding within a block, with street frontage | Area partition | lib | shown: `town.lot`, `[cells.<id>]` |
| frontage, setback | which street a building faces, and how far back | Area partition | lib | shown: `frontage`, `lib/town/houses.py` |
| courtyard, back-of-lot | the space inside a block behind the frontages | Area partition | lib | none |
| vacant lot, leftover land, under-bridge space | land no rule claims, which still needs filling | Area partition | lib | none |
| plaza, square, park, playground | open public lots | Area partition | lib | partly: `vegetation.green` |
| waterfront, promenade, marina | the city's edge on water | Meeting | lib | shown: `town.waterfront` |
| port, container yard, rail yard | large industrial transport land | Area partition | lib | none |
| airport: runway, taxiway, apron, terminal | an airfield and its buildings | Recipe | lib | none |
| industrial land: factory, tank farm, substation | big-lot uses with fences and truck access | Area partition | lib | none |
| suburb, subdivision | low-density housing with driveways and lawns | Area partition | lib | none |
| construction site, scaffolding | a lot or facade in a temporary state | Overlay | lib, pack | none |
| works: mine, quarry, mill, logging camp | places built around a resource | Recipe | lib | partly: `examples/example` |
| homestead, farmstead, ranch | a dwelling with its working buildings | Recipe | lib | partly: `examples/example` |
| campsite, fire ring | a temporary place people stop | Recipe | lib | none |
| address, street name, house number | names and numbers for streets and lots | Markup | lib | none |
| lot subdivision | splitting by oriented bounding box or straight skeleton (Vanegas et al. 2012) | Area partition | core | shown: `[cells.<id>]` |

## 9. Buildings and kits

From general knowledge (2026-11-12), re-read against the first cut on 2026-11-18.

| Term | Meaning | Archetype | Whose | Now |
|---|---|---|---|---|
| footprint | the ground a building covers | Asset | pack | shown: `assets.md 3` |
| massing | a building as plain volume: its lot shape extruded | Grammar | lib | shown: `building.building` |
| blockout, greybox | a design phase in plain shapes, for any space | Practice | core | shown: `lib/packs/primitive.toml` |
| building typology | rowhouse, walk-up, tower, warehouse: recurring kinds | Recipe | lib, pack | partly: `lib/town/houses.py` |
| podium and tower, stepback | a wide base with a narrower tower above | Grammar | lib | none |
| modular kit | pieces made to fit together | Asset | pack, lib | shown: `lib/town/kit.py`, `examples/market` |
| module, grid, snapping | the size pieces share and lock to | Asset | pack | partly: `examples/market` |
| kit metrics | the kit's agreed dimensions: bay, storey, door | Asset | pack | partly: `examples/market` |
| pivot, origin | the point a piece is placed by | Asset | pack | shown: `pivot` |
| front, facing | which way a piece looks: its agreed forward axis, so a rule can aim it | Asset | pack | shown: `front` |
| socket, connector | where one piece joins another | Asset | pack | shown: `[asset.<id>.mount.<name>]`, `connector` |
| bay, corner, end, filler, centre | the roles pieces play in a row | Linear placement | pack, lib | shown: `lib/town/kit.py` |
| facade | the face of a building as a pattern of pieces | Grammar | lib | partly: `lib/town/kit.py` |
| facade variation | rules that stop neighbours looking identical | Grammar | lib | none |
| shape grammar, facade grammar | rules that subdivide a volume into floors, bays and openings | Grammar | lib | none |
| storey, floor | one layer of a building | Grammar | lib | shown: `building.storey` |
| basement, sub-grade level | floors below the street | Grammar | lib | none |
| roof, eave, ridge, gable | the top and its parts | Grammar | lib | partly: `building.roof` |
| rooftop props | plant, tanks, vents, antennas on a flat roof | Scatter | lib, pack | none |
| trim, cornice, plinth | strips that finish edges | Linear placement | lib | none |
| foundation, stilts, plinth skirt | what takes up the gap under a building | Meeting | lib, pack | partly: `sink` |
| porch, boardwalk, steps to grade | pieces that reach the door from the ground | Meeting | lib | partly: `building.flight` |
| storefront, awning, shop sign | the commercial ground floor and what projects from it | Grammar | lib, pack | none |
| fire escape, external stair | stairs hung on a facade | Grammar | lib, pack | none |
| party wall, blind wall, lightwell | side and back faces between unequal buildings | Meeting | lib | none |
| assembly, saved group | pieces saved and reused as one | Recipe | core | shown: `[set.<id>]` |
| hero asset | a one-off made for one place | Asset | pack | shown: `[asset.<id>]` |
| set dressing, props, clutter | things that make a place lived in | Scatter | lib | shown: `examples/station` |
| grime, wear, graffiti, litter | marks of age and use | Scatter | lib, pack | none |
| environmental story vignette | props arranged to tell what happened: a tiny authored scene, often round a reward | Recipe | game, lib | none |
| interior, enterable building | a building with an inside to walk | Area partition | lib | shown: `building.room`, `examples/station` |
| shell, facade-only building | a building that is only its outside | Asset | pack | shown: `lib/packs/starter.toml` |
| fake interior | a room faked behind glass | Asset | pack, engine | none |
| room, wall, door, window, stair | the parts of an interior | Grammar | lib | shown: `building.room`, `building.doorway`, `building.flight` |
| doorway, threshold | where inside and outside meet | Meeting | core, lib | shown: `building.doorway` |
| lift, ladder, balcony | other ways between levels | Meeting | lib | none |
| skybridge, underground concourse | pedestrian links above or below the street | Representation switch | lib | none |
| wall, fence, hedge, gate | linear boundaries | Linear placement | lib | shown: `examples/monastery`, `[run.<id>]`, `town.gate` |
| ruins, damage states | the same place broken | Overlay | core, pack | none |
| wave function collapse | filling a grid with tiles under adjacency constraints | Grammar | lib | none |

## 10. Asset facts

From general knowledge (2026-11-12), re-read against the first cut on 2026-11-18.

| Term | Meaning | Archetype | Whose | Now |
|---|---|---|---|---|
| axis, handedness and unit convention | each tool's up, front and unit; conversions must agree | Asset | pack, loader | shown: `numbers.md 1`, `front`, `up` |
| interchange format | the file an asset crosses tools in | Asset | pack | shown: `assets.md 3` |
| import settings, reimport | options applied each time an asset is brought in | Asset | pack, loader | none |
| tool round trip | out to a modelling tool and back without loss | Practice | pack | none |
| scan data, bought library | assets captured or bought, with their own habits | Asset | pack | none |
| generated asset | a model made by a generative tool, with uneven facts | Asset | pack | none |
| naming convention, folder structure | names and paths that tools and people rely on | Practice | pack | none |
| asset library, catalogue, taxonomy | how assets are filed and found | Asset | pack | shown: `taxon`, `assets.md 5` |
| asset contract | what an asset must state to be usable | Asset | pack | shown: `assets.md 4` |
| level of detail (LOD) | simpler copies of a model for distance | Delivery | pack | partly: `detail` |
| impostor | a picture standing in for a far object | Delivery | pack, engine | none |
| simple versus complex collision | cheap proxy shapes versus the drawn triangles | Asset | pack | partly: `collision` |
| collision layer, mask | which kinds of thing collide with which | Asset | pack, game | none |
| surface type | what a surface is, for footsteps and friction | Asset | pack | none |
| climbable, no-climb | whether a face can be climbed | Asset | pack, game | none |
| ledge, handhold | an edge a character can grab | Markup | pack, lib | none |
| elemental surface tag | burnable, conductive, wet, frozen: what a surface does when acted on | Asset | pack, game | none |
| breakable, destructible | a thing removed or toppled by force | Asset | pack, game | none |
| texel density | texture pixels per metre, kept even | Asset | pack | none |
| trim sheet, texture atlas | textures shared by many pieces | Asset | pack | none |
| texture set, channel packing | which maps an asset has and how they are packed | Asset | pack | none |
| material instance | one material retuned by parameters | Asset | pack, loader | none |
| per-instance variation | tint or wear per copy without breaking batching | Scatter | loader | shown: `tint`, `scale` |
| vertex paint | per-vertex colour for blending dirt, moss, damage | Asset | pack, loader | none |
| decal | a picture projected on any surface: dirt, leaves, stains, wear | Scatter | lib, pack, loader | none |
| mesh decal | a decal modelled as geometry | Asset | pack | none |
| second texture layout for baked light | what a piece needs to be light-baked | Asset | pack | none |
| emissive surface, lit windows | surfaces that glow without being lights | Asset | pack, loader | none |
| stylised shading | the look as a material rule assets follow | Runtime | pack, engine | none |
| occluder | a shape that hides what is behind it | Delivery | pack, loader | none |
