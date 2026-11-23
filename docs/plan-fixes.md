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
| F3b | **Pitch and span from the model.** Split from F3 on 2026-11-24 (below): every reader defaults a piece's pitch and span to its measured box (a panel between posts: its box and the post), and the packs drop the restatements (review item 4, 80 numbers). | The review's pack counts fall by what was removed; every example's rows hold. |
| F4 | **One kit reader in the SDK.** `viasdk` reads a kit (pieces, tags, bounds, params, the taxon's `shared` once), folds a feature's attributes, and reads the ground at points (review section 4). lib/building and lib/town use it; lib/town reads `shared`. | The six copies, five folds and six ground reads are one each; all library tests and example rows hold. |
| F5 | **A plan from the model's geometry.** An outline measured from the model's triangles at `via pack accept`, a fact beside `bounds`. `check` holds a corner's turn with it; a library says an open corner; declared footprints are no longer typed (item 5). | Kenney's corner turned a quarter wrong is said; the bevelled hip's open eaves are said by `lay`, not only by the example's script; the 48 declared footprint numbers are gone. |
| F5b | **What the plan is for.** `check` holds a corner's turn by its measured plan; `lay` says a roof's corner left open over a bevel. Split from F5 on 2026-11-24. | Kenney's corner turned a quarter wrong is said; the bevelled hip's open eaves are said by `lay`. |
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
- **F3 split, 2026-11-24.** Pitch and span from the model is its own slice, F3b: it changes how
  a kit laid by posts is read and touches most packs, so it gets its own review and commit.
  F3 is the storey and the plinth from the kit, a door on its side's middle, `storey = 0` left
  out, `check` judging every kit a building names, and the records of review item 7 (two of
  which, the works asset count and the kit contract's stale paragraph, were already mended in
  K10's fix pass; challenge 32's summary row is corrected).
- **F3, 2026-11-24.** Built as the split says. In the examples it took out 36 `storey_h`, 12
  `plinth`, 27 `storey = 0`, 9 door `at` and realm's four restated `kits`; every built row of the
  eight kit examples held, and only their written counts fell (realm 141 to 133, temple 17 to 13,
  court 22 to 20, borrowed 67 to 60, block 52 to 48). The independent review compared 63 change
  builds with HEAD and found every one the same; it found a misspelt kit laid with no walls and
  no word, now `params` (older, and worse once the kit gives the heights), `shared.storey = []`
  refused where it had been no storey (put back), untested paths (now tested) and stale wording.
- **F3b, 2026-11-24.** Every reader takes a pitch and a span the pack leaves out from the model:
  lay.py (a panel's and a door's between posts with the post), court.py's bay between posts,
  check.py's span (a slope that states none is no longer `kit_deck`). Of 249 pitches and 54 spans
  the eight kit examples' packs stated, 230 and 49 equalled the box; 76 were taken out of the seven
  main packs (variants regenerated, facts re-accepted). Kept, as claims a case tests: the plain
  kit's bay and window (`module`, `wide_window`), Kenney's window (`revised`), the works pipe
  (`short_pipe`), the temple's lintel (`fat`), the village valley's 3.3 m span. The first rebuild
  found `check` still reading a missing pitch as the box alone and a missing span as none
  (a timber floor tile said to fit no bay, the eaves rule judging nothing): both read as `lay`
  does now.
  The independent review found my own fault: accepting every pack had overwritten the plain
  kit's variant facts (`pivot`, `stub`, `tall`, `module`, `wide_window`), which `make_kit.py`
  writes as copies of the plain kit's facts from before each re-export, so the three cases lost
  their `asset.revised`. Restored by running the generator; those packs are never accepted. It
  also found a posts bay's variant refused when only one of the two stated its pitch (the post
  is now added before a variant is matched, in `lay` and in `check`), a door of another storey's
  group given the post's width (now only the post's own group), and a hip's message wording.
- **F4, 2026-11-24.** `viasdk.kit`, `viasdk.attrs` and `viasdk.ground` (also `step.kit`,
  `FeatureList.attrs`, `step.ground`), built on the step's own reads so the libraries' test fakes
  serve. lib/building's five steps and lib/town's kit run read through them; the six `_json`
  copies are gone. Not met of the row: lib/town reads no `shared` (reading what it lays by none
  of cost the run its reuse whenever a kit's statement changed, F4's review: read when it lays
  by it), the SDK gives `shared` per piece and each library picks the first, and two field reads
  at points stay each library's own (below). Not done: ground read at points for a field other than the surface (check.py's
  `read_along` takes any selector; lib/town's `_field` folds several fields by `max`), each its
  own and left so. The engine carries the SDK, so its binary changed: the conformance suite runs
  again.
  The review found the town run re-running for a `shared` it did not use (taken out), the
  `head` view not asked of the surface field (a step that writes the surface could not use
  `ground`), `kit` taking a string column letter by letter, `class` counted as an attribute: all
  mended, with tests of the methods.
- **F5, 2026-11-24.** A measured fact `plan` (spec/assets.md 3.4): the cells of a lattice the
  model's triangles cover, traced into rings, written only where it is not the box's rectangle.
  Measured only for an asset that asks (`plan = true`, also in a taxon's `defaults`, or
  `footprint = "plan"`): as first built every model measured it, and the shipped starter pack's
  trees would have written canopy outlines of hundreds of points no one reads into every facts
  file, and paid the tracing on every build. `footprint = "plan"` is the plan's outer ring; the
  kits' four typed L footprints (48 numbers) are now that, each equal to what was typed (13
  asset entries in 10 packs with the generated variants and realm's merged pack).
  `pack.query` gives the column `plan`. What it is for, a corner's turn held and a roof's open
  corner said, is F5b: it needs `lay`'s corner convention read first.
  The independent review found formatting and clippy failing (mine), a traced ring that touches
  itself at a corner and a plan of two islands both taken as a footprint (now
  `asset.bad_footprint`: declare it as points), tracing time growing with islands times edges
  (each start once now: 8 192 islands in 0.02 s), every model reading its indices even unasked
  (and newly refusable: only where asked now), `footprint = "plan"` on a cylinder giving its box
  (its own round one now), `plan` unchecked on a primitive, a misspelt word told to reverse its
  points, and the outward rounding by up to a cell unstated (3.4 says it). All mended, with cases.
- **F5b, 2026-11-24.** `check` rule 16, `kit_turn`: an outside corner piece whose pack asks for its
  plan is held to the way `lay` sets it (its front outward from the wall after the corner, so the
  building's own corner is the piece's front right); a plan open at any other corner of its box is a
  piece turned, and the record names the turn that mends it. Kenney's corner asks for its plan;
  the new case `turned` (its `front` taken out, as exported) is said, the base is not. Not built:
  a roof's corner left open over a bevel said by `lay` (filed in TODO.md): it needs every roof
  piece's plan placed and read against the outline and the eaves.
  The independent review checked the convention in world space (every corner instance of the
  base and `turned`, outlines of both windings, an L) and every `front` read back: right. It found
  an L corner and a round corner post flagged (only corners that stand against a wall are judged
  now; a post or a piece open all round is not), and the message naming the turn that mends where
  a reader takes it for the present turn: it now says the turn the piece stands at and the `front`
  to write, from the model's own axes, which `pack.query` now answers (`front`, `up`).
- **F8 before F6 and F7, 2026-11-24.** The scale is the gate for T1, and it is medium where F6 is
  large: the manifest's `to_asset` is already a full matrix that the Godot loader applies, so a
  factor rides in it with no loader change. F6 and F7 follow F8.
- **F8, 2026-11-24.** A declared fact `scale` (spec/assets.md 2.3, 2.4): the factor from a model's
  units to metres, on the asset or in a taxon's `defaults`; the model is measured scaled, the facts
  file writes it where it is not 1, and the manifest's `to_asset` carries it, so a loader draws the
  model at its scaled size with no change of its own. Chosen over a fit to a declared height: a
  factor is one number a kit states once, where a height differs piece by piece. And the ruling on
  challenge 33, R79: a street of buildings is lib/building's `row`; lib/town's kit run is a run of
  one piece along a path. Kenney's kit at a person's size is T1's to show, in its own world: the
  borrowed example's world is drawn at the kit's quarter size, and stays the record of that.
  The independent review loaded a scaled cube in Godot (4 m and 0.3 m, normals normalised after
  the matrix) and found every unscaled pack's records byte-equal. It found a huge `scale` blamed
  on the model (`pack.bad_model`) and a tiny one measuring silently to nothing: a factor outside
  0.001 to 1000 is now `pack.range` on `scale`; and several branches untested (now cased: a
  factor of 0.5 in a pack's `[defaults]`).
