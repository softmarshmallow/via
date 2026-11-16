# county

The town as a fill on simulated land (docs/plan.md M4; R8, R9, R11; composition.md 8 and 12). The
land is this world's: downs rising north from a bay, a river down a valley into it. The town is
`examples/quay`, built apart and imported by file (not a copy). It reads this land through `give`
(its `land`), so its streets and houses stand on the county's ground; its generate steps run in the
county's build (`town.step:streets`, `town.step:houses`). The road from the north edge ends at the
town's exit `town.gate` (R9).

| File | What |
|---|---|
| `world.toml` | Rank 0: the land (height formula, the river, the bay's water), `[import.town]` with `give = { land = "height" }`, the road to `town.gate`. |
| `layers/views.toml` | Rank 1, owner `views`: the views at the join and at the exit (composition.md 6.5). |
| `changes/move_town.toml` | The town moved 24 m east, 6 m north and turned 8 degrees west; nothing else is written, so the road and views follow. |
| `changes/forgotten_exit.toml` | Asks for the town's change `east_gate` and answers nothing: `import.exit_unanswered` with the line to write. |
| `expect.toml`, `measure.py` | The measures and their bands, with the protocol they were taken by. |

## Build

```
export VIA_LIB=$PWD/lib
via trust examples/county/world.toml
via build examples/county/world.toml --out build/county
via build examples/county/world.toml --change examples/county/changes/move_town.toml --against build/county --out build/county+move_town
python3 examples/county/measure.py build/county --check base
python3 examples/county/measure.py build/county+move_town --run build/county+move_town.run.json --check move_town
engines/godot/sheet.sh --before build/county --after build/county+move_town --out review/county
```

Cold build about 15 s (release, M4 slice 4 tree, one run, other agents building); an unchanged
rebuild about 3.5 s, all of it preparing the imports to make the build key (TODO).

## Known faults

- Only 2 of quay's 5 streets are laid on the county's land: the `lot_street` view stands on grass.
- The paving in `street_street` is visibly lumpy.
- The erosion step is not run on the county's land (skipped for the box).
- A road of the county's cannot be given to the town (given features, TODO From the M4 close).
