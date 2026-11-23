# court: a walled court of timber halls

A compound about one axis: a gate, a yard wall, a main hall across the axis with a veranda along
its front, and two side halls that face each other, walled from a grey kit made by code. It is
the East Asian half of K8 of `docs/plan-kits.md`: the roof as a kit's main piece, posts and
bays, and a walled compound.

**This is our own design.** The research found no developer's account of East Asian building
from pieces: no curved roofs or deep eaves as kit pieces, no bracket sets, no post-and-bay
modules, no verandas, no walled compounds with gates; only that generated streets came out too
straight and were replaced by period maps, and one mention of a separate node for layered roofs
(`docs/research/Modular kit failures in studios.md`, section 12). So nothing here recreates a
studio's failure. The cases are what the building library said and did when the court was first
written with what it had, and what a court like this one gets wrong when one number is changed.

## What is here

| File | What it is |
|---|---|
| `packs/make_kit.py`, `packs/timber.toml` | Two kits in one pack. `structure/timber`, the halls: a post 0.3 m square, which also stands on the corners; a panel 2.7 m long at a place that repeats every 3.0 m, the same with a lattice window, the same as four door leaves; a door frame; a floor tile half a bay square; the base, half a post thick and 0.6 m deep; the veranda's post and beam, tagged `storey:colonnade`; and the roof, a slope a bay long and a hip one span square, for each of two spans, 6.0 and 4.5 m. It states `eaves = 1.35`. `structure/yard`, the wall round the court: a bay 3.0 m long and 2.4 m high, a corner post, a gate. `packs/short_eaves.toml` is the same pack stating eaves of 0.9 m. |
| `world.toml`, `walls.toml` | The court on level ground: one `court` feature. `walls.toml` is the layer with `lay` and `check`. |
| `by_hand.toml` | The same court as four `building` features written by hand. |
| `changes/` | Seven changes. |
| `measure.py`, `expect.toml` | What each build is measured by, and the measured numbers with their protocol. |

```sh
python3 examples/court/packs/make_kit.py
for p in timber short_eaves; do via pack accept examples/court/packs/$p.toml; done
export VIA_LIB=$PWD/lib
via trust examples/court/world.toml --yes
via build examples/court/world.toml --out build/court --store build/court.store
python3 examples/court/measure.py build/court --world examples/court/world.toml --check base
```

The ground is level (10 m): the kit and the plan, not the land, are the subject. The gate is in
the south wall and the axis runs north.

## Before

The court was first written as four buildings by hand and built with the library as it stood
after K8's first slice (posts and bays, the colonnade). It was the first building walled by
posts on its own outline: the temple's posts stand in its colonnade only.

| What was tried | What the build said | What the build did |
|---|---|---|
| `roof = "hip"` on the three halls | `params`, three errors: "`roof` is 'hip'", hint "write "flat", "gable" or "garden"" | Stopped: exit 1, nothing laid |
| `roof = "gable"`, the nearest word it knew | `roof_none`, three times: "the kit has no `role:roof_slope` and `role:roof_gable` pieces. The flat roof is laid in its place". `kit_deck`: "`structure/timber` has `roof_slope_45` and no `role:roof_gable` piece: a gable roof needs both" | Three flat roofs, on the outlines: 259 m2 where the kit's roofs cover 432 m2. No eaves: nothing could be laid beyond a building's outline. One slope piece was read, the first by name, so the kit's second span was never seen |
| The halls' floors, a tile half a bay square | `slab_open`, twice, of the main hall: "11 floor tiles of the platform cannot be laid whole: between the colonnade and the wall behind it is no whole number of 1.5 m tiles", and 15 of the flat roof | The floor was laid inside the panels' inner face and the veranda's inside its posts: 14.7 m and 2.7 m, no whole number of any tile but one a post wide |
| The yard as a building of one storey, from a kit with a wall, a corner and a gate and no floor or roof piece, the halls inside its outline | Nothing: no error, no warning, from the engine or the library | The wall and the gate stood. And the court had a storey: a level floor of 39.6 by 36.6 m at the mean of its ground, a second surface from wall to wall under the three halls, drawn as a slab (the sheet's left column) |
| The two side halls as mirror images | Nothing | What the author worked out: the east hall's four corners are the west hall's reflected about x = 32 and started from another corner, so that side 0 is its front; its far end is its side 3 where the west hall's is its side 1 |
| Two things the kit said wrongly (ours) | `kit_pitch` and `kit_module` of the door frame, stated at its own length, 2.7 m; `kit_stated` of the veranda's beam, 0.3 m thick where the kit states walls of 0.15 m | Corrected in the kit: a door between two posts repeats as a panel does, at 3.0 m; the beam is a panel's thickness |

What the author writes, counted by `measure.py` (`written_numbers`: the number literals in the
`building` and `court` tables, comments left out):

| | Tables | Numbers | Of which corner coordinates |
|---|---|---|---|
| `by_hand.toml` | 4 `building` | 63 | 32 |
| `world.toml` | 1 `court` | 22 | 0 |

Each of the 32 is a sum: a wing's back is its front and two bays and a post (9 + 6.3, from the
axis at 32: 47.3 and 16.7); the yard is thirteen bays of its own kit and two corners (39.6).
The 22 are counts of bays, two points, metres along the axis and from it, and the sides a
pattern is laid on.

## What was added

- **The roof beyond the walls** (`lib/building/lay.py` rule 17). `roof = "hip"`: the kit's hip
  piece on each corner and a run of slopes along each side between two hips. `shared.eaves`:
  the kit's roof stands that far beyond a building's outline on every side, and the `roof`
  feature has the grown region. A kit may hold a slope and its ends for each depth of building
  it roofs, and the roof's depth chooses.
- **A wall on the ground** (rule 18). `storeys = 0`: a building that is its walls alone, round
  a `yard` feature that owns no surface, each piece on the ground under it. A door in it is a
  gate: the kit's door piece in a gap, and no doorway.
- **The floor under posts** (rule 15). A storey walled by posts is tiled to the middle of its
  line of posts, and a platform's tiles come to that line from outside.
- **A court** (`lib/building/court.py`, kind `court`). A point, the gate, and `to`, a point that
  gives the axis; `yard`, `hall` and `wings` in bays and in metres along the axis and from it.
  A wing is written once and returned as two mirror images. The step says `court_meets` where
  two halls' roofs, or a roof and the yard's wall, would stand in one place.
- **`check`** holds a hip to one span square, a slope to an end of its own span, and under
  stated eaves each slope to a depth a building of the kit can have (`kit_stated`). A slope
  and a hip are a roof with no gable.
- **A stretch a fraction short of whole pieces closes** (rule 6, `fit`). An outline turned off
  the axes and rounded to the millimetre left a 6 m stretch 0.4 mm short of two 3 m pieces, and
  it was `wall_open`: six times in this court when it was turned. Not new with K8; found here.
  How short is the lattice's own rounding and no more: 1.5 mm (`FIT_SHORT`, two corners rounded
  to the millimetre). As first built it was 5 mm, and K8's review showed a yard side 4 mm short
  close with its pieces in each other; such a side is `wall_open` again.

## The cases

`expect.toml` holds 253 rows over 9 builds.

| Change | What it is | What the build says | What the build does |
|---|---|---|---|
| (base) | The court about its axis | Nothing | Four buildings from one table. The yard's 54 pieces on the ground. Three halls on 40 posts with 31 panels, floored whole. Three hipped roofs 1.35 m beyond their walls, 432 m2 covered once. Every piece has its image across the axis |
| (`by_hand.toml`) | The same court as four buildings | Nothing | The same 311 pieces |
| `deep` | The side halls three bays deep | Nothing | Their roofs are 12 m deep and take the kit's other slope and hip. Nothing is written of the roof |
| `shallow` | The side halls one bay deep | `roof_none`, of each: "the roof is 6 m deep and the kit's slope pieces are made for roofs 9 and 12 m deep (twice their spans); the roof stands 1.35 m beyond the outline on every side (`shared.eaves`). The flat roof is laid in its place" | Two flat roofs on the outlines, with no eaves; the main hall keeps its roof |
| `even` | The main hall four bays across | `params` (error), from the court step: "the hall is 4 bays across, and its door stands on its middle, where a front of 4 bays has a post: the kit's door takes a bay, and as many bays must stand on each side of it", hint "make it 3 or 5 bays across" | Stops: exit 1 |
| `close` | The side halls 4 m further in | `court_meets`, of each: "`court_w0_l`, with its eaves, stands 1 m into `court_hall` and its eaves", hint "move it 1 m or more: `from` is along the axis and `off` away from it; or move the other". The engine's `place.crowded`, twice, naming two hip pieces | All four are laid; the two roofs stand in each other |
| `turned` | The axis 16.7 degrees off the lattice | `slab_open`, of each hall: no floor tile on a turned building. `place.crowded`, 40 times, of pieces that abut by the rounding | The court keeps its lengths; every side closes and every roof is laid; each piece's image is within 4 mm of its twin |
| `short_eaves` | The kit restated with eaves of 0.9 m | `kit_stated`, of each slope: "`roof_slope_6` has a span of 6 m, which under the stated eaves of 0.9 m roofs a building 10.2 m deep: no building of the kit is that deep (whole bays of 3 m and one post of 0.3 m)". `roof_none`, of each hall, naming the depths the kit roofs | Three flat roofs on the outlines |
| `sloped` | The ground falling 5 m in 100 to the east | `base_short`, of each hall; of the west one: "the ground falls 0.75 m below the floor and the kit's base is 0.6 m deep: 13.2 m of the base stops short of the ground, by 0.15 m at most" | The yard's wall follows the land, 1.965 m from its high side to its low one; each hall stands level |

`close` is the one case where the build said something before the court existed: two
`place.crowded` records naming roof pieces. `court_meets` says the same thing of the two
buildings, in metres, with what to move.

`turned` is a known debt, not a finding of this court: bodies that meet only by the lattice's
rounding (R69 of `docs/doctrine.md`, owed since K5) are why no tile is laid and why 40 pairs of
pieces are said to overlap.

**The wings' patterns** are of two and three words since K8's review: each wing's far end reads
`doors, wall` and its back `lattice, wall, wall`, mirrored. With every pattern one word, as first
recorded, no row could tell a pattern reflected from one turned about: the base built with
`court.py`'s `mirrored` taken out passed all its rows. With these patterns that build fails
`mirror_alone` (8 pieces with no twin, where 0 is recorded).

## Pattern against archetype

| Pattern of a timber court | Archetype (`docs/ledger/archetypes.md`) | Where it lives now | Verdict |
|---|---|---|---|
| Posts on a lattice, panels between: walls that are not walls | Linear placement | lay.py rule 15, on a storey's own outline for the first time | Holds. The floor had to follow: a bay is from post middle to post middle, and so is a floor of whole tiles |
| The roof as the kit's main piece, standing beyond the walls | Grammar over Area; Asset: what a kit states once | `shared.eaves`, and the roof's own region | Holds for one depth of eaves a kit. The roof's region is the first thing a building owns outside its outline: only the court step knows what it then stands in (`court_meets`) |
| A roof made for one depth of building | Asset: a claim a piece makes alone (`params.span`) | Roof pieces chosen by span | Holds. A kit carries a slope and a hip for each depth it roofs, and says which depths those are when a building has another |
| A hipped roof | Grammar over Area: the roof is decided by the whole plan | Four hips and the slopes between, on a rectangle | Holds on a rectangle. Over an L, and in layers, it is not built |
| A veranda | Grammar: a storey whose walls are not on its building's outline | `colonnade` on one side (lay.py rule 16) | Holds, with K8's first slice unchanged but for where its tiles lie |
| A walled yard with a gate | Linear placement along an outline; not a storey | A building of no storeys, a `yard` feature with no surface | Holds as a wall. A yard is not yet a place: nothing says what is inside it, and a wall along an open line is not laid |
| A plan that is one half and its image | Grammar: a generator that returns buildings, as a row does | `court.py`: a wing written once, returned twice | Holds for halls across and along one axis. No archetype of the ledger is "symmetry": it is a property the generator gives, measured (`mirror_mm`) and not held by any rule |
| A door on the axis | Meeting | The door on the middle bay; an even front refused | Holds |
| Level halls on sloping ground, inside a wall that follows it | Meeting the ground (K7) | Rule 13 for the halls, rule 18 for the wall | Partly: each is laid as its rule says, and nothing levels the court between them |

## Still open

- **Layered roofs** (a lower roof round a storey set back above it) and **curved eaves**: not
  built. The first is a roof on a terrace; the second is the kit's own modelling.
- **A wider middle bay.** A run holds one pitch, so the bays of a side are one width (filed
  with K8's first slice).
- **The yard's ground.** The halls' doorways each raise the ground 0.6 m to their sills; the
  court between them is the land as it lies. A yard that levels its ground, and steps to a
  door (filed at K7), are not built.
- **A wall on a slope has no base.** Each piece of the yard's wall stands at the height of the
  ground under its middle, so on the `sloped` ground one end of a 3 m piece that runs down the
  slope is 7.5 cm in the ground and the other 7.5 cm over it. Nothing says so.
- **Eaves that meet outside a court.** `court_meets` is the court's; two buildings written by
  hand, or two rows, whose roofs meet are told only by the engine's `place.crowded`, in roof
  pieces, and only where two pieces' boxes overlap.
- **A wing's `bays`** (a pin on one bay) is not passed on with its place reflected; `fill` is.
- **A gable under eaves** is laid on a rectangle and tested with made-up rows; this kit has no
  gable piece and no example builds one.

## The record

Measured 2026-11-23 on macOS arm64, release `via` of the tree at 26cb33f, lib/building of the
working tree, one fresh store for the base and its seven changes and one for `by_hand.toml`,
one build each: 253 of 253 rows over 9 builds (`expect.toml`), recorded in one run and checked
in a second, both after the mends of K8's one review (245 rows before it: the wings' patterns
were one word each, and `mirror_none` was not a code). The Before table's rows were read from builds made before this slice's code, with
the pack as it then was (the door frame stated at 2.7 m, the beam 0.3 m thick); they are not
kept as builds.

## Views

`inside` (from just inside the gate, toward the main hall, at eye height), `over` (from the
south-west, 22 m up).
