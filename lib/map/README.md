# lib/map

Things a game reads from the world that change no field (`kinds.toml`: `poi`, `spawn_zone`,
`area`: a named part of the map, such as a place an import brings), and
the player map as data (`export.py`, build.md 9.4).

## The player map

An export step: it reads the finished build through the SDK and writes the map's layers under
`export/<step id>/`. A reader finds it by the step's `output`, `player_map` (protocol.md 1), in the
manifest's `exports`, and reads `map.json` there.

```toml
[step.map]
when    = "export"
output  = "player_map"
command = ["python", "-m", "viasdk", "lib:map/export.py"]

[step.map.params]                 # what each of the world's kinds is on the map
ways   = { lane = "lane", crossing = "bridge" }
water  = { beck = "stream" }
floors = { storey = "level" }     # region kinds that are floors, and their level attribute
rooms  = { room = "storey" }
doors  = { doorway = "sill_z" }
links  = { flight = ["z0", "z1"] }
walls  = ["structure"]            # taxon paths of wall pieces for the floor plans
buildings = ["building"]          # taxon paths of assets drawn as footprints by their bounds
regions = { area = "label" }      # named areas, and the attribute their label reads
```

`export.py`'s docstring lists every param and its default. lib/map knows no kind but its own
`poi`: what a lane or a storey is on the map is the world's to say.

### The map's files

| File | Holds |
|---|---|
| `map.json` | `format` `via-map-0`, `output`, `transform` (cell side, size, `affine` from world metres to cells, rows south first), `classes`, `layers`, `floors` (index, levels, height range, file, fingerprint), `tiles` (side, count, one fingerprint per plan and tile), `discovery`, `files`. |
| `ground.npy` | One class id per cell, `\|u1`, row 0 south (build.md 3.2). |
| `vectors.json` | `ways`, `water`, `footprints`, `regions` in world metres. |
| `labels.json` | POIs, labels, region and room names: `key` (the feature's name, for the game to localise), `text`, `class`, `at`, `floor`. |
| `floors/<n>.json` | Per floor: `surfaces`, `rooms`, `walls` (outlines cut at `cut` m), `doors`, `links` with `from` and `to` floors. |

What stays the game's: discovery state (keyed on `discovery`'s grid, `reveal` m a side, and on
room names), style, icon art, localised text.

### Labels

A text read from a name is its last part, `_` as a space; a produced name (`#`) gives no label,
and a produced room reads `Room <floor>.<n>`. Labels are placed apart at the stated zoom
(`label_zoom`, `floor_label_zoom`) with a glyph box (`glyph`): the first of eight positions round
the anchor where the box meets no other box and covers no other anchor, and on the ground plan
covers no footprint where any position can (M7: atlas 8 of 44 labels over a footprint before, 3
after, those 3 with no clear position at 1 px a metre); `box` null where none fits. Rules in
`export.py`'s docstring and build.md 9.4.

### Measures (protocol)

`tools/map_render.py BUILD MAP --out P [--expect examples/<w>/expect.toml [--section T]]
[--against OLDBUILD OLDMAP]` draws the map beside the build's plan view, at the map's label zoom
unless `--ppm` is given, and measures:

- **Registration**, ways and water courses apart (`way_on_cells_min`, `water_on_cells_min`): every
  0.25 m along each line in `features.json`, the share whose map cell is a way, water course or
  footprint class. A way marked `under` is listed in `skipped` with its reason (`under`).
- **Labels**: `label_overlaps`, pairs of drawn texts (PIL's default font) that meet;
  `labels_unplaced`. Per floor, the floor plans' link notes are placed by the same rule after the
  labels, and counted too.
- **Tiles** (`--against`): `tiles_changed`, each with what changed in it; `tiles_unexplained`
  (none named) and `tiles_cells_only`.

Taken in M6 slice 2 (2026-11-17), macOS arm64, release `via` of the working tree, VIA_LIB=lib,
one fresh store, one run each:

| World | Rows |
|---|---|
| coast (cell 1 m) | 6 in band; ways and water 1.0; the cave skipped as `under` |
| station (cell 0.5 m) | 6 in band; floors 0 to 3: overlaps 0; `move_stair`: 4 of 80 tiles changed, every one named (the stair, the watch room) |
| atlas (cell 2 m) | `[map]` and `[map_move_market]` in its expect.toml |

The engine runs the step (`when = "export"`, M6 slice 2) after the lints and lists what it wrote
under `export/<step id>/` in the manifest; `coast`, `station` and `atlas` declare it in their
`world.toml`.

## Tests

    PYTHONPATH=sdk/python python3 -B -m unittest discover -s lib/map -p 'test_*.py'

Library code imports only viasdk and the standard library (R19b).
