# pistol

A real object from its published numbers, with no hand-placed coordinate, two states and three
optional accessories (challenge 47; docs/plan.md M5 "the measured object matches its published
numbers"; spec/assets.md 12). The spike's pistol written again in the v1 vocabulary: what it
showed is kept, how it was said is not (no formulas as attributes, no free solid).

| File | What |
|---|---|
| `published.toml` | The numbers: a service pistol's published overall length, barrel length, sight radius, height and width; the proportions no source gives, marked as estimates (the grip's lean among them); the racked travel; the accessories' own sizes. |
| `make_pack.py` | Writes `pack.toml`, `accessories/*.toml` and `models/*.glb` from `published.toml`, in whole millimetres. `--check` exits 1 when any file is not what it writes. |
| `pack.toml`, `models/`, `pack.facts.toml` | Written. Every part an asset; its height its `sink`; each socket a mount on the part it rides on. The leaning grip and its panels are models (a part stands upright, assets.md 12.5), the rest primitives. Facts by `via pack accept`. |
| `world.toml` | The frame placed once (`pistol`); every other part stands `on` a socket of the part it rides on. Four views round the object. |
| `changes/racked.toml` | The slide drawn back: one edit, the slide on the frame's `slide_racked` socket. |
| `accessories/` | Overlays on named sockets (assets.md 12.3): `optic` on `slide.optic`, `suppressor` on `barrel.muzzle`, `light` on `pistol.rail`. |
| `measure.py`, `expect.toml` | The build against the published numbers, per state; the state read from the build (assets.md 12.4). Protocol in both. |

## Build

```
export VIA_LIB=$PWD/lib
P=sdk/python/.venv/bin/python                                  # Python 3.12 (tomllib)
$P examples/pistol/make_pack.py --check                         # after editing published.toml: without --check,
via pack accept examples/pistol/pack.toml                       # then accept the models
via build examples/pistol/world.toml --out build/pistol --store build/pistol.store
$P examples/pistol/measure.py build/pistol --check rest
```

A state is the base and its change files, built from the world (`--against` is not needed):

| State | Change files | `--check` |
|---|---|---|
| at rest | none | `rest` |
| racked | `changes/racked.toml` | `racked` |
| an accessory | `accessories/<a>.toml` (`optic`, `suppressor`, `light`) | `rest+<a>` |
| racked with one | `changes/racked.toml`, `accessories/<a>.toml` | `racked+<a>` |
| racked with all three | `changes/racked.toml`, then `optic`, `suppressor`, `light` | `racked+optic+suppressor+light` |

```
via build examples/pistol/world.toml --change examples/pistol/changes/racked.toml \
    --change examples/pistol/accessories/optic.toml --out build/pistol+racked+optic --store build/pistol.store
$P examples/pistol/measure.py build/pistol+racked+optic --check racked+optic
engines/godot/sheet.sh --before build/pistol --after build/pistol+racked+optic --out review/pistol
```

## What the nine states measure (expect.toml; 0 errors in each)

Millimetres, the pistol's own parts: 219 long at rest, 247 racked (the slide's back passes the
tail by the travel, 40, less the tang, 12), 33 wide, 140 high, sight radius 165, every `off_*` 0.
Accessories are reported as added size, never as a fault: the optic adds 21 high, the
suppressor 130 long, the light nothing (it sits under the rail, inside the length and height).
The optic rides back with the racked slide; the suppressor stays with the barrel, which does not
move. A build checked as a state it is not fails (`rest` on an optic build: `fitted` and
`added_height`).

## Views

`bm_left` (square on), `bm_quarter` (muzzle toward the eye), `bm_rear` (down the sights; the
optic's window shows through its hood), `bm_right` (from above).

## Known faults

- A part's height is its `sink` (assets.md 12.2): the pistol must lie on flat ground; a mount with
  a height is owed.
- No hole is cut (bore, ejection port, serrations: assets.md 12.5); the barrel and the
  suppressor are square boxes (a primitive cylinder stands upright). A notch or window is the gap
  between parts: the rear sight's two posts, the hood's two posts and bar.
- The trigger, the hammer and the light's head stand upright; only the grip and its panels lean.
- Nothing holds the slide on its rails (no mutual constraint): a state moves a part, the measure
  checks the result.
- The estimates in `published.toml` are by eye; the views look like a pistol, not like a named
  one.
