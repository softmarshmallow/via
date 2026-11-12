# Ledger: the vocabulary of a game world

Every term someone has to know about when a world is built, from the big picture down to the
seams. The frame runs from a stylised exploration world, through a naturalistic wilderness, to a
dense modern city. A term being listed is not a promise to handle it. The point is that none of
them surprises us.

Third draft, 2026-11-12. How it was made:
1. One author's pass: 201 terms in 14 groups.
2. Four independent readers, each with one lens (stylised exploration, naturalistic wilderness,
   dense city, production in Godot and Unreal), returned about 400 additions and 40 corrections.
3. Four sections on looks were added: the work, the theories, and the tricks (sections 21 to 24).
   These had one author and no review.
4. The project's own words and what the whole list shows were added (sections 25 and 26).

All of it is from general knowledge, not from sources. Engine names are in section 20 and
attributions in section 22; both must be checked before being relied on. The structure is meant to
be stable. The rows are not: add freely, one line each.

**Whose** is a first guess at the owner:

| Mark | Owner |
|---|---|
| core | the compiler: a mechanism with no domain word in it |
| lib | a domain library of ours on top of the core |
| pack | the asset contract: a fact stated about an asset |
| loader | our engine-side loader for Godot or Unreal |
| engine | the engine itself |
| game | the user's game code and design |
| carries | the game owns the meaning; the core carries the data to it |
| ? | undecided |

**Now** is where the spike stands: shown, partly, none.

## 1. World structure and scale

| Term | Meaning | Whose | Now |
|---|---|---|---|
| open world | one continuous map the player crosses freely; teleports and instances may still load | game | none |
| region, zone, area | a named part of the world with its own character and rules | core | partly |
| sub-area, locale, named place | a named spot inside a region; shows a title on entry | core | none |
| biome | a kind of land: its ground, growth and climate together | lib | none |
| landmark | something seen from afar that the player steers by | core | shown |
| point of interest (POI) | a place worth going to, marked for map and design | core | shown |
| vista, composition, focal point, reveal | a view arranged on purpose | game | none |
| sightline, viewshed | what can be seen from a point | core | partly |
| silhouette, skyline | the outline of land and landmarks against the sky | core, game | none |
| world scale compression | distances shortened so travel stays interesting | game, core | none |
| map layer: underground, surface, sky | stacked playable strata over one map footprint | core | none |
| layered levels, stacked spaces | several walkable levels over one ground point | core | partly |
| instance, instanced dungeon, domain | a separate map entered through a door, with its own load | core, game | partly |
| region gate, hazard barrier | what keeps the player out: a wall, a level, a deadly cold | game, carries | none |
| world bounds, playable area | where the player may go, and what stops them | game, core | none |
| backdrop, distant land | what lies beyond the playable world | loader | none |
| authoring tile, chunk | the unit we compile and hand over | core | shown |
| runtime cell, streaming cell | the unit the engine loads; rarely the same as our tile | engine, loader | none |
| world streaming | loading and unloading parts of the world as the player moves | engine, loader | none |
| streaming source, loading range | the point cells load around, and how far | loader, game | none |
| always loaded versus streamed | whether a thing stays resident or comes and goes with its cell | loader | none |
| cell straddler | a thing larger than a cell | core, loader | none |
| streaming hitch, late load | a stall or missing ground when loading lags the player | engine, game | none |
| traversal speed budget | the fastest the player moves that loading can keep up with | game, core | none |
| loading corridor | a narrow passage that hides a load | core, loader | none |
| placed level, level instance | a piece of world kept as its own file and placed in another | core | partly |
| switchable set, data layer | a named set of things turned on or off together | loader | none |
| world precision | keeping positions exact far from the origin | engine | none |
| scale, units | how big a metre, a door and a character are | core, pack | shown |

## 2. Terrain and ground

| Term | Meaning | Whose | Now |
|---|---|---|---|
| heightmap, heightfield | the ground as one height per grid post | core | shown |
| heightmap format, bit depth | how heights are stored; 8 bits shows as steps | core | shown |
| height scale, vertical range | what world heights the stored values stand for | core, loader | shown |
| tiled terrain import | very large terrain as a grid of files | core, loader | none |
| terrain resolution | metres between posts | core | shown |
| feature narrower than the posts | a rut or ditch the height field cannot hold | core | none |
| landform: ridge, valley, saddle, pass, mesa, canyon, plateau | named shapes of land that rules and people refer to | lib | none |
| relief at three scales | terrain shape read large, medium and fine | core | none |
| slope, aspect, curvature | measures of the ground that rules read | core | partly |
| flow accumulation, drainage, wetness | how much upstream ground drains through each post | core | partly |
| erosion | water and slumping shaping the land, simulated at build time | lib, game | partly |
| deposition, talus, alluvial fan | where eroded material comes to rest | lib | partly |
| erosion masks: flow, wear, deposit | by-products of erosion that drive ground and scatter | core | none |
| strata, bedrock, outcrop | rock type and layering showing through the soil | lib, pack | none |
| cliff | a face too steep or overhanging for a height field | lib | shown |
| overhang, cave, arch | ground above ground | core | partly |
| pit, chasm, sinkhole | a vertical opening joining surface and underground | core | none |
| floating island | a landmass with no ground link and a visible underside | lib | none |
| terrain hole | a gap cut in the height field | core | partly |
| building pad, platform | ground levelled for something to stand on | lib | shown |
| natural or farmed terrace | stepped ground made by a river or by farming | lib | none |
| stamp | a reusable shape pressed into the terrain | core | partly |
| weight map, splat map | how much of each ground material lies at each post | core | shown |
| ground material, terrain layer | one material the ground is painted with | pack | shown |
| material limit per tile | a cap on how many materials meet in one terrain piece | core, loader | partly |
| rule-driven ground material | material chosen by slope, height and wetness, not painted | lib | shown |
| height blend, layer blend | how two ground materials mix at their border | pack, loader | none |
| macro variation, colour map | large-scale colour that hides texture repeats | core, loader | none |
| triplanar mapping | texturing steep faces without stretching | engine, pack | none |
| mesh-to-terrain blend | softening the line where a mesh enters the ground | loader, engine | none |
| snow cover, snow line | where snow lies, by height, slope and aspect | core | partly |
| ground wetness, puddles, mud | ground changed by rain or thaw | loader, engine | none |
| deformable ground | snow and mud that record what passed over | engine, game | none |
| coarser ground with distance | terrain drawn simpler further away | engine | none |
| displacement | fine relief added when drawn | engine | none |
| terrain texture streaming | the ground's texture loaded by need | engine | none |

## 3. Water

| Term | Meaning | Whose | Now |
|---|---|---|---|
| sea level | the height below which there is sea | core | shown |
| ocean, sea | open water to the horizon | lib, engine | partly |
| shoreline, coast, beach | where land meets water and how the ground grades in | core | shown |
| lake, pond | still water held by the ground | lib | shown |
| water depth, bathymetry | the ground under water, as a depth field | core | partly |
| wade, swim, dive depths | the depths at which movement changes | core, game | partly |
| watershed, drainage basin, divide | the land draining to one outlet, and its edge | lib | none |
| river network, tributary, confluence | rivers as a tree joining downstream | lib | none |
| river, stream | water flowing downhill along a course | lib | none |
| channel, bed, bank | the trench a river runs in, and its sides | lib | partly |
| meander, oxbow, braided channel | forms a river takes on flat ground | lib | none |
| floodplain, river terrace | flat ground a river made | lib | none |
| delta, estuary, river mouth | where a river spreads into still water | lib | none |
| spring, source | where a stream begins | lib | none |
| ford, shallows | water shallow enough to cross | lib, game | none |
| rapids | a steep, rough reach of river | lib | none |
| waterfall, plunge pool | where a river drops | lib | none |
| flow map, current | direction and speed of water | lib, game | none |
| wetland, marsh | ground at water level | lib | none |
| changeable water level | a water body whose height is a world state | core | none |
| ice, frozen lake, glacier | water in its solid states | lib, engine | none |
| hazard liquid: lava, poison | a liquid surface that harms or slows | game, carries | none |
| cloud sea | a cloud sheet used as a false ground or sea | lib, engine | none |
| quay, wharf, pier, seawall, dock, jetty | the built edge between land use and water | lib | none |
| swim region | a region that counts as "in water" | loader, game | none |
| waves, tides, foam, caustics | how water moves and looks | engine | none |
| underwater | the world seen and moved through below the surface | engine, game | none |

## 4. Vegetation and ecology

| Term | Meaning | Whose | Now |
|---|---|---|---|
| vegetation, foliage | everything that grows | lib | shown |
| climate fields: temperature, moisture, rainfall | the fields biomes are derived from | core | partly |
| altitude band, tree line | growth changing with height until trees stop | lib | shown |
| ecotone, biome transition | the mixed strip between two biomes | lib | none |
| scatter, procedural placement | many things placed by a rule | core | shown |
| density map, mask | where and how thick, as a field or painted picture | core | shown |
| points and attributes | what placement rules pass along: positions with named values | core | partly |
| minimum spacing | keeping scattered things apart | core | partly |
| clumping, clustering | things gathering in groups with gaps | lib | none |
| species mix, palette | which plants, in what share | lib, pack | partly |
| per-species settings | a plant's density, draw distance and collision | pack, loader | none |
| growth stage, size variation | the same species young, mature and dying | pack, lib | partly |
| hero tree, specimen | one hand-placed plant that anchors a view | pack | shown |
| canopy, forest edge, glade, meadow | the forest's cover and its openings | lib | none |
| understory, undergrowth | what grows beneath and because of the trees | lib | none |
| riparian growth, reeds | plants that follow water | lib | partly |
| leaf litter, forest floor | ground material caused by the trees above | lib | none |
| deadfall, snag, stump, log | dead wood standing or lying | lib, pack | shown |
| disturbance: burn scar, clear-cut | growth removed by an event, and regrowing | lib | none |
| ecosystem simulation | placement by what could survive there | lib | none |
| ground cover, grass | growth too fine to place as objects | loader, engine | partly |
| exclusion, clearing | growth kept off roads, buildings and paths | core | shown |
| rocks, boulders | loose stone placed like growth | lib | shown |
| scree | a slope deposit caused by the cliff above | lib | shown |
| interactive growth | cuttable, burnable, shakeable, bending underfoot | pack, game | none |
| harvestable, resource node | a plant or rock the player can take from | game, carries | partly |
| region-exclusive resource | a gatherable found only in one region | game, carries | none |
| wind | what makes growth move; needs a world direction and per-asset facts | engine, pack | none |
| seasons | the same world in another state | core | shown |

## 5. Land use and farming

| Term | Meaning | Whose | Now |
|---|---|---|---|
| land use, land cover | what each piece of land is used for | core | none |
| field, field pattern, strip, furrow | farmland divided into worked pieces | lib | partly |
| field boundary: fence, dry-stone wall, hedgerow, ditch | what divides one field from the next | lib | none |
| pasture, paddock, corral, pen | enclosed or open ground for stock | lib | none |
| crop, orchard, row planting | growth planted in rows by people | lib | shown |
| stile, cattle grid, field gate | where a way passes through a boundary | lib | none |

## 6. Ways: roads, streets, paths

| Term | Meaning | Whose | Now |
|---|---|---|---|
| road network, street graph | ways and where they meet | lib | partly |
| street hierarchy, way class | which ways are main, minor or mere tracks | lib | shown |
| street typology | avenue, boulevard, one-way, dead end: named kinds | lib | none |
| centreline, spline | the line a way follows | core | shown |
| cross-section, profile | what a way is across its width | lib | shown |
| carriageway, roadway | the part vehicles use | lib | shown |
| lane configuration | how many lanes, which direction, how wide | lib | none |
| turn lane, slip lane | a lane added before a junction for turning | lib | none |
| lane connectivity, turn restrictions | which entry lane leads to which exit lane | lib | none |
| lane taper, lane drop | where a road gains or loses a lane | lib | none |
| bike lane, bus lane | lanes reserved for one kind of user | lib | none |
| sidewalk, pavement | where people walk beside a road | lib | shown |
| sidewalk zones, tree pit | the strips a sidewalk is divided into | lib | none |
| kerb, curb | the step between road and sidewalk | lib | partly |
| kerb ramp, dropped kerb | the kerb lowered where people cross | lib | none |
| driveway apron, curb cut | the kerb lowered for a vehicle entrance | lib | none |
| corner radius, bulb-out | the kerb's curve round a junction corner | lib | none |
| median, verge, shoulder | the strips between and beside | lib | partly |
| gutter, storm drain, road crown | how a street sheds water | lib | none |
| grade, gradient | how steep a way climbs | lib | shown |
| camber, banking | a way's tilt across its width | lib | none |
| intersection, junction | where ways meet at one level | lib | none |
| stop line, right of way | who yields, and where they wait | lib, game | none |
| traffic signal, phasing | lights, and which movements go together | game, lib | none |
| traffic island, refuge | raised islands that split movements | lib | none |
| roundabout | a junction round an island | lib | none |
| crossing, crosswalk | where people cross a road | lib | none |
| road marking | paint on the surface | lib | none |
| traffic sign, street name sign, wayfinding | signs placed to face a particular approach | lib, pack | none |
| bus stop, taxi rank, loading zone | kerbside places where vehicles stop | lib | none |
| on-street parking, bay, meter | marked parking along the kerb | lib | none |
| car park, parking garage | off-street parking: aisles, bays, ramps | lib | none |
| alley, service lane, loading dock | a narrow way behind buildings | lib | none |
| highway, freeway, motorway | a fast road with no junctions at grade | lib | partly |
| on-ramp, off-ramp, merge lane, gore | where traffic joins or leaves a highway | lib | partly |
| ramp, slip road | a way between two levels | lib | shown |
| interchange types | diamond, trumpet, stack, cloverleaf: standard ramp layouts | lib | partly |
| frontage road, service road | a parallel road taking local access | lib | none |
| crash barrier, noise wall | protection at a highway's edge and middle | lib, pack | none |
| gantry, toll plaza | structures spanning a highway | lib, pack | none |
| bridge, viaduct | a way carried over a gap | lib | shown |
| bridge types | girder, arch, truss, suspension: form chosen by span | lib, pack | none |
| pier, abutment, parapet, expansion joint | the parts of a bridge | lib | partly |
| overpass, flyover, underpass | one way over another | lib | shown |
| tunnel, tunnel portal | a way through the ground, and its mouth | lib | none |
| embankment, cutting | ground raised or cut to carry a way | lib | none |
| retaining wall | a wall holding ground beside a way or lot | lib | none |
| culvert, ditch | where water passes under or beside a way | lib | none |
| switchback, hairpin | a way zigzagging up a slope | lib | none |
| wagon road, dirt road, two-track | an unsurfaced vehicle way | lib | shown |
| ruts, wheel tracks, wear | what a way's traffic leaves in its surface | lib | none |
| footpath, trail | a way for walkers, made or unmade | lib | shown |
| desire line | an unplanned shortcut worn by feet | lib | none |
| game trail | a narrow way worn by animals | lib | none |
| footbridge, stepping stones, rope bridge | light crossings for people only | lib | shown |
| waymarker, signpost, cairn | objects that say where a way goes | lib | none |
| street furniture | lamps, benches, bins, bollards along a way | lib | shown |
| manhole, hydrant, utility cover | utility access in road and sidewalk | lib, pack | none |
| utility pole, overhead wires | poles along a route with wires hung between | lib | none |
| waterway, ferry route | a route on water | lib | shown |

## 7. Rail and transit

| Term | Meaning | Whose | Now |
|---|---|---|---|
| railway, track | rail on its own right of way | lib | partly |
| ballast, sleepers, siding, turnout | the parts of track and its support | lib, pack | partly |
| curve radius, ruling grade | limits rail places on a route | lib | partly |
| level crossing | where road and rail meet at one level | lib | partly |
| rail bridge, trestle | rail carried over a gap | lib | partly |
| elevated rail, monorail | track on columns above the street | lib | none |
| subway, station entrance, concourse | underground rail and its way to the street | lib | none |
| tram, light rail, catenary | rail sharing the street, with overhead wire | lib | none |
| station, platform | where trains stop | lib | none |
| transit route, timetable | scheduled vehicles on fixed routes | game, lib | none |

## 8. Settlements and city structure

| Term | Meaning | Whose | Now |
|---|---|---|---|
| settlement hierarchy | hamlet, village, town, city, and their typical form | lib | partly |
| settlement siting | why a place stands here: water, flat ground, a crossing | lib | none |
| district, neighbourhood | an area of one character | lib, game | shown |
| zoning, land use class | the per-lot rule that picks the building type | lib | partly |
| density, height limit, skyline | how much building a lot may carry | lib | none |
| block | the land enclosed by streets | lib | shown |
| lot, parcel, plot | one holding within a block | lib | partly |
| frontage, setback | which street a building faces, and how far back | lib | partly |
| courtyard, back-of-lot | the space inside a block behind the frontages | lib | none |
| vacant lot, leftover land, under-bridge space | land no rule claims, which still needs filling | lib | none |
| plaza, square, park, playground | open public lots | lib | shown |
| waterfront, promenade, marina | the city's edge on water | lib | none |
| port, container yard, rail yard | large industrial transport land | lib | none |
| airport: runway, taxiway, apron, terminal | an airfield and its buildings | lib | none |
| industrial land: factory, tank farm, substation | big-lot uses with fences and truck access | lib | none |
| suburb, subdivision | low-density housing with driveways and lawns | lib | none |
| construction site, scaffolding | a lot or facade in a temporary state | lib, pack | none |
| works: mine, quarry, mill, logging camp | places built around a resource | lib | none |
| homestead, farmstead, ranch | a dwelling with its working buildings | lib | shown |
| campsite, fire ring | a temporary place people stop | lib | none |
| address, street name, house number | names and numbers for streets and lots | lib | none |

## 9. Buildings and kits

| Term | Meaning | Whose | Now |
|---|---|---|---|
| footprint | the ground a building covers | pack | shown |
| massing | a building as plain volume | lib | shown |
| blockout, greybox | a design phase in plain shapes, for any space | core | shown |
| building typology | rowhouse, walk-up, tower, warehouse: recurring kinds | lib, pack | partly |
| podium and tower, stepback | a wide base with a narrower tower above | lib | none |
| modular kit | pieces made to fit together | pack, lib | partly |
| module, grid, snapping | the size pieces share and lock to | pack | partly |
| kit metrics | the kit's agreed dimensions: bay, storey, door | pack | none |
| pivot, origin | the point a piece is placed by | pack | none |
| front, facing | which way a piece looks | pack | partly |
| socket, connector | where one piece joins another | pack | none |
| bay, corner, end, filler, centre | the roles pieces play in a row | pack, lib | partly |
| facade | the face of a building as a pattern of pieces | lib | none |
| facade variation | rules that stop neighbours looking identical | lib | partly |
| shape grammar, facade grammar | rules that subdivide a volume into pieces | lib | none |
| storey, floor | one layer of a building | lib | shown |
| basement, sub-grade level | floors below the street | lib | none |
| roof, eave, ridge, gable | the top and its parts | lib | none |
| rooftop props | plant, tanks, vents, antennas on a flat roof | lib, pack | none |
| trim, cornice, plinth | strips that finish edges | lib | none |
| foundation, stilts, plinth skirt | what takes up the gap under a building | lib, pack | partly |
| porch, boardwalk, steps to grade | pieces that reach the door from the ground | lib | none |
| storefront, awning, shop sign | the commercial ground floor and what projects from it | lib, pack | none |
| fire escape, external stair | stairs hung on a facade | lib, pack | none |
| party wall, blind wall, lightwell | side and back faces between unequal buildings | lib | none |
| assembly, saved group | pieces saved and reused as one | core | shown |
| hero asset | a one-off made for one place | pack | shown |
| set dressing, props, clutter | things that make a place lived in | lib | shown |
| grime, wear, graffiti, litter | marks of age and use | lib, pack | none |
| environmental story vignette | props arranged to tell what happened | game, lib | none |
| interior, enterable building | a building with an inside to walk | lib | shown |
| shell, facade-only building | a building that is only its outside | pack | shown |
| fake interior | a room faked behind glass | pack, engine | none |
| room, wall, door, window, stair | the parts of an interior | lib | shown |
| doorway, threshold | where inside and outside meet | core, lib | partly |
| lift, ladder, balcony | other ways between levels | lib | none |
| skybridge, underground concourse | pedestrian links above or below the street | lib | none |
| wall, fence, hedge, gate | linear boundaries | lib | partly |
| ruins, damage states | the same place broken | core, pack | none |

## 10. Asset facts

| Term | Meaning | Whose | Now |
|---|---|---|---|
| axis, handedness and unit convention | each tool's up, front and unit; conversions must agree | pack, loader | partly |
| interchange format | the file an asset crosses tools in | pack | partly |
| import settings, reimport | options applied each time an asset is brought in | pack, loader | none |
| tool round trip | out to a modelling tool and back without loss | pack | none |
| scan data, bought library | assets captured or bought, with their own habits | pack | none |
| generated asset | a model made by a generative tool, with uneven facts | pack | none |
| naming convention, folder structure | names and paths that tools and people rely on | pack | none |
| asset library, catalogue, taxonomy | how assets are filed and found | pack | partly |
| asset contract | what an asset must state to be usable | pack | partly |
| level of detail (LOD) | simpler copies of a model for distance | pack | none |
| impostor | a picture standing in for a far object | pack, engine | none |
| simple versus complex collision | cheap proxy shapes versus the drawn triangles | pack | partly |
| collision layer, mask | which kinds of thing collide with which | pack, game | none |
| surface type | what a surface is, for footsteps and friction | pack | none |
| climbable, no-climb | whether a face can be climbed | pack, game | none |
| ledge, handhold | an edge a character can grab | pack, lib | none |
| elemental surface tag | burnable, conductive, wet, frozen | pack, game | none |
| breakable, destructible | a thing removed or toppled by force | pack, game | none |
| texel density | texture pixels per metre, kept even | pack | none |
| trim sheet, texture atlas | textures shared by many pieces | pack | none |
| texture set, channel packing | which maps an asset has and how they are packed | pack | partly |
| material instance | one material retuned by parameters | pack, loader | none |
| per-instance variation | tint or wear per copy without breaking batching | loader | shown |
| vertex paint | per-vertex colour for blending dirt, moss, damage | pack, loader | none |
| decal | a picture projected on any surface | lib, pack, loader | none |
| mesh decal | a decal modelled as geometry | pack | none |
| second texture layout for baked light | what a piece needs to be light-baked | pack | none |
| emissive surface, lit windows | surfaces that glow without being lights | pack, loader | none |
| stylised shading | the look as a material rule assets follow | pack, engine | none |
| occluder | a shape that hides what is behind it | pack, loader | none |

## 11. Movement, traversal and collision

| Term | Meaning | Whose | Now |
|---|---|---|---|
| collision | what stops a body | pack, loader | partly |
| walkable slope, step height | what a character can walk up | game, core | shown |
| traversal metrics | jump gap, climb height, glide ratio: numbers levels are built to | game, core | partly |
| climb, mantle, vault | getting up and over | game | none |
| stamina-gated route, rest ledge | a route reachable only with enough stamina | game, lib | none |
| glide, updraft, wind current | air that carries a glider | game, carries | none |
| launcher, grapple point, zipline | a placed device that throws or pulls the player | game, carries | none |
| moving platform, mechanism | walkable geometry that moves | game, loader | none |
| swim, dive, mount, drive | other ways of moving | game | none |
| mount needs: turning room, jumpable obstacle | what a ridden animal needs to pass | game, lib | none |
| driveable surface, kerb mount height | what a vehicle can drive on and over | game, pack | none |
| fall height, slide slope | drops and slopes that hurt or carry the player | game, core | none |
| reachability | whether a given mover can get from here to there | lib | partly |
| sequence break, shortcut | reaching a place earlier than designed | game, lib | none |
| soft lock, stuck spot | a place the player can enter and not leave | lib | none |
| clearance, headroom | free space a body or vehicle needs | pack, lib | partly |
| navigation mesh | where an agent can walk, as a mesh | engine, loader | none |
| navigation agent | the body size a navigation mesh is built for | game, loader | none |
| agent class | separate walkability for person, horse, wagon, car | game, loader | partly |
| navigation area, cost | ground tagged as sidewalk, road, forbidden | lib, loader | none |
| navigation link | a jump, ladder or door joining walkable areas | lib, game | none |
| navigation built ahead or live | computed at build, or around agents at run time | engine, loader | none |
| navigation across cells | navigation data that loads with its cell | loader | none |
| blocking region | a stop that is not drawn | game, loader | none |
| out of bounds, fall-out floor | where the player is returned or dies | game | none |
| teleport point, fast travel | a place the player may jump to, often unlocked | game, carries | none |
| teleport arrival spot | exact position and facing after a teleport | core | none |

## 12. Gameplay markup

| Term | Meaning | Whose | Now |
|---|---|---|---|
| tags, groups | free labels game code finds things by | core, game | shown |
| spawn point, spawn zone | where things appear | game, carries | shown |
| respawn rule, refresh timer | when a taken thing or beaten enemy returns | game | none |
| enemy camp | a physical place enemies hold | game, lib | none |
| encounter | a designed fight and its ground | game | none |
| boss arena | a fixed strong enemy and its shaped ground | game, lib | none |
| leash, aggro range | how far an enemy chases before resetting | game | none |
| difficulty zone, level band | how strong foes are per area | game, carries | none |
| patrol path | a route an agent follows | game, carries | none |
| trigger region | a region that starts something | game, carries | partly |
| quest marker, objective | where the story points | game | none |
| checkpoint, save point | where progress is kept | game | none |
| chest, loot, reward tier | things to find, matched to effort | game, carries | partly |
| collectible | counted finds scattered to reward exploring | game, carries | none |
| puzzle element | plate, torch, switch: placed parts the player must solve | game, carries | none |
| puzzle link, wiring | which switch opens which gate | carries | none |
| seal, barrier, lock | a block removed by a condition | game, carries | none |
| shrine, mini-dungeon entrance | a world door to a small instanced challenge | game, carries | none |
| time trial, open-world challenge | a marker that starts a timed task | game, carries | none |
| guide object | a thing that leads the player along a route | game, carries | none |
| station: campfire, forge, cooking pot | a placed spot for a crafting activity | game, carries | none |
| shop, safehouse, service location | an enterable place with a game function | game | none |
| interactable | something the player can use | game, carries | shown |
| scenario point, smart object | a spot telling a character what to do there | game, carries | none |
| world event, random encounter | something staged where the player happens to be | game | none |
| emergency response, dispatch | services sent to an incident | game | none |
| safe zone | a region with other rules | game, carries | none |
| cover point, vantage | positions tactical agents use | game | none |
| concealment, stealth grass | growth or shadow that hides the player | game, carries | none |
| tracks, clues | marks the player follows to a target | game, carries | none |
| golden path, breadcrumb, leading line | the route design pulls the player along | game, lib | none |
| POI density, content spacing | how often something new comes into view | game, lib | none |
| secret, hidden nook | a place meant to be found only by looking | game | none |
| lookout, viewpoint | a spot giving a survey view | game, carries | partly |
| beat, pacing | the planned rhythm along a route | game | none |
| camera region, camera hint | a region or mark that steers the camera | game, loader | none |
| music region, soundscape | what a place sounds like | game, carries | none |
| sound emitter, reverb region | a placed sound, or a region that changes sound | game, loader | none |

## 13. A living world

| Term | Meaning | Whose | Now |
|---|---|---|---|
| NPC | a character who is not the player | game | partly |
| crowd, pedestrians | many characters, simulated cheaply | game, engine | none |
| pedestrian graph, crossing link | where walkers go and where they may cross | lib, game | none |
| traffic | vehicles and riders driven by the game | game | none |
| traffic lane graph | the lanes traffic follows | game, lib | none |
| parked vehicle, parking slot | where stationary vehicles are placed or spawned | game, lib | none |
| density by district and hour | how full the streets are, where and when | game, carries | none |
| spawn and despawn radius | ambient agents existing only near the player | game, engine | none |
| archetype mix | which people and vehicles appear in which district | game, pack | none |
| schedule, routine | where a character is at what hour | game | none |
| fauna, wildlife | animals as part of the game; no one word is standard | game | none |
| habitat, range | where a species may be found | game, carries | partly |
| herd, flock, pack; predator and prey | animals as groups, and who hunts whom | game | none |
| den, roost, watering hole | fixed places animals use | game, carries | none |
| fishing spot | water marked for what can be caught | game, carries | none |
| livestock, draft animals | owned animals, and non-motor traffic | game | none |
| activity pattern, migration | which animals appear at what hour and season | game | none |
| catchable critter | a small animal that is a pickup | game, carries | none |
| ambient life | birds, fish, insects that are only scenery | game | none |

## 14. Atmosphere, light and sound

| Term | Meaning | Whose | Now |
|---|---|---|---|
| time of day | the sun's place; also gates content | engine, game | none |
| weather | rain, snow, storm; also changes ground and mechanics | engine, game, carries | none |
| weather zone, local climate | weather fixed or weighted per area | carries, game | none |
| temperature zone | an area that drains the player without protection | game, carries | none |
| sky, clouds, cloud shadows | the dome above, and its moving shade | engine | none |
| fog, haze, aerial perspective | air that is seen; far things fading to its colour | engine | partly |
| dynamic bounced light | bounced light computed live, with a moving sun | engine | none |
| baked light | light computed ahead; rare with a moving sun | engine, pack | none |
| light and reflection samples | probes placed in the world | loader | none |
| placed light | a lamp that is a light | lib, loader | shown |
| street lighting, lamp spacing | lamps placed for even night coverage | lib | partly |
| night readability | moon, stars and exposure keeping the dark world legible | engine | none |
| shadow method, shadow distance | how far and how sharply shadows are drawn | engine | none |
| rain shelter | where rain and snow do not reach | loader, engine | none |
| ambient particles | dust, pollen, leaves, embers, fireflies | loader, game | none |
| picture-change region | a region that changes the final image | game, loader | none |
| colour script, regional palette | the planned colour and mood of each region | game | none |
| art direction | the style everything shares | game | none |

## 15. Seeing far and running fast

| Term | Meaning | Whose | Now |
|---|---|---|---|
| instancing | drawing many copies of one mesh as one | loader | shown |
| batching | merging different things into fewer draws | loader, engine | none |
| merged far proxy | many far objects replaced by one simple mesh | engine, loader | none |
| far proxy build | the offline step that makes those | loader, engine | none |
| always-visible landmark | a far landmark kept beyond streaming range | loader, pack | partly |
| far terrain | simplified ground beyond streaming range | loader, engine | none |
| far lights, city glow | distant lights faked cheaply | loader, engine | none |
| automatic geometric detail | detail chosen per cluster, replacing hand-made copies | engine, pack | none |
| culling: view, occlusion, distance | not drawing what is not seen | engine | partly |
| cull distance, fade range | where a thing stops being drawn | pack, loader | partly |
| detail switch size | when each simpler copy takes over | pack, loader | none |
| pop-in | a thing appearing suddenly, from loading or a detail switch | engine, loader | none |
| overdraw | the cost of many see-through layers, chiefly growth | pack, engine | none |
| texture memory, blurry textures | what happens when textures exceed their budget | engine | none |
| shader stutter | a hitch when a material is first drawn | engine | none |
| interior streaming, rooms and portals | loading and hiding interiors apart from the street | loader, engine | none |
| teleport preload | world loaded and solid before the player lands | loader | none |
| region bundle | assets shipped per region and fetched when needed | pack, loader | none |
| quality tier, platform profile | the same world at different costs per machine | game, loader | none |
| budget | allowed instances, triangles, draws and memory | core, engine | partly |
| profiling | measuring frame time, memory and loads against budget | loader, engine | partly |

## 16. The player's map

| Term | Meaning | Whose | Now |
|---|---|---|---|
| minimap | the small map in the corner | core data, game look | shown |
| world map | the full map screen | core data, game look | partly |
| map marker, icon | what is shown on it | core | shown |
| place name, label | names of places, as translatable data | core | partly |
| contours, relief shading | how the map shows the shape of land | core | partly |
| map reveal region | the piece of map one unlock uncovers | core | none |
| layered map | a separate map per vertical layer | core | none |
| floor map | one map per storey indoors | core | partly |
| exploration progress | share of a region's finds already taken | core data, game | none |
| search area | a fuzzy region in place of an exact marker | game | none |
| detector, compass | a pointer to a goal or the nearest find | game, core data | none |

## 17. Persistence and a live world

| Term | Meaning | Whose | Now |
|---|---|---|---|
| stable ID | an identity that survives regeneration, so references and saves hold | core | partly |
| one-time versus respawning | whether a taken thing stays gone | game, carries | none |
| world state, phase | a region changed for good by story progress | core | partly |
| time-gated, weather-gated content | things present only at night or in rain | game, carries | none |
| event overlay | temporary dressing and content laid on a region | core | partly |
| live update, map expansion | shipping new or changed world to existing saves | core | none |
| save compatibility | a save still valid after the world is rebuilt | core, game | none |
| shared versus per-player state | which world facts co-op guests see and change | game, carries | none |

## 18. Making it: pipeline and people

| Term | Meaning | Whose | Now |
|---|---|---|---|
| source of truth | the one place a fact is edited | core | shown |
| hand-authored, procedural, hybrid | made by a person, by a rule, or both | core | partly |
| non-destructive | an edit that does not bake away what came before | core | partly |
| override, pin, lock | a hand decision that survives regeneration | core | none |
| seed, determinism | the same input giving the same world | core | shown |
| variant | the same world in another condition | core | shown |
| layer | a separately owned document of one world | core | shown |
| merge, conflict, file locking | two people's changes meeting; unmergeable files held by one | core | none |
| diff, blast radius | what a change touched | core | none |
| validation, lint | automatic checks on the world | core | shown |
| asset validation | checks on assets when saved or submitted | pack, loader | partly |
| review, capture | looking at a change before accepting it | core, loader | shown |
| automated fly-through, nightly build | a scripted flight that catches breakage | core, loader | none |
| headless build | running the build with no editor open | core, loader | shown |
| compile, bake, cook, package | make the world; precompute light and navigation; convert per platform; bundle | core, engine | partly |
| shared build cache | built results shared so not everyone rebuilds | core, engine | none |
| references, renames | how a moved or renamed asset keeps its users | engine, loader | none |
| unused assets, dependency size | finding what nothing uses and what costs most | pack | shown |
| editor-only versus shipped data | markup that helps authoring and must not ship | core, loader | none |
| editor extension | how our loader is delivered into each engine | loader | none |
| generator, step | user code that makes world | core | shown |
| placement graph | a node graph of placement rules, in the engine or a tool | engine, ? | none |
| geometry along a line | a mesh swept or lofted along a path | core | partly |
| real-world data | measured terrain and land cover used as input | core | partly |
| reference, mood board | the real places a region is modelled on | game | partly |
| import, migration | bringing an existing world in | ? | partly |

## 19. Seams: where two things meet

This is the reason the tool exists. Each row is a place where two things made apart must agree,
and the visible fault when they do not.

**Ground and water**

| Seam | The fault | Whose | Now |
|---|---|---|---|
| biome to biome | a straight edge between two lands | core | partly |
| ground material to ground material | a hard or smeared border | core, loader | partly |
| cliff into terrain | a hard line where rock mesh meets ground | lib | partly |
| cliff foot and cliff top | no scree below, grass hanging over the edge above | lib | partly |
| cave mesh to height field | a gap at the mouth | core | partly |
| surface to underground | a mouth where light, map and loading disagree | core | none |
| snow line | a level cut; snowy ground beside bare rocks and roofs | core, loader | none |
| wetness across things | wet ground with dry props, or rain under a roof | loader, engine | none |
| water at the shore | water through the ground, or a dry rim | core | shown |
| river in its bed | water hovering over, or drowned by, uncarved ground | lib | none |
| river along its course | water running uphill or tilted across the channel | lib | none |
| river meets river, lake or sea | a step in the surface | lib | none |
| waterfall to river | a fall joined to nothing above or below | lib | none |
| water to quay wall | water through the wall, or a gap below the waterline | lib | none |
| playable land to backdrop | a visible join or scale jump at the horizon | loader | none |
| world edge from height | gliding into an unseen wall, or seeing where the world ends | core, game | none |

**Ways**

| Seam | The fault | Whose | Now |
|---|---|---|---|
| road on terrain | a road floating, buried or tilted across a slope | core | shown |
| way edge to ground | a hard strip where road meets grass | lib, loader | partly |
| road meets road | a bump, a gap or a painted mess at the junction | lib | none |
| trail meets road | one surface cut straight across another | lib | partly |
| kerb to road | a gutter gap; kerb height wrong at driveways and ramps | lib | none |
| sidewalk corner at a junction | an unfilled or overlapping wedge | lib | none |
| markings across a junction | lane lines through the box, misaligned crossings | lib | none |
| lot to street | an entrance with no kerb cut | lib | none |
| lane graph to road surface | cars driving off the paint or through a median | lib, game | none |
| way crosses water | a road entering a river with no ford or bridge | lib | none |
| road meets bridge | a step or kink where the deck begins | lib | partly |
| bridge into bank and bed | piers and abutments floating, buried or in a lane below | lib | partly |
| elevated structure on a street | columns standing in lanes, crossings or doorways | lib | none |
| tunnel portal | a mouth not cut into the ground; light jumping | core, lib | none |
| furniture against access | a pole or bin blocking a door, driveway or crossing | lib | none |
| wire to world | a sagging wire through a tree, roof or hill | lib | none |
| fence on a slope | panels stepping with gaps, or posts leaning | lib | none |
| boundary across way or stream | a fence barring a road or hanging over water | lib | none |
| field to field, field to wild | crop rows cut off or spilling past the boundary | lib | partly |

**Buildings**

| Seam | The fault | Whose | Now |
|---|---|---|---|
| building on terrain | a corner in the air, a door below ground | core | shown |
| door to ground | a threshold with no step, or steps into the air | lib | none |
| building to sidewalk on a slope | exposed foundation, a stepped plinth missing | lib | partly |
| building beside building | gaps, overlaps or steps along a row | lib | partly |
| unequal neighbours | a taller building's blank side wall exposed | lib, pack | none |
| alley gap, building backs | a view between buildings to nothing | lib | none |
| projections clipping | awnings, signs and fire escapes through neighbours | lib | none |
| kit piece to kit piece | cracks, doubled faces, mouldings that do not line up | pack, lib | partly |
| kit piece lighting | neighbouring pieces lit differently along the join | pack | none |
| repeated facade or prop | the same piece visibly side by side | lib | none |
| district to district | style or street width changing on a hard line | lib | none |
| terrain under the city | ground poking through road, basement or tunnel | core | none |
| inside to outside | a view through a door that does not match; light jumping | core, lib | partly |
| window to interior | a faked room that is not the real one | lib, loader | none |
| instance door to world | entrance and interior differing in size or facing | core, lib | partly |
| floor to floor | stairs that do not land | lib | shown |
| seen from above | roofs and backs never meant to be seen | lib, pack | none |

**Growth and things**

| Seam | The fault | Whose | Now |
|---|---|---|---|
| growth against built things | a tree through a wall, grass on a road | core | shown |
| path through growth | a trail with no wear, or wear with no trail | core | shown |
| tree on a slope | roots hanging on the downhill side | lib, pack | none |
| object on sloped ground | a lamp leaning or hovering | core | partly |
| prop to ground contact | no dirt or wear where a thing sits | loader, pack | none |
| habitat to world | deer spawning in town, fish in a dry bed | core | none |

**Movement and play**

| Seam | The fault | Whose | Now |
|---|---|---|---|
| collision to what is drawn | walking through a wall, or stopped by nothing | pack, loader | none |
| walkable to what looks walkable | a slope that looks fine and cannot be climbed | lib | partly |
| climbable to what looks climbable | a wall that looks climbable and is not | pack, lib | none |
| roof to roof | a gap that looks jumpable and is not | lib, game | none |
| puzzle to terrain | a solution out of reach, or a link with no target | core, lib | none |
| enemy leash to arena | a boss dragged off its ground | lib, game | none |
| camera to tight space | the camera through cave walls and growth | game, lib | none |
| teleport arrival | landing inside geometry or through unloaded ground | core, loader | none |
| ground material to surface type | grass footsteps on what looks like gravel | core, loader | none |

**Loading and detail**

| Seam | The fault | Whose | Now |
|---|---|---|---|
| tile to tile | a crack or line at a tile border | core, loader | none |
| terrain detail to terrain detail | gaps between ground drawn at different detail | engine, loader | none |
| near detail to far detail | a jump when the simpler copy takes over | pack, loader, engine | none |
| far proxy to loaded cell | proxy and real thing both drawn, or neither | loader, engine | none |
| navigation across a tile border | paths that stop at an edge | loader | none |
| unloaded collision | falling through ground whose physics has not arrived | loader, engine | none |
| night lights across distance | window and sign glow vanishing or popping | loader, engine | none |
| precision far from origin | things shaking | engine | none |
| tool to engine | wrong scale, flipped axis or turned pivot after import | pack, loader | partly |
| decal on a surface | a marking that stretches or floats | loader | none |

**State and data**

| Seam | The fault | Whose | Now |
|---|---|---|---|
| generated to hand-made | a hand edit erased when the rule runs again | core | none |
| part to part | a ridge or trench where two maps join | core | shown |
| season and weather across layers | summer trees on winter ground | core | partly |
| time and weather to lighting | stored light wrong for the current hour | engine, loader | none |
| event overlay to base world | event props overlapping, or left behind after | core | none |
| save to world version | a saved flag for a chest that moved or vanished | core | none |
| map to world | a map that shows a road where there is none | core | shown |
| map icon to layer | an underground find drawn as if on the surface | core | none |
| host to guest | co-op players seeing different world states | game | none |

## 20. One thing, three names

The same thing is called differently by each engine, and several common words belong to an engine
we are not targeting. All from memory; check each before use.

| General term here | Unreal | Godot | Note |
|---|---|---|---|
| runtime cell | runtime cell, runtime grid (World Partition) | none built in | Godot has no built-in world streaming |
| placed level | Level Instance, Packed Level Actor | instanced scene | "sublevel" is the older Unreal word |
| switchable set | Data Layer | none built in | |
| world precision | Large World Coordinates | a double-precision build | origin shifting is the older way |
| terrain | Landscape | none built in; a plugin | |
| weight map | layer weightmap | a plugin's control map | "splat map" is Unity's word |
| terrain hole | landscape visibility mask | plugin-specific | |
| stamp | landscape brush or patch | plugin-specific | unsure of current names |
| mesh-to-terrain blend | Runtime Virtual Texture | none built in | |
| terrain texture streaming | Streaming Virtual Texture | none built in | |
| displacement | Nanite displacement | none | hardware tessellation is gone from Unreal 5 |
| assembly, saved group | Blueprint, Packed Level Actor | scene | "prefab" is Unity's word |
| instancing | instanced static mesh, hierarchical instanced | MultiMesh | |
| tile grid of pieces | none direct | GridMap with a MeshLibrary | |
| merged far proxy | HLOD | visibility ranges, no merging | |
| automatic geometric detail | Nanite | generated on import, simpler | |
| cull distance | cull distance | visibility range | |
| occluder | none authored | OccluderInstance3D | |
| navigation mesh | NavMesh, navigation invokers | NavigationRegion3D | |
| navigation link | Nav Link Proxy | NavigationLink3D | "off-mesh link" is Recast's and Unity's word |
| surface type | Physical Material with a surface type | custom metadata | Godot's physics material has no surface type |
| trigger, blocking, out-of-bounds region | Trigger Volume, Blocking Volume, Kill Z | Area3D, a static body | |
| picture-change region | Post Process Volume | WorldEnvironment; no regions | |
| sound region | Audio Volume | Area3D with an audio bus | |
| reflection sample | Reflection Capture | ReflectionProbe | "light probe" is Unity's word |
| dynamic bounced light | Lumen | SDFGI, VoxelGI | |
| placement graph | PCG framework | none built in | unsure of Unreal's status |
| crowd and traffic | Mass, ZoneGraph | none built in | unsure of status |
| water body | Water plugin's water bodies | none built in | unsure of status |
| scenario point | Smart Object | none built in | |
| free labels | Gameplay Tags | groups, metadata | |
| one file per placed thing | One File Per Actor | each scene is a text file | |
| asset rename tracking | redirectors | resource UIDs | |
| import cache | Derived Data Cache | the `.godot/imported` folder | |
| cook | cook | export | |
| editor extension | editor plugin, editor utility | EditorPlugin, GDExtension, `@tool` | |

## 21. Looks: the work

Sections 21 to 24 are about what makes a world look good. Most of it is nobody's to check: it is
craft and judgement. It is listed so the words are known, and so that where a rule or a measure
could help, we notice. Written by one author, not yet reviewed by others. Attributions are from
memory.

**Kind** is task (something people do), theory (something they rely on) or trick (a known move).
**Tool** says what a tool could do at most: nothing, carry (hold the intent as data), rule (place
by it), or measure (report a number, never a verdict).

| Term | Meaning | Kind | Tool |
|---|---|---|---|
| art direction | deciding what the world should look and feel like | task | nothing |
| style guide, art bible | the written rules of the look: shapes, palette, detail | task | carry |
| concept art, key art | pictures that set the target before anything is built | task | carry |
| paintover | painting over a screenshot to show what to change | task | nothing |
| reference gathering, location study | collecting real places the world is modelled on | task | carry |
| look development | settling how materials and light should read | task | nothing |
| environment art | making and placing the things a place is built from | task | rule |
| level art, art pass | turning a grey blockout into a finished place | task | rule |
| set dressing, dressing pass | placing props so a place looks used | task | rule |
| lighting pass | placing and tuning light for mood and guidance | task | carry |
| composition pass | adjusting a view so the eye lands where intended | task | measure |
| beauty pass, polish pass | the last round of small fixes that lifts a place | task | nothing |
| decal pass | laying stains, cracks and markings over surfaces | task | rule |
| terrain sculpting | shaping land by hand for a view or a route | task | rule |
| foliage painting | brushing growth onto land by eye | task | rule |
| biome design | deciding what a land is made of and how it differs from the next | task | carry |
| skyline design | shaping what is seen against the sky | task | measure |
| world building, in the story sense | deciding why places are the way they are | task | nothing |
| kitbashing | making new things by combining parts of existing ones | task | rule |
| playtest for readability | watching whether players see what they were meant to | task | nothing |
| review in context | judging a place from where the player stands, not from above | task | carry |

## 22. Looks: theories people rely on

| Term | Meaning | Kind | Tool |
|---|---|---|---|
| composition | arranging what is in view | theory | measure |
| focal point | the one thing a view is about | theory | carry |
| visual hierarchy | first, second and third things the eye should find | theory | carry |
| rule of thirds | placing the subject off centre on a three-by-three grid | theory | measure |
| leading lines | roads, rivers and edges that point the eye onward | theory | measure |
| framing | near things arranged round a far subject | theory | measure |
| foreground, midground, background | depth built from three layers | theory | measure |
| silhouette, read at a distance | a shape understood from its outline alone | theory | measure |
| the three reads | what a thing says from far, from mid and from near | theory | carry |
| big, medium, small | a mix of shape sizes, never all one size | theory | measure |
| shape language | round, square and pointed shapes carrying feeling | theory | nothing |
| rhythm, repetition with variation | repeating a thing, changed a little each time | theory | measure |
| contrast | difference in light, colour or detail that draws the eye | theory | measure |
| value structure, the squint test | the picture reduced to light and dark masses | theory | measure |
| detail and rest | busy areas set against plain ones | theory | measure |
| negative space | empty area that lets the rest be seen | theory | measure |
| balance, symmetry and asymmetry | visual weight spread across a view | theory | nothing |
| grouping, proximity | things read as one when they sit together | theory | rule |
| colour harmony | palettes built from related or opposed hues | theory | measure |
| warm and cool | temperature of colour separating light from shadow, near from far | theory | nothing |
| limited palette | few colours, used on purpose | theory | measure |
| colour script | the planned change of colour across a journey | theory | carry |
| atmospheric perspective | far things paler and bluer, giving depth | theory | nothing |
| scale cue | a door, a figure or a stair that tells how big things are | theory | rule |
| motivated light | every light having a visible source | theory | rule |
| light as guide | the eye and the feet going to the bright place | theory | carry |
| landmark, "weenie" | a tall, distinct thing that pulls the player; Disney's word for it | theory | carry |
| paths, edges, districts, nodes, landmarks | the five things that make a city legible; Kevin Lynch's | theory | carry |
| prospect and refuge | people like to see far from a sheltered spot; Appleton's | theory | measure |
| compression and release | a tight passage before a wide space makes it feel wider | theory | measure |
| reveal, denial and reward | hiding a goal, then showing it | theory | measure |
| borrowed scenery | composing a near place to include a far view | theory | measure |
| the triangle rule | hills as triangles that hide and then reveal what is behind | theory | measure |
| the forty-second rule | something of interest about that often along a ride | theory | measure |
| points of interest pulling like gravity | bigger sights drawing the player from further | theory | carry |
| affordance and signifier | a thing looking like what can be done with it | theory | carry |
| visual language | one meaning, one look, everywhere | theory | carry |
| readability, signal and noise | what matters standing out from what does not | theory | nothing |
| plausibility | a place that could work: where water runs, where people would build | theory | rule |
| lived-in, imperfection | wear and disorder as proof of use | theory | rule |
| a pattern language | recurring good arrangements of space; Alexander's | theory | carry |
| the picturesque | landscape composed like a painting | theory | nothing |

## 23. Looks: placement craft and tricks

| Term | Meaning | Kind | Tool |
|---|---|---|---|
| clustering in odd numbers | groups of three or five read as natural | trick | rule |
| hero and supporters | one large thing with smaller ones round it | trick | rule |
| random turn, scale and tint | small variation so copies do not read as copies | trick | rule |
| mirror and rotate | reusing a piece so it is not recognised | trick | rule |
| breaking up repetition | interrupting a run with a different piece | trick | rule |
| breaking a straight line | bending or interrupting anything too regular | trick | rule |
| hiding a seam with a prop | a rock, bush or crate placed where two things meet badly | trick | rule |
| transition piece | a piece made to join two kits or two grounds | trick | rule |
| grounding | dirt, shadow and debris where a thing meets the ground | trick | rule |
| growth at edges | plants gathering along walls, fences and water | trick | rule |
| growth by cause | moss on the wet side, grass worn where feet go | trick | rule |
| debris by cause | leaves in corners, scree below cliffs, litter by bins | trick | rule |
| wear by use | paths, doors and handles showing traffic | trick | rule |
| prop storytelling | a few things arranged to say what happened here | trick | carry |
| dressing to the camera | detail put where the player looks, saved where they do not | trick | measure |
| detail falloff | less detail further from the route | trick | rule |
| blocking a sightline | a wall or hill placed to hide what is not ready to be seen | trick | measure |
| framing a reveal | an arch, gap or bend placed before a view | trick | measure |
| breadcrumbs | small things laid in a line toward a goal | trick | rule |
| pool of light | a lit spot that draws the player to a door or path | trick | rule |
| colour or mark as a signal | a painted ledge or a coloured cloth meaning "climb here" | trick | rule |
| contrast at the focal point | the strongest contrast put where the eye should land | trick | measure |
| a clear silhouette against sky | placing a landmark so nothing merges with its outline | trick | measure |
| forced perspective | far things built smaller so they seem further | trick | nothing |
| exaggerated scale | doors, cliffs and towers larger than life | trick | nothing |
| vertical exaggeration | terrain made steeper than real land to read as dramatic | trick | measure |
| layered horizon | ridges stacked back so depth reads | trick | measure |
| leaving room | open ground kept empty so the rest can be seen | trick | measure |
| asymmetry | avoiding the centred and evenly spaced | trick | measure |
| a regional signature | one plant, rock or colour that says where you are | trick | carry |

## 24. Looks: surface and finish tricks

| Term | Meaning | Kind | Tool |
|---|---|---|---|
| weathering | rain streaks, rust and fade laid where water and sun reach | trick | rule |
| edge wear | worn corners and rims | trick | nothing |
| dirt gathering | grime in corners, at bases, under ledges | trick | rule |
| macro and micro variation | change at the scale of a field and at the scale of a hand | trick | nothing |
| breaking up tiling | hiding a repeated texture with variation or decals | trick | rule |
| decal layering | stains, leaks and cracks built up in passes | trick | rule |
| vertex blending | painting one material into another across a mesh | trick | rule |
| roughness variation | a surface shiny in some places and dull in others | trick | nothing |
| wet-down | wetting streets so they catch light | trick | carry |
| specular breakup | interrupting large even highlights | trick | nothing |
| contact shadow, ambient occlusion | darkness where things touch, which seats them | trick | nothing |
| rim light | light from behind that separates a shape from its background | trick | nothing |
| fog card, light shaft | cheap pieces of air and light placed by hand | trick | rule |
| emissive accents | small glowing things that mark life at night | trick | rule |
| matte painting, backdrop card | a painted far view | trick | nothing |
| fake depth | a shader or a card giving depth where there is none | trick | nothing |
| colour grading | the final tint of the whole picture | task | nothing |
| stylisation | simplifying detail into larger, clearer shapes | theory | nothing |
| hand-painted, toon, realistic | named looks with their own rules | theory | carry |
| material palette | the short list of surfaces a place is allowed | theory | measure |

## 25. This project's own words

Words the spike coined or bent, with the nearest word a practitioner would use. If we keep one of
ours, it should be because no existing word fits.

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

## 26. What the ledger shows

Counted on 2026-11-12, over sections 1 to 19 (541 terms) and 21 to 24 (113 terms).

**Where the spike stands.** Of 541 world terms: 81 shown, 109 partly, 351 none. Most of the
"none" rows are the engine's or the game's. The rows that are ours and still none are thickest in
three places: ways (39), seams (57) and asset facts (23).

**What keeps recurring across groups.** These are the design questions the list points at, each
appearing under several headings:

- *More than one surface.* Caves, interiors, decks, tunnels, stacked map layers, floating
  islands, basements, rooftops, underground concourses. One model is needed; the spike has three.
- *Identity that survives.* Stable IDs, saves, overrides, hand edits, map reveal, quest hooks and
  live updates all rest on a thing keeping its name when the world is rebuilt.
- *Who gives way, and in what order.* Authority, sequence, blends between materials, biomes and
  districts, and every seam in section 19.
- *Lanes, not roads.* Traffic, markings, signals, junctions, kerb cuts and parking all hang off
  lane-level data the spike does not have.
- *Causes, not placements.* Scree below a cliff, reeds by water, wear where feet go, dirt in
  corners: the wilderness reader and the looks sections ask for the same thing from two sides.
- *Movers, not a walker.* Reach, clearance and navigation differ for a person, a climber, a
  horse, a car and a glider.
- *An asset is a set with facts.* Files, simpler copies, collision, surface type, facing, pivot,
  module: none standardised, all needed, and now arriving uneven from generative tools.

**What the looks sections add.** Of 113 terms about looks, a tool could do nothing for 24, carry
intent for 21, place by a rule for 36 and report a measure for 32. So about three quarters are
reachable in some form, and the largest share is placement by rule: clustering, grounding, growth
and debris by cause, hiding a seam with a prop, breaking repetition. That is the part of "the rest
still looks right" that is ours to attempt. The measures (sightlines, silhouette, spacing of
sights, share of empty ground) can inform a person and should never pass or fail a world.

**What to leave alone on purpose.** Rendering, weather, light, sound, crowds, traffic behaviour,
animal behaviour, combat, saves and co-op are the engine's or the game's. For each, the most we do
is carry the data that says where and what, in a form their systems can read.

**What this list is not.** It is a dictionary, not a plan. A row marked "lib" is a guess that the
term belongs to a domain library, not a decision to write one.
