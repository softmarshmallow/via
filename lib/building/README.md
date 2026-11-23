# lib/building: buildings as a library (design note, M5)

A building is library text over core mechanisms: region and path features, kind surfaces, mounts,
connectors, meetings, runs, scatters and steps. This note fixes the library's kinds and steps before
any code. The core contract is `spec/surfaces.md` (1.3, 3.2 to 3.4, 4.1, 4.4, 6.4 to 6.6) and
`spec/assets.md` (2.7, 10). Nothing here needs a core change beyond those; if it ever does, the core
is missing a mechanism (principle 5), and the mechanism is named without these words.

Words. The building-information vocabulary, as the TODO's lesson asks: building, storey, space
(here `room`), wall, door, window, stair, slab, roof. All are on the deny-list (`spec/vocabulary.md`
7): they live here and in worlds, never in `crates/`.

## What is core and what is library

| Need | Core (no domain noun) | This library |
|---|---|---|
| A floor to stand on | a kind surface on a region owner (`[kind.<k>.surface.<name>]`, `base`) | `storey` and its `floor` |
| The terrain not showing inside | `cut` on that surface (`surfaces.md` 1.3) | `headroom`, default 2.6 m |
| Getting in | a meeting: a path end at a kind surface place (`surfaces.md` 3.3), checked for place and height | `doorway`, and the `sill` effect that sets the ground to the floor |
| Going up | a path whose kind owns one surface, ends meeting two floors | `flight` (stairs and ramps alike); going and riser are a lint |
| What may join what | `connector`, compared for equality (`surfaces.md` 3.4) | the words `foot` (doorways, flights, footpaths) and `way` (roads) |
| A door leaf, a wall piece, a roof | instances of pack assets, `collision` from the pack | where they go: runs along the outline, leaving doorways open |
| Furniture on an upper floor | `surface = "<storey>.floor"` on an `at` placement (4.1) | the `furnish` step, or hand placements |
| A roof garden | a kind surface on the roof's owner; a scatter with `surface` | `roof` with `garden` |
| A gate you walk through | `solidity = "passable"` (`assets.md` 2.4) | gatehouse and arcade assets filed so |
| A door that faces | `face` on an asset mount; `turn` in `meetings()` | the walk lint judges `turn` |
| A floor's height | world metres in `base` (no "above the ground" in core) | `lay` computes `level` from `sample` of the terrain |
| Volume | none (`surfaces.md` 1.3) | "a building as a volume" is this library's word for the set below |

## Kinds (`lib/building/kinds.toml`)

Library text in the form a user writes. Attributes illustrative until the code; the shapes are fixed.

- **`building`** (region owner, anchored). The one feature a world writes. Attributes: `storeys`
  (integer, default 1), `storey_h` (default 3.2), `plinth` (height of the ground floor above the
  mean ground under the outline, default 0.3), `front` (a bearing, the main door's side: not
  taken yet, `kinds.toml` has no default for it and `lay` reads `doors`), `doors`
  (a list of `{ storey, t, width }` on the outline, `t` as `on` takes it), `stair` (`{ t, width,
  run }`, where along the inner outline the stair climbs), `kit` (a taxon of pieces), `fill` and
  `bays` (which variant of the kit's bay stands where, "Choosing a bay's piece", below). No surface of
  its own: its storeys own them. Its footprint is solid: a `ground = "pad"` instance per building is
  the town kit's convention, kept.
- **`storey`** (region owner, anchored). `[kind.storey.surface.floor] base = "= level"`,
  `cut = "= headroom"`. Requires `level`. An upper storey's region excludes its stairwell (a
  notch), so a walker on it cannot step into the stair's void.
- **`footing`** (region owner, anchored; K4). `[kind.footing.surface.top] base = "= level"`. The top
  of the wall below, where a storey has no floor under an outer wall: the strip, one wall thick,
  of the side a stair climbs along. That side's wall pieces stand on it ("Building upward",
  below). It is not a floor: nothing ends at it and no room is cut from it.
- **`room`** (region owner). A space inside a storey that the library names for dressing and the
  player map; it owns no surface (the storey's floor is the room's floor). A door between two rooms
  of one storey is no feature: the floor is continuous, the inner wall run leaves the door's width
  open, and the wall pieces' collision is what stops a walker elsewhere; the `check` lint walks the
  floor with `room` outlines as walls except at those openings. Rooms at two heights (a split level)
  are two storeys joined by a `flight`.
- **`doorway`** (path owner, connector `foot`). A short path across the wall line, outside to
  inside. It owns no surface: it lies on the terrain and its end names `<storey>.floor` (the meeting
  of the door). Its effect `sill` sets the ground along it to `sill_z` (the floor's level), so the
  meeting has gap 0 by construction. Upper-floor doors to a balcony: the balcony is a storey-like
  region with a floor; the doorway then owns a `tread` surface at `sill_z` and starts at the balcony's
  floor.
- **`flight`** (path owner, connector `foot`). `[kind.flight.surface.tread]`, `base = "= z0 + (z1 -
  z0) * t"`, `bound = "= width / 2"`. `starts_at = "<lower>.floor"`, `ends_at = "<upper>.floor"`. A
  spiral stair is a flight whose path follows an arc (the station's tower); `surface.fold` guards a
  radius tighter than the half width. A ramp is the same kind with a gentler `z1 - z0` per metre.
- **`footpath`** (path, connector `foot`). The walk from a way to a door. A road (`lib/ways`,
  connector `way`) never ends at a doorway (`surface.connector`), nor at a footpath's start (`way`
  against `foot` is the same error): the footpath's start is left free (a free end is checked for
  nothing, `surfaces.md` 3.4) beside the way, or ends at a point feature there.
- **`roof`** (region owner). `[kind.roof.surface.garden] base = "= eaves"` when the roof is walkable;
  pitched roofs own no surface and are assets.
- **`yard`** (region owner, anchored; K8). The outline of a building of no storeys
  (`storeys = 0`): its walls stand round it on the terrain. It owns no surface and writes no
  field ("A wall on the ground", below).
- **`court`** (a point, anchored; K8). A walled yard and the halls in it, written about one axis:
  `to`, `kit`, `yard`, `hall`, `wings`, and `storey_h`, `plinth`, `porch`, `walk_out`, `roof`
  for its halls. `court.py` returns a `building` for the yard and for each hall ("A court",
  below).

Asset side (packs, `spec/assets.md` 2.7): a door leaf asset has one mount, `entry`, on its doorstep,
`face` 180 (into the building), `connector = "foot"`; a whole-building asset (the college's hall)
has the same `entry` mount, so a walk `ends_at = "hall.entry"` and follows the instance when it is
moved and turned (case `surfaces/moved_turned`). Kit pieces carry `params.pitch` (assets.md 10) and
tags for their role (`role:wall`, `role:corner`, `role:corner_in`, `role:door`, `role:filler`,
`role:roof`, `role:floor`, `role:roof_slope`, `role:roof_gable`), for the kind of storey they are
made for (`storey:ground`, `storey:upper`, `storey:top`) and state (`state:open`). A corner piece may carry `params.reach`, the metres of each
wall it takes ("Sizing a plan to its kit", below). A second `role:wall` piece carries
`variant:<word>` ("Choosing a bay's piece", below). A door leaf the `lay` step places in a doorway
carries no mount with a connector: the doorway ends at the floor, not at the leaf, so a `foot` mount on the leaf would be
`surface.unmet`. The `entry` mount is for whole-building assets placed by hand.

## Steps (through `viasdk` only)

**As built (M5 slice 1).** `walls`, `roof` and `furnish` below are parts of `lay`'s one returned
document, not steps of their own: a produced name is never written in text (R3,
`identity.key_in_text`), so a second step could not write `on = { ref = <storey> }` or `surface =
"<storey>.floor"` for the storeys `lay` returned; inside one document a name resolves to the
document's own keys (`protocol.md` 5). The world declares two steps, `lay` and the lint `check`
(`kinds.toml` shows both). `lay.py` and `check.py` hold their rules in their docstrings; what is
not built yet is listed at the end of this note.

Each is a generate step unless said; each imports only `viasdk` and the standard library, reads
what the step protocol gives it and returns features and instances keyed by the building's name, so
no distant edit shifts a key (`spec/protocol.md` 5, `identity.md` 2.2).

1. **`lay`** (generate, `for_each = "kind:building"`). Reads the building's outline and attributes
   and the terrain (`sample` over the outline's posts). Returns, keyed `<building>_<n>`: one `storey`
   per storey with `level = mean ground + plinth + n * storey_h` and its region (the upper ones less
   the stairwell); one `flight` per storey pair at `stair`, `z0`, `z1` from the two levels; one
   `doorway` per entry of `doors`, its path square to the outline at `t`, `sill_z` its storey's level;
   one `footpath` stub from each ground doorway to the outline grown by 3 m, which a way or a
   hand path may `ends_at`. Moving or turning the building re-runs only this step for that building
   (it read that outline and that ground), and every part follows, so the walk is kept.
2. **`walls`** (generate, after `lay`). Places wall pieces along each storey's outline as runs
   (`spec/rules.md` runs), the run's spacing from the kit's `params.pitch`, leaving each doorway's
   width open and placing the door leaf there; a stretch whose length is not a whole number of
   pitches gets a filler piece, and one that cannot close is a diagnostic naming the lengths that
   would (the TODO's kit contract). A mount the step leaves unanswered on purpose (a blind door in
   the kit) gets its filler here, since the core has no fallback (`surfaces.md` 3.2).
3. **`roof`** (generate, after `lay`). A roof asset scaled to the top storey's outline, or a `roof`
   feature with a `garden` surface; trees on it are a world scatter with `surface`.
4. **`furnish`** (generate). Props on each storey, `at` with `surface = "<storey>.floor"`, kept off
   the flights and doorways by footprint overlap.
5. **`check`** (lint). Reads `meetings()`, `surfaces_at` and the instances: every `doorway` and
   `flight` end met with gap 0 (the core already fails a gap; this lint names the building); every
   room reachable from the terrain by walking floors and crossing only at meetings (`surfaces.md`
   4.4, the reach lint of `lib/review` asked per building); stair going and riser within a profile;
   door `turn` under 45 degrees; kit pieces measured against `params.pitch` (the college's 6.44 m bay
   at a 6 m pitch), a corner against the plan its reach states (`kit_corner`) and a wall piece
   against the storey height of a building that stacks it (`kit_height`); a published object's measured size against its numbers (the pistol: its
   measure reads the state from where the build placed the slide, spec/assets.md 12.4, not from a
   `state:` tag, so a racked slide is not a fault; examples/pistol/measure.py).

## Using it in a world (M5 close, from a fresh agent's house by a footbridge)

- **Python for the steps.** `via config program python <a Python 3.11+ with viasdk>` before
  `via trust` and `via build` (the repo's `sdk/python/.venv` is 3.12); without it the build says
  `protocol.no_program`.
- **Keep doors off a way's bed.** A doorway's `sill` sets the ground at its end to the floor, but a
  way's `bed` effect reaching the same posts may win there (lib/ways: a lane's bed fades out
  `width / 2 + 6` m from its line, a road's `width / 2 + 9`), and the meeting then gaps
  (`surface.gap` names the doorway's end and the storey's floor, both returned by `lay`: change the
  building's place, its `plinth` or `porch`, not those names). Stand each door's end farther from a
  way's line than that.
- **Turned buildings.** Any turn builds: an end that rounding to the millimetre left outside its
  floor is moved onto it (`lay.py` rules 3 and 4, `onto`). It does not yet look right: the kit's
  wall pieces are not turned with the outline, so a turned building's walls show as a saw-tooth with
  open corners (M7 close, TODO). Keep a kit-walled building square to the axes for now.
- **Where the stair goes.** The stairwell runs from 1 m (`LANDING`, or the landing the kit
  states) before the flight's foot to its head. It must reach a corner or stop at least 0.3 m short of it (`STUB_MIN`): a foot 1.0 m or less
  from the corner, or 1.3 m or more. Between, `lay` refuses `stair` (`params`, naming the gap); before
  M7 close it built a stub no piece closes, and the errors named produced corners you cannot write.
- **Levelling the site.** `ground` is the mean of the terrain under the outline, and the doorway's
  `sill` sets the ground at its end to the floor. Any shaped kind under the building (a `terrace`,
  a way's bed) may set those posts instead, and the doorway and floor then gap (`surface.gap`, a
  fresh agent's terrace: 29 mm, M7 close). On a slope, `plinth = 0` keeps the sill from raising a
  mound at the door (the storey's floor cuts the terrain inside, `cut`); put no other ground-shaping
  kind under the doorway. A yard or pad of the building's own is TODO (M7 close).
- **Keeping trees out.** A `building` has no keep-out of its own, and a `clearing` over it puts
  `place.keep_out` on `lay`'s own pieces. Fade the scatter's `density` near it instead:
  `density = "= ... * smoothstep(5, 8, dist('kind:building', 10))"` (station's shrubs). A house
  placed as an instance with `ground = "pad"` (lib/land) needs nothing: the pad writes `keep_out`
  5.5 m past its footprint and `margin`, and every scatter applies `keep_out` itself (`rules.md`
  7.1); a density that also fades by it is `scatter.keep_out_twice`.
- **Placing a building beside something.** A `region` holds absolute points and cannot be placed
  `on` or `at` anything. To have a building follow another feature (a mill beside its weir), a
  small generate step returns the `building` from that feature's points (`step.out.feature`, points
  in millimetres) and a lint names it in `checks` (`authority.unchecked`). A region placed by `on`
  and `face` is TODO (M7 close). **`lay` and `check` then go in a layer above that step**: a step
  never sees what another step of its own layer returned (`authority.md` 4.4), so in the same layer
  `lay` says only `graph.for_each_empty` (an info, in `diagnostics.json` only) and no building is
  laid. The Sonnet `mill` agent met this at M7 close (the Fable one put `lay` a layer up):

  ```toml
  # world.toml: the step that returns the building stays here
  [layer.laying]
  file = "layers/laying.toml"
  rank = 1

  # layers/laying.toml
  [step.lay]
  when     = "generate"
  worker   = true
  for_each = "kind:building"
  command  = ["python", "-m", "viasdk", "lib:building/lay.py"]
  [step.check]
  when    = "lint"
  worker  = true
  command = ["python", "-m", "viasdk", "lib:building/check.py"]
  checks  = ["lay", "<your step>"]
  params  = { kits = ["structure/kit"] }
  ```

  A view on the building goes in that layer too, or a higher one, since what `lay` returns is not
  in a lower layer's view either. A step's own document may name its own keys (`starts_at`, `on`,
  a bookmark's `toward`) as `protocol.md` 5 says; the Godot loader reads a bookmark's `along` the
  same way (`build.md` 15.1).
- **Doors.** With no `role:door` piece in the kit a doorway is open the storey's full height
  (`storey_h`); give the kit a door piece for a lintel.
- **Sizing a plan to its kit** (after the first cut, from examples/kit). A building's `region` is
  the outer face of its walls. A wall stretch is a side less what the corner at each end takes,
  its `reach`: by default the wall's thickness at an outside corner (a post) and nothing at an
  inside one (a post in the notch). Each stretch must be a whole number of bays and fillers.
  - *A rectangle* closes when each side, less two outside reaches, is whole pieces.
  - *A plan with an inside corner* (an L, a T, a court) closes only when the kit has an inside
    corner piece, `role:corner_in`, whose reach with the outside corner's makes a whole number of
    the smallest piece: a 0.3 m post and an inside corner reaching 1.2 m, on a 1.5 m half bay.
    With posts alone the sides either side of an inside corner come out one wall's thickness
    short of the side opposite, whatever the plan's size. Do not close them with a filler as thin
    as the wall: that piece exists for one plan, and the next plan needs another.
  - *An inside corner piece* is placed by the middle of the notch the two walls leave, so its
    pack entry says `pivot = "origin"` and its model's origin is there, with one leg along each
    wall; declare its `footprint` as the L it stands on. `check` compares its measured plan
    (`kit_corner`).
  - *A door* stands where the wall either side of it is whole pieces, and is the kit's door size:
    the door piece's width, less the 0.1 m margin. A wall cut back to close beside a door leaves
    the gap wider, and more than the margin of that is `gap_wide`, whose hint gives the two
    nearest places along the side where the door would close.
  - *A stair* no longer decides where a wall closes: the wall over its stairwell stands on a
    footing and is laid as on the ground ("Building upward", below; before K4 the stairwell's
    length came out of that wall, and what was left either side had to close). It still shapes
    the floor above: where the kit has a floor tile, a stairwell that is not whole tiles deep
    and long leaves the tiles beside it out (`slab_open`, with the stair width that closes).
  - *Write a door and a stair by `side` and `at`*: a side of the outline (side `k` runs from its
    point `k` to the next, counter-clockwise) and the metres along it. They then keep their place
    when another side grows, which `t`, a share of the whole outline, does not. A side is named
    by its number, so a plan that gains a corner renumbers the sides after it.
  - *The kit itself is checked* (`check`, with `kits`): pieces that share no module
    (`kit_module`), corners that cannot close an inside corner a building has (`kit_ell`), a
    filler shorter than the wall is thick (`kit_slip`), wall pieces of more than one thickness or
    height (`kit_uneven`). The contract is `docs/design/kit-contract.md`.
  - *A bay re-exported on another module* fits nowhere, and `lay` closes every wall in fillers:
    `fillers_only` says so. A piece taller or shorter than `storey_h` is `kit_height`.
- **Choosing a bay's piece** (K3 of `docs/plan-kits.md`, from examples/kit). Design note; the rule
  is `lay.py` rule 8. A kit's `role:wall` pieces are its bay (the first with no `variant:` tag) and
  the bay's variants, each tagged `variant:<word>`: a window bay, a vent, an arch. A variant stands
  where a bay stood, so it must be the bay's size: its `params.pitch`, and its measured box
  (length, thickness, height, and where the box lies about the origin), each to 5 mm. The box is
  compared because a re-export changes the model and not the pack's text. One that is not is
  refused: `check` names the piece (`kit_variant`), and where a building asks for it `lay` lays
  the bay and says `variant_none`, as it does for a word no piece carries. Nothing is stretched
  and nothing is swapped for a near fit. A building chooses by rule, never by a pin:
  - *A pattern along the walls.* `fill = [{ storeys = [0, 1], sides = [0, 2], pattern = ["window",
    "wall"] }]`: on those storeys and sides (each `"all"` when left out), the bays of each wall
    stretch take the pattern's words in turn from the stretch's start. `"wall"` is the kit's bay.
    A later entry of `fill` is laid over an earlier one.
  - *No two neighbours alike.* A pattern entry that is itself a list, `pattern = [["window", "vent",
    "wall"]]`, is a choice among those words. It is made from the bay's place alone (the
    building's name, the storey, the side, the distance along it), not from a seed and not from
    the bay's number in its stretch, so the same world lays the same wall and a bay that does
    not move keeps its word. Two bays that touch do not take one piece where the words allow:
    two pieces go in turn along the side; among three or more, a bay whose word is the one the
    bay before it kept takes another. So a bay's piece changes only when a bay beside it with
    the same word comes or goes (in 200 trials of removing a wall's first bay, more than half
    changed nothing else, and none changed more than the bays next to it). A word may stand
    once in a choice (no weights); two words that lay one piece count as one. Bays either side
    of a door, a corner or a half bay do not touch and are not compared.
  - *One bay.* `bays = [{ storey = 1, side = 0, at = 5.0, variant = "window" }]`: the bay that
    holds the point 5 m along side 0. It is an input to `lay`, not an edit of what `lay` returned,
    so when the plan or the module changes, the bay that then holds the point takes it; a point
    on a joint is the bay after it. A point in a half bay, a corner, a door or no wall swaps
    nothing: `bay_none`. `bays` is laid over `fill`, whatever stands beside it.
  - *What is refused outright* (`params`, an error for that building): a storey or a side the
    building has not, a key these tables do not take (`storey` for `storeys` would otherwise fill
    every wall in silence), and `fill` or `bays` on a building with no `kit`.
  - *What has no variants.* Fillers, corners and doors: one piece per role, and for every role a
    piece with no `variant:` tag is read before one with. A window in a half bay
    needs a half-bay window piece and a rule for it (TODO).
  - *Names.* A stretch's bays are one run, named `<stretch>` as before. Where a variant stands in
    it the run names each station's piece (`assets`, `spec/rules.md` 5), so a bay keeps its name
    whichever piece it holds, and a pin on its neighbour keeps its target. Until K5 a run held
    one asset and a stretch was split into a run per spell of one piece, which renamed the bays
    after a swap (`examples/kit`, `window_at`: three names for one piece); R68 ruled the run.
- **What a kit states** (K6 of `docs/plan-kits.md`; R66). Design note; the rule is `lay.py` rule
  11 and `check.py` rule 10; the contract is `docs/design/kit-contract.md`, "What a kit states".
  Source: the studios' accounts of a kit as a system and of metrics written down before the
  first piece (docs/research/Modular kit failures in studios.md, sections 1 and 2).
  - *Where.* The `shared` table of the kit's taxon's `defaults`, in the pack (`spec/assets.md`
    2.5). The core reads it for nothing; `lay` and `check` ask for it with the pieces.
  - *What `lay` lays by.* `wall`: the walls' thickness, in place of the bay's measured box.
    `stair = { lane, width, run, landing }`: a building's stair writes where it is (`side`,
    `at`) and takes the rest from the kit; `lane` is how deep one lane of the stairwell is, in
    place of the stair's width and 0.1 m each side. `door`: a door's width where it writes none.
  - *What `check` holds the pieces to.* `wall`, `module`, `storey` (one height or a list):
    `kit_stated` names a piece that differs. A kit that states nothing is `kit_unstated` (info),
    with what was worked out of its bay and the line that would state it; it is laid as before.
  - *Why the stair.* An opening in a floor is a module too. With the library's own lanes (the
    stair's width and 0.1 m each side) and its 1 m landing, no stair width made the second
    stairwell of a three-storey building whole 1.5 m tiles (`examples/kit`, `tower`: 5.25 m2
    open). A kit whose lane, landing and run are whole tiles gets whole stairwells on every
    storey, and `slab_open` then says how far to move a stair that stands between tiles.
  - *Not yet.* A building's `storey_h` is still its own; lib/town does not read `shared`.
- **The roof beyond the walls** (K8 of `docs/plan-kits.md`, second slice, from examples/court).
  Design note; the rule is `lay.py` rule 17. Our own design: the research found one mention of
  a separate node for layered roofs and no account of deep eaves or hipped roofs from pieces.
  - *Why.* In the timber kit the roof is the main piece: it is hipped and stands 1.35 m beyond
    the posts on every side. Before, with the library as it stood: `roof = "hip"` was `params`
    (three errors, the build stopped); `roof = "gable"` was refused, the kit having no gable,
    and a flat roof laid in its place on the outline; nothing could be laid beyond a building's
    outline at all; and a kit held one slope piece, so one depth of building.
  - *Hipped.* `roof = "hip"`, on a rectangle: the kit's `role:roof_hip` piece on each corner
    (the piece the L's outer corner already takes: one span square, its eaves on its front and
    its +x side), at vertex `i` with its front out of side `i`; and a run of slopes along each
    side between its two hips. The roof is twice the span deep, so the hips of a short side
    meet. Whole pieces or `roof_none`, with the lengths that close.
  - *Eaves.* The kit states them once, `shared.eaves`, because they are the roof pieces' own
    size and no building's choice: a slope made for a hall 9.3 m deep under eaves of 1.35 m has
    a span of 6 m. The roof is then laid on the building's outline grown by the eaves, and its
    `roof` feature has that region. A gable under eaves has its slopes run the roof's whole
    length and its gable halves the eaves in from each end, over the wall; the kit makes that
    half for the slope's span less the eaves, and one of the whole span is refused with the
    span it should have. A gable under eaves over an L is not built.
  - *Pieces by span.* A kit may hold a slope and its ends for each depth of building it roofs.
    Of several slopes of different spans the one whose span is half the roof's depth is taken,
    and the hip, gable and valley of that span; with none, `roof_none` names the depths the
    kit's slopes roof. A kit of one span is laid as before.
  - *`check`.* A hip is held to one span square, a slope to an end of its span (a gable half,
    or a hip), and under stated eaves each slope to a depth a building of the kit can have
    (`kit_stated`: restate the eaves at 0.9 m and both slopes are named, with the depth each
    would roof).
  - *Not built.* Layered roofs (a lower roof round a storey set back above it), curved eaves,
    a hipped roof over an L, eaves under a flat roof, and a building's own `eaves` over its
    kit's.
- **A wall on the ground** (K8, second slice, from examples/court). Design note; the rule is
  `lay.py` rule 18.
  - *Before.* A walled yard was tried as a building of one storey from a kit with a wall, a
    corner and a gate and no floor or roof piece, with three halls standing inside its outline.
    The engine and the library said nothing: no error, no warning. But a storey is a floor, so
    the court had a second surface across it at the mean of its ground, drawn as a slab from
    wall to wall under the halls, and on any slope it would be a level deck with the wall
    standing level on it.
  - *What.* `storeys = 0`: the building is its walls alone. Its outline is a `yard` feature,
    which owns no surface; the walls are laid round it by the same rule as a storey's, on the
    terrain, each piece on the ground under it; a door is a gap with the kit's door piece in
    it, a gate, and returns no doorway. It is the smallest thing that holds a wall and a gate:
    one branch in `lay` and one kind with nothing in it.
  - *Not built.* The base under such a wall on a slope (each piece stands at its own middle's
    height, so on a slope the ends of one piece are above and below the ground by half its
    length times the slope); a wall along an open line that is no outline.
- **A court** (K8, second slice, from examples/court). Design note; the rules are in
  `court.py`'s docstring.
  - *Why.* A compound is one half and its mirror image about an axis. By hand that was four
    buildings and 61 numbers, 32 of them corner coordinates worked out from the kit's bay and
    post and the distances from the gate and the axis, the east hall the west hall's numbers
    reflected by the author, with its sides renumbered so that its far end, not its near end,
    took the doors. Nothing held the two to each other.
  - *What.* A `court` is a point, the gate, and `to`, a point that gives the axis. `yard`,
    `hall` and `wings` are written in bays and in metres along the axis and from it; a wing is
    written once and returned twice. The step returns `building` features, as `row.py` does,
    and `lay` walls them from a layer above. 21 numbers in place of 61.
  - *A door on the axis.* Every front has its door on its middle, so it has an odd number of
    bays where the door takes one: an even number has a post there, and is refused with the
    two numbers next to it.
  - *The image.* The right-hand wing's outline starts at its far end so that side 0 is its
    front; its sides 1 and 3 change places in `fill`; and every `fill` entry of a wing is laid
    mirrored. `test_court.py` lays both and compares them piece for piece.
  - *What stands in what.* The step knows each hall's eaves from its kit and says
    `court_meets` where two halls' roofs, or a roof and the yard's wall, would stand in one
    place. Before, two roofs that met were two `place.crowded` warnings naming roof pieces.
  - *Not built.* A second court behind the first on the same axis; halls that are not across
    or along the axis; `bays` (a pin on one bay) passed to a wing with its place reflected; a
    gate house.
- **Posts and bays** (K8 of `docs/plan-kits.md`, first slice, from examples/temple). Design
  note, written before the code; the rule is `lay.py` rule 15 (`post_bays`, `post_walls`).
  Source: ours. The research found no developer's account of post-and-bay or classical building,
  and reasons only that a visible member on every module line is where a kit hides its seams
  (docs/research/Modular kit failures in studios.md, section 12).
  - *The count.* `N` bays have `N + 1` posts. Every rule before this one lays a side as bays
    and fillers between two corner pieces, so a kit of posts had to hide a post in its bay (a
    panel with its post) and close the side with a filler that is a panel without one: the last
    bay of every side was then not a bay, took no variant, and a door between two posts left
    the wall after it starting with a post, which no piece is.
  - *What.* A kit with a `role:post` piece of its bay's own group is laid by posts. The bay's
    pitch `B` is from one post's middle to the next; the post is `p` wide; the `role:wall`
    piece is a panel `B - p` long. A corner is a post. A side closes at `N * B + p`.
  - *Two runs.* The panels: `N` members over the stretch widened by half a post at each end. A
    run's members stand evenly over its span, so a piece shorter than its pitch leaves the post
    its room, and no run needs a pitch of its own. The posts: `N - 1` members at the joints.
  - *A door* stands in one bay, between two posts, and the kit's door piece takes the panel's
    place, so its middle is the bay's middle; otherwise the error names the middles.
  - *No fillers.* A side that is not whole bays is `wall_open`, with the two lengths next to it
    that close.
  - *What `check` holds.* A panel to its pitch less the post's width (`kit_pitch`); a corner
    that states no reach to the post's width (`kit_corner`).
  - *The floor* (second slice, from examples/court, the first building walled by posts). A bay
    is from one post's middle to the next, so a floor of whole tiles is too: a storey walled
    by posts is tiled to the middle of its line of posts, half a post in, whatever its panels'
    thickness, and where a veranda stands before it the platform's tiles come to that same line
    from outside. Tiled inside its walls' inner face, as a storey of whole wall pieces is, a
    hall of five bays was 14.7 m of floor between posts, no whole number of any tile but the
    post's own width, and its veranda 2.7 m: 11 tiles of the platform could not be laid. The
    platform's own edge is its base's thickness in, where the kit has a base.
  - *Not yet.* An inside corner (refused, `params`); bays of two widths on one side (a wider
    middle bay), which a run of one pitch cannot hold.
- **The colonnade** (K8, first slice, from examples/temple). Design note; the rule is `lay.py`
  rule 16. Source: ours, as above.
  - *The structure.* In a temple, and under the deep eaves of a timber hall, the roof's outline
    is not the walls': a line of posts stands on the building's outline, the walls stand inside
    it, and one roof covers both. Before this rule a building's walls, floor and roof were all
    laid on its one outline, so the room inside was the `building` and every column was an
    instance written by hand, on the terrain: a height written outside the room is
    `surface.none_at`, since no surface holds the point (`examples/temple/by_hand.toml`).
  - *What.* `colonnade = [{ sides, depth }]`: on those sides the walls stand `depth` inside
    the outline, from the ground storey. It is rule 14's moved outline, on the ground storey,
    with all four sides free to move. One storey, a rectangle.
  - *The platform.* The building's whole outline is a `platform` feature at the ground storey's
    level, and the posts, the base and the platform's own tiles stand on it. **Changed from the
    first design**, which made each strip between the two outlines a feature of its own, as a
    terrace is: a colonnade turns the corner, so a lintel beside a corner post would stand on
    two strips. One surface under the whole colonnade, with the room's storey over part of it
    at the same height, is what R70 already calls one level.
  - *The posts* are the kit's pieces tagged `storey:colonnade`, laid by posts and bays along
    the outline where the walls left it: a whole side, and on a side the walls keep, the
    platform's end beside it, from the corner post to the face of the wall's own corner.
  - *A door through it* is two doorways on one axis. The first ends on the platform at the
    outline's edge, and must pass between two posts: a front with an even number of bays has a
    post on its axis, and the error names the bays' middles. The second starts on the platform
    and ends on the room's floor, through the wall's opening. **Found by trying:** the core
    needs nothing more. A doorway whose path lies on a kind surface is accepted, its end meets
    the room's floor with gap 0, and its start is a free end, checked for nothing
    (surfaces.md 3.4). `check` walks it from the platform (rule 1), so a room behind a platform
    with no way onto the platform is `unreached`.
  - *The roof* covers the outline, and its ends stand a post's width in, over the colonnade.
  - *Not yet.* Two storeys; a plan that is no rectangle; eaves beyond the posts; a doorway's
    steps up a podium (the doorway holds the ground at its sill, as on every building: K7).
- **A mirrored fill** (K8, first slice). `fill = [{ ..., mirror = true }]` reads its pattern
  from both ends of a side toward its middle (`lay.py` rule 8). Before it, a pattern was read
  from each stretch's start: an even side read `wall, niche, wall, niche`, and the wall after a
  door on the axis was the wall before it moved along, not turned about. Symmetry about an
  axis is what a classical front is; it is one word here because the choice of a bay's piece
  was already a function of the bay's place.
- **Every size from one unit** (K8, first slice). A classical kit takes every length from one,
  the column's width. `shared.module` could not say it: since K6 it is the bay, and `check`
  holds every `role:wall` piece to it, so a kit that stated `module = 0.9` was told its 1.8 m
  bay, its niche bay, its base and its 2.7 m lintel were each wrong (`kit_stated`, four times
  on the first build of `examples/temple`; a fifth there was of the lintel's height). So the unit is its own key, `shared.unit`, which `check` holds
  every stated size and every wall piece's measured plan to (`kit_unit`, check.py rule 11).
  `module` keeps its meaning.
- **K8's one independent review** (2026-11-23), and what it changed. Each mend has the case
  that showed it in `test_k8fix.py`.
  - *A stair beside a wall of posts.* The flight was set off the panel's thickness and the
    crossing read off the post's width, so a stair took every panel it ran along, on both
    storeys, and nothing was said. The wall's band is the post's width for all three: the
    stair's lane, the footing over the stairwell, and what counts as crossing.
  - *A door behind a colonnade* is held twice on one axis, by the colonnade's posts and by the
    wall behind. It was refused by whichever rule met it first, in the wall's own measure, and
    was offered places the other rule then refused. It is one error now (`through_door`), in
    the measure the door was written in, and it offers only the places both take.
  - *A gable half under eaves* is chosen by the slope's span less the eaves. It was matched to
    the slope's own span, so a kit with the right half was told it had none.
  - *The colonnade's end* on a side the walls keep is as long as the colonnade is deep:
    `wall_open` named the side's length as what to change, and now names `depth`. A `depth`
    less than a post is refused as that.
  - *The platform's tiles.* With no base they started a post's width in, and a veranda one
    bay deep before a room of posts could not be tiled. They start on the middle of the
    colonnade's line of posts there, as that kit's floors do; and `slab_open` names the depth,
    or the side's length, that would be whole tiles, where it had named a sum that was not.
  - *A door on a court's axis* (`court.py`): an odd number of bays left over was said to be a
    joint on the middle, which is false of a door two bays wide. It says how many are left.
  - *A post is chosen by its storey's height,* as a panel is; `check` holds it (`kit_height`),
    and holds a hipped building's pieces too.
  - *How short a stretch may be.* K8's second slice let a stretch close up to 5 mm short, for a
    court turned off the axes; a yard side 4 mm short then closed with its pieces in each
    other and no word but the core's. A stretch may be 5 mm long (`FIT`) and 1.5 mm short
    (`FIT_SHORT`: two corners rounded to the millimetre), for walls, bays and eaves alike.
  - *A mirrored fill is an image only where the bays are.* A side with a half bay at one end,
    or a door off its middle, is laid by the same reading and said (`mirror_none`).
- **A gable roof over an L** (K7 of `docs/plan-kits.md`, third slice). Design note, written
  before the code; the rule is `lay.py` rule 10, "Over an L" (`ell_plan`). Source: the forum rule of
  thumb that a roof kit is built like a hallway kit (middle, inner corner, outer corner, end
  cap) is all that was found; no developer's account of pitched-roof pieces on an L was
  (docs/research/Modular kit failures in studios.md, section 4, "Roofs depend on the whole
  plan"). The decomposition is our own.
  - *The plan.* An L whose two wings are one depth, twice the slope piece's span `s`. Its
    outline has six sides and one inside corner. Take the square of side `2 s` where the wings
    cross. The two ridges meet at its middle.
  - *The corner square is mitred along its diagonal,* from the L's outer corner to its inside
    corner. The half of the diagonal from the outer corner to the middle is a hip; the half from
    the middle to the inside corner is a valley. Each wing's roof is an ordinary gable roof up
    to that diagonal.
  - *Two new pieces.* `role:roof_hip`: one span square, climbing from its two outer edges to
    the hip, placed at the L's outer corner; the eaves run on from it with no gable there.
    `role:roof_valley`: the span-square quarter beside the inside corner, where the two inner
    slopes meet along the valley, and `params.reach` of each inner eave beyond the corner, as
    the wall's inside corner piece reaches along both walls. It is the same from both wings
    (it is its own mirror about the diagonal), so one piece serves, placed by the inside corner.
  - *Runs.* Each outer eave: slope pieces from one span along (past the hip) to the wall's
    thickness short of the wing's free end, where two gable halves stand. Each inner eave: from
    the valley piece's reach past the inside corner to the same. The other two quarters of the
    corner square are ordinary slope pieces of the outer runs.
  - *Closure.* On a kit whose bay is 3 m, post 0.3 m and span 3.3 m, an outer eave is
    `3 a + 0.6 - 0.3 - 3.3 = 3 a - 3`, whole 1.5 m pieces. An inner eave is the inside side,
    `3 a + 0.6 - 6.6`, less the wall's thickness at the free end and the reach: `3 a - 6.3 -
    reach`, which a reach of 1.2 m closes. It is the reach the wall's inside corner piece has on
    the same kit, for the same reason. This is what K4 found and filed: slopes beside a valley
    are whole pieces only if the valley piece takes a stated length of each eave.
  - *Refused, with its reason* (`roof_none`): wings of two depths; a plan with more than one
    inside corner, or that is neither a rectangle nor an L; a kit without the hip or the valley
    piece; an eave that is no whole number of slope pieces, with the lengths that would close.
  - *As built* (`examples/village`, `ell`): an L of 12.6 by 9.6 m is 14 slopes, four gable
    halves, the hip and the valley, covering the outline once, measured on a 0.1 m lattice.
- **A storey with its own outline** (K7 of `docs/plan-kits.md`, second slice, from
  examples/block and examples/village). Design note; the rule is `lay.py` rule 14. Source: ours;
  the set-back of a city block and the jetty of a timber house are named in no developer's
  account read.
  - *What.* `setback` and `overhang`, each `[{ storey, sides, by }]`: from that storey up, those
    sides stand `by` metres inside or outside the building's outline. A storey then has its own
    outline and every rule is laid on it: its walls close on it, its floor is tiled inside it,
    the roof covers the top storey's.
  - *What is refused.* An outline that is no rectangle; two sides that meet at a corner; the
    side a stair climbs and the two that meet it, so the stairwell stays where it is.
  - *The terrace.* Where a storey stands inside the one below, the strip between is a `roof`
    feature at its level, tiled, with the kit's parapet on its three outer sides.
  - *Nothing is fitted.* A move that leaves a side no whole number of pieces is `wall_open`. And
    on a kit laid from its walls' outer face a set-back of whole bays leaves a terrace one
    wall's thickness short of whole tiles, with side parapets two posts short of whole pieces
    (`examples/block`, `setback`); a jetty smaller than the smallest piece does not close and a
    deeper storey has no roof piece (`examples/village`, `jetty`). Each is said.
  - *Not yet.* A terrace round a corner; an outline that is no rectangle; a piece that closes a
    terrace or a jetty.
- **The wall's foot** (K7 of `docs/plan-kits.md`, first slice, from examples/village). Design
  note; the rule is `lay.py` rule 13. Source: the stone base under one studio's modular buildings
  and the foundation layer of another's facades are the only shipped accounts, and flat ground
  is a stated precondition of the tools that say anything (docs/research/Modular kit failures in
  studios.md, section 4, "Meeting the ground"). The rule is our own design.
  - *What was wrong.* A floor is level and a wall piece stands on its floor, so wherever the
    ground is lower than the floor the wall's foot is in the air: 0.3 m, the plinth, round every
    building on level ground since M5, and up to 0.87 m along 126 m of exposed wall on the village's
    slope. Nothing said so.
  - *The base.* A kit's wall pieces tagged `storey:base` are laid round the outline as a
    storey's walls are, under the door too, on the ground storey's floor. The pack sinks each by
    its own height, so its top is the floor. No new way of placing: a sunk piece on a floor.
  - *What is said.* `lay` reads the ground along the outline every 0.5 m. `base_short` where the
    ground lies lower than the base reaches; `foot_open` where a kit with no base stands over
    ground that falls more than the plinth; `plinth_bare` (info) where only the plinth shows.
    Nothing is stretched to reach the ground.
  - *Not yet.* Courses of base stacked to a depth; steps to a door (the doorway shapes the
    ground at its sill); a yard or pad that shapes the ground under the building.
- **A row of buildings** (K6 of `docs/plan-kits.md`, from examples/block). Design note; the rules
  are `row.py`'s eight, `lay.py` rule 12 (`faces`) and rule 10 (the parapet). Source: a city block
  as lots along a frontage (docs/research/Modular city asset contracts.md, section 2); the
  studios' accounts of leftover length, of neighbours that may not repeat and of a change at a
  district's border (docs/research/Modular kit failures in studios.md, sections 7, 8 and 11).
  The row itself is our own design.
  - *Why.* One block written by hand was twelve `building` tables and 192 numbers, each a sum of
    bays and posts worked out by the author; and a region cannot say what stands beside it, so
    312 windows and shopfronts faced a neighbour's wall and nothing said so.
  - *A row.* `kind = "row"`: a point (`at`) and `to`, the frontage; `kit`; `depth` in bays;
    `lots = [{ bays, storeys, door }]`. The step `row.py` returns one `building` for each lot,
    sized from the kit's module and its corner's reach, wall to wall from `at`, on the left of
    the way to `to`. `lay` then walls them: it runs over `kind:building`, so a world declares it
    in a layer above the `rows` step.
  - *Closure.* The lots are summed against the frontage. Where they differ the row says how far
    (`row_open`) and, where that is whole bays, how many. Every lot is still returned.
  - *What a side faces.* Each building gets `faces`, a word for each side: `front`, `back`,
    `party` (a neighbour's wall lies along it) or `end` (the open end of a row). A `fill` entry
    may name `faces` in place of `sides`, so one line on the row blanks every party wall. A
    building written by hand may write `faces` itself.
  - *Heights.* A lot that writes no `storeys` takes one of the row's `heights`, never its
    neighbour's where the list holds two different numbers, the same in every build.
  - *Door and stair.* The door stands in a bay of the front; the stair is the kit's, along the
    back, or along a side where the back is too short for a landing, the run and a landing past
    its head. A lot no side of which holds the
    stairwell is refused (`row_stair`).
  - *Two rows at a corner.* A lot that stands where another row's does is refused and said
    (`row_meets`). The block has no feature of its own yet: the author still writes where each
    side's row starts and ends.
  - *The parapet.* A kit's wall pieces tagged `storey:parapet` are laid round a flat roof as a
    storey's walls are, on the roof.
  - *Not yet.* A side only part of which a neighbour stands against is one word: where that
    part is less than half, its windows stay (`examples/block`: 4). A block as one feature.
- **Building upward** (K4 of `docs/plan-kits.md`, from examples/kit). Design note; the rules are
  `lay.py` rules 2, 6, 9 and 10. Source: a facade read as base, ground storey, repeating storeys
  and top (docs/research/Modular city asset contracts.md, section 5); the studios' accounts of
  pieces that stop stacking and of roofs as the part a kit covers last
  (docs/research/Modular kit failures in studios.md, sections 5 and 9). The roofs here are our own
  design: the research found no developer's account of roofs from pieces.
  - *Storeys of two heights.* `ground_h` and `top_h` on a building (0: as `storey_h`) are the
    heights of its ground storey and its top storey; the storeys between repeat at `storey_h`. A
    building of one storey has a ground storey only; of two, a ground and a top.
  - *Pieces by storey.* A storey takes the pieces of its own height: of each role, the pieces
    nearest the storey's height stay and the others are left out there, so one kit serves a
    building with a 4 m ground storey and one whose ground storey is 3 m, and a bay re-exported
    a little too tall is still its storey's bay and not replaced by one of another storey. Among those, a piece tagged `storey:ground`, `storey:upper` or `storey:top` is laid on
    that kind of storey only, and is read before an untagged piece of its role; an untagged piece
    serves every storey with no piece of its own. So a kit states a shopfront bay, a taller door
    and a taller corner for the ground storey and nothing else changes. Every rule of a kit
    (module, reach, variants) is read per storey from the pieces that storey gets. `check` holds
    each piece to the height of the storey it serves (`kit_height`, which so also says a storey
    whose nearest piece of a role is of another height), the bays of all storeys to one thickness
    (`kit_uneven`: walls stack flush or not at all, and `lay` refuses two thicknesses outright),
    and says a kind of storey the kit has no bay for (`kit_bay`).
  - *A wall stands on the wall below, not on the floor.* Where a stairwell opens the floor beside
    an outer wall, the storey above has no floor under that wall, and `lay` built none (6 m open
    in examples/kit). Now the side the stair climbs along gets a `footing`: a region as wide as
    the wall is thick along that whole side, with one surface at the storey's level, which the
    wall pieces of that side and its two corners stand on. The storey's own region keeps off it:
    that side is moved in by the wall's thickness, and the stairwell is a notch from there, held
    off the walls beside it by one wall's thickness, or by what the corner piece there takes
    where that is more. The stair therefore no longer decides where the wall above closes: each
    bay above stands over the bay below. Needs a kit (with none there is no wall and the notch
    is cut from the outline as before) and square corners at both ends of that side, to the
    2 mm a corner rounded to the lattice may be off, so a rectangle turned off the axes has its
    footing; where a corner is not square there is none, and `wall_open` says the side stands
    open over the stairwell. A corner piece stands on the footing where its placing point lies
    in it (a post at an outside corner of the stair's side), else on the floor (an inside corner
    piece, a wide pier). A stairwell that would come within a wall's thickness of another wall,
    or out of the building, is refused.
  - *Names.* A wall run is named by its storey and the outline's side: `<storey>_w<side>_<j>`,
    `j` counting the stretches a door cuts the side into. On a ground storey this is the name it
    always had. On a storey over a stairwell the sides after the stair's were numbered by the
    notched ring's edges and are renamed once by this change.
  - *Floor slabs.* A kit's `role:floor` piece is a square tile one module on a side, its top at
    the floor and its thickness below it (`sink`). `lay` tiles each storey's floor inside the
    walls: the outline moved in by the wall's thickness, on a grid from the first corner of side
    0. A plan sized to a kit whose outside corners take the wall's thickness is whole tiles
    there, where the kit's module is a whole number of tiles (`kit_deck`). A tile is laid only
    where it lies wholly on the floor, so a stairwell that is not whole tiles wide and long
    leaves the floor short of it: `slab_open` (warning), with the stair width that would make
    that stairwell whole tiles deep. On a building turned off the axes no tile is laid, and
    `slab_open` says so once: tiles are instances on the millimetre lattice, and off the axes
    two that abut overlap by the rounding, which the core takes for two things in one place.
    Walls do not have this trouble, since neighbours of one run are never compared (TODO).
  - *Roofs from pieces.* Every roof piece stands at the eaves (the top storey's level plus its
    height), on the surface of a `roof` feature over the footprint. `roof = "flat"`: the floor
    tiles again, laid as on a floor; the walls' tops stand round them. `roof = "gable"`, on a
    rectangle: slope pieces from each long side's eave to the ridge, and a gable under each end.
    A slope piece (`role:roof_slope`) is a wedge that states `params.pitch`, its length along the
    eave, and `params.span`, the plan distance from the eave to the ridge; its measured height is
    the roof's rise. It is made for one depth of building, twice its span, as the roof pieces of
    real kits are. The slopes run between the gables: along each long side less the wall's
    thickness at each end, which must be whole pieces. A gable half (`role:roof_gable`) is a
    wall as thick as the kit's, one span long, rising to the ridge; two stand on each short
    side, the second turned about. The ridge lies along the longer sides (the first, of equal
    ones).
  - *A roof that cannot be laid is refused with its reason* (`roof_none`, warning): a gable on a
    plan that is not a rectangle; a depth no slope piece's span is half of; an eave that is not
    whole slope pieces between the gables; a kit without the pieces. The building then gets the
    flat roof where the kit has a floor tile, and the message says so; else none. Nothing is
    stretched or cut to fit. A rectangle is four sides each square to the next, to the same 2 mm
    as a footing's corners, so a turned one takes its gable; a flat roof on a turned building is
    refused, as its floor tiles are. A gable over an L is not built here: its valley and the slopes
    beside it need the roof's module to agree with the inside corner's reach, which is a rule
    of its own (K7 of the plan, pitched roofs over plans that are not rectangles).
- **Walking to an upper floor.** A storey, flight or doorway `lay` returns has a produced name,
  which text may not write (R3), so a bookmark cannot name it as `toward`. Place your own kindless
  point features `on = { ref = "<building>", t, offset }` (they follow the building when it moves
  or turns; `offset` is metres to the left of the ring's travel, inward on a region, `geometry.md`
  2.6), list them in a kindless path, and give the bookmark `along = "<that path>"`. From a step,
  `step.out.feature(key, at=...)` with no `kind` writes a kindless point or path; a kindless path
  is a guide, so it raises no `geom.unowned_crossing` against `lay`'s doorways and flights
  (`geometry.md` 2.7; a path of a kind of your own raises one at each). Put a point in
  the flight's own line before its foot, or the walker skips the foot (`build.md` 15.1), and judge
  the walk by `on.stop` in `sheet.json`, not by `arrived` (arrival is in plan only). The walker
  takes a point once it is nearer the next point than that point is (`build.md` 15.1): from a door,
  a stair's head is often nearer than its foot, so the foot is skipped. Add a turning point between
  them, so that from where the walker stands each point is nearer than the one after it (door,
  turn in front of the foot, foot, head, the far side of the upper floor). The walker starts on the
  highest floor under the bookmark's eye (`build.md` 15.1): a walk up starts from a bookmark on the
  ground floor (at the door), and a walk down from one on the upper floor. A path that climbs from
  a bookmark already standing on the upper floor walks off it and `fell` (M7 close). Two bookmarks,
  one per direction, each with its own path.
- **A place on a laid floor (R65).** `at = [x, y, z]` stands a hand instance, a pin, a set or a
  point feature on the floor at that point whose height is nearest `z`, with no name written: a
  window pinned in place of a bay, a view on an upper floor (`examples/station`, `loft_top`), a
  chair in a laid room. The two notes below were written before it and still work.
- **A view on an upper floor.** A storey's floor is a produced name, so a bookmark cannot write
  `surface = "<storey>.floor"`. Lift the eye instead: `eye_height = storey_h + plinth + 1.7`
  (station's tower views); the walker still starts on the storey's floor. A way to stand a view on
  a produced storey is TODO (M7 close).
- **Asking about it.** `via query` runs no step (`surfaces.md` 7): what `lay` returned is in the
  build's `features.json` and `meets.json`, not in `via query`'s answers.

## What the M5 worlds need from it

- `station`: four tower storeys and a cottage storey; three spiral flights; six doorways; lamps and
  furniture with `surface`. The walk test (challenge 15) is the `check` step's reach.
- `college`: whole-building assets with `entry` mounts and walks to them; the gatehouse
  `passable`; cloister and wing runs from the kit with `params.pitch`.
- `coast`: no building; its cave is a `lib/terrain` kind with a floor surface and `cut`
  (`surfaces.md` 6.5).
- `market`: rows of houses from the town kit, now with corners, ends and fillers from `walls`.

## Not here

The core's rules are not restated: if this note and a chapter disagree, the chapter wins and this
note is fixed.

## Not built yet (M5 slice 1)

- **Engine:** kind surfaces and mounts (`doc.not_yet`, NotYet(3)), `connector` on a kind, a kind
  surface as a place, `surface` on an `at` placement: the library is written to the spec and its
  world test is skipped naming them. Two engine gaps against `protocol.md` seen by running `lay` on
  the current binary (a scratch copy with surfaces taken out): a list or table attribute
  (`doors`, `stair`, `rooms`) reaches the step with `<attr>?` = 0 instead of canonical JSON text
  (4.3), and a returned document's name does not resolve to the document's own keys first (5,
  "Names": `ends_at = "hall_d0.start"` is `identity.unknown_place`, whose hint asks for the
  produced name R3 forbids in text).
- **Library:** `furnish`; doors above the ground storey (a balcony); rails along a stairwell's
  open sides; a pitched roof over a plan that is not a rectangle (a gable from pieces is laid on
  rectangles since K4, "Building upward"; the outer wall over a stairwell, listed here until then,
  stands on its footing); a plinth course and steps to a door;
  the walk lint over kind surfaces in `lib/review` (the SDK has `meetings()` but not `surfaces_at`
  or `sample`, so `check` judges reach by meetings and room openings, not by posts); the measured
  object against its published numbers (the pistol).
