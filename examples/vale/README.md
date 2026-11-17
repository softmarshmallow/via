# vale

A mountain valley 512 m square: a road edge to edge, a trail up the east side, a tarn that finds its
own shore, a mill pond held to its shape, a hamlet of four houses and a cliff on the north-west wall.
The spike's `vale` (spike-final:spike/worlds/vale), rewritten in the v1 vocabulary on the libraries in `lib/`
and the starter pack (`lib/packs/starter.toml`). M2's first world (docs/plan.md).

| File | What |
|---|---|
| `world.toml` | The land and what is decided on it (layer `world`, rank 0): fields, the cliff, the tarn and its fill step, the pond, the road and trail, the houses, the season `winter`. |
| `dress.toml` | Layer `dress` (rank 1, owner "dressing"): the scatters, the player's map marks, and the views. It writes nothing the land reads. |
| `changes/` | One change file each, below. |
| `packs/revised.toml` | The starter pack with `house_low` re-exported wider (`packs/make_revised.py`), its facts file the old one. |
| `expect.toml`, `measure.py` | Measures of a build compared with the spike's (fixtures/oracle/vale*/). |

## Build

```
export VIA_LIB=$PWD/lib
via config program python $(command -v python3)
via trust examples/vale/world.toml --yes            # and once per --change, as each is a different build
via build examples/vale/world.toml
via build examples/vale/world.toml --change changes/move_road.toml --against examples/vale/build/vale
python3 examples/vale/measure.py examples/vale/build/vale --check base
engines/godot/sheet.sh --before examples/vale/build/vale --after examples/vale/build/vale+move_road \
    --out review/vale_move_road --option water=tag:water_level --option paths=kind:road,kind:trail
```

## Changes

| Change | From | What must follow |
|---|---|---|
| `winter` (inline) | spike variant `winter` | `snowline` to 34 m and no turf, so the brush that grows on turf thins; nothing else made again. |
| `changes/move_road.toml` | spike `move_road` | The road swings west; the houses, the trail's foot, the green and the views on the road follow. On the moved road the west side is the valley wall, too steep for two pads (`authority.max_change` on `barn` and `cottage_a`, as the spike's `seam.change_exceeds`), so the change crosses those two to the east side. The fill step is reused (it reads no road). |
| `changes/add_cliff.toml` | spike `add_cliff` | A cliff across the east wall and a view of it. It crosses the east trail: `geom.unowned_crossing`, and on foot the trail runs up the face. Kept as a fault the build names. |
| `changes/bigger_house.toml` | spike `bigger_house` | `cottage_b` swapped to `house_wide`; its pad grows. |
| `changes/bad_asset.toml`, `bad_cycle.toml`, `bad_overlap.toml`, `bad_scarp.toml`, `bad_on_road.toml`, `outside.toml` | spike `invalid`, one fault per file | `asset.unknown`; `graph.cycle`; `place.overlap`; `authority.max_change` (the hut's pad cannot level the scarp); `place.keep_out` (a warning); and nothing for a spawn zone wholly outside the world (`geometry.md` 2.8: the kernel reports nothing about the extent). |
| `changes/new_seed.toml` | M2 acceptance | A new world seed moves the valley's noise, so the hollow, the tarn's shore and whatever stands on it. |
| `changes/revised_house.toml` | M2 acceptance | `asset.revised` names every instance of `house_low` (`cottage_a`, `cottage_b`, `cottage_c`); nothing else changes. |
| `changes/move_tarn.toml` | M2 acceptance | The tarn's point moves: the fill step reads it, so it runs again and the lake, its point of interest and its view follow. |
| `changes/dress_only.toml` | M2 acceptance | An edit in `dress`, a layer the fill step does not read: the step is reused and no field changes. |

The other M2 acceptance check, a moved river mouth, has no river here: it is `basin`'s.

The M2 engine builds the world and every change above (macOS arm64, release build, 2026-11-14,
each change `--against` the base): the `bad_*` files exit 1 with their one record, `bad_on_road`
and `outside` exit 0, and every other change exits 0 with its measures inside `expect.toml`.
