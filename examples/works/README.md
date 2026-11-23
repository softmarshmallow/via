# works: two kits, a pipe between them, and a ring of rooms

A shed and a boiler house on level ground, from two grey kits on two modules; a pipe from one to
the other 4.5 m up, carried over the road between them at 7.5 m; a spur that ends in a cap; and
a ring of six whole rooms inside the shed. It is K9 of `docs/plan-kits.md`: pipes joined by
sockets in three dimensions, a glue kit between two kits, and an interior kit.

**This is our own design.** The research found the pipe kit as the model of a kit, and pipes
kept as the glue between kits when the kit they came with was not worth it, both from one
studio; a room on another module that fails only when the layout loops back; and no account by
anyone of pipes, gantries or conveyors joined freely in three dimensions
(`docs/research/Modular kit failures in studios.md`, sections 3, 6 and 12). The ring's cases
recreate the looped room; the pipe's are what this tool said and did when a pipe was first
written with what it had.

## What is here

| File | What it is |
|---|---|
| `packs/make_kit.py`, `packs/works.toml` | Four kits in one pack. `structure/shed`: laid by posts, a bay of 6.0 m, a post, a panel, a door frame, a floor tile, and a panel with a port. `structure/boiler`: walls at a pitch of 3.0 m, with its own port panel. `service/pipe`: a straight piece of 2.0 m, a slip piece of 0.5 m, an elbow, a riser of 3.0 m, a flange, a cap, a support. `interior/room`: a room 6 m square, a hall, a hall's corner, and three pieces that are wrong on purpose. `packs/low.toml` and `packs/wrong_plug.toml` are the pack with one fact restated each; `packs/short_pipe.toml` is the pack with the straight piece's model exported 0.2 m short. |
| `world.toml`, `walls.toml` | The works: two `building` features and two `pipe` features. `walls.toml` is the layer with `lay`, `pipes` and `check`, and the six room pieces, which stand on a floor `lay` makes. |
| `changes/` | Thirteen changes. |
| `measure.py`, `expect.toml` | What each build is measured by, and the measured numbers with their protocol. |

```sh
python3 examples/works/packs/make_kit.py
for p in works low wrong_plug short_pipe; do via pack accept examples/works/packs/$p.toml; done
export VIA_LIB=$PWD/lib
via trust examples/works/world.toml --yes
via build examples/works/world.toml --out build/works --store build/works.store
python3.11 examples/works/measure.py build/works --world examples/works/world.toml --check base
```

`--check` reads `expect.toml` with `tomllib`, so it wants Python 3.11 or later (the SDK's
`sdk/python/.venv/bin/python` is one); the measures alone print with any Python 3.

The ground is level (10 m): the kits, not the land, are the subject. There is no `by_hand.toml`.
A pipe piece 4.5 m up could not be written by hand before this milestone at all (the first probe
below), and since it a pipe by hand is the step's own document copied out, which shows nothing.

## Before

**The engine**, at 2d88999, on three probe worlds (kept in the session's scratch, not in the
repository; `docs/doctrine.md` R75 and R76 are what they led to).

*A pipe at a height.* A path with its own surface 5 m over the ground, ending at a hole 5 m up a
wall piece that stands on the ground:

```
error[surface.gap] feature.line: `line.end` is at 15 m and `wall.port` at 10 m: a gap of 5000 mm
  hint: make the two sides equal: change `line.end` or `wall.port`
1 errors, 0 warnings, 1 infos
```

A mount lay at the height of what its piece stands on. No kit could say that a hole is up a
wall.

*A ring that misses.* Four rooms, each placed on the last one's way out, the third 128 mm longer:

```
0 errors, 0 warnings, 1 infos
```

and `meets.json` held no meeting. The same with a connector on every door: eight
`surface.unmet` warnings, one for each door, the joined ones and the open one alike:

```
warning[surface.unmet] instance.r1: the mount `r1.in` has connector `door` and no path end or paired mount answers it
  hint: end a path whose kind has connector `door` here: `ends_at = "r1.in"`
```

Two pieces' mounts could not meet each other. A loop that did not close was told nothing, or
told the same thing about every door.

**The library**, with the engine's two mends, when the works was first written (the builds are
not kept; each line is one try):

| Tried | Said | So |
|---|---|---|
| The port as a variant of the bay, laid in its stretch's run, with nothing to answer it | `diag.internal`, an error, three times: `surface.unmet` on `feature."lay#boiler_s0_w3_0#r0"` "was raised at the header, which has no span in this merge" | The engine could say nothing of a member of a run. First worked round in `lay` (a piece with a mount returned as its own instance, which renamed the bays after it); K9's fix pass mended the engine instead: a record about a member is its run's, and the port is a member of its run again |
| The pipe written as a path, its legs returned over it | `geom.unowned_crossing`, 14 times: the pipe with each of its legs, and legs with each other at each turn | A pipe is a point and the points it goes on through; a leg after a turn starts at the end of the leg before it |
| Each leg ending on its riser, and the riser standing on the lower leg | `graph.cycle` twice, `surface.host_missing` four times, 1032 things not built | The riser stands on the lower leg and only the upper leg names it. Nothing holds the lower leg to the riser (filed) |
| The leg's height as an attribute `z` | `formula.shadow`: a built-in name | It is `level`, as a storey's is |
| A support standing on the ground under the pipe | `place.crowded`, 11 times: each support with the pieces of the leg over it | The core reads a thing on the ground under a surface as on one level with what stands on that surface (`spec/rules.md` 8.1). A support stands on the leg's surface and its model reaches down (filed) |
| One call of the step for every pipe | One refused pipe: no pipe at all, and every port `surface.unmet` | `for_each = "kind:pipe"`: one call a pipe |
| The first room written in `world.toml`, on the shed's floor | `surface.none_at`: "no kind surface holds that point" | The floor is made by `lay`, a layer above. The rooms are written in that layer, the first by a point and a height |

## What was built

- **A mount's rise and the joint** (the engine; K9's first slice; `spec/assets.md` 2.7,
  `spec/surfaces.md` 3.3; R75, R76). A mount may be above what its piece stands on. Two pieces'
  mounts with connectors at one point are a joint: checked for height, connector and facing,
  and where one is answered by nothing the record names the nearest and the miss.
- **Pipes** (`lib/building/pipe.py`, rules 1 to 6). A pipe is a point, the points it goes on
  through, and a height at each. Each level leg is a path that owns a surface at its height,
  and the leg's pieces stand on it: straight pieces and slip pieces fitted as a wall's are, an
  elbow at a turn, a riser at a point with two heights, a flange or a cap on each end,
  supports under a leg at the height the kit's support is made for. The step reads no
  building and pairs nothing.
- **A port** (`docs/design/kit-contract.md`, "A kit that joins others"): a wall piece's mount
  with a connector and a rise, on the building's outline; `lay` returns the piece as its own
  instance.
- **A ring of rooms** needs nothing from a library: each piece is placed on the last one's way
  out, and the doors are joints.

## The cases

Each is one change against the base (`via build --change ... --against BASE`). The numbers are
`expect.toml`'s.

| Change | What it is | What is said | What is done |
|---|---|---|---|
| base | The works as written: 64 numbers in two buildings, two pipes and six room pieces | Nothing, but that one leg (the one at 7.5 m) has no support | 6 legs, 36 pipe pieces, every piece's end on the next one's (27 joins, none loose). Nine joints nothing wrote: three flanges in three ports, six doors. The upper leg starts on one riser's top and ends on the other's, each 3 m above what its riser stands on (two meetings of one leg) |
| `moved` | The boiler house 0.3 m east | `surface.unmet` of the port and of the flange, each naming the other: "300 mm away in plan and 0 mm in height" | The pipe is laid whole, 0.3 m short of the wall |
| `no_port` | The boiler house with no port bay | `surface.unmet` of the flange, naming no other | The pipe is laid to a plain wall. Nothing says so: only that its plug is answered by nothing |
| `low` | The shed's port restated 3.5 m up | `surface.gap`, an error, of each of the shed's two ports: 1000 mm, the hint naming the port's rise | The build stops. The hole in the model is still at 4.5 m: nothing reads it |
| `wrong_plug` | The flange's connector restated `steam` | `surface.connector`, an error, at each of the three ports; all six mounts `surface.unmet` | The build stops |
| `diagonal` | A leg at 45 degrees | `params`, of `main`: "the pipe turns 45 degrees at point 3: `service/pipe` has an elbow of 90 degrees". Its two ports are `surface.unmet` | `main` is not laid; the spur is. The build stops |
| `climb` | The road crossed 2 m up | `params`: "the pipe climbs 2 m at point 1: `service/pipe` has a riser of 3 m" | `main` is not laid. The build stops |
| `sloped_leg` | The spur climbing along its leg | `params`: "leg 0, from point 0 to point 1, climbs from 4.5 m to 5 m: a leg is level". The hint says a riser stands at a point between two legs in one line, that this pipe of one leg has none, and that the kit's riser makes 3 m (as first written it offered `heights[1] = [4.5, 5]`, which gives the pipe's last point two heights and is refused) | The spur is not laid. The build stops |
| `short_leg` | The spur 0.2 m longer | `pipe_open`: 6.2 m less the metre its ends take is no whole number of pieces; "make the leg 6 m or 6.5 m long" | Its flange and cap stand with 5.2 m of nothing between them; no support |
| `short_pipe` | The straight piece's model exported 0.2 m short; its pack still says 2 m | `check`'s `kit_pitch`: "`pipe` is 1.8 m wide at a 2 m pitch" (the pipe kit is under `check`'s `kits`) | The pipes are laid as before, from what the pack states. Measured from the models' boxes, 26 piece ends meet no other, each 200 mm from the next |
| `sloped` | The ground rising 2 m in 100 to the north | `surface.gap`, an error, of all three ports (149, 4 and 188 mm); `support_off` of both pipes; lay's `foot_open` of both buildings | The build stops. Each building is level at the mean of its ground and each pipe at heights above its first point: nothing fits one to the other |
| `long_room` | The ring's fourth piece 128 mm longer | `surface.unmet` of the closing join's two doors, each naming the other: "128 mm away in plan and 0 mm in height" | Five doors join. Every piece after the long one stands 128 mm on |
| `flipped` | The ring's fifth piece the handed hall | The same, 300 mm | Five doors join. Nothing says which piece is the wrong hand |
| `other_door` | The ring's second piece with a hatch for its way out | `surface.connector`, an error, where it meets the next piece's door | The build stops |

## Pattern against archetype

| Pattern of the works | Archetype (`docs/ledger/archetypes.md`) | Where it lives now | Verdict |
|---|---|---|---|
| A socket at a height | Asset: a claim a piece makes alone; Meeting | A mount's `rise` (R75), checked at every meeting of that mount | Holds. The height is the piece's; no world writes one |
| Plug and socket between two kits | Meeting | A joint: two instances' mounts, one connector, one point, facing (R76) | Holds, with nothing written. Either kit may be swapped for another whose port says the word. An explicit veto of one pair is not built |
| A glue kit | Linear placement over Grammar | `pipe.py`: legs fitted with a filler, as a wall's stretch is | Holds on level ground. The glue does not know what it joins, which is the point of it |
| A piece placed in the air | Representation switch (a kind surface) | Each leg owns a surface at its height | Holds. Nothing stands in the air: a riser's climb is the piece's own, and a support hangs from the pipe's surface because the core reads the ground under a surface as that surface's level |
| No loose ends | Grammar: what a generator returns is complete | Every end is a flange or a cap | Holds for the step's own pieces. A flange nothing answers is the core's to say |
| A ring of whole rooms that closes | Meeting | Joints at every door; the closing one found, not written | Holds. A miss is said at the closing join, in millimetres. Which piece is wrong is not said: no generator knows the ring |
| One door size | Meeting: a connector is a free word | `door` against `hatch` | Holds: refused, naming both words |
| Two modules in one world | Asset: what a kit states once | 6.0 m and 3.0 m; the pipe's slip piece takes up what is between | Holds. Nothing compares the two modules, and nothing needs to |
| A pipe over land | Meeting the ground (K7) | Heights above the ground at the pipe's first point | Does not hold off level ground: said by the ports, as an error, and by the supports |

## Still open

- **A pipe is level and straight between its points.** No sloped piece, no tee, no valve; a
  turn is the kit's elbow or it is refused.
- **Little is checked between a pipe's ends.** A pipe through a wall with no port is laid and
  nothing says so. Two legs that cross in plan at one height are the core's
  `geom.unowned_crossing` (each leg is a path that owns a surface); at two heights half a
  metre apart or more they do not meet (R71). Nothing reads a pipe piece's body against a
  wall's or another pipe's.
- **A pipe does not follow the land**, and a support is one length (`sloped`).
- **A port's hole is not held to its mount** (`low`).
- **A ring has no generator**: its pieces are written one on the last, by hand, in the layer
  that makes their floor.
- **A joint is found at one lattice point.** Pieces turned off the axes may miss by a
  millimetre and be told so (filed at K9's first slice). This example is on the axes.
- **The loader** draws a navigation link for every meeting, a pipe's joints among them (filed
  at K9's first slice).

## How it is measured

`measure.py` reads the build directory alone. The pipe is measured from the instances and each
model's measured box (`bounds` in the manifest), not from the step's plan nor from what the
pack states of a piece: each piece's open ends are worked out and matched within a millimetre
in space, so a piece exported another length shows as loose ends (`short_pipe`: as first
written the ends were worked out from `params.pitch` and `params.reach`, and a straight piece
0.2 m short still read 27 joins and none loose). A build that stopped is told from another by
its records' figures (the gap in millimetres, the turn, the climb, the leg's rise, whose
connectors differ), not by their counts alone. Joints are read from `meets.json`; the ring's miss from the
doors' places. The protocol of every number is at the head of `expect.toml`: one fresh store,
the base and then each change against it, measured 2026-11-23 in one run and checked in a
second.
