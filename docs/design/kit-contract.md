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
| `role:<word>` | a tag | The part the piece plays. One piece per role is read: the first id, bytewise. Variants of a role are K3. |
| `params.pitch` | the pack | The length of wall the piece takes, metres. Without it, the measured width. |
| `params.reach` | the pack, on a corner | The length of each wall a corner takes, from the outline's vertex. |
| `bounds`, `footprint` | measured (`via pack accept`) | The model's box and the ground it claims. |
| `pivot` | the pack, once per kit | The rule that puts the placing point: `base_centre` unless a piece says otherwise. A model's own origin moves nothing (`examples/kit`, `pivot`). |
| `sink` | the pack | How far a piece reaches below its placing point: a plinth. |

A piece lies along its x axis with its front on +y. A wall piece is as thick as the kit's bay.

## Roles

| Role | Read by | The piece |
|---|---|---|
| `wall` | both | The bay: the piece a wall or a row is made of. Required. |
| `filler` | both | A shorter piece for what whole bays leave. It is a part of the module (a half bay), not a patch. |
| `corner` | both | Where a line turns. In a building, the outside corner. |
| `corner_in` | lib/building | The inside corner of a building's outline. Placed by the middle of its notch (`pivot = "origin"`). |
| `door` | lib/building | What stands in a doorway: a frame or a leaf. Its width is the kit's door size. |
| `roof` | lib/building | One piece for a whole roof. Roofs from pieces are K4. |
| `end` | lib/town | The piece at a row's free end and either side of a gap. |

## The kit as a whole

Some facts belong to no one piece. Today each reader works them out from the pieces, and
lib/building's `check` holds the kit to them (`lib/building/check.py`, rule 9):

| Fact | Worked out as | Held by |
|---|---|---|
| The module | The pitch of the smallest wall piece: the filler's, else the bay's. | `kit_module`: the bay and the door piece are whole numbers of it. |
| The wall's thickness | The bay's measured depth. | `kit_uneven`: wall, filler and door are one thickness. |
| The storey | The pieces' measured height. | `kit_uneven`: one height; `kit_height`: the height of a building that stacks them. |
| The door size | The door piece's width, less the margin `lay` leaves (0.1 m). | `gap_wide`, from `lay`: a door asked wider. |
| The corners' reach | `params.reach`: the wall's thickness outside and nothing inside, unless said. | `kit_corner`: the model is the plan its reach states. `kit_ell`: the two reaches add up to whole modules, or no plan with an inside corner closes. |
| No patch piece | A filler is not shorter than the wall is thick. | `kit_slip`. |

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

## What a plan owes its kit

`lib/building/README.md`, "Sizing a plan to its kit". In short: a side is whole pieces plus what
its two corners take; a door and a stair stand where the wall either side of them is whole pieces,
and are written by `side` and `at`, a distance along one side, so they keep their place when
another side grows.

## Not in the contract yet

- **Variants of a role, and a rule for choosing among them** (K3). Today the first id is read.
- **Roles by storey** (a base, a ground storey, a repeating storey, a top) and **roofs from
  pieces** (K4).
- **A wall's free end and a T-junction of walls** have no role.
- **Sockets between two kits** (a door size shared across kits, a glue kit, a plug for a hole):
  K9. The core has typed connectors (`spec/assets.md` 2.7); no library kit uses one.
- **Stretching a piece.** No reader stretches. The studios' tools that stretch by default say
  they regret it (the report, section 7).
