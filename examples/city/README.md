# city

A city on a river (docs/plan.md, M3; R13, R15 to R18): the spike's `city` (1.6 km, a river,
freeways and arterials drawn by hand, districts each with its own street pattern cut into wards by
the arterials, bridges where an arterial meets the river, ramps where one meets a freeway, lots
filling blocks, a proof kit of boxes) rebuilt in the v1 vocabulary on the new core and lib/. 1.6 km
at 2 m posts.

| File | What |
|---|---|
| `world.toml` | The land (a valley written by the river's line, a hill), the river (lib/water `beck`), the limits, gates, three crossings and three interchanges, nine arterial pieces, three avenues and four freeway pieces routed by what they connect, the wards network, three districts and their streets steps, one block network per district, three lot tables. |
| `layers/town.toml` | Houses (one lib/town houses step per district), the lints (`review`, `review_houses`, `form`, `reach`), the paint, 19 views at the meetings. |
| `changes/` | `move_avenue` (the grand avenue 40 m east), `move_bridge` (the central crossing 48 m upstream), `hand` (a clear and a pin). |
| `expect.toml`, `measure.py` | Counts, the meetings and who owns them, the street form, reach, names elsewhere unchanged after each change; with protocol. |

New in lib for this world: lib/ways `freeway` and `interchange`, and since M5 slice 3 `viaduct`
and `ramp` (levels, below),
lib/town `ward` (a named region, no effect).

## Build

```
export VIA_LIB=$PWD/lib
via config program python $PWD/sdk/python/.venv/bin/python
via trust examples/city/world.toml --yes                       # and once per --change
via build examples/city/world.toml --out build/city
via build examples/city/world.toml --change examples/city/changes/move_avenue.toml \
    --against build/city --out build/city+move_avenue
python3.11 examples/city/measure.py build/city+move_avenue --base build/city --check move_avenue
engines/godot/sheet.sh --before build/city --out review/city --option water=tag:water_level
```

**Post spacing: 2 m** (640,000 posts). At 1 m (2.56 M posts) the fields alone would take about four
times what townland's 262,144 posts take for each field chunk, on a city whose streets are 4.5 m
and whose arterials are 13 m wide: 2 m still puts two posts across the narrowest carriageway. Cold
build: see expect.toml's protocol (about 2.5 min, 4.6 GB peak). Measured on the way:

| What | Cold build | Peak memory |
|---|---|---|
| no houses at all | 59 s | 0.98 GB |
| houses on every lot, standing on the ground (what is committed) | 154 s | 4.6 GB |
| houses on terraces (townland's) in the old town only, 777 lots | killed at 1.5 min, 7.5 GB and rising | |
| houses on terraces everywhere, at 4 m posts | killed at 11 min, 9 GB | |
| the same at 2 m | killed twice, at 8 and 9 min | |

The terrace kind reads every other terrace (`inside('kind:terrace')`), and the core's graph gives
each field chunk an edge from every entry that reads a selector to every member of the selector,
whether or not its window meets the chunk (via-model `graph.rs`, `define_all`, the "entry ... reads"
edges; and `shape_members` filters and sorts every feature and made thing on each call). With
hundreds of terraces that is quadratic in them per chunk. graph.md 8.1 says a chunk reads the made
things whose windows meet it: the code reads all of them. Owed in the core (TODO.md). So every
house here stands on the ground as it lies (`max_relief`), and the old town's hill has few houses.

## What it shows (2026-11-15)

- R16: every arterial is written by what it connects: a gate, an interchange, a crossing's end, by
  name, routed over the land. The avenues run gate to gate and cross the arterials where the wards
  network nodes them (a network owns the crossings of its inputs, geometry.md 8.1).
- R18: three crossings, one deck each (form `decks` 3 on the river, 0 doubled, 0 ways over the
  water); all six crossing ends have an arterial arriving; 0 `geom.unowned_crossing`. Each
  freeway-arterial meeting is a named interchange with four ways ending on it.
- Two street patterns: `grid` downtown (along the river's bearing, 50 by 100 m) and in the rows
  (turned 30 degrees, 46 by 92 m), `contour` round the old town's hill.
- Wards: the skeleton's faces within the limits (24). Lots fill every block's street sides (2,744);
  houses stand on 1,277 of them.
- R15 (lib/review form): bearing spread 0.74 (challenges.md row 59: the spike's city alone 0.45,
  in atlas 0.76); for the main ways 0.39 (spike 0.40). 0 dead ends; 94 edge ends, apart.
- Reach: a walker from the central arterial's west gate reaches the three east gates over the
  three bridges, the hill and the grand avenue's two gates (6 of 6).
- R13: `move_avenue` and `move_bridge` resettle only the wards their way bounds; `hand` clears one
  lot and pins a shed in another. Names elsewhere unchanged: expect.toml, measured.

## Given in M4

The spike's city took `[given.limits]`, `[given.urban]` and `[given.districts]` from a larger map.
Here they are written standalone: `limits` a square 60 m inside the extent (the gates stand on it),
three districts as regions. In M4 a parent gives the limits and the districts, answers each gate as
an exit (TODO.md, "From the city's exits"), and `urban` (how built up a lot is) gets a meaning; today
every lot a house fits gets one.

## Stand-ins (say what is not real)

- **Interchanges have levels since M5 slice 3** (challenge 38): the freeway passes over each
  arterial on a lib/ways `viaduct`, a deck 300 m long, 6.0 m over the interchange's plate, where
  the arterial is taken 3 m down; two loop `ramp`s per interchange (270 degrees, west of the
  freeway, where no district is) run from the deck to the ground beside the arterial (`ix_*_view`,
  `ix_central_freeway`). What is not there: a ramp from the freeway on the ground (a diamond's
  slips), a merge onto the arterial (a loop lands 7 m beside it), piers, deck edges and parapets
  (the deck is a surface); the viaduct climbs at up to 6.1 % (interchanges 400 m apart leave 300 m).
- **The freeway lies on the ground between viaducts.** The spike's crosstown freeway, over the
  river and three avenues on a viaduct, is still left out.
- **The bridges are decks over the river** since M5 slice 2 (lib/ways `crossing`'s kind surface; 284
  wet posts under the three), but lib/review's reach walks the terrain only, so the river cuts the
  city in two for it (`reach` 2 of 6, OWED in expect.toml).
- **Boxes**: the starter pack's two row houses everywhere; the spike's towers, mid-rises and sheds
  per district need buildings as a library (M5).

## Known faults (read from the sheet, on foot, 2026-11-15; 19 views, before and after `move_bridge` and `move_avenue`)

- **R13 for lots held (M4 slice 1).** `move_avenue` renamed 212 lots inside blocks whose names and
  outlines were unchanged, `move_bridge` 16: every crossing of a district outline was named by the
  outline alone and numbered round it, so the avenue crossing one outline renumbered the rest and
  every lot anchored on them. A cut source now has its role at the outline (identity.md 2.3,
  roles by travel): 0 and 0.
- **The grand avenue held to its line (M4 slice 1)**: routed over 1.5 m lumps it ran from x 661 to
  728 for a line written gate to gate at 700, into both districts. It now passes four named places
  on a guide between its gates (derived.md 3, way points by name) and lies on its line, base and
  `move_avenue`.
- **The hill is nearly empty**: 213 houses on 137 of the old town's 777 lots (315 left out as
  steep), because houses stand on the ground as it lies (terraces did not build, "Build" above).
  `old_town_street` shows rows standing on the slope with grass between them and backs to the
  street. 156 `place.relief` warnings, all houses on the slope
  (190 on the combined slice 5 tree, where rule 4's exact keep_out lets 2,201 houses stand).
- **Houses stand a strip of grass back from the paving**: `downtown_at_central` looks into a house
  wall with grass between the footway's paint and the door. The flags paint stops at the footway
  (3.75 m), the house front at the keep_out's end.
- **Interchanges read as a wide grey plain** (`ix_central_view`): a level plate where four 13 to 20 m
  ways meet, nothing built near (no lots outside the districts). The stand-in, above.
- **The decks are causeways** (`central_head`, `on_central_bridge`): flat, the water at both
  faces; the river reads well at 30 m. The views along the arterials show no town: lots exist only
  inside the three districts.
- **1,467 of 2,744 lots hold no house** (410 to 442 per district "small"): the pack's two row
  houses do not fit their frontage after `gap`, or the ground rises more than `max_relief`.

## What the vocabulary could not say

- **A place an arterial passes, by name.** R16 allows "the places it passes, by name"; a path's
  way points are coordinates only (core: path way points by name, not started). The avenues are
  gate to gate.
- **A street pattern per ward.** The spike cut each district into wards and laid streets per ward.
  A ward is a network area (a made thing), and a step reads one only from a layer above the network
  (geometry.md 8.3): a streets step per ward means a layer per stratum, and names it cannot know.
  So the wards are named and measured, and the streets run per district, crossed by the arterials.
- **A district given as "within 260 m of downtown" or "everything else"** (the spike's `where`
  districts): a region feature can be `where`, but the streets step needs one ring
  (`district_ring`), and the block networks need `within` a region of one ring.
- **Lots too small for a house**: 1,467 of 2,744 lots hold none (the 5 m and 7 m row houses do not
  fit their frontage after `gap`, or the land under them rises more than `max_relief`). No asset
  of another size is in the pack.
