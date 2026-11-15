# quay

A small harbour town on a 256 m slope down to water (docs/plan.md, M3). The land rises north from a
harbour; a waterfront (`quay`, lib/town's `waterfront`) owns the bank and the streets end on it
(R18); one main way (`high_street`) comes down from the gate to a named point on the quay, written
by its ends and routed over the land (R16); lib/town's streets step lays contour streets, cross
streets and streets down to the quay; the core nodes them into the network `town` (blocks) and cuts
the lots (`lots`); the houses step puts a house on each lot that fits one. Eight views stand at the
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

## Known faults (M3 slice 2 walk, 2026-11-15)

- (Fixed) Lots facing the quay beside a cross street were 12.4 m deep: the cross street's first
  station cut ran square across the quay's band (geometry.md 3.6 step 5 now stops a cut at the
  corner's bisector, case `cells_corner`). Base now has 52 houses, 9 of them fronting the quay.
- 9 houses on pads still show `place.relief`; the houses step leaves a lot empty where its pad
  would move the land beside it past `max_cut` (houses.py rule 6), which is a prediction, not the
  core's own check.
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
