# station

A lighthouse and its keeper's cottage, inside (docs/plan.md M5: "indoors reachable through a door";
challenges 14, 15). The spike's station written again in the v1 vocabulary on lib/building's kinds:
every floor is a kind surface of a `storey`, and every way between floors is a meeting
(spec/surfaces.md 4.4, 6.4). Four round tower storeys joined by three spiral `flight`s, each upper
storey leaving out its stairwell; a cottage storey of four rooms 0.15 m above the store, joined by
the `tower_step`; two `doorway`s in from the ground. Written by hand: lib/building's `lay` lays
rectangular buildings with straight flights. Beside them, the net loft is one `building` feature
that `lay` lays: two storeys of a primitive kit, a door, a straight flight, three rooms.

| File | What |
|---|---|
| `world.toml` | Storeys, rooms, flights, doorways, footpaths; wall runs ending 1.1 m clear of each door and the step; dressing as sets, runs and hand instances with `surface`; the net loft, one `building` laid by `lib:building/lay.py`; the building check (`lib:building/check.py`, measuring each door's clear width). |
| `pack.toml` (kit) | `kit_wall`, `kit_filler` (a 0.1 m slip), `kit_corner` under `structure/kit`, with `role:` tags and `params.pitch`, for `lay`. |
| `pack.toml` | Primitives for the furniture and walls (the shipped packs have none); `*_top` ids lowered onto table tops. |
| `layers/views.toml` | Bookmarks at each door and inside every room; one out of the front door, one down the step, two up the loft's flight. |
| `changes/move_stair.toml` | `stair_2` and its stairwell 30 degrees on: both meetings hold, every room reached. |
| `changes/close_passage.toml` | `tower_step` no longer ends at the passage: the tower's 4 storeys and 4 rooms reported unreached. |
| `stair_points.json` | Points for `via query surfaces_at`. |
| `expect.toml`, `measure.py` | Measures, bands and protocol. |

## Build

```
export VIA_LIB=$PWD/lib
via trust examples/station/world.toml --yes --change examples/station/changes/move_stair.toml --change examples/station/changes/close_passage.toml
via build examples/station/world.toml --out build/station
via query examples/station/world.toml surfaces_at --in examples/station/stair_points.json --out build/station.sat.json
via build examples/station/world.toml --change examples/station/changes/close_passage.toml --against build/station --out build/station+close_passage
python3 examples/station/measure.py build/station --surfaces build/station.sat.json --check base
engines/godot/sheet.sh --before build/station --after build/station+close_passage --out review/station
```

## Known faults (M5 slice 2 tree)

Measured with `measure.py` and read on the sheet (`engines/godot/sheet.sh`, both changes):

- Every view's `toward` target on a floor gets the terrain's z in `features.json` (`lens` 0.01,
  `telescope` 0.01, `stair_2.end` 0.01, `kitchen_table` -0.01), so each camera in the tower looks
  steeply down through its floor at the ground. The eye itself stands on its floor.
- No walk climbs a spiral flight: the sheet walks a straight line toward `toward`, and a chord of
  a 122 degree spiral leaves the tread. The net loft's straight flight is climbed (`loft_foot`,
  0.31 m to 3.51 m), and `down_step` crosses the step into the store.
- With the passage closed the walker still crosses the step (`down_step` arrives in the store):
  a closed door is the meeting taken away (the lint reports the tower unreached), not a leaf.
- Stacked flights cross in plan: 10 `geom.unowned_crossing` warnings.
- lib/building's `lay` leaves no gap in a wall between rooms for a flight: with the loft's cut at
  x = 49.3 the walk up its flight stopped at 2.89 m against it. The cut is now west of the flight.
