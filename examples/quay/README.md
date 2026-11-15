# quay

A small harbour town on a 256 m slope down to water (docs/plan.md, M3). The land rises north from a
harbour; a waterfront (`quay`, lib/town's `waterfront`) owns the bank and the streets end on it
(R18); one main way (`high_street`) comes down from the gate to a named point on the quay, written
by its ends and routed over the land (R16); lib/town's streets step lays contour streets, cross
streets and streets down to the quay; the core nodes them into the network `town` (blocks) and cuts
the lots (`lots`); the houses step fills each lot's frontage with a row of town houses wall to wall, on one level
terrace per row. Eight views stand at the
meetings (R17): way and water, street and waterfront, street and street, lot and street, way and
street, the town from the gate, and two at the quay wall (along its top, and at its face from the
water).

| File | What |
|---|---|
| `world.toml` | Layer `world`: the land, the harbour, the quay, the main way, the district, the streets step, `[network.town]`, `[cells.lots]`, the lint. |
| `layers/town.toml` | Layer `dwellings` (rank 1): the houses step, the ground paint, the views. |
| `changes/` | `move_hill` (the crest moves 80 m west), `clear_one`, `pin_one` (a shed on a lot), `unrelated` (the district's east edge comes in 6 m). |
| `expect.toml`, `measure.py` | R13's four obligations and the counts, measured. |

## Build

```
export VIA_LIB=$PWD/lib
via config program python $PWD/sdk/python/.venv/bin/python     # the steps want 3.11+ and numpy
via trust examples/quay/world.toml --yes                         # and once per --change
via build examples/quay/world.toml --out build/quay
via build examples/quay/world.toml --change examples/quay/changes/move_hill.toml --against build/quay --out build/quay+move_hill \
    --diagnostics build/quay+move_hill.diagnostics.json            # the run's records too
python3 examples/quay/measure.py build/quay+move_hill --base build/quay
python3.11 examples/quay/measure.py build/quay+move_hill --base build/quay \
    --run build/quay+move_hill.diagnostics.json --check move_hill
engines/godot/sheet.sh --before build/quay --after build/quay+move_hill --out review/quay_move_hill \
    --option water=tag:water_level
```

No `--option paths=…`: quay paints its own ways (the `street` and `shore` rows), and the loader's
paths overlay draws each chosen path as a brown strip its width wide, which hides the deck's gravel.

## Town-lib pass (2026-11-15)

Rows of `house_row` (7 m, stone) and `house_narrow` (5 m, render) stand wall to wall along each
lot's frontage, fronts on the footway's back edge (the street's own `keep_out`), doors on the
street; gaps fall at lot edges (0.5 m) and junctions. Each run stands on one `terrace`. Streets are
setts kerb to kerb (4.5 m, main way 6.5 m) with flagged footways (1.5 m, 2 m); the deck and 3 m of
quayside are flagged, the wall face stone. Base: 123 houses on 74 of 135 lots (was 52 on 52),
`place.keep_out` 0 (was 7), `place.relief` 9. The 61 empty lots are corner triangles and wedges no
8.6 m deep house fits, and corner houses inside a cross street's keep_out.

## Known faults (M3 slice 2 walk, 2026-11-15)

- (s4 town pass) 8 `place.relief` in base: on a down street each house of a row is its own floor,
  and the step under the party wall falls within one 0.5 m post cell of the footprint, so the
  exact relief reads part of the neighbour's floor. A raised floor (`doors_off`) has no steps.
- (s4 town pass) 60 of 135 lots still empty in base: by the frontage frame, 16 triangles, 4
  quads and 36 polygons whose frontage and depth would hold a house but no rectangle stands inside
  the outline (or the corner house falls in a cross street's keep_out), and 5 too shallow. A
  shallower `fallback` asset (houses.py rule 2b, `house_low`) filled none: it is 8 m wide. A
  corner house facing both streets is owed.
- (s4 town pass) `move_hill`: `d105` now runs down from `s0` and ends on the main way where it
  crosses it (streets.py step 6), a street of 23 m between the way's hairpin and s0.
- Dead ends are lib/review/form.py's (`dead_end_count` 0, `edge_ends` 5 in base: ends on the
  district's edge and at the gate are counted apart, form.py "Dead end and edge end").

- (Fixed) Lots facing the quay beside a cross street were 12.4 m deep: the cross street's first
  station cut ran square across the quay's band (geometry.md 3.6 step 5 now stops a cut at the
  corner's bisector, case `cells_corner`). Base now has 52 houses, 9 of them fronting the quay.
- 9 houses still show `place.relief` on their terraces; the houses step leaves a run out where
  its terrace would move the land past `max_cut` (houses.py rule 7), a prediction, not the core's
  own check.
- The paint's edge to the grass blends over about a metre at 0.5 m posts: crisper than the earth
  smudge, not a kerb line.
- The deck read as earth on the slice-2 walk because the sheet command drew the paths overlay
  over it, and because the shore row sorted before the street row by id (fixed: `order = 1`).
- The wall face shows from the water (`wall_face`) only as a low grey band; there is no face.
- The main way's foot cuts a 1:2 cone into the harbour floor (`field.effect_cut`, lib/town: a
  path's length has no name in a kind's formula).
- The route cost is convex in the land's slope but not in the course's grade: on an even fall the
  way still runs straight down; with the hill moved it winds round the hill's east side with
  small zigzags `smooth` leaves.
- `move_hill`: `s0` ends on the main way where the way runs along the contour, so the east half of
  the town has no streets, only lots along the way.
- (Fixed, s4 town pass) Doors do not meet the footway: each terrace now stands at the footway's
  height read 2.5 m out from its doors (houses.py rule 8), cut into steps where the footway climbs
  more than 0.6 m, and carries a metre of flagged apron out to it; base 103 of 125 doors within
  0.3 m of the ground 3 m out (`doors_level`, measure.py), 12 more than 1 m off: rows whose floor
  rule 7 raised (up to 1.5 m) to keep the terrace's cut within `max_cut`, which have no front
  steps yet. Was: each row's terrace is level at its own
  ground, not at the footway's, so 1 m out from the door the land already falls or rises 0.5 m and
  on the footway (3 m out) it is more than 1 m off the floor for 91 of 123 doors in base (59 above
  the street, 32 below; 10 within 0.3 m). On foot the doors stand over grass banks.
- (quay-walk) `street_quay` is written at a point, so in `move_hill`, where no street reaches the
  quay at x = 198, it looks into the quay row's back walls; it does not follow a street end.
- (quay-walk) Rows wall to wall close their blocks: the reach lint's walker finds 12 separate
  pieces of open ground in base (back gardens no one can walk into), though all 132 stated pairs hold.
