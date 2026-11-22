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
  `cut = "= headroom"`. Requires `level`. An upper storey's region excludes its stairwell (a notch
  or a hole), so a walker on it cannot step into the stair's void.
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

Asset side (packs, `spec/assets.md` 2.7): a door leaf asset has one mount, `entry`, on its doorstep,
`face` 180 (into the building), `connector = "foot"`; a whole-building asset (the college's hall)
has the same `entry` mount, so a walk `ends_at = "hall.entry"` and follows the instance when it is
moved and turned (case `surfaces/moved_turned`). Kit pieces carry `params.pitch` (assets.md 10) and
tags for their role (`role:wall`, `role:corner`, `role:corner_in`, `role:door`, `role:filler`,
`role:roof`) and state (`state:open`). A corner piece may carry `params.reach`, the metres of each
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
- **Where the stair goes.** The stairwell runs from 1 m (`LANDING`) before the flight's foot to its
  head. It must reach a corner or stop at least 0.3 m short of it (`STUB_MIN`): a foot 1.0 m or less
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
  - *A stair* takes its stairwell's length out of the wall of the storey above (1 m before its
    foot to its head): what is left either side must close too. Where it does not, `wall_open`
    says the stretch ends at the stairwell and how far it is off.
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
  - *Names.* A run holds one asset (`spec/rules.md` 5), so a stretch with variants is one run per
    spell of one piece: `<stretch>` for the first, `<stretch>b<k>` for one that starts at bay
    `k`. A stretch of the bay alone is named as before. **Swapping one bay renames the bays after
    it in its stretch** (`examples/kit`, `window_at`: one piece changes, three names do), so a pin
    on one of those is left without its target. This is the run's limit, not the kit's (TODO,
    `spec/rules.md`; ruled at K5).
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
- **Library:** `furnish`; doors above the ground storey (a balcony); the outer wall along an upper
  storey's stairwell (no floor there for a piece to stand on: needs a two-storey piece in the kit,
  `role:wall_tall`); rails along a stairwell's open sides; a pitched roof scaled to the outline;
  the walk lint over kind surfaces in `lib/review` (the SDK has `meetings()` but not `surfaces_at`
  or `sample`, so `check` judges reach by meetings and room openings, not by posts); the measured
  object against its published numbers (the pistol).
