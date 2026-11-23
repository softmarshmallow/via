# The kit contract

What a modular kit must state for our libraries to lay it, who reads each fact, and what checks
it. Written at K2 of `docs/plan-kits.md`, from `examples/kit` and the studios' accounts in
`docs/research/Modular kit failures in studios.md`. This page describes the libraries as they
are; where it says "not yet", the plan names the milestone.

The lesson behind the page: studios did not suffer from placing pieces. They suffered from
contracts that were unwritten, unreadable or unchecked (the report's conclusion). A kit here is
text and measured facts, so each line of the contract below is either stated in the pack or
measured from the models, and each has a check.

## What a kit is

A kit is the pieces under one taxon of a pack (`spec/assets.md` 5). A world names the taxon
(`kit = "structure/plain"`), never a piece. A piece is a pack asset with:

| Stated or measured | Where | Meaning |
|---|---|---|
| `role:<word>` | a tag | The part the piece plays. One piece per role is read: a piece with no `variant:` tag before one with, then the first id, bytewise. |
| `storey:<word>` | a tag | The kind of storey the piece is made for: `ground`, `upper` or `top`. It is laid there only, and read there before an untagged piece of its role. `storey:parapet` marks the wall round a flat roof: a bay, and a filler and a corner where the kit has them, laid on the roof as a storey's walls are (K6). `storey:base` marks the wall under the ground storey's floor, laid the same way on that floor, each piece sunk by its own height in the pack (`sink`) so that its top is the floor (K7). `storey:colonnade` marks the pieces that stand along a building's outline outside its walls: a `role:post` and the `role:wall` piece that spans from one post to the next, a lintel, placed by the floor under it (`pivot = "origin"`) with its box at the storey's top (K8). |
| `params.span` | the pack, on a roof piece | The plan distance from the eave to the ridge the piece is made for: half the depth of the building it roofs. |
| `role:post` | a tag | The kit, or the group of its pieces that carry one `storey:` tag, is laid by posts (lay.py rule 15): the bay's `params.pitch` is then from one post's middle to the next, the `role:wall` piece is a panel that long less the post's measured width, a `role:door` piece the same, and a corner is a post. There are no fillers. |
| `variant:<word>` | a tag, beside `role:wall` | A variant of the bay: a piece that may stand where a bay stands. The bay is the first `role:wall` piece with no such tag. |
| `params.pitch` | the pack | The length of wall the piece takes, metres. Without it, the measured width. |
| `params.reach` | the pack, on a corner | The length of each wall a corner takes, from the outline's vertex. |
| `bounds`, `footprint` | measured (`via pack accept`) | The model's box and the ground it claims. |
| `pivot` | the pack, once per kit | The rule that puts the placing point: `base_centre` unless a piece says otherwise. A model's own origin moves nothing (`examples/kit`, `pivot`). |
| `sink` | the pack | How far a piece reaches below its placing point: a plinth. |

A piece lies along its x axis with its front on +y. A wall piece is as thick as the kit's bay.

## Roles

| Role | Read by | The piece |
|---|---|---|
| `wall` | both | The bay: the piece a wall or a row is made of. Required. lib/building also reads the bay's variants; lib/town reads the bay alone. |
| `filler` | both | A shorter piece for what whole bays leave. It is a part of the module (a half bay), not a patch. |
| `corner` | both | Where a line turns. In a building, the outside corner. |
| `corner_in` | lib/building | The inside corner of a building's outline. Placed by the middle of its notch (`pivot = "origin"`). |
| `door` | lib/building | What stands in a doorway: a frame or a leaf. Its width is the kit's door size. |
| `roof` | lib/building | One piece for a whole roof, where a building asks for none by name. |
| `floor` | lib/building | A square tile, one module on a side, sunk by its thickness. It tiles each storey's floor inside the walls, and a flat roof where the kit has no `roof_flat`. |
| `roof_flat` | lib/building | A flat roof's tile, square like the floor's, standing on the walls' top unsunk: a kit's roof is not its floor (K10, Kenney's tile stands level with its border's flat part, and the kit has no floor). |
| `roof_slope` | lib/building | A wedge from the eave to the ridge, `params.pitch` along the eave and `params.span` deep. A kit may hold several, one for each depth of building it roofs: the one whose span is half the roof's depth is taken, and the gable, hip and valley of that span with it (K8). |
| `roof_gable` | lib/building | Half a gable: a wall one span long, as thick as the bay, rising to the ridge. Two stand at each end of a gable roof, the second turned about. |
| `roof_hip` | lib/building | The roof over an outer corner: one span square, eaves on its front and its +x side, climbing to the hip between them. One on the outer corner of an L under a gable (K7); one on each corner of a rectangle under a hipped roof, `roof = "hip"` (K8). |
| `roof_valley` | lib/building | The roof at the inside corner of an L, placed by the corner itself (`pivot = "origin"`): the span-square quarter where the two inner slopes meet, and `params.reach` of each inner eave beyond the corner, with the footprint it covers declared (K7). |
| `end` | lib/town | The piece at a row's free end and either side of a gap. |

## The kit as a whole

Some facts belong to no one piece. Today each reader works them out from the pieces, and
lib/building's `check` holds the kit to them (`lib/building/check.py`, rule 9):

| Fact | Worked out as | Held by |
|---|---|---|
| The module | The pitch of the smallest wall piece: the filler's, else the bay's. | `kit_module`: the bay and the door piece are whole numbers of it. |
| The wall's thickness | The bay's measured depth. | `kit_uneven`: wall, filler and door are one thickness. |
| The storey | The pieces' measured height. A storey takes, of each role, the pieces nearest its own height, so one kit may hold a 3 m and a 4 m bay. | `kit_uneven`: the pieces one storey reads are one height; `kit_height`: each is the height of the storey it serves. |
| One wall thickness for all storeys | The bays' measured depth. | `kit_uneven`; `lay` refuses a building whose storeys' bays differ (`params`). |
| The floor's module | The floor tile's pitch. | `kit_deck`: the tile is square, a whole number of the module. `slab_open`, from `lay`: a floor or a stairwell that is not whole tiles. |
| The roof's depth | The slope piece's span, twice which is the building's depth. | `kit_deck`: slope and gable agree in span and rise, the gable is the wall's thickness. `roof_none`, from `lay`: a building the roof does not fit, with the reason. |
| The door size | The door piece's width, less the margin `lay` leaves (0.1 m). | `gap_wide`, from `lay`: a door asked wider. |
| The corners' reach | `params.reach`: the wall's thickness outside and nothing inside, unless said. | `kit_corner`: the model is the plan its reach states. `kit_ell`: the two reaches add up to whole modules, or no plan with an inside corner closes. |
| No patch piece | A filler is not shorter than the wall is thick. | `kit_slip`. |
| A variant is the bay's size | Its stated pitch and its measured box (length, thickness, height, place about the origin) within 5 mm of the bay's. | `kit_variant`; `variant_none`, from `lay`, where a building asks for one that is not, or that the kit has not. |

That these are worked out twice, and stated nowhere once, is the gap the research calls "the kit
as a unit". Ruled at K5 (R66 in `docs/doctrine.md`): a kit is no archetype of its own, but assets
filed together as a kit must state once what they share, and a rule must lay by that. Since K6
the pack's taxon carries those facts (`defaults.shared`, the next section); this table is how a
reader works a fact out where the kit does not state it, and what `check` holds a stated one
against.

## What a kit states

A kit is a taxon's pieces, and what they share is written once, in that taxon's `defaults.shared`
(`spec/assets.md` 2.5; R66; Asset law 5 of `docs/ledger/archetypes.md`). The core reads it for
nothing and hands it to a step with each piece (`pack.query`, column `shared`). No asset may
write its own.

```toml
[taxon."structure/plain".defaults.shared]
module = 3.0                     # the bay
wall   = 0.3                     # every wall piece's thickness
storey = [3.0, 4.0]              # the wall pieces' heights
door   = 1.4                     # the doorway the door piece is made for

[taxon."structure/plain".defaults.shared.stair]
lane    = 1.5                    # how deep one lane of the stairwell is
width   = 1.3
run     = 4.5
landing = 1.5
```

| Key | What lib/building does with it |
|---|---|
| `wall` | It is the wall's thickness in every rule, in place of the bay's measured box. |
| `stair` | `width` and `run` are a building's stair's where it writes none; `landing` and `lane` size the stairwell. A lane, a landing and a run of whole floor tiles give stairwells of whole tiles on every storey. |
| `door` | A door that writes no `width` takes it. |
| `unit` | Not laid by. The length of which every size of the kit is a whole number (a classical kit's column width): `check` holds each piece's `params.pitch`, `reach` and `span`, each wall piece's measured plan, and the kit's other stated lengths to it (`kit_unit`, K8). `module` is the bay; a kit whose sizes come from something smaller states that here. |
| `eaves` | How far the kit's roof stands beyond a building's outline on every side. A gable or a hipped roof is laid on the outline grown by it, so a roof piece's span is half of a building's depth and the eaves together. `check` holds each slope to a depth a building of the kit can have. |
| `module`, `storey`, `parapet`, `base` | Not laid by: a stretch closes by its pieces' pitches and a storey's height is the building's. `check` holds the pieces to them (`parapet`, `base`: the heights of the pieces tagged `storey:parapet` and `storey:base`). |

Every key is optional. What one piece alone takes of a layout stays on the piece (`params.pitch`,
`reach`, `span`: R69). `check` holds each piece to what its kit states (`kit_stated`: a wall piece
of another thickness, a bay of another module, a piece of none of the stated storeys), and a kit
that states nothing is told once (`kit_unstated`, info) with what was worked out of its bay and
the line that would state it. Such a kit is still laid, by its pieces, as before.

Before this the plain kit's own author stated its storey wrongly at the first try (3 m, where the
kit also has 4 m ground-storey pieces), and `kit_stated` named the three pieces: the statement and
the pieces check each other.

## Two readers

`lib/building/lay.py` lays the walls of a building round its outline. `lib/town/kit.py` lays a row
along a path. They do not share code: a library step imports only the SDK and the standard library
(R19b), so each reads the pack through `pack.query` and keeps its own small reader. This page is
what they share.

They differ where a length is not whole pieces, and each for a reason:

| | lib/building `lay` | lib/town kit run |
|---|---|---|
| What must close | A wall between two corners, or a corner and a door. | A row between two ends, corners or gaps. |
| Leftover length | Refused: `wall_open`, with the two nearest lengths that close. A wall has no slack: a gap in it is a hole in a building. | Spread over the joints, up to `joint` (0.05 m) each, and beside a gap the gap widens. A row of houses may stand a little apart. Past that it is `open`, an error. |
| Beside a gap | The wall is cut back to a length that closes; more than the margin left open is `gap_wide`, with where the door would close. | The cap piece moves into the gap. |
| A fit with no bay | `fillers_only`. | Not checked. |

## A kit that joins others (K9)

A third reader, `lib/building/pipe.py`, lays a pipe: the glue between two kits
(`examples/works`). It keeps its own small reader, as the other two do.

| Role | The piece |
|---|---|
| `pipe` | The straight piece, `params.pitch` long along its x. Required. |
| `slip` | A shorter straight piece for what whole pieces leave of a leg: the pipe kit's filler. |
| `elbow` | A turn of `params.angle` degrees, one arm to its back and one to its right; it takes `params.reach` of each leg. One piece turned makes every quarter turn. |
| `riser` | A climb of `params.rise` metres at one point; it takes `params.reach` of each leg. Its pack gives it a mount `top`, with `rise` the same height and the connector `pipe`, which is a leg's own word. |
| `flange` | The plug on a pipe's end: a mount `plug` at its origin, facing out of the pipe, with the kit's connector. |
| `cap` | A closed end. |
| `support` | A post and saddle for a pipe `params.height` up, reaching that far down from the pipe's middle line. The kit states the most metres between two in `shared.support`. |

Every pipe piece is made about the pipe's middle line and says `pivot = "origin"`: it stands on
a surface at the pipe's height.

**A port** is what a kit offers another: a wall piece's mount with a `connector`, and a `rise`
where the hole is above the floor (`spec/assets.md` 2.7; R75). It is stated on a variant of the
bay (`variant:port`, or any word), laid in its stretch's run as any variant is. By this library's
convention a port lies on the building's outline: half a post in front of a panel laid by posts,
on the outer face of a wall laid at a pitch. Two pieces whose mounts say one connector, at one
point and one height, facing each other, are a joint, which the engine finds and nothing writes
(`spec/surfaces.md` 3.3; R76). A kit of whole rooms joins the same way, door to door: a piece's
origin is its way in, and the next stands on its way out.

Not checked: that the hole in a port's model is where its mount says (a `rise` restated a metre
low moved the mount and not the hole, `examples/works` `low`), and that a pipe or a room kit's
stated sizes are whole numbers of anything (`check` reads wall kits).

## Choosing among pieces

Written at K3. A world never names a piece; it names a variant's word, and where it goes by rule
(`lib/building/README.md`, "Choosing a bay's piece"):

| What is chosen | Written as | Fallback, always said |
|---|---|---|
| A pattern along a wall | `fill = [{ storeys, sides, pattern }]` on the building | A word the kit has not, or whose piece is not the bay's size: the bay, with `variant_none`. |
| No two neighbours alike | A list of words in a pattern's place | The choice is made from the bay's place, not a seed; with one piece to choose from it is that piece. Bays that do not touch (across a door or a corner) are not compared. |
| One bay | `bays = [{ storey, side, at, variant }]` | A point no whole bay holds: nothing swapped, with `bay_none`. |

A storey or side the building has not, a key these tables do not take, and either table with no
kit are errors (`params`), not fallbacks.

Three things the studios' accounts ask for and this does not give: a variant of another length
(a double bay, a half-bay window), which is a fit problem and not a swap; a choice that depends on
what stands beside the wall (a door facing a street); and a weight on each word. None is built.

A stretch of mixed pieces is one run that names each station's piece (`assets`, R68), so a bay
keeps its name whichever piece it holds.

Only lib/building chooses. lib/town's kit run lays the bay alone: in a kit with variants it takes
the first `role:wall` piece with no `variant:` tag, as lib/building does, and no variant.

## What a plan owes its kit

`lib/building/README.md`, "Sizing a plan to its kit". In short: a side is whole pieces plus what
its two corners take; a door and a stair stand where the wall either side of them is whole pieces,
and are written by `side` and `at`, a distance along one side, so they keep their place when
another side grows.

## Not in the contract yet

- **Variants of a filler, a corner or a door,** and a variant of another length than the bay.
- **Steps to a door,** and a base of more than one course (the base itself came with K7).
  Storeys by height and by word, floors, and gable roofs on rectangles came with K4.
- **A pitched roof on a plan that is neither a rectangle nor an L:** a T, a U, an oblique wall,
  wings of two depths. The L came with K7 (`role:roof_hip`, `role:roof_valley`, above).
- **The kit's facts in lib/town.** lib/building lays by what a kit states (below); lib/town's
  rows of pieces along a street still read each piece's pitch alone and state nothing of a kit.
- **A storey's height taken from the kit.** A building still writes `storey_h`; the kit's
  `storey` only holds the pieces to it.
- **A piece that closes a terrace or a jetty.** A storey may have its own outline (K7); a
  terrace of whole bays less a wall, and a jetty smaller than a half bay, close on no kit here.
- **A wall's free end and a T-junction of walls** have no role.
- **A tee, a valve and a sloped piece** in a pipe kit; a pipe that follows the land. A port
  that two kits each bring half of.
- **Stretching a piece.** No reader stretches. The studios' tools that stretch by default say
  they regret it (the report, section 7).
