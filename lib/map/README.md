# lib/map

Things a game reads from the world that change no field (`kinds.toml`: `poi`, `spawn_zone`), and
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

### Measures (protocol)

`tools/map_render.py BUILD MAP --out P --expect examples/<w>/expect.toml` draws the map beside
the build's plan view and measures **way registration**: every 0.25 m along each way's and water
course's line in `features.json` (not under), the share whose map cell is a way, water course or
footprint class. Taken on one build each of `coast` and `station`, cell 1 m and 0.5 m: 1.0 on
every way. Bands in each world's `expect.toml` `[map]`.

Until the engine runs `when = "export"` (doc.not_yet in M6 slice 1), `tools/map_export.py BUILD
--step examples/<w>/map.toml --out M` hands a finished build to the step as a plain command, as
the engine will.

## Tests

    PYTHONPATH=sdk/python python3 -B -m unittest discover -s lib/map -p 'test_*.py'

Library code imports only viasdk and the standard library (R19b).
