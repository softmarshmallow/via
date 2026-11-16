# atlas

Every example a player can walk, joined in one map (docs/plan.md M6, challenge 53): the capstone,
not a composition test. Thirteen places, each its own world built apart and imported by file
(composition.md 2), on a plain the atlas owns and settles with the terrain library (lib/terrain),
joined by the atlas's own roads, a creek and its bridge.

```
world.toml          the frame, the fields, [budget], the thirteen imports and where each stands
layers/land.toml    rank 1: the river that bends (terrain_river) and the terrain step and lint
layers/ways.toml    rank 2: the roads that answer the places' exits, the city's river mouth, the creek and its bridge
layers/views.toml   rank 3: a bookmark at every join a road makes, along the roads, over the river
changes/            move_market, forgotten_exit, move_square (+ market/move_square.toml), over_budget
measure.py          the measures expect.toml bands (Python 3.11+, standard library)
```

## What is in it

city, quay, coast, college, market, glade, relief, example, massif, vale, monastery, townland, and
border's `wild` (its two trails' exits). Left out, each for a reason a build gives:

| Left out | Why |
|---|---|
| pistol, sidearm | objects on a bench, not land |
| ford | its pack `primitive` binds `tree_oak`, `house_low` and eight more ids otherwise than `starter`: `import.binding_differs` (composition.md 10) |
| station | its pack binds `lamp` otherwise than college's and `lamp_post` otherwise than `starter`, and paints `dry_grass` and `path`, which `starter` does not bind: `import.binding_differs`, `pack.unknown_material` |
| county, border | each is a composition of places here: county is quay on a parent's land (the atlas is that parent now); border is monastery and the wild |
| strait | its islands' slipways are strait's to answer; its bay floor lies at -14 m |
| basin | 3.2 km a side, larger than the map; the plain is settled by the same library |

## Build

```
export VIA_LIB=$PWD/lib
via config program python $PWD/sdk/python/.venv/bin/python
via config program via-terrain target/release/via-terrain
via trust examples/atlas/world.toml
via build examples/atlas/world.toml --out build/atlas
via build examples/atlas/world.toml --change examples/atlas/changes/move_market.toml --against build/atlas --out build/atlas+move_market
python3.12 examples/atlas/measure.py build/atlas --check base
python3.12 examples/atlas/measure.py build/atlas+move_market --run build/atlas+move_market.run.json --check move_market
engines/godot/sheet.sh --before build/atlas --out review/atlas --stream 400 --option water=tag:water_level
```

## Lifts

Only quay declares a datum (`sea`, the atlas's); it is a fill and stands on the atlas's land
(`give = { land = "height" }`). Every other place has its own zero and takes a `lift`
(composition.md 5). Each lift was measured, not chosen by eye: build with `lift = 0`, take
`join_step` (measure.py: per import, the median over its outline every 8 m of the finished height
4 m outside less 28 m inside), and add it. After one round the largest median step is 7.5 m (coast,
whose outline is half sea; its lift is set instead so its sea at its 0 lies on the atlas's at 2).

## Budget

`[budget] triangles = 100000` source triangles per 128 m chunk. No target is known (build.md 5.1
has no default); the number is ours: the base's heaviest chunk holds 77,904 ([22, 12], woods), so
the limit leaves about a quarter for growth. `changes/over_budget.toml` writes `chunk = 160` (a
change may not write `[budget]`, it may write `chunk`): one chunk, [17, 8], holds 104,008, and the
build exits 1 with `tree_oak` named, 63%, 260 instances, and the waiver to write.

## Changes

- `move_market`: the market 40 m east and 30 m north. The outputs follow: of the atlas's features
  only `market_road`, `college_road` and the view `join_market` move, and the ground changes inside
  2530..3054 m east, 410..704 m north. The work does not: every routed road runs again (below).
- `forgotten_exit`: the road to `wild.south_end` removed: `import.exit_unanswered`, `data.write`
  `ends_at = "wild.south_end"`, exit 1.
- `move_square`: the market's own square 10 m east, 6 m south, asked of the market by
  `[import.market] changes = ["../atlas/changes/market/move_square.toml"]`. The change file is in
  the market's terms but held here, since the market has none (decided, open to the owner). Of the
  atlas's features none moves; inside the market only its own; 614 posts of ground change.
- `over_budget`: above.

## Build times

One fresh store, one run each, in order (protocol in expect.toml; other agents building):

| Build | Wall s | Nodes ran |
|---|---|---|
| cold (13 places and the plain) | 166.1 | all |
| the same, into a new directory | 33.7 | 0 |
| move_market | 103.0 | 2801 (2243 of 11,500 field chunks) |
| forgotten_exit (exit 1) | 90.6 | 1077 |
| move_square | 100.5 | 2271 |
| over_budget (exit 1) | 106.4 | 8154 (a new chunk side re-plans every field) |

A change costs about 60% of a cold build. Taken again in the M6 slice 2 fix pass (same protocol,
load 9 to 15): cold 245.3 s, move_market 161.8, forgotten_exit 105.1, move_square 148.3,
over_budget 165.6: a change 43 to 68% of cold. The two runs differ by up to 57% under other
agents' load, so a time here says where to look, not a speed.

## What does not hold

- **Water at the joins.** 39,434 posts (about 16 ha) inside the 24 m blend rings stand wet: where a
  place's lifted ground lies below the plain, the plain's water is mixed in by `1 - w` while the
  ground fades to the place's (composition.md 6.2's derived rule offsets the place's water, not the
  parent's). Visible at `join_city_north`.
- **The college's lake.** The college declares no water field; the plain's water stands under it,
  and the terrain step's standing water (a row above every row it read) floods its low corner up
  to 7.2 m. A row of the atlas cannot reach under a step's row.
- **Routes re-run on any ground change.** `route` reads the whole surface, so moving one place
  re-runs all eight routed roads (`shapes_ran`), and 2243 of 11,500 field chunk nodes: 103 s
  against 166 s cold.
- **An unchanged build into a new directory** ran 0 nodes and took 33.7 s: preparing the thirteen
  imports for the build key (TODO "an unchanged rebuild ... preparing the imports").
- **Roads inside a place.** The plain's roads are earth outside every place (`road_painted` 1.0,
  from layers/steps/track.py since M6 slice 2; before, a row of world.toml could not see the ways
  layer's roads and none was painted). Inside a place's outline its own ground is brought over the
  plain's rows, so the last stretch to its gate is the place's paint (`road_painted_in` 0.10).
  The step exists only because a layer may not write a row on a field world.toml declares (TODO).
- **Seams, and roads over them.** One `import.seam` per place (13), worst grade 4.1. The steepest
  4 m step of each road lies where it crosses a place's edge: `hill_trail` 15.2 at relief's
  north-east corner (2543, 1229), `west_road` 5.95 and `wild_road` 4.31 at the wild's edges,
  `college_road` 3.17 at the college's arrival. The roads are `shaped`: they neither hold the
  ground across the edge nor are met by it (composition.md 6.6, 6.7).
- **Trees on the road.** The west trail runs through the wild's wood; the wild's trees are its
  own, placed before any parent way, and one stands on the trail (`join_wild_west`).
- **The sheet.** 124 bookmarks (13 here, the rest the places') compose to an image 47,664 px tall:
  the GL target fails and the sheet tool writes no `sheet.png` (shots are in `before/`).
- **The river that bends** runs from its spring to the city's north edge, then on under the city:
  the model's water reaches the sea at 1472 m east, through the city's ground (its course `ends`
  `sea`); the city's own river is what a walker sees.
