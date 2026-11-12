# Spike: a landscape from text

Throwaway proof of concept. It exists to answer the questions below and is deleted once
`FINDINGS.md` is written. Nothing outside `spike/` may import from it, and the rewrite does not start
from this code.

## Goal

**Decide what matters; the world settles around it. Change your mind, and it settles again.
You never repair a map by hand.**

Three things follow from that, and every concept here should serve one of them:

1. **Authority.** The creator states the decisions they care about: this landmark stands here,
   this road runs there, this view must stay open. Those hold. Everything else gives way to them.
2. **The rest still looks right.** What the creator did not decide is worked out to fit what they
   did: the ground under a building, the growth beside a road, the seam where two things meet.
3. **Iteration is the normal case.** A change is an edit to a decision, never a repair of its
   consequences. Nobody pushes terrain or nudges objects to make a map whole again.

**An assumption behind it.** People now make assets quickly, with generative tools for models
and images. That is not ours to do or to judge, but it changes where the work is: assets arrive
fast and keep being replaced, so the slow part is placing them well, at scale, again and again.
The tool exists to let a world keep up with that pace. "Looks right" here means placement and
seams, not the quality of a model.

This is a goal, not a claim. The spike supports the first, supports the third for the cases it
tried, and has not shown the second: its worlds are stand-in shapes, and one compiled without
complaint while making no sense.

## Questions

1. **Authoring surface.** Does a plain declarative file hold up, or are expressions needed almost at once?
2. **Seams.** Can the compiler resolve real seams from declared intent, with no magic numbers from the
   author? Road on sloped terrain, building on a hill, cliff, biome boundary, vegetation clearance.
3. **Engine handoff.** What does the engine need to receive, and how thin can the engine side be?
4. **Agent loop.** Are bookmark renders plus compiler diagnostics enough for an agent to improve a map
   over several rounds without a human in the editor?
5. **Concepts.** Do six concepts (frame, field, feature, instance, asset, variant) carry a believable
   landscape, or does every new need demand a new concept?

## Scope

In: one world of about half a kilometre, one engine (Godot 4), nature first. Ground materials,
vegetation, rocks, water, a road, a cliff, a few buildings as boxes.

Out: streaming at runtime, performance work, Unreal, navigation, gameplay, any generator beyond scatter.

Shortcuts allowed here and nowhere else: Python only, no tests, expressions evaluated with `eval`.

## Asset rules

The world source names assets by logical ID only. An **asset pack** binds each ID to a file and declares
its facts. Two packs exist:

- `packs/primitive.toml` is committed. Every asset is a box, cone or sphere, so the world always builds.
- A local pack lives in `spike/local/`, which is gitignored. It may point at assets that cannot be
  redistributed. No file, path, name or render of such assets is ever committed.

Renders go to `spike/out/`, also gitignored.

## Change scenario

The same edits a production makes, applied one at a time, each followed by a recompile and the same
bookmark renders:

1. Swap the whole asset pack (primitive to local).
2. Move the road.
3. Swap a building for one with a larger footprint.
4. Add a cliff across existing ground.
5. Apply the winter variant.
6. Make a deliberately invalid edit (a building on a slope too steep to pad, overlapping footprints).

For each: what had to be edited by hand, what followed automatically, what the compiler reported.

## Exit criteria

- All six changes run, with renders and diagnostics kept for comparison.
- `FINDINGS.md` answers each question with evidence and states which decisions are locked.

## Run

From `spike/`:

```bash
uv run python -m worldc worlds/vale/world.toml
```

```
python -m worldc WORLD.toml [--pack PACK.toml] [--overlay FILE]... [--variant NAME]... [--out NAME]
                            [--lint NAME]... [--map]
```

- The build goes to `build/NAME`; without `--out` the name is made from the world's folder, the
  overlays and the variants. The exit status is 1 when there is any error.
- `--pack` replaces the pack the world names. `--overlay` merges a change file over the world, and
  `--variant` merges one of the world's own `[variant.<name>]` tables.
- `--lint NAME` runs an on-demand check. `reach` is the only one: it tests the world's `[[expect]]`
  rows and writes `reach.png`. An unknown name is an error that lists the lints there are.
- `--map` also writes the player-facing map's layers to `build/NAME/map/`.

```bash
uv run python check.py            # compile six example builds, compare with golden.json
uv run python check.py --record   # record a new baseline
```

`check.py` takes about a minute, uses only the committed pack, and writes to `build/check_*`. Record a
new baseline only when every difference is understood.

The viewer scripts need Godot 4 and a compiled build: `shoot.sh [BUILD]` renders every bookmark to
`out/`, `explore.sh [BUILD]` walks the world, `bench.sh [BUILD]` measures frame rate at the bookmarks.
`python -m worldc.zoo PACK` lists a pack's assets by their facts, filters them (`--match`, `--tag`, `--where`), prints JSON (`--json`) and lays the matches out as a world (`--world NAME`).
`sheet.py` makes a contact sheet of renders and `maptool.py` works on an emitted map.

## Layout

```
worldc/      the compiler
sdk/         viasdk, a Python convenience over the step contract
kinds/       the one library of kinds
worlds/      example worlds
packs/       primitive.toml, the committed asset pack
viewer/      the Godot project that loads a build
check.py     the regression check, with its baseline in golden.json
build/ out/  compiled worlds and renders
local/       the local pack (gitignored)
```

### `worldc/`

| Module | Owns |
|---|---|
| `__main__.py` | the command line: arguments in, report out |
| `pipeline.py` | `compile_world`: the one order of steps, for a world and for every part |
| `document.py` | loading the TOML: includes, layers, overlays, variants, and the layer-direction check |
| `diagnostics.py` | the collector and `CODES`, the registry of every diagnostic code |
| `world.py` | grid, feature shapes, expressions, field resolution, effects and their authority, floods |
| `geometry.py` | paths, polygons, noise, smoothstep |
| `assets.py` | the asset pack and the facts it declares |
| `producers.py` | kit runs, sets, dressing, flanks and scatter, lowered to instances |
| `placement.py` | one instance spec to one placed instance |
| `checks.py` | checks that are always on: overlaps, relief under an asset, `avoid`, path joins |
| `volume.py` | volume fields: carving, meshing, and the heights written back |
| `decks.py` | ways that leave the ground: bridge and overpass meshes and their clearance check |
| `interior.py` | rooms on levels, openings, stairs, and the meshes they produce |
| `parts.py` | a world placed inside another: seam and portal boundaries |
| `outputs.py` | bookmarks, budgets, water measurements, and writing every output file |
| `playmap.py` | the player-facing map as data (`--map`) |
| `steps.py` | the points user code can join (`generate`, `lint`), the request and answer files, and the input hash that skips a step |
| `lints/` | on-demand checks; `LINTS` in `__init__.py` lists them, `reach.py` is the only one |

Every diagnostic is raised before the first file is written; only a lint adds to them afterwards.

### `kinds/`

A kind is a named bundle of effects a feature has on fields. Each kind is defined once here, and each
file's header lists the attributes a feature may set.

| File | Kinds |
|---|---|
| `terrain.toml` | guide, cliff, lake, stack, pad, road, terrace, lawn |
| `ways.toml` | street, motorway, railway, waterway, bridge, rail_bridge, lot, park: a starting vocabulary a project replaces |
| `rock.toml` | rockwork, cave |
| `building.toml` | room, tower_room, door, window, stair, lamp |
| `review.toml` | bookmark, poi |

A world names the files it needs in `include`, and so may each of its layer files, with paths relative
to the file that names them. It varies a
kind by setting an attribute on a feature, or by overriding a default for the whole world
(`[kind.road.defaults]`); it does not redefine the kind. Tables merge key by key, but a list of effects
is replaced whole, so overriding `effect` means restating all of them. Effects apply in the order the
kinds are declared, so the order inside these files is part of the result. An effect on a field the
world does not declare is never applied, and a kind no feature uses brings no ground material.

### `worlds/`

| World | What it tests |
|---|---|
| `example/` | the canonical project: one of everything a user writes, commented; read this first |
| `pistol/` | the stress test pushed: a real object from six published numbers, every part placed by formula, accessories as overlay files on named mounts |
| `sidearm/` | a stress test: a small mechanical object as 13 features in a solid, with a variant as its second state |
| `vale/` | a valley with a road, a tarn, a hamlet and a cliff; `changes/` holds the change scenario's overlays, and `winter` is a variant |
| `coast/` | a cove with sea stacks and a cave (volumes); places the college and the station as parts |
| `massif/` | land made by the user's own Python through `[[step]]` rows and `sdk/viasdk`; `gen/` holds what the steps wrote (gitignored) |
| `relief/` | land and cover read from pictures (a heightmap, a mask's channels, a stamp); run `make_inputs.py` once to make them |
| `market/` | a hill town: streets, a square, rows of a proof kit standing along the streets |
| `city/` | a 1.6 km city: a river, hand-drawn arterials and freeways, and steps that derive districts, local streets, bridges, interchange ramps and lots |
| `station/` | a lighthouse interior: rooms on levels, doors, stairs |
| `college/` | a hillside college written as five layers under `layers/` |

**Layers.** A world may list `[[layer]]` files: separately owned documents for the same world, lowest
first. Their lists are joined, every item remembers its layer, and an item may refer to its own layer
or a lower one, never a higher one. A layer file may `include` the kinds it uses; all includes are
merged first, beneath the world's own text and every layer's.

**Parts.** A `[[part]]` row places another world inside this one. The part is compiled alone, in its
own frame, then meets the master at a `seam` (it brings terrain, blended over an apron) or a `portal`
(it brings an interior, hung on one of its doors). Everything it brings is prefixed with the part's id,
and its diagnostics carry that id. Parts do not nest.
