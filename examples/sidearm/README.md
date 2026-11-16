# sidearm

A small mechanical object with full parts, as a modelling tool would hold it (challenge 46;
spec/assets.md 12). The spike's sidearm written again in the v1 vocabulary: the object is a set,
its parts are items, and the parts that move together are a set inside it (`upper`: the slide and
its three sight posts), a nested frame (numbers.md 5.4). Where `pistol` places parts on sockets
from published numbers, this one places them by hand in the set's frame.

| File | What |
|---|---|
| `world.toml` | The set `parts` (frame, grip, base plate, guard, trigger, barrel, and `upper`), placed once as `sidearm`. Four views round the object. |
| `pack.toml`, `models/grip.glb`, `pack.facts.toml` | Every part an asset; its height its `sink`. The grip leans 0.28 as the spike's did, so it is a model (a part stands upright, assets.md 12.5); the rest primitives. Facts by `via pack accept`. |
| `make_grip.py` | Writes `models/grip.glb` (a box sheared by the lean, packgen's writer); `--check` exits 1 when the file is not what it writes. |
| `changes/racked.toml` | The slide drawn back: one edit, `upper` 32 mm back; its four parts follow. The spike moved six parts by hand. |
| `numbers.toml` | The sidearm's own numbers: length, height, width, sight radius, the barrel's crown, the travel. No source publishes any; they are what its parts add up to. |
| `measure.py`, `expect.toml` | The build against its own numbers, per state; the state read from the build (the slide's offset over the frame). Protocol in both. |

## Build

```
export VIA_LIB=$PWD/lib
P=sdk/python/.venv/bin/python                                  # Python 3.12 (tomllib)
$P examples/sidearm/make_grip.py --check                        # after editing it: without --check,
via pack accept examples/sidearm/pack.toml                      # then accept the model
via build examples/sidearm/world.toml --out build/sidearm --store build/sidearm.store
via build examples/sidearm/world.toml --change examples/sidearm/changes/racked.toml \
    --out build/sidearm+racked --store build/sidearm.store
$P examples/sidearm/measure.py build/sidearm --check rest
$P examples/sidearm/measure.py build/sidearm+racked --check racked
engines/godot/sheet.sh --before build/sidearm --after build/sidearm+racked --out review/sidearm
```

## What the two states measure (expect.toml; 0 errors in each)

Millimetres, 11 parts. At rest: 202 long (the slide's back to the barrel's crown), 28 wide, 151
high, sight radius 180, the crown 2 mm ahead of the slide's face. Racked: 234 long and the crown
34 mm out (both grow by the travel, 32, since the slide's back is the rearmost part at rest);
height, width and sight radius unchanged. Before the M5 close the barrel stood 17 mm past the
slide at rest, as the spike's did: `crown` 17 and `length` 217, both outside their bands.

## Views

`bm_left` (square on), `bm_quarter` (the muzzle: the barrel's end shows in the slide's face at
rest, 34 mm of it racked), `bm_rear` (down the sights), `bm_right` (from above).

## Known faults

- A part's height is its `sink` (assets.md 12.2): the object must lie on flat ground.
- No hole is cut (bore, ejection port, serrations: assets.md 12.5); the barrel is a square box.
  The rear notch is the gap between two posts.
- The trigger guard is two bars: the frame closes it above and the leaning grip behind.
- The trigger stands upright (the spike leaned it 0.25); only the grip is a model.
- No hammer, no tail behind the grip: the spike had none.
- Nothing holds the slide on its rails (no mutual constraint).
