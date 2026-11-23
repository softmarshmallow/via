# temple: a colonnade outside the walls

A classical temple on a podium: a room inside a ring of columns, under one gable roof, walled
from a grey kit made by code. It is the classical half of K8 of `docs/plan-kits.md`: a line of
posts that stands outside a building's walls, a roof over both, every size from one unit, and
symmetry about an axis.

**This is our own design.** The research found no developer's account of classical building: no
column orders, colonnades, pediments, proportion from one module or symmetry, only a planned
Neo-Classical kit that was folded into another and an unopened portfolio line
(`docs/research/Modular kit failures in studios.md`, section 12). So nothing here recreates a
studio's failure. The cases are what the building library said and did when the temple was first
written with what it had, and what a kit like this one gets wrong when one piece is revised.

## What is here

| File | What it is |
|---|---|
| `packs/make_kit.py`, `packs/order.toml` | The order kit, 12 pieces of one unit, 0.9 m (a column's width). The room's walls: a bay of two units, the same with a niche, a half bay, a corner post, a door frame of four, a floor tile of one. The base, one unit deep. The colonnade, tagged `storey:colonnade`: a column, which is a post and stands on the corners too, and a lintel two units long whose place repeats every three. A roof slope and half a pediment for a temple sixteen units wide. `packs/fat.toml` is the kit with its column re-exported 1.0 m wide. |
| `world.toml`, `walls.toml` | The temple on level ground: one `building` of 14.4 by 19.8 m with `colonnade = [{ sides = "all", depth = 2.7 }]`. `walls.toml` is the layer with `lay` and `check`. |
| `by_hand.toml` | The same temple written with what the library had before K8: the room as the building, and 24 columns and 24 lintels as instances. |
| `changes/` | Six changes. |
| `measure.py`, `expect.toml` | What each build is measured by, and the measured numbers with their protocol. |

```sh
python3 examples/temple/packs/make_kit.py
for p in order fat; do via pack accept examples/temple/packs/$p.toml; done
export VIA_LIB=$PWD/lib
via trust examples/temple/world.toml --yes
via build examples/temple/world.toml --out build/temple --store build/temple.store
python3 examples/temple/measure.py build/temple --world examples/temple/world.toml --check base
```

The ground is level (10 m): the kit, not the land, is the subject. The front is the south side.

## Before

The temple was first written against the tree as it stood (26cb33f and K7's third slice), before
any rule was added. The library walled a building's own outline and nothing else, so the room
was the `building` and every column and lintel was an instance, its place worked out by hand
from the column's width and the 2.7 m between two columns' middles.

| What was tried | What the build said | What the build did |
|---|---|---|
| Columns and lintels at the podium's height (`at = [x, y, 10.9]`) | `surface.none_at`, 48 errors: "writes a height at [..], and no kind surface holds that point" | Nothing: the build stops. No surface exists outside the room, so nothing can stand at the room's floor height there |
| The same on the terrain, the room's plinth set to 0 (`by_hand.toml`, kept) | `roof_none`: "the building is 9 m deep and the kit's slope piece is made for 14.4 m" | 48 instances stand where they were written. The temple has no podium. A flat roof covers the room inside its walls: 31.8% of the temple's outline is under a roof and none of it pitched. 136.1 m2 between the columns and the room has no floor |
| The kit stating its column width as `shared.module = 0.9` | `kit_stated`, four times: the bay, the niche bay, the base and the lintel each "takes 1.8 m (2.7 m) of wall where the kit states a module of 0.9 m" | Nothing changes. `module` is the bay since K6, and a kit whose sizes come from something smaller than its bay had nowhere to say so |
| The lintel, a beam at the storey's top | `kit_stated`: "`lintel` is 1.8 m high where the kit states a storey of 9 m"; `kit_pitch`: "`lintel` is 1.8 m wide at a 2.7 m pitch" | Nothing changes. Both are true of its box and wrong of the piece: it stands a storey high, and it is a bay less one column |
| Niches along the room's walls, `pattern = ["wall", "niche"]` | Nothing | The back wall, four bays, reads wall, niche, wall, niche: its two halves are not each other's image |

What the author wrote, counted by `measure.py --world` (number literals in the `building` and
`instance` tables, comments left out):

| | Buildings | Instances | Numbers |
|---|---|---|---|
| `by_hand.toml` | 1 | 48 | 136 |
| `world.toml` | 1 | 0 | 17 |

And once the colonnade was laid by the library, one more fault of the tool showed on the first
build: `door_narrow`, "0.000 m clear" at the front door, because the lintel over the doorway was
counted as standing in it. Its box starts 7.2 m above the floor.

## What was added

Four things, each written as a design note in `lib/building/README.md` before its code.

- **Posts and bays** (`lay.py` rule 15). A kit with a `role:post` piece is laid by posts: a bay
  is from one post's middle to the next, the wall piece is a panel between two posts, a corner
  is a post, and a side closes at whole bays and one post. No fillers.
- **The colonnade** (`lay.py` rule 16). `colonnade = [{ sides, depth }]`: the walls stand
  `depth` inside the outline, the outline is a `platform` at the ground floor's level, the kit's
  `storey:colonnade` pieces stand along it, and the roof covers the outline.
- **A mirrored fill** (`lay.py` rule 8). `mirror = true` reads a pattern from both ends of a
  side toward its middle.
- **A kit's unit** (`check.py` rule 11). `shared.unit`: every stated size and every wall
  piece's plan is a whole number of it, and `kit_unit` names what is not. `check` also holds a
  panel to its pitch less the post's width, a colonnade's piece to the storey by its top, and
  does not count a lintel as standing in a doorway.

One part of the first design was changed by the work. The strip between the two outlines was to
be a feature for each side, as a terrace is (`lay.py` rule 14). A colonnade turns the corner, so
the lintel beside a corner post would have stood on two strips. The platform is one feature over
the whole outline, with the room's storey over part of it at the same height, which R70 already
reads as one level.

How a walker passes from the platform to the room was found by trying, not designed: the core needs
nothing new. A second doorway whose path lies on the platform is accepted, its end meets the room's
floor with gap 0, and its start is a free end, checked for nothing. `check` walks it from the
platform, so a room behind a platform that nothing reaches is `unreached`. The base's
`unreached = 0` says only that the room is reached: K8's review took `check`'s reading of the
platform out and the row still held, since the walk then starts on the terrain. So the walk over
the platform is measured from the build's own meetings (`onto_platform` and `from_platform`,
one doorway each), and `check`'s reading of it is held by a unit test (`ReadPlatform`).

## The cases

| Change | What it is | What the build says | What the build does |
|---|---|---|---|
| (base) | The temple, its colonnade stated | Nothing | 24 columns and 24 lintels on the outline, on a podium 0.9 m high; the walls one bay inside; one gable roof over the whole outline, each point covered once; the podium's top covered edge to edge; the door 1.8 m clear between the two middle columns |
| `even` | A front of four bays: five columns, one on the axis | `params` (error): "door 0 is 5.85 m along side 0, and a post of the colonnade stands 5.85 m along it, in the doorway". Its hint says the front has no place for the door: between two posts its middle is at 1.8, 4.5, 7.2 or 9.9 m, and the wall behind, 6.3 m long, takes the 3.6 m frame only from 5.4 to 6.3 m | The build stops |
| `fat` | The column re-exported 1.0 m wide: off the unit | `kit_unit`: the column's measured length and thickness are no whole number of 0.9 m. `kit_pitch`: the lintel is 1.8 m wide and its bay leaves 1.7 m between two posts 1 m wide. `wall_open`, of each of the four sides: no whole number of bays, resize by +0.1 m. `roof_none`; `slab_open` once, of the flat roof | No colonnade is laid but the four corner columns; the gable is refused, since its ends stand a column's width in, and the flat roof laid; 70.2% of the outline is roofed. The platform keeps its tiles, which lie inside the base (with K8's first slice alone they lay inside the posts, and a wider post unlaid them: a second `slab_open`) |
| `short` | The flanks 0.5 m longer | `wall_open`, six times: the colonnade, the room's walls and the base on each flank, each with the lengths that close (the colonnade: -0.5 m or +2.2 m). `roof_none`; `slab_open`, three times | The front and the back are laid; neither flank |
| `plain` | The niches with no `mirror` | Nothing: a pattern is not a fault | Two pieces change. The back wall reads wall, niche, wall, niche; two pairs of bays are not each other's image (`mirror_off`) |
| `front` | A colonnade across the front only | `mirror_none`, of each flank: its wall, now 17.1 m, is eight bays and a half bay, so the mirrored niches have no image about its middle (said since K8's review; before it the two halves differed and nothing was said) | Six columns; the front's five lintels and one on each flank, from the corner column to the wall's own corner; the walls on the outline on the other three sides; the same roof |
| `no_colonnade` | No colonnade | Nothing | The walls on the building's own outline, as on every building before K8 |

A column on the axis is the classical reason a temple front has an even number of columns.
`even` shows the tool has no such rule and needs none: the door is refused because a post stands
in it, and the message says where the bays' middles are and where the wall behind takes the
door's frame. On an even front the two never agree, and it says that too (K8's review: it had
offered the four middles, each of which the wall behind then refused).

`fat` is one piece revised by 0.1 m. Before K8 nothing in the kit's check named it: a column is a
piece of the right role at no stated pitch.

## Pattern against archetype

| Pattern of a classical temple | Archetype (`docs/ledger/archetypes.md`) | Where it lives now | Verdict |
|---|---|---|---|
| Posts and bays: `N` bays, `N + 1` posts | Linear placement | Two runs over one stretch: a run's members stand evenly over its span, so a piece shorter than its pitch leaves the post its room (`post_walls`) | Holds, with no change to the core. A run needed no pitch of its own |
| A line of posts outside the walls, under the roof | Grammar: a storey whose walls are not on its building's outline | `colonnade`, lay.py rule 16: rule 14's moved outline on the ground storey | Holds on one storey and a rectangle |
| The platform: a floor that is neither a room's nor the ground | Representation switch (a kind surface) | `kind.platform`, one surface over the whole outline, the room's storey over part of it at one height | Holds. Two surfaces at one height are one level (R70); nothing was added to the core |
| A door through a colonnade | Meeting | Two doorways on one axis; the second starts on the platform | Holds. The second doorway's start is a free end, so nothing checks that it starts on the platform: `check` does |
| Every size from one unit | Asset: what a kit states once | `shared.unit`, `kit_unit` | Holds. `module` could not carry it: it is the bay |
| Symmetry about an axis | Grammar: a bay's piece from its place | `mirror` on a `fill` | Holds for a wall's pieces. A plan is not held symmetric: the door is on the axis because the author wrote 7.2 |
| A piece a walker passes under | Asset: a claim and a body (R69) | The lintel's box starts 7.2 m up; `check` reads a piece's foot from its box | Holds in `check`. The core's own overlap test was not asked: nothing stands under a lintel here |
| A pediment | Grammar over Area: the roof is decided by the whole plan | The gable halves of lay.py rule 10, over the colonnade's line | Holds: a pediment is the gable, on the outline in place of the walls |

## Still open

- **Steps up the podium.** The doorway holds the ground at its sill, so the land rises 0.9 m to
  the door over the doorway's 2 m, as at every door since K7. A kit has no step piece.
- **Two storeys behind a colonnade,** a colonnade on a plan that is no rectangle, and an inside
  corner in a kit laid by posts: each refused (`params`).
- **A wider middle bay.** A run holds one pitch, so the bays of a side are one width.
- **Eaves.** This kit states none, and its roof covers the outline and stops there. A kit that
  states `shared.eaves` has its roof laid beyond the outline (K8's second slice, `examples/court`).
- **A plan held symmetric.** `mirror` mirrors a wall's pieces; nothing says a door is off its
  building's axis.
- **A doorway that starts on a platform** is not known to the core as one: its start is a free end.

## The record

Measured 2026-11-23 on macOS arm64, release `via` of the tree at 26cb33f, lib/building of the
working tree, one fresh store for the base and its six changes and one for `by_hand.toml`, one
build each: 190 of 190 rows over 8 builds (`expect.toml`), recorded in one run and checked in a
second, both after the mends of K8's one review (175 rows before it). The Before table's first, third and
fourth rows were read from builds made before any of K8's code, with the pack as it then was
(`shared.module = 0.9`, no half bay, a tile of 1.8 m); they are not kept as builds.

## Views

`front` (the front from the south, at eye height), `corner` (the south-west corner from 14 m up),
`over` (from the south, 30 m up).
