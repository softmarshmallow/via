# examples/borrowed: someone else's kit

K10 of `docs/plan-kits.md`: a freely licensed kit that nobody here made, laid by lib/building,
to find where our contract fits a kit written without it and where it does not. The kit is
**Kenney's Modular Buildings 2.1**, by Kenney (www.kenney.nl), under **CC0 1.0** (public domain;
credit is not required, and is given here). It is fetched on demand and never committed.

## Fetching the kit

```bash
python3 examples/borrowed/fetch.py
python3 examples/borrowed/packs/make_revised.py
```

`fetch.py` (standard library only) asks kenney.nl for a download link the way the asset page's
button does, refuses the zip unless its SHA-256 is
`47e1614686b4c0fe55190e88b22ff2b8b10935edf61f37f478a9b6a99477edc6` (1,825,490 bytes, the kit this
example was measured with), extracts only the 109 GLB models, the one texture they name and
`License.txt` into `packs/kenney/`, refuses any entry that would land outside that folder, runs
nothing from the zip, and prints the licence. `packs/kenney/` is in this folder's `.gitignore`.
`make_revised.py` writes the one model the `revised` case needs, from the fetched one, into the
same folder. `via pack accept` then holds the files to the digests in the committed facts files.
Tested 2026-11-24: one download through the script, the same SHA-256 and the same 110 files.

What is committed: the packs (`kenney.toml`, the pack the example is laid by; `plain.toml`, the
kit as handed over; `sill.toml` and `revised.toml`, each one change from `kenney.toml`, written by
`packs/make_cases.py`), their facts files (measured boxes and file digests, no model data), the
world, the changes, `measure.py`, `expect.toml`, this file, and a render
(`docs/reports/borrowed-sheet.png`).

## The kit as it is

Read from the 109 GLB files as data (bounds through each node's transform, faces by where their
triangles lie; glTF's frame, +Y up, the front +Z), and confirmed by `via pack accept`:

| Pieces | Count | Box (x by height by z) | What they are | Role here |
|---|---|---|---|---|
| `building-block` | 1 | 1 by 0.625 by 1 | A closed block, plain on every face | `role:wall`, the bay |
| `building-window*`, `building-windows*` | 23 | 15 are 1 by 0.625 by 1; 8 reach 0.025 to 0.075 past a face | A block with windows on its +z face; the 8 with a sill, an awning or a balcony | 3 as `variant:` (window, windows, wide); the proud ones refused (below) |
| `building-corner*` | 9 | 1 by 0.625 by 1 | A block whose +x+z corner is bevelled at 45 degrees (a cut on the plane x + z = 0.5, half that quarter), the outside faces on +x and +z | `role:corner`, turned (below); 8 with windows have no place |
| `building-door*`, `building-edges-door` | 4 | 1 by 0.625 by 1 | A block with a doorway 0.25 by 0.5 through its +z face | `role:door`, the doorway its width (`shared.door`) |
| `roof-flat-center` | 1 | 1 by 0.106 by 1 | A flat roof tile | `role:roof_flat` (new) |
| `roof-flat-border-*` | 8 | 1 by 0.212 by 1 | A tile with a border on one side, a corner, an end... | `storey:parapet`: the side and the corner, turned |
| `roof-flat-*` (others), awnings | 10 | up to 1.1 by 0.212 | Roof dressing, awnings | none |
| `roof-slanted` | 1 | 1 by 0.782 by 1.139 | One slope, 0.139 past the block at its eave, its ridge at the block's back | `role:roof_slope`, span 1.139 |
| `roof-slanted-corner-a/b/c` | 3 | 1.139 by 0.782 by 1.139 | A hip, its eaves on +x and +z, its outer corner bevelled as the corner block is | `role:roof_hip`, turned |
| `roof-slanted-corner-inner`, others | 4 | | A valley, a flat end, a slope with a window | none (a hip over an L is not built) |
| `roof-gable*` | 3 | 1.09 by 0.519 by 1 | A whole ridge for a building one block deep | none: not two slopes and two gable halves |
| `building-steps-*` | 4 | 1 by 0.837 by 1.2 | Steps up to a door block | none: steps to a door are not built (K7) |
| `building-sample-*` | 7 | up to 2 by 3.8 by 2.2 | Whole houses and towers | none here (lib/town's whole-house pieces would take them) |
| `door-*`, `window-*` | 28 | 0.19 to 0.75 wide, 0.05 thick | Leaves that sit in a block's hole | none |
| `detail-ac-*` | 2 | | Air conditioners | none (dressing) |

So: **the module** is a block, 1 in plan; **the wall** is the block, 1 thick, since a building is
blocks round its outline; **the storey** is a block's height, 0.625; **the door** is a block with
its doorway, 0.25 wide and 0.5 high, through it; the pieces are **whole blocks, not panels**, and
closed on every face, so a building has no inside to floor and the kit has no floor tile, stair
or inside corner. Every piece's origin is its base's middle but the slope's and the hip's, whose
origin is at the wall's top over the block, with the eave dipping 0.057 below it: the build
moves each to its box (`asset.pivot_moved`, an info), so the slope sits 0.057 higher than Kenney
places it. The texture is one external PNG (`Textures/colormap.png`).

### Scale

The kit's unit is not a metre. A doorway 0.25 by 0.5 and a storey 0.625 are a quarter of a
person's: a person is 3.4 doorways tall. The contract takes model units as metres and leaves a
model made at another scale open (`spec/assets.md` 2.3), and lib/building reads every size from
the pack, so this town is a quarter of a person's size. Everything that is the kit's own (closure,
corners, roofs, doors in blocks) is measured here; everything that is a person's says so: four
`door_narrow` in the base, one per doorway, because the block or the roof over each doorway
stands within a walker's head (check.py's `HIGH`, 2 m). A factor belongs in the pack, applied
where a model is measured, so no library reads a scaled number (filed under `spec/assets.md`).
Nothing was scaled by hand here: a world written in the kit's units is what a studio's author
would get today.

### The bevels, and what the sheet shows

Kenney's building corner is not a block with a quarter cut out: it is bevelled, its outer corner
cut at 45 degrees by the plane x + z = 0.5 of the model, so the outline's corner is open by a
triangle of 0.125 m2 at every storey; its hip is bevelled the same way, so the hipped roof is open
over each bevel (55 dm2 inside the cottage's outline, 81 in its eaves, all within 0.49 m of a
corner). Nothing over a room is open: the hole is where the block itself is cut. That is the kit,
not a fault of the fit, and it is measured from the models' triangles (`corners_cut_out`,
`roof_open_dm2`, `roof_eaves_open_dm2`), which a quarter turn the wrong way changes: with the
corner turned `-z` and the hip `-x`, every corner is cut on its inner side and 120 dm2 of the
roof is open inside the walls, none in the eaves, and the base's rows fail (K10's review).
Neither `lay` nor `check` can say it: both read a piece's box, which is the same however a piece
is turned and whatever is cut from it, so only a measured plan from the geometry would (filed
under `spec/assets.md`).

The dark triangles at every building corner on the sheet are the storeys' floors seen through
the bevel: a storey's floor is its outline, square at the corner under the bevel, and a kind
surface has no material, so the loader draws it dark grey (filed under `engines/godot`). The
floor also lies under the blocks themselves, which are solid: it is the storey's walk, not a
slab, and is kept.

## Before

The kit as a studio would hand it over (`packs/plain.toml`): each piece's role and pitch, and
the three things every block shares, read off the files, nothing fitted. `via pack check` and
`via pack accept`: `0 errors, 0 warnings, 3 infos`. A world written as for our own kits (a house
of two storeys with a flat roof, a cottage under a hipped roof, a row of three), built with
lib/building of 8c9ba2c:

```
error[pack.unknown_material] field.ground: the pack binds no material `ground`
warning[step.lay.wall_open] feature.cottage: a wall stretch of 0.95 m on side 0 of storey 0 does not close with the kit's pieces
  (twice; and four times of the row's houses)
warning[place.crowded] run."lay#cottage_roof_e0": `lay#cottage_roof_e0#r0` of `lay#cottage_roof_e0` and `lay#cottage_roof_h0` overlap; the step `lay` returned both, so neither gives way
  (eight in all: every slope with a hip)
warning[step.lay.roof_none] step.rows: the hip roof is not laid: the roof is 4 m deep and the kit's slope piece is made for 2 m (twice its span). The flat roof is laid in its place
3 errors, 15 warnings, 11 infos
```

The other two errors were the world's, not the kit's (a house of two storeys with no `stair`;
a row running past the extent, which `lay` raised as `protocol.raised` with a traceback: filed).
Then, with the pack as below and lib/building of 8c9ba2c, `check` said of the kit
"`slope` has a span of 1.139 m, which under the stated eaves of 0.139 m roofs a building 2 m deep:
no building of the kit is that deep (whole pieces of 1 m and two corners of 1 m)": wrong, since
the cottage is that building. The case `handed_over` keeps the before as a build on today's
library (`expect.toml`): it stops on the material, and says the hip and the slope stated at the
block's 1 and not their box's 1.139 (`kit_deck`, twice), so they stand in each other (`place.crowded`,
ten), and leaves 1.9 m open at each doorway (`gap_wide`).

## What the pack states, and what was mended

`packs/kenney.toml` against `plain.toml`, each line for a reason the before gave:

| Stated | Why |
|---|---|
| `[material.ground]` | A world paints its ground by name; every pack binds it. The colour is ours |
| `front = "+x"` on the corner, the hip and the parapet's corner | Kenney's corners have their outside faces on +x and +z; lib/building's on the front and the front turned a quarter right (lay.py rule 6). A quarter turn, read off the geometry |
| `shared.door = 0.25` | The doorway through the door block. With it, rule 6 closes the wall: the gap is 0.35 m, each stretch beside it is shortened to whole blocks, and the door block fills the metre left. Without it, the library's 1 m doorway and 1.9 m open |
| `shared.eaves = 0.139`, `span = 1.139` | The slope and the hip reach 0.139 past the wall's face: under eaves a hip roof is laid on the outline grown by them (rule 17), and the pieces' span is their box's |
| `role:roof_flat` | The flat roof's tile is a roof piece, not a floor: it stands proud, level with its border's flat part, and the kit has no floor |

Mended in lib/building (each with its rule, a note in `lib/building/README.md`, "Someone else's
kit", and tests in `test_borrowed.py`):

- **A roof's own tile** (lay.py rule 10). A flat roof (and a terrace) is tiled with the kit's
  `role:roof_flat` piece where it has one, its floor tile where not. Before, a kit's floor tile
  was its roof, sunk by its thickness.
- **A stretch beside a gap that no piece fits goes** (rule 6). A stretch beside a door that does
  not close was shortened to the next length some piece makes, never to none, so a door block
  in the bay beside a corner left 0.325 m open. Now a stretch shorter than any piece of the kit
  goes and the gap runs to the corner's face.
- **The wall either side of a door piece, each on its own** (rule 6, K10's review). The door
  piece stands on the door's own middle, so a gap widened on one side left it off the opening's
  middle while the widths alone passed: a village door 1.24 m along left 0.19 m open beside its
  piece with no word. `gap_wide` now judges the open wall on each side of the piece as laid. A
  sweep of door places 10 mm apart over six kits, reading holes from what was laid, finds none
  unsaid (240 in the village kit and 264 in this one with the library before K10). With both
  rule 6 mends, none of the village, temple, court, works and kit examples' 1,203 recorded rows
  moved, nor `realm`'s 184, nor `station`'s and `college`'s bases (`block` was not rebuilt).
- **A building as deep as its two corners** (check.py rule 10). Under eaves, a slope is held to
  a depth a building of the kit can have; in a kit of blocks, whose corner reaches as far as its
  bay, that may be the two corners and nothing between (Kenney's hipped houses); in a kit of
  panels it is at least one piece, and with posts one bay (K10's review: as first written a
  panel kit's slope roofing a building as deep as its two corners passed).

A door rule of its own was built and taken out again: "a door piece that is a bay" (stated on
the door, cut to its pitch with no margin, held to a bay's middle). With the doorway's true width
stated, rule 6 as it stood does the same, and its `gap_wide` already names what is open and
where a door would close. One rule fewer.

Kept as it was, and said: **a variant proud of the wall.** Eight of the 23 window blocks reach
0.025 to 0.075 past the block's face (a sill, an awning, a balcony), and rule 8 refuses each as
a variant, as K3 decided and a test pins (a variant is the bay's whole box). A front proud of the
bay overlaps nothing in its run, but allowing it reverses a ruling for dressing; a sill as a piece
of its own would take it. The case `sill` shows the refusal.

## The base build

`world.toml`: a house of two storeys, six blocks by four, a door, a flat roof with its border; a
cottage, five by two, under a hipped roof; a row of three houses, five by four, flat roofs; level
ground. `measure.py`'s protocol is in its header; every figure here is from `expect.toml`.

| Measure | Base | |
|---|---|---|
| Built | 0 errors, 5 warnings | 4 `door_narrow` (the kit's size), 1 `unreached` (the cottage: no door, no inside) |
| Laid | 21 features, 178 instances | 84 blocks (4 doors, 24 corners), 10 roof pieces, 26 roof tiles, 58 parapet pieces |
| Walls | 0 m open | every storey of every building, measured from the instances |
| Roofs | 55 dm2 open inside the outlines and 81 in the eaves, 0 dm2 twice | from the models' triangles, on a 0.1 m lattice over each roof's region (the hipped roof's grown by the eaves): every open point is the cottage's, within 0.49 m of a corner of its outline, where Kenney's hip is bevelled over the bevel of its corner block. The flat roofs are covered once |
| Corners | 24 bevelled outward, 16 whole | each corner piece's shape, read from its triangles: the 24 building corners are cut at the outline's corner, the 16 parapet corners are whole |
| Written | 17 tables, 67 numbers | 42 in the three building tables (the house, the cottage, and the row of three), 18 in the views, 6 the frame's (extent, resolution, seed) and 1 the walls layer's `rank` |
| Unchanged on its store | 0 moved, 0 added | `warm` |

## Cases

| Change | What it is | What is said | What is done |
|---|---|---|---|
| `handed_over` | The kit as handed over (`plain.toml`) | `pack.unknown_material` (error, the build stops); `kit_deck` twice; `place.crowded` ten; `gap_wide` twice, 3 m left for a 1.1 m doorway; `slab_open`; `variant_none` (the wide window) | Nothing |
| `three_storeys` | The house a storey taller | Nothing new | 16 instances added: a storey of blocks, the stair's second flight |
| `wide` | The house half a block wider | `wall_open` five times, 4.5 m (the front and the back, each storey and the parapet); `gap_wide`; `slab_open` | 14 m of wall open; 10 m2 of roof open |
| `door_off` | The door half a block over | `gap_wide`: "the wall leaves 2 m at door `house_d0`, which needs 1 m: 1 m stands open" | 1 m of wall open beside the door block |
| `solid` | A door on the cottage | `door_narrow` of the cottage: its doorway ends in the back wall's blocks | The door block is laid; a building two blocks deep has no inside |
| `deep_hip` | The cottage three blocks deep | `roof_none`: "the roof is 3.278 m deep and the kit's slope piece is made for 2.278 m (twice its span)" | The flat roof inside the walls; the wall tops and the eaves not roofed by pieces |
| `sill` | The window with a sill asked for | `variant_none`, `kit_variant` and `kit_stated`: 1.05 m thick for the block's 1 m | The plain block in its place |
| `revised` | The window block re-exported 1.1 times as long | `variant_none` twice; `kit_variant`, `kit_pitch` | The plain block wherever the window was asked; nothing stands over its neighbour |
| `turned` | The corner with no `front`, as Kenney exported it (fix plan F5b) | `kit_turn`: its measured plan open at its front left, standing a quarter left; `front = "+x"` named | Every corner turned a quarter: its bevel against the walls, unsaid before |

## What each author wrote

**A kit's author** (`measure.py --packs`): the pack a kit is laid by, counted the same way for
each (lines that are neither blank nor a comment; numbers outside comments, an array's each;
tags; facts in `defaults.shared`; assets that write `front`).

| Pack | Assets | Lines | Numbers | Tags | Shared facts | Turned |
|---|---|---|---|---|---|---|
| `examples/kit` plain | 14 | 86 | 38 | 20 | 8 | 0 |
| `examples/block` city | 15 | 91 | 29 | 26 | 9 | 0 |
| `examples/village` village | 16 | 105 | 71 | 20 | 9 | 0 |
| `examples/temple` order | 12 | 72 | 21 | 18 | 5 | 0 |
| `examples/court` timber | 17 | 101 | 30 | 25 | 9 | 0 |
| `examples/works` works | 23 | 196 | 85 | 20 | 13 | 0 |
| **Kenney, as handed over** (`plain.toml`) | 10 | 55 | 12 | 14 | 3 | 0 |
| **Kenney, laid** (`kenney.toml`) | 11 | 67 | 18 | 16 | 5 | 3 |

What the second pack has that the first had not, and was worked out rather than read off a
number in a file: the three quarter turns (read off which quadrants of each piece hold
geometry), the doorway's width (the door leaf's box, and the hole in the door block's face),
the slope's span as its box and not its block, and the ground's colour (ours). Every other
number is a measured box.

**A world's author**: 67 numbers in `world.toml` for 199 things laid (21 features, 178
instances). Of the 42 in the buildings, 10 are the kit's or the library's, written again: the
storey's height, 0.625, three times (the kit states it; a building still writes `storey_h`); a
plinth of 0, three times (the library's default, 0.3, is a person's plinth); the stair's place,
width and run (the kit has none); and a door's place on a block's middle. Against our own kits'
worlds: 27 numbers in the buildings of `examples/kit`, 28 of `village`, 17 of `temple`, 22 of
`court`, 52 of `block`, 61 of `works` (the same count: `[feature.*]` and `[instance.*]` tables
that are no view).

**Reusing a kit in a second world** (K11, `examples/realm`): a script of 115 lines that writes
one pack from three, since a world takes one pack and kits' ids collide. Here no such script was
needed: one kit, one pack.

## Pattern against archetype

| Pattern | Archetype (`docs/ledger/archetypes.md`) | Where it lives now | Verdict |
|---|---|---|---|
| A kit nobody here made | Asset: what a kit states once | A pack that states what is true of the files; the files fetched, checked by digest, never committed | Holds: nothing in the kit was changed, and five facts and three turns made it fit |
| Blocks, not panels | Linear placement | A block is a wall piece as thick as itself (rule 6) | Holds, with no rule added |
| A door that is a block | Meeting; Linear placement | Rule 6's gap with the doorway's true width | Holds; one fault mended (a stretch shorter than any piece) |
| A corner of another hand | Asset: a claim a piece makes alone | `front` in the pack | Holds; a quarter turn is all the contract can say. Nothing in the library sees a wrong one (it reads boxes): `measure.py` does, from the triangles, and `check` would need the core's measured plan |
| A roof that is not a floor | Grammar | `role:roof_flat` | Holds |
| A roof made for one depth, and eaves | Asset; Grammar over Area | `shared.eaves`, span by box (rule 17) | Holds; `check` corrected to know a building as deep as its corners |
| The kit's own unit | Asset: measured, not asked | Not in the contract (`spec/assets.md` 2.3, open) | Does not hold: the town is the kit's size, and a person's measures say so |
| Dressing proud of a wall | Asset: a claim and a body (R69) | Refused as a variant (rule 8) | Said; a piece of its own would take it |

## Still open

- **Scale** (`spec/assets.md`): the one thing that keeps this kit out of a person-sized world.
- **What has no role** (`lib/building` in `TODO.md`): corners with windows, Kenney's gable, a hip
  over an L, steps to a door, leaves in a block's hole, whole sample houses.
- **Challenge 33**: a real kit laid by lib/building, not yet by lib/town's kit run (`market`);
  Kenney's sample houses are the shape that run takes (`examples/market` in `TODO.md`).
