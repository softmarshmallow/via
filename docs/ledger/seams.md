# Seams

Section 19 of the [ledger](README.md). The row shape and the marks are explained in the README;
the archetypes in [archetypes.md](archetypes.md).

## 19. Seams: where two things meet

This is the reason the tool exists. Each row is a place where two things made apart must agree,
and the visible fault when they do not.

From general knowledge (2026-11-12), re-read against the first cut on 2026-11-18. A seam is
filed by what the fault is about: a place where two forms join is a Meeting, an agreement
between what is drawn and what is played is a Measure, a join between delivered pieces is
Delivery.

**Ground and water**

| Seam | The fault | Archetype | Whose | Now |
|---|---|---|---|---|
| biome to biome | a straight edge between two lands | Meeting | core | none |
| ground material to ground material | a hard or smeared border | Meeting | core, loader | partly: `lib/land/ground.md` |
| cliff into terrain | a hard line where rock mesh meets ground | Meeting | lib | partly: `land.sheer_cliff` |
| cliff foot and cliff top | no scree below, grass hanging over the edge above | Meeting | lib | partly: `examples/example` |
| cave mesh to height field | a gap at the mouth | Meeting | core | partly: `land.cave`, `cut` |
| surface to underground | a mouth where light, map and loading disagree | Meeting | core | none |
| snow line | a level cut; snowy ground beside bare rocks and roofs | Meeting | core, loader | none |
| wetness across things | wet ground with dry props, or rain under a roof | Meeting | loader, engine | none |
| water at the shore | water through the ground, or a dry rim | Meeting | core | shown: `where`, `lib/water/fill.py` |
| river in its bed | water hovering over, or drowned by, uncarved ground | Meeting | lib | shown: `water.beck` |
| river along its course | water running uphill or tilted across the channel | Meeting | lib | partly: `terrain.terrain_river` |
| river meets river, lake or sea | a step in the surface | Meeting | lib | partly: `examples/coast` |
| waterfall to river | a fall joined to nothing above or below | Meeting | lib | partly: `water.weir` |
| water to quay wall | water through the wall, or a gap below the waterline | Meeting | lib | partly: `town.waterfront` |
| playable land to backdrop | a visible join or scale jump at the horizon | Delivery | loader | none |
| world edge from height | gliding into an unseen wall, or seeing where the world ends | Meeting | core, game | none |

**Ways**

| Seam | The fault | Archetype | Whose | Now |
|---|---|---|---|---|
| road on terrain | a road floating, buried or tilted across a slope | Meeting | core | shown: `ways.road`, `[kind.<k>.effect.<id>]` |
| way edge to ground | a hard strip where road meets grass | Meeting | lib, loader | partly: `ways.road` |
| road meets road | a bump, a gap or a painted mess at the junction | Meeting | lib | none |
| trail meets road | one surface cut straight across another | Meeting | lib | partly: `ways.trail` |
| kerb to road | a gutter gap; kerb height wrong at driveways and ramps | Meeting | lib | none |
| sidewalk corner at a junction | an unfilled or overlapping wedge | Meeting | lib | none |
| markings across a junction | lane lines through the box, misaligned crossings | Meeting | lib | none |
| lot to street | an entrance with no kerb cut | Meeting | lib | none |
| lane graph to road surface | cars driving off the paint or through a median | Measure | lib, game | none |
| way crosses water | a road entering a river with no ford or bridge | Meeting | lib | shown: `ways.ford`, `ways.crossing` |
| road meets bridge | a step or kink where the deck begins | Meeting | lib | shown: `ways.crossing` |
| bridge into bank and bed | piers and abutments floating, buried or in a lane below | Meeting | lib | partly: `ways.crossing` |
| elevated structure on a street | columns standing in lanes, crossings or doorways | Meeting | lib | none |
| tunnel portal | a mouth not cut into the ground; light jumping | Meeting | core, lib | none |
| furniture against access | a pole or bin blocking a door, driveway or crossing | Measure | lib | none |
| wire to world | a sagging wire through a tree, roof or hill | Measure | lib | none |
| fence on a slope | panels stepping with gaps, or posts leaning | Meeting | lib | none |
| boundary across way or stream | a fence barring a road or hanging over water | Meeting | lib | partly: `town.gate` |
| field to field, field to wild | crop rows cut off or spilling past the boundary | Meeting | lib | none |

**Buildings**

| Seam | The fault | Archetype | Whose | Now |
|---|---|---|---|---|
| building on terrain | a corner in the air, a door below ground | Meeting | core | shown: `land.pad`, `ground` |
| door to ground | a threshold with no step, or steps into the air | Meeting | lib | partly: `examples/college` |
| building to sidewalk on a slope | exposed foundation, a stepped plinth missing | Meeting | lib | partly: `examples/market` |
| building beside building | gaps, overlaps or steps along a row | Meeting | lib | partly: `lib/town/kit.py` |
| unequal neighbours | a taller building's blank side wall exposed | Meeting | lib, pack | none |
| alley gap, building backs | a view between buildings to nothing | Meeting | lib | none |
| projections clipping | awnings, signs and fire escapes through neighbours | Measure | lib | none |
| kit piece to kit piece | cracks, doubled faces, mouldings that do not line up | Meeting | pack, lib | partly: `lib/town/kit.py` |
| kit piece lighting | neighbouring pieces lit differently along the join | Runtime | pack | none |
| repeated facade or prop | the same piece visibly side by side | Measure | lib | none |
| district to district | style or street width changing on a hard line | Meeting | lib | none |
| terrain under the city | ground poking through road, basement or tunnel | Meeting | core | partly: `cut` |
| inside to outside | a view through a door that does not match; light jumping | Meeting | core, lib | partly: `building.doorway` |
| window to interior | a faked room that is not the real one | Meeting | lib, loader | none |
| instance door to world | entrance and interior differing in size or facing | Meeting | core, lib | partly: `composition.md 13` |
| floor to floor | stairs that do not land | Meeting | lib | shown: `building.flight` |
| seen from above | roofs and backs never meant to be seen | Measure | lib, pack | none |

**Growth and things**

| Seam | The fault | Archetype | Whose | Now |
|---|---|---|---|---|
| growth against built things | a tree through a wall, grass on a road | Meeting | core | shown: `rules.md 7` |
| path through growth | a trail with no wear, or wear with no trail | Meeting | core | shown: `ways.trail` |
| tree on a slope | roots hanging on the downhill side | Meeting | lib, pack | partly: `sink` |
| object on sloped ground | a lamp leaning or hovering | Meeting | core | shown: `rules.md 6` |
| prop to ground contact | no dirt or wear where a thing sits | Meeting | loader, pack | none |
| habitat to world | deer spawning in town, fish in a dry bed | Measure | core | none |

**Movement and play**

| Seam | The fault | Archetype | Whose | Now |
|---|---|---|---|---|
| collision to what is drawn | walking through a wall, or stopped by nothing | Measure | pack, loader | none |
| walkable to what looks walkable | a slope that looks fine and cannot be climbed | Measure | lib | partly: `lib/review/reach.py` |
| climbable to what looks climbable | a wall that looks climbable and is not | Measure | pack, lib | none |
| roof to roof | a gap that looks jumpable and is not | Measure | lib, game | none |
| puzzle to terrain | a solution out of reach, or a link with no target | Measure | core, lib | none |
| enemy leash to arena | a boss dragged off its ground | Measure | lib, game | none |
| camera to tight space | the camera through cave walls and growth | Measure | game, lib | none |
| teleport arrival | landing inside geometry or through unloaded ground | Measure | core, loader | none |
| ground material to surface type | grass footsteps on what looks like gravel | Measure | core, loader | none |

**Loading and detail**

| Seam | The fault | Archetype | Whose | Now |
|---|---|---|---|---|
| tile to tile | a crack or line at a tile border | Delivery | core, loader | partly: `build.md 5` |
| terrain detail to terrain detail | gaps between ground drawn at different detail | Delivery | engine, loader | none |
| near detail to far detail | a jump when the simpler copy takes over | Delivery | pack, loader, engine | none |
| far proxy to loaded cell | proxy and real thing both drawn, or neither | Delivery | loader, engine | none |
| navigation across a tile border | paths that stop at an edge | Delivery | loader | none |
| unloaded collision | falling through ground whose physics has not arrived | Runtime | loader, engine | none |
| night lights across distance | window and sign glow vanishing or popping | Delivery | loader, engine | none |
| precision far from origin | things shaking | Runtime | engine | none |
| tool to engine | wrong scale, flipped axis or turned pivot after import | Asset | pack, loader | shown: `assets.md 3` |
| decal on a surface | a marking that stretches or floats | Meeting | loader | none |

**State and data**

| Seam | The fault | Archetype | Whose | Now |
|---|---|---|---|---|
| generated to hand-made | a hand edit erased when the rule runs again | Principle | core | shown: `identity.md 4` |
| part to part | a ridge or trench where two maps join | Meeting | core | shown: `composition.md 6` |
| season and weather across layers | summer trees on winter ground | Overlay | core | partly: `examples/vale` |
| time and weather to lighting | stored light wrong for the current hour | Runtime | engine, loader | none |
| event overlay to base world | event props overlapping, or left behind after | Overlay | core | partly: `documents.md 6` |
| save to world version | a saved flag for a chest that moved or vanished | Principle | core | none |
| map to world | a map that shows a road where there is none | Measure | core | shown: `lib/map/export.py` |
| map icon to layer | an underground find drawn as if on the surface | Measure | core | partly: `lib/map/export.py` |
| host to guest | co-op players seeing different world states | Runtime | game | none |
