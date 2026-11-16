# station

A lighthouse and its keeper's cottage, inside (docs/plan.md M5: "indoors reachable through a door";
challenges 14, 15). The spike's station written again in the v1 vocabulary on lib/building's kinds:
every floor is a kind surface of a `storey`, and every way between floors is a meeting
(spec/surfaces.md 4.4, 6.4). Four round tower storeys joined by three spiral `flight`s, each upper
storey leaving out its stairwell; a cottage storey of four rooms 0.15 m above the store, joined by
the `tower_step`; two `doorway`s in from the ground. Written by hand: lib/building's `lay` lays
rectangular buildings with straight flights.

| File | What |
|---|---|
| `world.toml` | Storeys, rooms, flights, doorways, footpaths; wall runs; dressing as sets, runs and hand instances with `surface`; the building check (`lib:building/check.py`). |
| `pack.toml` | Primitives for the furniture and walls (the shipped packs have none); `*_top` ids lowered onto table tops. |
| `layers/views.toml` | Bookmarks at each door and inside every room. |
| `changes/move_stair.toml` | `stair_2` and its stairwell 30 degrees on: both meetings hold, every room reached. |
| `changes/close_passage.toml` | `tower_step` no longer ends at the passage: the tower's 4 storeys and 4 rooms reported unreached. |
| `stair_points.json` | Points for `via query surfaces_at`. |
| `expect.toml`, `measure.py` | Measures, bands and protocol. |

## Build

```
export VIA_LIB=$PWD/lib
via trust examples/station/world.toml
via build examples/station/world.toml --out build/station
via query examples/station/world.toml surfaces_at --in examples/station/stair_points.json --out build/station.sat.json
via build examples/station/world.toml --change examples/station/changes/close_passage.toml --against build/station --out build/station+close_passage
python3 examples/station/measure.py build/station --surfaces build/station.sat.json --check base
engines/godot/sheet.sh --before build/station --after build/station+close_passage --out review/station
```

## Known faults (M5 slice 1 tree)

- `surface` on a set and a run `on` a storey stand on the terrain (102 instances off their floor);
  bookmarks with `surface` too, so every view inside is at ground height.
- `place.overlap` reads plan footprints only: things on different storeys clash, a roof clashes
  with all under it (no roofs here), and 10 tower shell pieces give way to furniture on upper floors.
- Stacked flights cross in plan: 16 `geom.unowned_crossing` warnings.
- The loader draws no floor and the terrain is not cut (no `cut` file, no floor meshes yet).
- With the passage closed, `meets.json` still lists `tower_step.end` at the cottage floor, while
  the lint's `meetings()` has 11 rows.
