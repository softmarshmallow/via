# monastery

A walled compound on a shelf above a gorge (docs/plan.md, M3), the spike's `monastery` rebuilt in
the v1 vocabulary on the new core. 320 m at 1 m posts. The author writes the land (its rise, the
shelf, the gorge by the stream's line), the stream, the tarn's point, the bridge's two ends on the
stream, the road by what it connects and where it turns, the compound as one `enclosure` with its
gate as a point on the outline, and what stands inside. The tarn's level, the road's course between
its turns and the wall are derived.

| File | What |
|---|---|
| `world.toml` | The land and the gorge (one formula of the stream's line), the stream (lib/water `beck`), the tarn (`tarn`, lib/water's fill step), the crossing `bridge` (lib/ways), the switchback `road_up` (routed through six named turns, `turn_1` to `turn_6`), the compound `court` (lib/town `enclosure`, new) with its `gate` (lib/town `gate`, new), the approach; the inline `winter` change. |
| `layers/compound.toml` | The hall, three cells, two garden terraces, lamps, the paint, eight views at the meetings, the reach lint with its sealed pairs. |
| `changes/` | `guesthouse` (the spike's), `move_bridge` (19 m downstream), `no_bridge` (the bridge removed: is it the only way over?). |
| `expect.toml`, `measure.py` | Road grade and turns, deck, wall and gate, reach, what a change kept; with protocol. |

## Build

```
export VIA_LIB=$PWD/lib
via config program python $PWD/sdk/python/.venv/bin/python
via trust examples/monastery/world.toml --yes                      # and once per --change
via build examples/monastery/world.toml --out build/monastery
via build examples/monastery/world.toml --change examples/monastery/changes/move_bridge.toml \
    --against build/monastery --out build/monastery+move_bridge
via build examples/monastery/world.toml --change winter --against build/monastery --out build/monastery+winter
sdk/python/.venv/bin/python examples/monastery/measure.py build/monastery+move_bridge \
    --base build/monastery --check move_bridge
engines/godot/sheet.sh --before build/monastery --after build/monastery+move_bridge \
    --out review/monastery --option water=tag:water_level
```

A cold build takes about 3 s, a change about 2.5 s.

## What it shows (2026-11-15)

- R16: the road is written by what it connects (`road_entry`, `bridge.start`) and the places it
  turns, by name (`turn_1` to `turn_6`; M4, `path` takes names); the course between is found. Built, it climbs 31 m over 603 m; its steepest 10 m is 10.1 %, and
  6 m of it climb over 10 % (measure.py's protocol). Until slice 4 the first two legs ran on top of
  each other (0.27 m apart at (256, 20); the lattice walk runs each leg flat first, then
  diagonally): the first turn moved from (228, 18) to (228, 12) and the place (240, 26) makes the
  second leg climb at once; `road_self_near` counts what is left (the (300, 44) turn's legs, 4.8 m
  apart).
- R18: one crossing owns its deck and its two ends; the road arrives at its start and the approach
  leaves its end by name. `move_bridge` moves it whole: the road keeps its course up to the last
  turn (2082 of 2393 points to the millimetre) and only the last leg is found again; the approach
  follows; 0 `geom.unowned_crossing`; 0 deck posts wet.
- The compound is one region of lib/town's new `enclosure`: the court levelled; its wall (M5 slice
  3) a run of wall bays round the outline, open where the gate stands (5.1 m). 0 metres of wall
  uncovered away from the gate; the step across the gate is 31 cm.
- Reach (lib/review): from the road's entry a walker reaches both bridge ends, the gate, the hall's
  step, the well and every door (the guesthouse's too, in its change); the gorge's floor is sealed
  from the road and from the hall. `no_bridge` removes the crossing and states the hall, the gate,
  the well and the west rim sealed from the road, and it holds: the bridge is the only way over.
  The same statement with the bridge standing gives 4 `not_sealed`, so the lint can fail.

## What tripped (the spike's LOG, and this build)

- **The gorge was walkable twice** (`not_sealed`, the spike's most expensive item too). First the
  crossing's `heads` held the ground within a 1:2 bank of the deck out to 12 m beside the deck, a
  ramp into the gorge: lib/ways now holds it only behind each end and a metre past the deck's edge
  (townland's expect still passes, deck_wet 0). Then a wall eased over 3 m, and over 0.6 m, was
  walked obliquely down a row of posts nearly parallel to it; the gorge's wall is now a step from
  one post to the next. `not_sealed` still gives no place: a throwaway walk over the build's fields
  found both routes.
- **A held grade cannot be said.** A route's cost is per post, the same whichever way the course
  crosses it, so it cannot tell a climb across the slope from one straight up it. The author places
  the turns; with four turns the course bowed round the lumps and climbed 12.3 % for 102 m.
- **The first leg ran along the south edge**: an edge post's slope is read one-sided, so the cost
  there is lower; the cost now rises within 8 m of the extent.
- **`deck_z` is typed** from `via eval` (as the spike's bridge was); `move_bridge` writes it again.

## Known faults (read from the sheets, on foot)

- (Fixed, M3 slice 5 ways) The approach's last metres ran 0.31 m a metre into a 1:2 lip at the
  bridgehead and the stream pooled 0.22 m against both faces of the deck: lib/ways' crossing now
  brings ways to the deck at `approach` (1:10; `head_grade` 0.10) and keeps the stream at its
  `flow` past the faces (`face_pool` 0.06).

- **The deck is a surface over the gorge** since M5 slice 2 (no longer a causeway; 11 wet posts
  under it), but lib/review's reach walks the terrain only, so the road no longer reaches the gate
  (`reach` 3 of 10, OWED in expect.toml, as in townland).
- **The wall is a run of wall bays** since M5 slice 3 (`court_wall`, the starter pack's `wall_bay` at
  scale 1.5: 2.4 m high, 98 bays): masonry the engine draws and collides, owned by the run, the gap
  5.1 m where the span ends at the gate (`bm_gate`, `bm_court`). No coping, no gatehouse; bays at a
  corner overlap or leave its outer corner open.
- **The gorge's walls are sheer faces** since M5 slice 3 (layers/gorge.toml, lib/land `sheer_cliff`
  in 16 pieces, written by gorge_faces.py): drawn along their line, the terrain's step cut away
  under them. Before, the terrain drew every wall stair-stepped at the 1 m posts (`bm_tarn`; 606
  quads within 20 m of the stream spanning more than 2 m, the worst 21.15 m); now 68 and 2.81 m,
  all in the tarn's head, where each face's flat foot (2.75 m) fades into the hollow. Seen on the
  sheet: a saw-tooth of open slivers along the brink (a cut quad reaches up to 0.71 of a post past
  the face's strip, coast's fault too), thin lit wedges where a piece's strip bends, and the
  bridge's line crosses each face unowned (2 `geom.unowned_crossing`: a crossing names only the
  water it crosses).
- **The switchbacks do not read from eye height** (`bm_turn`): the legs lie 30 m apart on open grass.
- **No bell tower, no well, no huts**: the starter pack has no such assets; the hall is a barn.
- **`no_bridge` floods the floor** at the bridge site in its `bm_gorge` view (seen, not measured).
