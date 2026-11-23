# realm

A small fantasy map: three settlements built from three of this plan's kits, on land a step
makes, joined by a street, two roads, a trail and a ford, with the player map as data
(`docs/plan-kits.md`, K11). Our own design, and our own names: none is from a game, a book or a
film. Every number below is from a run; its protocol is in `expect.toml`.

```
python3 examples/realm/packs/make_pack.py      # after a kit changes (Python 3.11+)
via pack accept examples/realm/packs/realm.toml
via build examples/realm/world.toml --out build/realm --store build/realm.store
engines/godot/sheet.sh --before build/realm --out review/realm --pack $PWD/examples/realm/packs --option water=tag:water_level
python3 tools/map_render.py build/realm build/realm/export/map --out review/realm-map   # needs PIL
```

| File | What it is |
|---|---|
| `world.toml` | The land: 768 by 640 m at 1 m, made by lib/land's `uplands` step, the sea to the south |
| `layers/places.toml` | The three settlements, and the steps that return their buildings (`rows`, `courts`) |
| `layers/ways.toml` | The street, the ford, the roads and the trail; the ground's paint |
| `layers/walls.toml` | `lay` and `check`, which read every building the layers below wrote or returned |
| `layers/names.toml` | The map's names and areas, the map's export, the reach lint, the views |
| `packs/make_pack.py`, `packs/realm.toml` | One pack holding the three kits (below, "Reusing a kit") |
| `changes/` | Nine changes an author of a map would make, each built `--against` the base |
| `measure.py`, `expect.toml` | The measures, and the rows a build must hold |

## Design

Proposed in K11's note and built as proposed but where said.

- **One world, no imports.** 768 by 640 m: three settlements a few hundred metres apart, the
  temple up the hill. An import was not used because a world has one pack and each kit's example
  binds `door`, `tile` and `base` to its own pieces: three imports would be
  `import.binding_differs` (spec/composition.md 10), and the kits are the subject.
- **1 m between posts.** `lay`'s rules are in millimetres and do not depend on the posts: the
  temple alone, on a slope of 5 in 100, says the same at 0.5 m and at 1 m (one `base_short`,
  0.36 m along 31.8 m, in both). The doorway's sill is held over at least the posts' diagonal
  (lay.py rule 4), 1.5 m at either. A cold build of the realm takes about 37 s at 1 m (the
  `build_ms` row); 0.5 m would be four times the posts for nothing the kits need.
- **Land that is made, not written.** lib/land's `uplands` step: noise falling to the south and
  the water that runs off it (30 rounds), returning the land and the main stream as a `beck`
  (`uplands#stream`). Chosen over lib/terrain's erosion model for its time: the land builds in
  about 30 s alone, where basin's land takes minutes. The shape asked for (`relief = 40`,
  `rough = 0.6`, `erode = 0.008`) is the third tried: at `relief = 60` the hills were one ramp,
  and at `erode = 0.02` the slopes were gullies a metre apart with no ground a court could
  stand on. The sea is the land below 0, `sea = 0`. Hills to the north, a beck down a valley to
  the sea: as proposed. No lake: the step makes none, and the sea does its part (`flood`).
- **Three settlements.**
  - **Marrowick**, the village: two rows of timber-framed houses either side of a street, from
    the village kit of `examples/village` (`row`, lay.py rules 1 to 14), 60 m west of the beck.
  - **Senhal**, the court: a walled yard of 13 by 12 bays with a hall and two wings, from the
    timber and yard kits of `examples/court` (`court`), on a shoulder of the hills to the east.
  - **Quillow Fane**, the temple: a cella in a colonnade on four sides, on a podium, from the
    order kit of `examples/temple` (`building`, rules 15 and 16), on a knoll to the north-west
    that tops the ground within 30 m of it by 4 m.
  No pipes: K9's pipes hold on level ground only, and nothing here is.
- **What joins them.** A lib/town `street` between the rows (proposed as a road: see "Before");
  a `ford` on the beck, placed on the step's stream by `{ of = "uplands", class = "feature" }`,
  so it moves with the land; two lib/ways `road`s, village to ford and ford to court, and a
  `trail` from the village to the temple, each written by what it joins and routed over the
  land. The doors meet the ways by lib/building's own footpaths and doorways.
- **The player map.** lib/map: four `poi`s and two `area`s with labels, the export
  (`export/map/`), drawn by `tools/map_render.py`.
- **The walk.** lib/review's reach lint with quay's walker (rise over run 0.5, a step of 0.2 m,
  0.6 m wide, wading 0.3 m): from the village to the ford, the court's gate and the temple's
  steps, and from the court to the temple.

## Reusing a kit

A kit was written as one example's pack. Read together, the three do not make a pack:

| What | Said |
|---|---|
| A world naming two packs, `pack = [".../village.toml", ".../timber.toml"]` | `doc.type`: "the world's `pack` takes a string, and a list is written" |
| The two packs' text joined in one file | `doc.syntax`: "the table `taxon.structure` is defined twice" |
| With that mended by hand | seven ids bound twice: `base`, `base_corner`, `door`, `post`, `roof_gable`, `roof_slope`, `tile`; and `material.ground` coloured two ways |

So `packs/make_pack.py` (115 lines, standard library) reads the three packs and writes one:
every asset's id gains its kit's prefix (`v_`, `t_`, `o_`: 45 assets), every `file` is written
from one `root`, the `examples` folder, so the models are the other examples' own files and
none is copied, `[taxon."structure"]` is written once, and the land's paints are added. Nothing
else changes, and no line of a world: lib/building reads a kit by its taxon and each piece by
its `role:` tag (docs/design/kit-contract.md), so no world names a piece by its id. What it took,
for K10's last review: **one script, and nothing in the kits**. What it says: a pack is a file
and an id is global to a world, so a kit is not yet a unit a second world can name (Asset law 5
holds for what a kit states, not for where it lives). Filed under `spec/assets.md`.

## Before

Built with the library as it stood at 8c9ba2c, in this order, before anything was mended. The
outputs are quoted, not kept as builds (the scratch folder they were in did not survive).

**First build** (the layout of `layers/places.toml` with a lib/ways `road` for a street, the
ford further down the beck, the ways routed over the land alone): 13 errors, 395 warnings.

| Said | Count | Why |
|---|---|---|
| `surface.gap` | 13 | Every door beside a way: "`lay#rows_north_row_0_d0.end` is at 10.049 m and `lay#rows_north_row_0_s0.floor` at 10.68 m: a gap of 631 mm"; the temple's two doorways 270 and 432 mm. A doorway's sill was derived and gave way to the way's bed |
| `step.check.unreached` | 17 | The storeys behind those doors |
| `place.keep_out` | 362 | A way's verge keeps trees off 3 to 5 m past its edge, and pieces of all three settlements stood in one: the rows beside the road, the yard's gate and corners where a road ended, the temple's door where the trail did |
| `geom.unowned_crossing` | 14 | 12 the rows' footpaths across the road; 2 the roads across the beck (still said, below) |
| `step.lay.base_short` | 4 | The temple's base 0.786 m short along 49.7 m; the court's 0.271 m along 14.2 m |

What it took, each tried in turn:

- **A doorway beside a way.** Made anchored, the 13 gaps went and `authority.squeezed` came:
  two doors that face each other across a street ask two levels of one post (three pairs in
  the realm, four a build in `examples/village`, which had built clean), and the two doorways
  of a door behind a colonnade, one behind the other, were squeezed against each other (two). So the sill is
  **shaped with `order = 2`**: after every other shaped entry, whatever its layer. Mended in
  lib/building (README, "Level ground, and a sill beside a way").
- **The street.** A lib/town `street` (a carriageway and two footways, its verge only to the
  footway's back edge) with the rows 6 m off its line and no footpaths (`walk_out = 0`), the
  court's gate point 9 m out from its wall: the 362 `place.keep_out` and 12 of the crossings
  went.
- **A road through the temple.** The trail, routed over the land alone, came over the knoll and
  through the temple's outline (`through_fane`). The route's cost now rises by 60 within 12 m
  of a building. Written by hand in each route; filed: a way that keeps off what stands is the
  route's to know.
- **The ford.** Placed first at `t = 0.72`, 40 m below the village, so the road ran back up the
  far bank; now at `t = 0.565`, beside the street's end.

**Level ground.** Then, with no `ground = "level"`, the temple and the court stood on the land
as it lies (`no_level`, below). What existed, each written by hand over the temple's outline
and the yard's four corners (which a court's author never writes: they follow from `at`, `to`
and the bays), with the library as it now is:

| Kind, written by hand | What was said | What it did |
|---|---|---|
| lib/land `pad` | `place.keep_out` 403: a pad keeps trees off, and every piece of the two kits on it is warned of; `base_short` of the temple, 0.063 m along 4 m | Levelled both: the halls on one level, the yard wall's pieces within 0.119 m |
| lib/land `firm_pad` | `surface.gap` 6 (the temple's two doorways 820 and 823 mm, the court's four 577 mm), `place.keep_out` 403, `unreached` 4 | Anchored: it holds the ground at its own mean over every doorway's sill, and the build stops |
| lib/town `terrace` | `base_short` of the temple, 0.147 m along 41.7 m | Levelled at its own mean, read where its bank begins: the court's halls on one level, its wall's pieces 0.259 m apart, the temple's foot 1.047 m over the ground |
| lib/town `enclosure` (the court alone) | `surface.gap` 4 (577 mm each), `unreached` 3; `formula.empty_selection`, since it has no `gate` | Anchored, as `firm_pad`; and its wall is a bank raised on the terrain, not the yard kit's pieces |

(With the sill anchored, as first tried, the anchored two were `authority.conflict` at every
door, an error.) Each needed the outline
written again; none let the step that reckoned the floors say what level the ground is.

So `ground = "level"` (lay.py rule 19, court.py rule 9, `kind.plot`): a plot levelled at the
level the building's floor was reckoned from, with pad's bank outside and nothing kept off.

**What the review found, and the plot's order** (K11's one independent review). As first built
the plot was shaped at `order = 0`, so a way in a layer above cut it: `senhal_road`, which ends
9 m before the gate, cut the court's plot up to 117 mm at the gate (what this README then
called the yard wall's pieces "within 0.065 m, on the plot" was that cut), and in the
reviewer's own worlds a road beside a temple cut 0.99 m under its colonnade and two temples
2 m apart stood one in the other's bank, with nothing said: `lay` judged each wall's foot
against the level the plot states and cannot see what is shaped after it. The plot is now
**`order = 1`**, after every shaped way and before a doorway's sill: the court's plot holds to
the millimetre (`plot_off_mm` 0, `yard_z_mm` 0), and so do the reviewer's road probes. What can
still override a plot (an anchored kind, a second plot over the same ground, an entry of a
higher order) `check` now says (check.py rule 12, `plot_lost`): the case `shrine`. And the
plot's `max_cut` and `grade` are the court's or the building's, so `steep_court`'s error is
answered on the court: with `max_cut = 6` on Senhal it builds (checked once, not recorded).

**A step that reads what it returns.** The court's plot was first returned by the court step
in the same layer as the ways: `diag.internal` "`step:courts` gave another value in the final
graph than in its layer's pass". The court step reads the ground to find the plot's level, and
the roads, a step's peers in that layer, wrote the ground it read. With the ways in a layer
above the places, it builds. Filed under `spec/graph.md`: the engine knows, and says it as an
internal fault, not as what to do.

## The base build

Nothing is said but the two roads' crossing of the beck at the ford they end on (`Still open`).

| Measure | Value | |
|---|---|---|
| Written | 29 feature tables, 141 numbers; 104 in the places and the ways | `written_*` |
| Laid | 87 features (15 buildings, 20 storeys, 17 doorways, 15 roofs, 2 plots...; the temple, the sixteenth building, is written by hand), 1511 instances | `laid_features`, `instances` |
| Doors met | 17 of 17, gap 0 | `doors_met` |
| The wall's foot | 0.9 m over the ground at most (the temple's podium, on its plot, where its base is 0.9 m deep), none bare | `foot_max_mm`, `foot_bare_dm` |
| The ground over a floor | 0.255 m at most: the uphill back of a north-row house, whose floor is the mean of a slope | `buried_max_mm` |
| The court | its halls on one level (0 mm); the yard wall's pieces on one level (0 mm); both plots level to the millimetre away from the doors' sills (117 mm at the court's gate before the review's fix, where a road cut it) | `court_floor_spread_mm`, `yard_z_mm`, `plot_off_mm` |
| The ford | 0.281 m of water, which a walker wades | `ford_depth_mm` |
| The ways | the street 68 m (5 in 100 at most over 10 m); village to ford 49 m (9); ford to court 345 m (12); village to temple 429 m (25: the trail climbs the knoll) | `*_m`, `*_grade_pct` |
| The walk | 4 of 4 stated pairs: village to ford, to the court's gate and to the temple's steps; court to temple | `walk_pairs`, `walk_held` |
| The map | 11 labels, 25 ways, 1 stream, 2 areas, 3 floors; every way and the stream on its own cells (`tools/map_render.py`: registration 1.0, 0 label overlaps) | `map_*` |
| Time | about 37 s cold (4786 nodes, 4605 run); the base again on its store 0.15 s, nothing run | `build_ms`, `warm` |

Sheet: `docs/reports/realm-sheet.png` (the base beside `flood`); the player map beside the build's
plan: `docs/reports/realm-map.png`.

## Cases

Each built `--against` the base on its store. "Nothing" means no record at all.

| Change | What an author did | Said | Done |
|---|---|---|---|
| `village_moved` | Moved Marrowick 40 m up the valley: ten numbers in three tables | Nothing new | The roads and the trail, written by what they join, follow (833 instances move); the street climbs 10 in 100. The houses step with steeper ground: the ground stands 0.70 m over a house's floor at its uphill back, which nothing says |
| `new_seed` | Another seed for the land | `geom.unowned_crossing` 25: the roads cross the new stream's course | All 1511 instances move with the land; the ford moves with the stream. The settlements stay where they were written; a temple's base still reaches, the ground stands 0.40 m over a floor. Nothing says that a settlement no longer stands where it was chosen to |
| `larger_court` | Senhal of 17 by 14 bays, the hall and wings further out: one table | Nothing | Its plot follows (311 moved, 12 added), the halls on one level, the yard wall's pieces on one level |
| `flood` | The sea 9 m higher | `step.reach.stranded`: the ford is 3.2 m deep | Two halls and 32 pieces stand in water and 66.3 m of road under more than 0.3 m. Nothing says a house is in the sea |
| `no_ford` | The ford taken away, one road from the village to the court | `geom.unowned_crossing` 1 | The road crosses the beck where the land sends it; the walker wades it, 3 of 3 walks held |
| `no_level` | No plot under the court or the temple | `base_short` 4 | The temple's foot 1.706 m over the ground, 102.9 m of foot bare; the court's halls 0.499 m apart in height, its wall's pieces 1.225 m |
| `steep_court` | Senhal on the hillside west of the village | `authority.max_change`: "`courts#senhal_plot` moves `height` by 5.15474059 where `max_change` allows 3" | Stops. The land is too steep for the court as it is written, and the error names the plot; the court's `max_cut` (the plot's, passed on) answers it: with `max_cut = 6` it builds |
| `through_fane` | The trail routed over the land alone | `place.keep_out` 108 | The trail goes through the temple: 15.2 m of it inside the temple's outline. The temple's plot holds under it (`plot_off_mm` 0) |
| `shrine` | A shrine of the timber kit 2.1 m behind the temple, on a plot of its own: one table | `plot_lost` of the temple: "the ground along its wall falls 1.259 m below the floor, 0.359 m past the base's 0.9 m, along 15.4 m; the nearest thing that shapes the ground there is `lay#shrine_plot` (plot, 0.6 m off)" | The two plots overlap and the shrine's, later by name, holds the ground between them: the temple's plot is 0.539 m off its level there. Said, not mended: two buildings laid in one layer do not see each other's plots |

## Pattern against archetype

| Pattern of a map | Archetype (`docs/ledger/archetypes.md`) | Where it lives now | Verdict |
|---|---|---|---|
| A kit in a second world | Asset: what a kit states once | One pack written from three by a script, ids prefixed | Partly. What a kit states travels; the kit does not: a pack is a file and an id is the world's |
| Settlements on land a step made | Placement | `row`, `court`, `building` at coordinates, on `uplands`' land | Partly. Each stands where it was written; a new seed moves the land under it (`new_seed`) |
| Level ground under a building | Profile | `kind.plot`, returned with its level by the step that reckoned the floor, ordered after the ways; `check`'s `plot_lost` where something overrode it | Holds. A row is not levelled: its houses step |
| A door beside a way | Meeting over Profile: the order of shaped entries | The doorway's sill shaped, `order = 2` | Holds against every shaped kind; an anchored one still wins |
| A way between named places | Profile; Network for the street | `road`, `trail` routed by a cost, ends named | Holds; keeping off a building is written in each route |
| A way across water | Meeting (R18) | A `ford` on the step's stream | Partly: the roads end on it and are still said to cross the stream (below) |
| Names on the map | Markup | `poi`, `area`, lib/map's export | Holds |
| A walk from one place to the next | Measure | lib/review's reach | Holds: four pairs of four |

## Still open

- **A way that ends on a ford placed on a made stream is said to cross it**, at the ford's own
  point: a produced name cannot be written, so the ford cannot be named onto the stream (TODO,
  `spec/geometry.md`).
- **Nothing says a house stands in water** (`flood`) or **that the ground stands over a floor**
  of a house on no plot (the uphill back of a house that steps, 0.26 m here and 0.70 m once
  moved): TODO, `lib/building`. On a plot `check` says it (`plot_lost`).
- **A settlement on land a seed changed** is laid again where it was written (`new_seed`): a
  place chosen for the land it stands on (a knoll, a shoulder, a bank) is not stated as such.
- **A way that keeps off what stands** is written into each route by hand (`through_fane`).
- **Two plots over one ground are said, not avoided** (`shrine`): two buildings laid in one
  layer do not see each other's plots, and the later by name holds the ground between them.
  `check` says it (`plot_lost`); nothing prevents it (TODO, `lib/building`).
- **A step that reads the ground in the layer where the ways that shape it stand** is
  `diag.internal` (TODO, `spec/graph.md`): the ways are a layer above the places for that.
- **A kit is not a unit a second world can name** (TODO, `spec/assets.md`; above).
- **Steps up to a door** are still not built: each doorway raises a mound to its sill, plainest
  in front of the temple's podium (K7, TODO).
- **No imports, no import of a settlement**: the realm is one world because of the pack; with a
  kit a world can name, each settlement could be its own example, imported.
- **Pipes** are not placed: K9's hold on level ground only.
