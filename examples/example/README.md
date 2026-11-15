# example

A village in a hollow, 256 m square: a tarn that finds its own shore, a beck down the middle with a
footbridge over it, a crag with fallen rock at its foot, a green, two farm plots and an orchard,
six houses, a mill and a farm. It is the canonical project: one of everything a user writes, each
in the place it belongs. The spike's `example` (spike/worlds/example) showed the same things; this
is a new world in the v1 vocabulary (spec/vocabulary.md) on the libraries in `lib/` and the
starter pack. M3 (docs/plan.md).

## Read it in this order

| File | What it shows |
|---|---|
| `world.toml` | The world file, which is also the layer `world` (rank 0). Top: `name`, `extent`, `seed`, `include` (libraries by name, never by path), `pack` (assets are bound by the pack; a world never names a file). Then `[layer.*]` (the two other documents, each with an owner and a rank), `[let]` (names any formula reads), the fields (`role = "surface"` is the terrain; `role = "keep_out"` is "not here" for scatters; `type = "weights"`, `role = "material"` is the paint), a `then` row on a field (the hollow under the tarn), the land's features, the ways, the open ground, the fill step, and the inline change `winter`. |
| `lib/hollow/kinds.toml` | This project's own kinds, as a library: `include = [..., "hollow"]` finds it in `lib/` beside the world file. `plot` paints earth and keeps growth off; `lair` raises this project's own field `danger` and shares lib/map's output `spawn_zone`. A kind writes fields by role or tag, never by name. |
| `layers/hamlet.toml` | Layer `hamlet` (rank 1): what is built. Instances placed by hand, each `on` the thing it belongs to (a lane, the green), with `ground = "pad"` to level their ground; one placed `at` a point with a bearing. The project's own generate step `moisture`. |
| `tools/moisture.py` | A generate step of this project, through the same SDK the libraries use: it reads the ground and the water below its layer and returns a field `wet`. |
| `layers/dress.toml` | Layer `dress` (rank 2): fields that read `wet` (`cover`, `danger`), four scatters (by a field, by a taxon path `@plant/bush`, `within` an outline, at a cliff's foot by `sdist`), the game's data (`lair`, `poi` by point, by name and on a produced shore), thirteen views (`bookmark`), three lint steps, and two inline changes, a clear and a pin. |
| `tools/homes_near_lane.py` | A lint step of this project: every home within 30 m of a lane. |
| `changes/` | Change files: `third_cottage.toml` (a house kept apart), `move_tarn.toml`, `move_beck.toml`. |
| `measure.py`, `expect.toml` | Measures of a build, and the bands each build must fall in, with the protocol. |

### Things to notice

- **Every table has an id** (`[feature.tarn]`, `[instance.mill]`, `[field.ground.then.rock]`), so a
  change names exactly one thing and the order of tables never matters.
- **Meetings are owned by a named thing** (R18). The lane is two ways and a crossing: `footbridge`
  (lib/ways `crossing`) owns its deck and its two ends; `lane_west` ends at `footbridge.start`,
  `lane_east` starts at `footbridge.end`. The two ends are points `on` the beck, so when the beck
  moves the bridge, both lanes and every house on them follow (`move_beck`). The church path
  starts at a named point on the lane, `church_foot`.
- **Derived things are found, then stood on.** The tarn is a point; lib/water's `fill` step finds
  the level its hollow runs over at and returns a lake, `fill#tarn_lake`. Text never writes that
  name (R3): the shore view and the map's name stand on it by `{ of = "fill", class = "feature",
  near = [70, 190] }`, so they follow it when the tarn moves.
- **Layers refer downward only.** The terrain's hollow reads the tarn, which is why the tarn is in
  `world.toml`. `cover` reads `wet`, which a step in `hamlet` returns, so `cover` is declared in
  `dress`. A step's `command` path is relative to the document that declares it
  (`../tools/moisture.py` from `layers/`).
- **A season is a change.** `winter` lowers the treeline, removes the meadow row and adds a snow
  row after the others (`order = 1`): it merges key by key, so it restates nothing.
- **Unknown words pass through.** `crop = "barley"` is declared by the project's `plot` kind and
  reaches `features.json` for the game; the engine never reads it.

## Build

```
export VIA_LIB=$PWD/lib
via config program python $PWD/sdk/python/.venv/bin/python     # the steps want 3.11+
via trust examples/example/world.toml --yes                      # and once per --change
via build examples/example/world.toml --out build/example
via build examples/example/world.toml --change examples/example/changes/third_cottage.toml \
    --against build/example --out build/example+third_cottage
via build examples/example/world.toml --change winter --against build/example --out build/example+winter
python3.11 examples/example/measure.py build/example+third_cottage --check third_cottage
engines/godot/sheet.sh --before build/example --after build/example+third_cottage \
    --out review/example_third_cottage --option water=tag:water_level
```

`--change` takes a path (it holds `/` or `.`) or the name of an inline change (`winter`,
`clear_one`, `pin_one`).

## Changes

| Change | Where | What must follow (expect.toml) |
|---|---|---|
| `third_cottage` | changes/ | One house and its pad; 3 trees and 2 bushes on the pad go; nothing moves; the fill step is reused. One warning, `doc.change_adds_near`: `cottage_c` is a near name of `cottage_a` (a typo guard), meant here. |
| `move_tarn` | changes/ | The tarn's point moves 30 m east; the hollow moves with it, both steps run again, the lake, its name and its view follow (centre 70, 190 to 98, 202). |
| `move_beck` | changes/ | The beck moves up to 10 m west; the bridge, both lanes, the church path, the seven buildings on the lanes, the mill on the beck (`on`, `face = "toward"`, since slice 4) and their pads, and the views on them follow; the walk still crosses the bridge; the tarn is untouched (fill reused). |
| `clear_one` | inline, layers/dress.toml | Six fallen rocks within 10 m of [205, 212] go; nothing else. |
| `pin_one` | inline, layers/dress.toml | A shed in the orchard stands; the one orchard tree it overlaps goes. |
| `winter` | inline, world.toml | Snow paint; the treeline at 40 m takes 220 of 273 trees; no step runs again. |

Numbers (protocol in expect.toml; one build each): cold base 6.9 s, each change 5.9 to 7.0 s.
Base: 680 instances, 0 errors, 0 warnings, reach 24 of 24 pairs (every door, the row's over the
bridge, every view and the crag's top; the tarn's point sealed), deck 0 wet posts.

## Gaps: what the vocabulary cannot say yet here

Written down, not worked around in the core:

1. **`run` and `set` are read only from M3** (`doc.not_yet`, crates/via-doc schema). The spike's
   row of houses (`kitrun`), its lamps and stalls (`run`) and its well with a bench (`set`) would
   be `[run.east_row]`, `[run.lamps]`, `[run.stalls]` and `[set.well_corner.item.*]` with
   `[instance.square] set = "well_corner"`. The row is written house by house (`row_1` to
   `row_4`, each `on` the lane), the well alone; lamps, stalls and the bench are left out.
2. **A change file cannot hold a hand edit that speaks to a producer above `world`.** An entity
   a change file adds is declared in layer `world` (documents.md 6), so `[pin.shed] by =
   ["orchard_trees"]` in a change file is `doc.upward_ref`. documents.md leaves "which layer a
   change file's added entities belong to" open until "the first example whose change file must
   add a table naming something above rank 0": this is that example. Until it is decided, the
   clear and the pin are inline changes in `layers/dress.toml`.
3. **The starter pack has no church, well, bench, stall or inn**, and a pack cannot extend
   another (a project pack copies every file, as vale's `revised` does). The church and the inn
   are `house_wide`, the well `box_small`.
4. **An instance carries no data of its own** (the spike's `mill_wheel = true`): instance keys
   are closed. Data for the game goes on a feature (`poi`, `lair`).
5. **The deck is a stand-in on the terrain** (lib/ways `crossing`, until kind surfaces): the beck
   does not run under it; from the bank it reads as a causeway. `deck_z` is a number measured from
   a build (38.5 m), not a formula of the banks, so a moved beck keeps the old level (`move_beck`:
   0.7 m from the deck's end to the lane 4 m out).
6. **No soft reach for a view's own clearance**: a bookmark writes no keep_out (lib/review), so a
   view had to be moved off a tree (`under_bridge`).

## Known faults (walk of 2026-11-15)

Read from the sheets at every view (scratch renders, not committed):

- `under_bridge`: the footbridge is a grey causeway across the beck, since the stand-in deck is
  terrain (gap 5). (Fixed, M3 slice 5) The beck no longer pools against it: depth 1 m off a face
  0.55 to 0.30 m, as 15 m off (lib/ways crossing `pass`). Water still cannot run under terrain.
- `lane_bridge`: (Fixed, M3 slice 5) the dip before the deck where the lane's bed met the
  bridgehead's 1:2 bank: the lanes now reach the deck at 1:10 (`approach`; steepest metre of the
  last 20 m 0.49 to 0.10). The lane's own cut-only bed still sags 0.47 m below the deck 7 m back.
- `east_row_view`: the four row houses stand in a line, doors to the lane, gables a metre or two
  apart: a row by hand, not a terrace (houses.py's rows stand wall to wall).
- `under_crag`: the crag reads as a grey wall with a flat top, the cliff kind's scarp.
- `on_green`: the green is a level court but painted grass like the field round it, with a grey
  smear along its north side not yet traced; nothing on it but the well (the stalls are a `run`).
- `fields`: from east of the farm the barley's earth is a thin brown band beyond the grass; the
  plots lie lower than the lane and the view sees little of them.
- `winter`: snow everywhere but the water and the lanes; no snow on roofs or trees (the loader
  draws the pack's models as they are).
