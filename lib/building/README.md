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
  run }`, where along the inner outline the stair climbs), `kit` (a taxon of pieces). No surface of
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
tags for their role (`role:wall`, `role:corner`, `role:door`, `role:filler`, `role:roof`) and
state (`state:open`). A door leaf the `lay` step places in a doorway carries no mount with a
connector: the doorway ends at the floor, not at the leaf, so a `foot` mount on the leaf would be
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
   at a 6 m pitch); a published object's measured size against its numbers (the pistol: its
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
  floor is moved onto it (`lay.py` rules 3 and 4, `onto`).
- **Doors.** With no `role:door` piece in the kit a doorway is open the storey's full height
  (`storey_h`); give the kit a door piece for a lintel.
- **Walking to an upper floor.** A storey, flight or doorway `lay` returns has a produced name,
  which text may not write (R3), so a bookmark cannot name it as `toward`. Place your own kindless
  point features `on = { ref = "<building>", t, offset }` (they follow the building when it moves
  or turns; `offset` is metres to the left of the ring's travel, inward on a region, `geometry.md`
  2.6), list them in a kindless path, and give the bookmark `along = "<that path>"`. Put a point in
  the flight's own line before its foot, or the walker skips the foot (`build.md` 15.1), and judge
  the walk by `on.stop` in `sheet.json`, not by `arrived` (arrival is in plan only).
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
