# atlas

Every example a player can walk, joined in one map (docs/plan.md M6, challenge 53): the capstone,
not a composition test. Thirteen places, each its own world built apart and imported by file
(composition.md 2), on a plain the atlas owns and settles with the terrain library (lib/terrain),
joined by the atlas's own roads, a creek and its bridge.

```
world.toml          the frame, the fields, [budget], the thirteen imports and where each stands
layers/land.toml    rank 1: the river that bends (terrain_river) and the terrain step and lint
layers/ways.toml    rank 2: the roads that answer the places' exits, their approaches, the creek and its bridge
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

**At the exit** (M7 slice 1). A place a road answers is lifted so the place the road ends on meets
the plain, not by its outline's median: the plain's finished height outside the place less the
place's own at it, from a build, added to the lift. The vale is a valley whose road enters at its
mouth: by its median its valley floor lay 37 m under the plain at both ends, so the hill trail
fell into a trench (`hill_trail@vale` 8.026, a pool in it, `join_vale` blocked at 41.9 degrees).
Measured on the build before: vale, the plain 13.9 m at (2607, 1290), its road's start -23.1 m:
-52.5 -> -15.5. Market, the plain 6.5 m 4 m out, its main street 1.6 m 12 m in (a 4.7 m drop where
the plain's road bed met it, `market.arrive` blocked 2 m in): -16.5 -> -11.6. College, the plain
6.6 m south of its arrival, the arrival 3.2 m: -3.5 -> -0.1 (the 3.2 was the old pad's cut, not the
college's own: its arrival stands at 4.1 now, 2.5 m under the plain, still; "What does not hold").
The cost is the rest of each outline: the vale's walls now stand 37 m over the plain on its median
(`join_step` -37.24), the market's 5.15 and the college's 3.55 (each by the lift's change).
At the M7 close the college's lift is +2.4 (-0.1 before): its arrival stood 2.5 m under the plain,
so its approach cut the plain below the terrain step's water and `join_college` was blocked by
0.70 m of it. Measured on the build after: `college_road`'s last 30 m hold 0.14 m of water (1.86
before) and climb at 0.197 (0.273), the college's own approach 0.091 (0.175); its outline's median
step is -5.68 (-3.55), the price of the same trade.

**The market's north end** (M7 close). The market's main street runs out at its north edge, which
no road answers, and the market is lifted at the street's start in the south-west, so its north
edge stands 12.5 m over the plain: the 16 m blend left a scarp across the street's end
(`market.top` blocked 0.7 m in at 45 degrees). An `approach` 24 m inside the edge, where the ground
is the market's own (18.76 m), holds the plain to it at 1:5 out to 60 m (layers/ways.toml,
`market_top_approach`): a fan down to the plain, 14.4 m at the edge where the plain was 6.4. The
lip inside the edge, where the market's blended ground meets the fan, is 0.48 (2.9 m in 6 m).

The wild's kept corner is the plain's, but the wild was built to meet a neighbour on the slope it
agreed there (its `agreed`: 18 m at its y = 240, rising 0.17 m a metre north), so the plain takes
that slope near the wild's two edges of the corner (world.toml `[field.start]`), and its west exit
at 51.3 m meets it; before, the plain lay 12 m under it (`join_wild_west` blocked at 73.9 degrees).

## Joining an example

What a parent needs to join each example, as atlas does (M7 close: a fresh agent found only quay's
exit by grepping). Only quay and border's wild declare an exit (`kind = "entry"`, lib/town); the
others are joined by ending a road on a named place of theirs, which the join then holds (6.7).
End the road on the place, not beside it.

| Example | End a parent's road on | Datum | Atlas's `lift` |
|---|---|---|---|
| quay | `quay.gate` (an exit) | `sea` | none, a fill (`give = { land = "height" }`) |
| city | `city.harbor_gate_e`, `city.north_gate_e`, `city.north_gate_w` | none | -2 |
| college | `college.arrival` | none | 2.4 |
| market | `market.main_street.start` | none | -11.6 |
| relief | `relief.track.start` | none | -25 |
| vale | `vale.valley_road.start` | none | -15.5 |
| townland | `townland.west_gate` | none | 16 |
| wild (border) | `wild.south_end`, `wild.west_end` (exits) | none | none: its kept corner meets the plain |
| coast, glade, example, massif, monastery | no road: placed beside the plain | none | 2, -14, -27, -31, -19 |

A lift is the step between the import's zero and the parent's ground where the road arrives, so it
holds only where the import stands: moving or turning an import with no shared datum needs its lift
measured again (`composition.md` 5). Measure it on the road, not only from `import.no_datum`'s or
`import.seam`'s `lift_hint`, which is the median over the whole edge: a fresh agent's market,
lifted by the hint's 0.74, still dropped 0.92 m in the road's last 4 m and needed 1.66.

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
- `forgotten_exit`: the trail from `wild.west_end` removed: `import.exit_unanswered`, `data.write`
  `ends_at = "wild.west_end"`, exit 1 (the road to `wild.south_end` until the atlas pass; the west
  trail now ends on that road, so removing the road would leave the trail's end on nothing). Not
  re-run in the atlas pass.
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
agents' load, so a time here says where to look, not a speed. At the M6 close (on-foot review,
release, `--threads 14`, one fresh store, load 7 to 84): cold 198.7 s, unchanged into the same
directory 1.0 s (the 33.7 above was into a new one, before imports entered the key by their text),
move_market 115.9, move_square 168.3, forgotten_exit 108.9, over_budget 175.5.

## What does not hold

- **Water at the joins** (composition.md 6.2, the parent's water re-laid; `joins`, M6 close). The
  plain's water is now mixed in by its depth over the plain's own ground, not its level over the
  place's lower ground: posts of the blend rings standing more than 0.05 m wet fell from 38,777 to
  3,156 of 112,975 (measure.py `ring_wet`; the rest are the city's river and harbour and coast's sea,
  which cross their rings), bookmarks in water from 15 to 6 (`monastery.bm_arrive` dry). The six
  left were the city's three `on_*_bridge` and townland's `on_bridge`, which stand on decks since an
  import's kind surfaces are brought (composition.md 6.1) and are dry since measure.py reads a
  bookmark's own `z` (M6 close), and `quay.wall_face` 3.6 m and `monastery.bm_gorge` 0.34 m, which
  are left.
- **The college's lake** is gone: an import that declares no water field is dry under ours
  (composition.md 6.2, "An import without the derived field"); `college.arrival`'s approach held
  6.53 m of water in its first 30 m and holds none. The fix is the import rule's, not a row here: a
  row of the atlas cannot reach under the terrain step's row.
- **Exits on a cliff** (closed for the vale and the wild's west exit in M7 slice 1, "Lifts": the
  text below is the M6 close's). Each place a road joins now holds the place's ground (composition.md 6.7,
  "Held"), so no place's own way is under water at its join (`exit_in_depth_max` 8.76 -> 0) and
  `import.place_blended` is gone (5 -> 0). But the vale's valley road starts 37 m below the plain:
  held, that step moved to the plain's side of the edge, where the hill trail falls 12.06 in its
  last 30 m (`exit_out_grade_max`, 0.004 before) and cuts a trench below the plain's water level
  (11.95 m deep at most), and the vale's road still climbs at 1.79 (`exit_in_grade_max`, 2.27
  before) where the hold fades. The atlas must meet the vale at its mouth: a lower plain there, a
  lift taken at the exit rather than the outline's median, or a graded ramp the plain owns.
- **Change builds.** The time is in placement and whole builds, not routes (`perf`, M6 close):
  move_market 115.9 s, 58% of a cold 198.7 s (on-foot review, release, load 18). An unchanged
  build into the same directory takes 1.0 s since an import enters the build key by its text
  (graph.md 6.2; 33.7 s before); into a new directory with the map export trusted, 89.8 s, of which
  about 54 s is in no node (TODO From M6 close).
- **Approaches** (M7 slice 1). The college's arrival and the wild's west exit are met by lib/land's
  `approach` (anchored, so read after an import's anchored ground; lib/land/test_approach_world.py)
  in layers/ways.toml. Measured on the build, the step the parent's road bed leaves is not an
  anchored-ground matter at the college (it has no anchored entry): a road's bed is
  `smoothed_along(40)` of the ground along its own line, so at its end it carries the plain's level
  a few metres into the place (`market.arrive`'s drop). The lift at the exit makes the two levels
  one; a road that ends on a place taking that place's level at its end is proposed (TODO). The
  college's arrival still stands 2.5 m under the plain (its lift should be about +2.4, not built), so
  the approach cuts the plain below the terrain step's level and `college_road` holds 1.86 m of
  water in its last 8 m (`exit_profile`, `join_college`).
- **Roads inside a place.** The plain's roads are earth outside every place (`road_painted` 1.0,
  from layers/steps/track.py since M6 slice 2; before, a row of world.toml could not see the ways
  layer's roads and none was painted). Inside a place's outline its own ground is brought over the
  plain's rows, so the last stretch to its gate is the place's paint (`road_painted_in` 0.10).
  The step exists only because a layer may not write a row on a field world.toml declares (TODO).
- **Seams, and roads over them.** One `import.seam` per place (13), worst grade 4.1. Measured
  every 4 m of a way's length (measure.py `along`; the 15.2, 5.95, 4.31 and 3.17 of M6 slice 2 were
  a post's rise over a fraction of a metre of path), the steepest step within 30 m of where a road
  meets a place (`road_join`) is 0.633: `west_road` at the wild's west exit (the wild's ground at
  the exit 47.1 m, the plain in its kept corner 39.3), `college_road` 0.558 at the college's
  arrival, `wild_road` 0.298 at the city's gate; the other ten 0.004 to 0.139. Target 0.25 (ours).
  With the joins' hold (M6 close, combined tree): `hill_trail@vale` 8.026 into the vale's trench,
  `west_road@wild` 2.102, `college_road@college` 0.600 (0.911 without its pad, whose `max_cut` is
  6.5 since the hold made it cut 6.09 m), the rest 0.010 to 0.298.
  The hill trail no longer crosses relief's ridges (2.03 there): its route costs 40 a metre inside
  relief and the vale, so it leaves relief's west gate outward and goes round. The pads at the
  college's arrival and relief's gate (layers/ways.toml, "the approaches") ramp the plain at 1:10
  to the level they read, but they are `shaped`: they read the ground before the college's
  anchored arrival, so they level toward the plain's 6.6 m, not its 3.9 (0.637 -> 0.558). A ramp
  that meets an anchored place must come after that place's anchored ground: open. Without the
  joins' hold (composition.md 6.7); with it the vale's exit is 37 m below the plain (above).
- **Trees on the road.** Gone with the route: the wild's footprint leaves out the corner it keeps
  for a neighbour (`footprint` in world.toml), so the corner is the plain's, and both roads reach
  the wild's exits from it (`west_road` from `west_end` to a point on `wild_road`, which comes
  round the wild's east side), never through its wood: no tree, wood or brush instance within its
  half width and a metre of any atlas way (a scratch check over `instances/`). The map's way in
  from the west edge (`west_edge`) is gone with it. A parent's way still does not clear an import's
  scatter (TODO).
- **The sheet.** 123 bookmarks (13 here, the rest the places') compose in 7 pages of 20 since the
  M6 close (build.md 15.1); 84 walks arrive, 33 are blocked and 6 fall (on-foot review, streamed at
  96 m with water; equal to whole). The worst: the vale's trench, the wild's west scarp,
  coast.along_cliff through the brought cut (TODO From M6 close). At the M7 close, 126 bookmarks:
  97 arrive, 28 are blocked, 1 falls (91, 33, 2 at M7 slice 1; streamed at 96 m with water, the
  release binary of 58febc6): the city's two bridge views, `market.top`, `join_college`,
  `coast.under_deck` and `monastery.bm_tarn` arrive; `coast.along_cliff` still falls (core).
  After the M7 close's core fix (R53), on the M7 sheet (release binary of 4016c85, one fresh
  store, streamed at 96 m with water): 97 arrive, 29 are blocked, none falls; `coast.along_cliff`
  arrives (59.6 m) and `coast.in_cave` is blocked 1.95 m short (TODO).
- **The river that bends** runs from its spring to the city's north edge and on along the city's
  river's line to the sea (layers/land.toml): its course carves the mouth south of the city (the
  beck `city_mouth` did, on the same line, until the atlas pass) and ends in the sea at (1008, 224),
  where the eroded groove falls below the sea's level; under the city the course is `derived` and
  the city's river is what a walker sees. The map draws the course (`terrain_course`), not the
  stated line, which lay up to 23 m from the wet channel. The terrain lint says
  `river_wanders` (72 m at (948, 96)): retraced over the finished ground, the water strays on the
  beach before the sea. At the city's south edge the water narrows from the city's 30 m river to
  the course's 10 m channel (a thread 0.5 m deep, 6 to 10 m wide, from y 320 to 250), and on the
  beach below y 216 it fans out 60 to 100 m wide: the terrain step's eroded groove lies under the
  sea's 2 m there, and the sea row fills it (`terrain#standing` 0.76 m at (1008, 150), level 2.0).
  The course's `width = 30` (the city's) was tried at the M7 close: the facts lint then found the
  water lost in a hollow at the city's edge, (994, 322), an error (`step.facts.river_lost`), so
  it stays 10 (TODO).
- **The creek's bridge** (M7 close). `creek_road` and `college_road` ended at `bridge_west` and
  `bridge_east`, the points the deck is written at, so no meeting joined either road to the deck;
  they now name its ends (`creek_bridge.start`, `.end`), and lib/ways's crossing lint runs here
  (`[step.crossings]`, layers/ways.toml) over the atlas's own crossing and every way end beside
  another path's end (`end_unjoined`): 1 crossing, 2 ends, no record.
