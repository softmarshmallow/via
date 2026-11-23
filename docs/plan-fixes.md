# via: the fix plan

Status: started 2026-11-24 on the owner's direction ("If you have things to fix, you fix them
first"), after the kit plan (`docs/plan-kits.md`). Nothing is pushed.

## Context

The kit plan ended with a ranked list of what is owed (`docs/design/kit-review.md`, section 6)
and with faults filed in `TODO.md` along the way. The owner's ruling on the order: fix what is
owed first, and build the town on someone else's kit after.

What counts as a fix, in the order it is done:

1. **A fault that says nothing.** A world builds wrong and no diagnostic, no check and no measure
   but an example's own script says so. These come first: every later example stands on them.
2. **A fact typed twice.** A number the kit or the model already gives, written again by the kit's
   or the world's author. Each is a place two statements can disagree, and K10's review counted
   them (80 of 292 kit numbers, 47 of 318 world numbers).
3. **A copy where one reader is owed.** The same reading written in six places, so a mend in one
   misses the others.
4. **A gap that keeps a real kit out.** Model scale, a turn nothing checks, a kit only one world
   can name.

Then the town (T1), which needs the scale from F8 and closes challenge 33.

## Milestones

| # | Milestone | Done when |
|---|---|---|
| F1 | **The engine's silences.** A condition region that keeps only its largest piece (half a town vanished, `TODO.md` `spec/geometry.md`); an unknown key on a taxon (`shared` beside `defaults` builds and states nothing); a step reading a field its layer's peers write, which is `diag.internal` where the cause is known (`spec/graph.md`, from `realm`). | Each has a conformance case that was silent or internal before and is said now, naming what to change. |
| F2 | **The library's silences.** A house standing in water (`realm` `flood`: 32 pieces, nothing said); ground over a floor at every building, after a ruling on a house that steps with its land (5 houses of `realm`, 11 of `village`); a kit param no library reads (`bay = true`); a building partly outside the extent (a traceback today); the temple's mirror measure on a half bay and a mirrored fill that starts part way along a side. | Each case is said by `lay` or `check` with its numbers; recorded rows that move are re-recorded with the reason. |
| F3 | **What the kit states, read.** The storey's height and the plinth from the kit (review item 1, 28 numbers); pitch and span from the model in every reader (item 4, 80 numbers); a door with no `at` on its side's middle; `storey = 0` dropped from examples; `check` reads the kits off the features it judges; the records of item 7. | The review's counts are taken again by one protocol and fall by what was removed; every example's rows hold. |
| F4 | **One kit reader in the SDK.** `viasdk` reads a kit (pieces, tags, bounds, params, the taxon's `shared` once), folds a feature's attributes, and reads the ground at points (review section 4). lib/building and lib/town use it; lib/town reads `shared`. | The six copies, five folds and six ground reads are one each; all library tests and example rows hold. |
| F5 | **A plan from the model's geometry.** An outline measured from the model's triangles at `via pack accept`, a fact beside `bounds`. `check` holds a corner's turn with it; a library says an open corner; declared footprints are no longer typed (item 5). | Kenney's corner turned a quarter wrong is said; the bevelled hip's open eaves are said by `lay`, not only by the example's script; the 48 declared footprint numbers are gone. |
| F6 | **A kit a second world can name.** A pack that takes another pack, or ids scoped by their taxon, so two kits' `base` and `door` are not one id (item 9). | `realm` builds from its three kits with no merge script, and its rows hold. |
| F7 | **Mounts from the model.** A mount read from a named node in the model where the model has one; a stated mount stays where it has none (item 8). A port's hole held to its mount. | works' 51 mount numbers fall by those its models carry; `low` (a rise restated a metre low) is said. |
| F8 | **Scale, and the ruling on challenge 33.** A scale factor in the pack, applied where the model is measured (item 6). Then the ruling: is lib/town's kit run still the town's way, now that `row.py` lays rows of kit-built houses (item 3)? | Kenney's kit stands at a person's size with no `door_narrow`; the ruling is in `docs/doctrine.md`. |
| T1 | **A town on someone else's kit.** A market street of Kenney's buildings at a person's size, by whichever way F8 rules, with the town's changes run on it: a piece remodelled, the module changed, the street moved. | Challenge 33 closes or says what is still owed; a sheet; measures with their protocol. |

## How every milestone runs

As the kit plan's (`docs/plan-kits.md`, "How every milestone runs"): the case before the mend,
a note before code, tests, one review by an agent that did not write it and one fix pass, the
record (`docs/challenges.md`, the ledger, `TODO.md`, rulings in `docs/doctrine.md`), one local
commit in the window. A sheet only where there is something to look at.

A fault is mended by making it said before making it right: where the right answer needs a
ruling, it is said first and ruled after, so that no milestone waits on a ruling.

Changed so far:

- **F1, 2026-11-24.** Of its three silences, one was already mended: a condition region keeps every
  piece since the rewrite (`spec/geometry.md` 3, R18; the case `geometry/many_rings`), and its two
  TODO entries were stale. The other two are mended: a taxon's unknown key is `doc.unknown_key`,
  and a step whose two answers differ because a feature of its own layer is routed over that
  layer's returns is `graph.read_outside` naming the feature, not `diag.internal`. Building that
  case found a fourth fault, which was not filed: a route's diagnostic was routed to the feature's
  attributes, whose key does not hold what the route reads, so a warm build after a step changed
  its returns failed "not stable". It is now the shape's. The independent review found the named cause
  could be the wrong feature (one the step never read, or of another layer): it is now only a
  feature of the step's own layer, over that layer's returns, that the step's recorded reads
  reach, `route` or `where`. It found two faults older than F1: a second build on the same store
  hid the drift (both graphs reused the step by two stored traces, and only a step that ran in the
  layer pass was compared), and `holds` had the attributes fault `route` had. Both mended, with
  cases. A taxon, or its `defaults`, that is no table is `doc.type`.
- **F2, 2026-11-24.** Built: `lay`'s `outside` (rule 20, a building past the extent, which had
  ended in a traceback), `check`'s `ground_over` (rule 13, ground over a floor off a plot, ruled
  R78: said everywhere until a kit states a piece that holds the ground back), `in_water` (rule
  14, a wall in water, whether or not the floor is dry) and `kit_param` (rule 15, a piece's param
  no step reads). The temple's `mirror_off` now counts a piece with no image, and a mirrored fill
  on a stretch that starts part way is tested (it was right). Not a silence after all: a pipe over
  uneven ground (its supports' feet are `support_off`, and a port missed in height is the
  engine's `surface.gap`, an error with the miss); what is owed there is naming the cause, which
  stays filed. Kenney's dead `bay = true` was already gone (K10's fix pass). The independent review found
  that a court levelled past the edge still raised (its plot's level was read past it: now only
  inside), that two fields tagged `water_level` silenced rule 14 (now `water_many`), a negative
  clearance in a message, a distance past the extent measured per axis, and no test of the
  steps' wiring (now a world built by the engine, `test_ground.Wiring`). The temple's measure
  counts pieces on every side, mirrored or not; its new figures are recorded with that protocol.
