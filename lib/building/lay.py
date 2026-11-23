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
   `role:corner_in`, `role:filler`, `role:door`, `role:roof`, `role:floor`, `role:roof_flat`,
   `role:roof_slope`, `role:roof_gable`): along every stretch of a storey's outline that lies on the footprint's
   outline, less the corners and a clear gap for each doorway and flight that crosses it, one `run` of wall pieces at their pitch
   (`params.pitch`, else their `bounds` width) and one of fillers for what is left: the most wall
   pieces such that the rest is a whole number of fillers, the stretch no more than `FIT`
   (5 mm) longer than them and no more than `FIT_SHORT` (1.5 mm, what rounding two corners to
   the millimetre takes off a side) shorter (`fit`). A stretch that does
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
   gap widens; that length may be none, and a stretch shorter than any piece of the kit, between a
   corner and a door, goes, the gap running to the corner's face (K10: Kenney's door block in the
   bay beside a corner left 0.325 m open). Nothing longer than the kit's smallest piece goes this
   way: such a stretch closes or is `wall_open`. What the wall then leaves, less what the gap needs (the path's width and the
   margin, or the kit's door piece, `role:door`, where that is wider), is open wall. More than the
   margin of it is `gap_wide` (warning), naming the door or flight and the metres left open: a
   door may sit a margin loose in its wall, not a piece's length. **Each side on its own.** The
   kit's door piece stands on the door's own middle, and a gap widened on one side leaves it off
   the opening's middle: where the wall left open on one side of the piece as it stands is more
   than the margin, and the two sides differ, that is `gap_wide` too, with the metres open before
   and after the piece, though the widths alone would pass (K10's review: a village door 1.24 m
   along its side left 0.19 m open beside the door piece and said nothing; a sweep of door places
   over six kits found 240 such places in the village kit and 264 in Kenney's even before K10).
   The open wall is measured between what stands: the faces of the wall pieces, a corner's face
   where a stretch went, and the door piece's box. The kit's door piece is its
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
     there are no weights. With `mirror = true` the pattern is read from both ends of the side
     toward its middle: a bay takes the word at the number of bays between it and the nearer end
     of its side, so the two halves of a side are each other's image, and so are the two walls
     either side of a door on its axis (read from the stretch's start, the second wall was the
     first one moved along, not turned about). A choice in a mirrored pattern is made from that
     number too, so both halves choose alike; the two bays at the middle of an even side then
     hold one piece. The halves are images only where the side's bays stand as images about
     its middle: a side that a half bay closes at one end, or that a door cuts off its middle,
     is laid by the same reading and said, `mirror_none` (warning).
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
   of its own height: for each of `wall` (its bays), `filler`, `corner`, `corner_in`, `door` and `post`,
   the pieces nearest the storey's height stay (within `FIT` of the nearest) and the role's
   others are left out; where the nearest is not the storey's height `check` says `kit_height`. A piece tagged
   `storey:ground`, `storey:upper` or `storey:top` is for that kind of storey only, and there it
   is read before an untagged piece of its role. The bays of all storeys are one thickness, or
   the building is refused (`params`). **Names.** A run is `<storey>_w<side>_<j>`: the outline's
   side and the number of the stretch a door cuts it into; a corner piece `<storey>_c<vertex>`.
   On a storey whose stairwell is cut from the outline (no footing) the numbers are its own
   ring's edges and vertices.
7. **Roof.** `roof = "garden"`: a `roof` region over the footprint whose `garden` is at the
   eaves. `"flat"` and `"gable"` are rule 10, `"hip"` rule 17. With none said and a `role:roof` piece in the kit,
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
   feature over the footprint (`<building>_roof`). `roof = "flat"`: the kit's roof tile
   (`role:roof_flat`), or its floor tile where it has none, laid as rule 9 lays a floor over the
   whole footprint (`<building>_roof_f<i>_<j>`); the walls' tops stand round them. A roof tile is
   its own role because a kit's roof is not its floor: Kenney's flat roof tile stands its
   thickness proud of the wall's top, where its parapet's flat part is, and a floor tile is sunk
   by its thickness; a kit made only of a building's outside has no floor at all (K10). A
   terrace (rule 14) is tiled the same way. `roof = "gable"`, on a rectangle: the ridge lies along the longer sides (the first of
   equal ones). `role:roof_slope` is a wedge that climbs from its front edge to its back,
   `params.pitch` long along the eave and `params.span` deep in plan; the building's depth must be
   twice the span within `FIT`. One run of slopes stands along each long side
   (`<building>_roof_e<side>`), from `T` in at one end to `T` in at the other, which must be whole
   pieces, their middles half a span in from the eave and their fronts out. `role:roof_gable`
   is half a gable: a wall `T` thick, one `params.span` long, low at its +x end and as high as
   the slope at its -x end. Two stand on each short side (`<building>_roof_g<side>_0`, `_1`),
   half a span and a span and a half along it, the first's front out and the second's in, so
   their high ends meet under the ridge. A rectangle is four sides each square to the next
   (`square`), turned or not. **Over an L** (six sides, each square to the next, one inside
   corner, both wings twice the span deep): the square where the wings cross is mitred along
   its diagonal from the outer corner to the inside one. `role:roof_hip`, one span square with
   eaves on its front and its +x side, stands on the outer corner (`<building>_roof_hip`), its
   front out of the outer eave that starts there. `role:roof_valley` is placed by the inside
   corner itself (`<building>_roof_valley`), its front out of the side that follows the
   corner: the quarter of the roof beside the corner, where the two inner slopes meet along
   the valley, and `params.reach` of each inner eave beyond it. Each of the four eaves then
   holds one run of slopes: an outer eave from one span past the outer corner to `T` short of
   its wing's free end, an inner eave from the valley's reach to the same; and two gable
   halves stand on each free end. Each run must be whole pieces, or the roof is refused with
   the lengths that would close. A roof that cannot be laid (no kit; a flat roof on a building
   turned off the axes, as rule 9; for a gable: a plan that is neither a rectangle nor an L, a depth that is not twice a slope's span, an eave that is not whole pieces, a
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
   is where it was on every storey: since a building of two storeys has a stair, the side that
   can move is the one opposite the stair. Each is `params` (error) otherwise. A bay's `at`
   (rule 8) on a moved storey is measured along that storey's own side. **The terrace.** Where a
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
15. **Posts and bays.** A kit is laid by posts when it has a `role:post` piece of its bay's own
   group (both tagged `storey:<word>` alike, or neither tagged). The bay's pitch `B`
   (`params.pitch` of the `role:wall` piece) is then the distance from one post's middle to the
   next, the post is `p` wide (its measured box), and the wall piece is a panel `B - p` long
   that stands between two posts: `check` holds it to that (`kit_pitch`). A corner is a post
   (the kit's `role:corner`, else its post), reaching `p`. A side from corner to corner closes
   when it is `N * B + p` long; any other is `wall_open`, with the two lengths next to it that
   close. There are no fillers: `N` bays have `N + 1` posts, and a kit that walls with a bay
   and a filler has one bay on every side that is not a bay. A stretch is laid as two runs.
   The panels are one run of `N` over the stretch widened by half a post at each end
   (`<storey>_w<side>_<j>`): a run's members stand evenly over its span (rules.md 5.1), so a
   piece shorter than its pitch leaves the post its room. The posts between two bays are a
   second run of `N - 1` (`<storey>_p<side>`). Both stand `p / 2` in from the outline. A door
   stands in one bay: its doorway must lie clear between that bay's two posts, and where the kit
   has a `role:door` piece (a frame `B - p` long) its middle must be the bay's middle, since
   the piece takes the panel's place; otherwise it is `params` (error), naming the middles of
   the side's bays. The panel run splits at a door, and every post still stands. `fill` and
   `bays` (rule 8) choose every panel, the last too. **The floor.** A storey walled by posts is
   tiled (rule 9) to the middle of its line of posts, half a post in from the outline, not to
   the panels' inner face: a bay is from one post's middle to the next, and so is a floor of
   whole tiles. **The wall's band** is the post's width, not the panel's thickness: a stair
   keeps off it (rule 3's `T` is `p`, so the flight's lane starts at the posts' inner face),
   the footing over a stairwell is as deep, and the floor over that footing is still tiled to
   the post line's middle. A flight beside a wall of posts therefore crosses nothing; one that
   does cross the wall takes the panels and posts it crosses. An end of a stretch that is no corner (beside a stairwell cut
   from the outline) has its own post, `<storey>_p<side>_a` or `_b`. An outline that turns in
   is refused (`params`): the post in an inside corner is not laid yet. Source: ours; the
   research found no developer's account of post-and-bay building, and reasons only that a
   visible member on every module line is where a kit hides its seams (docs/research/Modular
   kit failures in studios.md, section 12).
16. **The colonnade.** `colonnade = [{ sides, depth }]` (`sides`: `"all"` or a list of the
   outline's sides): on those sides the walls stand `depth` metres inside the building's
   outline, from the ground storey, by rule 14's mechanism; all four may move. On a building of
   one storey and a rectangular plan only (`params` otherwise). The ground storey's region is
   then the walls' outline, the room inside, and every rule above is laid on it. The
   building's whole outline is a `platform` feature (`<building>_platform`, kinds.toml), one surface
   at the ground storey's level, and what stands outside the walls stands on it. The room's
   storey lies over part of it at the same height: two surfaces at one height are one level
   (R70), so nothing on the one is blind to the other. **The posts.** Along the outline where
   the walls left it, the kit's pieces tagged `storey:colonnade` are laid by rule 15
   (`<building>_col_...`): a side with a colonnade from corner to corner, and on a side the
   walls keep, the platform's end beside it, from the footprint's corner to the face of the wall's
   own corner, which is whole bays when `depth` is: where it is not, `wall_open` names the
   `depth`s that close it, since the side's length does not. A `depth` less than the
   colonnade's post is wide is `params`. The lintel is not cut for a doorway. A kit
   with no such post and lintel is `colonnade_none` (warning): the walls still stand inside
   and the roof still covers the outline. **Doors.** A door on a side with a colonnade is two
   doorways on one axis: `<building>_d<i>` comes from outside to the outline's edge and ends on
   the platform, and must be clear of the colonnade's posts (a front with an odd number of
   bays has a bay on its axis, one with an even number a post);
   `<building>_d<i>_in` starts on the platform a metre before the wall (the colonnade's depth
   where that is less) and ends on the room's floor, and the wall has its opening and the
   kit's door piece there (rule 6). So the door is held twice on one axis, by the colonnade's
   posts and by the wall behind (its corners, or its own bays where it is laid by posts), and
   one `params` (error) says which, in the measure the door was written in, along the
   building's side, and names only the places both take, or says that the side has none. A door on a side the walls keep is one doorway, as before,
   placed along the building's side, and said in that measure too. A door beside the room, where no wall stands behind the
   colonnade, is `params`. **The floor, the base, the roof.** The platform is tiled (rule 9) over
   the outline, inside the kit's base (its thickness in; with no base, the colonnade's post's
   width in, or half of it where the room's own walls are laid by posts), less the room, whose floor its storey tiles; a cell partly on the room is
   `slab_open`, said of the platform, with the `depth` or the side's length that would be whole
   tiles. Where the room is walled by posts the platform's tiles
   run to the middle of that line of posts, as the room's do from inside (rule 15), and with
   no base they start on the middle of the colonnade's own line of posts, so a veranda one
   bay deep is whole tiles where a bay is; a base moves that start to its own inner face, and
   the veranda is whole tiles only where the base is half a post thick (examples/court's is). The base
   (rule 13) goes round the building's outline, on the platform. The roof (rule 10) covers the
   outline, and its ends stand a post's width in, over the colonnade, not over the walls.
   Source: ours; the research found no developer's account of a colonnade, a porch or a
   portico built from pieces (the same report, section 12).
17. **The roof beyond the walls.** Three things a roof that is its kit's main piece needs, each
   on a rectangle. **Hipped.** `roof = "hip"`: a `role:roof_hip` piece (rule 10: one span
   square, its eaves on its front and its +x side) stands on each of the roof's four corners,
   at vertex `i` with its front out of side `i` (`<building>_roof_h<i>`), and one run of
   `role:roof_slope` pieces along each side between its two hips (`<building>_roof_e<side>`),
   which must be whole pieces. The roof's depth is twice the span, so the two hips of a short
   side meet and no slope stands between them; a square roof is four hips. **Eaves.** A kit
   that states `shared.eaves` has a roof that stands that far beyond the building's outline on
   every side: a gable or a hipped roof is laid on the outline grown by the eaves, and its
   `roof` feature has that region. So the roof's pieces are made for a building's depth and
   the eaves together: twice the span is the depth and twice the eaves. Under eaves a gable's
   slopes run the roof's whole length, past the wall's end, and its two gable halves stand the
   eaves in from each end, over the wall, each made for the slope's span less the eaves (the
   kit makes it to fill from the wall's top to the slopes); a gable piece of any other span is
   `roof_none`, and so is a gable under eaves over an L. A flat roof and a `garden` have no
   eaves. **Pieces by span.** A kit may hold a slope, and a hip or a gable half, for each of
   several depths of building. Of several `role:roof_slope` pieces of different spans the
   first whose span is half the roof's depth is taken, and of each other roof role the first
   piece of that span (of gable halves under eaves, of that span less the eaves); where none is made for the depth it is `roof_none`, naming the depths
   the kit's slopes roof. With one span in the kit the piece is taken as before, and rule 10
   says what does not fit. Any of the three that cannot be laid is `roof_none` with its reason
   and the flat roof in its place where the kit has a tile, as rule 10. `check` holds a hip to
   its span, a slope to an end of its span, and each slope under stated eaves to a depth a
   building of the kit can have (`kit_deck`, `kit_stated`). Source: ours; the research found
   one mention of a separate node for layered roofs and no account of deep eaves or hipped
   roofs from pieces (docs/research/Modular kit failures in studios.md, section 12).
18. **A wall on the ground.** `storeys = 0`: the building is its walls and nothing else, a yard.
   It has no storey, so no floor, no roof and no level: its outline is a `yard` feature
   (`<building>_yard`, kinds.toml), which owns no surface, and the kit's walls are laid round
   it by rule 6 (or 15) on the terrain, each piece on the ground under it, so on a slope the
   wall follows the land where a storey's wall would stand level over it. `storey_h` is the
   height the kit's pieces are chosen by. A door is a gap in the wall with the kit's door
   piece in it, a gate: no doorway and no footpath are returned, since no floor's sill has to
   be met and the ground runs through. `fill` and `bays` choose its bays as on storey 0.
   `stair`, `rooms`, `roof`, `colonnade`, `setback` and `overhang` are `params` (error), and so
   is a building of no storeys with no kit. Why not one storey: a storey is a level floor over
   the whole outline, a second surface across a court at the mean of its ground, drawn as a
   slab and standing under every hall in it (examples/court, "Before"). Source: ours; a walled
   compound with a gate is named in no developer's account read (docs/research/Modular kit
   failures in studios.md, section 12).
19. **Level ground.** `ground = "level"`: the building also returns a `plot` (kinds.toml),
   `<building>_plot`: its outline moved out `PLOT_APRON` (1.5 m) on every side, with `level`
   the ground rule 2 read, so the land under the building and a walk round it is held at the
   level its floor was reckoned from, and banks to the land outside no steeper than the plot's
   `grade`. The wall's foot (rule 13) is then judged against that level, not against the land
   as this step read it: the base reaches the ground where it is as deep as the plinth is
   high. Levelling that would move the ground more than the plot's `max_cut` is the core's
   `authority.max_change`, naming the plot: the land is too steep for this building as it is
   written. The building's `max_cut` (3 m) and `grade` (0.3) are its plot's, passed on, so
   that record is answered on the building: a larger `max_cut`, or gentler ground. A plot is
   shaped with `order = 1`, so a way's bed in any layer gives way to it, and a doorway's sill,
   ordered after it (kinds.toml), holds on it. What this step cannot see is what shapes the
   ground after it: an anchored kind, a second plot over the same ground (another building's,
   returned by this step in the same layer), or anything ordered after it. The wall's foot is
   judged here against the level the plot states; `check` judges it again against the ground
   as it ended (check.py rule 12, `plot_lost`). Any other
   word is `params`. A `row` does not take it (its houses step with the land, rule 13); a
   `court` takes it for the whole court (court.py rule 9), and its halls then read level
   ground and need none of their own. Source: ours; in the studios' accounts flat ground is a
   precondition a terrain team meets by hand (docs/research/Modular kit failures in
   studios.md, section 4, "Meeting the ground"). Asked by examples/realm, the first kit
   buildings on land that was not written for them.
11. **What a kit states.** A kit is a taxon's pieces, and what they share it says once: the
   `shared` table in that taxon's `defaults` (spec/assets.md 2.5; R66 of docs/doctrine.md; Asset
   law 5 of docs/ledger/archetypes.md). `lay` reads these of its keys, each optional: `wall`, the
   walls' thickness, which is then `T` in every rule above in place of the bay's measured box;
   `stair = { lane, width, run, landing }`, the kit's stair: `width` and `run` are a building's
   stair's where it writes none, `landing` replaces `LANDING`, and `lane` is how deep one lane
   is (lane 0's centre is `T + lane / 2` in and its stairwell `lane` deep, lane 1 one `lane`
   further), in place of the width and `CLEAR` each side, so a kit whose lane is whole floor
   tiles gets stairwells that are; `door`, the width of the doorway its door piece is made for,
   which a door that writes no `width` takes; `module`, `storey` (one height or a list) and
   `parapet` and `base` (the heights of its `storey:parapet` and `storey:base` pieces),
   which `lay` does not lay by (a stretch closes by its pieces' pitches, a storey's height is
   the building's) and `check` holds the pieces to (`kit_stated`); and `unit`, the length of
   which every size of the kit is a whole number (a classical kit's column width), which
   `check` holds every piece to (`kit_unit`); and `eaves`, how far the kit's roof stands
   beyond the walls on every side (rule 17), which `lay` lays the roof by. `module` is the bay, as it has been since K6; a
   kit whose sizes all come from something smaller than its bay states that as `unit`. What a piece alone takes of a
   layout stays on the piece: `params.pitch`, `reach`, `span` (R69). A kit that states nothing
   is laid by its pieces as before, and `check` says so once (`kit_unstated`, info). Two
   `shared` tables among a kit's rows, or a value of one of these keys that is no length
   above 0, is `params` (error); a key not in this list is another library's, and is passed over. Source: the studios' accounts of a kit as a system and not a
   list of assets, and of the metrics written down before the first piece (docs/research/
   Modular kit failures in studios.md, sections 1 and 2).

`params` of the step: `inner_door` (0.9 m), `margin` (0.1 m). Codes are `step.<id>.<code>`: `params` (error, the
building's attribute or a kit piece's param), `wall_open`, `side_short`, `corner_none`,
`fillers_only`, `gap_wide`, `variant_none`, `bay_none`, `slab_open`, `roof_none`, `foot_open`,
`base_short`, `colonnade_none`, `mirror_none` (warnings), `plinth_bare` and `laid` (infos; `laid` is what was returned).

Library code imports only viasdk and the standard library (R19b).
"""

import hashlib
import json
import math

from viasdk import RpcError

CLEAR = 100          # mm between a flight and the wall, and between two lanes
LANDING = 1000       # mm of stairwell before an even flight's start and past an odd one's
STUB_MIN = 300       # mm: the least wall a stairwell may leave before a corner (ours: a corner piece)
FIT = 5              # mm a stretch may be longer than its pieces' pitches and still close
FIT_SHORT = 1.5      # mm it may be shorter: what rounding its two corners to the millimetre can take off a side
FOOT_STEP = 500      # mm between the points the ground is read at along an outline (rule 13)
FOOT_SEEN = 0.05     # metres of wall foot above the ground that is said (rule 13)
PLOT_APRON = 1500    # mm a building's plot reaches beyond its outline (rule 19)
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
    "mirror_none": "A mirrored fill reads its pattern from both ends of a side toward its middle. The two halves "
                   "are images only where the bays themselves are: a side closed by a half bay at one end, or "
                   "cut by a door off its middle, has bays that face no bay across the middle, and a pattern "
                   "cannot make an image of pieces that do not stand as one.",
    "colonnade_none": "A colonnade is the kit's own pieces: a post, and the lintel that spans from one post "
    "to the next, tagged `storey:colonnade` so no storey's wall takes them. With neither, the walls still stand "
    "inside the outline and the roof still covers it, on nothing.",
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


def fit(length, pitch, filler, tol=FIT, short=FIT_SHORT):
    """The most wall pieces `n` and fillers `k` that close a stretch of `length` (all mm), or
    None and the nearest lengths below and above that would close. A stretch closes when it is
    no more than `tol` longer than its pieces and no more than `short` shorter. The two are not
    one number, because they are two things. Longer, the pieces stand a hair apart: `FIT`, 5 mm
    over a whole stretch, which no one sees and a kit's own export rounds by. Shorter, they
    stand in each other: the only shortness let pass is the lattice's own, an outline turned
    off the axes whose two corners are each rounded to the millimetre (0.71 mm each at most, so
    `FIT_SHORT`, 1.5 mm). A stretch shorter than that is `wall_open` with its length, as any
    stretch that does not close: before K8's review it closed up to 5 mm short and the only
    word was the core's, of pieces in each other's place."""
    best = None
    for n in range(int((length + short) // pitch), -1, -1):
        rest = length - n * pitch
        k = int(round(rest / filler)) if filler else 0          # `rest` is never below `-short`
        if -short <= rest - k * (filler or 0) <= tol:
            best = (n, k)
            break
    if best is not None:
        return best, None
    closes = sorted({n * pitch + k * (filler or 0) for n in range(int(length // pitch) + 2)
                     for k in (range(0, int(length // filler) + 2) if filler else (0,))})
    below = max((c for c in closes if c < length), default=None)
    above = min((c for c in closes if c > length), default=None)
    return None, (below, above)


def closes(length, n, pitch):
    """True where `n` pieces of `pitch` close `length` as `fit` closes a stretch: up to `FIT`
    longer than the pieces, up to `FIT_SHORT` shorter."""
    return -FIT_SHORT <= length - n * pitch <= FIT


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


BY_HEIGHT = ("wall", "filler", "corner", "corner_in", "door", "post")  # the roles a storey's height chooses among
SHARED = ("module", "unit", "wall", "door", "storey", "parapet", "base", "stair", "eaves")  # what a kit may state once (rule 11)
STAIR_KEYS = ("lane", "width", "run", "landing")
ROOF_ROLES = ("roof_slope", "roof_gable", "roof_hip", "roof_valley")   # the roles a roof's depth chooses among (rule 17)


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
    out, walls, roofs = {}, [], {}
    rows = kit_rows(rows, word, height)
    for r in sorted(rows, key=lambda r: (bool(words(r)), not any(t.startswith("storey:") for t in r["tags"]),
                                         r["id"])):
        for tag in r["tags"]:
            if not tag.startswith("role:"):
                continue
            role = tag[5:]
            if role == "wall":
                walls.append(r)
                continue
            if role in ROOF_ROLES:               # every roof piece, for the roof's depth to choose (rule 17)
                roofs.setdefault(role, []).append(piece(r))
            if role not in out:
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
    if roofs:
        out["roofs"] = roofs
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
        ok = ok and isinstance(f.get("mirror", False) if isinstance(f, dict) else False, bool)
        extra = sorted(set(f) - {"storeys", "sides", "faces", "pattern", "mirror"}) if isinstance(f, dict) else []
        if not ok or extra:
            raise ParamError("fill", f"fill {i} is not `{{ storeys, sides, pattern }}`"
                             + (f": it has `{extra[0]}`" if extra else ""),
                             "write `storeys` and `sides` as \"all\" or lists of whole numbers (or `faces` as a "
                             "list of the words the building's `faces` gives its sides), and `pattern` "
                             "as a list of variant words (\"wall\" for the kit's bay), or lists of words to choose "
                             "among, each word once; `mirror` is true or false")
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
        out["fill"].append({"storeys": f.get("storeys", "all"), "sides": f.get("sides", "all"), "pattern": pattern,
                            "mirror": bool(f.get("mirror", False))})
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


def bay_variants(n, first, pitch, side, storey, picks, name, variants=None, side_len=None, lead=0):
    """The variant word of each of the `n` bays of one stretch, None for the kit's bay (rule 8).
    `first` is where the first bay starts, mm along the outline's side `side`; `variants` the
    words the kit can give (`kit_pieces`), by which two words are known to lay one piece: a word it
    cannot give lays the bay, as `"wall"` does (with None, every word is its own piece).
    A `fill` gives each bay the word of its pattern at the bay's number in the stretch. A list of
    words there is a choice made from the bay's place (the building's name, its storey, its side
    and how far along it stands), so that two bays that touch do not take one piece where the
    words allow it; rule 8 says how. A `bays` entry then names one bay by a point along the side.
    A `fill` that says `mirror` reads its pattern from both ends of the side toward its middle:
    a bay's number is then how many bays stand between it and the nearer end, counted from
    `lead` mm in from that end (where the side's first bay starts) on a side `side_len` mm long,
    and a choice is made from that number, so the two halves agree."""
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
        mirror = bool(f.get("mirror")) and side_len is not None

        def ix(i, mirror=mirror):
            if not mirror:
                return i
            c = place[i] + pitch / 2
            return max(0, int((min(c, side_len - c) - lead + FIT) // pitch))

        def fixed(i, pat=pat, ix=ix):
            return pat[ix(i) % len(pat)] if 0 <= i < n and isinstance(pat[ix(i) % len(pat)], str) else None
        cands, raw = [None] * n, [None] * n
        for i in range(n):
            item = pat[ix(i) % len(pat)]
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
                raw[i] = c[((ix(i) if mirror else place[i] // int(pitch)) + _hash(name, storey, side)) % 2]
            else:
                raw[i] = c[_hash(name, storey, side, ix(i) if mirror else place[i]) % len(c)]
        final = list(raw)
        for i in range(1, n):
            c = cands[i]
            if mirror or c is None or len(c) < 3 or piece(final[i - 1]) != piece(raw[i]):
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


def mirror_held(spans, picks, storey, fes, notes):
    """Rule 8: a mirrored `fill` makes a side's two halves images only where the bays themselves
    stand as images about the side's middle. `spans` is {side: [(a, b)]}: where each bay laid
    stands, mm along its side (a bay a door or a flight took is not among them, so the bay
    across the middle from it faces none, unless it is the middle bay itself). A side under a
    mirrored fill on which a bay faces no bay is `mirror_none`: a half bay closes one end, or
    a door stands off the middle. The pattern is still laid."""
    for side, bays in sorted(spans.items()):
        if not any(f.get("mirror") and (f["storeys"] == "all" or storey in f["storeys"])
                   and (f["sides"] == "all" or side in f["sides"]) for f in picks["fill"]):
            continue
        L = fes[side]["L"]
        if not all(any(abs(a - (L - b2)) <= 2 * FIT and abs(b - (L - a2)) <= 2 * FIT for a2, b2 in bays) for a, b in bays):
            notes.append({"code": "mirror_none", "side": side, "storey": storey, "at": rnd(local(fes[side], L / 2, 0))})


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
    if by_posts(kit):                       # a kit of posts and bays (rule 15)
        return post_walls(ring, footprint, doors, kit, prefix, storey_key, crossers, margin, notes, picks, name)
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
    held = {}                           # {side: where each piece of its wall stands}, for a mirrored fill
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
        ends = []                       # each piece's ends as laid, for the gaps between them
        for j, (p0, p1, gap0, gap1) in enumerate(pieces):
            L = p1 - p0
            ends.append((p0, p1))
            if L <= FIT:
                continue
            got, near = fit(L, wall["pitch"], filler["pitch"] if filler else 0)
            if got is None and (gap0 or gap1) and near[0] is not None:
                # A stretch beside a gap is shortened to the next length that closes, from its
                # gap's side: the gap only widens, so the clear width only grows (rule 6). That
                # length may be none: a stretch shorter than any piece, between a corner and a
                # door, goes, and the gap runs to the corner (K10: a door block in a corner's bay).
                # The cut is always less than the shortest piece: two lengths that close are never
                # further apart than that, so no bound is written (K10's review found the old
                # bound, `L - near[0] <= step_max`, could not fail: two mutations of it survived).
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
                                 variants, edges(footprint)[st["side"]]["L"], st["off"] + a)
            pieces_ = [variants[w]["id"] if w in variants else wall["id"] for w in words]
            # One run for the stretch's bays, whatever they are: where they are not all one piece
            # it names each station's (`assets`, rules.md 5, R68), so a bay keeps its name
            # whichever piece it holds.
            bays_end = s_mid if k else s1
            at_ = st["off"] + p0        # each bay laid, by where it stands along the side (half bays stand after them)
            held.setdefault(st["side"], []).extend((at_ + i * wall["pitch"], at_ + (i + 1) * wall["pitch"]) for i in range(n))
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
            # The faces either side: a wall piece's overhang reaches past its stretch's end; a
            # stretch that went (shorter than any piece) leaves the corner's own face.
            f0 = ends[j][1] + (over if ends[j][1] - ends[j][0] > FIT else 0)
            f1 = ends[j + 1][0] - (over if ends[j + 1][1] - ends[j + 1][0] > FIT else 0)
            left = f1 - f0
            need = g1 - g0 - 2 * over
            if label and label[0] == "door" and door_w:
                need = max(need, door_w)
            # Where the kit's door piece stands (on the door's own middle, as laid below): the
            # open wall either side of it, each side on its own, so a piece that stands off the
            # opening's middle leaves no hole unsaid (K10's review: a stretch that went beside a
            # door widened the opening on one side only).
            beside = None
            if label and label[0] == "door" and kit.get("door") and kit["door"].get("box"):
                bx = kit["door"]["box"]
                for d in doors:
                    c_ = d["s"] - st["off"]
                    if d["edge"] == st["side"] and g0 - FIT <= c_ <= g1 + FIT:
                        ha, hb = (c_ - bx[3]) - f0, f1 - (c_ - bx[0])    # asset +x runs against the side
                        if abs(ha - hb) > FIT and max(ha, hb) > margin + FIT:
                            beside = (max(ha, 0), max(hb, 0))
                        break
            if left - need > margin + FIT or beside:
                lo = st["gaps"][j - 1][1] if j else a
                hi = st["gaps"][j + 1][0] if j + 1 < len(st["gaps"]) else b
                places = gap_places(lo, hi, need + 2 * over, wall["pitch"], filler["pitch"] if filler else 0,
                                    (g0 + g1) / 2)
                notes.append({"code": "gap_wide", "label": label, "left": left, "need": need, "path": w,
                              "side": st["side"], "places": places, "others": len(st["gaps"]) > 1,
                              "beside": beside, "at": rnd(local(e, (f0 + f1) / 2, 0))})
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
    mirror_held(held, picks, storey_of(storey_key), fes, notes)
    return runs, insts, opens


def by_posts(kit):
    """True for a kit laid by posts (rule 15): it has a `role:post` piece of the bay's own group,
    both tagged for one kind of storey or neither tagged."""
    post, wall = (kit or {}).get("post"), (kit or {}).get("wall")
    return bool(post and wall and post.get("storey") == wall.get("storey"))


def post_bays(length, kit, first="corner", last="corner"):
    """The joints of one stretch of posts and bays (rule 15): `(n, joints, near)`. `length` (mm)
    is the stretch from what bounds its start to what bounds its end; `first` and `last` say
    what that is: `"corner"` (a corner post, whose reach is taken off), `"face"` (the face of a
    piece that stands beyond it) or `"free"` (nothing: a post stands at the end itself).
    `joints` are the middles of the posts, mm from the stretch's start, `n + 1` of them for `n`
    bays; where the stretch is no whole number of bays `n` is 0, `joints` is empty, and `near`
    is the two stretch lengths next to it that would close (None where there is none)."""
    post, wall = kit["post"], kit["wall"]
    p, B = post["width"], wall["pitch"]
    r = corner_of(kit if kit.get("corner") else dict(kit, corner=post), True, p)[1]
    a = r if first == "corner" else 0.0
    b = length - (r if last == "corner" else 0.0)
    v0 = a + p / 2 if first == "free" else a - p / 2
    v1 = b - p / 2 if last == "free" else b + p / 2
    n = int(round((v1 - v0) / B)) if B else 0
    if n < 1 or not closes(v1 - v0, n, B):
        lo = int((v1 - v0) // B) if B else 0
        return 0, [], tuple(length + m * B - (v1 - v0) if m >= 1 else None for m in (lo, lo + 1))
    return n, [v0 + k * B for k in range(n + 1)], (None, None)


def post_walls(ring, footprint, doors, kit, prefix, storey_key, crossers=(), margin=MARGIN, notes=None,
               picks=None, name="", parts=None):
    """Runs and instances for walls laid by posts (rule 15), as `wall_plan` returns them.
    `parts`, where given, says which stretches of each edge of `ring` are laid, `{edge: [(u0, u1,
    first, last)]}` with `first` and `last` as `post_bays` takes them; an edge it does not name
    is not laid, and a corner is given its post only where a part ends at it. With none, every
    edge is one stretch, from corner to corner (an end that is no corner of the footprint is
    free)."""
    wall, post = kit["wall"], kit["post"]
    runs, insts, opens = [], [], []
    notes = [] if notes is None else notes
    picks = picks or {"bays": [], "fill": []}
    p, B = post["width"], wall["pitch"]
    kit_c = kit if kit.get("corner") else dict(kit, corner=post)
    stretches, corners = wall_stretches(ring, footprint, crossers, p, margin, 0.0)
    variants = wall.get("variants") or {}
    for w in asked(picks, storey_of(storey_key)):
        if w not in variants and not any(x["code"] == "variant_none" and x["variant"] == w for x in notes):
            notes.append({"code": "variant_none", "variant": w, "refused": (wall.get("refused") or {}).get(w),
                          "at": rnd(footprint[0])})
    res, fes = edges(ring), edges(footprint)
    starts, c = [], 0.0
    for e in res:
        starts.append(c)
        c += e["L"]
    total = c
    stood = set()                           # the corners a part ends at
    held = {}                               # {side: where each bay stands}, for a mirrored fill
    leaf = kit.get("door")
    for st in stretches:
        e, edge = st["e"], st["edge"]
        todo = ([(0.0, st["L"], "corner" if st["start_corner"] else "free", "corner" if st["end_corner"] else "free")]
                if parts is None else parts.get(edge, []))
        j = nposts = 0
        common = {"face": outward(e), "offset": round(p / 2000, 3), "ends": "half"}
        for u0, u1, first, last in todo:
            for at_, kind in ((e["a"], first), (e["b"], last)):
                if kind == "corner":
                    stood.add(at_)
                    if not convex_at(corners, at_):
                        raise ParamError("kit", "the outline turns in, and a kit laid by posts walls no inside "
                                         "corner yet", "make the plan a shape with no inside corner, or use a kit "
                                         "of whole wall pieces")
            n, joints, near = post_bays(u1 - u0, kit, first, last)
            reach = corner_of(kit_c, True, p)[1]
            a = u0 + (reach if first == "corner" else 0.0)
            b = u1 - (reach if last == "corner" else 0.0)
            if b - a < -FIT:
                notes.append({"code": "side_short", "side": st["side"], "length": u1 - u0, "takes": (u1 - u0) - (b - a),
                              "at": rnd(local(e, (u0 + u1) / 2, 0))})
                continue
            if not n:
                opens.append({"key": f"{prefix}_w{edge}_{j}", "length": b - a,
                              "near": tuple(v - (u1 - u0) + (b - a) if v is not None else None for v in near),
                              "side": st["side"], "stairwell": False, "storey": storey_of(storey_key), "posts": True,
                              "whole": u1 - u0,
                              "bay": B, "post": p, "at": rnd(local(e, (a + b) / 2, 0))})
                j += 1
                continue
            joints = [u0 + v for v in joints]
            gone, lost = set(), set()       # the bays with no panel, the joints with no post
            for i, d in enumerate(doors):
                s = d["s"] - st["off"]
                # `along`: how much further the author's `at` is than the wall's own measure (a
                # wall behind a colonnade starts inside its building's side, rule 16).
                off = st["off"] + d.get("along", 0)
                mids = ", ".join(f"{(off + (joints[k] + joints[k + 1]) / 2) / 1000:g}" for k in range(n))
                if d["edge"] != st["side"] or not joints[0] - p / 2 - FIT <= s <= joints[-1] + p / 2 + FIT:
                    continue
                k = min(n - 1, max(0, int((s - joints[0]) // B)))
                lo, hi = joints[k] + p / 2, joints[k + 1] - p / 2
                need = d["width"]            # no margin: nothing is cut for it, a bay is as clear as it is
                if s - need / 2 < lo - FIT or s + need / 2 > hi + FIT:
                    raise ParamError("doors", f"door {i} is {(s + off) / 1000:g} m along side {st['side']}, where a post "
                                     f"stands in its opening ({need / 1000:g} m): a bay is "
                                     f"{(B - p) / 1000:g} m clear between two posts",
                                     f"put its middle in the middle of a bay: {mids} m along the side")
                if leaf is not None and abs(s - (lo + hi) / 2) > FIT:
                    raise ParamError("doors", f"door {i} is {(s + off) / 1000:g} m along side {st['side']}, off the middle "
                                     "of its bay: the kit's door piece takes a panel's place, between two posts",
                                     f"put its middle in the middle of a bay: {mids} m along the side")
                gone.add(k)
                if leaf is not None:
                    insts.append({"key": f"{prefix}_door{i}", "asset": leaf["id"],
                                  "at": rnd(local(e, (lo + hi) / 2, p / 2)), "face": outward(e)})
            for g0, g1, label, _ in st["gaps"]:
                if label and label[0] == "door":
                    continue                 # a door is held to one bay, above
                gone.update(k for k in range(n) if g0 < joints[k + 1] - p / 2 and g1 > joints[k] + p / 2)
                lost.update(k for k in range(1, n) if g0 < joints[k] + p / 2 and g1 > joints[k] - p / 2)
            # No `lead`: the side's first bay starts half a post in, less than half a bay, so a
            # bay's number from the nearer end is the same counted from the side's own end.
            words = bay_variants(n, st["off"] + joints[0], B, st["side"], storey_of(storey_key), picks, name, variants,
                                 fes[st["side"]]["L"])
            held.setdefault(st["side"], []).extend((st["off"] + joints[k], st["off"] + joints[k + 1])
                                                   for k in range(n) if k not in gone)
            k = 0
            while k < n:                    # the panels: one run for each spell of bays that have one
                if k in gone:
                    k += 1
                    continue
                k1 = k
                while k1 < n and k1 not in gone:
                    k1 += 1
                ids = [variants[w]["id"] if w in variants else wall["id"] for w in words[k:k1]]
                run = {"key": f"{prefix}_w{edge}_{j}", "count": k1 - k, "ref": storey_key,
                       "span": [round((starts[edge] + joints[k]) / total, 9),
                                round((starts[edge] + joints[k1]) / total, 9)], **common}
                if len(set(ids)) == 1:
                    run = {"key": run["key"], "asset": ids[0], **{x: run[x] for x in run if x != "key"}}
                else:
                    run["assets"] = ids
                runs.append(run)
                j, k = j + 1, k1
            k = 1
            while k < n:                    # the posts between two bays
                if k in lost:
                    k += 1
                    continue
                k1 = k
                while k1 < n and k1 not in lost:
                    k1 += 1
                runs.append({"key": f"{prefix}_p{edge}" + (f"_{nposts}" if nposts else ""), "asset": post["id"],
                             "count": k1 - k, "ref": storey_key,
                             "span": [round((starts[edge] + joints[k] - B / 2) / total, 9),
                                      round((starts[edge] + joints[k1 - 1] + B / 2) / total, 9)], **common})
                nposts, k = nposts + 1, k1
            for kind, v, word in ((first, joints[0], "a"), (last, joints[-1], "b")):
                if kind == "free":          # nothing bounds the stretch there: its own post
                    insts.append({"key": f"{prefix}_p{edge}_{word}", "asset": post["id"],
                                  "at": rnd(local(e, v, p / 2)), "face": outward(e)})
    for c_ in corners:
        if c_["at"] not in stood:
            continue
        piece, r = corner_of(kit_c, c_["convex"], p)
        nb, na = c_["before"]["n"], c_["after"]["n"]
        at = (c_["at"][0] + (nb[0] + na[0]) * r / 2, c_["at"][1] + (nb[1] + na[1]) * r / 2)
        insts.append({"key": f"{prefix}_c{ring.index(c_['at'])}", "asset": piece["id"], "at": rnd(at),
                      "face": outward(c_["after"])})
    mirror_held(held, picks, storey_of(storey_key), fes, notes)
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


def slab_plan(floor, footprint, T, tile, prefix, surface, hole=None):
    """Floor tiles over `floor` (a storey's ring, or the footprint for a flat roof), inside the
    walls (rule 9). Returns (instances, short): `short` is how many grid cells lie partly on the
    floor and so hold no tile, with the middle of the first.
    The grid is the tile's pitch, from the first corner of side 0 moved in by the wall's
    thickness, along side 0 and square to it. A cell holds a tile when every one of 25 points
    spread over it (`FIT` in from its edges) lies on the floor and inside the walls' inner face.
    On a building whose side 0 does not lie along an axis no tile is laid, and `short` is None:
    tiles are instances, placed on the millimetre lattice, and off the axes two that abut overlap
    by the rounding, which the core takes for two things in one place (rules.md 8.1).
    `hole`, a ring, is left untiled (the room inside a platform, rule 16): a cell wholly in it
    holds no tile and is not short, one partly in it is short."""
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
            if hole is not None:
                on = [a_ and not inside(hole, p) for a_, p in zip(on, pts)]
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
            for k in range(int(e["storey"]), storeys):
                for x in sorted({int(x) for x in e["sides"]}):
                    out[k][x] += sign * int(round(e["by"] * 1000))
    for k in range(storeys):                     # over every entry: no two sides that meet at a corner
        if any(out[k][x] and out[k][(x + 1) % n] for x in range(n)):
            raise ParamError("setback", f"storey {k} moves two sides that meet at a corner",
                             "move one side: a terrace or an overhang round a corner is not laid")
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


def colonnade_of(attrs, storeys, footprint):
    """Per side, the mm the walls stand inside the footprint behind a colonnade (rule 16), or
    None where the building has none. `colonnade = [{ sides, depth }]`, checked: `sides` is
    `"all"` or a list of the outline's sides, `depth` metres above 0."""
    v = attrs.get("colonnade")
    if v in (None, []):
        return None
    n = len(footprint)
    if not isinstance(v, list):
        raise ParamError("colonnade", "`colonnade` is a list of tables", "write it as `[{ sides, depth }]`")
    if storeys != 1:
        raise ParamError("colonnade", f"a colonnade is laid on a building of one storey, and this one has {storeys}",
                         "take `colonnade` out, or make the building one storey")
    if not rectangle(footprint):
        raise ParamError("colonnade", "a colonnade is laid on a rectangle only, and this outline is not one",
                         "take `colonnade` out, or make the building a rectangle")
    out = [0] * n
    for i, e in enumerate(v):
        sides = e.get("sides", "all") if isinstance(e, dict) else None
        depth = e.get("depth") if isinstance(e, dict) else None
        ok = (isinstance(e, dict) and set(e) <= {"sides", "depth"}
              and (sides == "all" or (isinstance(sides, list) and sides and all(
                  isinstance(x, (int, float)) and not isinstance(x, bool) and x == int(x) and 0 <= x < n for x in sides)))
              and isinstance(depth, (int, float)) and not isinstance(depth, bool) and math.isfinite(depth)
              and 0 < depth <= 1000)
        if not ok:
            raise ParamError("colonnade", f"colonnade {i} is not `{{ sides, depth }}`",
                             f"write `sides` as \"all\" or a list of the outline's sides (0 to {n - 1}) and `depth` "
                             "as metres above 0")
        for x in (range(n) if sides == "all" else sorted({int(x) for x in sides})):
            if out[x]:
                raise ParamError("colonnade", f"side {x} is given a colonnade twice", "name each side once")
            out[x] = int(round(depth * 1000))
    es = edges(footprint)
    for x in range(n):
        if out[x] + out[(x + 2) % n] >= es[(x + 1) % n]["L"] - 1000:
            raise ParamError("colonnade", "the colonnade leaves the room behind it less than a metre deep",
                             "make it shallower, or the building larger")
    return out


def through_door(i, outer, dp, kit, ck, footprint, walls, margin=MARGIN):
    """A door through a colonnade (rule 16) is held twice on one axis: its doorway must be clear
    of the colonnade's posts, and its opening must fit the wall behind (rule 6, or rule 15 where
    that wall is laid by posts). Raises `params` where it is not, in the measure the author
    wrote, along the building's side, and offers only the places that both take. `outer` is
    the door as planned on the `footprint`, `dp` as planned on the `walls`' outline, `ck` the
    colonnade's kit where it has its post and lintel."""
    k, m, need, off = outer["edge"], outer["s"], outer["width"], dp["along"]
    cj = post_bays(edges(footprint)[k]["L"], ck)[1] if ck else []
    p_c = ck["post"]["width"] if ck else 0
    hit = [v for v in cj if abs(m - v) < (p_c + need) / 2 - FIT]
    cands = {(a + b) / 2 for a, b in zip(cj, cj[1:])}

    def ok_col(v):
        return not any(abs(v - j) < (p_c + need) / 2 - FIT for j in cj)
    posts = bool(kit and kit.get("wall") and by_posts(kit))
    why, span = None, None
    if posts:
        wj = post_bays(edges(walls)[k]["L"], kit)[1]
        p_w, B, leaf = kit["post"]["width"], kit["wall"]["pitch"], kit.get("door")

        def bay_of(u):
            return next(((a, b) for a, b in zip(wj, wj[1:])
                         if a + p_w / 2 - FIT <= u - need / 2 and u + need / 2 <= b - p_w / 2 + FIT), None)

        def ok_wall(v):
            bay = bay_of(v - off)
            return not wj or (bay is not None and (leaf is None or abs(v - off - sum(bay) / 2) <= FIT))
        cands |= {off + (a + b) / 2 for a, b in zip(wj, wj[1:])}
        if not ok_wall(m):
            why = (f"where a post of the wall behind the colonnade stands in its opening ({need / 1000:g} m): a "
                   f"bay is {(B - p_w) / 1000:g} m clear between two posts" if bay_of(m - off) is None else
                   "off the middle of its bay in the wall behind the colonnade: the kit's door piece takes a "
                   "panel's place, between two posts")
    elif kit and kit.get("wall"):
        T = kit["wall"]["thick"]
        n = len(walls)

        def turns_left(v):
            a, b, c = walls[v - 1], walls[v], walls[(v + 1) % n]
            return (b[0] - a[0]) * (c[1] - b[1]) - (b[1] - a[1]) * (c[0] - b[0]) > 0
        r0, r1 = corner_of(kit, turns_left(k), T)[1], corner_of(kit, turns_left((k + 1) % n), T)[1]
        piece = kit["door"].get("width", kit["door"]["pitch"]) if kit.get("door") else 0
        half = max(piece, need + margin) / 2
        lo, hi = off + r0 + half, off + edges(walls)[k]["L"] - r1 - half
        span = (lo, hi)

        def ok_wall(v):
            return lo - FIT <= v <= hi + FIT
        if not ok_wall(m):
            why = (f"its opening ({2 * half / 1000:g} m) runs into a corner of the wall behind the colonnade, "
                   f"which takes {(r0 if m < lo else r1) / 1000:g} m of that wall")
    else:
        def ok_wall(v):
            return True
    if not hit and why is None:
        return
    places = sorted(v for v in cands if ok_col(v) and ok_wall(v))
    if hit:
        message = (f"door {i} is {m / 1000:g} m along side {k}, and a post of the colonnade stands "
                   f"{hit[0] / 1000:g} m along it, in the doorway")
    else:
        message = f"door {i} is {m / 1000:g} m along side {k}" + (": " if span else ", ") + why
    if places:
        hint = (f"put the door's middle between two posts: {', '.join(f'{v / 1000:g}' for v in places)} m along the "
                "side" + (", where the wall behind takes it too" if kit and kit.get("wall") and cj else ""))
    elif cands:
        def said(vs):
            return ", ".join(f"{v / 1000:g}" for v in sorted(vs))
        clear = [v for v in ((a + b) / 2 for a, b in zip(cj, cj[1:])) if ok_col(v)]
        took = [v for v in cands if ok_wall(v)]
        behind = (f"from {span[0] / 1000:g} to {span[1] / 1000:g} m" if span and span[1] >= span[0] else
                  f"at {said(took)} m" if took else "nowhere")
        hint = (f"no bay of the wall behind takes a door {need / 1000:g} m wide: narrow it, or use another side"
                if not cj else
                f"the doorway is {need / 1000:g} m wide, and the colonnade leaves {(ck['wall']['pitch'] - p_c) / 1000:g} m "
                "clear between two posts: write a `width` of that or less" if not clear else
                f"no place on this side takes the door: between two posts of the colonnade its middle is at "
                f"{said(clear)} m along the side, and the wall behind takes it {behind}. Change the number of "
                "bays or the door's width, or use another side")
    elif span and span[1] >= span[0]:
        hint = f"keep its middle between {span[0] / 1000:g} and {span[1] / 1000:g} m along the side"
    else:
        hint = "the wall behind the colonnade is too short for this door: use another side"
    raise ParamError("doors", message, hint)


def rectangle(ring):
    """True when `ring` is four sides, each square to the next (`square`)."""
    es = edges(ring)
    return len(ring) == 4 and all(square(es[i - 1], es[i]) for i in range(4))


def gable_plan(footprint, kit, T, prefix, eaves=0):
    """A gable roof over a rectangle (rule 10): (runs, instances), or (None, why) when it cannot
    be laid, `why` a sentence for `roof_none`. With `eaves` (mm, rule 17) `footprint` is the
    roof's own outline, the building's grown by them: the slopes run its whole length, and the
    gable halves stand `eaves` in from each end, over the wall, each made for the slope's span
    less the eaves.
    Slopes: one run of `role:roof_slope` pieces along each long side, from the wall's thickness in
    at one end to the same at the other, their middles half a span in from the eave, fronts out.
    Gables: two `role:roof_gable` halves on each short side, each half a wall's thickness in, the
    first's front out and the second's in, so their high ends meet under the ridge."""
    slope, gable = kit.get("roof_slope"), kit.get("roof_gable")
    if slope is None or gable is None:
        return None, "the kit has no `role:roof_slope` and `role:roof_gable` pieces"
    if not rectangle(footprint):
        if eaves:
            return None, "the plan is not a rectangle, and a gable under eaves over any other plan is not built"
        return ell_plan(footprint, kit, T, prefix)
    es = edges(footprint)
    long = max(range(4), key=lambda i: (es[i]["L"], -i))
    L, D = es[long]["L"], es[(long + 1) % 4]["L"]
    span = slope.get("span")
    if not span or abs(2 * span - D) > FIT:
        return None, (f"the building is {D / 1000:g} m deep and the kit's slope piece is made for "
                      f"{2 * (span or 0) / 1000:g} m (twice its span)")
    if eaves and (not gable.get("span") or abs(gable["span"] - (span - eaves)) > FIT):
        return None, (f"the gable piece is made for a span of {(gable.get('span') or 0) / 1000:g} m, and under eaves of "
                      f"{eaves / 1000:g} m the wall it stands on is {(D - 2 * eaves) / 1000:g} m long: a gable piece "
                      f"for this roof has a span of {(span - eaves) / 1000:g} m (the slope's less the eaves)")
    if not eaves and (not gable.get("span") or abs(gable["span"] - span) > FIT):
        return None, (f"the gable piece is made for a span of {(gable.get('span') or 0) / 1000:g} m and the "
                      f"slope piece for {span / 1000:g} m")
    lead = 0 if eaves else T                 # under eaves the slopes run past the gable, to the roof's end
    eave = L - 2 * lead
    n = int(round(eave / slope["pitch"])) if slope["pitch"] else 0
    if n < 1 or not closes(eave, n, slope["pitch"]):
        below, above = int(eave // slope["pitch"]) * slope["pitch"] + 2 * lead, (int(eave // slope["pitch"]) + 1) * slope["pitch"] + 2 * lead
        if eaves:
            return None, (f"the roof is {eave / 1000:g} m long with its eaves, no whole number of {slope['pitch'] / 1000:g} m "
                          f"slope pieces: a building {(eave - below) / 1000:g} m shorter or "
                          f"{(above - eave) / 1000:g} m longer would close")
        return None, (f"the eave between the gables is {eave / 1000:g} m, no whole number of {slope['pitch'] / 1000:g} m "
                      f"slope pieces: a side of {below / 1000:g} or {above / 1000:g} m would close")
    starts, c = [], 0.0
    for e in es:
        starts.append(c)
        c += e["L"]
    runs, insts = [], []
    for i in (long, (long + 2) % 4):
        runs.append({"key": f"{prefix}_e{i}", "asset": slope["id"], "count": n, "ref": prefix,
                     "span": [round((starts[i] + lead) / c, 9), round((starts[i] + L - lead) / c, 9)],
                     "face": outward(es[i]), "offset": round(span / 2000, 3), "ends": "half"})
    half = gable["span"]                     # a gable half's own length: the span, less the eaves under them
    for i in ((long + 1) % 4, (long + 3) % 4):
        e = es[i]
        out = outward(e)
        insts.append({"key": f"{prefix}_g{i}_0", "asset": gable["id"],
                      "at": rnd(local(e, eaves + half / 2, eaves + T / 2)), "face": out, "surface": f"{prefix}.garden"})
        insts.append({"key": f"{prefix}_g{i}_1", "asset": gable["id"],
                      "at": rnd(local(e, D - eaves - half / 2, eaves + T / 2)),
                      "face": round((out + 180.0) % 360.0, 3), "surface": f"{prefix}.garden"})
    return runs, insts


def ell_plan(footprint, kit, T, prefix):
    """A gable roof over an L (rule 10, "Over an L"): (runs, instances), or (None, why).
    With `c` the outline's inside corner, its sides in order from `c` are: `c`, an inner eave;
    `c + 1`, a wing's free end; `c + 2` and `c + 3`, the two outer eaves, which meet at the
    outer corner; `c + 4`, the other free end; `c + 5` (that is `c - 1`), the other inner eave."""
    n = len(footprint)
    es = edges(footprint)
    turns = [es[i - 1]["e"][0] * es[i]["e"][1] - es[i - 1]["e"][1] * es[i]["e"][0] for i in range(n)]
    inner = [i for i, t in enumerate(turns) if t < 0]
    if n != 6 or len(inner) != 1 or not all(square(es[i - 1], es[i]) for i in range(n)):
        return None, ("the plan is neither a rectangle nor an L (six sides, each square to the next, one inside corner), "
                      "and a gable over any other plan is not built")
    slope, gable, hip, valley = (kit.get(k) for k in ("roof_slope", "roof_gable", "roof_hip", "roof_valley"))
    if hip is None or valley is None:
        return None, "the plan is an L, and the kit has no `role:roof_hip` and `role:roof_valley` pieces for its corner"
    span, pitch = slope.get("span"), slope["pitch"]
    for piece, word in ((gable, "gable"), (hip, "hip"), (valley, "valley")):
        if not span or not piece.get("span") or abs(piece["span"] - span) > FIT:
            return None, (f"the {word} piece is made for a span of {(piece.get('span') or 0) / 1000:g} m and the slope "
                          f"piece for {(span or 0) / 1000:g} m")
    c = inner[0]
    side = [es[(c + k) % n] for k in range(6)]       # 0 and 5 inner eaves, 1 and 4 free ends, 2 and 3 outer eaves
    for k in (1, 4):
        if abs(side[k]["L"] - 2 * span) > FIT:
            return None, (f"a wing of the L is {side[k]['L'] / 1000:g} m deep and the kit's slope piece is made for "
                          f"{2 * span / 1000:g} m (twice its span): both wings must be that deep")
    reach = valley.get("reach") or 0
    if not pitch or pitch <= 0:
        return None, "the kit's slope piece states no length along the eave (`params.pitch`)"
    # Each eave's stretch of slope pieces, in mm along its side: what the gable's wall, the hip
    # and the valley leave of it.
    stretch = {0: (reach, side[0]["L"] - T), 5: (T, side[5]["L"] - reach),
               2: (T, side[2]["L"] - span), 3: (span, side[3]["L"] - T)}
    counts = {}
    for k, (a, b) in stretch.items():
        i = (c + k) % n
        what = "inner" if k in (0, 5) else "outer"
        took = "the valley piece" if k in (0, 5) else "the hip"
        if b - a < -FIT:
            # The side is shorter than what stands on it: no length of it is an L's wing.
            need = (reach if k in (0, 5) else span) + T
            return None, (f"side {i} of the L, an {what} eave, is {side[k]['L'] / 1000:g} m long, and {took} and the "
                          f"gable's wall take {need / 1000:g} m of it: a side of {need / 1000:g} m or more, by whole "
                          f"{pitch / 1000:g} m, would close")
        m = max(0, int(round((b - a) / pitch)))
        if not closes(b - a, m, pitch):
            lo = int((b - a) // pitch) * pitch
            return None, (f"side {i} of the L, an {what} eave, holds {(b - a) / 1000:g} m of slope between {took} "
                          f"and its gable, no whole number of {pitch / 1000:g} m slope pieces: a side "
                          f"{(b - a - lo) / 1000:g} m shorter or {(lo + pitch - (b - a)) / 1000:g} m longer would close")
        counts[k] = m
    starts, total = [], 0.0
    for e in es:
        starts.append(total)
        total += e["L"]
    runs, insts = [], []
    for k, (a, b) in sorted(stretch.items()):
        if not counts[k]:
            continue
        i = (c + k) % n
        runs.append({"key": f"{prefix}_e{i}", "asset": slope["id"], "count": counts[k], "ref": prefix,
                     "span": [round((starts[i] + a) / total, 9), round((starts[i] + b) / total, 9)],
                     "face": outward(side[k]), "offset": round(span / 2000, 3), "ends": "half"})
    surface = f"{prefix}.garden"
    for k in (1, 4):                                 # the free ends: two gable halves each, as on a rectangle
        e, i = side[k], (c + k) % n
        out = outward(e)
        insts.append({"key": f"{prefix}_g{i}_0", "asset": gable["id"], "at": rnd(local(e, span / 2, T / 2)),
                      "face": out, "surface": surface})
        insts.append({"key": f"{prefix}_g{i}_1", "asset": gable["id"], "at": rnd(local(e, 2 * span - span / 2, T / 2)),
                      "face": round((out + 180.0) % 360.0, 3), "surface": surface})
    # The hip: on the outer corner, where side 2 ends and side 3 starts, its front out of side 3,
    # so its other eave (the asset's +x, against that side's travel) lies on side 2.
    insts.append({"key": f"{prefix}_hip", "asset": hip["id"], "at": rnd(local(side[3], span / 2, span / 2)),
                  "face": outward(side[3]), "surface": surface})
    # The valley: at the inside corner itself, its front out of the side that follows it.
    insts.append({"key": f"{prefix}_valley", "asset": valley["id"], "at": rnd(side[0]["a"]),
                  "face": outward(side[0]), "surface": surface})
    return runs, insts


def roof_for(kit, ring, eaves=0):
    """The kit with the roof pieces made for this roof (rule 17): `(kit, why)`. Of several
    `role:roof_slope` pieces of different spans the first whose span is half the roof's depth is
    taken, and of each other roof role the first piece of that span; a gable half under `eaves`
    (mm) is made for the slope's span less the eaves, and is chosen by that. `ring` is the roof's
    outline; its depth is a rectangle's shorter side, and on any other plan the length two or
    more of its sides share with twice a slope's span. With one span in the kit, or none, the
    slope is the kit's and the roof's own rule says what does not fit. Where no slope
    fits, `kit` is None and `why` names the depths the kit's slopes roof."""
    roofs = (kit or {}).get("roofs") or {}
    slopes = [x for x in roofs.get("roof_slope", []) if x.get("span")]
    spans = sorted({x["span"] for x in slopes})
    if len(spans) < 2 and not eaves:
        return kit, None
    slope = (kit or {}).get("roof_slope")
    if len(spans) >= 2:
        es = edges(ring)
        lens = [e["L"] for e in es]
        if rectangle(ring):
            depths = [min(lens)]
        else:
            depths = [2 * sp for sp in spans if sum(1 for v in lens if abs(v - 2 * sp) <= FIT) >= 2] or [min(lens)]
        mine = [x for d in depths for x in slopes if abs(2 * x["span"] - d) <= FIT]
        if not mine:
            made = " and ".join(f"{2 * sp / 1000:g}" for sp in spans)
            return None, (f"the roof is {depths[0] / 1000:g} m deep and the kit's slope pieces are made for roofs "
                          f"{made} m deep (twice their spans)")
        slope = mine[0]
    if not slope or not slope.get("span"):
        return kit, None
    out = dict(kit, roof_slope=slope)
    for role in ROOF_ROLES[1:]:
        want = slope["span"] - (eaves if role == "roof_gable" else 0)
        same = [x for x in roofs.get(role, []) if x.get("span") is not None and abs(x["span"] - want) <= FIT]
        if same:
            out[role] = same[0]
    return out, None


def hip_plan(ring, kit, prefix):
    """A hipped roof over a rectangle (rule 17): (runs, instances), or (None, why). A
    `role:roof_hip` piece stands on each corner, at vertex `i` with its front out of side `i`,
    so its other eave lies on the side before; one run of `role:roof_slope` pieces stands along
    each side between the two hips, whole pieces. `ring` is the roof's outline."""
    slope, hip = kit.get("roof_slope"), kit.get("roof_hip")
    if slope is None or hip is None:
        return None, "the kit has no `role:roof_slope` and `role:roof_hip` pieces"
    if not rectangle(ring):
        return None, "the plan is not a rectangle, and a hipped roof over any other plan is not built"
    es = edges(ring)
    D = min(e["L"] for e in es)
    span, pitch = slope.get("span"), slope["pitch"]
    if not span or abs(2 * span - D) > FIT:
        return None, (f"the roof is {D / 1000:g} m deep and the kit's slope piece is made for "
                      f"{2 * (span or 0) / 1000:g} m (twice its span)")
    if not hip.get("span") or abs(hip["span"] - span) > FIT:
        return None, (f"the hip piece is made for a span of {(hip.get('span') or 0) / 1000:g} m and the slope "
                      f"piece for {span / 1000:g} m")
    if not pitch or pitch <= 0:
        return None, "the kit's slope piece states no length along the eave (`params.pitch`)"
    counts = []
    for i, e in enumerate(es):
        eave = e["L"] - 2 * span
        n = max(0, int(round(eave / pitch)))
        if not closes(eave, n, pitch):
            lo = int(eave // pitch) * pitch
            return None, (f"side {i} holds {eave / 1000:g} m of eave between its two hips, no whole number of "
                          f"{pitch / 1000:g} m slope pieces: a side {(eave - lo) / 1000:g} m shorter or "
                          f"{(lo + pitch - eave) / 1000:g} m longer would close")
        counts.append(n)
    starts, total = [], 0.0
    for e in es:
        starts.append(total)
        total += e["L"]
    runs, insts = [], []
    for i, e in enumerate(es):
        if counts[i]:
            runs.append({"key": f"{prefix}_e{i}", "asset": slope["id"], "count": counts[i], "ref": prefix,
                         "span": [round((starts[i] + span) / total, 9), round((starts[i] + e["L"] - span) / total, 9)],
                         "face": outward(e), "offset": round(span / 2000, 3), "ends": "half"})
        insts.append({"key": f"{prefix}_h{i}", "asset": hip["id"], "at": rnd(local(e, span / 2, span / 2)),
                      "face": outward(e), "surface": f"{prefix}.garden"})
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


def yard_plan(name, footprint, attrs, kit=None, kits=None, margin=MARGIN):
    """A building of no storeys (rule 18): its walls on the ground, round a `yard` feature that
    owns no surface. Returns what `plan` returns, with no storey."""
    prefix = key_of(name)
    for key in ("stair", "rooms", "roof", "colonnade", "setback", "overhang"):
        if attrs.get(key):
            raise ParamError(key, f"a building of no storeys is a wall on the ground, and takes no `{key}`",
                             f"take `{key}` out, or give the building a storey")
    high = _num(attrs, "storey_h", 3.2, 0.5)
    porch = int(round(_num(attrs, "porch", 2, 0.5) * 1000))
    k = kits("ground", int(round(high * 1000))) if kits else kit
    if not k or not k.get("wall"):
        raise ParamError("kit", "a building of no storeys is a wall on the ground, and this one has no kit with a bay",
                         "name a `kit`, or give the building a storey")
    stated = k["wall"].get("stated") or {}
    ykey = f"{prefix}_yard"
    doors, crossers = [], []
    for i, d in enumerate(attrs.get("doors") or []):
        if int(d.get("storey", 0)) != 0:
            raise ParamError("doors", f"door {i} is on storey {int(d.get('storey', 0))}, and the building has none",
                             "write `storey = 0`: a gate in the wall")
        dp = door_plan(footprint, d, porch, 0, stated.get("door"))
        doors.append(dp)
        crossers.append((dp["doorway"], dp["width"], ("door", f"{prefix}_d{i}")))
    picks = picks_of(attrs, 1, len(footprint), True)
    notes = []
    runs, insts, opens = wall_plan(footprint, footprint, doors, k, ykey, ykey, crossers, margin, notes, picks, name)
    for it in insts:
        it["surface"] = None                 # on the terrain
    for b in picks["bays"]:
        if not b["hit"]:
            notes.append({"code": "bay_none", "index": b["index"], "storey": b["storey"], "side": b["side"],
                          "at_m": b["at"] / 1000, "variant": b["variant"],
                          "at": rnd(local(edges(footprint)[b["side"]], b["at"], 0))})
    return {"storeys": [], "rooms": [], "flights": [], "doorways": [], "footpaths": [], "runs": runs,
            "instances": insts, "open": opens, "openings": [], "roof": None, "notes": notes, "footings": [],
            "kit": k, "heights": [high], "terraces": [], "platform": None, "yard": {"key": ykey, "region": footprint}}


def plan(name, ring, attrs, ground_m, grid=None, kit=None, inner=INNER_DOOR, margin=MARGIN, kits=None, foot=None):
    """Everything one building returns, as plain data (the step writes it): rules 1 to 19.
    `kit` is one kit for every storey (`kit_pieces`); `kits`, where given, is a function of a
    storey's word and its height in mm that gives that storey's kit, and `kit` is not read.
    `foot` is the ground along the outline, [(x mm, y mm, metres)] (`read_foot`): with it the
    wall's foot is held against the ground (rule 13)."""
    footprint = ccw(ring)
    prefix = key_of(name)
    storeys = _num(attrs, "storeys", 1, 0, integer=True)
    level = attrs.get("ground") or ""
    if level not in ("", "level"):
        raise ParamError("ground", f"`ground` is {level!r}",
                         'write `ground = "level"` for a plot levelled under the building, or leave it out')
    if storeys == 0:                         # a wall on the ground (rule 18)
        if level:
            raise ParamError("ground", "`ground` is written on a building of no storeys",
                             "a wall stands on the ground as it lies: write `ground` on the court it is the yard of")
        return yard_plan(name, footprint, attrs, kit, kits, margin)
    if level and foot:                       # the ground the plot will hold, all round (rule 19)
        foot = [(x, y, ground_m) for x, y, _ in foot]
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
    # The wall's band, from the outline in: a wall's thickness, and for a kit laid by posts its
    # post's width, which the panel stands inside (rule 15). The stair keeps off it and the
    # footing over a stairwell is as deep, so a flight beside a wall of posts crosses nothing.
    band = kit["post"]["width"] if by_posts(kit) else wall_t
    sp = stair_plan(footprint, stair, storeys, band, stated.get("stair"))
    # Each storey's own outline (rule 14): the footprint, with the sides a `setback` or an
    # `overhang` moves. `crown` is the top storey's, which the roof covers.
    moved = offsets_of(attrs, storeys, footprint, sp.get("edge"))
    # A colonnade (rule 16): the walls stand inside the footprint from the ground storey, and
    # the roof covers the footprint, not the walls' outline.
    col = colonnade_of(attrs, storeys, footprint)
    if col:
        moved[0] = [a_ + b_ for a_, b_ in zip(moved[0], col)]
    pkey = f"{prefix}_platform"
    ck = (kits("colonnade", None) if kits else {}) if col else {}
    col_posts = bool(ck) and (ck.get("post") or {}).get("storey") == "colonnade" and \
        (ck.get("wall") or {}).get("storey") == "colonnade"
    if col_posts:
        thin = [d for d in col if 0 < d < ck["post"]["width"] - FIT]
        if thin:
            raise ParamError("colonnade", f"the colonnade's `depth` is {thin[0] / 1000:g} m, less than its post is wide "
                             f"({ck['post']['width'] / 1000:g} m): the walls would stand in the posts",
                             f"write a `depth` of whole bays: {ck['wall']['pitch'] / 1000:g} m, or a multiple of it")
    outline = [shift_ring(footprint, ds) if any(ds) else footprint for ds in moved]
    crown = footprint if col else outline[-1]
    rings, footings = [outline[0]], {}
    for k in range(1, storeys):
        notch = sp["notches"][k]
        stood = None
        if band and kit_of[k] and kit_of[k].get("wall"):
            n_ = len(outline[k])

            def takes(v):                        # what the corner piece at an outside corner takes of the wall
                a_, b_, c_ = outline[k][v - 1], outline[k][v], outline[k][(v + 1) % n_]
                left = (b_[0] - a_[0]) * (c_[1] - b_[1]) - (b_[1] - a_[1]) * (c_[0] - b_[0]) > 0
                return corner_of(kit_of[k], True, band)[1] if left else band
            stood = footing_plan(outline[k], notch, band, (takes(notch[0]), takes((notch[0] + 1) % n_)))
        if stood:
            rings.append(stood[0])
            footings[k] = {"key": f"{prefix}_s{k}_footing", "region": stood[1], "level": lv[k], "side": notch[0]}
        else:
            rings.append(notch_ring(outline[k], notch))
    out = {"storeys": [], "rooms": [], "flights": [], "doorways": [], "footpaths": [], "runs": [],
           "instances": [], "open": [], "openings": [], "roof": None, "notes": [],
           "footings": [footings[k] for k in sorted(footings)], "kit": kit, "heights": heights, "terraces": [],
           "platform": {"key": pkey, "region": footprint, "level": lv[0]} if col else None}
    if level:
        out["plot"] = {"key": f"{prefix}_plot", "region": [rnd(q) for q in inset(footprint, -PLOT_APRON)],
                       "level": round(ground_m, 3), "grade": _num(attrs, "grade", 0.3, 0.0),
                       "max_cut": _num(attrs, "max_cut", 3.0, 0.0)}
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
        outer = None
        if col:
            # Behind a colonnade the wall's own door is on the wall's outline (rule 16): the same
            # side, the same axis. On a side with a colonnade the doorway from outside comes to
            # the footprint's edge, onto the platform, and a second one crosses the wall.
            k_ = dp["edge"]
            s_in, L_in = dp["s"] - col[k_ - 1], edges(outline[0])[k_]["L"]
            if not dp["width"] / 2 <= s_in <= L_in - dp["width"] / 2:
                raise ParamError("doors", f"door {i} is {dp['s'] / 1000:g} m along side {k_}, and the wall behind the "
                                 f"colonnade stands from {col[k_ - 1] / 1000:g} to {(col[k_ - 1] + L_in) / 1000:g} m "
                                 "along it", "move the door to where the wall stands")
            behind = {"side": k_, "at": s_in / 1000, "width": dp["width"] / 1000}
            if col[k_]:
                outer, dp = dp, door_plan(outline[0], behind, min(col[k_], 1000), 0)
            else:
                dp = door_plan(outline[0], behind, porch, walk_out)
            # The author wrote `at` along the building's side: what is said of the door is said
            # in that measure, `along` further than the wall's own.
            dp["along"] = col[k_ - 1]
            if outer is not None:
                through_door(i, outer, dp, kit, ck if col_posts else None, footprint, outline[0], margin)
        if sp.get("edge") == dp["edge"]:
            k = sp["edge"]
            lo = min(n[1] for n in sp["notches"].values())
            hi = max(n[2] for n in sp["notches"].values())
            if dp["s"] + dp["width"] / 2 > lo and dp["s"] - dp["width"] / 2 < hi and k == dp["edge"]:
                raise ParamError("doors", f"door {i} opens into the stairwell",
                                 "move the door or the stair along the wall")
        if kit and kit.get("wall") and not by_posts(kit):
            ring_ = outline[0]
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
                off = dp.get("along", 0)
                raise ParamError("doors", f"door {i} is {(dp['s'] + off) / 1000:g} m along side {k_}: its opening "
                                 f"({2 * half / 1000:g} m) runs into a corner"
                                 + (" of the wall behind the colonnade" if col and (off or col[(k_ + 1) % len(col)]) else "")
                                 + f", which takes {(r0 if dp['s'] < lo else r1) / 1000:g} m of that wall",
                                 f"keep its middle between {(lo + off) / 1000:g} and {(hi + off) / 1000:g} m along the side"
                                 if hi >= lo else "the side is too short for this door: use another side")
        doors_by_storey.setdefault(st, []).append(dp)
        key = f"{prefix}_d{i}"
        if outer is not None:
            outer["doorway"][-1] = onto(footprint, outer["doorway"][-1], outer["doorway"][0])
            out["doorways"].append({"key": key, "path": outer["doorway"], "ends_at": f"{pkey}.floor",
                                    "sill_z": lv[st], "flat": round(flat / 1000, 3)})
            if walk_out > 0:
                out["footpaths"].append({"key": f"{prefix}_p{i}", "path": outer["footpath"],
                                         "ends_at": f"{key}.start"})
            key = f"{prefix}_d{i}_in"
        dp["doorway"][-1] = onto(rings[st], dp["doorway"][-1], dp["doorway"][0])
        out["doorways"].append({"key": key, "path": dp["doorway"], "ends_at": f"{prefix}_s{st}.floor",
                                "sill_z": lv[st], "flat": round(flat / 1000, 3)})
        if walk_out > 0 and outer is None:
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
            # Round the footprint: on the ground storey's floor, or on the platform where the walls
            # stand inside it (rule 16).
            under = pkey if col else f"{prefix}_s0"
            runs, insts, opens = wall_plan(footprint, footprint, [], base, f"{prefix}_base", under, (),
                                           margin, out["notes"], None, name)
            for it in insts:
                it["surface"] = f"{under}.floor"
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
    roof_t = wall_t
    if col:                                      # the colonnade (rule 16)
        cpost = ck.get("post") or {}
        n_, es_ = len(footprint), edges(footprint)
        if col_posts:
            roof_t = cpost["width"]
            parts = {}
            for x in range(n_):
                L_ = es_[x]["L"]
                if col[x]:
                    parts[x] = [(0.0, L_, "corner", "corner")]
                    continue
                # A side the walls keep: the platform's two ends beside it, each from the
                # footprint's corner to the face of the wall's own corner.
                before, after = float(col[x - 1]), float(col[(x + 1) % n_])
                parts[x] = ([(0.0, before, "corner", "face")] if before else []) + \
                           ([(L_ - after, L_, "face", "corner")] if after else [])
            seen = len(out["notes"])
            runs, insts, opens = post_walls(footprint, footprint, [], ck, f"{prefix}_col", pkey, (), margin,
                                            out["notes"], None, name, parts)
            for it in insts:
                it["surface"] = f"{pkey}.floor"
            for o_ in opens + out["notes"][seen:]:
                o_["colonnade"] = True
            for o_ in opens:                    # on a side the walls keep, the stretch is the colonnade's depth
                if not col[o_["side"]]:
                    o_["depth"] = True
            out["runs"].extend(runs)
            out["instances"].extend(insts)
            out["open"].extend(opens)
        else:
            out["notes"].append({"code": "colonnade_none", "at": rnd(footprint[0])})
        tile = (kit_of[0] or {}).get("floor")
        if tile:
            # The platform's own tiles: over the footprint inside the base (the colonnade's line
            # where the kit has no base), less the room, whose floor the storey tiles (rule 9).
            # Walls laid by posts are tiled to the post line's middle from both sides (rule 15).
            bk = (kits("base", None) if kits else {}).get("wall") or {}
            room_in = kit_of[0]["post"]["width"] / 2 if by_posts(kit_of[0]) else 0
            # With no base: inside the colonnade's posts, or, where the room's own walls are laid
            # by posts, from the middle of the colonnade's line of posts, as every floor of that
            # kit is (rule 15).
            edge = (bk.get("measured", bk.get("thick", 0)) if bk.get("storey") == "base"
                    else roof_t / 2 if room_in else roof_t)
            room = [rnd(q) for q in inset(outline[0], room_in)] if room_in else outline[0]
            insts, (short, first) = slab_plan(footprint, footprint, edge, tile, pkey, f"{pkey}.floor", room)
            out["instances"].extend(insts)
            if short:
                m_ = tile["pitch"]

                def off_tiles(v):
                    return min(v % m_, m_ - v % m_) > FIT
                # What is off the tiles: a strip's depth, from the platform's edge line to the
                # room's (the wall's face, or its post line's middle), or a side's length inside
                # the edge line. `closes` are the `depth`s, or the side lengths, that are whole tiles.
                deep = [{"side": x, "depth": col[x], "deep": col[x] + room_in - edge,
                         "closes": [v for v in (math.floor((col[x] + room_in - edge) / m_) * m_ - room_in + edge,
                                                math.ceil((col[x] + room_in - edge) / m_) * m_ - room_in + edge) if v > 0]}
                        for x in range(n_) if col[x] and off_tiles(col[x] + room_in - edge)]
                long = [{"side": x, "length": es_[x]["L"], "inside": es_[x]["L"] - 2 * edge,
                         "closes": [math.floor((es_[x]["L"] - 2 * edge) / m_) * m_ + 2 * edge,
                                    math.ceil((es_[x]["L"] - 2 * edge) / m_) * m_ + 2 * edge]}
                        for x in range(n_) if off_tiles(es_[x]["L"] - 2 * edge)]
                out["notes"].append({"code": "slab_open", "storey": 0, "cells": short, "at": first,
                                     "tile": tile["pitch"], "platform": True, "deep": deep, "long": long,
                                     "to": "the middle of the wall's line of posts" if room_in else "the wall"})
    for k in range(storeys):                     # floors (rule 9)
        tile = (kit_of[k] or {}).get("floor")
        if not tile:
            continue
        # Inside the walls; of walls laid by posts, to the post line's middle (rule 15).
        within = kit_of[k]["post"]["width"] / 2 if by_posts(kit_of[k]) else wall_t
        # Over a footing a floor of posts is still tiled to the post line's middle: its last row
        # lies half a post onto the footing, which is at the floor's level (rule 3).
        floor_k = notch_ring(outline[k], sp["notches"][k]) if k in footings and by_posts(kit_of[k]) else rings[k]
        insts, (short, first) = slab_plan(floor_k, outline[k], within, tile, f"{prefix}_s{k}", f"{prefix}_s{k}.floor")
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
                deep, lo, hi = depth - within, max(a, within) - within, min(b, L_ - within) - within
                n_ = max(1, int(round(deep / tile["pitch"])))      # the nearest whole number of tiles

                def whole(v):
                    return min(v % tile["pitch"], tile["pitch"] - v % tile["pitch"]) <= FIT
                if not (whole(deep) and whole(lo) and whole(hi)):       # the stairwell is what is off the tiles
                    note.update({"side": side, "deep": deep, "from": lo, "to": hi, "lane": sp.get("lane"),
                                 "width": (n_ * tile["pitch"] - (band - within) - (lanes + 1) * CLEAR) / lanes,
                                 "tiles": n_,
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
            tile = (kit_of[k - 1] or {}).get("roof_flat") or (kit_of[k - 1] or {}).get("floor")
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
                seen = len(out["notes"])
                runs, insts, opens = wall_plan(strip, strip, [], pk, key, key, (), margin, out["notes"], None, name)
                # No parapet against the wall above: the strip's side 2. A run is `<key>_w<side>_<j>`.
                runs = [r for r in runs if not r["key"].startswith(f"{key}_w2_")]
                for it in insts:
                    it["surface"] = f"{key}.garden"
                # What is said of the strip is said in the building's own sides: the strip's
                # side 0 is the building's side `i`, its sides 1 and 3 the two next to that.
                own = {0: i, 1: (i + 1) % n_, 2: i, 3: (i - 1) % n_}
                for o_ in opens:
                    o_["parapet"], o_["terrace"] = True, k
                for x in list(opens) + out["notes"][seen:]:
                    if "side" in x:
                        x["strip"], x["side"] = x["side"], own[x["side"]]
                    x["terrace"] = k
                out["notes"][seen:] = [x for x in out["notes"][seen:] if x.get("strip") != 2]
                out["runs"].extend(runs)
                out["instances"].extend(insts)
                out["open"].extend(o_ for o_ in opens if o_.get("strip") != 2)
    top = storeys - 1
    roof = attrs.get("roof") or ""
    if not isinstance(roof, str) or roof not in ("", "garden", "flat", "gable", "hip"):
        raise ParamError("roof", f"`roof` is {roof!r}",
                         "write \"flat\", \"gable\", \"hip\" or \"garden\", or leave it out")
    eaves = round(lv[top] + heights[top], 3)
    if roof == "garden":
        out["roof"] = {"key": f"{prefix}_roof", "region": crown, "eaves": eaves}
    elif roof in ("flat", "gable", "hip"):        # roofs from pieces (rules 10 and 17)
        rkey, kt = f"{prefix}_roof", kit_of[top] or {}
        tile, why, laid = kt.get("roof_flat") or kt.get("floor"), None, False   # a roof tile of its own first (rule 10)
        region = crown
        if not kt.get("wall"):
            why = "the building has no `kit`"
        elif roof in ("gable", "hip"):
            # The roof's own outline (rule 17): the building's, grown by the kit's eaves on every
            # side; and of the kit's roof pieces, those made for its depth.
            out_mm = ((kt.get("wall") or {}).get("stated") or {}).get("eaves", 0)
            over = shift_ring(crown, [-out_mm] * len(crown)) if out_mm else crown
            rk, why = roof_for(kt, over, out_mm)
            if rk is not None:
                runs, insts = (gable_plan(over, rk, roof_t, rkey, out_mm) if roof == "gable"
                               else hip_plan(over, rk, rkey))
                if runs is None:
                    why = insts
                else:
                    out["runs"].extend(runs)
                    out["instances"].extend(insts)
                    laid, region = True, over
            if why and out_mm:
                why += f"; the roof stands {out_mm / 1000:g} m beyond the outline on every side (`shared.eaves`)"
        if not laid and kt.get("wall"):
            if tile:
                # Inside the roof's ends; over walls laid by posts, to the post line's middle (rule 15).
                flat_in = kt["post"]["width"] / 2 if by_posts(kt) else roof_t
                insts, (short, first) = slab_plan(crown, crown, flat_in, tile, rkey, f"{rkey}.garden")
                out["instances"].extend(insts)
                laid = short is not None
                if short is None:
                    why = (why + "; and " if why else "") + ("the building is turned off the axes, where floor tiles "
                                                             "do not abut to the millimetre and none is laid")
                elif short:
                    out["notes"].append({"code": "slab_open", "storey": -1, "cells": short, "at": first,
                                         "tile": tile["pitch"]})
            elif roof == "flat":
                why = "the kit has no `role:roof_flat` or `role:floor` piece"
        if laid:
            out["roof"] = {"key": rkey, "region": region, "eaves": eaves}
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
         "setback", "overhang", "colonnade", "ground", "max_cut", "grade",
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


def foot_word(note):
    """What a note of the wall's foot is said as (rule 13): `base_short` (warning) where the
    kit's base stops short of the ground; with no base, `plinth_bare` (info) where the floor
    only stands its plinth above the ground, else `foot_open` (warning)."""
    if note["code"] == "base_short":
        return "base_short"
    return "plinth_bare" if note["hang"] <= note["plinth"] + FOOT_SEEN else "foot_open"


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
    if out.get("platform"):
        f = out["platform"]
        o.feature(f["key"], "platform", region=f["region"], level=f["level"])
    if out.get("yard"):
        o.feature(out["yard"]["key"], "yard", region=out["yard"]["region"])
    if out.get("plot"):
        pl = out["plot"]
        o.feature(pl["key"], "plot", region=pl["region"], level=pl["level"], grade=pl["grade"], max_cut=pl["max_cut"])
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
        on = {"surface": it["surface"]} if it["surface"] is not None else {}       # none: on the terrain (rule 18)
        o.instance(it["key"], it["asset"], at=it["at"], face=it["face"], **on)


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
                  else f" on side {w['side']} of the colonnade" if w.get("colonnade")
                  else f" of the parapet of the terrace before storey {w['terrace']}, beside side {w['side']}"
                  if w.get("parapet") and w.get("terrace") is not None and w.get("strip") != 0
                  else f" of the parapet of the terrace before storey {w['terrace']}, on side {w['side']}"
                  if w.get("parapet") and w.get("terrace") is not None
                  else f" on side {w['side']} of the roof's parapet" if w.get("parapet")
                  else f" on side {w['side']} of storey {w.get('storey', 0)}" if "side" in w else "")
        said = f"a wall stretch of {w['length'] / 1000:g} m{where_} does not close with the kit's pieces"
        if w.get("posts"):
            # Posts and bays (rule 15): what closes is whole bays between two posts.
            said = (f"a stretch of {w['length'] / 1000:g} m{where_} is no whole number of bays: a bay is "
                    f"{w['bay'] / 1000:g} m from one post's middle to the next, and a post {w['post'] / 1000:g} m wide")
            hint = ("resize the side by " + moves + ": between its two corner posts it is then whole bays less one "
                    "post's width") if moves else "lengthen the side to a bay at least"
            if w.get("depth"):
                # The colonnade's end on a side the walls keep: its length is the `depth` of the
                # colonnade round the corner, so that is what closes it, not the side's length.
                gone = w["whole"] - w["length"]             # what the corner post takes
                depths = [v + gone for v in (below, above) if v is not None]
                said = (f"the colonnade's end on side {w['side']}, {w['whole'] / 1000:g} m from the building's corner "
                        f"to the wall, is no whole number of bays: a bay is {w['bay'] / 1000:g} m from one post's "
                        f"middle to the next")
                hint = ("make the colonnade's `depth` " + " or ".join(f"{v / 1000:g}" for v in depths)
                        + " m: its end beside the room is then whole bays")
        if w.get("unfloored"):
            said = (f"`{w['key']}` on storey {w.get('storey', 0)} has neither the storey's floor nor its footing "
                    "under it: it is not laid")
            hint = "move the stair, or change the plan, so the storey's floor reaches under its walls"
        step.warning("wall_open", name, said, hint,
                     position=(w["at"][0] / 1000, w["at"][1] / 1000),
                     data={"length": round((w["whole"] if w.get("depth") else w["length"]) / 1000, 3),
                           "closes": [round((v + (w["whole"] - w["length"] if w.get("depth") else 0)) / 1000, 3)
                                      for v in (below, above) if v is not None],
                           **({"depth": True} if w.get("depth") else {}),
                           **({"side": w["side"], "storey": w.get("storey", 0),
                               "stairwell": bool(w.get("stairwell"))} if "side" in w else {})},
                     **({"explain": EXPLAIN["wall_open"]} if i == 0 else {}))
    for n in (n for n in out["notes"] if n["code"] == "side_short"):
        whose = (f"the parapet of the terrace before storey {n['terrace']}, by side {n['side']}," if n.get("terrace") is not None
                 else f"side {n['side']}")
        step.warning("side_short", name,
                     f"{whose} is {n['length'] / 1000:g} m long and its two corners take "
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
        bes = g.get("beside")
        if bes:
            over = round((bes[0] + bes[1]) / 1000, 3)
            said = (f"the wall leaves {g['left'] / 1000:g} m at {what}, and the door piece stands off its middle: "
                    f"{bes[0] / 1000:g} m stands open before it and {bes[1] / 1000:g} m after it")
        else:
            said = (f"the wall leaves {g['left'] / 1000:g} m at {what}, which needs {g['need'] / 1000:g} m: "
                    f"{over:g} m stands open")
        step.warning("gap_wide", name, said, hint,
                     position=(g["at"][0] / 1000, g["at"][1] / 1000),
                     data={"left": round(g["left"] / 1000, 3), "need": round(g["need"] / 1000, 3),
                           "open": round(over, 3), "path": round(g["path"] / 1000, 3),
                           "side": g.get("side"), "closes_at": places,
                           **({"before": round(bes[0] / 1000, 3), "after": round(bes[1] / 1000, 3)} if bes else {})},
                     **({"explain": EXPLAIN["gap_wide"]} if i == 0 else {}))
    for i, n in enumerate(n for n in out["notes"] if n["code"] == "slab_open"):
        what = "the flat roof" if n["storey"] < 0 else f"storey {n['storey']}"
        if n.get("turned"):
            said = ("no floor tile is laid: the building is turned off the axes, where tiles placed on the millimetre "
                    "lattice do not abut and would stand in each other's place")
            hint = "keep a building with a tiled floor square to the axes for now"
        elif n.get("platform"):
            said = f"{n['cells']} floor tiles of the platform cannot be laid whole: "
            if n["deep"]:
                d = n["deep"][0]
                said += (f"on side {d['side']} it is {d['deep'] / 1000:g} m from the line its tiles start on to "
                         f"{n['to']} behind, no whole number of {n['tile'] / 1000:g} m tiles")
                hint = ("make the colonnade's `depth` there " + " or ".join(f"{v / 1000:g}" for v in d["closes"])
                        + " m: the platform is then whole tiles deep")
            elif n["long"]:
                d = n["long"][0]
                said += (f"side {d['side']} is {d['inside'] / 1000:g} m long inside the line its tiles start on, no "
                         f"whole number of {n['tile'] / 1000:g} m tiles")
                hint = ("make that side " + " or ".join(f"{v / 1000:g}" for v in d["closes"])
                        + " m long: the platform is then whole tiles along it")
            else:
                said += f"the room's outline does not lie on the platform's {n['tile'] / 1000:g} m tiles"
                hint = "make each side's `depth` and the building's sides whole tiles from the platform's edge line"
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
    for n in (n for n in out["notes"] if n["code"] == "mirror_none"):
        step.warning("mirror_none", name,
                     f"the `fill` of side {n['side']} on storey {n['storey']} asks for a mirror, and that side's "
                     "bays do not stand as each other's image about its middle: the pattern is laid, and its two "
                     "halves differ",
                     "make the side whole bays with no half bay at one end, and put its door on its middle; or "
                     "take `mirror` out",
                     position=(n["at"][0] / 1000, n["at"][1] / 1000), data={"side": n["side"], "storey": n["storey"]},
                     explain=EXPLAIN["mirror_none"])
    for n in (n for n in out["notes"] if n["code"] == "colonnade_none"):
        step.warning("colonnade_none", name,
                     "the building asks for a colonnade, and its kit has no post and no lintel for one: the platform "
                     "stands open under the roof, with nothing along its edge",
                     "give the kit a `role:post` piece and a `role:wall` piece (the lintel from one post to the "
                     "next), both tagged `storey:colonnade`",
                     position=(n["at"][0] / 1000, n["at"][1] / 1000), explain=EXPLAIN["colonnade_none"])
    for n in (n for n in out["notes"] if n["code"] == "roof_none"):
        step.warning("roof_none", name,
                     f"the {n['roof']} roof is not laid: {n['why']}"
                     + (". The flat roof is laid in its place" if n["flat"] else ". The building has no roof"),
                     "size the building to the roof pieces, give the kit the pieces, or ask for a roof it can lay",
                     position=(n["at"][0] / 1000, n["at"][1] / 1000),
                     data={"roof": n["roof"], "fallback": "flat" if n["flat"] else ""}, explain=EXPLAIN["roof_none"])
    for n in (n for n in out["notes"] if n["code"] in ("foot_open", "base_short")):
        pos = (n["at"][0] / 1000, n["at"][1] / 1000)
        if foot_word(n) == "base_short":
            step.warning("base_short", name,
                         f"the ground falls {n['hang']:g} m below the floor and the kit's base is {n['deep']:g} m deep: "
                         f"{n['length']:g} m of the base stops short of the ground, by {n['hang'] - n['deep']:.3g} m at most",
                         "level the ground under the building, lower its `plinth`, or give the kit a deeper base",
                         position=pos, data={"hang": n["hang"], "base": n["deep"], "length": n["length"]},
                         explain=EXPLAIN["base_short"])
        elif foot_word(n) == "plinth_bare":
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
