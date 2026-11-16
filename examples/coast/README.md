# coast

A headland ending at the sea in a sheer cliff, a sea cave under it, and a harbour beach where a beck
runs to the sea under a bridge (docs/plan.md M5; challenges 7 and 8). The spike's coast written again
in the v1 vocabulary: everything that is not the terrain is a kind surface of a named feature
(spec/surfaces.md 1, R25), joined to the terrain only at meetings (3.3).

- **The cliff says it is sheer.** Two `sheer_cliff`s (lib/land). The face is the kind's surface, a
  1 m strip rising 20 m, whose mesh has vertices across the strip only on its outline, so its brink is
  drawn along the cliff's line (1 mm off) where the terrain alone would stair-step it at the 0.5 m
  posts (491 mm). The terrain's own step under it is cut (surfaces.md 1.3).
- **The cave** (lib/land `cave`, surfaces.md 6.5) is a floor under the headland, 19 m of rock over
  it, its mouth cut open and met by the beach at gap 0. The cave owns its mouth (R18): the two faces
  end at its jambs by name, so moving the cave moves the opening in the face.
- **The bridge** (lib/ways `crossing`) owns a deck that is its kind surface, so the beck runs on
  beneath it; its parapet is lib/ways' `parapet` step, its water checked by `crossings`.

| File | What |
|---|---|
| `world.toml` | The land read from the faces' line, the cave, the two faces, the beck, the bridge and its lanes, the parapet step and the crossing lint. |
| `layers/views.toml` | Bookmarks at the mouth, inside the cave, along the face, from the sand, on and beside the bridge. |
| `changes/move_cave.toml` | The cave 16 m west along the cliff: only the cave is written (and the view inside). |
| `changes/long_deck.toml` | The deck 16 m to 24 m: its two ends 12 m off the beck's line. |
| `cave_points.json` | Points for `via query surfaces_at` (the cave's middle first). |
| `expect.toml`, `measure.py` | Measures, bands and protocol. |

## Build

```
export VIA_LIB=$PWD/lib
via config program python $PWD/sdk/python/.venv/bin/python     # steps run on Python 3.12 (3.11+)
via trust examples/coast/world.toml
via build examples/coast/world.toml --out build/coast
(cd examples/coast && via query world.toml surfaces_at --in cave_points.json --out ../../build/coast.sat.json)
via build examples/coast/world.toml --change examples/coast/changes/move_cave.toml --against build/coast --out build/coast+move_cave
python3 examples/coast/measure.py build/coast --surfaces build/coast.sat.json --check base
engines/godot/sheet.sh --before build/coast --after build/coast+move_cave --out review/coast --option water=tag:water_level
```

## Known faults (M5 slice 2 tree)

- The mouth is a slot the full height of the face (the face's quads at the mouth are cut, as 6.5
  says), and inside, the hill has no rock: nothing is drawn round the passage, so the cave's walls
  are the sky under the terrain. The rock round the passage is assets a library places (1.3, 5):
  none yet (challenge 8).
- A cut quad reaches up to 0.35 m past the face's strip: 0.13 m² a metre of face is open at its
  foot and brink (`sliver_m2_per_m`).
- A bookmark placed `on` the cave stands on the terrain, 20 m up, not on the floor (4.1 says the
  floor); `in_cave` is placed by `at` and `surface`, so the change file moves it by hand.
- (Fixed in M5 slice 2: the parapet's posts stand on the deck, the first one 1 mm past the strip's
  end now on it, surfaces.md 1.2; a warm `long_deck` lays the 26 posts a clean build does.)
- No `keep_out` field: the crossing's `clear` would refuse its own parapet's posts.
