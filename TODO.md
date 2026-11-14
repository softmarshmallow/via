# TODO

This list now feeds the rewrite milestones; the plan is `docs/plan.md`. Every open item names a
destination: `-> M<n>` (the milestone that closes it), `-> spec/<chapter>.md` (the M0 chapter that
owns it), `-> set aside (doctrine section 7)`, or `-> done (R<n>)` when a ruling answers it. Items
carry the context they came from and the lesson learnt, so the rewrite can use them without the
session that produced them.

Before 2026-11-13 this was the pre-kickoff list: high-level items to settle before the rewrite
started, each ending in a document or a written decision, with the spike as the only code.

## Spikes

Throwaway code under `spike/`. First passes are done; findings are in `spike/FINDINGS.md`.

- [x] Write the spike brief: questions, scope, asset rules, change scenario, exit criteria
      (`spike/README.md`).
- [x] Build the landscape spike: text source, compiler, Godot viewer, bookmark renders, walk mode.
- [x] Run the change scenario and record what broke and what the compiler caught.
- [x] Write `spike/FINDINGS.md`: the decisions the spike locks and the ones it leaves open (first pass).
- [x] Spike derived features and authority: a flood-fill lake beside an anchored one
      (see "Derived features and authority" in `spike/FINDINGS.md`).
- [x] Spike a building layout, first pass (world `spike/worlds/college`; see "A building layout"
      in `spike/FINDINGS.md`).
- [x] Spike a full interior of a single building, first pass (world `spike/worlds/station`; see
      "Interiors" in `spike/FINDINGS.md`). Original brief: a full interior of a single building. Why: no heightfield, no outdoors; rooms, doors and
      floors. Get a second opinion on whether the framework should fit this at all, or only hand off
      to an interior tool at a declared boundary (the footprint and its openings).
- [x] Spike what a heightfield cannot hold: a cave, a sea cliff with overhang, an ocean shore
      (world `spike/worlds/coast`; see "Volumes" in `spike/FINDINGS.md`).
- [x] Spike parts, first pass (the college as a seam part and the station as a portal part on
      Coast; see "Parts" in `spike/FINDINGS.md`). Open after the spike: rotation, deriving the
      ground a part needs from the part, a shared material budget, checking a part's things against
      the master's, caching a compiled part, and an unexplained defect where terrain shows through
      the station's floor.
- [x] Spike layers, first pass (the college split into five layer files with a people layer; see
      "Layers" in `spike/FINDINGS.md`). Open after the spike: namespaces per layer, which effects
      may cross layers, direction checks inside expressions, compiling or swapping one layer alone,
      and per-layer variants.
- [x] Study the player-facing map (report: `reports/Game map production pipelines.md`). Findings:
      wherever a pipeline is documented, the map is derived from the level's own data and only a
      small style layer is authored by hand; regeneration that overwrites hand edits is the
      recurring failure in shipped editors; engines ship only a camera-to-texture primitive; no
      shipped game is documented using an image model for its map; no published way exists to
      measure whether a restyled map still registers with its data; and how multi-floor interior
      maps are produced is not documented anywhere found.
- [x] Spike the player-facing map, first pass (see "The player-facing map" in `spike/FINDINGS.md`).
      Open after the spike: a registration measure that works on painted water, roads and
      footprints (edge-based, not colour-based); keeping thin features such as roads through
      restyling; relief in the input being read as features (a part's seam became a painted wall);
      repainting a single changed tile in a consistent style; several seeds per variant before
      quoting any number; per-floor maps and discovery state, which were not attempted.
- [x] Spike the reference-image workflow, first pass (worlds `stacks` and `lighthouse`; see
      "Working from reference images" in `spike/FINDINGS.md`). Original brief: the agent starts from concept art or landmark images
      (likely made by an image model), not from a text description.
      Why: a detailed image fixes composition, silhouette and palette before any map exists. Test
      whether the framework is friendly to that start, and what is rightly ours to provide: for
      example a bookmark matched to the image's viewpoint, a side-by-side of render and reference,
      and measurements that say how far the two differ.
- [ ] -> none yet (open; owner to confirm at the M0 sheet whether it joins doctrine section 7 beside row 44): Fit other map examples to the spike model (recreations of well-known game maps) and tune the model.
- [ ] -> M7: Delete `spike/`. The rewrite starts from the findings, never from the spike code.
- [x] Refine the spike as if it were the product (done 2026-11-12): `spike/SPEC.md` states the
      contract as implemented, including a section of known limitations to review before the
      rewrite; `spike/kinds/` is the one library of kinds; `spike/check.py` guards six builds.
- [ ] -> M0: Review `spike/SPEC.md` section 11 (known limitations and inconsistencies) and sort each line
      into: fix in the design, accept and document, or drop. Why: it is the most complete list of
      what the current model gets wrong, and the rewrite should start from decisions on it.
- [x] -> done (duplicate of the refine item above): (original brief, done above) Refine the spike as if it were the product: one pipeline, focused modules, shared kinds, a
      regression check (`spike/check.py`), and the contract written down as implemented. Why: so the
      rewrite can filter, review and track what exists instead of rediscovering it.

### From the city's exits (2026-11-12, `city` and `atlas`)

- -> M4: **Check an answer against the exit's facts.** Today any path answers any exit. Lesson: the facts
  are only useful if something compares them; decide which are core (width) and which a library checks.
- -> M4: **Let a parent offer, not only a part ask.** A world built apart cannot say "I need a river in
  from the north"; it can only say where its own leaves. The join contract should run both ways.
- -> M4: **Put exits on the boundary.** They stand 10 m past it to avoid the blended strip. A place on a
  part's edge should be exact by definition, not by a margin.
- -> M4: **A fade that changes surface along its length.** It needs a paint chosen by `t`, or two effects
  with crossed masks, and the output's limit of eight materials makes each one costly.
- -> M5: **Facing for every named place, not only exits.** `onward` works because an exit faces somewhere.
  Mounts and path ends still face nowhere (the same gap as challenge 48).
- -> spec/build.md (doctrine section 6: every field is exportable): **`atlas` exported no water and nothing said so.** A master with a `water` field and a viewer with
  none is silent. Lesson: a field every loader needs should not depend on a list someone forgot.

### From the city given its limits (2026-11-12, `city` and `atlas`, SPEC 12.6i)

- -> M4: **Let hand-drawn ways bend to what is given.** The city's arterials ignore `limits` and `urban`;
  they stay straight and stop dead at the edge. Lesson: once a world takes inputs, anything in it
  written as coordinates is the part that will not adapt. Arterials should be rules too.
- -> spec/graph.md: **Let the parent see the part before it gives.** The limits were drawn blind, by coordinates,
  because a given thing must exist before the part does. A part could publish a cheap outline
  (its arterials, its river) ahead of its build. This is the one-graph question again.
- -> M4: **A part whose ground gives way to the master's.** The city cannot have a waterfront: a seam
  replaces the master's ground and never the reverse. `strait`, `border` and now this all want a
  blend that reads the ground, not distance from an edge.
- -> spec/graph.md: **Rebuild a part by area.** Any change to what is given reruns every step of the part (60 s for
  the city). Lesson: a step's record covers all its inputs or none.
- -> M4: **An exit should excuse a step in the ground** the way it excuses water at the edge
  (`part.seam` fired where the river crosses the limits).
- -> M4: **Try `given` on a second kind of world** (a wood given a climate field, a village given a
  road). One use does not show it is the right mechanism.
- -> spec/documents.md: **Numbers too.** `[params]` are already a world's declared knobs with defaults. A parent giving
  a param is the same idea and is not possible yet; decide whether `given` and `params` are one table.

### From simulating the land under stated facts (2026-11-13, `basin`, SPEC 12.6k)

- -> done (R14): **Authority reaches a simulation in two forms: held, and shape.** A held cell ends water, so it
  fits a shore or an edge; a river needed a groove in the starting surface instead. Lesson: the
  rewrite's step contract should not promise "anchored means held"; each model maps facts its own
  way, and the fixed part is facts in, fields and found geometry out, facts checked after.
- -> spec/graph.md: **Let a step say what it reads by name** (these kinds, this field), not only by which layer it
  stands above. Today a harmless edit to the facts layer reruns the model.
- -> spec/graph.md: **A step's record is one slot.** Building with and without an overlay by turns reruns it every
  time. Key the record by what was read, as `given` already does for parts.
- -> spec/identity.md: **Derived geometry needs to be a first-class result.** The river's found course is handed back
  as a guide feature and views stand on it; when no water runs, everything on it fails with four
  unrelated-looking errors. Same need as generated streets.
- -> M2: **Rectilinear drainage.** At 12.5 m cells on an even tilt, streams follow the grid. Try a rougher
  starting surface, a coarser model with finer detail added after, or routing that is not D8.
- -> M2: **Use what the model knows about the river.** Width and depth from discharge, a wandering line,
  water in the tributaries. The step ignores all of it and draws a ribbon.
- -> M2: **The two-ends course is a lattice walk.** Replace with any-angle routing or smooth harder.
- -> M2: **The Rust model's defaults assume 200 m cells and uplift.** At game scale four numbers were
  found by trial. Ship presets per scale, or derive them from the cell size.
- -> M2: **The check suite now needs the Rust engine built** for `basin`. Decide whether an outside
  program in a step is allowed in the suite, or whether its output should be recorded.
- -> M6: **`atlas` should take its plain from this**: the river that bends and the land under the city
  were the reason the original library was brought up.

### From the M2 integration (2026-11-14, the five M2 worlds' library requests)

- -> M2: **Engine version in the build key.** A changed engine with the same `CARGO_PKG_VERSION`
  replays a stored build (vale kept a fixed `diag.internal` until its store was cleared). Lesson:
  the key must name the engine build, or every engine change during a milestone needs a version bump.
- -> M2: **Distance to large regions is slow.** A fill lake from marching squares has thousands
  of vertices; exact `project_region` per post made vale take ~6 min. A box bound now prunes
  segments; a grid of segments per region is the next step if it is still slow.
- -> M3: **A way that ends at a door** (relief A): `ends_at = "bothy.door"` reads the instance's
  height and loops through the trail's own effect (`graph.cycle`). Proposed: the `height` edge
  only for a read of the mount's height (`graph.md` 2, `mount` row). Not done.
- -> set aside until a world needs it (relief C): `smoothed_along` shrinking its window
  symmetrically at a path's end. It changes ford's bytes and formulas.md 8.5's vectors; the road
  kind's 1.7 m step at an end stays, and `lib/ways` documents keeping 40 m from the edge.
- -> M3: **A bookmark cannot keep its eye clear** (glade 4): writing keep_out from a bookmark loops
  through the instance it looks at. Lesson: a view's clearance is a placement fact, not a field.
- -> M2: **`facts:` nodes for every model asset of the pack**, not only those the build uses
  (graph.md 1). Which assets a build uses is known only after placement; the nodes are defined
  before. Harmless (an unused asset's node raises nothing) but each runs on a facts-file edit.
- -> M2: **A traced lint cannot see a feature that failed late.** Late feature errors (e.g.
  `identity.near_none`) are routed to `merge`, so a lint reading that feature is not skipped. Lesson:
  a late error should fail its own `shape:` node, as an early one does.
- -> M3: **Consequential errors in basin `cannot_hold`.** With the river stated uphill the step
  returns no course, and the five views standing on it said `identity.near_none` each: one fault,
  six reports. Now one `identity.near_none` at the step listing the five (identity.md 3, "said
  once, at its maker"), so two reports. Left: the generate step knows why it made no course, but
  failing it would skip the lint that names the cause. Lesson: a step needs a way to say "made
  nothing, because" that its readers' selections defer to.
- -> done (2026-11-14): **`authority.max_change` said once per build, not per feature.** The
  resolver's "once" key was code, subject and site, and every pad shares the kind's site, so a
  second pad past its cut merged into the first (vale `move_road` named the barn, not cottage_a).
  The key now holds the feature for that code and `field.effect_cut` (authority.md 2.2).
- -> M3: **Incremental builds recompute every field over the whole extent** (`fields_memo`): a vale
  change costs 4 to 8 minutes though the graph reuses most nodes. The change report's chains were
  worked out per instance (basin 30 min+); now once per producer.
- -> M2: **Measures beyond `checks.py`.** `water_*`, `trees_on_way` and `crowded_view` landed in
  lib/review (relief 2, vale 5); worlds' `expect.toml` still read their own scripts.

### From the M2 review fixes (2026-11-14)

- -> owner (doctrine): **The terrain library is a Rust program, not a `viasdk` step** (R19a/b). It
  links `via-num`, the seed derivation and the store, and computes its own noise. Recorded in
  protocol.md 8 and challenge 58 ("no private door" not met). Rule: publish `via-step` and friends
  as a door, or port the library to Python. Meanwhile a one-word configured program's file digest
  is in the step's static key (graph.md 5.1), so a revised binary re-runs basin.
- -> M3: **`uplands` traces its stream over its own cells.** `derive.descend` and `derive.spill`
  read fields of the build; a step's own land is in none while it runs. Lesson: a library that wants
  the engine's line over its returned land needs a second step above it.
- -> M3: **Clumps of different parents and scatters still interpenetrate** (rules.md 15). Members now
  give way to their parent and earlier siblings; a scatter giving way to lower layers' scatters is
  the next question, decided at eye height.
- -> M2: **`d20_basin` stays pending**: the runner configures only `python`, and basin's step is the
  `via-terrain` program. Lesson: a case that needs a configured program needs a runner key for it.
- -> M3: **A worker step's `exit` is always null** (graph.md 6.3 now says so): a pooled worker exits
  when the build ends, after the record. Lesson: per-call status belongs to the call, not the process.

### From the M2 close (2026-11-15; the disk filled mid-run, review round lost)

- -> M3: **A change still costs ~90% of a cold build** (vale move_road 49.6 s vs 54.8 s, after cap
  bounding, no per-object fsync, one placement pass). Remaining: the `checks` lint step (~21 s,
  re-runs whole on any features change), the whole-extent fields memo (~9 s; `distance:kind:road`
  re-runs in all 64 chunks though far chunks are capped constant), `pre_edit`'s fourth placement.
  Lesson: profile before guessing; the biggest cost was paid by every build.
- -> M3: **Land look not finished.** massif: new `uplands` params (`channel`, `creep_steps`) bend the
  streams, but rocks fell to 0 (weather sees about tan/√2 of a diagonal slope), expect.toml not
  re-recorded, uplands docstring lacks the two params. basin: dress/land edits unbuilt (woods
  expected ~1,000), mouth and straight course over the shelf untouched. vale after move_road: not
  started. A wrong `role:surface matches 0 fields` error hides a road end outside the extent.
- -> owner: **build.md 12 vs the null vector**: the id digests the canonical JSON without its final
  line feed; build.md should say so. **ford's pack for build.md 29/32** (stand-ins on a pack with no
  model asset) needs a ruling; `c29_flags_in_key_m2` stays pending.
- -> M3: **The on-foot review of M2 worlds was never run** (disk full). Run it with M3's first sheet.
- -> M3: **PNG field rows without cases**: 16-bit, inverted, orientation, channels, palette, interlaced
  and others in fields.md 4.

### From building quay (2026-11-15, M3 world; examples/quay)

- -> core: **A routed path vanishes silently** when its cost reads a field that reads a feature whose
  id sorts after the path's (place.rs evaluates the cost over the features placed so far). quay names
  its hill `crest` to dodge it. Then `shape:<path>` gave `diag.internal` (unstable) on a warm store:
  its key misses what the cost's field reads. Lesson: a dependency read through a field is a dependency.
- -> core: **Step diagnostics replay from the store**: `authority.unchecked` stayed after `checks` was
  added, `step.streets.none` showed beside 11 streets; a fresh store cleared both.
- [x] -> core: **`columns=["class"]` in build.features overwrote the core's `class` column** with an absent
  attribute (every lot read as class ""). Fixed in the M3 review pass: a core column name in `columns` gives
  the core column, `*` leaves such an attribute out, and a mixed list gets `class` = "" for plain features
  (protocol.md 3.3). Lesson: a reserved column name must be reserved in code, not by convention.
- -> via-geom (M3 slice 2): **A cell cut stops short near a convex vertex.** c25's 2 m bend puts a
  station 19 mm before the vertex; the cut, square to its segment and `band_depth` + 2 mm long,
  misses the far side the next segment sets, so two cells merge. Make each cut reach its own far side
  (geometry.md 8.4, Open). Lesson: "renames nothing" was written before it was built.
- -> via-geom (M3 slice 2): **Spur sides are not frontage.** 3.6 makes each spur side its own chain;
  `cells_on_area` walks the shell only, and the model's `area_front` skips spurs (geometry.md 8.2,
  Spurs: Open). A dead-end street inside a block gets no lots.
- -> core (M3 slice 2): **A made area or cell cannot write a field.** Fields are made before makers;
  a maker `kind` with effects is `doc.not_yet` (geometry.md 8.1, Open). Order the graph so they can.
- -> spec/cases (M3 slice 2): owed cases `geom.many_rings` on a maker (needs a `where` region),
  `protocol.output_table` for a returned `[network]`/`[cells]`, `of` a cells table without `class`,
  `min_length`/`min_area` merging through text, rest `kind` null; identity 27 and 28 are pending.
- -> core: **A pad in a layer above a step that reads the ground is `doc.not_yet`**: quay's houses are
  unlevelled (30 of 30 `place.relief`). This blocks a town on a slope.
- -> lib/town: **The quay reads as an earth apron on foot**; no wall face seen from eye height; main way
  and street `d…` run 9 m apart down to it. Only 30 of 178 lots hold a house (curved, shallow lots).

## Framing

- [x] Draft `docs/ledger.md` (2026-11-12): the vocabulary of a game world in 14 groups, each term with an
      owner and where the spike stands. Written from general knowledge; check engine names before
      relying on them. Keep adding to it.
- [x] State who it is for (2026-11-12): new games, independent developers first, Godot or Unreal,
      an agent writing. Studios later, not ignored in engineering. Recorded in `docs/faq.md`.
      Consequence: porting existing worlds is low priority.
- [x] State the goal in one line (2026-11-12): decide what matters; the world settles around it;
      change your mind and it settles again; you never repair a map by hand. In `spike/README.md`,
      the design pages and the FAQ. Unproven half: that the undecided rest looks good.
- [ ] -> spec/assets.md: Design for assets that arrive fast and uneven (2026-11-12). Assumption: creators generate
      models and images quickly and replace them often. Consequences: stating an asset's facts must
      be nearly free (measure, do not ask); odd pivots, facings and sizes are the normal case;
      a kit run cannot demand one exact width; a replaced asset must show what it affects.
- [ ] -> M7: Check the ledger's engine names (section 20) and attributions (section 22) against sources,
      and have the looks sections (21 to 24) reviewed by a second reader. Context: third draft,
      about 680 terms, all from general knowledge.
- [ ] -> spec/vocabulary.md: Settle the names flagged in ledger section 25: `part`, `level`, `set`, `portal` and `layer`
      collide with established words; `kind`, `pack`, `deck`, `build` are open.
- [ ] -> M7: Write the charter: what we own, what we don't, the intended use, and explicit non-promises.
      Derived from the ledger verdicts.
- [x] -> done (R27; what carries over and what retires is in docs/plan.md): Decide what carries over from the existing repo (code, ADRs, experiments) and what is archived.
- [ ] -> M4 (`county` proposed in docs/plan.md, the owner's to redirect): Pick the dogfood target: which game, which kind of map, at what scale.

## Core model

What the compiler understands.

- [ ] -> M1: Design the core IR v0 against one tiny example world. Candidate primitives, as tested by the spike:
  - [ ] -> spec/geometry.md: Frame: units, axes, extents, chunk grid, and feature-local frames
  - [ ] -> spec/fields.md: Fields: functions over the domain (height, masks, density, custom)
  - [ ] -> spec/documents.md: Features: typed points, paths and regions with attributes (POIs, roads, zones)
  - [ ] -> spec/assets.md: Instances: asset reference plus transform
  - [ ] -> spec/assets.md: Asset contract: opaque ID plus declared facts (bounds, footprint, origin, per-LOD cost)
  - [ ] -> spec/documents.md: Variants: overlays on a shared base (e.g. seasons)
- [ ] -> M4: Design the seam model: what a seam is, when the compiler blends, when it errors.
- [ ] -> spec/authority.md: Design authority: per feature, whether the world conforms to it (anchored) or it conforms to
      the world (derived), with tiers in between and a blend apron around every anchor.
      Context: a landmark lake needs an exact shape and things placed beside it; a scenery lake can
      emerge from terrain. Same vocabulary, opposite direction. Lesson: the spike ordered effects by
      declaration order and it was not a real answer (a road regraded a pad). Authority tiers replace
      order; two anchors whose aprons overlap is the case that must error or name a winner.
- [ ] -> spec/authority.md: Design nesting for authority. Context: every building on a terrace conflicted with it.
      Lesson: "the more specific anchor wins" (smaller region applied later) removed eleven false
      errors and also ordered paint. Decide whether that rule is enough or containment should be
      declared ("this stands on that").
- [ ] -> spec/fields.md: Design derived features: features computed from fields (flood fill, contour, flow line).
      Context: the spike's lake was an ellipse forced onto the terrain and needed a dam. Lesson: the
      model can turn a feature into a field but not the reverse, and anything that follows terrain
      (shores, rivers) needs the reverse.
- [ ] -> spec/rules.md: Design the producer model: how procedural generators emit the explicit core form.
- [ ] -> spec/protocol.md: Design one operator interface that built-ins and user code both use (inputs, outputs, seed,
      determinism). Context: lake shaping could be a hand polygon, an expression, or a simulation.
      Lesson: the escape hatch must enter before validation, never after, so nothing bypasses the
      seam checks.
- [x] -> done (R3): Design how hand edits survive regeneration (overrides layered on generated results, with
      provenance). Context: this is the maintenance pain the project exists to remove.
- [ ] -> spec/diagnostics.md: Design the compile pipeline and diagnostics: stages, determinism, error and warning shape.
- [x] -> done (doctrine section 4): Define the promotion rule: what a concept must prove to become first-class.
- [ ] -> M2: Define the standard vocabulary tier between core and user: named kinds and operators we ship,
      keep stable, and map in every exporter. Water bodies are the first candidate (still: lake,
      pond, sea; flowing: river, stream). Context: "does a lake exist?" Lesson: the physical form
      (a water-surface field) is core, but identity and meaning (a named body with a level, a shore,
      inlets) must be vocabulary, or gameplay, minimap and engine water systems have nothing to bind.
- [ ] -> spec/surfaces.md: Design volumes: bounded 3D fields for caves, overhangs and cliffs. Context: the coast spike
      meshed a sea cliff and cave from a density field whose base is the ground, blended back to the
      terrain at its edges. Lessons: the model extended to 3D with no new concept; but standing,
      water and placement still assume one or two surfaces per column, nothing can be placed inside,
      and whether we hand the engine triangles or the density field is undecided.

### From the boxiness measure and the town on simulated land (2026-11-13; FINDINGS, challenges 59 and 60)

- -> M3: **Give the street-form numbers a reference.** Bearing spread and junction shares have no target.
  The earlier research measured real towns; bring a few of those numbers in as ranges per pattern.
  Lesson: a measure without a reference only ranks our own builds.
- -> M3: **Dead ends tripled once districts arrived (10% to about 35%).** Grid streets are cut off where a
  ward ends. The street tool should end a street at a junction or not lay it. Found by the measure.
- -> M3: **Local streets and lots ignore the land.** Only main ways are routed. 163 of townland's 166
  warnings are buildings on slopes. Either the lot tool reads slope, or a lot levels its ground.
  Started (lib/town, M3): local streets follow contours above `steep`, houses take `ground = "pad"`.
  Not yet built on real land: needs the engine's `[network]`/`[cells]` first.
- -> M3: **lib/town has only fake-engine tests.** Build a world with it (quay) once the core parses
  `[network]` and `[cells]`; check every lot holds a house facing a street, on foot (R17). Also run
  `lib/*/test_*.py` in CI: today no workflow runs the library tests. Lesson: a step written against
  a contract is unproven until the engine answers it.
- -> M3: **lib/town streets: open shapes.** Cross streets run straight down the fall (steep on steep
  land: no zigzag or stair yet); long streets that converge in flat ground can cross; a contour that
  spirals off a summit is dropped, not ended at a junction.
- -> M6: **Put townland's authoring into `atlas`.** The city still carries hand-drawn arterials and its own
  river. It needs the parent's land and main ways handed down (`give`), and a build under a minute.
- -> spec/identity.md: **A found course cannot be adjusted except by `via`.** Same gap as hand edits for paths.
- -> spec/geometry.md: **Main ways are found in the order written,** and join only by a discount. A network search that
  places junctions is the real thing; this is the "network and faces" mechanism again (R13).
- -> M2: **Rectilinear tributaries at 12.5 m cells** are hidden in townland by not painting them.

### From walking townland (2026-11-13; SPEC 12.6m, challenges 61 to 63)

- -> done (R17): **Review every world on foot, not only from the air.** Context: townland passed from above and
  failed at eye level (streets dead at the river, a mangled bridge, a stepped shore). Lesson: each
  world needs ground bookmarks at its joints (water's edge, bridge, edge of town) before it is shown.
- -> spec/geometry.md: **A path along another feature is a shape the core lacks.** The waterfront is written as a region
  and the tool offsets the water's edge. Lesson: "the bank of X", "the kerb of Y" are intent (R13).
- -> spec/geometry.md: **A condition region keeps only its largest piece, silently.** The river's banks cut the `rows`
  district in two and half the town vanished with no message. It should warn, or keep every piece.
- -> M3: **Bridgeheads are raised by a step writing `height`.** A crossing's abutment should be a kind with
  an effect, so the raise is visible in the document and obeys authority.
- -> M3: **Bends are a jittered sine.** Good enough to stop the river reading as a canal; a real meander
  model (or the terrain engine at a finer cell) is the proper source.
- -> M3: **More buildings on slopes after the embankment (155 to 193 warnings).** The new frontage along
  the banks and the raised bridgeheads add lots on graded ground. Same fix as "lots ignore the land".
- -> M3: **35 dead ends remain, 9 of them stubs.** Five are the gates at the map's edge. The rest are
  thinned fringe streets longer than the join distance. Decide whether a fringe lane is meant.
- -> M3: **The deck's parapet is stepped boxes** and the deck is 36 m long over a 13 m river (it laps both
  bridgeheads). A deck with wing walls and a sloped parapet is asset work in `worldc/decks.py`.

## Composition

How one world is built from several documents.

- [ ] -> M4: Give every world a declared datum (what its zero is: sea level, or nothing). Context: `atlas`
      placed nine worlds whose edges ran from 0 m to 140 m; each needed a `lift` found by eye.
      Lesson: without a shared zero, joining is guesswork and water levels drift.
- [ ] -> M4 (exits: R9): Let a part's footprint be a shape, and let it declare what crosses its edge (a river out at
      this level, a road out there). Context: in `atlas` every flaw was a straight line: scarps
      400 m long, a river that stops at the edge. Lesson: a rectangle with one blend cannot say
      where a place really ends or what continues past it.
- [ ] -> M4: Decide which side yields at a join by authority, not by key. Context: `apron` bends the part,
      `outer` bends the master; `atlas` needed the second, `coast` the first. Lesson: this is the
      same question authority already answers inside one world.
- [ ] -> M4: A step that runs at a join. Context: the skirt is a mix of heights and leaves bare smooth
      ramps. Lesson: "a custom function" for blending means code handed both sides, not a falloff.
- [ ] -> M4: Make the master a layer above its parts by default, not below. Context: `border` needed
      `after = "parts"` so a ridge on the join was not blended away (7 m kept 4.1 m without it).
      Lesson: the order was an accident of the spike; one graph of layers and parts should set it.
- [ ] -> M4: Name the sides of a part's outline, and "the sides facing part X". Context: `border` writes
      `wild.edge.2`. Lesson: a number breaks when the outline gains a point.
- [ ] -> M4: Let a master name a point along a part's path, not only its ends. Context: the west walk in
      `border` starts at a bookmark because no road of the college ends on that side.
- [ ] -> M4: Let a part's own rule grow past its edge, fading. Context: growth across the join in `border`
      is a second scatter written to match the first. Lesson: the cause lives in the part; the
      master should extend it, not copy it.
- [ ] -> M4: A master feature that overrules a part's anchored ground only warns (`part.overruled`), and
      never clears the part's instances under it. Decide: error, or authority across parts.
- [ ] -> M4: `max_step` and the edge checks per side. Context: an outline with sides on the world's edge
      had to raise the limit for every side.
- [ ] -> spec/build.md: Lift the limit of eight ground materials, or make it per chunk. Context: `atlas` had eleven
      and renamed three (`ground_as`), so city asphalt is drawn as rock.
- [ ] -> M4: Filter when a fine part is sampled onto a coarse master, and say when detail is lost.
- [ ] -> M4: Find why a part's water body (the city's river) draws no water once it is in a master.
- [ ] -> M4: Decide whether "part" is one concept with two kinds of boundary, or two concepts. A piece
      blended into the terrain around it (the coast case) meets the master along a continuous seam:
      an apron where fields are blended. An interior behind a door (the station case) meets it at
      discrete portals: a footprint, openings and their heights, and nothing is blended. They share
      a frame, a seed, scoped names and building alone. A cave is both at once (a mouth, and rock
      continuous with the terrain), which argues for one concept whose boundary lists seams and
      portals. Naming to settle: other tools say stamp or patch for the first and cell, sublevel or
      interior for the second.
- [ ] -> M4: Design parts: a piece of map authored and built by itself, then placed in a master map like an
      asset (position, rotation, seed, parameters), with its seams resolved by the compiler.
      Context: two reference scenes were built as standalone worlds and merged into Coast by hand.
      Lessons: every coordinate was retyped; the coastline was welded by editing a vertex list; one
      height formula per world cannot be composed; global `look`, seed and kind names collided; and
      moving a piece changed its noise. A part needs its own frame and seed, contributions written as
      effects inside a footprint with an apron, namespaced IDs, scoped materials, and declared
      expectations at its boundary (sea here, land joining there) that the compiler checks, the same
      contract as sockets on a modular kit. Possibly the feature that matters most.
- [ ] -> spec/documents.md: Design layers: separately owned documents for one world (terrain, buildings, dressing, NPCs,
      gameplay), compiled and checked together, emitted separately. Context: an NPC is not part of
      the map, yet where it stands, patrols and sleeps is planned against the map. Lessons from the
      spike: one file already mixed rooms, furniture, hero props and lint expectations at 800 lines;
      overlays change a base and are the wrong tool for adding a concern beside it. A layer needs
      its own namespace, references to things in lower layers by ID and frame, a declared direction
      (upper layers may read lower ones, never the reverse), per-layer output the game can consume
      or replace, and per-layer variants (a season for terrain, a quest state for NPCs). Layers
      split by concern and parts split by space; a world can use both.
- [ ] -> M2: Design per-variant asset and material swaps. Context: the winter variant moved the snowline but
      left green trees and open water. Lesson: parameter overlays alone do not make a season.

## Assets and buildings

The asset contract, kits, and the building domain on top of the core.

- [ ] -> M5: Design the kit contract. Context: the college spike laid real kit bays in runs. Lessons: a
      piece's bounds are not its module, so the pack must declare pitch, pivot and which ends join;
      a run that does not close is a compile error that names the lengths that would; pieces of one
      run must be allowed to meet; corners, ends and stepped runs are untested.
- [ ] -> M5: Design passable footprints. Context: a gatehouse blocked the path through it. Lesson: solid
      or not is too coarse; gates, arcades and cloister walks are buildings you walk through.
- [ ] -> M5: Scope a building domain as a sub-kit on top of the core, as vegetation will need its own.
      Context: the interior spike reinvented storeys, spaces, doors and stairs. Lessons: use the
      established vocabulary (building-information modelling defines storey, space, wall, door,
      stair, slab) instead of inventing one; model a stair as a thing, not a hand-made connection;
      keep a non-parametric escape hatch (a room that is one authored mesh, with only its footprint,
      openings and floor heights declared); make the kitbash contract explicit (module sizes and
      sockets constrain room sizes from the start); and carry dressing down to small props on
      furniture, which needs assets to declare their surfaces.
- [ ] -> spec/surfaces.md: Decide what the core owns for buildings. Working answer from the spike: levels as frames,
      grounding on a named surface (not only terrain), the threshold seam between a door and the
      ground, and the footprint a building claims. Everything inside belongs to the building domain.
- [ ] -> spec/assets.md: Carry asset shape into placement. Context: dressing an interior. Lessons: footprints need a
      shape (round or rectangular), assets need to declare their top surface and wall-mount side,
      and "against a wall" must respect the wall's curve. Each was patched in the spike after the
      compiler reported a false or missing error.
- [ ] -> spec/identity.md: Decide how hero and interactive things are marked. Context: one painting and one book among
      a hundred dressed props are gameplay objects. Two patterns: explicit annotation in the map
      (free attributes that travel untouched), or the game queries by stable ID or tag and keeps the
      meaning itself. Working position: support both, own neither's meaning. What we must own is
      stable identity, so a hero item survives regeneration of the clutter around it, and a rule
      hook so a studio can check "every interactable is reachable" or "no hero prop is hidden
      inside generated clutter".

## Checks

Structural checks stay in the core; everything else is an opt-in rule.

- [ ] -> M3: Design the reach lint properly: opt-in, driven by declared movement profiles and stated
      expectations, never by assumed defaults. Context: a first version in the spike found the
      lighthouse and a headland cut off before being told where to look. Needs: several profiles
      (on foot, mount, glider, boat); links and barriers that add or remove connection (ladder,
      door, ferry, locked gate) with conditions for progression; negative expectations (a sealed
      zone must stay sealed); waivers that carry a reason; and honesty about what it reads (terrain
      top surface only). Lesson: visual QA is not ours, but "is this promise about the map true" is.
- [ ] -> spec/protocol.md: Design lint rules as plugins over a read-only view of the compiled world. Context: reach was
      written into the spike's compiler; a studio will have its own rules ("a barrier at a cliff
      edge must be backed by real fence objects"). Split three tiers: structural checks the compiler
      needs to be correct (seams, cycles, conflicts) stay in core; opt-in rules we ship (reach,
      budgets) use the public rule interface; user rules use the same one. Core owns the query
      surface, the diagnostic shape, rule configuration and waivers, and knows nothing of "fence" or
      "player". Rules read declared facts, never meshes, and only emit diagnostics. Consider a
      declarative form for the common pattern "every X must have a Y within d".
- [ ] -> M2: Lint idea: collision that is missing by mistake. Some assets have no collision on purpose
      (grass, small props); a large one without it is probably an error. Context: adding a walking
      body to the spike, where scattered trees and rocks had no colliders at all. Needs the asset
      contract to declare collision (none, box, mesh) so a rule can compare it with size, and a
      waiver for intended cases.
- [ ] -> M6: Decide how budgets block. Context: the compiler flagged 21 chunks over the triangle budget and
      the warning was ignored until the frame rate collapsed. Lesson: a budget that only warns gets
      skipped; consider error thresholds and naming the asset that dominates.

## Look and reference

- [ ] -> set aside (doctrine section 7): Decide the boundary for sky, clouds and backdrop. Context: asked while matching reference
      photos, where sky and distant land carry much of the picture. Working position: sky and cloud
      rendering are the engine's; time of day and sun direction are world facts we carry; and what
      lies beyond the playable edge (open sea, distant ranges, a skyline as billboards or impostors)
      is world content we should place and could derive from our own neighbouring maps, as we do the
      minimap. Stylised fantasy maps lean on that backdrop heavily.
- [ ] -> spec/vocabulary.md (doctrine section 6: `[look]` leaves the world file): Give `look` an owner. Context: sun, sky, fog, surf, water and rock tints all ended in one
      untyped table only the viewer understood. Lesson: split world facts (time of day, sea state)
      from engine hints, and type both.
- [ ] -> M5: Design shape character, not only shape position: profiles and cross-sections for cliffs,
      stacks and caves. Context: compositions matched the references in one pass; "barrel versus
      weathered stack" could only be nudged with noise amplitudes.
- [ ] -> none yet (open; in no milestone, like challenges 9 and 10, and the owner decides at the M0 sheet): Design reference matching: a bookmark solved from an image (horizon, lens, anchor points), a
      render-beside-reference sheet, and measured differences (palette, horizon, silhouette).

## Player map

- [ ] -> M6: Design the player-facing map as layers plus a separate style: what we emit (classes, masks,
      footprints, icons, labels, regions, floors, the world-to-map transform, tile fingerprints)
      and what stays the game's (discovery state, style). Context: study and spike on Coast.
- [ ] -> set aside (doctrine section 7, image-model map styling): Define a registration measure for a restyled map that works on painted water, roads and
      footprints. Context: the spike's colour-based water overlap failed on pale washes; only the
      shore distance was trustworthy. Nothing is published. Needs several seeds before any number
      is quoted.
- [ ] -> M6: Design per-floor maps and discovery state. Context: not attempted in the spike, and the
      study found no documentation of how shipped games produce multi-floor interior maps.
- [ ] -> set aside (doctrine section 7, image-model map styling): Decide how a single changed map tile is repainted in a consistent style. Context: the spike
      fingerprints 64 tiles and found one changed after a level edit, but never repainted it. This
      is the map's version of hand edits surviving regeneration.

## Surface and delivery

- [x] -> done (R1, R6, R20, R22): Decide the authoring surface: declarative format, expression sublanguage, SDK, or a mix.
      Format decided 2026-11-12: flat TOML keyed by id, see `docs/design/blackbox.md`. HCL was
      measured and was not shorter. Still open: the expression language, and confirming "commands,
      not an SDK". Lesson: measure a real excerpt before arguing syntax.
- [x] -> done (R21): State effect order explicitly. Keyed tables are unordered, and the spike relied on the order
      kinds were declared in.
- [x] -> done (R19): Decide the implementation language(s), with a written rationale if polyglot.
      Direction 2026-11-12: a Rust engine holding all logic, set up for an opt-in SDK from the
      start. Not yet confirmed as final.
- [x] -> done (R21): Decide what a step's returned field means: the result so far, replaced, or new ground that
      every lower effect is applied to again. Context: the SDK spike's weathering step read a
      height with the road already cut, and the compiler cut it again. Harmless for a road, wrong
      for anything that adds. See FINDINGS, "User code joining a build".
- [ ] -> M2: Decide how hand-placed things stay valid on generated land. Lesson: the lane on `massif` was
      drawn after looking at one seed's terrain; another seed moves the valley, not the lane.
- [ ] -> spec/fields.md: Decide which picture formats and depths a field may read (16-bit PNG works; EXR, tiles for
      large worlds and colour profiles do not), and whether a picture can pass through to the
      engine as a texture. Context: the `relief` world; see FINDINGS, "Pictures as fields".
- [ ] -> spec/assets.md: Draft the asset contract from the city study (`reports/Modular city asset contracts.md`,
      section 5): a written front and up rule, module per axis apart from bounds, a pivot rule per
      kit checked against the model, a kit table of roles and variants, connectors, surfaces and
      clearances. Lesson from studios: the kit is cheap to build and expensive to change.
- [ ] -> M2: Decide on a lock of each asset's contract facts, diffed on every compile, so a revised piece
      names every instance it affects. No studio source describes a way to do this.
- [ ] -> M5: Dogfood: the market-street district from the same report, built from a box kit then a real
      one, judged on three changes (piece remodelled, piece's module or pivot changed, street moved).
- [ ] -> M3: Decide how junctions are modelled. Context: in the `city` world two ways crossing at grade
      each grade the ground in turn, and a deck's ends are lined up with their roads by hand.
      See `docs/design/ways-and-assets.md`, "Open".
- [x] -> done (R13): Decide whether block, lot and frontage are framework concepts or stay user code. The city
      was built without them; the cost is that no check can say a building has no street.
- [x] -> done (R20, R27; `spec/` is the contract now, SPEC is record only): Merge SPEC section 12 into the sections it amends before the rewrite reads it as the contract.
- [ ] -> M2, M5 and M6, row by row (docs/challenges.md, "Rows by milestone"); row 41 set aside (doctrine section 7): Work down the "Owed" table in `docs/challenges.md`: six requests that have only talk or a
      proposal behind them. Lesson (2026-11-12): a request answered in prose leaves no evidence
      for the rewrite; build a crude demonstration first, then analyse.
- [ ] -> M3: Revisit the `city` world. Parked 2026-11-12: the point (draw one level, derive the rest in
      user code) is proven, the result still needs work. Known: boxes for buildings, empty block
      middles, no junction geometry, arterials painted across the river bed, five unlabelled
      districts. See FINDINGS, "A city that makes sense".
- [ ] -> M2: Idea (2026-11-12): own the asset contract and ship a zoo with it: query a pack by any fact
      from the command line or as data, lint the pack itself, and lay matches out as a world any
      loader can show. Context: a crude `python -m worldc.zoo` did all three in 70 lines, with no
      renderer of its own. Lesson: a pack is already a database; what is missing is the question
      mark. Corrected the same day: managing assets here means a taxonomy, not storage. Added
      `taxon` and `labels` on an asset, a tree view, and `@taxon/path` in a kit run so a world
      asks for a role and the pack answers. Still to design: who defines the tree (the user;
      we ship the mechanism and a starter), selection everywhere a list of assets is taken, and
      usage (which worlds use an asset, which assets nothing uses). Out of scope: storage,
      versioning, import and conversion.
- [ ] -> M7: Answer or disown the 19 open questions in `docs/faq.md`. Context: written 2026-11-12 from a
      newcomer's seat; a blank answer is a design question we have not faced. Keep it a list of
      questions, not of promises.
- [ ] -> by item below: The four things only doing can settle (filtered 2026-11-12). Everything else on this list
      is engineering the rewrite can own. These have no known right answer, or depend on judgement
      or on how agents and people behave:
      1. -> done (R3): What "the same thing" means after a rule changes, so a hand edit can hold on to it.
      2. -> M2 (R5 sets the default; the person judges per pack): Whether placement by rule looks good enough unattended, with real assets.
      3. -> M7: Whether an agent that is not the author of the tool can build and iterate from the
         documents alone.
      4. -> M7: Whether a creator can live with text as the only source of truth. Only real users answer
         this; 1 decides how much it hurts.
- [ ] -> by item below: From spiking the four (2026-11-12; see "Three questions only doing could settle" in
      `spike/FINDINGS.md`):
      1. -> done (R3): Decided by measurement: a hand edit is stored as intent at a place (pin, clear), never
         as a patch to a produced id. Ids exist for the change report. Lesson: no id survives a
         change of spacing, and by-name once re-attached silently to a different tree.
      2. -> spec/identity.md: Design the same for things that are not points: a path vertex, a hand-painted field, a
         pin that follows something. A lock must say what happens when its ground changes.
      3. -> spec/graph.md: One scatter reading another (`around`) is a dependency: fold into the one graph.
      4. -> done (R5); recipes in the M2 docs: Placement by cause is the default to teach, not even scatter. Put the `glade` patterns
         in the docs as recipes. No measure exists; the person judges per pack.
      5. -> spec/diagnostics.md: From the fresh agent, fix when touched: formula in a run's `count` crashes; a
         `reach.not_sealed` with no location; reach lint blind to decks; the lower compile of a
         generate step is undocumented and prints diagnostics twice; SDK `points` are every
         eighth sample and do not say so; a passing lint step is silent; `explicit` rows lack
         `tint` and `scale`; `bookmark.crowded` on non-solid things.
      6. -> M2 and M3 (libraries): Asked for and not expressible: an enclosure with a gate; a stream from a lake's
         outflow; "A to B at no more than N percent". Candidates for the domain library.
      7. -> M7: Repeat the fresh-agent test with a different model family before trusting it.
- [ ] -> by item below: From the city as intent at three strengths (2026-11-13; FINDINGS, same title; challenge 57).
  - [ ] -> M4: Main ways belong to the parent. Context: the city's arterials are its own, so `atlas`
        restates four axes by coordinates. Lesson: the anchored skeleton is handed down like the
        districts are; a fill should own nothing a parent would want to speak of.
  - [ ] -> spec/geometry.md: A network and its faces, and cells in a region, in the core. Context: streets, wards and
        lots exist only inside two tools; a clear did nothing, a pin stood inside a building, an id
        changed when another district did. Lesson: what a tool derives must be nameable by place.
  - [ ] -> spec/identity.md: `[[clear]]` and `[[pin]]` should speak to every producer, not scatters only, or say so.
  - [ ] -> spec/geometry.md: A `where` region in several pieces keeps the largest silently; overlaps go by order
        written. Both want a message or a stated rule.
  - [ ] -> spec/protocol.md: A step cannot ask for the pack; `lots.py` reads the committed file. Lesson: the SDK needs
        the pack and its taxonomy.
  - [ ] -> spec/diagnostics.md: `[[override]]` without `id` crashes (`KeyError`) instead of reporting.
  - [x] -> done (M3, assets.md 6.1): The pack has no small urban building; an old town is 22 m boxes.
        `house_narrow` (5 m front, 8 m deep, two storeys) added to the primitive pack; lib/town's
        houses step prefers it.
  - [ ] -> spec/graph.md: A part given anything rebuilds whole: `atlas` is 200 s. Partial rebuild by district.

- [ ] -> by item below: Engine gaps, triaged from the ledger reading (2026-11-12). Kept only where a real request
      hit the gap; the test was "did someone ask, or did a world fail", not "is it missing".
      **Must design, no way round them:**
      1. -> done (R25), spec/surfaces.md: One model for surfaces above and below ground. Asked five times (caves, interiors,
         bridges, ramps, per-floor maps); the spike has three unrelated mechanisms.
      2. -> spec/identity.md (R3, R4): Identity that survives a rebuild, and with it keeping a hand edit, locking a finished
         area and seeing what a change touched. Both outside readers asked this first.
      3. -> spec/assets.md: The asset contract: facing, pivot, an asset as several files, connection points. Caused
         the white buildings; generated assets make it the normal case.
      4. -> spec/geometry.md: A profile swept along a path. Walls, rims, hedges and roofs
         had nowhere to go.
      **Fix when next touched, each already cost us something:**
      clumping in scatter (asked directly); kit run end pieces, filler and spacing
      (city study); eight ground materials; whole-map
      distance (a ten-minute compile); a check on a pack by itself (would have caught the missing
      textures); a placed part cannot be turned; indoor plans and place names as map data (asked).
      Destinations: clumping -> M2 (R5); kit run end pieces, filler and spacing -> M5; eight ground
      materials -> spec/build.md; whole-map distance -> spec/fields.md; a check on a pack by itself -> M2;
      a placed part cannot be turned -> M4; indoor plans and place names as map data -> M6.
      **Watch, not yet proven needed:**
      user code inside a layer (every world so far managed at layer boundaries); tiling for very
      large worlds; lane-level road data (needed
      only once traffic is someone's problem).
      Destinations: user code inside a layer -> done (R21, R22: a step runs at a layer); tiling ->
      set aside (doctrine section 7); lane-level road data -> none yet (open): decided when traffic
      becomes someone's problem.
      **Nitpicks, deliberately not listed as work:**
      a set taking parameters, a part's water level, role selection in every producer, scripts
      reading unexported fields.
      Destination: none, as stated above.
- [ ] -> spec/formulas.md: Carry formula attributes into the design (2026-11-12). Context: the `pistol` stress test
      needed `= slide.top + hz` to place 17 parts with no coordinates. It answers derived
      dimensions and most of nested frames, and marks a formula apart from a string. Open: the
      same for positions (`at`, `path`), rotation, and whether it replaces `on` and `along`.
- [ ] -> M5 (open): Decide whether "a thing with parts" is one interface at every scale: facts, mounts, states,
      options, for a pack asset, a kit piece and a placed map alike. Proposed in
      `docs/design/ways-and-assets.md`. It would merge `part`, the asset contract, variants and
      overlays under four words. Needs mounts with a direction and a type.
- [ ] -> spec/graph.md: One dependency graph. Context: mounts, `on`, `along` and formula attributes each say "this
      depends on that" and each got its own ordering code; adding mounts broke every bookmark on a
      lane until the order was patched. Lesson: the rewrite needs one graph of references,
      resolved once, with one cycle report.
- [ ] -> M5: Give a mount a direction, a height and a type. Context: built with position only
      (2026-11-12). A path should arrive square to a door, a stair should land at its height, and
      a road should not end at a window.
- [ ] -> spec/assets.md: Make a mount the asset contract's connector. Context: three accessory files attached to
      named mounts with no change to the host. Lesson: a socket is a named place derived from the
      host, and the thing attached refers only to it.
- [ ] -> M5: Let a check know the state it is checking. Context: the measuring lint reports the pistol
      off its published size when the slide is back or accessories are on, which is true and not
      a fault.
- [ ] -> M1 (the sheet tool must not repeat it): Find why the render script hangs on a second or third build in a row.
- [ ] -> spec/geometry.md: Design frames that nest: a thing placed in a thing, turned and moved as one. Context: the
      `sidearm` stress test moved six parts by hand to draw one slide back; the same gap stops a
      placed part being turned and a set holding a set. See FINDINGS, "A hard-surface stress test".
- [x] -> done (R22): Define the lifecycle contract: the points user code can join (generate, effect, place, lint,
      export) and the plain data crossing each. A command and the SDK are two transports for it.
      Lesson: the spike cannot inform this, because every need was met by changing the core.
- [ ] -> spec/build.md: Decide compile-time export versus runtime loading.
- [ ] -> spec/build.md: Design the handoff representation: realisation classes (unique, repeated, field-driven, static),
      spatial partition, asset facts for culling and detail levels, stable identity.
      Context: the spike's first output was one flat instance list per asset and ran at 12 fps;
      grouping by chunk plus engine-side culling took it past 110. Lesson: we don't own draw calls, but
      we do own the structure that makes culling and streaming possible.
- [ ] -> spec/build.md (R27 rules Godot for the first cut and Unreal set aside; close when `spec/build.md` holds the Unreal mapping table): Design the engine target strategy: Godot and Unreal mapping, and what "first-class" means for each.
- [ ] -> spec/diagnostics.md: Design the agent loop surface: bookmarks, review renders, machine-readable diagnostics.
- [ ] -> M6: Scope the derived products: minimap, POI metadata, streaming partition, static per-chunk budgets.
- [ ] -> M6: Explain Coast's frame rate after parts were placed. Context: 22 fps in one capture, below the
      30 asked for, with about 9 million source triangles. Lesson so far: a part brings its whole
      budget with it and nothing in the master accounts for that.

## Kickoff gate

- [x] -> done (milestone one defined in docs/plan.md): Define milestone one: the smallest end-to-end slice, its example world, and its acceptance check.
