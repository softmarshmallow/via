# college

A hillside college that grew court by court (docs/plan.md, M5; challenges 20 and 48), the spike's
`college` rebuilt in the v1 vocabulary. 320 m at 1 m posts. The author writes the land, four courts
(lib/town `terrace`), the ways by what they connect (lib/ways `road`), seven buildings as whole
assets of the college's own pack, six runs of kit bays along their lines, and each door's walk by
what it connects: a named place by a way, and the building's door,
`ends_at = "<building>.entry"` (lib/building `footpath`). Nobody writes where a door is in the world.

| File | What |
|---|---|
| `world.toml` | The land, the courts, the ways, the buildings, the walks, the kit runs, trees, lamps, a view at every door, lib/review's reach lint and lib/building's check. |
| `packs/college.toml` | Primitive assets: five buildings with an `entry` mount (`face = 180`, `connector = "foot"`), a bell tower whose mount has no connector, a `passable` gatehouse, three kit bays with `params.pitch` (the cloister's passable). |
| `changes/` | `move_and_turn` (the pavilion moved 18 m and turned a quarter), `no_walk` (the chapel's walk removed). |
| `expect.toml`, `measure.py` | Doors met, walks at their doors, turns, every kit joint, what a change kept; with protocol. |

## Build

```
export VIA_LIB=$PWD/lib
via config program python $PWD/sdk/python/.venv/bin/python
via trust examples/college/world.toml --yes
via build examples/college/world.toml --out build/college
via build examples/college/world.toml --change examples/college/changes/move_and_turn.toml \
    --against build/college --out build/college+move_and_turn
sdk/python/.venv/bin/python examples/college/measure.py build/college+move_and_turn \
    --base build/college --check move_and_turn
engines/godot/sheet.sh --before build/college --after build/college+move_and_turn \
    --out review/college_move_and_turn
```

No `--option paths=…`: the walks are painted (`earth`), the ways `setts`. A cold build takes
under a second, each change about 0.65 s (expect.toml's protocol).

## What it shows (2026-11-16, M5 slice 1)

- **Doors that paths find, by name (challenge 48).** Five doors are asset mounts with
  `connector = "foot"`; each walk ends at one by name. All five meet, gap 0, connectors equal; the
  meeting's door faces into the building and the core reports each walk's `turn` (0 for four,
  36.5 degrees for the pavilion's diagonal walk). The bell tower's mount has no connector, so no
  walk is owed there.
- **A building moved and turned keeps its walk (M5 acceptance).** `move_and_turn` writes only
  the pavilion's `at` and `face`. Its walk's end follows the door to (197.5, 92), 0 mm off where the
  pack and the instance put it, and now arrives square (turn 0). Nothing else moved: 1 instance,
  1 feature (the walk), 19 of 20 meetings unchanged; the change report names the pavilion as the
  cause of the walk and of the view at its gate.
- **A forgotten walk (`no_walk`).** With the chapel's walk removed the build warns
  `surface.unmet` on `instance.chapel`, and the hint is the line to write:
  `ends_at = "chapel.entry"`. Nothing else changes.
- **Kit bays measured against each other (challenge 20).** Each run places its bay at the bay's
  pitch along a line a whole number of pitches long. Of 40 joints between neighbours, none gapes;
  the east wing's seven overlap by 0.44 m each, the bay's published fault (6.44 m wide at a 6 m
  pitch), which lib/building's check names (`kit_pitch`). The three cloister runs meet at their
  corners without overlapping (`place.crowded` 0).
- **Passable.** The gatehouse and the cloister bays are `solidity = "passable"`; lib/review's reach
  lint now treats a passable footprint as ground (`surfaces.md` 4.4 item 4). Reach: from the
  arrival a walker reaches the four court places and all five doors.

## Known faults (read from the sheets, on foot)

- **The gatehouse is a closed box over the approach.** A primitive has no arch, so `bm_arch`'s eye
  (1.7 m above the road at y = 42) stands inside it, and from `bm_gate` the road runs into a wall.
  The reach lint walks through it (passable) but there is nothing to see; and it is the one
  `place.keep_out` warning: it stands on the approach's verge, and rules.md 8.3 reads no `passable`.
- **No door is drawn.** Every walk ends against a blank wall (`bm_armoury`, 5.5 m from it, sees only
  wall); the mount is a point, the box has no opening.
- **Walls do not stop the loader's walker.** The primitive pack gives every asset
  `collision = "none"`, so the sheet's walk to `bm_wing` passes through a bay ("through 1 without
  collision"): neither solid nor passable can be judged by the loader here.
- **The chapel's door is 0.58 m below the court its walk leaves** (`walk_rise_max`): the chapel's
  pad levels its footprint at its own mean, off the court's terrace. The meeting holds (gap 0: a
  path end lies on the ground wherever that is), so nothing but the measure says so.
- **The wing reads as one block.** The overlapping bays draw as one long box; the 0.44 m overlap is
  seen only in the numbers.
- **The dormitories float and sink.** The dormitory runs stand straight up the slope with no pad,
  each bay at the ground under its origin (no `max_relief` on the bay, so nothing warns).
- **The reach lint does not prove the arch.** The college has no wall: a walker can go round the
  gatehouse, so `9 of 9` would hold with the gatehouse solid too.

## What the vocabulary could not say

- **A door on a whole-building primitive.** The `entry` mount is only a point; there is no way to
  say "an opening here" on a primitive, so the model shows none.
- **A building's walk that keeps square.** A walk starts at a fixed place; after a turn the arrival
  angle is whatever the new door makes with it (`turn` reports it; lib/building judges it over
  45 degrees). The pavilion's change was chosen so the turned door faces the walk's start.
- **A gate across a way.** `keep_out` is the only "not here" (rules.md 8.3), and a way's verge
  writes it, so a passable gate astride a way always warns.
- **A building standing on a court.** A pad levels to its own mean; the court terrace levels to the
  court's. Saying "stand at the court's level" needs the terrace's `level` typed (`set_level`) or
  the building inside the terrace's region; neither follows the building when it moves.
- **A lint subject for a pack asset.** A step may not name `asset.<id>` (diagnostics.md 1.1 has
  only world entities), so lib/building's `kit_pitch` is reported on `step.check` with the asset in
  `data.asset`.
