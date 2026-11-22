"""The building library's lay step: one building feature becomes storeys, rooms, flights, doorways,
footpaths, its wall runs and its roof, in one returned document.

A generate step, `for_each = "kind:building"`, run as a worker through the SDK (kinds.toml shows
the table a world restates). It reads the building's outline and attributes (`build.features`),
the ground under the outline (`expr.eval` of the terrain), and the kit (`pack.query`). Everything it
returns is keyed by the building's name, so no distant edit shifts a key (identity.md 2.2), and
everything is in one document because a produced name is never written in text (R3,
`identity.key_in_text`): a second step could not name this one's storeys in `on` or `surface`.

The rules (ours, written for this library; README.md is the design note):

1. **Outline.** The building's region, one ring, counter-clockwise (a clockwise ring is read
   reversed from its first point). `t` along it is arc length from its first point, as `on` takes
   it (geometry.md 2.6). Each edge has its direction `e` and its inward normal `n` (the left one).
2. **Levels.** `level_k = ground + plinth + k * storey_h`, in world metres, where `ground` is the
   mean of the terrain read at every post inside the outline (edges included) and at its vertices,
   rounded to the millimetre, and every level is rounded to the millimetre so a floor and the f32
   post under its sill round to one millimetre (surfaces.md 3.4, `gap`). **Storeys of two
   heights.** Storey 0 is the `ground` storey, the last of two or more the `top`, the rest `upper`.
   `ground_h` and `top_h` (0: as `storey_h`) are the heights of the ground and the top storey, and
   a level is then the ground, the plinth and the heights of the storeys below. The eaves are the
   top storey's level and its own height.
3. **Stair.** `stair = { side, at, width, run }` (or `{ t, width, run }`): the flights climb along the outline edge holding that place (`where`: `side` and `at` are a side of the outline and the metres along it; `t` is a share of the whole outline, and moves when any side grows),
   from `s0` (the point at `t`, along that edge) to `s1 = s0 + run`, in lanes `CLEAR` (0.1 m) off
   the wall's inner face and off each other: lane 0's centre is `T + CLEAR + width / 2` in, lane
   1's one lane further, where `T` is the kit's wall thickness (0 with no kit: surfaces.md 6.4's
   `[21, 20.6]`). Measured from the outline, a 0.3 m wall stood 0.2 m into a 1 m flight. Flight `k` joins storey `k` to storey `k + 1`: an even flight climbs lane 0 from `s0`
   to `s1`, an odd one lane 1 back from `s1` to `s0` (a switchback). Storey `k + 1` loses its
   stairwell, a notch from the wall: for an even flight `[s0 - LANDING, s1]` one lane deep, for an
   odd one `[s0, s1 + LANDING]` two lanes deep and `T` more (both clamped to the edge), so each flight's end
   lies on the notch's side (on the outline: inside, surfaces.md 1.2) and the next flight starts on
   the floor. An end rounded to the millimetre outside its floor (an outline turned off the
   lattice's axes) moves on along its flight, up to 3 mm, until the floor holds it (`onto`). A notch that reaches a corner needs the next edge square to this one, or it is
   refused (`params`); one that stops short of a corner by less than `STUB_MIN` (0.3 m, ours)
   leaves a wall too short to build and is refused too (`params`, naming the stair's `t`).
   **The footing.** A wall stands on the wall below, not on the floor. With a kit, where the two
   corners of the stair's side are square, the storey above gets a `footing` (`<storey>_footing`):
   the strip of that side as deep as the wall is thick, its first edge the side itself, with one
   surface at the storey's level. The storey's own ring keeps off it: that side is moved in by
   `T`, and the stairwell is a notch from the moved side, held `T` off the walls beside it,
   or by what the corner piece there takes of the wall where that is more, so it never reaches
   a corner and no corner piece stands over it. The walls of that storey are then laid round the
   whole outline as on the ground (rule 6), each bay over the bay below; the stair's side stands
   on the footing, the others on the storey's floor, and a corner piece on the footing where
   its placing point lies in it, else on the floor. The stair no longer decides where a wall
   closes, and no stub of wall is left beside a stairwell (`STUB_MIN` holds only where the
   stairwell is cut from the outline). Square is to the 2 mm a corner rounded to the lattice may
   be off (`square`), so a rectangle turned off the axes has its footing. A stairwell that
   would come within `T` of any other side, or leave the outline, is refused (`params`). With no
   kit there is no wall and no footing; with a corner that is not square there is no footing,
   and the side stands open over the stairwell (`wall_open`). Every flight `starts_at = "<storey k>.floor"`, `ends_at = "<storey
   k+1>.floor"`, with `z0`, `z1` the two levels (surfaces.md 6.4).
4. **Doors.** `doors = [{ storey, side, at, width }]` (or `t` for `side` and `at`, as rule 3): a doorway square to the outline there, from
   `porch` metres outside to the outline (its end moved in up to 3 mm where rounding left it
   outside, as rule 3's), `ends_at = "<storey>.floor"`, `sill_z` its level, `flat`
   at least the posts' diagonal so the sill effect holds every post round the end (kinds.toml). A
   `footpath` runs from `walk_out` metres beyond the doorway's start and `ends_at` its start; its
   own start is left free (surfaces.md 3.4: a free end is answered by nothing and checked for
   nothing), for a hand path or a way's footpath to meet. A door on an upper storey needs a
   balcony (README) and is refused for now; a door inside the stairwell's stretch is refused.
5. **Rooms.** `rooms = [{ storey, shares }]`: the storey's floor is cut across the footprint's
   longest edge (the first of equal ones) at the cumulative `shares`, each piece a `room` region
   with `storey` naming its storey. A cut that crosses the outline more than twice (the room would
   be two pieces) is refused. Two rooms beside each other share an opening `inner_door` (0.9 m)
   wide at the middle of their cut: the wall pieces there leave it open (the walk crosses it).
6. **Walls** (with `kit`, a taxon path of pieces tagged `role:wall`, `role:corner`,
   `role:corner_in`, `role:filler`, `role:door`, `role:roof`, `role:floor`, `role:roof_slope`,
   `role:roof_gable`): along every stretch of a storey's outline that lies on the footprint's
   outline, less the corners and a clear gap for each doorway and flight that crosses it, one `run` of wall pieces at their pitch
   (`params.pitch`, else their `bounds` width) and one of fillers for what is left: the most wall
   pieces such that the rest is a whole number of fillers within `FIT` (5 mm). A stretch that does
   not close is `wall_open` (warning) naming the lengths near it that would. Runs stand on the
   storey's floor (`on` a storey: its one surface, surfaces.md 4.1), `offset` half a wall's
   thickness inward, their front (+y) outward. A corner piece stands at each corner, a
   door leaf in each doorway, an inner wall between rooms leaving its opening, each `at` with
   `surface = "<storey>.floor"` (surfaces.md 4.1, M5).
   **Corners.** The outline is the walls' outer face. A corner takes `reach` metres of each wall it
   joins, measured from the outline's vertex (`params.reach` on the corner piece), and a wall
   stretch is its side less the reach of the corner at each end. At an outside corner (the outline
   turns left) the piece is `role:corner` and its reach defaults to the wall's thickness `T`: a
   post. Its box is `reach` square, in the corner. At an inside corner (the outline turns right)
   the piece is `role:corner_in`, and its reach defaults to 0: a post in the notch the two walls
   leave. A kit with no `role:corner_in` has its `role:corner` stand there, taking nothing, where
   that piece is a post (it says no reach, or the wall's thickness). It is placed by the notch's middle, which is on
   the floor; the middle of a piece that reaches along both walls is not (it lies in the quarter
   the plan leaves out), so such a piece says `pivot = "origin"` in its pack and has its model's
   origin at the notch's middle. In the asset frame it then spans x from `-(reach + T / 2)` to
   `T / 2` and y from `-T / 2` to `reach + T / 2` (lib/building's `check` compares, rule 7). Both
   corners face out of the wall that follows them round the outline. Why two pieces: with
   posts alone, a side between two outside corners holds its length less `2T` and a side from an
   outside corner to an inside one its length less `T`, so the two sides that make up one side of
   an L are `T` short of it and no L closes on the kit's module. It closes when the two reaches
   add up to a whole number of the smallest piece (a post of 0.3 m outside and an inside corner
   reaching 1.2 m, on a 1.5 m half bay: examples/kit). Studios answer the inside corner in the
   corner piece, never with a filler as thin as the wall (docs/research/Modular kit failures in
   studios.md, sections 3 and 4).
   A corner with no piece (the kit has no `role:corner`, or the outline turns in and the kit's
   corner is no post and it has no `role:corner_in`) is counted and said once per building
   (`corner_none`). A side shorter than what its two corners take is `side_short`, and gets no
   wall. A door whose opening reaches into what a corner takes is refused (`params`), with the
   stretch of the side its middle may stand on.
   **A fit that drops the bay.** A stretch at least one bay long that closes with fillers alone
   is counted; a building with any is `fillers_only` (warning): the bay's pitch fits none of them
   (a bay re-exported on another module re-fits every wall in fillers, and nothing else says so).
   **The clear gap.** A doorway or a flight crosses a stretch where its centreline comes within
   `width / 2` of the wall's band (from the outline to `T` in). The wall leaves it `width + margin`
   (`params.margin`, 0.1 m) clear, centred on where it crosses, and each side of the gap is pulled
   back by the overhang of the pieces beside it (half a piece's `bounds` width past its pitch), so
   the clear width between wall instances is never less than the path's width plus the margin.
   Before, the gap was the door's width to the pitch, and a piece wider than its pitch closed it
   (TODO, M5 slice 1: the station's front door 0.20 m clear, its tower step 0.29 m).
   A stretch beside a gap that does not close is shortened to the next length that does, so the
   gap widens. What the wall then leaves, less what the gap needs (the path's width and the
   margin, or the kit's door piece, `role:door`, where that is wider), is open wall. More than the
   margin of it is `gap_wide` (warning), naming the door or flight and the metres left open: a
   door may sit a margin loose in its wall, not a piece's length. The kit's door piece is its
   door size; a door asked wider than it is told so here.
8. **Variants of the bay.** A kit may hold several `role:wall` pieces: the bay (the first with no
   `variant:` tag) and its variants, each tagged `variant:<word>` (a window bay, a vent, an arch).
   A variant stands where a bay stood, so it is the bay's size: its `params.pitch` and its measured
   box (length, thickness, height, and where the box lies about the piece's origin), each within
   `FIT`. One that is not is refused, and where it is asked for the bay is laid and `variant_none`
   (warning) says what differs; so is a word no piece carries. Of two pieces with one word the
   first that is the bay's size is taken. A building chooses in two ways, the second over the
   first:
   - `fill = [{ storeys, sides, pattern }]`: on those storeys and sides (`"all"`, or lists), the
     bays of each stretch take the words of `pattern` in turn, from the stretch's start, round
     again as often as it takes. `"wall"` is the kit's bay. A later entry is laid over an earlier.
     An entry of `pattern` that is a list of words is a choice among them, made from the bay's
     place alone (the building's name, the storey, the side and the distance along it): nothing is
     drawn, so a build is the same every time, and a bay that does not move keeps its word. Two
     bays that touch in one stretch do not take one piece where the words allow it: a choice
     leaves out the fixed word before and after it in the pattern; two pieces are laid in turn
     along the side; among three or more, a bay whose word is the one the bay before it kept
     takes another, which is neither that word nor the next bay's. So a bay's piece can change
     only when a bay beside it with the same word comes or goes. Bays either side of a door, a
     corner or a half bay do not touch and are not compared. A word may stand once in a choice:
     there are no weights.
   - `bays = [{ storey, side, at, variant }]`: the bay that holds the point `at` metres along that
     side takes the variant; a point on a joint is the bay after it. It is an input, not an edit
     of what was laid: when the plan or the module changes, the bay that then holds the point
     takes it. Where no whole bay holds it (a half bay, a corner, a door, no wall) nothing is
     swapped and `bay_none` (warning) says so. It is laid whatever stands beside it.
   A storey or a side the building has not, a key these tables do not take, and `fill` or `bays`
   on a building with no kit are `params` (error). Fillers, corners and doors have no variants.
   A stretch's bays are one run, named as it always was: of the kit's bay alone it names the
   one `asset`; where a variant stands in it, it names each station's piece (`assets`, rules.md 5,
   R68), so a bay keeps its place and its name whichever piece it holds. Source: one-dimensional patterns
   over roles in studios' kit recipes and engines' placement grammars (docs/research/Modular
   city asset contracts.md, section 5); edits kept as inputs to the generator, not as changes to
   its output (docs/research/Modular kit failures in studios.md, section 11); a rule that
   neighbours may not repeat (the same report, section 8).
   **Pieces by storey.** Each storey reads the kit for itself (`kit_rows`). It takes the pieces
   of its own height: for each of `wall` (its bays), `filler`, `corner`, `corner_in` and `door`,
   the pieces nearest the storey's height stay (within `FIT` of the nearest) and the role's
   others are left out; where the nearest is not the storey's height `check` says `kit_height`. A piece tagged
   `storey:ground`, `storey:upper` or `storey:top` is for that kind of storey only, and there it
   is read before an untagged piece of its role. The bays of all storeys are one thickness, or
   the building is refused (`params`). **Names.** A run is `<storey>_w<side>_<j>`: the outline's
   side and the number of the stretch a door cuts it into; a corner piece `<storey>_c<vertex>`.
   On a storey whose stairwell is cut from the outline (no footing) the numbers are its own
   ring's edges and vertices.
7. **Roof.** `roof = "garden"`: a `roof` region over the footprint whose `garden` is at the
   eaves. `"flat"` and `"gable"` are rule 10. With none said and a `role:roof` piece in the kit,
   one instance at the footprint's middle on the top floor, facing out of the longest edge. Any
   other word is refused (`params`).
9. **Floors** (with a `role:floor` piece in the storey's kit: a square tile, its `params.pitch` on
   a side, sunk in the pack by its thickness so its top is the floor). Each storey's floor is
   tiled inside the walls: on a grid of the tile's pitch from the first corner of side 0 moved in
   by `T`, along side 0 and square to it, a cell holds a tile (`<storey>_f<i>_<j>`, `at` its
   middle, `surface` the storey's floor) where it lies wholly on the storey's ring and inside the
   outline moved in by `T`. A cell that lies partly on the floor holds none and the floor stands
   open there: `slab_open` (warning), with the stair width that would make its stairwell whole
   tiles deep. Nothing is cut to fit. On a building whose side 0 lies along neither axis no tile
   is laid and `slab_open` says so once: tiles are instances on the millimetre lattice, and off
   the axes two that abut overlap by the rounding (rules.md 8.1).
10. **Roofs from pieces.** Every roof piece stands at the eaves, on the `garden` of a `roof`
   feature over the footprint (`<building>_roof`). `roof = "flat"`: the floor tiles, laid as rule
   9 lays them over the whole footprint (`<building>_roof_f<i>_<j>`); the walls' tops stand round
   them. `roof = "gable"`, on a rectangle: the ridge lies along the longer sides (the first of
   equal ones). `role:roof_slope` is a wedge that climbs from its front edge to its back,
   `params.pitch` long along the eave and `params.span` deep in plan; the building's depth must be
   twice the span within `FIT`. One run of slopes stands along each long side
   (`<building>_roof_e<side>`), from `T` in at one end to `T` in at the other, which must be whole
   pieces, their middles half a span in from the eave and their fronts out. `role:roof_gable`
   is half a gable: a wall `T` thick, one `params.span` long, low at its +x end and as high as
   the slope at its -x end. Two stand on each short side (`<building>_roof_g<side>_0`, `_1`),
   half a span and a span and a half along it, the first's front out and the second's in, so
   their high ends meet under the ridge. A rectangle is four sides each square to the next
   (`square`), turned or not. A roof that cannot be laid (no kit; a flat roof on a building
   turned off the axes, as rule 9; for a gable: not a rectangle, a depth that is not twice a slope's span, an eave that is not whole pieces, a
   gable piece of another span, a kit without the pieces) is `roof_none` (warning) with its
   reason, and the flat roof is laid in its place where the kit has a floor tile. **The
   parapet.** Round a flat roof that was asked for, the kit's wall pieces tagged
   `storey:parapet` (a bay, and a filler and a corner where the kit has them) are laid along the
   outline exactly as a storey's walls are (rule 6), standing on the roof: runs
   `<building>_roof_w<side>_<j>`, corners `<building>_roof_c<i>`. A kit with no such bay gets
   none. A stretch that does not close is `wall_open`, said of the parapet. Source: ours;
   the research found no developer's account of roofs from pieces (docs/research/Modular kit
   failures in studios.md, section 9).
13. **The wall's foot.** A wall piece stands on its storey's floor, and a floor is level: where
   the ground is lower than the ground storey's floor, the wall's foot is in the air. The ground
   is read along the outline every 0.5 m (`read_foot`: the terrain as this step sees it, before
   the building's own doorways shape it). **The base** is the kit's wall pieces tagged
   `storey:base` (a bay, and a filler and a corner where the kit has them), laid round the whole
   outline exactly as a storey's walls are (rule 6), under the door too: runs
   `<building>_base_w<side>_<j>`, corners `<building>_base_c<i>`, on the ground storey's floor.
   The pack sinks each by its own height (`sink`), so its top is the floor and it reaches its
   height below it. Where the ground lies lower than the base reaches, by more than 0.05 m, the
   building says `base_short` (warning): the most the ground falls below the floor, the base's
   depth, and the length of outline it stops short along. Nothing is stretched to reach the
   ground. A kit with no base says `foot_open` (warning) where the ground falls more than the
   building's `plinth` below the floor, and `plinth_bare` (info) where the floor only stands its
   plinth above it: the walls' feet show that much air all round, as they have on every
   building of a kit since M5. Source: the stone base under PUBG's modular buildings and the
   foundation layer of Fallout 4's facade stack are the only shipped accounts; flat ground is a
   stated precondition of the tools that say anything (docs/research/Modular kit failures in
   studios.md, section 4, "Meeting the ground"). The rule is ours.
14. **A storey with its own outline.** `setback = [{ storey, sides, by }]`: from that storey up,
   those sides of the outline stand `by` metres inside the footprint; `overhang`, the same,
   outside it. Each storey then has its own outline (the footprint with its sides moved,
   `shift_ring`), and every rule above is that storey's on its own outline: its walls close on
   it (rule 6), its floor is tiled inside it (rule 9), and the roof covers the top storey's
   (rule 10). On a rectangle only. Sides that meet at a corner are not moved together, and the
   side a stair climbs and the two that meet it keep the building's outline, so the stairwell
   is where it was on every storey. Each is `params` (error) otherwise. **The terrace.** Where a
   storey stands inside the one below, the strip between the two outlines is a `roof` feature
   at that storey's level, `<building>_t<storey>_<side>`, tiled (rule 9) from inside its parapet
   to the wall above, with the kit's parapet (rule 10) on its three outer sides. Under an
   overhang nothing is laid: the storey's own floor tiles are its underside. Nothing is fitted:
   a move that leaves a side no whole number of pieces is `wall_open`, as on any plan, and a
   terrace that is no whole number of tiles deep is `slab_open`. On a kit whose walls are laid
   from their outer face, a set-back of whole bays leaves a terrace one wall's thickness short
   of whole tiles, and its side parapets two posts short of whole pieces: both are said, and
   neither is closed (README.md, "A storey with its own outline"). Source: ours; the set-back
   of a city block and the jetty of a timber house are named in no developer's account read.
11. **What a kit states.** A kit is a taxon's pieces, and what they share it says once: the
   `shared` table in that taxon's `defaults` (spec/assets.md 2.5; R66 of docs/doctrine.md; Asset
   law 5 of docs/ledger/archetypes.md). `lay` reads six of its keys, each optional: `wall`, the
   walls' thickness, which is then `T` in every rule above in place of the bay's measured box;
   `stair = { lane, width, run, landing }`, the kit's stair: `width` and `run` are a building's
   stair's where it writes none, `landing` replaces `LANDING`, and `lane` is how deep one lane
   is (lane 0's centre is `T + lane / 2` in and its stairwell `lane` deep, lane 1 one `lane`
   further), in place of the width and `CLEAR` each side, so a kit whose lane is whole floor
   tiles gets stairwells that are; `door`, the width of the doorway its door piece is made for,
   which a door that writes no `width` takes; `module`, `storey` (one height or a list) and
   `parapet` and `base` (the heights of its `storey:parapet` and `storey:base` pieces),
   which `lay` does not lay by (a stretch closes by its pieces' pitches, a storey's height is
   the building's) and `check` holds the pieces to (`kit_stated`). What a piece alone takes of a
   layout stays on the piece: `params.pitch`, `reach`, `span` (R69). A kit that states nothing
   is laid by its pieces as before, and `check` says so once (`kit_unstated`, info). Two
   `shared` tables among a kit's rows, or a value of one of these keys that is no length
   above 0, is `params` (error); a key not in this list is another library's, and is passed over. Source: the studios' accounts of a kit as a system and not a
   list of assets, and of the metrics written down before the first piece (docs/research/
   Modular kit failures in studios.md, sections 1 and 2).

`params` of the step: `inner_door` (0.9 m), `margin` (0.1 m). Codes are `step.<id>.<code>`: `params` (error, the
building's attribute or a kit piece's param), `wall_open`, `side_short`, `corner_none`,
`fillers_only`, `gap_wide`, `variant_none`, `bay_none`, `slab_open`, `roof_none`, `foot_open`,
`base_short` (warnings), `plinth_bare` and `laid` (infos; `laid` is what was returned).

Library code imports only viasdk and the standard library (R19b).
"""

import hashlib
import json
import math

from viasdk import RpcError

CLEAR = 100          # mm between a flight and the wall, and between two lanes
LANDING = 1000       # mm of stairwell before an even flight's start and past an odd one's
STUB_MIN = 300       # mm: the least wall a stairwell may leave before a corner (ours: a corner piece)
FIT = 5              # mm a stretch may differ from its pieces' pitches and still close
FOOT_STEP = 500      # mm between the points the ground is read at along an outline (rule 13)
FOOT_SEEN = 0.05     # metres of wall foot above the ground that is said (rule 13)
INNER_DOOR = 900     # mm, an opening between two rooms
MIN_FLAT = 1500      # mm, the least flat ground beside a doorway
MARGIN = 100         # mm a wall's gap is wider than the doorway or flight through it

EXPLAIN = {
    "params": "An attribute of a building is outside what the building library's lay step takes; "
    "the message names the attribute and what it must be (lib/building/lay.py, its rules).",
    "wall_open": "A stretch of wall cannot be closed with whole wall pieces and fillers at their "
    "pitch: the pieces would leave a gap or overlap. The data lists the lengths near it that would "
    "close: resize that side, or move the door or stair that cuts it. A stretch is its side less "
    "what the corner at each end takes (`params.reach`); a plan with an inside corner closes only "
    "when the kit has an inside corner piece (`role:corner_in`) whose reach, with the outside "
    "corner's, is a whole number of the smallest piece. A filler made to fit one gap is a patch: it "
    "hides the fault and the kit grows by a piece for every plan.",
    "fillers_only": "Stretches of wall long enough for a bay were closed with fillers alone: the "
    "bay's pitch fits none of them. If the bay's module was changed, the plan's sides no longer "
    "suit it; resize them to the lengths `wall_open` would name, or restore the module.",
    "side_short": "A side of the outline is shorter than what the corners at its two ends take from "
    "it (`params.reach`), so no wall can be laid on it and the two corner pieces overlap.",
    "corner_none": "The walls turn where the kit has no piece to stand: an outside corner needs a "
    "`role:corner` piece and an inside corner a `role:corner_in` piece, or a post for either "
    "(lib/building/README.md, Sizing a plan to its kit).",
    "variant_none": "A building asks for a variant of the bay that the kit cannot give: no `role:wall` "
    "piece carries that `variant:` tag, or the piece that does is not the bay's pitch, thickness and "
    "height. A variant stands where a bay stood, so it must be the bay's size. The kit's bay is laid.",
    "bay_none": "A `bays` entry names a point along a side where no whole bay stands, so no piece is "
    "swapped: only a bay has variants, never a half bay, a corner or a door.",
    "gap_wide": "The wall beside a door or a flight was shortened to a length that closes, so the "
    "gap is wider than the door piece or the path: the rest stands open. Move the door so the wall "
    "either side is a whole number of pieces, or make the door the width of the kit's door piece.",
    "slab_open": "A floor tile is laid only where it lies wholly on the floor, inside the walls. Where "
    "the floor is no whole number of tiles (beside a stairwell that is not whole tiles wide and long, or in "
    "a plan not sized to its kit) the tiles that would hang over the edge are left out and the floor "
    "stands open there. Nothing is cut to fit.",
    "roof_none": "The roof asked for cannot be laid from the kit's pieces on this plan. A roof piece is "
    "made for one depth of building and whole lengths of eave; nothing is stretched or cut. Where the "
    "kit has a floor tile the flat roof is laid in its place.",
    "foot_open": "A wall piece stands on its storey's floor, and a floor is level. Where the ground falls away "
    "under the building, the wall's foot is in the air unless a base stands under it: the kit's wall pieces "
    "tagged `storey:base`, laid round the outline with their tops at the floor.",
    "base_short": "The kit's base reaches a stated depth below the floor, and the ground under part of the "
    "outline is lower than that. Nothing is stretched to reach it.",
    "plinth_bare": "The floor stands its `plinth` above the ground, and with no base in the kit the walls' feet "
    "show that much air all round.",
    "laid": "What the building library's lay step returned for one building: storeys, rooms, "
    "flights, doorways, footpaths, wall runs and pieces.",
}


class ParamError(ValueError):
    def __init__(self, key, message, hint):
        super().__init__(message)
        self.key, self.message, self.hint = key, message, hint


# --- geometry, in millimetres ----------------------------------------------------------------


def area2(ring):
    return sum(a[0] * b[1] - b[0] * a[1] for a, b in zip(ring, ring[1:] + ring[:1]))


def ccw(ring):
    """The ring without a closing repeat, counter-clockwise; a clockwise one reversed from its
    first point, so `t = 0` stays there."""
    ring = [(int(p[0]), int(p[1])) for p in ring]
    if len(ring) > 1 and ring[0] == ring[-1]:
        ring = ring[:-1]
    if area2(ring) < 0:
        ring = [ring[0]] + ring[:0:-1]
    return ring


def edges(ring):
    """One record per edge: `a`, `b`, length `L` (mm, float), unit `e`, inward normal `n`."""
    out = []
    for a, b in zip(ring, ring[1:] + ring[:1]):
        dx, dy = b[0] - a[0], b[1] - a[1]
        L = math.hypot(dx, dy)
        e = (dx / L, dy / L)
        out.append({"a": a, "b": b, "L": L, "e": e, "n": (-e[1], e[0])})
    return out


def at_t(es, t):
    """(edge index, mm along it) of the point at `t` (a fraction of the outline's arc length);
    a point on a vertex is held by the outgoing edge, `t = 1` by the last (geometry.md 2.6)."""
    total = sum(e["L"] for e in es)
    s = t * total
    c = 0.0
    for k, e in enumerate(es):
        if s < c + e["L"] or k == len(es) - 1:
            return k, min(max(s - c, 0.0), e["L"])
        c += e["L"]
    raise AssertionError("unreachable")


def where(es, spec, key):
    """(edge index, mm along it) of a door or a stair: `t`, a share of the whole outline, or
    `side` and `at`, a side of the outline (side `k` runs from its point `k` to the next, counter-
    clockwise) and the metres along it from where it starts. The second keeps its place when
    another side grows."""
    def number(v):
        return isinstance(v, (int, float)) and not isinstance(v, bool) and math.isfinite(v)
    if "side" in spec or "at" in spec:
        if "t" in spec:
            raise ParamError(key, f"`{key}` says both `t` and `side`", "write `side` and `at`, or `t`")
        k, at = spec.get("side"), spec.get("at")
        if not (isinstance(k, int) and not isinstance(k, bool)) or not number(at):
            raise ParamError(key, f"`{key}` needs `side` (a whole number) and `at` (metres along that side)",
                             "write both, as `side = 0, at = 4.05`")
        s = float(at) * 1000
        if not 0 <= k < len(es):
            raise ParamError(key, f"`{key}` names side {k}; the outline has sides 0 to {len(es) - 1}",
                             f"write a side from 0 to {len(es) - 1}")
        if not 0 <= s <= es[k]["L"] + 1e-6:
            raise ParamError(key, f"`{key}` is {s / 1000:g} m along side {k}, which is {es[k]['L'] / 1000:g} m long",
                             "write `at` from 0 to the side's length")
        return k, min(s, es[k]["L"])
    if not number(spec.get("t")):
        raise ParamError(key, f"`{key}` says no place: `t` is a number from 0 to 1" if "t" in spec
                         else f"`{key}` says no place", "write `side` and `at`, or `t`")
    return at_t(es, float(spec["t"]))


def local(e, s, v):
    """The point `s` mm along edge `e` and `v` mm in from it."""
    return (e["a"][0] + e["e"][0] * s + e["n"][0] * v, e["a"][1] + e["e"][1] * s + e["n"][1] * v)


def rnd(p):
    return (int(round(p[0])), int(round(p[1])))


def bearing(v):
    """Degrees clockwise from north of a plan vector, to the millidegree, in [0, 360)."""
    b = math.degrees(math.atan2(v[0], v[1])) % 360.0
    b = round(b, 3)
    return 0.0 if b >= 360.0 else b


def outward(e):
    return bearing((-e["n"][0], -e["n"][1]))


def inside(ring, p):
    """Point in ring, the edge counting as inside (surfaces.md 1.2)."""
    x, y = p
    hit = False
    for a, b in zip(ring, ring[1:] + ring[:1]):
        cross = (b[0] - a[0]) * (y - a[1]) - (b[1] - a[1]) * (x - a[0])
        if cross == 0 and min(a[0], b[0]) <= x <= max(a[0], b[0]) and min(a[1], b[1]) <= y <= max(a[1], b[1]):
            return True
        if (a[1] > y) != (b[1] > y):
            xi = a[0] + (y - a[1]) * (b[0] - a[0]) / (b[1] - a[1])
            if x < xi:
                hit = not hit
    return hit


def onto(ring, p, behind, most=3):
    """`p` (mm, on the ring's edge before rounding) or the first point up to `most` mm further
    from `behind` along the line through both that the ring holds: an end laid on an outline or a
    stairwell's side, rounded to the millimetre off a lattice-aligned edge, can fall a hair outside
    its floor (a 20 degree turn gave `surface.outside`, M5 close)."""
    if inside(ring, p):
        return p
    dx, dy = p[0] - behind[0], p[1] - behind[1]
    n = math.hypot(dx, dy) or 1.0
    for d in range(1, most + 1):
        q = rnd((p[0] + dx / n * d, p[1] + dy / n * d))
        if inside(ring, q):
            return q
    return p


def same_dir(u, v):
    return abs(u[0] * v[1] - u[1] * v[0]) < 1e-9 and u[0] * v[0] + u[1] * v[1] > 0


def square(a, b):
    """True when two edges are square to each other, to the 2 mm over the shorter one that
    corners rounded to the lattice may be off (a rectangle turned off the axes is still one)."""
    return abs(a["e"][0] * b["e"][0] + a["e"][1] * b["e"][1]) * min(a["L"], b["L"]) <= 2.0


def seg_dist(p, e):
    """mm from a point to an edge."""
    u = (p[0] - e["a"][0]) * e["e"][0] + (p[1] - e["a"][1]) * e["e"][1]
    u = max(0.0, min(e["L"], u))
    return math.hypot(p[0] - e["a"][0] - e["e"][0] * u, p[1] - e["a"][1] - e["e"][1] * u)


# --- levels -----------------------------------------------------------------------------------


def levels(ground_m, plinth, heights, storeys=None):
    """Each storey's level (rule 2): the ground, the plinth, and the heights of the storeys below.
    `heights` is one height in metres per storey, or one number for `storeys` storeys alike."""
    g = round(ground_m, 3)
    if storeys is not None:
        return [round(g + plinth + k * heights, 3) for k in range(storeys)]
    out, below = [], 0
    for h in heights:
        out.append(round(g + plinth + below / 1000, 3))
        below += int(round(h * 1000))
    return out


def storey_kinds(storeys, storey_h, ground_h=0, top_h=0):
    """`[(word, height in metres)]` for each storey (rule 2): the first is `ground`, the last of
    two or more `top`, the rest `upper`; `ground_h` and `top_h` are their heights where not 0."""
    out = []
    for k in range(storeys):
        if k == 0:
            out.append(("ground", ground_h or storey_h))
        elif k == storeys - 1:
            out.append(("top", top_h or storey_h))
        else:
            out.append(("upper", storey_h))
    return out


def square_ends(footprint, k, T):
    """Whether side `k` can have a footing (rule 3): each neighbouring side square to it, and
    longer than the wall is thick."""
    es = edges(footprint)
    e, n = es[k], len(footprint)
    return all(square(o, e) and o["L"] > T + 0.5 for o in (es[(k - 1) % n], es[(k + 1) % n]))


def footing_plan(footprint, notch, T, reach=None):
    """Where a storey has a stairwell beside an outer wall (rule 3): the footing its wall stands
    on, and the storey's own ring. Returns `(storey ring, footing ring)`, or None when a corner at
    either end of that side is not square.
    The footing is the strip of that side `T` deep, its first edge the side itself. The storey's
    ring is the outline with that side moved in by `T` and the stairwell cut from the moved side,
    kept off the walls beside it by `T`, or by what the corner piece at that end takes of the
    wall where that is more (`reach`, mm at the side's start and end), so a notch never reaches a
    corner and a corner piece never stands over it. A stairwell that comes within `T` of any
    other side, or leaves the outline, is refused."""
    k, a, b, depth = notch
    if not square_ends(footprint, k, T):
        return None
    es = edges(footprint)
    e, n = es[k], len(footprint)
    r0, r1 = reach or (T, T)
    p0, p1 = footprint[k], footprint[(k + 1) % n]
    q0 = rnd((p0[0] + e["n"][0] * T, p0[1] + e["n"][1] * T))
    q1 = rnd((p1[0] + e["n"][0] * T, p1[1] + e["n"][1] * T))
    footing = [p0, p1, q1, q0]
    moved = list(footprint)
    moved[k], moved[(k + 1) % n] = q0, q1
    a, b = max(a, float(max(T, r0))), min(b, e["L"] - max(T, r1))
    if b - a <= 0.5 or depth - T <= 0.5:
        return moved, footing
    # The stairwell's three inner sides, every quarter metre, each 2 mm inside it (a point on a
    # line is a coin's toss off the axes): on the building, and a wall's thickness off every side
    # but the stair's own.
    along = [a + 2 + i * (b - a - 4) / max(1, math.ceil((b - a) / 250)) for i in range(max(1, math.ceil((b - a) / 250)) + 1)]
    deep = [T + i * (depth - 2 - T) / max(1, math.ceil((depth - T) / 250)) for i in range(max(1, math.ceil((depth - T) / 250)) + 1)]
    pts = [local(e, s, depth - 2) for s in along] + [local(e, s, v) for s in (a + 2, b - 2) for v in deep]
    others = [o for i, o in enumerate(es) if i != k]
    if not all(inside(footprint, q) and all(seg_dist(q, o) >= T - 2 for o in others) for q in pts):
        raise ParamError("stair", "the stairwell is deeper than the building beside it",
                         "make the building deeper there, the stair narrower, or move it along the wall")
    return notch_ring(moved, (k, a, b, depth - T)), footing


def on_footing(runs, insts, footprint, floor, floor_key, footing, footing_key, side):
    """Walls laid round the outline as on a storey with no stairwell (`wall_plan` with the
    outline for its ring), put where they stand (rule 3): the runs of `side` on the footing, the
    others on the storey's ring, each `span` measured along its own ring; a corner piece on the
    footing where its placing point lies in it, else on the floor. Returns what found nothing to
    stand on, as `(runs, instances)`: they are taken out.
    A run's edge on its ring is the one its middle lies on (within 3 mm: corners are rounded to
    the lattice), so two sides in one line, or a stairwell's edge in line with a side, are not
    mistaken for each other. A stretch that would start before its ring's first point or end past
    its last is written by its members' own stations (`ends = "both"`), so no bay moves."""
    fes, res, ges = edges(footprint), edges(floor), edges(footing)

    def starts(es_):
        out, c = [], 0.0
        for e in es_:
            out.append(c)
            c += e["L"]
        return out, c
    fstart, ftotal = starts(fes)
    rstart, rtotal = starts(res)
    _, gtotal = starts(ges)

    def side_of(s):
        return max(i for i in range(len(fes)) if fstart[i] <= s + 1e-6)
    lost_runs, lost_insts = [], []
    for r in runs:
        u0, u1 = r["span"][0] * ftotal, r["span"][1] * ftotal
        i = side_of((u0 + u1) / 2)
        fe, half = fes[i], (u1 - u0) / 2
        mid = local(fe, (u0 + u1) / 2 - fstart[i], 0)
        if i == side:
            where_, ref = [(0.0, ges[0])], footing_key
            total = gtotal
        else:
            where_, ref = list(zip(rstart, res)), floor_key
            total = rtotal
        found = None
        for start, re in where_:
            u = (mid[0] - re["a"][0]) * re["e"][0] + (mid[1] - re["a"][1]) * re["e"][1]
            v = (mid[0] - re["a"][0]) * re["n"][0] + (mid[1] - re["a"][1]) * re["n"][1]
            if re["e"][0] * fe["e"][0] + re["e"][1] * fe["e"][1] > 0.999 and abs(v) <= 3 and -0.5 <= u <= re["L"] + 0.5:
                found = start + u
                break
        if found is None:
            lost_runs.append(r)
            continue
        lo, hi = found - half, found + half
        r["ref"] = ref
        if lo < 0 or hi > total:
            step = (hi - lo) / r["count"]
            first, last = lo + step / 2, hi - step / 2
            if r["count"] >= 2:
                r["ends"], r["span"] = "both", [round(first / total, 9), round(last / total, 9)]
            else:
                h = min(first, total - first, step / 2)
                r["span"] = [round((first - h) / total, 9), round((first + h) / total, 9)]
        else:
            r["span"] = [round(lo / total, 9), round(hi / total, 9)]
    for it in insts:
        if inside(footing, it["at"]):
            it["surface"] = f"{footing_key}.top"
        elif inside(floor, it["at"]):
            it["surface"] = f"{floor_key}.floor"
        else:
            lost_insts.append(it)
    runs[:] = [r for r in runs if not any(r is x for x in lost_runs)]
    insts[:] = [i for i in insts if not any(i is x for x in lost_insts)]
    return lost_runs, lost_insts


def posts_in(ring, grid):
    """Every post of the grid inside the ring (edges included), then the ring's vertices."""
    out = []
    if grid and grid.get("resolution_mm") and grid.get("extent_mm"):
        x0, y0 = int(grid["extent_mm"][0]), int(grid["extent_mm"][1])
        r = int(grid["resolution_mm"])
        nx, ny = int(grid.get("nx", 0)), int(grid.get("ny", 0))
        i0 = max(0, -(-(min(p[0] for p in ring) - x0) // r))
        i1 = min(nx - 1, (max(p[0] for p in ring) - x0) // r)
        j0 = max(0, -(-(min(p[1] for p in ring) - y0) // r))
        j1 = min(ny - 1, (max(p[1] for p in ring) - y0) // r)
        for j in range(j0, j1 + 1):
            for i in range(i0, i1 + 1):
                p = (x0 + i * r, y0 + j * r)
                if inside(ring, p):
                    out.append(p)
    return out + list(ring)


# --- stair ------------------------------------------------------------------------------------


def stair_plan(ring, stair, storeys, wall_t=0, stated=None):
    """Flights and stairwells (rule 3). Returns {flights: [(path mm, lower, upper)], notches:
    {storey: (edge, a, b, depth)}} with `a`, `b` mm along the edge, and the flights' `width` (mm).
    `stated` is the kit's stair (`kit_shared`, rule 11): its `width` and `run` are the building's
    where the building writes none, its `landing` replaces `LANDING`, and its `lane` is how deep
    one lane is, in place of the width and `CLEAR` each side."""
    if storeys < 2:
        return {"flights": [], "notches": {}, "edge": None}
    if stair is None:
        raise ParamError("stair", f"a building of {storeys} storeys has no `stair`",
                         "write `stair = { side, at, width, run }`: where along the outline the flights climb")
    stated = stated or {}
    es = edges(ring)
    k, s0 = where(es, stair, "stair")
    e = es[k]
    w = int(round(float(stair["width"]) * 1000)) if "width" in stair else stated.get("width", 1000)
    if "run" not in stair and "run" not in stated:
        raise ParamError("stair", "the stair has no `run`, and the kit states none",
                         "write `run`, the metres the flight climbs along the wall")
    run = int(round(float(stair["run"]) * 1000)) if "run" in stair else stated["run"]
    landing = stated.get("landing", LANDING)
    s1 = s0 + run
    if w <= 0 or run <= 0 or s1 > e["L"] + 1e-6:
        raise ParamError("stair", f"the stair runs {run / 1000:g} m from {s0 / 1000:g} m along an edge "
                         f"{e['L'] / 1000:g} m long", "move it or shorten `run` so the flight stays on one edge")
    if stated.get("lane"):
        deep = stated["lane"]
        if w > deep:
            raise ParamError("stair", f"the stair is {w / 1000:g} m wide and the kit's lane is {deep / 1000:g} m",
                             f"write a `width` of {deep / 1000:g} m or less, or leave it out for the kit's")
        lane = [wall_t + deep / 2, wall_t + 1.5 * deep]
        depth = [wall_t + deep, wall_t + 2 * deep]
    else:
        lane = [wall_t + CLEAR + w / 2, wall_t + CLEAR + w / 2 + (w + CLEAR)]
        depth = [wall_t + w + 2 * CLEAR, wall_t + 2 * w + 3 * CLEAR]
    flights, notches = [], {}
    for f in range(storeys - 1):
        if f % 2 == 0:
            path = [rnd(local(e, s0, lane[0])), rnd(local(e, s1, lane[0]))]
            notch = (k, max(0.0, s0 - landing), s1, depth[0])
        else:
            path = [rnd(local(e, s1, lane[1])), rnd(local(e, s0, lane[1]))]
            notch = (k, s0, min(e["L"], s1 + landing), depth[1])
        flights.append((path, f, f + 1))
        notches[f + 1] = notch
        # A stairwell that stops short of a corner leaves a stub of wall there; under STUB_MIN no
        # piece closes it and the corner pieces fall off the floor (M7 close: a fresh agent's
        # notch 0.08 m from a corner gave six errors on produced names). Refuse it by `t`.
        for gap, side in ((notch[1], "start"), (e["L"] - notch[2], "end")):
            if wall_t and square_ends(ring, k, wall_t):
                continue                         # the wall there stands on a footing: no stub is left (rule 3)
            if 0.5 < gap < STUB_MIN:
                raise ParamError("stair", f"the stairwell ends {gap / 1000:.3f} m from the corner at its edge's "
                                 f"{side}: too short a wall to build there",
                                 f"move the stair (or change its `run`) so the stairwell reaches that corner or stops "
                                 f"at least {STUB_MIN / 1000:g} m from it")
    return {"flights": flights, "notches": notches, "edge": k, "span": (s0, s1), "width": w,
            "lane": stated.get("lane")}


def notch_ring(ring, notch):
    """The ring less a rectangle on edge `k` from `a` to `b` along it, `depth` in (rule 3). At a
    corner the rectangle takes the corner, which must be square."""
    k, a, b, depth = notch
    es = edges(ring)
    e = es[k]
    n = len(ring)
    start_corner = a <= 0.5
    end_corner = b >= e["L"] - 0.5
    prev, nxt = es[(k - 1) % n], es[(k + 1) % n]
    if start_corner and not same_dir(prev["e"], (-e["n"][0], -e["n"][1])):
        raise ParamError("stair", "the stairwell reaches a corner that is not square",
                         "move the stair's `t` off the corner")
    if end_corner and not same_dir(nxt["e"], e["n"]):
        raise ParamError("stair", "the stairwell reaches a corner that is not square",
                         "move the stair's `t` or shorten `run`")
    if (start_corner and prev["L"] <= depth) or (end_corner and nxt["L"] <= depth):
        raise ParamError("stair", "the stairwell is deeper than the wall beside it",
                         "make the building deeper or the stair narrower")
    bite = []
    if not start_corner:
        bite.append(rnd(local(e, a, 0)))
    bite.append(rnd(local(e, a, depth)))
    bite.append(rnd(local(e, b, depth)))
    if not end_corner:
        bite.append(rnd(local(e, b, 0)))
    out = []
    for i, p in enumerate(ring):
        if i == k and start_corner:
            pass                                 # the corner vertex goes; the bite starts there
        elif i == (k + 1) % n and end_corner:
            continue
        else:
            out.append(p)
        if i == k:
            out.extend(bite)
    return out


# --- doors ------------------------------------------------------------------------------------


def door_plan(ring, door, porch, walk_out, stated=None):
    """A doorway path (outside to the outline), a footpath (further out, ending at the doorway's
    start), the edge and the interval along it the door opens (rule 4). A door that writes no
    `width` takes `stated` (mm), the kit's (rule 11), else 1 m."""
    es = edges(ring)
    k, s = where(es, door, "doors")
    e = es[k]
    w = int(round(float(door["width"]) * 1000)) if "width" in door else (stated or 1000)
    if s - w / 2 < 0 or s + w / 2 > e["L"]:
        raise ParamError("doors", f"a door {w / 1000:g} m wide {s / 1000:g} m along side {k} runs past a corner",
                         "move it along the wall or narrow the door")
    on = rnd(local(e, s, 0))
    out = rnd(local(e, s, -porch))
    far = rnd(local(e, s, -(porch + walk_out)))
    return {"edge": k, "s": s, "width": w, "doorway": [out, on], "footpath": [far], "at": on}


# --- rooms ------------------------------------------------------------------------------------


def long_axis(ring):
    es = edges(ring)
    best = max(range(len(es)), key=lambda i: (es[i]["L"], -i))
    return es[best]["e"]


def _u(p, ax):
    return p[0] * ax[0] + p[1] * ax[1]


def _cut(p, q, ax, c):
    up, uq = _u(p, ax), _u(q, ax)
    lam = (c - up) / (uq - up)
    return rnd((p[0] + lam * (q[0] - p[0]), p[1] + lam * (q[1] - p[1])))


def clip(ring, ax, c, keep_above):
    """Sutherland-Hodgman against the half-plane `u >= c` (or `<=`)."""
    def ok(p):
        return _u(p, ax) >= c if keep_above else _u(p, ax) <= c
    out = []
    for i, cur in enumerate(ring):
        prev = ring[i - 1]
        if ok(cur):
            if not ok(prev):
                out.append(_cut(prev, cur, ax, c))
            out.append(cur)
        elif ok(prev):
            out.append(_cut(prev, cur, ax, c))
    dedup = []
    for p in out:
        if not dedup or dedup[-1] != p:
            dedup.append(p)
    if len(dedup) > 1 and dedup[0] == dedup[-1]:
        dedup.pop()
    return simplify(dedup)


def simplify(ring):
    """The ring less every vertex in line with its two neighbours: a straight one, or a spike a
    cut along an edge leaves (the cut through a stairwell's side)."""
    ring = list(ring)
    changed = True
    while changed and len(ring) > 3:
        changed = False
        for i in range(len(ring)):
            a, p, b = ring[i - 1], ring[i], ring[(i + 1) % len(ring)]
            if (p[0] - a[0]) * (b[1] - p[1]) - (p[1] - a[1]) * (b[0] - p[0]) == 0:
                del ring[i]
                changed = True
                break
    return ring


def crossings(ring, ax, c):
    """The points where the line `u = c` crosses the ring's edges, ordered along it."""
    pts = []
    for i, cur in enumerate(ring):
        prev = ring[i - 1]
        up, uc = _u(prev, ax) - c, _u(cur, ax) - c
        if (up < 0 < uc) or (uc < 0 < up):
            pts.append(_cut(prev, cur, ax, c))
        elif uc == 0 and up != 0:
            pts.append(cur)
    norm = (-ax[1], ax[0])
    return sorted(set(pts), key=lambda p: _u(p, norm))


def room_plan(ring, footprint, shares, inner=INNER_DOOR):
    """Rooms of one storey (rule 5): their rings, and the openings between neighbours as
    `(i, j, centre, half-chord unit, width, cut ends)`."""
    shares = [float(s) for s in shares]
    if not shares or any(s <= 0 for s in shares):
        raise ParamError("rooms", "`shares` must be a list of numbers above 0",
                         "write one share per room, such as [2, 1]")
    ax = long_axis(footprint)
    us = [_u(p, ax) for p in ring]
    lo, hi = min(us), max(us)
    total = sum(shares)
    cum, cuts = 0.0, []
    for s in shares[:-1]:
        cum += s
        cuts.append(lo + (hi - lo) * cum / total)
    rooms, openings = [], []
    for i in range(len(shares)):
        r = ring
        if i > 0:
            r = clip(r, ax, cuts[i - 1], True)
        if i < len(shares) - 1:
            r = clip(r, ax, cuts[i], False)
        rooms.append(r)
    for i, c in enumerate(cuts):
        pts = crossings(ring, ax, c)
        if len(pts) != 2:
            raise ParamError("rooms", f"the cut between rooms {i} and {i + 1} crosses the storey's "
                             f"outline {len(pts)} times", "change `shares` so each room is one piece")
        a, b = pts
        L = math.dist(a, b)
        if L < inner:
            raise ParamError("rooms", f"rooms {i} and {i + 1} meet along {L / 1000:g} m, less than "
                             "their opening", "change `shares`")
        mid = ((a[0] + b[0]) / 2, (a[1] + b[1]) / 2)
        openings.append({"rooms": (i, i + 1), "at": rnd(mid), "ends": (a, b), "width": inner})
    return rooms, openings


# --- the kit and the walls --------------------------------------------------------------------


def fit(length, pitch, filler, tol=FIT):
    """The most wall pieces `n` and fillers `k` with `n * pitch + k * filler` within `tol` of
    `length` (all mm), or None and the nearest lengths below and above that would close."""
    best = None
    for n in range(int(length // pitch), -1, -1):
        rest = length - n * pitch
        if filler:
            k = int(round(rest / filler))
            if abs(rest - k * filler) <= tol:
                best = (n, k)
                break
        elif abs(rest) <= tol:
            best = (n, 0)
            break
    if best is not None:
        return best, None
    closes = sorted({n * pitch + k * (filler or 0) for n in range(int(length // pitch) + 2)
                     for k in (range(0, int(length // filler) + 2) if filler else (0,))})
    below = max((c for c in closes if c < length), default=None)
    above = min((c for c in closes if c > length), default=None)
    return None, (below, above)


def gap_places(a, b, need, wall, filler, near, tol=FIT):
    """Where the middle of a gap `need` mm wide may lie between `a` and `b` (mm along a side) so the
    wall each side of it is whole pieces: the nearest such middle at or below `near`, and the
    nearest above it (None where there is none)."""
    room = b - a - need
    if room < -tol:
        return None, None
    lengths = sorted({n * wall + k * filler for n in range(int(room // wall) + 2)
                      for k in (range(int(room // filler) + 2) if filler else (0,))})
    ok = [a + left + need / 2 for left in lengths
          if left <= room + tol and (room - left <= tol or fit(room - left, wall, filler, tol)[0] is not None)]
    below = max((c for c in ok if c <= near + tol), default=None)
    above = min((c for c in ok if c > near + tol), default=None)
    return below, above


def variant_off(v, bay):
    """What makes the piece `v` not the bay's size (rule 8), as words; empty when it is."""
    off = [f"{word} {v[k] / 1000:g} m for the bay's {bay[k] / 1000:g} m"
           for k, word in (("pitch", "pitch"), ("width", "length"), ("thick", "thickness"), ("high", "height"))
           if abs(v[k] - bay[k]) > FIT]
    if not off and v["box"] and bay["box"]:
        d = max(abs(x - y) for x, y in zip(v["box"], bay["box"]))
        if d > FIT:
            off.append(f"its box lies {d / 1000:g} m off the bay's about its origin")
    return off


BY_HEIGHT = ("wall", "filler", "corner", "corner_in", "door")      # the roles a storey's height chooses among
SHARED = ("module", "wall", "door", "storey", "parapet", "base", "stair")  # what a kit may state once (rule 11)
STAIR_KEYS = ("lane", "width", "run", "landing")


def kit_shared(rows):
    """What a kit states once (rule 11): the `shared` table of its taxon's `defaults`
    (spec/assets.md 2.5), which every piece's row carries. Returns {} where the kit states
    nothing, else, in mm, those of: `module`, `wall`, `door` (lengths), `storey` (a sorted list
    of heights, from one height or a list), `stair` ({lane, width, run, landing}, each where
    stated). A kit is one taxon's pieces and says a thing once: two tables among its rows, or a
    value of a key read here that is not a length above 0, is the kit's fault and is said as one
    (`params`). A key this library does not read is another's, and is passed over."""
    tables = []
    for r in rows or []:
        t = r.get("shared") or {}
        if t and t not in tables:
            tables.append(t)
    if not tables:
        return {}
    if len(tables) > 1:
        raise ParamError("kit", "the kit's pieces carry two `shared` tables: some are filed at a taxon below "
                         "that writes its own", "state `shared` once, in the `defaults` of the kit's own taxon")
    t = tables[0]

    def mm(v, key):
        if isinstance(v, bool) or not isinstance(v, (int, float)) or not math.isfinite(v) or v <= 0:
            raise ParamError("kit", f"the kit states `shared.{key} = {v!r}`, not a length above 0",
                             f"write `shared.{key}` as metres")
        return int(round(v * 1000))
    out = {}
    for k, v in t.items():
        if k not in SHARED:              # another library's key: `shared` holds any (spec/assets.md 2.5)
            continue
        if k == "storey":
            out[k] = sorted({mm(x, k) for x in (v if isinstance(v, list) else [v])})
        elif k == "stair":
            if not isinstance(v, dict) or any(x not in STAIR_KEYS for x in v):
                raise ParamError("kit", f"the kit states `shared.stair = {v!r}`",
                                 "write `stair = { lane, width, run, landing }`, each in metres and each optional")
            out[k] = {x: mm(y, f"stair.{x}") for x, y in v.items()}
        else:
            out[k] = mm(v, k)
    st = out.get("stair") or {}
    if st.get("lane") and st.get("width") and st["width"] > st["lane"]:
        raise ParamError("kit", f"the kit's stair is {st['width'] / 1000:g} m wide in a lane of "
                         f"{st['lane'] / 1000:g} m", "make `shared.stair.lane` at least its `width`")
    return out


def kit_rows(rows, word=None, height=None):
    """The pack rows one storey reads (rule 6, "Pieces by storey"). A row tagged `storey:<word>`
    is for that kind of storey only, so with `word` the rows of other words are left out, and
    with none every tagged row is. With `height` (mm), for each role a storey's height chooses
    among, the rows nearest that height stay (those within `FIT` of the nearest) and the role's
    others are left out. Where the nearest is not the storey's height, `check` says so
    (`kit_height`): a bay re-exported 0.3 m too tall is still its storey's bay."""
    def tagged(r):
        return next((t[7:] for t in r["tags"] if t.startswith("storey:")), None)
    rows = [r for r in rows if tagged(r) in (None, word)]
    if height is not None:
        for role in BY_HEIGHT:
            # Of the walls, the bays choose: a variant is held to its storey's bay afterwards.
            mine = [r for r in rows if f"role:{role}" in r["tags"] and r["bounds"]
                    and not any(t.startswith("variant:") for t in r["tags"])]
            near = min((abs(r["bounds"][5] - r["bounds"][2] - height) for r in mine), default=0)
            fits = [r for r in mine if abs(r["bounds"][5] - r["bounds"][2] - height) <= near + FIT]
            if len(fits) < len(mine):
                drop = {id(r) for r in mine} - {id(r) for r in fits}
                rows = [r for r in rows if id(r) not in drop]
    return rows


def kit_pieces(rows, word=None, height=None):
    """{role: {id, pitch (mm), thick (mm), width, high, box, reach, span}} from pack rows (id,
    tags, bounds, params, shared), for a storey of kind `word` and `height` mm (`kit_rows`). The
    bay also holds `stated`, what the kit states once (`kit_shared`), and where that states the
    wall's thickness the bay's `thick` is the stated one and `measured` its own. One piece per
    role: a piece with no `variant:` tag before one with, then a piece tagged for this kind of
    storey before an untagged one, then the first id bytewise (docs/design/kit-contract.md). For `wall` that piece is the kit's bay; the
    other `role:wall` pieces that carry `variant:<word>` are its variants, in `wall["variants"]` by
    word when they are the bay's size (`variant_off`), else in `wall["refused"]` with what differs
    (rule 8). Of two pieces with one word the first that is the bay's size is taken. In a kit
    whose every wall piece is a variant, the bay answers to its own word. `wall` itself is no
    variant's word."""
    def piece(r):
        b = r["bounds"]
        width = b[3] - b[0] if b else 0
        pitch, reach = (kit_number(r, k) for k in ("pitch", "reach"))
        span = kit_number(r, "span")
        return {"id": r["id"], "pitch": int(round(pitch * 1000)) if pitch else width,
                "thick": b[4] - b[1] if b else 0, "width": width, "high": b[5] - b[2] if b else 0,
                "box": list(b) if b else None,
                "storey": next((t[7:] for t in r["tags"] if t.startswith("storey:")), None),
                "reach": int(round(reach * 1000)) if reach is not None else None,
                "span": int(round(span * 1000)) if span is not None else None}

    def words(r):
        return [t[8:] for t in r["tags"] if t.startswith("variant:")]
    out, walls = {}, []
    rows = kit_rows(rows, word, height)
    for r in sorted(rows, key=lambda r: (bool(words(r)), not any(t.startswith("storey:") for t in r["tags"]),
                                         r["id"])):
        for tag in r["tags"]:
            if not tag.startswith("role:"):
                continue
            role = tag[5:]
            if role == "wall":
                walls.append(r)
            elif role not in out:
                out[role] = piece(r)
    if walls:
        bay = dict(piece(walls[0]), variants={}, refused={})
        out["wall"] = bay
        for w in words(walls[0]):
            bay["variants"][w] = piece(walls[0])
        for r in walls[1:]:
            v = piece(r)
            off = variant_off(v, bay)
            for w in words(r):
                if w in bay["variants"]:
                    continue
                if w == "wall":
                    bay["refused"].setdefault(w, {"id": r["id"], "why": "`wall` is the bay's own word"})
                elif off:
                    bay["refused"].setdefault(w, {"id": r["id"], "why": ", ".join(off)})
                else:
                    bay["variants"][w] = v
                    bay["refused"].pop(w, None)
    stated = kit_shared(rows)
    if "wall" in out:
        # What the kit states is what a wall is laid by (rule 11); the bay's own measure stays in
        # `measured`, for `check` to hold against it (`kit_stated`).
        out["wall"]["measured"] = out["wall"]["thick"]
        out["wall"]["stated"] = stated
        if stated.get("wall"):
            out["wall"]["thick"] = stated["wall"]
    return out


def kit_number(row, key):
    """A kit piece's `params.<key>` in metres, None where it says none; not a number, or below
    0, is the kit's fault and is said as one (`params`)."""
    v = (row.get("params") or {}).get(key)
    if v is None:
        return None
    if isinstance(v, bool) or not isinstance(v, (int, float)) or not math.isfinite(v) or v < 0:
        raise ParamError("kit", f"the kit piece `{row['id']}` has `params.{key} = {v!r}`, not a length",
                         f"write `params.{key}` as metres, 0 or more")
    return float(v)


def corner_of(kit, convex, T):
    """`(piece or None, reach mm)` for a corner (rule 6): at an outside corner the kit's `corner`,
    reaching the wall's thickness unless it says; at an inside one its `corner_in`, reaching
    nothing unless it says, else its `corner` where that is a post, reaching nothing."""
    post = kit.get("corner")
    if convex:
        return post, (T if post is None or post.get("reach") is None else post["reach"])
    piece = kit.get("corner_in")
    if piece is not None:
        return piece, (0 if piece.get("reach") is None else piece["reach"])
    # No inside corner piece: a post stands in the notch and takes nothing from either wall,
    # whatever it takes at an outside corner. A corner piece that is no post (it says a reach
    # other than the wall's thickness) is not made for the notch, and none is placed.
    if post is not None and post.get("reach") in (None, T):
        return post, 0
    return None, 0


def overhang(kit):
    """mm a wall or filler piece reaches past its pitch at each end (the larger of the two)."""
    return max([0.0] + [max(0.0, (p.get("width", p["pitch"]) - p["pitch"]) / 2)
                        for p in (kit.get("wall"), kit.get("filler")) if p])


def crossing_gap(e, path, width, T, half):
    """The interval `(u0, u1)` mm along edge `e` a path of `width` (mm) crossing the wall's band
    (from the outline to `T` in) needs clear, `half` mm each side of where its centreline lies
    within `width / 2` of the band (rule 6); None where it does not come near."""
    lo, hi = -width / 2, T + width / 2
    us = []
    for p, q in zip(path, path[1:]):
        up = (p[0] - e["a"][0]) * e["e"][0] + (p[1] - e["a"][1]) * e["e"][1]
        uq = (q[0] - e["a"][0]) * e["e"][0] + (q[1] - e["a"][1]) * e["e"][1]
        vp = (p[0] - e["a"][0]) * e["n"][0] + (p[1] - e["a"][1]) * e["n"][1]
        vq = (q[0] - e["a"][0]) * e["n"][0] + (q[1] - e["a"][1]) * e["n"][1]
        if abs(vq - vp) < 1e-9:
            if not lo <= vp <= hi:
                continue
            l0, l1 = 0.0, 1.0
        else:
            a, b = (lo - vp) / (vq - vp), (hi - vp) / (vq - vp)
            l0, l1 = max(0.0, min(a, b)), min(1.0, max(a, b))
            if l0 > l1:
                continue
        us += [up + (uq - up) * l0, up + (uq - up) * l1]
    if not us or max(us) < -half or min(us) > e["L"] + half:
        return None
    return (min(us) - half, max(us) + half)


def wall_stretches(ring, footprint, crossers=(), T=0, margin=MARGIN, over=0.0):
    """Stretches of `ring` on the footprint's outline, as `(ring edge, a, b)` mm along it, with the
    gaps the doorways and flights crossing it need (`crossers`: `(path, width)` in mm; rule 6); and
    the footprint corners the ring keeps, as `(point, edge before, edge after, convex)`."""
    fes = edges(footprint)
    res = edges(ring)
    out, corners = [], []
    fset = {p: i for i, p in enumerate(footprint)}
    for i, re in enumerate(res):
        host = None
        for k, fe in enumerate(fes):
            if not same_dir(re["e"], fe["e"]):
                continue
            da = (re["a"][0] - fe["a"][0]) * fe["n"][0] + (re["a"][1] - fe["a"][1]) * fe["n"][1]
            if abs(da) > 0.5:
                continue
            off = (re["a"][0] - fe["a"][0]) * fe["e"][0] + (re["a"][1] - fe["a"][1]) * fe["e"][1]
            if -0.5 <= off < fe["L"] - 0.5:         # a start at a side's far end is the next side's
                host = (k, off)
                break
        if host is None:
            continue
        gaps = []
        for c in crossers:
            path, w = c[0], c[1]
            g = crossing_gap(re, path, w, T, (w + margin) / 2 + over)
            if g is not None:
                gaps.append((g[0], g[1], c[2] if len(c) > 2 else None, w))
        out.append({"edge": i, "side": host[0], "off": host[1], "L": re["L"], "gaps": sorted(gaps), "start_corner": re["a"] in fset,
                    "end_corner": re["b"] in fset, "e": re})
    for i, p in enumerate(ring):
        if p not in fset:
            continue
        before, after = res[i - 1], res[i]
        turn = before["e"][0] * after["e"][1] - before["e"][1] * after["e"][0]
        corners.append({"at": p, "before": before, "after": after, "convex": turn > 0})
    return out, corners


def convex_at(corners, p):
    for c in corners:
        if c["at"] == p:
            return c["convex"]
    return False


def picks_of(attrs, storeys=None, sides=None, kit=True):
    """`bays` and `fill` of a building, checked (rule 8): `{"bays": [...], "fill": [...]}`, `at` in
    whole mm. With `storeys` and `sides` (how many the building has), an entry that names one it
    has not is refused; so is either table on a building with no `kit`."""
    def whole(v):
        return isinstance(v, int) and not isinstance(v, bool) and v >= 0

    def word(v):
        return isinstance(v, str) and v != ""
    out = {"bays": [], "fill": []}
    faces = attrs.get("faces") or []
    if not isinstance(faces, list) or not all(word(w) for w in faces) or (
            faces and sides is not None and len(faces) != sides):
        raise ParamError("faces", f"`faces` is not a word for each of the outline's {sides} sides",
                         "write one word a side, in the outline's order: `faces = [\"front\", \"party\", "
                         "\"back\", \"party\"]`, or leave it out")
    for key in ("bays", "fill"):
        v = attrs.get(key)
        if v is not None and not isinstance(v, list):
            raise ParamError(key, f"`{key}` is a list of tables", "write it as `[{ ... }]`, or leave it out")
        if v and not kit:
            raise ParamError(key, f"`{key}` chooses among a kit's pieces, and the building has no `kit`",
                             "give the building a `kit`, or take `" + key + "` out")
    for i, b in enumerate(attrs.get("bays") or []):
        ok = (isinstance(b, dict) and whole(b.get("storey", 0)) and whole(b.get("side"))
              and isinstance(b.get("at"), (int, float)) and not isinstance(b.get("at"), bool)
              and math.isfinite(b["at"]) and abs(b["at"]) <= 1e6 and word(b.get("variant")))
        extra = sorted(set(b) - {"storey", "side", "at", "variant"}) if isinstance(b, dict) else []
        if not ok or extra:
            raise ParamError("bays", f"bay {i} is not `{{ storey, side, at, variant }}`"
                             + (f": it has `{extra[0]}`" if extra else ""),
                             "write the storey and side as whole numbers, `at` as metres along that side, "
                             "`variant` as a word a kit piece carries (`variant:<word>`, or \"wall\" for the bay)")
        for key, have in (("storey", storeys), ("side", sides)):
            if have is not None and b.get(key, 0) >= have:
                raise ParamError("bays", f"bay {i} names {key} {b.get(key, 0)} of a building with {have}",
                                 f"count {key}s from 0")
        out["bays"].append({"storey": b.get("storey", 0), "side": b["side"], "at": int(round(float(b["at"]) * 1000)),
                            "variant": b["variant"], "hit": False, "index": i})
    for i, f in enumerate(attrs.get("fill") or []):
        pattern = f.get("pattern") if isinstance(f, dict) else None
        ok = (isinstance(pattern, list) and pattern and all(
            word(x) or (isinstance(x, list) and x and all(word(y) for y in x) and len(set(x)) == len(x))
            for x in pattern))
        for key in ("storeys", "sides"):
            v = f.get(key, "all") if isinstance(f, dict) else None
            ok = ok and (v == "all" or (isinstance(v, list) and v and all(whole(x) for x in v)))
        facing = f.get("faces") if isinstance(f, dict) else None
        ok = ok and (facing is None or (isinstance(facing, list) and facing and all(word(x) for x in facing)))
        extra = sorted(set(f) - {"storeys", "sides", "faces", "pattern"}) if isinstance(f, dict) else []
        if not ok or extra:
            raise ParamError("fill", f"fill {i} is not `{{ storeys, sides, pattern }}`"
                             + (f": it has `{extra[0]}`" if extra else ""),
                             "write `storeys` and `sides` as \"all\" or lists of whole numbers (or `faces` as a "
                             "list of the words the building's `faces` gives its sides), and `pattern` "
                             "as a list of variant words (\"wall\" for the kit's bay), or lists of words to choose "
                             "among, each word once")
        if facing is not None:
            # By what a side faces (rule 12): the sides whose word in the building's `faces` is
            # one of these. A word no side has matches none, and is no fault: a row gives every
            # building one `fill`, and a building in its middle has no `end`.
            if "sides" in f:
                raise ParamError("fill", f"fill {i} has both `sides` and `faces`", "say which sides by one of them")
            if not faces:
                raise ParamError("fill", f"fill {i} names `faces`, and the building says of no side what it faces",
                                 "give the building `faces`, a word for each side of its outline in order, or "
                                 "name the `sides` by number")
            f = dict(f, sides=[k for k, w in enumerate(faces) if w in facing] or [-1])
        for key, have in (("storeys", storeys), ("sides", sides)):
            v = f.get(key, "all")
            if have is not None and v != "all" and max(v) >= have:
                raise ParamError("fill", f"fill {i} names {key[:-1]} {max(v)} of a building with {have}",
                                 f"count {key} from 0")
        out["fill"].append({"storeys": f.get("storeys", "all"), "sides": f.get("sides", "all"), "pattern": pattern})
    return out


def asked(picks, storey=None):
    """Every variant word a building's `fill` and `bays` name (for `storey`, where one is given),
    sorted; never `wall`."""
    words = {b["variant"] for b in picks["bays"] if storey is None or b["storey"] == storey}
    for f in picks["fill"]:
        if storey is not None and f["storeys"] != "all" and storey not in f["storeys"]:
            continue
        for item in f["pattern"]:
            words.update([item] if isinstance(item, str) else item)
    return sorted(words - {"wall"})


def _hash(*parts):
    """A number from a place: the same in every build and on every machine."""
    return int.from_bytes(hashlib.blake2b("\0".join(str(p) for p in parts).encode(), digest_size=8).digest(), "big")


def bay_variants(n, first, pitch, side, storey, picks, name, variants=None):
    """The variant word of each of the `n` bays of one stretch, None for the kit's bay (rule 8).
    `first` is where the first bay starts, mm along the outline's side `side`; `variants` the
    words the kit can give (`kit_pieces`), by which two words are known to lay one piece: a word it
    cannot give lays the bay, as `"wall"` does (with None, every word is its own piece).
    A `fill` gives each bay the word of its pattern at the bay's number in the stretch. A list of
    words there is a choice made from the bay's place (the building's name, its storey, its side
    and how far along it stands), so that two bays that touch do not take one piece where the
    words allow it; rule 8 says how. A `bays` entry then names one bay by a point along the side."""
    def piece(w):
        if w == "wall" or (variants is not None and w not in variants):
            return None
        return w if variants is None else variants[w]["id"]
    out = [None] * n
    place = [int(round(first + i * pitch)) for i in range(n)]
    for f in picks["fill"]:
        if (f["storeys"] != "all" and storey not in f["storeys"]) or (f["sides"] != "all" and side not in f["sides"]):
            continue
        pat = f["pattern"]

        def fixed(i):
            return pat[i % len(pat)] if 0 <= i < n and isinstance(pat[i % len(pat)], str) else None
        cands, raw = [None] * n, [None] * n
        for i in range(n):
            item = pat[i % len(pat)]
            if isinstance(item, str):
                raw[i] = item
                continue
            c, seen = [], set()
            for w in item:                      # one word for each piece: two words of one piece are one
                if piece(w) not in seen:
                    seen.add(piece(w))
                    c.append(w)
            beside = {piece(w) for w in (fixed(i - 1), fixed(i + 1)) if w is not None}
            c = [w for w in c if piece(w) not in beside] or c
            cands[i] = c
            if len(c) == 2:                     # two pieces can only go in turn: by the bay's number along the side
                raw[i] = c[(place[i] // int(pitch) + _hash(name, storey, side)) % 2]
            else:
                raw[i] = c[_hash(name, storey, side, place[i]) % len(c)]
        final = list(raw)
        for i in range(1, n):
            c = cands[i]
            if c is None or len(c) < 3 or piece(final[i - 1]) != piece(raw[i]):
                continue
            avoid = {piece(raw[i])} | ({piece(raw[i + 1])} if i + 1 < n else set())
            rest = [w for w in c if piece(w) not in avoid]
            if rest:
                final[i] = rest[(_hash(name, storey, side, place[i]) >> 20) % len(rest)]
        for i in range(n):
            out[i] = None if final[i] == "wall" else final[i]
    for b in picks["bays"]:
        if b["storey"] != storey or b["side"] != side or not n:
            continue
        i = (b["at"] - place[0]) // int(pitch)          # a point on a joint is the bay after it
        if 0 <= i < n:
            out[i] = None if b["variant"] == "wall" else b["variant"]
            b["hit"] = True
    return out


def storey_of(key):
    """The storey's number from its key `<building>_s<k>`."""
    tail = str(key).rsplit("_s", 1)[-1]
    return int(tail) if tail.isdigit() else 0


def wall_plan(ring, footprint, doors, kit, prefix, storey_key, crossers=(), margin=MARGIN, notes=None,
              picks=None, name=""):
    """Runs and instances for one storey's walls (rules 6 and 8). Returns (runs, instances, open).
    `crossers` are `(path, width mm[, label])`; `notes`, a list, takes what the fit did that a
    reader should know: `fillers_only` stretches, `gap_wide` gaps, a variant the kit has not.
    `picks` (`picks_of`) and the building's `name` choose each bay's variant."""
    wall, filler = kit.get("wall"), kit.get("filler")
    runs, insts, opens = [], [], []
    notes = [] if notes is None else notes
    picks = picks or {"bays": [], "fill": []}
    if wall is None:
        return runs, insts, opens
    T = wall["thick"]
    over = overhang(kit)
    stretches, corners = wall_stretches(ring, footprint, crossers, T, margin, over)
    variants = wall.get("variants") or {}
    for w in asked(picks, storey_of(storey_key)):
        if w not in variants and not any(x["code"] == "variant_none" and x["variant"] == w for x in notes):
            notes.append({"code": "variant_none", "variant": w, "refused": (wall.get("refused") or {}).get(w),
                          "at": rnd(footprint[0])})

    def reach_at(p):
        return corner_of(kit, convex_at(corners, p), T)[1]
    res = edges(ring)
    starts, c = [], 0.0
    for e in res:
        starts.append(c)
        c += e["L"]
    total = c
    for st in stretches:
        e = st["e"]
        a = float(reach_at(e["a"])) if st["start_corner"] else 0.0
        b = st["L"] - (reach_at(e["b"]) if st["end_corner"] else 0.0)
        if b - a < -FIT:
            notes.append({"code": "side_short", "side": st["side"], "length": st["L"], "takes": st["L"] - (b - a),
                          "at": rnd(local(e, st["L"] / 2, 0))})
            continue
        pieces, at, after_gap = [], a, False
        for g0, g1, _, _ in st["gaps"]:
            pieces.append((at, min(g0, b), after_gap, True))
            at, after_gap = max(at, g1), True
        pieces.append((at, b, after_gap, False))
        step_max = max(wall["pitch"], filler["pitch"] if filler else 0)
        ends = []                       # each piece's ends as laid, for the gaps between them
        for j, (p0, p1, gap0, gap1) in enumerate(pieces):
            L = p1 - p0
            ends.append((p0, p1))
            if L <= FIT:
                continue
            got, near = fit(L, wall["pitch"], filler["pitch"] if filler else 0)
            if got is None and (gap0 or gap1) and near[0] and L - near[0] <= step_max:
                # A stretch beside a gap is shortened to the next length that closes, from its
                # gap's side: the gap only widens, so the clear width only grows (rule 6).
                cut = L - near[0]
                if gap0 and gap1:
                    p0, p1 = p0 + cut / 2, p1 - cut / 2
                elif gap1:
                    p1 -= cut
                else:
                    p0 += cut
                L = p1 - p0
                got, near = fit(L, wall["pitch"], filler["pitch"] if filler else 0)
                ends[-1] = (p0, p1)
            key = f"{prefix}_w{st['edge']}_{j}"
            if got is None:
                # What bounds the stretch: a corner, a gap (a door or a flight), or neither, which
                # is the edge of a stairwell cut out of this storey (rule 3).
                well = (not gap0 and j == 0 and not st["start_corner"]) or \
                       (not gap1 and j == len(pieces) - 1 and not st["end_corner"])
                opens.append({"key": key, "length": L, "near": near, "side": st["side"], "stairwell": well,
                              "storey": storey_of(storey_key),
                              "at": rnd(local(e, (p0 + p1) / 2, 0))})
                continue
            n, k = got
            if n == 0 and k and L >= wall["pitch"] - FIT:
                notes.append({"code": "fillers_only", "key": key, "length": L, "fillers": k,
                              "at": rnd(local(e, (p0 + p1) / 2, 0))})
            s0 = starts[st["edge"]] + p0
            s_mid = s0 + n * wall["pitch"]
            s1 = starts[st["edge"]] + p1
            common = {"face": outward(e), "offset": round(T / 2000, 3), "ends": "half"}
            # Each bay's piece: the kit's bay, or the variant its place was given (rule 8). A run
            # holds one asset, so a stretch is as many runs as it has spells of one piece; the
            # first keeps the stretch's key, a later one adds the number of its first bay.
            words = bay_variants(n, st["off"] + p0, wall["pitch"], st["side"], storey_of(storey_key), picks, name,
                                 variants)
            pieces_ = [variants[w]["id"] if w in variants else wall["id"] for w in words]
            # One run for the stretch's bays, whatever they are: where they are not all one piece
            # it names each station's (`assets`, rules.md 5, R68), so a bay keeps its name
            # whichever piece it holds.
            bays_end = s_mid if k else s1
            if n:
                run = {"key": key, "count": n, "ref": storey_key,
                       "span": [round(s0 / total, 9), round(bays_end / total, 9)], **common}
                if len(set(pieces_)) == 1:
                    run = {"key": key, "asset": pieces_[0], **{x: run[x] for x in run if x != "key"}}
                else:
                    run["assets"] = pieces_
                runs.append(run)
            if k:
                runs.append({"key": key + "f", "asset": filler["id"], "count": k, "ref": storey_key,
                             "span": [round(s_mid / total, 9), round(s1 / total, 9)], **common})
        # What each gap was left, against what stands in it (rule 6, "the clear gap").
        door_w = kit["door"].get("width", kit["door"]["pitch"]) if kit.get("door") else None
        for j, (g0, g1, label, w) in enumerate(st["gaps"]):
            left = ends[j + 1][0] - ends[j][1] - 2 * over
            need = g1 - g0 - 2 * over
            if label and label[0] == "door" and door_w:
                need = max(need, door_w)
            if left - need > margin + FIT:
                lo = st["gaps"][j - 1][1] if j else a
                hi = st["gaps"][j + 1][0] if j + 1 < len(st["gaps"]) else b
                places = gap_places(lo, hi, need + 2 * over, wall["pitch"], filler["pitch"] if filler else 0,
                                    (g0 + g1) / 2)
                notes.append({"code": "gap_wide", "label": label, "left": left, "need": need, "path": w,
                              "side": st["side"], "places": places, "others": len(st["gaps"]) > 1,
                              "at": rnd(local(e, (ends[j][1] + ends[j + 1][0]) / 2, 0))})
    for c_ in corners:
        piece, r = corner_of(kit, c_["convex"], T)
        if piece is None:
            notes.append({"code": "corner_none", "outside": c_["convex"], "at": c_["at"]})
            continue
        nb, na = c_["before"]["n"], c_["after"]["n"]
        half = r / 2 if c_["convex"] else T / 2       # the box's middle outside, the notch's inside
        p = (c_["at"][0] + (nb[0] + na[0]) * half, c_["at"][1] + (nb[1] + na[1]) * half)
        insts.append({"key": f"{prefix}_c{ring.index(c_['at'])}", "asset": piece["id"], "at": rnd(p),
                      "face": outward(c_["after"])})
    leaf = kit.get("door")
    fes = edges(footprint)
    if leaf is not None:
        for i, d in enumerate(doors):
            fe = fes[d["edge"]]
            insts.append({"key": f"{prefix}_door{i}", "asset": leaf["id"], "at": rnd(local(fe, d["s"], T / 2)),
                          "face": outward(fe)})
    return runs, insts, opens


def inner_walls(openings, footprint, kit, prefix):
    """Instances of wall pieces along each cut between two rooms, from the outer walls' inner face,
    leaving the opening (rule 6)."""
    wall, filler = kit.get("wall"), kit.get("filler")
    insts, opens = [], []
    if wall is None:
        return insts, opens
    T = wall["thick"]
    ax = long_axis(footprint)
    for o in openings:
        a, b = o["ends"]
        L = math.dist(a, b)
        d = ((b[0] - a[0]) / L, (b[1] - a[1]) / L)
        e = {"a": a, "e": d, "n": (-d[1], d[0]), "L": L}
        half = o["width"] / 2
        for j, (p0, p1) in enumerate(((T, L / 2 - half), (L / 2 + half, L - T))):
            length = p1 - p0
            if length <= FIT:
                continue
            got, near = fit(length, wall["pitch"], filler["pitch"] if filler else 0)
            key = f"{prefix}_i{o['rooms'][0]}_{j}"
            if got is None:
                opens.append({"key": key, "length": length, "near": near, "at": rnd(local(e, (p0 + p1) / 2, 0))})
                continue
            n, k = got
            s = p0
            for m, (piece, count) in enumerate(((wall, n), (filler, k))):
                for q in range(count):
                    insts.append({"key": f"{key}_{m}{q}", "asset": piece["id"],
                                  "at": rnd(local(e, s + piece["pitch"] / 2, 0)), "face": bearing(ax)})
                    s += piece["pitch"]
    return insts, opens


# --- the plan of one building -----------------------------------------------------------------


# --- floors and roofs from pieces (rules 9 and 10) ---------------------------------------------

def inset(ring, d):
    """`ring` (counter-clockwise) with every side moved `d` mm inward: each vertex goes to where
    the two moved sides meet. Sides that are parallel at a vertex (a point in a straight wall)
    move it straight in."""
    es = edges(ring)
    out = []
    for i, p in enumerate(ring):
        a, b = es[i - 1], es[i]
        cross = a["e"][0] * b["e"][1] - a["e"][1] * b["e"][0]
        if abs(cross) < 1e-9:
            out.append((p[0] + b["n"][0] * d, p[1] + b["n"][1] * d))
            continue
        # the point q with (q - p).n_a = d and (q - p).n_b = d
        na, nb = a["n"], b["n"]
        det = na[0] * nb[1] - na[1] * nb[0]
        out.append((p[0] + d * (nb[1] - na[1]) / det, p[1] + d * (na[0] - nb[0]) / det))
    return out


def slab_plan(floor, footprint, T, tile, prefix, surface):
    """Floor tiles over `floor` (a storey's ring, or the footprint for a flat roof), inside the
    walls (rule 9). Returns (instances, short): `short` is how many grid cells lie partly on the
    floor and so hold no tile, with the middle of the first.
    The grid is the tile's pitch, from the first corner of side 0 moved in by the wall's
    thickness, along side 0 and square to it. A cell holds a tile when every one of 25 points
    spread over it (`FIT` in from its edges) lies on the floor and inside the walls' inner face.
    On a building whose side 0 does not lie along an axis no tile is laid, and `short` is None:
    tiles are instances, placed on the millimetre lattice, and off the axes two that abut overlap
    by the rounding, which the core takes for two things in one place (rules.md 8.1)."""
    m = tile["pitch"]
    es = edges(footprint)
    e0, n0 = es[0]["e"], es[0]["n"]
    if min(abs(e0[0]), abs(e0[1])) > 1e-12:
        return [], (None, None)                  # turned off the axes: see the docstring
    inner = inset(footprint, T)
    o = inner[0]

    def uv(p):
        return ((p[0] - o[0]) * e0[0] + (p[1] - o[1]) * e0[1], (p[0] - o[0]) * n0[0] + (p[1] - o[1]) * n0[1])

    def at(u, v):
        return (o[0] + e0[0] * u + n0[0] * v, o[1] + e0[1] * u + n0[1] * v)
    us, vs = [uv(p)[0] for p in inner], [uv(p)[1] for p in inner]
    i0, i1 = math.floor((min(us) + FIT) / m), math.ceil((max(us) - FIT) / m)
    j0, j1 = math.floor((min(vs) + FIT) / m), math.ceil((max(vs) - FIT) / m)
    steps = [FIT + k * (m - 2 * FIT) / 4 for k in range(5)]
    insts, short, first = [], 0, None
    for i in range(i0, i1):
        for j in range(j0, j1):
            pts = [at(i * m + a, j * m + b) for a in steps for b in steps]
            on = [inside(inner, p) and inside(floor, p) for p in pts]
            if all(on):
                insts.append({"key": f"{prefix}_f{i - i0}_{j - j0}", "asset": tile["id"],
                              "at": rnd(at((i + 0.5) * m, (j + 0.5) * m)), "face": outward(es[0]), "surface": surface})
            elif any(on):
                short += 1
                first = first or rnd(at((i + 0.5) * m, (j + 0.5) * m))
    return insts, (short, first)


def shift_ring(ring, ds):
    """`ring` (counter-clockwise) with side `i` moved `ds[i]` mm inward (outward where negative):
    each vertex goes to where the two moved sides meet. Whole millimetres."""
    es = edges(ring)
    out = []
    for i, p in enumerate(ring):
        a, b = es[i - 1], es[i]
        na, nb, da, db = a["n"], b["n"], ds[i - 1], ds[i]
        det = na[0] * nb[1] - na[1] * nb[0]
        if abs(det) < 1e-9:
            out.append(rnd((p[0] + nb[0] * db, p[1] + nb[1] * db)))
            continue
        out.append(rnd((p[0] + (da * nb[1] - db * na[1]) / det, p[1] + (na[0] * db - nb[0] * da) / det)))
    return out


def offsets_of(attrs, storeys, footprint, stair_side=None):
    """Per storey, per side, the mm that side stands inside the footprint (outside where
    negative): `setback` and `overhang`, checked (rule 14). Each is `[{ storey, sides, by }]`:
    from `storey` up, those sides stand `by` metres in (a set-back) or out (an overhang)."""
    n = len(footprint)
    out = [[0] * n for _ in range(storeys)]
    for key, sign in (("setback", 1), ("overhang", -1)):
        v = attrs.get(key)
        if v in (None, []):
            continue
        if not isinstance(v, list):
            raise ParamError(key, f"`{key}` is a list of tables", "write it as `[{ storey, sides, by }]`")
        if not rectangle(footprint):
            raise ParamError(key, f"a `{key}` is laid on a rectangle only, and this outline is not one",
                             f"take `{key}` out, or make the building a rectangle")
        for i, e in enumerate(v):
            ok = (isinstance(e, dict) and set(e) <= {"storey", "sides", "by"}
                  and isinstance(e.get("storey"), (int, float)) and not isinstance(e.get("storey"), bool)
                  and e["storey"] == int(e["storey"]) and e["storey"] >= 1
                  and isinstance(e.get("sides"), list) and e["sides"]
                  and all(isinstance(x, (int, float)) and not isinstance(x, bool) and x == int(x) and 0 <= x < n
                          for x in e["sides"])
                  and isinstance(e.get("by"), (int, float)) and not isinstance(e.get("by"), bool)
                  and math.isfinite(e["by"]) and 0 < e["by"] <= 1000)
            if not ok:
                raise ParamError(key, f"{key} {i} is not `{{ storey, sides, by }}`",
                                 "write `storey` as a storey above the ground one (from 1), `sides` as a list of the "
                                 f"outline's sides (0 to {n - 1}) and `by` as metres above 0")
            if e["storey"] >= storeys:
                raise ParamError(key, f"{key} {i} names storey {int(e['storey'])} of a building with {storeys}",
                                 "count storeys from 0")
            sides = sorted({int(x) for x in e["sides"]})
            if any((b_ - a_) % n in (1, n - 1) for a_ in sides for b_ in sides if a_ != b_):
                raise ParamError(key, f"{key} {i} moves two sides that meet at a corner",
                                 "move opposite sides, or one side: a terrace round a corner is not laid")
            for k in range(int(e["storey"]), storeys):
                for x in sides:
                    out[k][x] += sign * int(round(e["by"] * 1000))
    if stair_side is not None:
        for k in range(storeys):
            for x in (stair_side - 1, stair_side, (stair_side + 1) % n):
                if out[k][x]:
                    raise ParamError("setback", f"side {x % n} is moved on storey {k}, and the stair climbs side "
                                     f"{stair_side}: that side and the two it meets keep the building's outline",
                                     "move the side opposite the stair, or put the stair on another side")
    for k in range(storeys):
        for x in range(n):
            if out[k][x] + out[k][(x + 2) % n] >= edges(footprint)[(x + 1) % n]["L"] - 1000 and (out[k][x] or out[k][(x + 2) % n]):
                raise ParamError("setback", f"storey {k} is set back to less than a metre deep",
                                 "set it back less")
    return out


def rectangle(ring):
    """True when `ring` is four sides, each square to the next (`square`)."""
    es = edges(ring)
    return len(ring) == 4 and all(square(es[i - 1], es[i]) for i in range(4))


def gable_plan(footprint, kit, T, prefix):
    """A gable roof over a rectangle (rule 10): (runs, instances), or (None, why) when it cannot
    be laid, `why` a sentence for `roof_none`.
    Slopes: one run of `role:roof_slope` pieces along each long side, from the wall's thickness in
    at one end to the same at the other, their middles half a span in from the eave, fronts out.
    Gables: two `role:roof_gable` halves on each short side, each half a wall's thickness in, the
    first's front out and the second's in, so their high ends meet under the ridge."""
    slope, gable = kit.get("roof_slope"), kit.get("roof_gable")
    if slope is None or gable is None:
        return None, "the kit has no `role:roof_slope` and `role:roof_gable` pieces"
    if not rectangle(footprint):
        return None, "the plan is not a rectangle, and a gable over any other plan is not built"
    es = edges(footprint)
    long = max(range(4), key=lambda i: (es[i]["L"], -i))
    L, D = es[long]["L"], es[(long + 1) % 4]["L"]
    span = slope.get("span")
    if not span or abs(2 * span - D) > FIT:
        return None, (f"the building is {D / 1000:g} m deep and the kit's slope piece is made for "
                      f"{2 * (span or 0) / 1000:g} m (twice its span)")
    if not gable.get("span") or abs(gable["span"] - span) > FIT:
        return None, (f"the gable piece is made for a span of {(gable.get('span') or 0) / 1000:g} m and the "
                      f"slope piece for {span / 1000:g} m")
    eave = L - 2 * T
    n = int(round(eave / slope["pitch"])) if slope["pitch"] else 0
    if n < 1 or abs(n * slope["pitch"] - eave) > FIT:
        below, above = int(eave // slope["pitch"]) * slope["pitch"] + 2 * T, (int(eave // slope["pitch"]) + 1) * slope["pitch"] + 2 * T
        return None, (f"the eave between the gables is {eave / 1000:g} m, no whole number of {slope['pitch'] / 1000:g} m "
                      f"slope pieces: a side of {below / 1000:g} or {above / 1000:g} m would close")
    starts, c = [], 0.0
    for e in es:
        starts.append(c)
        c += e["L"]
    runs, insts = [], []
    for i in (long, (long + 2) % 4):
        runs.append({"key": f"{prefix}_e{i}", "asset": slope["id"], "count": n, "ref": prefix,
                     "span": [round((starts[i] + T) / c, 9), round((starts[i] + L - T) / c, 9)],
                     "face": outward(es[i]), "offset": round(span / 2000, 3), "ends": "half"})
    for i in ((long + 1) % 4, (long + 3) % 4):
        e = es[i]
        out = outward(e)
        insts.append({"key": f"{prefix}_g{i}_0", "asset": gable["id"], "at": rnd(local(e, span / 2, T / 2)),
                      "face": out, "surface": f"{prefix}.garden"})
        insts.append({"key": f"{prefix}_g{i}_1", "asset": gable["id"], "at": rnd(local(e, D - span / 2, T / 2)),
                      "face": round((out + 180.0) % 360.0, 3), "surface": f"{prefix}.garden"})
    return runs, insts


def _num(attrs, key, default, lo=None, integer=False):
    v = attrs.get(key, default)
    try:
        v = int(v) if integer else float(v)
    except (TypeError, ValueError):
        raise ParamError(key, f"`{key}` is not a number", f"write `{key}` as a number")
    if lo is not None and v < lo:
        raise ParamError(key, f"`{key}` is {v}, below {lo}", f"write `{key}` as {lo} or more")
    return v


def plan(name, ring, attrs, ground_m, grid=None, kit=None, inner=INNER_DOOR, margin=MARGIN, kits=None, foot=None):
    """Everything one building returns, as plain data (the step writes it): rules 1 to 14.
    `kit` is one kit for every storey (`kit_pieces`); `kits`, where given, is a function of a
    storey's word and its height in mm that gives that storey's kit, and `kit` is not read.
    `foot` is the ground along the outline, [(x mm, y mm, metres)] (`read_foot`): with it the
    wall's foot is held against the ground (rule 13)."""
    footprint = ccw(ring)
    prefix = key_of(name)
    storeys = _num(attrs, "storeys", 1, 1, integer=True)
    storey_h = _num(attrs, "storey_h", 3.2, 0.5)
    ground_h, top_h = _num(attrs, "ground_h", 0, 0), _num(attrs, "top_h", 0, 0)
    for key, v in (("ground_h", ground_h), ("top_h", top_h)):
        if 0 < v < 0.5:
            raise ParamError(key, f"`{key}` is {v}, below 0.5", f"write `{key}` as 0.5 or more, or 0 for `storey_h`")
    plinth = _num(attrs, "plinth", 0.3, 0)
    porch = int(round(_num(attrs, "porch", 2, 0.5) * 1000))
    walk_out = int(round(_num(attrs, "walk_out", 3, 0) * 1000))
    kinds = storey_kinds(storeys, storey_h, ground_h, top_h)
    heights = [h for _, h in kinds]
    lv = levels(ground_m, plinth, heights) if ground_h or top_h else levels(ground_m, plinth, storey_h, storeys)
    kit_of = [kits(w, int(round(h * 1000))) for w, h in kinds] if kits else [kit] * storeys
    kit = kit_of[0]
    stair = attrs.get("stair") or None
    wall_t = (kit or {}).get("wall", {}).get("thick", 0) if kit and kit.get("wall") else 0
    for k, other in enumerate(kit_of):
        if other and other.get("wall") and abs(other["wall"]["thick"] - wall_t) > FIT:
            raise ParamError("kit", f"the kit's walls are {wall_t / 1000:g} m thick on the ground storey and "
                             f"{other['wall']['thick'] / 1000:g} m on storey {k}: walls of two thicknesses do not stack",
                             "make the bay of every storey one thickness")
    stated = ((kit or {}).get("wall") or {}).get("stated") or {}
    sp = stair_plan(footprint, stair, storeys, wall_t, stated.get("stair"))
    # Each storey's own outline (rule 14): the footprint, with the sides a `setback` or an
    # `overhang` moves. `crown` is the top storey's, which the roof covers.
    moved = offsets_of(attrs, storeys, footprint, sp.get("edge"))
    outline = [shift_ring(footprint, ds) if any(ds) else footprint for ds in moved]
    crown = outline[-1]
    rings, footings = [footprint], {}
    for k in range(1, storeys):
        notch = sp["notches"][k]
        stood = None
        if wall_t and kit_of[k] and kit_of[k].get("wall"):
            n_ = len(outline[k])

            def takes(v):                        # what the corner piece at an outside corner takes of the wall
                a_, b_, c_ = outline[k][v - 1], outline[k][v], outline[k][(v + 1) % n_]
                left = (b_[0] - a_[0]) * (c_[1] - b_[1]) - (b_[1] - a_[1]) * (c_[0] - b_[0]) > 0
                return corner_of(kit_of[k], True, wall_t)[1] if left else wall_t
            stood = footing_plan(outline[k], notch, wall_t, (takes(notch[0]), takes((notch[0] + 1) % n_)))
        if stood:
            rings.append(stood[0])
            footings[k] = {"key": f"{prefix}_s{k}_footing", "region": stood[1], "level": lv[k], "side": notch[0]}
        else:
            rings.append(notch_ring(outline[k], notch))
    out = {"storeys": [], "rooms": [], "flights": [], "doorways": [], "footpaths": [], "runs": [],
           "instances": [], "open": [], "openings": [], "roof": None, "notes": [],
           "footings": [footings[k] for k in sorted(footings)], "kit": kit, "heights": heights, "terraces": []}
    for k in range(storeys):
        out["storeys"].append({"key": f"{prefix}_s{k}", "region": rings[k], "level": lv[k]})
    for path, lo, hi in sp["flights"]:
        path = list(path)
        path[0] = onto(rings[lo], path[0], path[1])
        path[-1] = onto(rings[hi], path[-1], path[-2])
        out["flights"].append({"key": f"{prefix}_f{lo}", "path": path,
                               "starts_at": f"{prefix}_s{lo}.floor", "ends_at": f"{prefix}_s{hi}.floor",
                               "z0": lv[lo], "z1": lv[hi], "width": sp["width"] / 1000})
    res_mm = int((grid or {}).get("resolution_mm") or 1000)
    flat = max(MIN_FLAT, int(math.ceil(res_mm * math.sqrt(2))))
    doors_by_storey = {}
    for i, d in enumerate(attrs.get("doors") or []):
        st = int(d.get("storey", 0))
        if st != 0:
            raise ParamError("doors", f"door {i} is on storey {st}; a door above the ground storey "
                             "needs a balcony, not laid yet", "put the door on storey 0")
        dp = door_plan(footprint, d, porch, walk_out, stated.get("door"))
        if sp.get("edge") == dp["edge"]:
            k = sp["edge"]
            lo = min(n[1] for n in sp["notches"].values())
            hi = max(n[2] for n in sp["notches"].values())
            if dp["s"] + dp["width"] / 2 > lo and dp["s"] - dp["width"] / 2 < hi and k == dp["edge"]:
                raise ParamError("doors", f"door {i} opens into the stairwell",
                                 "move the door or the stair along the wall")
        if kit and kit.get("wall"):
            ring_ = footprint
            k_ = dp["edge"]
            e_ = edges(ring_)[k_]

            def turns_left(v):
                a_, b_, c_ = ring_[v - 1], ring_[v], ring_[(v + 1) % len(ring_)]
                return (b_[0] - a_[0]) * (c_[1] - b_[1]) - (b_[1] - a_[1]) * (c_[0] - b_[0]) > 0
            r0 = corner_of(kit, turns_left(k_), wall_t)[1]
            r1 = corner_of(kit, turns_left((k_ + 1) % len(ring_)), wall_t)[1]
            piece = kit["door"].get("width", kit["door"]["pitch"]) if kit.get("door") else 0
            half = max(piece, dp["width"] + margin) / 2
            lo, hi = r0 + half, e_["L"] - r1 - half
            if dp["s"] < lo - FIT or dp["s"] > hi + FIT:
                raise ParamError("doors", f"door {i} is {dp['s'] / 1000:g} m along side {k_}: its opening "
                                 f"({2 * half / 1000:g} m) runs into a corner, which takes "
                                 f"{(r0 if dp['s'] < lo else r1) / 1000:g} m of that wall",
                                 f"keep its middle between {lo / 1000:g} and {hi / 1000:g} m along the side"
                                 if hi >= lo else "the side is too short for this door: use another side")
        doors_by_storey.setdefault(st, []).append(dp)
        key = f"{prefix}_d{i}"
        dp["doorway"][-1] = onto(rings[st], dp["doorway"][-1], dp["doorway"][0])
        out["doorways"].append({"key": key, "path": dp["doorway"], "ends_at": f"{prefix}_s{st}.floor",
                                "sill_z": lv[st], "flat": round(flat / 1000, 3)})
        if walk_out > 0:
            out["footpaths"].append({"key": f"{prefix}_p{i}", "path": dp["footpath"],
                                     "ends_at": f"{key}.start"})
    for r in attrs.get("rooms") or []:
        st = int(r.get("storey", 0))
        if not 0 <= st < storeys:
            raise ParamError("rooms", f"rooms name storey {st}; the building has {storeys}",
                             f"write a storey from 0 to {storeys - 1}")
        rooms, openings = room_plan(rings[st], outline[st], r.get("shares") or [], inner)
        if st in footings:
            # An inner wall is laid from the outline's faces (`inner_walls` takes the outer wall's
            # thickness off each end): its line is cut from the outline, not from the floor that
            # keeps off the footing.
            try:
                _, openings = room_plan(notch_ring(outline[st], sp["notches"][st]), outline[st], r.get("shares") or [], inner)
            except ParamError:
                pass
        for i, ring_ in enumerate(rooms):
            out["rooms"].append({"key": f"{prefix}_s{st}_r{i}", "region": ring_, "storey": f"{prefix}_s{st}"})
        for o in openings:
            o["storey"] = st
            out["openings"].append(o)
        if kit_of[st]:
            insts, opens = inner_walls(openings, outline[st], kit_of[st], f"{prefix}_s{st}")
            for it in insts:
                it["surface"] = f"{prefix}_s{st}.floor"
            out["instances"].extend(insts)
            out["open"].extend(opens)
    picks = picks_of(attrs, storeys, len(footprint), bool(kit))
    if kit:
        for k in range(storeys):
            crossers = [(d["path"], dp["width"], ("door", d["key"])) for d, dp in zip(
                [d for d in out["doorways"] if d["ends_at"] == f"{prefix}_s{k}.floor"], doors_by_storey.get(k, []))]
            crossers += [(f["path"], int(round(f["width"] * 1000)), ("flight", f["key"])) for f in out["flights"]]
            if not kit_of[k]:
                continue
            if k in footings:
                # Over a stairwell the walls are laid round the outline as on any storey, and the
                # stair's side stands on its footing (rule 3).
                f = footings[k]
                runs, insts, opens = wall_plan(outline[k], outline[k], [], kit_of[k], f"{prefix}_s{k}", f"{prefix}_s{k}",
                                               crossers, margin, out["notes"], picks, name)
                lost = on_footing(runs, insts, outline[k], rings[k], f"{prefix}_s{k}", f["region"], f["key"], f["side"])
                for x in lost[0] + lost[1]:      # nothing under it: said, not laid
                    opens.append({"key": x["key"], "length": 0, "near": (None, None), "storey": k, "unfloored": True,
                                  "at": rnd(x["at"]) if "at" in x else rnd(outline[k][f["side"]])})
            else:
                runs, insts, opens = wall_plan(rings[k], outline[k], doors_by_storey.get(k, []), kit_of[k],
                                               f"{prefix}_s{k}", f"{prefix}_s{k}", crossers, margin, out["notes"],
                                               picks, name)
                for it in insts:
                    it["surface"] = f"{prefix}_s{k}.floor"
            out["runs"].extend(runs)
            out["instances"].extend(insts)
            out["open"].extend(opens)
    if kit:
        for b in picks["bays"]:
            if not b["hit"]:
                out["notes"].append({"code": "bay_none", "index": b["index"], "storey": b["storey"],
                                     "side": b["side"], "at_m": b["at"] / 1000, "variant": b["variant"],
                                     "at": rnd(local(edges(footprint)[b["side"]], b["at"], 0))})
    if kit and kit.get("wall") and foot:         # the wall's foot against the ground (rule 13)
        base = kits("base", None) if kits else {}
        has = (base.get("wall") or {}).get("storey") == "base"
        deep = base["wall"]["high"] / 1000 if has else 0.0
        if has:
            runs, insts, opens = wall_plan(footprint, footprint, [], base, f"{prefix}_base", f"{prefix}_s0", (),
                                           margin, out["notes"], None, name)
            for it in insts:
                it["surface"] = f"{prefix}_s0.floor"
            for o_ in opens:
                o_["base"] = True
            out["runs"].extend(runs)
            out["instances"].extend(insts)
            out["open"].extend(opens)
        hang = [(lv[0] - h, x, y) for x, y, h in foot]
        worst = max(hang)
        bare = [t for t in hang if t[0] - deep > FOOT_SEEN]
        if bare:
            step_ = math.hypot(foot[1][0] - foot[0][0], foot[1][1] - foot[0][1]) / 1000 if len(foot) > 1 else 0.0
            out["notes"].append({"code": "foot_open" if not has else "base_short", "hang": round(worst[0], 3),
                                 "deep": deep, "length": round(len(bare) * step_, 1), "plinth": plinth,
                                 "at": (worst[1], worst[2])})
    for k in range(storeys):                     # floors (rule 9)
        tile = (kit_of[k] or {}).get("floor")
        if not tile:
            continue
        insts, (short, first) = slab_plan(rings[k], outline[k], wall_t, tile, f"{prefix}_s{k}", f"{prefix}_s{k}.floor")
        out["instances"].extend(insts)
        if short is None:
            if not any(n["code"] == "slab_open" and n.get("turned") for n in out["notes"]):
                out["notes"].append({"code": "slab_open", "storey": k, "cells": 0, "at": rnd(outline[k][0]),
                                     "tile": tile["pitch"], "turned": True})
        elif short:
            note = {"code": "slab_open", "storey": k, "cells": short, "at": first, "tile": tile["pitch"]}
            if k in sp["notches"]:
                side, a, b, depth = sp["notches"][k]
                L_ = edges(outline[k])[side]["L"]
                lanes = 1 if (k - 1) % 2 == 0 else 2
                deep, lo, hi = depth - wall_t, max(a, wall_t) - wall_t, min(b, L_ - wall_t) - wall_t
                n_ = max(1, int(round(deep / tile["pitch"])))      # the nearest whole number of tiles

                def whole(v):
                    return min(v % tile["pitch"], tile["pitch"] - v % tile["pitch"]) <= FIT
                if not (whole(deep) and whole(lo) and whole(hi)):       # the stairwell is what is off the tiles
                    note.update({"side": side, "deep": deep, "from": lo, "to": hi, "lane": sp.get("lane"),
                                 "width": (n_ * tile["pitch"] - (lanes + 1) * CLEAR) / lanes, "tiles": n_,
                                 "deep_whole": whole(deep), "long_whole": whole(hi - lo),
                                 "moves": [-(lo % tile["pitch"]), tile["pitch"] - lo % tile["pitch"]]})
            out["notes"].append(note)
    for k in range(1, storeys):                  # terraces (rule 14)
        for i, (now, was) in enumerate(zip(moved[k], moved[k - 1])):
            if now <= was:
                continue
            n_ = len(footprint)
            a_, b_ = outline[k - 1][i], outline[k - 1][(i + 1) % n_]
            b2, a2 = outline[k][(i + 1) % n_], outline[k][i]
            key = f"{prefix}_t{k}_{i}"
            strip = [a_, b_, b2, a2]
            e_in = edges(strip)[0]["n"]          # into the building, across the terrace
            out["terraces"].append({"key": key, "region": strip, "eaves": lv[k]})
            tile = (kit_of[k - 1] or {}).get("floor")
            if tile:
                # Tiles lie inside the parapet on three sides and run to the wall above on the
                # fourth: the ring `slab_plan` insets is the strip grown by `T` under that wall.
                grown = [a_, b_, (b2[0] + e_in[0] * wall_t, b2[1] + e_in[1] * wall_t),
                         (a2[0] + e_in[0] * wall_t, a2[1] + e_in[1] * wall_t)]
                insts, (short, first) = slab_plan(strip, grown, wall_t, tile, key, f"{key}.garden")
                out["instances"].extend(insts)
                if short:
                    out["notes"].append({"code": "slab_open", "storey": k, "cells": short, "at": first,
                                         "tile": tile["pitch"], "terrace": True,
                                         "deep": (now - was) - wall_t})
            pk = kits("parapet", None) if kits else {}
            if (pk.get("wall") or {}).get("storey") == "parapet":
                runs, insts, opens = wall_plan(strip, strip, [], pk, key, key, (), margin, out["notes"], None, name)
                runs = [r for r in runs if "_w2_" not in r["key"]]       # no parapet against the wall above
                for it in insts:
                    it["surface"] = f"{key}.garden"
                for o_ in opens:
                    o_["parapet"] = True
                out["runs"].extend(runs)
                out["instances"].extend(insts)
                out["open"].extend(o_ for o_ in opens if o_.get("side") != 2)
    top = storeys - 1
    roof = attrs.get("roof") or ""
    if not isinstance(roof, str) or roof not in ("", "garden", "flat", "gable"):
        raise ParamError("roof", f"`roof` is {roof!r}", "write \"flat\", \"gable\" or \"garden\", or leave it out")
    eaves = round(lv[top] + heights[top], 3)
    if roof == "garden":
        out["roof"] = {"key": f"{prefix}_roof", "region": crown, "eaves": eaves}
    elif roof in ("flat", "gable"):               # roofs from pieces (rule 10)
        rkey, kt = f"{prefix}_roof", kit_of[top] or {}
        tile, why, laid = kt.get("floor"), None, False
        if not kt.get("wall"):
            why = "the building has no `kit`"
        elif roof == "gable":
            runs, insts = gable_plan(crown, kt, wall_t, rkey)
            if runs is None:
                why = insts
            else:
                out["runs"].extend(runs)
                out["instances"].extend(insts)
                laid = True
        if not laid and kt.get("wall"):
            if tile:
                insts, (short, first) = slab_plan(crown, crown, wall_t, tile, rkey, f"{rkey}.garden")
                out["instances"].extend(insts)
                laid = short is not None
                if short is None:
                    why = (why + "; and " if why else "") + ("the building is turned off the axes, where floor tiles "
                                                             "do not abut to the millimetre and none is laid")
                elif short:
                    out["notes"].append({"code": "slab_open", "storey": -1, "cells": short, "at": first,
                                         "tile": tile["pitch"]})
            elif roof == "flat":
                why = "the kit has no `role:floor` piece"
        if laid:
            out["roof"] = {"key": rkey, "region": crown, "eaves": eaves}
        if laid and roof == "flat" and kits:
            # The parapet (rule 10): the kit's wall pieces tagged `storey:parapet`, laid round the
            # outline as a storey's walls are, on the roof.
            pk = kits("parapet", None)
            if (pk.get("wall") or {}).get("storey") == "parapet":
                runs, insts, opens = wall_plan(crown, crown, [], pk, rkey, rkey, (), margin, out["notes"],
                                               None, name)
                for it in insts:
                    it["surface"] = f"{rkey}.garden"
                for o_ in opens:
                    o_["parapet"] = True
                out["runs"].extend(runs)
                out["instances"].extend(insts)
                out["open"].extend(opens)
        if why:
            cx = sum(p[0] for p in crown) / len(crown)
            cy = sum(p[1] for p in crown) / len(crown)
            out["notes"].append({"code": "roof_none", "roof": roof, "why": why, "flat": laid, "at": rnd((cx, cy))})
    elif kit and kit.get("roof"):
        cx = sum(p[0] for p in crown) / len(crown)
        cy = sum(p[1] for p in crown) / len(crown)
        es = edges(crown)
        longest = max(range(len(es)), key=lambda i: (es[i]["L"], -i))
        out["instances"].append({"key": f"{prefix}_roof", "asset": kit["roof"]["id"], "at": rnd((cx, cy)),
                                 "face": outward(es[longest]), "surface": f"{prefix}_s{top}.floor"})
    return out


def key_of(name):
    """A key from the building's name: characters an id may not hold become `_` (identity.md 2.1)."""
    return "".join(c if c.isalnum() or c in "_-" else "_" for c in str(name))


# --- the step ---------------------------------------------------------------------------------


ATTRS = ["storeys", "storey_h", "ground_h", "top_h", "plinth", "porch", "walk_out", "doors", "stair", "rooms", "faces",
         "setback", "overhang",
         "kit", "roof", "bays", "fill"]


def _list(col):
    return col.tolist() if hasattr(col, "tolist") else list(col)


def _json(v):
    if isinstance(v, str):
        try:
            return json.loads(v) if v else None
        except ValueError:
            return v
    return v


def read_building(step, name):
    fl = step.features(select=[name], columns=ATTRS)
    if len(fl) != 1 or int(_list(fl["geometry"])[0]) != 2:
        raise ParamError("region", f"`{name}` is not one region feature", "write the building as a `region`")
    pieces = fl.rings(0)
    if len(pieces) != 1 or len(pieces[0]) != 1:
        raise ParamError("region", f"`{name}` is not one ring", "write the building as one ring with no hole")
    ring = [(int(v[0]), int(v[1])) for v in _list(pieces[0][0])]
    attrs = {}
    for a in ATTRS:
        if a in fl and (f"{a}?" not in fl or _list(fl[f"{a}?"])[0]):
            attrs[a] = _json(_list(fl[a])[0])
    return ring, attrs


def read_ground(step, ring):
    """The mean terrain height over the outline's posts and vertices (rule 2), metres."""
    try:
        (surface,) = step.fields(["role:surface"], window=[0, 0, 1, 1])
    except (LookupError, RpcError):
        return 0.0
    pts = posts_in(ring, step.grid)
    h = [float(v) for v in _list(step.eval(f"= f('{surface.name}')", x=[p[0] for p in pts],
                                           y=[p[1] for p in pts]))]
    return sum(h) / len(h)


def read_foot(step, ring):
    """The terrain along the outline (rule 13): [(x mm, y mm, metres)] every `FOOT_STEP` along
    each side, from half a step in. Empty where the world has no surface field."""
    try:
        (surface,) = step.fields(["role:surface"], window=[0, 0, 1, 1])
    except (LookupError, RpcError):
        return []
    pts = []
    for e in edges(ccw(ring)):
        n = max(1, int(round(e["L"] / FOOT_STEP)))
        pts += [rnd(local(e, (i + 0.5) * e["L"] / n, 0)) for i in range(n)]
    h = _list(step.eval(f"= f('{surface.name}')", x=[p[0] for p in pts], y=[p[1] for p in pts]))
    return [(p[0], p[1], float(v)) for p, v in zip(pts, h)]


def read_kit(step, taxon):
    if not taxon:
        return None
    t = step.pack(select="@" + taxon, columns=["bounds", "params", "shared"])
    ids = _list(t.get("id", []))
    rows = []
    for r, i in enumerate(ids):
        tags = _json(_list(t["tags"])[r]) if "tags" in t else []
        b = _list(t["bounds"])[r] if "bounds" in t else None
        rows.append({"id": i, "tags": tags if isinstance(tags, list) else [tags],
                     "bounds": [int(v) for v in _list(b)] if b is not None else None,
                     "params": _json(_list(t["params"])[r]) or {} if "params" in t else {},
                     "shared": _json(_list(t["shared"])[r]) or {} if "shared" in t else {}})
    return rows


def write(step, out):
    o = step.out
    for s in out["storeys"]:
        o.feature(s["key"], "storey", region=s["region"], level=s["level"])
    for f in out["footings"]:
        o.feature(f["key"], "footing", region=f["region"], level=f["level"])
    for r in out["rooms"]:
        o.feature(r["key"], "room", region=r["region"], storey=r["storey"])
    for f in out["flights"]:
        o.feature(f["key"], "flight", path=f["path"], starts_at=f["starts_at"], ends_at=f["ends_at"],
                  z0=f["z0"], z1=f["z1"], width=f["width"])
    for d in out["doorways"]:
        o.feature(d["key"], "doorway", path=d["path"], ends_at=d["ends_at"], sill_z=d["sill_z"], flat=d["flat"])
    for p in out["footpaths"]:
        o.feature(p["key"], "footpath", path=p["path"], ends_at=p["ends_at"])
    if out["roof"]:
        r = out["roof"]
        o.feature(r["key"], "roof", region=r["region"], eaves=r["eaves"])
    for r in out.get("terraces") or []:
        o.feature(r["key"], "roof", region=r["region"], eaves=r["eaves"])
    for r in out["runs"]:
        what = {"assets": r["assets"]} if "assets" in r else {"asset": r["asset"]}
        o.table("run", r["key"], **what, count=r["count"], ends=r["ends"], face=r["face"],
                on={"ref": r["ref"], "span": r["span"], "offset": r["offset"]})
    for it in out["instances"]:
        o.instance(it["key"], it["asset"], at=it["at"], face=it["face"], surface=it["surface"])


def generate(step):
    name = step.for_each_item
    try:
        inner = int(round(float((step.params or {}).get("inner_door", INNER_DOOR / 1000)) * 1000))
        margin = int(round(float((step.params or {}).get("margin", MARGIN / 1000)) * 1000))
        ring, attrs = read_building(step, name)
        rows = read_kit(step, attrs.get("kit"))
        kits = (lambda word, height: kit_pieces(rows, word, height)) if rows is not None else None
        ground, foot = read_ground(step, ring), read_foot(step, ring)
        out = plan(name, ring, attrs, ground, step.grid, None, inner, margin, kits, foot)
    except ParamError as e:
        step.error("params", name, f"{name}: {e.message}", e.hint, explain=EXPLAIN["params"],
                   data={"attribute": e.key})
        return
    kit = out["kit"]
    write(step, out)
    for i, w in enumerate(out["open"]):
        below, above = w["near"]
        moves = " or ".join(f"{(v - w['length']) / 1000:+g} m" for v in (below, above) if v is not None)
        hint = (f"it ends at the stairwell: move the stair's `at`, or change its `run`, so this wall is {moves}"
                if w.get("stairwell") and moves
                else "resize the side to a length that closes, or move the door or stair that cuts it")
        where_ = (f" on side {w['side']} of the base" if w.get("base")
                  else f" on side {w['side']} of the parapet" if w.get("parapet")
                  else f" on side {w['side']} of storey {w.get('storey', 0)}" if "side" in w else "")
        said = f"a wall stretch of {w['length'] / 1000:g} m{where_} does not close with the kit's pieces"
        if w.get("unfloored"):
            said = (f"`{w['key']}` on storey {w.get('storey', 0)} has neither the storey's floor nor its footing "
                    "under it: it is not laid")
            hint = "move the stair, or change the plan, so the storey's floor reaches under its walls"
        step.warning("wall_open", name, said, hint,
                     position=(w["at"][0] / 1000, w["at"][1] / 1000),
                     data={"length": round(w["length"] / 1000, 3),
                           "closes": [round(v / 1000, 3) for v in (below, above) if v is not None],
                           **({"side": w["side"], "storey": w.get("storey", 0),
                               "stairwell": bool(w.get("stairwell"))} if "side" in w else {})},
                     **({"explain": EXPLAIN["wall_open"]} if i == 0 else {}))
    for n in (n for n in out["notes"] if n["code"] == "side_short"):
        step.warning("side_short", name,
                     f"side {n['side']} is {n['length'] / 1000:g} m long and its two corners take "
                     f"{n['takes'] / 1000:g} m of it: no wall is laid there",
                     "lengthen the side, or use corner pieces that take less",
                     position=(n["at"][0] / 1000, n["at"][1] / 1000),
                     data={"side": n["side"], "length": round(n["length"] / 1000, 3),
                           "takes": round(n["takes"] / 1000, 3)}, explain=EXPLAIN["side_short"])
    bare = [n for n in out["notes"] if n["code"] == "corner_none"]
    if bare:
        outside = sum(1 for n in bare if n["outside"])
        inside = len(bare) - outside
        step.warning("corner_none", name,
                     f"{len(bare)} corners of the walls have no piece: the kit has none for "
                     + " or ".join(w for w, c in (("an outside corner (`role:corner`)", outside),
                                                  ("an inside corner (`role:corner_in`)", inside)) if c),
                     "give the kit the corner piece, with the `params.reach` it takes from each wall",
                     position=(bare[0]["at"][0] / 1000, bare[0]["at"][1] / 1000),
                     data={"outside": outside, "inside": inside}, explain=EXPLAIN["corner_none"])
    for i, n in enumerate(n for n in out["notes"] if n["code"] == "variant_none"):
        r = n.get("refused")
        step.warning("variant_none", name,
                     (f"the variant `{n['variant']}` is `{r['id']}`, which is not the bay's size ({r['why']}): "
                      "the kit's bay stands in its place" if r else
                      f"the kit has no piece tagged `variant:{n['variant']}`: the kit's bay stands in its place"),
                     ("make the piece the bay's pitch, thickness and height" if r else
                      "tag a `role:wall` piece `variant:" + n["variant"] + "`, or name a variant the kit has"),
                     position=(n["at"][0] / 1000, n["at"][1] / 1000),
                     data={"variant": n["variant"], "asset": r["id"] if r else ""},
                     **({"explain": EXPLAIN["variant_none"]} if i == 0 else {}))
    for i, n in enumerate(n for n in out["notes"] if n["code"] == "bay_none"):
        step.warning("bay_none", name,
                     f"bay {n['index']} asks for `{n['variant']}` {n['at_m']:g} m along side {n['side']} of storey "
                     f"{n['storey']}, where no whole bay stands: a half bay, a corner, a door or no wall",
                     "move `at` onto a bay, or resize the side so a bay stands there",
                     position=(n["at"][0] / 1000, n["at"][1] / 1000),
                     data={"side": n["side"], "storey": n["storey"], "at": n["at_m"], "variant": n["variant"]},
                     **({"explain": EXPLAIN["bay_none"]} if i == 0 else {}))
    only = [n for n in out["notes"] if n["code"] == "fillers_only"]
    if only:
        pitch = kit["wall"]["pitch"] / 1000
        step.warning("fillers_only", name,
                     f"{len(only)} wall stretches a bay or more long hold fillers only: the bay's pitch "
                     f"({pitch:g} m) fits none of them",
                     "resize the sides to suit the bay, or restore the bay's module",
                     position=(only[0]["at"][0] / 1000, only[0]["at"][1] / 1000),
                     data={"stretches": len(only), "fillers": sum(n["fillers"] for n in only), "pitch": pitch},
                     explain=EXPLAIN["fillers_only"])
    for i, g in enumerate(n for n in out["notes"] if n["code"] == "gap_wide"):
        what = f"{g['label'][0]} `{g['label'][1]}`" if g["label"] else "a path"
        over = (g["left"] - g["need"]) / 1000
        places = [round(c / 1000, 3) for c in g.get("places") or () if c is not None]
        hint = (f"put its middle {' or '.join(f'{c:g}' for c in places)} m along side {g['side']} "
                f"(`side = {g['side']}, at = {places[0]:g}`), where the wall either side closes" if places
                else "no place on this side closes round it with the side's other openings where they are: "
                "move them together, or resize the side" if g.get("others")
                else "no place on this side closes round it: make it the kit's door size, or resize the side")
        step.warning("gap_wide", name,
                     f"the wall leaves {g['left'] / 1000:g} m at {what}, which needs {g['need'] / 1000:g} m: "
                     f"{over:g} m stands open", hint,
                     position=(g["at"][0] / 1000, g["at"][1] / 1000),
                     data={"left": round(g["left"] / 1000, 3), "need": round(g["need"] / 1000, 3),
                           "open": round(over, 3), "path": round(g["path"] / 1000, 3),
                           "side": g.get("side"), "closes_at": places},
                     **({"explain": EXPLAIN["gap_wide"]} if i == 0 else {}))
    for i, n in enumerate(n for n in out["notes"] if n["code"] == "slab_open"):
        what = "the flat roof" if n["storey"] < 0 else f"storey {n['storey']}"
        if n.get("turned"):
            said = ("no floor tile is laid: the building is turned off the axes, where tiles placed on the millimetre "
                    "lattice do not abut and would stand in each other's place")
            hint = "keep a building with a tiled floor square to the axes for now"
        elif n.get("terrace"):
            said = (f"{n['cells']} floor tiles of the terrace before storey {n['storey']} cannot be laid whole: it is "
                    f"{n['deep'] / 1000:g} m deep inside its parapet, no whole number of {n['tile'] / 1000:g} m tiles")
            hint = ("a set-back of whole bays leaves a terrace that deep less one wall's thickness: the kit needs a "
                    "tile of the difference, or the terrace is left open there")
        elif "side" in n:
            said = (f"{n['cells']} floor tiles of {what} cannot be laid whole beside the stairwell, which is "
                    f"{n['deep'] / 1000:g} m deep and runs from {n['from'] / 1000:g} to {n['to'] / 1000:g} m along the "
                    f"inside of side {n['side']}: a tile is {n['tile'] / 1000:g} m")
            hint = (f"a stair `width` of {n['width'] / 1000:g} m makes this stairwell {n['tiles']} tiles deep; place it "
                    "and size its `run` so the stairwell starts and ends a whole number of tiles along the wall's inner face")
            if n.get("lane") and not n["deep_whole"]:      # the kit states its lane (rule 11): the depth is the kit's
                hint = (f"the kit's stair lane is {n['lane'] / 1000:g} m, no whole number of its {n['tile'] / 1000:g} m "
                        "tiles: that is the kit's to correct (`shared.stair.lane`)")
            elif n.get("lane") and n["long_whole"]:
                hint = (f"move the stair {n['moves'][0] / 1000:+g} or {n['moves'][1] / 1000:+g} m along the wall (`at`): "
                        "its stairwell then starts and ends on whole tiles")
            elif n.get("lane"):
                hint = (f"the stairwell is {(n['to'] - n['from']) / 1000:g} m long, no whole number of tiles: size the "
                        "stair's `run` so that it and the landing are, and place it (`at`) so the stairwell starts a "
                        "whole number of tiles along the wall's inner face")
        else:
            said = (f"{n['cells']} floor tiles of {what} cannot be laid whole: the floor inside the walls is no whole "
                    f"number of {n['tile'] / 1000:g} m tiles there")
            hint = "size the plan to the kit: each side, less its walls, a whole number of tiles"
        step.warning("slab_open", name, said, hint, position=(n["at"][0] / 1000, n["at"][1] / 1000),
                     data={"storey": n["storey"], "cells": n["cells"], "tile": n["tile"] / 1000,
                           **({"side": n["side"], "deep": n["deep"] / 1000, "width": n["width"] / 1000} if "side" in n else {})},
                     **({"explain": EXPLAIN["slab_open"]} if i == 0 else {}))
    for n in (n for n in out["notes"] if n["code"] == "roof_none"):
        step.warning("roof_none", name,
                     f"the {n['roof']} roof is not laid: {n['why']}"
                     + (". The flat roof is laid in its place" if n["flat"] else ". The building has no roof"),
                     "size the building to the roof pieces, give the kit the pieces, or ask for a roof it can lay",
                     position=(n["at"][0] / 1000, n["at"][1] / 1000),
                     data={"roof": n["roof"], "fallback": "flat" if n["flat"] else ""}, explain=EXPLAIN["roof_none"])
    for n in (n for n in out["notes"] if n["code"] in ("foot_open", "base_short")):
        pos = (n["at"][0] / 1000, n["at"][1] / 1000)
        if n["code"] == "base_short":
            step.warning("base_short", name,
                         f"the ground falls {n['hang']:g} m below the floor and the kit's base is {n['deep']:g} m deep: "
                         f"{n['length']:g} m of the base stops short of the ground, by {n['hang'] - n['deep']:.3g} m at most",
                         "level the ground under the building, lower its `plinth`, or give the kit a deeper base",
                         position=pos, data={"hang": n["hang"], "base": n["deep"], "length": n["length"]},
                         explain=EXPLAIN["base_short"])
        elif n["hang"] <= n["plinth"] + FOOT_SEEN:
            step.info("plinth_bare", name,
                      f"the floor stands {n['hang']:g} m above the ground, its plinth, and the kit has no base: "
                      f"{n['length']:g} m of wall has nothing under it",
                      "give the kit wall pieces tagged `storey:base`, or set `plinth = 0`",
                      position=pos, data={"hang": n["hang"], "length": n["length"]}, explain=EXPLAIN["plinth_bare"])
        else:
            step.warning("foot_open", name,
                         f"the ground falls {n['hang']:g} m below the floor and the kit has no base: {n['length']:g} m "
                         "of wall stands over nothing",
                         "give the kit wall pieces tagged `storey:base`, or level the ground under the building",
                         position=pos, data={"hang": n["hang"], "length": n["length"]}, explain=EXPLAIN["foot_open"])
    step.info("laid", name,
              f"{len(out['storeys'])} storeys, {len(out['rooms'])} rooms, {len(out['flights'])} flights, "
              f"{len(out['doorways'])} doorways, {len(out['runs'])} wall runs, {len(out['instances'])} pieces",
              "none needed", explain=EXPLAIN["laid"],
              data={k: len(out[k]) for k in ("storeys", "rooms", "flights", "doorways", "footpaths", "runs",
                                             "instances")})
