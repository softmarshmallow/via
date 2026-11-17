# Movement, play and a living world

Sections 11 to 13 and 17 of the [ledger](README.md). The row shape and the marks are explained
in the README; the archetypes in [archetypes.md](archetypes.md).

## 11. Movement, traversal and collision

From general knowledge (2026-11-12), re-read against the first cut on 2026-11-18.

| Term | Meaning | Archetype | Whose | Now |
|---|---|---|---|---|
| collision | what stops a body | Asset | pack, loader | partly: `collision`, `engines/godot` |
| walkable slope, step height | what a character can walk up | Measure | game, core | shown: `lib/review/reach.py` |
| traversal metrics, traversal envelope | jump gap, climb height, glide ratio, speed: the numbers levels are built to, deciding which terrain is a wall and which a route | Guidance | game, core | partly: `lib/review/reach.py` |
| climb, mantle, vault | getting up and over | Runtime | game | none |
| stamina-gated route, rest ledge | a route reachable only with enough stamina | Guidance | game, lib | none |
| glide, updraft, wind current | air that carries a glider | Markup | game, carries | none |
| launcher, grapple point, zipline | a placed device that throws or pulls the player | Markup | game, carries | none |
| moving platform, mechanism | walkable geometry that moves | Runtime | game, loader | none |
| swim, dive, mount, drive | other ways of moving | Runtime | game | none |
| mount needs: turning room, jumpable obstacle | what a ridden animal needs to pass | Measure | game, lib | none |
| driveable surface, kerb mount height | what a vehicle can drive on and over | Measure | game, pack | none |
| fall height, slide slope | drops and slopes that hurt or carry the player | Measure | game, core | none |
| reachability | whether a given mover can get from here to there | Measure | lib | shown: `lib/review/reach.py` |
| sequence break, shortcut | reaching a place earlier than designed | Measure | game, lib | none |
| soft lock, stuck spot | a place the player can enter and not leave | Measure | lib | none |
| clearance, headroom | free height and width a body or vehicle needs, kept above and beside a route passing beneath | Measure | pack, lib | partly: `lib/building/check.py` |
| navigation mesh | where an agent can walk, as a mesh derived from the ground | Derivation (focal) | engine, loader | none |
| navigation agent | the body size a navigation mesh is built for | Runtime | game, loader | none |
| agent class | separate walkability for person, horse, wagon, car | Measure | game, loader | partly: `lib/review/reach.py` |
| navigation area, cost | ground tagged as sidewalk, road, forbidden | Markup | lib, loader | none |
| navigation link | a jump, ladder or door joining walkable areas | Meeting | lib, game | partly: `lib/review/reach.py` |
| navigation built ahead or live | computed at build, or around agents at run time | Runtime | engine, loader | none |
| navigation across cells | navigation data that loads with its cell | Delivery | loader | none |
| blocking region | a stop that is not drawn | Markup | game, loader | none |
| out of bounds, fall-out floor | where the player is returned or dies | Markup | game | none |
| teleport point, fast travel | a place the player may jump to, often unlocked; their spacing sets how far anyone walks | Markup | game, carries | none |
| teleport arrival spot | exact position and facing after a teleport | Markup | core | none |

## 12. Gameplay markup

From general knowledge (2026-11-12), re-read against the first cut on 2026-11-18.

| Term | Meaning | Archetype | Whose | Now |
|---|---|---|---|---|
| tags, groups | free labels game code finds things by | Markup | core, game | shown: `tags` |
| spawn point, spawn zone | where things appear; chosen out of the player's sight | Markup | game, carries | shown: `map.spawn_zone` |
| respawn rule, refresh timer | when a taken thing or beaten enemy returns | Runtime | game | none |
| enemy camp | a physical place enemies hold | Recipe | game, lib | none |
| encounter | a designed fight and its ground | Markup | game | none |
| boss arena | a fixed strong enemy and its shaped ground | Recipe | game, lib | none |
| leash, aggro range | how far an enemy chases before resetting | Runtime | game | none |
| difficulty zone, level band | how strong foes are per area | Markup | game, carries | none |
| patrol path | a route an agent follows | Markup | game, carries | none |
| trigger region | a region that starts something | Markup | game, carries | partly: `build.md 9` |
| quest marker, objective | where the story points | Markup | game | none |
| checkpoint, save point | where progress is kept | Markup | game | none |
| chest, loot, reward tier | things to find, matched to effort | Markup | game, carries | partly: `build.md 9` |
| collectible | counted finds scattered to reward exploring | Markup | game, carries | none |
| puzzle element | plate, torch, switch: placed parts the player must solve | Markup | game, carries | none |
| puzzle link, wiring | which switch opens which gate | Markup | carries | none |
| seal, barrier, lock | a block removed by a condition | Markup | game, carries | none |
| shrine, mini-dungeon entrance | a world door to a small instanced challenge | Markup | game, carries | none |
| time trial, open-world challenge | a marker that starts a timed task | Markup | game, carries | none |
| guide object | a thing that leads the player along a route | Markup | game, carries | none |
| station: campfire, forge, cooking pot | a placed spot for a crafting activity | Markup | game, carries | none |
| shop, safehouse, service location | an enterable place with a game function | Markup | game | none |
| interactable | something the player can use | Markup | game, carries | shown: `build.md 9`, `examples/station` |
| scenario point, smart object | a spot or thing telling a character what can be done there and where to stand | Markup | game, carries | none |
| world event, random encounter | something staged where the player happens to be | Runtime | game | none |
| emergency response, dispatch | services sent to an incident | Runtime | game | none |
| safe zone | a region with other rules | Markup | game, carries | none |
| cover point, vantage | positions tactical agents use | Markup | game | none |
| concealment, stealth grass | growth or shadow that hides the player | Markup | game, carries | none |
| tracks, clues | marks the player follows to a target | Markup | game, carries | none |
| golden path, critical path, breadcrumb, leading line | the route design pulls the player along | Guidance | game, lib | none |
| POI density, content spacing, reward density | how often something new comes into view: things to find per area, and how far apart | Guidance | game, lib | none |
| secret, hidden nook | a place meant to be found only by looking | Guidance | game | none |
| lookout, viewpoint | a spot giving a survey view | Markup | game, carries | partly: `review.bookmark` |
| beat, pacing | the planned rhythm along a route | Guidance | game | none |
| camera region, camera hint | a region or mark that steers the camera | Markup | game, loader | none |
| music region, soundscape | what a place sounds like: the sound of unseen life chosen by place, time and weather | Markup | game, carries | none |
| sound emitter, reverb region | a placed sound, or a region that changes sound | Markup | game, loader | none |

## 13. A living world

From general knowledge (2026-11-12), re-read against the first cut on 2026-11-18.

| Term | Meaning | Archetype | Whose | Now |
|---|---|---|---|---|
| NPC | a character who is not the player | Runtime | game | none |
| crowd, pedestrians | many characters, simulated cheaply | Runtime | game, engine | none |
| pedestrian graph, crossing link | where walkers go and where they may cross | Network (undirected) | lib, game | none |
| traffic | vehicles and riders driven by the game | Runtime | game | none |
| traffic lane graph | the lanes traffic follows: directed, with crossings and right of way | Network (directed) | game, lib | none |
| parked vehicle, parking slot | where stationary vehicles are placed or spawned | Markup | game, lib | none |
| density by district and hour | how full the streets are, where and when: how many of which kind an area holds | Field | game, carries | none |
| spawn and despawn radius | ambient agents existing only near the player; nothing exists beyond that bubble | Runtime | game, engine | none |
| archetype mix | which people and vehicles appear in which district | Recipe | game, pack | none |
| schedule, routine | where a character is at what hour: home, work, tavern | Runtime | game | none |
| fauna, wildlife | animals as part of the game; no one word is standard | Runtime | game | none |
| habitat, range | where a species may be found, taken from biome, water and cover | Area partition | game, carries | partly: `map.spawn_zone` |
| herd, flock, pack; predator and prey | animals as groups, and who hunts whom | Runtime | game | none |
| den, roost, watering hole | fixed places animals use | Markup | game, carries | none |
| fishing spot | water marked for what can be caught | Markup | game, carries | none |
| livestock, draft animals | owned animals, and non-motor traffic | Runtime | game | none |
| activity pattern, migration | which animals appear at what hour and season | Runtime | game | none |
| catchable critter | a small animal that is a pickup | Markup | game, carries | none |
| ambient life | birds, fish, insects that are only scenery | Runtime | game | none |

### Ambient life

Everything that moves and lives without the player's involvement: people, traffic, animals,
small creatures. Almost always an illusion kept up near the player, resting on markup the world
provides. Read in 2026-11; a row marked `(?)` is from memory.

**Levels of ambition, cheapest first**

| Term | Meaning | Archetype | Whose | Now |
|---|---|---|---|---|
| posed life | a character fixed at a spot, repeating one activity | Placement | game, carries | none |
| persistent simulation | every inhabitant exists and acts all the time, seen or not | Runtime | game | none |

**What the world must provide**

| Term | Meaning | Archetype | Whose | Now |
|---|---|---|---|---|
| activity spot | a marked position and facing where an activity happens | Markup | game, carries | none |
| building role | what a building is for an inhabitant: home, workplace, shop, shelter | Markup | game, carries | none |

**How movement is produced**

| Term | Meaning | Archetype | Whose | Now |
|---|---|---|---|---|
| steering | each agent blends simple urges: seek, avoid, follow, wander | Runtime | game | none |
| flocking | group motion from three rules per member: keep apart, align, stay together (Reynolds 1987) | Runtime | game | none |
| collision avoidance | agents adjust speed and heading to pass without touching | Runtime | game | none |
| flow field | a direction at every location that a whole crowd follows towards a goal | Derivation (global) | game, lib | none |
| flee radius | the distance at which a creature reacts to an approach | Runtime | game | none |

**How the illusion is kept cheap**

| Term | Meaning | Archetype | Whose | Now |
|---|---|---|---|---|
| simulation level of detail | full behaviour near, simplified in the middle distance, only counts far away | Runtime | game, engine | none |
| recycling | an inhabitant leaving view is reused as a new one entering it | Runtime | game | none |
| alibi | a purpose and history invented for a background character only when the player starts watching it (Sunshine-Hill and Badler 2010) | Runtime | game | none |

## 17. Persistence and a live world

From general knowledge (2026-11-12), re-read against the first cut on 2026-11-18.

| Term | Meaning | Archetype | Whose | Now |
|---|---|---|---|---|
| stable ID | an identity that survives regeneration, so references and saves hold | Principle | core | shown: `identity.md 2` |
| one-time versus respawning | whether a taken thing stays gone | Markup | game, carries | none |
| world state, phase | a region changed for good by story progress: water lowered, a storm cleared, a ruin raised | Overlay | core | partly: `[change.<name>]` |
| time-gated, weather-gated content | things present only at night or in rain | Markup | game, carries | none |
| event overlay | temporary dressing and content laid on a region | Overlay | core | shown: `[change.<name>]`, `documents.md 6` |
| live update, map expansion | shipping new or changed world to existing saves | Delivery | core | none |
| save compatibility | a save still valid after the world is rebuilt | Principle | core, game | none |
| shared versus per-player state | which world facts co-op guests see and change | Runtime | game, carries | none |
