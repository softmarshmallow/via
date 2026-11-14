# quay

A small harbour town on a 256 m slope down to water (docs/plan.md, M3). The land rises north from a
harbour; a waterfront (`quay`, lib/town's `waterfront`) owns the bank and the streets end on it
(R18); one main way (`high_street`) comes down from the gate to a named point on the quay, written
by its ends and routed over the land (R16); lib/town's streets step lays contour streets, cross
streets and streets down to the quay; the core nodes them into the network `town` (blocks) and cuts
the lots (`lots`); the houses step puts a house on each lot that fits one. Six views stand at the
meetings (R17): way and water, street and waterfront, street and street, lot and street, way and
street, and the town from the gate.

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
via build examples/quay/world.toml --change examples/quay/changes/move_hill.toml --against build/quay --out build/quay+move_hill
python3 examples/quay/measure.py build/quay+move_hill --base build/quay
engines/godot/sheet.sh --before build/quay --after build/quay+move_hill --out review/quay_move_hill \
    --option water=tag:water_level --option paths=kind:main_street,kind:street,kind:waterfront
```

## Known faults (M3 report)

- Houses are not levelled: a pad from the houses' layer changes `height`, which the streets step
  below reads, and that view is owed by the model (`doc.not_yet`). Every house is `place.relief`.
- The quay reads as a flat earth apron on foot: the street paint covers the deck where the main
  way and the streets end, and no wall face shows from eye height (`way_water`, `street_quay`).
