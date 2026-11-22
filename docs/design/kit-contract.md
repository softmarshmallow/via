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
| `storey:<word>` | a tag | The kind of storey the piece is made for: `ground`, `upper` or `top`. It is laid there only, and read there before an untagged piece of its role. |
| `params.span` | the pack, on a roof piece | The plan distance from the eave to the ridge the piece is made for: half the depth of the building it roofs. |
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
| `floor` | lib/building | A square tile, one module on a side, sunk by its thickness. It tiles each storey's floor inside the walls, and a flat roof. |
| `roof_slope` | lib/building | A wedge from the eave to the ridge, `params.pitch` along the eave and `params.span` deep. |
| `roof_gable` | lib/building | Half a gable: a wall one span long, as thick as the bay, rising to the ridge. Two stand at each end of a gable roof, the second turned about. |
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
as a unit". It is ruled at K5.

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

Only lib/building chooses. lib/town's kit run lays the bay alone: in a kit with variants it takes
the first `role:wall` piece with no `variant:` tag, as lib/building does, and no variant.

## What a plan owes its kit

`lib/building/README.md`, "Sizing a plan to its kit". In short: a side is whole pieces plus what
its two corners take; a door and a stair stand where the wall either side of them is whole pieces,
and are written by `side` and `at`, a distance along one side, so they keep their place when
another side grows.

## Not in the contract yet

- **Variants of a filler, a corner or a door,** and a variant of another length than the bay.
- **A wall of mixed pieces as one thing.** A run holds one asset, so `lay` splits a stretch into a
  run per spell and the bays after a swap take new names. The research's gap "choosing among
  pieces" is here in the core, and is ruled at K5.
- **A base course,** and steps to a door (K7). Storeys by height and by word, floors, and gable
  roofs on rectangles came with K4.
- **A pitched roof on a plan that is not a rectangle** (K7). On the plain kit the slopes beside a
  valley must be whole pieces from the inside corner, and the walls there are whole pieces from
  1.5 m off it, so a wing that closes its walls cannot close its roof: the roof needs a reach of
  its own at a valley.
- **The stair as part of the kit.** Its lanes, clearances and landing are the library's numbers,
  not the kit's, so a stairwell is whole floor tiles only where a world sizes the stair by hand,
  and the second stairwell of a three-storey building (two lanes deep) is not.
- **A wall's free end and a T-junction of walls** have no role.
- **Sockets between two kits** (a door size shared across kits, a glue kit, a plug for a hole):
  K9. The core has typed connectors (`spec/assets.md` 2.7); no library kit uses one.
- **Stretching a piece.** No reader stretches. The studios' tools that stretch by default say
  they regret it (the report, section 7).
