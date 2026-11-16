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
- [ ] -> M7 (owner, 2026-11-16): **Cutover cleanup of the pre-existing code.** Before `main` moves,
      go through everything from before the rewrite and decide each piece: adopt, fold in, or drop.
      What stays is a utility, an archetype or a higher-level module that via includes, never a
      second core beside it.
      - **Today:**
        - `crates/via-terrain` (erosion, flow, sediment, climate) is the program behind `lib/terrain`,
          used by `basin` and six derived cases. It is the one library step that bypasses the SDK
          (`spec/protocol.md`).
        - `crates/via-artifact` is kept only for its seed derivation.
      - **To decide:**
        - Whether `via-terrain` becomes a `lib:` step through the SDK or stays a program that the
          spec admits.
        - Whether `via-artifact`'s seed code moves into `via-num` and the crate goes.
        - Which old modules (noise, rasters, config) duplicate the new crates and should go.
        - Old docs under `docs/archive/` and any leftover old-workspace files.
      - **Lesson:** the rewrite carried the old DEM work over as one library without settling its
        shape; leave the final tree with one way in (the core plus `lib/` through the SDK).
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
  M3 slice 4: the epoch is raised to 2 for the smoothed course held to the extent (`derived.md` 3
  step 6), so stores built before it rebuild; `epoch guard` covers only the null world, so a missed
  raise is caught by nothing. Until the key names the build, raise `EPOCH` with every change that
  moves bytes.
- -> M2: **Distance to large regions is slow.** A fill lake from marching squares has thousands
  of vertices; exact `project_region` per post made vale take ~6 min. A box bound now prunes
  segments; a grid of segments per region is the next step if it is still slow.
  M3 slice 4, measured: the kernel's fast paths (nearest vertex first in `nearest`, the wide
  product over significant limbs, winding from the edges that span a row) and `distance`'s member
  pruning. Cold `via build` of the working tree's townland and vale (same lib/ and examples/ for
  both binaries), fresh store per run, release, default threads, Apple M4 Pro shared with other
  agents (load average about 9), 3 runs each, alternating: townland 93.1 / 94.3 / 93.2 s at
  cbecb9d against 27.1 / 26.9 / 26.9 s now (3.5x); vale 39.1 / 38.9 / 38.9 s against 32.6 / 32.7 /
  32.5 s (1.2x). Values unchanged by the paths: `crates/via-geom/tests/exact_fast.rs` and
  `graph.rs` `distance_tests` compare each with the plain scan.
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

- [x] -> core: **A plain step's own returns refused it** (`doc.not_yet`) once `field_inexact` took the
  worker's view rank. Fixed: a plain command's own returns are no higher layer (authority.md 4.4, case
  `step_plain_own_returns`); the resolver and the step view share one `#` rule, "a step's `#` names"
  (`Decls::step_returned`, authority.md 4.4 What counts as returned). Lesson: one gate, two callers.
- [x] -> kernel: **Quay lots 12.4 m deep**: a cross street's first station cut ran square across the
  quay's band to the 2x depth cap. Fixed: a station's cut stops at the first stretch-end bisector it
  crosses (geometry.md 3.6 step 5, case `cells_corner`); base houses 32 -> 52.
- [x] -> lib/town (town-lib pass): **quay read as detached houses in fields with 10 m brown roads.**
  Done: houses.py fills a lot's frontage with a row wall to wall (rule 2; start/end alignment for
  corner triangles), pushes each row back until clear of the street's `keep_out` (rule 4: setback
  from the street's own width, no `place.keep_out`), and stands each run on one lib/town `terrace`
  (one lib/land pad per house in a row gave 50 `place.relief` between neighbours). Streets carry a
  `footway`; the verge keeps out only carriageway and footways. Starter pack: `house_narrow`,
  `house_row`, materials `setts` and `flags`. quay: 123 houses on 74 of 135 lots (was 52).
  Lesson: a weights row's id orders it: `footway` sorted before `grass` and was painted over.
- [x] -> lib/town: **61 of quay's 135 lots stay empty**: corner triangles (bisector cuts) and wedges
  fit no 8.6 m deep house. A corner house facing both streets, or a shallower asset, is owed.
  (M5 slice 3: houses.py rules 2b and 2c; quay 72 -> 46 empty, 9 corner houses on 18 lots.)
- [ ] -> lib/town (s4 town pass): **Doors level with the footway, stepped rows.** Done: terraces at the
  footway (houses.py rule 8), stepped, aprons; quay doors within 0.3 m 10 -> 103 of 125. Left: a raised
  floor (12 rows, rule 7) has no front steps; a party-wall step reads as `place.relief` (8) because the
  step falls in one post cell. Corner lots: 60 still empty, `fallback` (rule 2b) fills none in quay; a
  corner house facing both streets is owed. Lesson: a level read from the right place (the footway, not
  the ground under the house) fixed 90 doors; the measure from the walk named it.
- [ ] -> examples/townland (s4 town pass): lib/review/form.py now counts ends on a district edge or at a
  gate as `edge_ends`, apart from `dead_end_count`; townland expect `base.dead_ends` (18) must be
  re-recorded (18 were 15 at the edge and the rest at gates or inside) and `dead_end_at_edge` read from
  `edge_ends`. Done (townland pass): `base.dead_ends` [0, 0], `base.edge_ends` [16, 16].
- [x] -> lib/town (townland pass): **The waterfront's ends are sheer and its level is one number.** Done (M3 slice 5, ways): wall and deck fade over the ends by `t` (`end_fade`), nothing past an end, `level_start`/`level_end` by name; townland starts at `old_bridge.end` (south end 64.8 to 28.5 degrees, no pit, gap 34 to 0 m).
  Its `deck` effect's wall branch (`sd < -width`, mask to `width + 10`) runs round each end, so past an
  end the ground is cut to `level - wall` in a ring: townland's south end is a 71 degree face into a
  dry pit at 3.6 m, below the river beside it (4.5 m); the north end a 0.9 m step into the land. Fade
  the deck and the wall over the ends as lib/ways' `crossing` does (`ends` from `t`), with a 1:2
  return; and give it `level_end` (or `start_at`/`end_at` by name) so it can run up to
  `old_bridge.end` at 8.5 m. A world cannot do it: an anchored ramp there is `authority.conflict`
  with the wall. Lesson: an end is a meeting too (R18); a kind that owns a line owns how it stops.
- [ ] -> examples/townland: **One old-town street ends on the embankment** (`d68`). Step 6 drops the
  rest within `spacing / 2` of `quay_road` and `high_street`; `cross = 30` gave an `authority.max_change`
  of a terrace (3.10 m against 3, at (299, 198)) on the lib/town of that hour. Re-try with the town
  pass's end-on-a-main-way rule landed. `from_east_gate` sees no town over the hill's shoulder; move
  it to the east road's crest. move_hill empties the old town (52 houses): the districts are written
  where the hill was.
- [x] -> lib/ways + lib/water (townland pass): **The crossing's deck stood under water** (56 of 192
  posts, 0.61 m; the beck's level smoothed the stand-in deck into the river, 3.1 m over its line).
  Crossing `water` effect holds the level at `deck_z - freeboard` under the deck; the beck's level
  takes the lower of its smoothed floor and the floor it cut at the post, plus 0.25 m (ford and glade
  water byte for byte unchanged, `via eval`). Lesson: a smoothing window that reads the build's ground
  reads whatever was written after it; read what you cut.
- [ ] -> lib/town: houses.py rule 7 (was 6) predicts the pad's `max_change` on a 2 m lattice so a lot beside a
  deep cut stays empty; the move_hill error it hides was `authority.max_change` 3.15 m at (109, 99.5).
  The core's `authority.gave_way` landed (M3, after slice 2; authority.md 2.2) for returned instances with
  `ground` only: houses grounded one by one can drop the prediction and `max_cut`. Terraces are
  returned features, which stay `authority.max_change`, so for them the prediction stays.
- [ ] -> core: **Giving way resolves outside the graph.** When a returned instance's `ground` kind has
  an effect with `max_change`, `prepare` (build.rs `give_way`) resolves those fields in full at the
  build view, outside the graph, once per round, and re-runs the whole Placer each round, warm builds
  included: keys stay right, but the warm-store saving is lost for such worlds (any step-returned
  instance on lib/land `pad`; quay is not one, its terraces are features). Measured on the gave_way
  case world grown to 2048 by 2048 posts (height `0.2 * x`, release, one change build after an
  unrelated edit): 0.49 s with the pad's `max_change`, 0.45 s without; it grows with the cost of the
  fields the pad writes, so measure again on a world with a returned pad on eroded land. Then move
  the decision into a graph node. Lesson: a decision that changes what is
  placed was put before the graph because placing is; it costs every build its cache.
- [ ] -> lib/town: on-foot slice-2 items still open: cross street grade limit, banks over 35 degrees
  (cause not found), s0's kink in move_hill, the gate bowl, the main way's grade onto the quay and
  its bed raising the harbour floor (needs a path `length` in formulas), `way_water` still looks
  over a grey bump. Done in the town-lib pass: setback by keep_out (the S-bend house), door aprons
  (fronts on the footway), wall face paint (stone), `wall_deck` aimed along the deck, quay `bank = 3`
  (the anchored deck fade overrode the quay row's terraces: 18 `place.relief`).

- [x] -> core: **A routed path vanishes silently** when its cost reads a field that reads a feature whose
  id sorts after the path's (place.rs evaluates the cost over the features placed so far). quay names
  its hill `crest` to dodge it. Then `shape:<path>` gave `diag.internal` (unstable) on a warm store:
  its key misses what the cost's field reads. Lesson: a dependency read through a field is a dependency.
  Fixed (core-gaps): the placer places every shape the cost reads, through fields, first (as `where`
  did); derived.md 3 step 1, case d24 (cold, warm, moved, back). quay may rename `crest` back.
- [x] -> core: **Step diagnostics replay from the store**: `authority.unchecked` stayed after `checks` was
  added, `step.streets.none` showed beside 11 streets; a fresh store cleared both. Fixed (core-gaps):
  `authority.unchecked` is said after the run from each step's value and the build's lints, never
  kept in its trace (derived.md 5, case d23). The streets one was the pad gap below: the final graph's
  answers to `streets` differed from its layer pass's (a `doc.not_yet` refusal), so it ran again and
  laid none while the world had joined the first run's streets. A worker whose value differs between
  the two passes is now `diag.internal` (graph.md 4). Lesson: two passes over one step must agree, or say so.
- [x] -> core: **`columns=["class"]` in build.features overwrote the core's `class` column** with an absent
  attribute (every lot read as class ""). Fixed in the M3 review pass: a core column name in `columns` gives
  the core column, `*` leaves such an attribute out, and a mixed list gets `class` = "" for plain features
  (protocol.md 3.3). Lesson: a reserved column name must be reserved in code, not by convention.
- [x] -> via-geom (M3 slice 2): **A cell cut stops short near a convex vertex.** c25's 2 m bend puts a
  station 19 mm before the vertex; the cut, square to its segment and `band_depth` + 2 mm long,
  misses the far side the next segment sets, so two cells merge. Fixed: a station cut runs to its own
  chain's far side (3.6 step 5); c25 now checks every cell name and no `geom.cell_merged`.
  Lesson: "renames nothing" was written before it was built.
- [x] -> via-geom (M3 slice 2): **Spur sides are not frontage.** The kernel now walks a fronting spur
  up one side and down the other (3.6 step 1; case `cells_spur`). Also fixed on the way: a loop
  edge's own junction had no cut, merging its first and last cells.
- -> core (M3 slice 2): **Pass spur sides as frontage.** via-model's `area_front` still skips spur
  half-edges (geometry.md 8.2, Spurs: Open in text); drop that skip and add a text case. A dead-end
  street inside a block gets no lots until then.
- -> via-geom: **A block narrower than `depth` gives slivers** (quay: 17 m blocks, depth 24): both far
  sides lie outside, the two frontages' cuts cross it whole, strips merge and near cuts leave 1 m
  slivers (geometry.md 3.6 step 5, Open). Owed: a far side at the middle (skeleton). Meanwhile
  lib/town keeps `depth` under half its narrowest block. Lesson: the far-side guarantee held only
  between `depth` and `2 depth` wide; nobody had tried narrower.
- [x] -> core (M3 slice 2): **A made area or cell cannot write a field.** Landed (M3, after slice 2): made
  things stand as a step's returns, outside the views of steps at their layer, so head views, steps,
  makers, build views is acyclic; each made thing that writes is a `made` node reading its table's
  `op` node (graph.md 1), it writes as a feature and counts in `kind:<k>` (`Decls::stands_returned`),
  and a step's view leaves it out of shape selectors too (`Sel::sees_shape`, stepview
  `reads_returned_shape`). Cases n12, n13, n14. Lesson: the exclusion was first written for field
  entries only; a selector's members are a second door into the same view, and the review found the
  step saw the areas through `dist('kind:block')` (n14 failed before the fix: 0 where 20).
- -> spec/cases (M3 slice 2): owed cases `geom.many_rings` on a maker (needs a `where` region),
  `protocol.output_table` for a returned `[network]`/`[cells]`, `of` a cells table without `class`,
  `min_length`/`min_area` merging through text, rest `kind` null; identity 27 and 28 are pending.
- [x] -> core: **A pad in a layer above a step that reads the ground is `doc.not_yet`**: quay's houses are
  unlevelled (30 of 30 `place.relief`). This blocks a town on a slope. Fixed for workers (core-gaps):
  a full-view read is the build's order filtered to the layer and below (authority.md 4.4, case
  `step_full_view`). A scratch quay with `ground = "pad"` on `houses` builds warm and cold with no
  `place.relief`, but one house cuts 3.05 m against lib/land's `max_cut = 3` (`authority.max_change`
  at (196, 107)): quay's call. Still owed: plain commands (handed build files, not a view) and
  weights fields in a view stay `doc.not_yet`.
- -> lib/town: **The quay reads as an earth apron on foot**; no wall face seen from eye height; main way
  and street `d…` run 9 m apart down to it. Only 30 of 178 lots hold a house (curved, shallow lots).
  Partly done (town-lib pass, 2026-11-15): a down street within `spacing / 2` of a main way is not
  laid (`d105` gone); `lot_depth` keeps the lowest contour `2 * lot_depth` from the shore (a scratch
  quay with `lot_depth = 24`: 5 houses within 30 m of the quay, was 0); houses take the largest
  asset that fits (3 `house_wide`, was 0). The apron and the wall face are not touched.
- -> core (spec/formulas.md 6.5): **A path's length has no name in a kind's formula.** lib/town's
  bed fades out over a street's ends with `min(t, 1 - t) * length`, `length` an attribute the
  streets step writes; a routed main way has none an author can write, so its ends keep a round
  cap (now a 1:2 bank, not a wall). On quay the main way's cap at `quay_head` fills the harbour
  floor in a 1:2 cone past the waterfront's reach: `field.effect_cut` at (113.5, 22.5). A built-in
  per-feature `length` (metres) would let both kinds drop the attribute. Lesson: a cap is shape, not
  style; what an effect cannot see along its path it cannot stop at.
- -> examples/quay: **Set `lot_depth = 24` and `max_relief` in quay and re-measure expect.toml.**
  The town-lib pass changed only lib/town; quay as written now gives 15 blocks, 174 lots, 10 streets,
  29 houses (expect.toml: 16, 178, 11, 30; HEAD's lib/town on today's crates already gives 181 lots).
- -> lib/town: **Street banks at 1:2.** Done (town-lib pass): beds hold the ground within a `grade`
  0.5 bank of the bed's edge, so the bank grows with the cut. quay, posts 0.5 to 14 m past the bed's
  edge inside the district: street banks max 55.2 to 42.0 degrees, p99 38.0 to 26.6, posts over 30
  degrees 3369 to 22; main way p99 33.6 to 26.7; within 20 m of the gate, posts over 30 degrees 269
  to 2 (release `via`, one build each, fresh store). The few left over 30 lie where two banks meet.
- [x] -> core: **A worker's full view held its own layer's step returns** (quay, cold store:
  `diag.internal`, `step:streets` differed between the layer pass and the final graph). `field_inexact`
  ranked a step-returned feature or footprint at its layer, not one above it, so a field the step's own
  returns write (streets' beds on `height`) was read from the build. Fixed (world pass): it uses
  `view_rank`; case `authority/step_own_returns`. Lesson: the new two-pass check found it on the first
  real world; keep such checks.
- [x] -> lib/town: **A far edge edit turned every contour street** (quay `unrelated`: 28 lots renamed
  west of 200 m). The seed line followed the whole district's mean gradient. Fixed (world pass): with
  `origin` written the fall is read within `spacing` of it; test `test_far_edge_edit_turns_no_seed_line`.
  `unrelated` now renames no lot west of 200 m. Lesson: a written origin pins a point, not a direction.
- [x] -> examples/quay (world pass): `ground = "pad"` on houses, `hill` renamed back, `lot_depth = 24`
  with the seed origin at y 98 (the quay block is 50 m deep), `min_area = 60`, convex route cost and
  `smooth`, `quay_head` where the way lands straight (t 0.42), shore paint after street paint, two wall
  views, views re-aimed, R13 changes on a lot with a house; expect.toml re-recorded, `measure.py --check`.
- -> kernel or core: **Quay-facing lots stop 12.4 m deep** between the main way and `d175` at `depth`
  24 and 30 alike (and `d35`/`d175` lots about 11 m): no house fits. Not explained; the block's shell is
  plain (108 to 198 by 46 to about 96). Find what bounds them at y 58.4.
- -> lib/town: **Thinning is global**: Douglas and Peucker over a whole contour keeps other vertices when
  its far end moves, so a lot near the middle keeps its name but changes shape and loses its house
  (quay `unrelated`: 1 house west of 200 m). Thin per stretch between junctions, or snap kept vertices.
- -> lib/town: **A contour that meets a main way running beside it ends there for good** (quay
  `move_hill`: `s0` ends at x 140, the east half has no streets). Continue past the way on the other side.
- -> examples/quay: on foot the deck still reads earth at the eye (`way_water`, `street_quay`,
  `wall_deck`) though the shore row comes after the street row; check how rows combine. The wall face
  shows only as a grey band from the water.
- -> core (derived.md 3): a route cost of the course's own grade (direction-aware), so a main way can
  take a fall in switchbacks; a slope-only cost cannot.

### From building monastery (2026-11-15, M3 world; examples/monastery)

- [x] -> monastery: **the switchback's first two legs ran on top of each other** (0.27 m at (256, 20)):
  each leg is a lattice walk, flat first. First turn to (228, 12), the place (240, 26) added;
  `road_self_near` 199 -> 13 (the (300, 44) turn, 4.8 m apart). The real fix is the any-angle course.
- -> massif, basin: **expect rows OUT at cbecb9d and now alike** (slice 4 re-check): massif `rocks`
  0 (`scatter.empty` fallen_rock) and every change's counts; basin `ground_tarn_view` 44.4,
  `ground_middle` 55.8, `mouth_moves` (1866.6, 804.2). Not from slice 4: owed a look.

- -> lib/review (reach): **`not_sealed` says no place.** A sealed pair that holds a route names only
  the `to` place; finding the way in took a throwaway walk over the build's fields (twice, as in the
  spike's LOG entry 2). Give the walked route's first post past the `from` side's piece, or the
  narrowest stretch, as `unreached` gives its stop.
- [x] -> lib/ways: **A bridgehead filled the gorge beside the deck.** `heads` held the ground within
  1:2 of the deck out to 12 m beside it, a ramp a walker took into the gorge. Now behind each end and
  a metre past the deck's edge only; townland's expect passes unchanged. Lesson: a hold meant for the
  way's ends must not read as a hold on everything near them.
- -> lib/land or a kind: **A wall eased over a few metres is walkable obliquely** along a row of posts
  nearly parallel to it (monastery's gorge: 3 m and 0.6 m eases both leaked). Sheer faces are steps
  from post to post and draw stair-stepped; a cliff or gorge kind that says "sheer" should own this.
- -> core (derived.md 3): **a path's waypoints by name.** R16 says "the places it passes, by name",
  but `path` takes coordinates; monastery's six turns are typed numbers.
- -> lib/ways: **`deck_z` is typed from `via eval`**; a moved crossing must rewrite it. Read the rims'
  ground at the ends (`under` at the ends, or a default from the ends' heights).
- -> lib/town (`enclosure`, new, stand-in): **the wall is ground**, as the deck is; a wall as an asset
  run or a surface, a gatehouse at the gate, is owed with kind surfaces (M5).
- [x] -> examples/townland measure.py: `deck_and_ends` read `bridge.get("width", 6)` at the feature's
  top level, but features.json keeps it in `attrs`; it always used 6. Now reads `attrs` (slice 4).

### From M3 slice 5 (city, 2026-11-15)

- [ ] -> M3 core: **A selector's members are read whole, per chunk.** via-model `graph.rs`
  `define_all` gives each field chunk an edge from every entry that reads a selector to every
  member of it (and `shape_members` filters and sorts every feature and made thing per call);
  graph.md 8.1 says only the made things whose windows meet. lib/town's `terrace` reads
  `inside('kind:terrace')`, so terraces are quadratic: `city` with terraces under 777 lots passed
  7.5 GB in 1.5 min; without terraces, 1,992 houses build in 154 s at 4.7 GB. Lesson: a library
  kind that reads its own kind costs the square of its count in the core.
- [ ] -> M3 core: **Lots renamed in unchanged blocks.** `city` `move_avenue` renames 212 lots (106
  with identical outlines) inside 164 blocks kept with the same name and outline (measure.py
  `lot_changed_in_kept_blocks`); `move_bridge` 16. R13 says names elsewhere stay. Suspect the
  cell key's stretch anchor and ordinal (identity.md 2.3) when a long street loses a junction.
- [ ] -> M3 core: `network::representative` scans every post of an area's bounding box against
  every outline segment; the city's 24 wards (400 m across) made it the top of the profile.
- [ ] -> M5: `city`'s interchanges are at-grade plates and its freeway lies on the ground (lib/ways
  `interchange`, `freeway`: stand-ins); ramps, levels and the crosstown viaduct need kind surfaces.
- [ ] -> M3: `city`'s grand avenue strays 39 m off its gate-to-gate line over 1.5 m lumps into the
  rows district: a path way point by name (core item 3) or a stiffer route cost.

### From M3 slice 5 (ways, 2026-11-15)

- [x] -> lib/town + lib/ways: **ends level with what they meet (R16).** Quay's main way fell 2.38 m in its
  last 3 m, crossing heads had 1:2 lips (steepest metre 0.31 to 0.49). Waterfront `ends` hold at
  `end_grade` 1:5 along arriving ways (1:2 sideways); crossing `heads` at `approach` 1:10 out to `head`.
  Measured by each world's measure.py "meetings" block; small world lib/ways/test_crossing_world.py
  fails on the old kinds. Lesson: a hold whose band grows at 1:2 from zero makes a lip, not a ramp.
- [ ] -> lib/town: quay's `d35`/`d175` still take 0.34 m a metre in their first 2 m: the deck's land fade
  (`bank` 3, a `mix`) is concave under the `ends` band. Make the land fade a hold at `end_grade`.
- [ ] -> lib/ways: the crossing's head hold is a half disc behind each end, not the arriving way's
  corridor (a kind cannot tell what arrives); on a hillside it cuts a bowl round the head (monastery).
  A per-end corridor needs the arriving way's line in the formula (core: a path end's frame).
- [ ] -> lib/ways: a way arriving below `deck_z` with a cut-only bed (lane) still sags behind the ramp
  (example `lane_west`: 0.47 m below the deck 7 m back). The lane's bed could fill near a named end.
- [ ] -> examples/townland: one `authority.squeezed` at (202, 327): the embankment's faded wall and the
  crossing's deck edge at partial masks. Fade the wall to 0 before the crossing's `width / 2 + 0.25`.
- [ ] -> lib/town: waterfront `end_fade`, `rise` are fractions of `t` (a path length has no formula name);
  metres would not change with the line's length.

### From the M3 slice 5 review on foot (2026-11-15, five worlds, combined tree)

- [ ] -> lib/town: **junction plates with 1:2 shoulders.** Where two street beds meet, the owner of the
  crossing band holds a level plate and the other way climbs its `bank` at 1:2 (quay `high_street` on
  `s0` g1 0.50 at g10 0.21; townland `east_road` off the market 3.5 m over 8 m). Let the network node
  own the meeting (R18) with the crossing's head hold (`approach` 0.1), and a named place kind own
  the market's plate. Lesson: two beds that each hold level make a step where they hand over.
- [x] -> lib/town houses.py rule 4: **grass strip before the fronts.** keep_out read between posts
  against 0 put fronts one post past its zero line (door 1.55 m behind the footway in quay, 3.05 m in
  the city). Now read at the posts inside the footprint (place.keep_out's own test) plus the outline
  below 0.5: 1.05 m and 1.55 m (median), no place.keep_out. A 0.25 m push step gave 0.87 m in quay;
  not taken (twice the readings). Lesson: a lib test of a core rule reads what the core reads.
- [ ] -> lib/town: the door still stands about 0.5 m further back than the 0.5 m fade (mount depth,
  0.5 m push steps, the 2 m city posts). Set the front from the frontage street's own edge instead.
- [x] -> lib/town streets.py step 5b: city old-town cross streets ended 0.1 to 3 m apart on a long
  street and `x2_330` was 7 m: ends now join a node within `spacing / 4`, shorter ones are not laid.
- [ ] -> lib/town lay_grid: two grid corners on the outline still miss by 1.0 m (downtown `c1`/`s2`) and
  2.3 m (rows `c2`/`s0`); snap grid ends the same way.
- [ ] -> examples/city: `downtown_at_central` (eye 3 m from a house wall in base) and `old_town_street`
  (looks across lots) face a fixed point; anchor each on a street and face along it.
- [ ] -> examples/example: a bush lands on the `at_mill` eye after move_beck; keep scatter off bookmark eyes.
- [ ] -> lib/town: monastery `approach` steps -0.25 / +0.30 m in its last metre at the `gate`: the yard's
  mean level does not meet the way's bed; the gate, a named point, should own it with an approach hold.
- [ ] -> examples/measure: the "meetings" block is the same 120 lines in four measure.py files (each
  example is one standalone file); move it to one shared module if a fifth world needs it.

### From building example (2026-11-15, M3 world; examples/example)

- -> lib/ways or core (R18): **the church path cannot reach the door.** Ending it 2.5 or 3.8 m short
  of `church.door` puts the church in the lane's `keep_out` (`place.keep_out`); `ends_at =
  "church.door"` is `doc.upward_ref` (the path is in `world`, the church in `hamlet`). It stays
  7.5 m short. Lesson: a way that ends at a door must own that meeting, not be kept off it.
- [x] -> example: **the mill stood 29 m from the beck** and stayed put when the beck moved. Now `on`
  the beck with `face = "toward"`, viewed from the far bank (slice 4); `move_beck` moves 8.

- -> owner, documents.md 6: **which layer a change file's added entities belong to** is now asked.
  `[pin.shed] by = ["orchard_trees"]` in a change file is `doc.upward_ref` (added in `world`, the
  scatter in `dress`); example's clear and pin are inline changes in `dress` until it is decided.
  Proposal: a change file may write top-level `layer = "<id>"` for what it adds.
- -> M3 core: **`run` and `set` are still `doc.not_yet`** though their milestone is M3; example's
  row of houses is written house by house and its lamps, stalls and bench are left out.
- [x] -> lib/ways: **the stand-in deck dams the beck** (done M3 slice 5: crossing `pass` holds the water to `flow` over the ground 4 m past each face; `face_pool` 0.26 to 0.01 in example) (example `under_bridge`): water above it pools
  against a grey causeway. Owed with kind surfaces; until then say so in the crossing's comment.
- -> core (documents.md 7): a `doc.change_adds_near` warning on example's `third_cottage` (`cottage_c` near
  `cottage_a`): a typo guard firing on a numbered family of names is noise; consider not warning
  when the two names differ only in a trailing letter or digit.

### From the M3 close (2026-11-15; docs/reports/m3-sheet.png, the slice 5 reports)

M3 closes on paper with these open. Detail for most is in the sections above; each line here says
where it goes. Lesson: M3 carried four core items (incremental cost, `run`/`set`, way points by
name, lot names at size) that its worlds needed and did not get; carry them first, not last.

**-> M4, core, first (M3 debts the worlds tripped on)**
- [ ] **A change costs a cold build.** quay `unrelated` ~100% of cold, `clear_one` 73%, unchanged 64%
  (core report, 3 runs, HEAD ce470fd). On the m3-sheet run the slowest change is 81% (monastery) to
  128% (quay `move_hill`) of cold; city `move_avenue` 188 s against 157 s cold. The whole-extent
  fields memo runs once per plan, three plans a build (3.7 s of quay's 10). Owed: (a) a windowed
  `Fields` resolve per chunk; (b) `answer`, `define`, `compute_step` reading only what they need;
  (c) one plan for the three strata; (d) houses and lints split per item (`for_each`).
  M4 slice 1 (cost): profiled first (release, `sample`); the memo's top was `lineref::nearest`, a
  scan per post per path. Segments now indexed by cell, exact (geometry.md 3.8 How the nearest is
  found), and `representative` prunes its estimates; same bytes, same node keys and values. quay
  (3 runs, medians, warm store, `--against` cold, new directory): cold 11.9 -> 8.3 s, unchanged
  6.1 -> 4.1 s, `unrelated` 9.4 -> 7.5 s, `clear_one` 6.5 -> 4.7 s, memo 3.9 -> 2.4 s. city (1 run):
  cold 176 -> 124 s, unchanged 63 -> 34 s, `move_avenue` 194 -> 159 s, memo 40 -> 11 s (single
  runs, mixed trees). Re-measured by the fix pass, 3 runs each, medians, interleaved, release `via`
  of HEAD 7056fc3 (a worktree, its own city) against the working tree with all of slice 1, a fresh
  store per run, unchanged and `move_avenue` `--against` cold, `totals.ms`, load 6 to 9 from
  other agents: cold 152.0 -> 102.6 s, unchanged 62.8 -> 34.1 s (41% -> 33% of cold),
  `move_avenue` 182.8 -> 100.2 s (120% -> 98%), memo 36.1 -> 9.8 s; spread under 0.6% each. Ratios
  barely moved (quay unchanged ~50%, `unrelated` ~90% of cold): every rebuild still makes the
  whole build per plan, because a worker's trace replay (`answer`) and a plain step's request key
  read `ModelPlan::build()`. The cut owed is (b): graph.md 5.1 says answering a read computes only
  what it names. Lesson: an algorithmic win lowers cold too; ratios need the structural cut.
  M4 slice 2 (cost): profiled the no-op first (release, `sample`). Every rebuild made the whole
  build because a plain lint's request (its static key) and each worker trace check (`answer`)
  called `the_build()`. Now both are memo key records under the world's leaves (graph.md 5.1
  Memo; test `an_unchanged_world_keys_a_plain_lint_from_the_memo`, fails at HEAD); the engine no
  longer clones a definition each pass for a node whose edges are unsettled, nor builds the
  reader map when nothing failed. quay (3 runs, medians, interleaved, fresh store per run,
  HEAD fa672f7 against HEAD + this patch, same lib and examples, load ~8 from other agents,
  `totals.ms`; HEAD + the cost patch alone, not the combined slice 2 tree, whose `prepare` also
  builds every import inside `leaves()` and each stratum, so re-measure on the committed tree):
  cold 7.9 -> 7.5 s, unchanged 4.1 -> 2.2 s (52% -> 29% of cold), `unrelated`
  7.4 -> 7.2 s (97%), `move_hill` 9.5 -> 9.3 s (125%). city (same protocol; the third "after"
  run fell in a load spike, its `memo_ms` 1.6x, so medians): cold 102.3 -> 95.8 s, unchanged
  34.3 -> 22.5 s (34% -> 23%), `move_avenue` 100.3 -> 94.7 s (99%). Data files byte-identical
  before and after, every run. Left, in order:
  the change report still makes the whole build (0.9 s of quay's unchanged 2.2 s); three
  `prepare` passes (`leaves`, each stratum, the final plan); store opens per reused node; and the
  far edit, which misses every memo key: a read answered from the nodes it names (graph.md Open),
  then (a) and (d). Lesson: the no-op had no node to run; its cost was all outside the nodes.
- [ ] **Selector members read whole per chunk** (`define_all`, graph.md 8.1): terraces are quadratic,
  so the city has none (190 `place.relief`, doors to 0.5 m off). Then terraces in `city`.
- [x] **R13 for lots at size** (M4 slice 1): city `move_avenue` renamed 212 lots in kept blocks,
  `move_bridge` 16; now 0 and 0. Cause: the anchor's ordinal. A source cut by a network's `within`
  outline was dropped from the junction's parts (odd count, no pair), so every outline crossing was
  `{(#clip, pass)}` and numbered round the outline (106 in the city). Roles are now read by travel
  (identity.md 2.3; case `clip_cut_role`). Lesson: an ordinal over a digest that leaves a part out
  is a counter; check what a digest drops, not only what it holds.
- [x] **`run` and `set`** (M4 slice 1, 2026-11-16): read and placed (crates/via-model/src/sets.rs),
  cases r62 to r73 pass (renumbered from r50 to r61: r50 was taken); example's row is `[run.east_row]`, with `[run.lamps]`, `[run.stalls]` and
  the well and bench as `[set.well_corner]`. Lesson: lowering to plans let grounding, checks and
  hand edits read runs and sets unchanged; the graph's nodes did not follow by themselves.
  Left (each -> M4 core):
  - [x] **`third_cottage` and `move_tarn` failed** with `diag.internal`: `footprint:east_row#r1`
    gave another value under one key. Cause: the four members' footprint nodes had the same
    statements and edges and no parameter, so one key; the store gave r0's value to r1..r3 (they
    were never computed in base). Fixed: the member's name is the key's parameter, as `made` does.
    Unit test `no_two_nodes_share_a_key` (any two nodes of one family keyed on the same parts) and
    case r74 `member_keys`, both failing before. Lesson: a node family made per item needs the
    item in its key; check plans for key clashes, not only values.
  - [ ] A run member's pad past its kind's `max_change` is `authority.max_change` (as written),
    not given way: `Decls::produced_instance` and the give-way rounds know scatters and steps only.
  - [x] A run member's pad ordered its rows at rank 0 (`Eff`, the resolver's writers and
    `shape_rank` read `instances` only; set items too). Now `Decls::placed_rank`: the host
    instance's rank, a run member's its run's; scatters give way to run members by that rank.
    east_row is in `hamlet` (rank 1); every example row still passes, so none read that order.
  - [ ] `[pin] set` still `doc.not_yet`; `on = { of = "<run>", near }` (select a member) and the
    mount of a member (rules.md 15, "a lamp on a fence post") are not written.
  - [ ] Run members are not tested against other producers (`place.crowded`); the `kitrun` fit
    lint (rules.md 10) is not written; identity.md case 33's run half (cut by the extent) owed.
  - [x] Two rows were numbered r50 in rules.md 13: the run and set cases are now r62 to r73.
  - [ ] A store keeps a lint's diagnostics across a rebuilt binary: after a fix to step subjects
    the old `diag.malformed` came back until the store was wiped (node keys hold no engine version).
- [x] **Path way points by name** (M4 slice 1; derived.md 3, cases d26 to d28): a `path` entry may
  name a place. Monastery's six turns are named points (every row as before); city's grand avenue
  passes four named places on a guide between its gates and lies on its line (39 m off before).
- [ ] Way points by name, left: an inline `{ ref, t, offset }` entry in `path` (today a point
  feature `on` it, then its name); `holds` naming a point feature (derived.md Open).
- [x] `network::representative` scans the bounding box per segment (top of the city profile).
  Pruned (M4 slice 1): an estimate below the floor of the greatest so far stops early, trying the
  last candidate's nearest segment first; equal to the unpruned search (unit test). The winding
  scan per post is untouched; not re-profiled on the city alone.
- [x] `xtask check --milestone 3`: `cases` and `explain` clear (2026-11-16). Lock (identity.md 35 to
  39) and kind-surface cases are written as `pending` manifests with no world: locks and kind
  surfaces are not implemented. Lesson: an explain's example is compiled into via-diag, so it is
  held to the deny-list: write it with neutral names.
- [ ] Plan step 6 for M3: a fresh agent builds a small variation from the documents alone. Not run.

**-> M4, library (lib/town, lib/ways, lib/review)**
- [ ] **Junction plates with 1:2 shoulders** (high; above, "review on foot"): the node owns the
  meeting with an approach hold; a market place kind owns its plate. Worst on foot in every town.
- [ ] Door ~0.5 m further back than the 0.5 m gap (quay, townland median 1.05 m behind the footway's
  back edge, city 1.55 m): set the front from the frontage street's own edge.
- [ ] `raise_max` 0.3 (default) loses the hill-street rows (quay 125 -> 102 houses on the streets
  agent's tree); 1.5 keeps them with doors up to 1.5 m above the footway and no steps. -> owner.
- [ ] Front steps for raised floors (no room in the 0.5 m before the footway; an asset or a stair).
- [ ] Streets down to a waterfront (`d*`) have no grade limit (quay `d175` 25%); `d35`, `d175` take
  0.34 m a metre in their first 2 m (the deck's concave `bank` fade).
- [ ] lay_grid corners miss by 1.0 and 2.3 m (city downtown `c1`/`s2`, rows `c2`/`s0`).
- [ ] Monastery gate lip (-0.25 / +0.30 m): the gate, a named point, owns the meeting.
- [ ] The waterfront `ends` hold reads `kind:street`: a world with a waterfront and no streets gets
  `formula.empty_selection`.
- [ ] city: 1,465 of 2,779 lots hold no house (1,314 held; no asset fits, or too steep on the hill).
- [ ] reach `not_sealed`'s new `narrowest` place is unit-tested only, not exercised in a world.
- [ ] Footway hummock numbers (quay max 0.227 m) are the streets agent's, never re-measured.

**-> M4, examples**
- [ ] townland `move_hill`: the district regions stay where the hill was, so the old town empties
  (56 houses, base 97); districts should follow the hill (a region by name round it).
- [ ] townland: the river is a written straight course of one width (row 62 regressed from the
  spike's bends and estuary); `from_east_gate` sees no town.
- [ ] city: bookmarks `downtown_at_central`, `old_town_street` anchored on a street; houses along the
  arterials outside the districts; the hand-edit views never rendered.
- [ ] example: the bush on `at_mill`'s eye; the church path 7.5 m short of the door.
- [ ] The "meetings" block in four measure.py files; the probe in make_m3_sheet.py is a fifth copy
  of the same idea. One shared module when the next world needs it.

**-> M5 (kind surfaces, buildings)**
- [x] Decks are causeways: water cannot pass under; a deck as a surface. Interchanges are level
  plates, the freeway lies on the ground, no loop ramp (row 38): ramps and levels need surfaces.
  (M5 slice 2 decks; M5 slice 3: city's lib/ways `viaduct`s and loop `ramp`s, 24 level meetings
  at gap 0, 6.0 m headroom.)
- [x] The monastery wall is ground with a gap for a gate; sheer faces (gorge, tarn) draw
  stair-stepped at 1 m posts: a cliff or wall that says "sheer" owns it. (M5 slice 2: lib/land
  `sheer_cliff`, shown in `coast`. M5 slice 3: the gorge's and the tarn head's walls are 16
  `sheer_cliff` pieces, worst terrain step left 21.15 -> 2.81 m; the wall a run of wall bays.)
- [x] Corner houses facing both streets; a shallower house for wedges (quay's 60 empty lots).
  (M5 slice 3: `house_corner` across a corner's two bisector triangles, rule 2c; `house_shallow`
  4.5 m deep tried every half metre along a wedge's frontage, rule 2b; quay base rows.)

**-> later**
- [ ] A railway (row 36) is in no v1 world.
- [ ] Monastery `no_bridge` floods the floor at the bridge site (seen, not measured).

### From M4 slice 1 (2026-11-16; the change, on-foot and gate reviews, and the fix pass)

The fix pass fixed: run members' footprint keys (example `third_cottage`, `move_tarn`), run and
set-item ranks, the duplicate r50, quay `move_hill`'s three name rows (the names fix keeps 18 more
lots), and composition.md's graph (one `placed` node per scoped name, statements without `file`,
`joincheck` nodes), brought-entry order, given feature names and attributes, R13 across imports
(9.1), R36 recording R10's amendment. Lesson: two reviews from different angles (diff, on foot)
found different blockers; the gate alone found none of them.

**-> M4 slice 2**
- [ ] `after` on a layer: still `NotYet(4)` in via-doc's schema; R36 and composition.md 6.5 want
  `doc.unknown_key` with the fixed hint (case `after_dropped`).
- [ ] Composition cases owed (composition.md Conformance, the paragraph before "Changed from the
  spike"): 16 rules with no case, each to be written able to fail.
- [ ] Case details that depend on the implementation: the line numbers in `anchored_clash` and
  `after_dropped`, the node ids in `moved` and `moved_square` (`town.step:lay/i:districts.d1`);
  settle them against the first implementation, not the other way round.
- [ ] A run member's pad past its kind's `max_change` raises `authority.max_change` instead of
  giving way: `Decls::produced_instance` and the give-way rounds know scatters and steps only.
- [ ] The store kept a step's old `diag.malformed` after the binary was rebuilt: node keys hold the
  engine version string, not the engine's code, so a warm store is not clean across two builds of
  one version. Decide: a build id in `Versions`, or say in graph.md 3.3 that a dev rebuild needs a
  fresh store.
- [ ] Undeclared exits (R9; composition.md Open): `atlas`'s river at the edge decides between
  `import.exit_undeclared` and nothing.

**-> later**
- [ ] city `look_grand_central` is `at = [700, 830]`, so after `move_avenue` the view looks 40 m
  off the avenue: put it `on` `grand_ave_line` (review on foot, PNGs in its scratch `keep/`).
- [ ] monastery `bm_turn` (`at = [292, 62]`) stands on grass beside the road: `on` `road_up` near
  `turn_2`.
- [ ] Field matching by role as well as name across imports (composition.md Open).
- [ ] The key-clash test covers one small world; run `key_clashes` over every example's plan
  (crates/via-model/src/graph.rs, test-only) when a world adds a new per-item family.

### From building strait (2026-11-16, M4 slice 2; examples/strait)

Fixed in the slice: a scoped region lost its outline, so `on = { ref = "a.isle", t }` was
`geom.many_rings`; a brought feature's name attribute kept the import's name and frame (`a.at_slip`
looked at (186, 108), A's own point, named `slip`); an import's `changes` path was read from the
process's folder, not the import's; the instance columns and `features.json` did not read
`import:<id>`, so `harbour` built on the base's store lost A's new boathouse (47 against 48 fresh).
Lesson: build every change on the base's store as well as fresh; the fresh build hid all of it.

- [ ] Brought `weights` fields: the parent's paint holds only its own materials (strait: `sand`);
  composition.md 6.2 reads each material's weight. Both islands render as sand. No longer silent:
  `import.weights_dropped` (warning, case `weights_dropped`) since the slice 2 fix pass.
- [x] `import.seam` came back stale from a warm store after an edit inside `a/world.toml` (0.649
  and 0.886 repeated to the last digit; a fresh store gave 0.727 and 0.817). The merge-key fix
  covered text edits only; a file edit stayed stale. Fixed in the slice 2 fix pass: one node
  `joincheck:imports` holds seam, place_blended and exit diagnostics and reads every node (case
  `seam_file_edited`); the per-import, per-window `joincheck` nodes of section 12 are still owed.
- [ ] The seam check counts the import's own steep ground inside the blend band as a step at the
  join (strait's road shoulders fading into the sea, 0.7): say so in 6.6, or measure only the
  part of the grade the blend made (`|w' (h_import - h_parent)|`).
- [ ] Every field chunk re-runs on a move (180 of 180 in `move_b`, also for `harbour`): the
  `brought` nodes per chunk (composition.md 12) are owed.
- [ ] `lib/ways` `slipway` is a point with no direction: a square platform with a 2:1 face, not a
  ramp into the water. A way end with a facing, or a path kind, decides it.
- [ ] strait: no step at the join (the ferry is the only crossing; R10's join layer has nothing
  to read yet); `harbour`'s moved slipway leaves `harbour_house` in the lane's keep_out.

### From M4 slice 2 (2026-11-16; the change, on-foot and gate reviews, and the fix pass)

The fix pass fixed: an import's input leaves (a field `file`, the pack's bound files) in its digest
(case `import_file_edited`); `placed:<id>` reads the place `on` names and the parent region
`footprint` names (`placed_on` now moves `spot`); a brought chunk reads what its blend reads, fields
and `let`s (`blend_reads`); the join's diagnostics on a `joincheck` node that reads the finished
build (`seam_file_edited`); a field file edited in place has the cause `file:<path>` (it hit the
change report's no-cause assert); a brought thing's chain ends at `placed:<id>`; an unfaced point of
a turned import gains `face` (`turned`); `binding_differs` over every pair of worlds, nested
included; R29 order of a brought entry by its own effect's `order`; a `footprint` naming a parent
region or another import's place is brought, a non-region is said; the place-blended post is the
nearest; `import.weights_dropped`; no `sea` in core. Lesson: every probe that found a stale build
was warm against fresh; a case per family of input (text, file, place, blend read) is the guard.

**-> M4 slice 3**
- [ ] An import's own steps never run: `import::built` finishes with no step returns and no lints
  (composition.md 3, 12). Case `import_step`, pending. Fix: the import's graph in the parent's,
  ids prefixed `<id>.` (section 12).
- [ ] No `brought:<id>/<field>/w:` nodes and one coarse `joincheck:imports` that reads every node
  (skipped when any node fails, so join diagnostics vanish behind any error); every field chunk
  re-runs on a move (strait `move_b` 180 of 180).
- [ ] A change's chain through an import (`through import`, composition.md 11) and `related` on
  scoped diagnostics are dropped.
- [ ] `file` of `[import.<id>]` resolves from the world folder, not the declaring layer's file
  (composition.md 2 says the layer's): spec edit with its reason, or code.
- [ ] A seed override (`[import] seed`) does not reach attribute formulas.
- [ ] `import.overlap` areas in doubles with convex clipping, not on the lattice (composition.md
  6.8 wants exact).
- [x] Gate failures `anchored_clash` and `join_step` (M4 slice 3, label join): brought anchored
  ground is an `Anchor` in `anchors_meet`, related the import's anchored feature at its
  `authority` line; a step reads `<import>.<field>` (`ModelPlan::scoped_values`), refused at or
  below the import's rank; a full view leaves out an import of a higher layer (new case
  `join_below`). The `join_step` case itself was wrong twice: `look.py` must sit beside the
  declaring layer file (`protocol.md` 6.1, working folder), and a returned field is `look#own`.
  Lesson: run a new case's files against the protocol before calling the engine wrong.
- [ ] `moved_square` (no `town.step:lay/i:` nodes) waits on import steps running (item above), and
  then needs `for_each = "given:<name>"` (graph.rs and steps.rs accept only `kind:`) and a
  `given:<name>` selector in the step view (`stepview.rs` `select_rows`). Shape found: the alone
  build is eager in `import::built` (`finish_from` with no step returns) inside the parent's
  `prepare`, so the import's step nodes must enter the CLI graph as `<id>.step:...` and the
  `import:<id>`/`brought` nodes read them; a nested `build::run` per import would not give the
  parent's ran set or trust (composition.md Open "Trust for an import's steps"). `import_step`'s
  `a` declares no field `tone`, so its return is `make#tone` and the parent's `tone` is not
  brought: fix that case (declare `[field.tone]` in `a`) when it is un-pended.
- [ ] county (M4 slice 3): given features of a kind with effects cannot be given. `give = { roads =
  ["north_road"] }` into quay's `[given.roads] kind = "main_street"` gave `doc.upward_ref` (given
  rank is below the rank-0 fields its kind writes), `formula.type` (`width` not a number: the
  fill kind's defaults are not applied) and `formula.outside_field` (the given path is not clipped
  to the fill's extent). Fix all three, then give county's road to the town.
- [ ] county: the town is bare in the parent: no streets or houses (import steps, above) and no
  paint (`import.weights_dropped`); a turned town notches its quay wall (`wall_face`, seam 1.61).
  `move_town` re-runs `import:town` whole (land given) and all 1280 field chunks.
- [ ] A given region, a brought region and a `footprint` region keep only their first piece's shell:
  holes and further pieces are lost (`geo_to_parent`, `bring`).
- [ ] Given features do not take the parent's authority (composition.md 8): `GivenFeature` has no
  tier.
- [ ] A given field is read by a model-only `Fields::new` over the first placement: a parent field
  returned by a step or brought from another stratum reads wrong, and each give costs a resolve.
  Model fields only until it reads the graph.
- [ ] R29 among brought entries: the order is now the entry's own effect's `order`, but nesting is
  not compared and the tier's list is topological, so the position is approximate
  (`resolve::merge_brought`); a unit test with two effects of different `order` around one import.
- [ ] 6.7's named places miss name attributes, `follows` and `within` (documents.md 5.1).
- [ ] `footprint_ref` checks no point of the feature that `follows` `a.footprint`; `change_scoped`
  guards a pre-M4 rule (note it in the row).
- [ ] Cost numbers of slice 2 were HEAD + the cost patch alone; re-measure quay and city on the
  committed tree (its `prepare` builds every import in `leaves()` and each stratum).

**-> later**
- [ ] Incremental builds save little (review on foot): changes 80 to 123% of cold on quay,
  townland, example, city; example `winter` 90%; city unchanged 25% and peaks above cold (6.4 vs
  5.3 GB). Profile unchanged and `winter` first (`distance`, `write:changes.json` 13 s).
- [ ] townland change runs once took 4 to 35 times cold (1041 s wall, 60 s CPU) under load 9 to 18;
  a second fresh store did not repeat it. Re-time on a quiet machine; if it repeats, record what the
  build waits on. M4 slice 3 re-time (release, fresh store, 1 run each, load 10 to 15, `/usr/bin/time -l`):
  cold 22.7 s wall / 34.9 s CPU, `move_crossing` 23.3 s / 35.0 s, 0 swaps, 0 page faults, 0 block
  I/O: not repeated. The engine's own waits do not lock (the build lock never blocks: read in the code, not measured); a wall that
  far over CPU is a child step waiting (a plain step polls `try_wait`, a worker call
  `recv_timeout`, both up to the 600 s deadline) or memory pressure from parallel builds. If it
  repeats: `sample` the via pid and `ps` its children.
- [ ] **A far edit re-runs every step because its reads are whole-extent** (M4 slice 3 profile;
  graph.md Open). city `move_avenue` (1 run, 97.7 s against 101.6 s cold): all 10 steps re-run on a
  changed answer, not a memo miss: houses_old/houses_rows read every `kind:main_street`,
  houses_down every lot, old_streets `height` over [0,0,801,801]. Steps are 139 of the run's
  node-seconds; houses_down alone 49 s. quay `unrelated` (8.2 s against 8.0 s cold): houses and
  streets re-run on changed `build.features`. Fix in lib/town first (read the district's window
  and its own streets), then per-item houses (5.3), then node-answered reads.
- [ ] A turned import notches sheer ground (strait `move_b`, b at 80 degrees: water inside the
  slipway apron). No check sees it; a distortion measure, and resampling that keeps anchored edges.
- [ ] `import.seam`'s message says "across the edge" for a step inside the blend band (harbour,
  27 m inside a): say where it is.
- [ ] strait: the ferry is not drawn; no view shows the crossing between the imports.

### From M4 slice 3 (2026-11-16; the change, on-foot and gate reviews, and the fix pass)

The fix pass fixed: a scatter's `rule:` value was worked out from the world text alone
(`pre_edit` read `Decls::read`), so a density reading an import's place or a step's returned field
found nothing, the rule's value never changed and `held:` kept its key while its value moved:
`diag.internal` on border `move_abbey` and `shut` built `--against` base. It now reads the plan's
declarations (`Decls` and `Formulas` are `Clone`); case `graph/scatter_reads_step` failed before
(exit 3, `diag.internal`) and passes. `authority.conflict` with the import's ground the later
entry: the verbs were inverted and the hint asked the import to read the parent; now the earlier
leaves and the later asks, the hint fits an import, and `data.leaves`/`data.asks` say which (case
`anchored_clash_above`; `anchored_clash` checks the other order). `Anchor` owner is an enum. A
given field is written as a build output (composition.md 8, a spec line); quay re-measured, every
band held; quay `east_gate` section. Lesson: a value computed from a second, partial read of the
world is an edge the graph cannot see; compute it from the plan's own declarations.

**-> M4 slice 4**
- [ ] `moved_square` still fails (no `town.step:lay/i:` nodes): M4's acceptance is not met. See
  the slice 2 items (import steps, `for_each = "given:<name>"`).
- [ ] border: water stands 0.2 to 1.0 m in the walks (2188 posts above 0.05 m; max 1.02 m at
  (432, 364); west_opening blocked by 0.69 m). Each place's water is brought over its footprint as
  that place's ground alone, and the join's rank-1 walk beds lower the ground after it. Moving
  `[field.water]` into layers/join.toml changed nothing (2188 both ways: brought water, not the
  base); a layer cannot add a row to a field of another file (`identity.duplicate_id`). Needs a way
  to dry brought water against the joined ground (a kind effect on `tag:water_level` in the trail,
  or a rule in composition.md 6.2), then a `walk_wet` measure.
- [ ] A step other than a generate step at a join that reads `<import>.<field>` gets "no field",
  not a named refusal (composition.md 6.5 now says only a generate step reads it).
- [ ] `ModelPlan::scoped_values` makes a `Fields::new` per read (each window of `<import>.<field>`);
  unmeasured on a large world. Memoise per plan.
- [ ] county: `merge` rebuilds the imported town on every build (2.7 s of 5.4 s node time on
  unchanged, 63% of cold); `move_town` re-runs 1280 of 1280 chunks. Add an unchanged-percent row to
  county's expect.toml once `brought:` nodes exist.
- [ ] county `move_town`: the quay wall scalloped by the lattice and a staircase band along the
  turned town's edge in join_shore/join_west (paint or region edge on cells, not height: heights
  within 0.2 m over 3 m there). Resample paint with the footprint.
- [ ] border views: `at_gate` hugs the enclosure wall 0.75 m off (move the eye 5 to 8 m east, out
  of the gate); `south_opening` aims at the gate through the wall corner (aim at a point on the
  south walk); `wild.look_corner` to `wild.west_end` is blocked by the bank (add the reach pair or
  move the bank to the parent side); fringe trees on the steep bank (a slope limit is in the
  density, 22 to 30 degrees: check the bank's slope against it); brought views outside the
  footprint look at grass (drop or flag them).
- [ ] city unchanged peaks above cold (6.25 vs 5.05 GB) and `write:changes.json` takes 4.7 s
  (known, the slice 2 cost item).

### From the M4 close (2026-11-16; docs/reports/m4-sheet.png, the slice 3 reports)

Lesson: M4 slice 2 took 11 hours. A slice that grows past its box hides what is late; slice 3 ran
five agents in 90-minute boxes and wrote what did not fit here instead of carrying it. Keep that.
M4 closes on paper with its third acceptance unmet: a moved square is not shown (below, first).
The open "-> M4 slice 4" items above are re-sorted here; check them off in both places.

**-> M5 (first, before M5's own work: M4's unmet acceptance and its costs)**
- [x] Import steps run inside the parent's build (`<id>.step:...` nodes in the parent's graph,
  trust for an import's steps), then `for_each = "given:<name>"` and a `given:` selector; then
  `composition/moved_square` passes. Until then county's town is bare (`town_streets` 0,
  `town_instances` 0) and the moved square is not shown. Done M4 slice 4: a hook the model calls
  before it builds an import runs the import's generate steps in the parent's store
  (`build/imports.rs`); county's town has 2 streets and 68 instances; moved_square passes.
  Lesson: an import diagnostic's subject was stored as collection `town.cells`, which never read
  back, so every parent with an import that said anything lost its null build (county unchanged
  49% of cold -> 22%). A codec round-trip refusal is silent: say it (below).
- [ ] An import's lint steps do not run in the parent, and an import's fill pre-build (before
  give) runs none of its steps: decide whether a parent's build owes the import's lints
  (composition.md 12 says not). A step's join records (reading faults of a returned document)
  inside an import are dropped (`build/imports.rs`, `recs`).
- [ ] A diagnostic the store codec cannot read back is refused silently
  (`via-graph/src/codec.rs` encode): the record then has no world diagnostics and no null build
  follows. Raise `diag.internal` (run) naming the code and subject instead.
- [ ] The import's model nodes are still built in-process by `import:<id>` (leaves() prepares the
  parent and every import to make the build key): county unchanged is 3.5 s of 15.9 s cold (22%,
  six runs over three stores, release, other agents building), all of it that prepare.
- [ ] `brought:<id>/<field>` nodes per chunk and the import's graph in the parent's: county
  unchanged pays `merge` (2.7 s of 5.4 s node time; unchanged 56 to 63% of cold over three runs of
  the slice), `move_town` re-runs 1280 of 1280 chunks, strait `move_b` all of its chunks. Then an
  unchanged-percent row in county's expect.toml.
- [x] Paint and weights brought across an import (`import.weights_dropped` in every import of the
  three worlds): no setts, flags or trail paving show in county or border. Done M4 slice 4
  (composition.md 6.2 and 10; cases `weights_brought`, `binding_material`,
  `brought_unknown_material`): county shows earth, flags, gravel, setts.
- [ ] A turned import notches sheer ground: county quay wall scalloped, a staircase band along the
  turned town's edge (paint or region edge on cells), seam 14 pairs at 1.61; strait b's slipway
  apron. Resampling that keeps anchored edges, paint resampled with the footprint, a distortion
  measure. Part done M4 slice 4: the wall cell rule (6.2, lengths only after the review; cases
  `turned_wall`, `turned_tone`) takes county `wall_mid` 203 -> 122 after `move_town`; the seam
  stays 1.607; the 122 are slopes under 2 m a cell read bilinear.
- [x] border: brought water stands in the walks (2188 posts above 0.05 m, max 1.02 m); a way to dry
  brought water against the joined ground, then a `walk_wet` measure. Done M4 slice 4: a derived
  field brought as its offset from the field it is derived from (6.2, case `derived_brought`);
  border `walk_wet` 0 and `walk_wet_max` 0, base and `move_abbey`.
- [ ] Given features cannot be given (county: `doc.upward_ref`, `formula.type`,
  `formula.outside_field`); a `features` given has no `default` selector though the spec says so;
  a given region's default cannot be a named region. Then give county's road to the town.
- [x] Plan step 6 for M4: a fresh agent builds a small variation (a third import, or a world
  meeting another by ground) from the documents alone. Run M4 slice 4 (`hamlet`, a vale imported
  turned 30 degrees with an exit answered, then moved): built with 0 errors from the documents;
  eleven trips, the document ones fixed, the rest under "From M4 slice 4".

**-> later**
- [ ] border views and reach (`at_gate`, `south_opening`, `wild.look_corner`, trees on the bank,
  brought views outside the footprint): the slice 3 list above.
- [ ] Vocabulary the border agent could not say: growth from a part's own rule past its footprint,
  names for an outline's sides, two `follows` on one outline ending on each other, a name
  attribute holding several names, `blend` naming a place (composition.md 6.3 against 4).
- [ ] county: the erosion step on the county's land (skipped for the box); views of the town's
  district (its steps run since M4 slice 4; the README is written).
- [ ] Wide reads make a far edit re-run every step (city `move_avenue`, quay `unrelated`): the
  slice 3 profile item above; city unchanged peaks above cold.
- [ ] `scoped_values` per read, a named refusal for a non-generate step reading `<import>.<field>`,
  `anchored_at` by box containment: the slice 3 items above.

### From M4 slice 4 (2026-11-16; the change, review, fresh-agent and gate runs, and the fix pass)

Lesson: the wall rule was written for ground and applied to every field read; a threshold in metres
must say which fields are metres. A rule's unit is part of the rule.

- [ ] The terminal does not print the last 40 lines of a failed step's log (protocol.md 6.2 says it
  does; code and spec disagree since M1). The `protocol.raised` hint now names where the log is
  (`<out>.run.json` `steps[].log`, `<store>/objects/<2 hex>/<digest>`); print the tail, or a
  `via log <step>` command.
- [ ] lib/review reach lint: refuse an unknown params key or a malformed entry (`{ from, to }` at the
  top level ran silently; `sealed` entries without `profile` raised AttributeError) and say one
  info when it checks nothing. Its reference is now in lib/review/kinds.toml (hamlet trip 1).
- [ ] An exit on the outline of a turned import is `import.exit_outside` (hamlet: gate at (196, 0)
  on the vale's south edge, `face = 30`; `face = 0` clean). `inside` takes half a millimetre for the
  frame's rounding; the exit test does not, or the footprint's own turned ring rounds inward. Then a
  case `exit_on_turned_edge`.
- [ ] `via build` asks once per new `--change` to be trusted although every step is trusted and
  unchanged (hamlet trip 11, README "and once per --change"): decide whether a change that adds no
  step needs trust.
- [ ] A crossing's `deck_z` is a number measured from a build (hamlet, lib/ways README): a deck
  height from the ground at its ends.
- [ ] The river of an import leaves at two edges with no exit and nothing is said (composition
  Open "what leaves undeclared"); hamlet's last seam (0.62) is probably there, not checked.
- [ ] Not run in the fix pass: a small-world probe that a changed import step code file re-keys
  the parent (review "Left").
- [ ] A derived field read by its offset has no case at a wall cell where the derived field is flat
  (an underwater quay face, review issue 2): the code reads both at one post since the fix pass;
  a case with exact post values is owed.

### From M5 slice 1 (2026-11-16; the change and on-foot reviews, and the fix pass)

Lesson: a write node keyed by its reads' values misses what only a statement says: unsetting
`ends_at` changes no shape, so `meets.json` stayed stale while every node read was unchanged. A
file made from declarations holds those statements in its key.

To slice 2:
- [x] Core: `cut` (`fields/<terrain>.cut.npy` and the manifest's `cut`, build.md 6.5), kind
  surface meshes (`meshes/<owner>.<surface>.glb` with `material`, build.md 10: each grid cell
  clipped to the bound, then 3.10), `surface` on a scatter, a set and a point feature, a run `on` a
  storey (surfaces.md 4.1), and `place.overlap` by level (rules.md 8.1: one surface and meeting
  height spans, decided on plans). Cases `cut`, `cut_bad`, `floor_mesh`, `roof_scatter` pass; new
  `storey_overlap`, `floor_warm`. Station (debug build, one run, resolution 0.5, scratch copy):
  `off_floor` 102 -> 0, `shell_gave_way` 10 -> 0, 9 meshes; move_stair and close_passage warm
  equal clean. Lesson: a placement's own node must carry what another's check reads of it (an
  instance's `surface` entered `instance:` only after `storey_overlap` showed a stale overlap).
- [x] station expect.toml: the two OWED rows lifted (`off_floor`, `shell_gave_way` 0), and close_passage
  `meetings` 11 now holds (slice 2, station agent; release build, `measure.py --check` all three).
- [x] Engine (station agent): a run a step returns had its members on the terrain: `run_of` split
  `lay#loft_s1_w0_0#r3` at the first `#` and found no run `lay`, so the member had no stance and the
  storey-1 walls stood at the ground, overlapped storey 0's and gave `diag.internal` (place.overlap
  "raised at the header"). Now the last `#` (identity.md 2.3 says so); case
  `surfaces/step_run_on_floor` (fails before: z 10, not 13.3). Recheck the `place.keep_out` on a
  parapet member item below: likely the same root. Lesson: a produced name holds a `#` per
  producer; a parser of names must say which `#` it reads.
- [x] (slice 3: point_on_deck, targets take the owner's one surface where the bound holds them;
  M5 close: that left instances and path ends on the terrain, the on-foot review found every tower
  view still looking down; now an instance target takes the surface it stands on and a path's end
  the surface its `ends_at` names, case `surfaces/target_on_floor`, station `lens` 9.85)
  Engine: a bookmark's `toward` target on a kind surface gets the terrain's z in features.json
  (station `lens` 0.01 for 9.85, `stair_2.end` 0.01 for 6.7, `kitchen_table` -0.01 for 0.45): every
  tower view looks steeply down through its floor. The eye stands right (slice 2).
- [ ] lib/building `lay`: an inner wall between rooms leaves no gap for a flight (the station loft's
  walk up its flight was blocked at 2.89 m with the cut at x = 49.3), and a door may land on a
  rooms' cut (the lint's `door_narrow` said 0 m clear). Refuse or move either (`params`).
- [ ] Sheet walks go straight at `toward`, so no walk climbs a spiral flight (station stair_1 to 3);
  a walk along a path (`toward` a flight's end, following its line) would. Closing a passage takes
  the meeting away, not the way: the walker still crosses `tower_step` with it unset.
- [x] (slice 3: surfaces.md 4.1, 4.2, case `ramp_scatter`) A scatter on a kind surface skips the terrain's relief and does not yet take `relief` over
  the kind surface (surfaces.md 2.3) against `max_relief`; a flat roof hides it.
- [ ] A run's members have no node of their own: their change entries carry the cause (the model's
  deps) with an empty `via` chain (case `floor_warm`, `edge#r0`). Give runs `rule`/`height` nodes.
  (Slice 2: the instance file now reads what lays a run, so files are right; the nodes are still owed.)
- [ ] The review's "remove dead `is_path`" found nothing dead in crates/ (every `is_path` is read).
- [ ] meets.json's write node now reads the meeting features' shape and attr, what their ends
  name, every instance and height, the facts of assets with mounts, and the terrain under ends and
  under each plan's footprint grown by 5 m (`MOUNT_REACH`, an asset mount off its footprint); its key
  holds the statements of those features, their kinds and every instance. Replace the 5 m with the
  mounts' own points, and add warm-equals-clean cases for a connector edit in kinds.toml and a world
  with the crossing removed (review issue 2 (b), (c); the cases were written in slice 3:
  `meets_connector_edit`, `meets_crossing_removed`; the 5 m reach stays).
- [x] lib/building: wall runs leave each doorway and flight a clear width (station front door
  0.20 m, tower step 0.29 m; the walker is 0.6 m), and check.py measures the clear width between
  wall instances at each doorway against `inner_door`. Station's hand runs end short: 1.116 m clear
  at the doors, 1.156 m at the step (measure.py `clear_min_mm`; 0.203 / 0.259 with the old spans).
- [ ] lib/ways: `[kind.crossing.surface.deck]` in place of the deck effect once meshes and the cut
  land; a parapet run; a lint that water passes beneath; bookmark walks along the deck. Then rebuild
  townland and example, `measure --check`.
- [x] (coast agent) townland base, move_crossing, move_hill and example base, move_beck rebuilt on
  the deck surface: `deck_wet` rows now the river under the deck (townland 78 of 192, example 69
  of 240; example's measure counted dry posts as wet, `>=`, now depth over 1 mm). Every other row in
  band but townland `reach` (below). Renders read: water runs under both decks.
- [x] (M5, reach agent: 101, 96, 60 all held; OWED lifted) townland `reach` 50 of 101 (move_crossing 40 of 96, move_hill 9 of 60), OWED in its
  expect.toml: lib/review's reach walks the terrain only, and the deck is no longer terrain, so the
  river cuts the town in two. Reach over kind surfaces through meetings (4.4) fixes it. Lesson: a
  library that leaves the terrain must land with the walk that follows it, or every reach measure
  downstream falls at once.
- [x] (slice 2 fix: the instance file reads each run's `on` target, its kind and the ground
  under it; coast long_deck warm = clean, 26 posts; floor_warm `moved`) Engine: a run `on` a
  crossing is not re-placed when the crossing's ends move. Warm coast
  `long_deck` keeps base's 18 parapet posts where clean lays 26; a store shared by two change
  builds handed `move_cave` the 26 posts of `long_deck`; warm `move_cave` keeps a post's z that
  clean moves 0.02 mm. The run's key misses the resolved ends (`starts_at` points). Same root as
  "a run's members have no node" above. coast expect `long_deck.parapet_posts` OWED.
- [x] (slice 3, case `point_on_deck`) Engine: a point feature `on` a feature with one kind surface stands on the terrain, not that
  surface (surfaces.md 4.1 row 1): townland `on_bridge` (eye at the river, under the deck) and a
  bookmark `on` the cave (20 m up). coast's `in_cave` uses `at` + `surface` until then.
- [ ] Engine: `place.keep_out` raised on a step's run member is `diag.internal` ("raised at the
  header, which has no span"); seen with lib/ways parapet in a world with a `keep_out` field. And
  the crossing's own `clear` refuses its parapet: posts on the deck should not read the terrain's
  keep_out (or `clear` stops at the deck's edge). coast has no keep_out field meanwhile.
- [ ] coast (M5 slice 2): the cave's mouth is a slot the face's full height and the passage has no
  rock (challenge 8: a library places rock round a cave, surfaces.md 5); the sheer face leaves
  0.13 m² a metre open at foot and brink (a cut quad reaches 0.71 of a post past the strip). The
  sheer kinds for the monastery wall, gorge and tarn are not written (`sheer_cliff` is the cliff's).
- [x] Engine gaps lib/building found (slice 2, core-proto): list and table attributes reach a step
  as canonical JSON (`Attr::Json`, protocol.md 4.3; the never-given `integer | i64` row dropped);
  names in a returned document resolve to its keys first (`Decls::own_names`, protocol.md 5);
  `surface.surfaces_at` and `surface.sample` over the terrain and kind surfaces, in the SDK as
  `step.surfaces_at` and `step.sample`; cases protocol c72 to c75. `test_lay.World` runs.
  Lesson: an attribute type the reader drops (`_ => None`) is silent; the step saw `?` = 0 and a
  library had to guess. Every value the text accepts should reach a reader or be refused.
- [ ] (M5 close: surfaces.md 7 now says what the command does, no step, and where to ask instead;
  the step runner is owed) `via query` builds with `via_model::build` alone: it runs no generate step, though
  surfaces.md 7 says it does, so a world built by a step answers `meetings` empty (the house of
  `test_lay.World` gave `[]`; the test reads `meets.json` from `via build` instead). Also `relief`
  of a kind surface is not answered by `via query` (a usage error) nor by the protocol.
- [ ] A vector case for a noise `base`: the stream is now `["noise", "feature", <fid>, "surface",
  <name>, "base"]` (formulas.md), fixed in the fix pass; no world stored values under the old one.
- [x] Docs at the M5 close: challenges rows 7, 14, 15, 48 (mounts now have face, connector and the
  height of their surface). (M5 close fix pass, with 8, 18, 20, 33, 46, 47.)

Later:
- [x] station views: `in_living` and `in_porch` moved into open floor, `in_kitchen` off a barrel
  (slice 2).
- [ ] college: box collision for the primitive pack, a door at each `entry`, the chapel padded to
  its court (README "Known faults").
- [ ] A point feature's `surface` z as a meeting's default mount (3.3 now says so); until then the
  code gives the terrain. features.json gives its surface's z since M5 slice 2.

### From M5 slice 2 (2026-11-17; the change and on-foot reviews, and the fix pass)

Lesson: the slice-1 lesson again, three times (features.json, a mesh, the instance file): a file
made from declarations or pack text holds them in its key. A run's members and a set's items have
no node, so their file read nothing that lays them; coast long_deck wrote a manifest of 26 beside
arrays of 18.

Closed in the fix pass: features.json keyed on every point feature and every kind with a surface;
a mesh on its materials' colours; manifest `materials` holds mesh materials (build.md 7); the
terrain and a kind surface with `cut` (or whose bound the footprint meets) are one level (rules.md
8.1, owner's ruling; case `ground_floor_clash`); a run member 1 lattice unit past a strip's end
stands on it (surfaces.md 1.2; coast builds); the instance file reads runs' `on` targets and set
statements (floor_warm `unsurface`, `moved`, `reitem`, `rebase`; floor_mesh material/recolour);
geometry/triangulate runs (passes); ways' parapet test no longer expected to fail; test_lay uses
its own VIA_CONFIG_DIR; a mesh's formula fault is said, not dropped.

To slice 3:
- [x] (slice 3, town-levels: every row in band but the OWED reach rows, unchanged) townland and
  example not rebuilt after the level rule (rules.md 8.1) and the instance-file
  key: rebuild every change, `measure --check`; a terrain scatter under a deck now gives way to
  parapet posts whose footprint meets it (spans compared as one datum).
- [x] (slice 3) The 14 surfaces cases marked owed (apart, bound_zero, bridge_gap, clipped, fold,
  ground_on_deck, meets_errors, mount_face, name_clash, outside, owner_shape, relief_outside,
  through, unknown_surface) have no world.toml: write each from surfaces.md Conformance.
- [x] station: 8 wall members give way to furniture (south wall hole 2.8 m, both bunk walls);
  set dressing off the wall line or rank walls higher; measure `wall_gave_way` over every
  `structure/wall` run, band [0, 0] (on-foot review 3).
- [x] station: the tower's doorway gap runs the shell's full height; give the gap storey 0's
  height (per-storey shell runs or a lintel run) and measure open wall above the door head
  (on-foot review 4).
- [x] walk.gd stops a walker "by water" on a deck over deep water (townland move_crossing);
  take depth against the feet, add a walk test across the townland deck (on-foot review 5).
- [x] station `at_back_door` arrives through the wall (bookcase within 1 m); aim it inside
  (on-foot review 6).
- [x] stepview: `surfaces_at` lists owners `sample` then refuses (rank test differs); share one
  predicate (change review 14). (Slice 3: `out_of_view`; slice 3 fix: case
  `protocol/c76_surfaces_at_returned`, protocol.md row.)
- [ ] The glTF reader (build.md 15.3) is not written: only engines/godot exists, so nothing to
  skip cut quads in yet (change review 15). Owed with the glTF reader, M7 (engines).
- [x] parapet.py: a post at span 0 overhangs the deck's end by half its depth; inset the span by
  half the post's depth over the deck length (the core now grounds it; the overhang is a look).
- [x] station: the lens's body loads unnamed (`@StaticBody3D@9`), so `up_stair_3` and `in_lantern`
  end `blocked` against their own target, not `arrived` (loader node names; slice 3 loader-station).
  (Slice 3 fix: a chunk's bodies under their own `bodies` node; test_loader checks no `@` names.)
- [ ] Spiral walks need `along` (build.md 15.1, lib/review `bookmark.along`, text, not a name
  attribute: decided in slice 3, open to the owner); a lint that every flight is walked by some
  bookmark is not written.
- [x] (slice 3, parts agent) Things with parts (spec/assets.md 12): a set or sockets, one root one
  unit (rules.md 8.1, case `assets/parts_one_root`), heights by `sink`, accessories as overlays on
  sockets, state read from the build. `sidearm` (racked: one item moved, 4 parts follow) and
  `pistol` (make_pack.py from published.toml; rest, racked, three accessories: measure.py off 0 in
  every state). Lesson: an overlap edge between two branches of one tree of parts closed a loop
  through each part's host height; parts of one root had to be one unit before any accessory built.
- [x] Engine: two trees of parts side by side, each two deep, can still close the overlap loop
  across trees (height(a1) reads plan(b2) reads height(b1) reads plan(a2)); the overlap test reads
  plans and should not be an input of a height. (Slice 3 fix: one part deep sufficed, any distance;
  the test is now its own `overlap` node, graph.md 1; case `assets/parts_two_hosts`. M5 close:
  the two-deep case `assets/parts_two_hosts_deep` written; no loop; moved half over each other it
  found an overlap on a set's item as `diag.internal`, fixed in rules.rs.)
- [ ] A mount with a height (spec/assets.md 12.2, owed): parts lie on flat ground only, a part at
  two heights is two assets, an accessory's asset states its socket's height. Build when a world
  needs it.
- [x] pistol/sidearm: README, expect.toml and a fresh-agent run not written (time box); the
  trigger guard is two bars open at the back; challenges rows 46 and 47 to update at the M5 close.
  (M5 close, parts agent: READMEs; expect.toml per state, `measure.py --check STATE` reads it and
  the build's own state; sidearm's `numbers.toml` and measure; the guard's bottom bar runs into the
  leaning grip, closed; rows 46, 47. Lesson: a world without its own numbers had nothing to show
  the barrel 17 mm proud; the measure found it at once, `crown` 17. The fresh-agent run is not
  done: below.)
- [ ] pistol/sidearm: a fresh agent, given only the READMEs and spec/assets.md 12, builds a
  variation (a fourth accessory on a new socket, or the sidearm racked half way).
- [ ] An overlap on a set's item is now at its set placement's header (case
  `assets/parts_two_hosts_deep`; it was `diag.internal`); other records raised at an item's own
  header (`place.keep_out`, rules.rs) still are: route them through `place::declared_at` too, with a
  case each.

Later:
- [ ] The level test compares spans from two datums (terrain and floor) as one; a ground floor
  above the terrain by more than a piece's height still clashes. Read the floor's `base` at plan
  time once surfaces settle before placement.
- [x] Warm-equals-clean cases for a connector edit in kinds.toml and a world with the crossing
  removed (meets.json, slice-1 item): `meets_connector_edit`, `meets_crossing_removed` (slice 3).

### From M5 slice 3, levels and sheer faces (2026-11-17; examples/monastery, city; lib/land, lib/ways)

Lesson: a world not rebuilt after a library change that moves a surface hides OWED rows for two
slices: monastery and city (decks as surfaces, slice 2) failed `deck_wet` and `reach` at HEAD
already; rebuild every world a library change reaches in the slice that makes it.

- [x] (M5, reach agent: one node per (surface, post) from `surfaces_at`, surfaces joined only at
  meetings that hold; the step a worker; monastery 10 of 10 in base and every change, city 6 of 6;
  lib/review test_reach `KindSurfaces`, test_reach_world deck cases) lib/review reach over kind surfaces (4.4): monastery `reach` 3 of 10 and city 2 of 6 are now
  OWED as townland's (the road cannot cross the gorge or the river on a deck).
- [ ] geometry.md 2.7: a bridge's line over a sheer face is `geom.unowned_crossing` (monastery 2,
  every change): a deck over a face is no meeting, and a crossing names only the water it crosses.
  A kind listing more than one name in `names`, or a face (a path whose kind lays no way) left out
  of the check: decide (open to the owner).
- [ ] Monastery faces, seen on the sheet: (M5 close, look pass: the saw-tooth of open slivers is
  closed by the mesh's skirt, build.md 10, case `surfaces/skirt`: open cut area 124 to 0.2 m^2 on
  the east wall, 95 to 0.5 on the west, coast's `sliver_m2_per_m` 0.129 to 0; the lit wedges were
  the pieces' `taper` 0.0001, which brought every vertex on a piece's end line to the foot: now
  `taper = 0` and joints mitred square to the bisector, gorge_faces.py `mitre`. Left: the skirt is
  the face's one material, so from above the brink shows a rock fringe up to 0.71 m wide with the
  cut quads' saw-tooth outline (monastery `bm_tarn`, near wall); a loader drawing a cut quad
  clipped to the strip would end it; hairline seams at the joints.) In the tarn's head the faces' flat 2.75 m foot fades into
  the hollow (68 quads over 2 m, worst 2.81 m). The pieces are written by coordinates
  (gorge_faces.py): moving the stream leaves them; a face `follows` with a varying offset, or a
  face whose heights read the terrain (surfaces.md Open 9), would not need the generator.
- [ ] Monastery wall: lib/town kit.py would give corners and end piers, but its gaps come from
  keep_out only: the approach's verge opened 15 m at the gate (tried; the run's span gives 5.1 m).
  A gap by place (a gate point) in kit.py, then move the wall to it. The run's bays overlap at
  the corners.
- [ ] City levels: no slip ramps from the freeway on the ground (a diamond), a loop lands 7 m
  beside the arterial with no merge, no piers or deck edges or parapet; the viaduct climbs at up to
  6.1 % in 300 m. `z1` of each ramp and the viaducts' `z_start`/`z_end` are typed from a build's
  ground (as `deck_z`): a moved freeway end or arterial leaves them (Open 9 again).
- [ ] lib/ways `interchange` now requires `level` (was the ground read): a 56 m plate for the
  ramps' feet took 55 rows houses; the feet hold their own ground (`ramp.foot`) outside the plate
  instead. A plate that is not a disc (only where ramps land) would let them land on it.

### From M5 slice 3, town kit (2026-11-17; corners, wedges, kit runs; examples/quay)

Lesson: a fallback chosen before rule 4 (keep_out) never ran in quay: the rows fitted the lots'
geometry and were then dropped by the footway's keep_out, so `fallback` saw no empty lot. A rule
that answers "nothing stood" must run after every rule that removes, in the same batch.

- [x] (M5 close, core: named, not a fault) Core: a distant edit reshapes a lot that keeps its name (quay `unrelated`:
  `lots#c48422ba69c7929ee_2` at (134.6, 139.7), 83 -> 87 outline points), and its shallow house no
  longer stands clear; `unrelated.houses_west` re-recorded 0 -> 1. Same root as the slice 5 case
  (s1's thinned points). Found: `streets#s1`'s points near the lot are equal in both builds (its
  east end moves 232 -> 226 with the district); the lot's side cuts along s1 move about 2.2 m
  ((142.013, 142.21) -> (140.385, 143.716)). geometry.md 3.6 step 4: a stretch's stations stand at
  `k L / n`, so shortening a stretch at its far end slides every station along it; the lot keeps
  its station number, so its name. Rule as written. A station by distance from one end (the rest
  at the far end), so a far edit moves only the last cells, is a choice for the owner (it changes
  every cells case's numbers).
- [ ] lib/town houses.py: rule 2c pairs only lots no row fits by geometry; a pair whose rows fit
  but are dropped by keep_out is not tried (a second batch, or corner tries in rule 4's). 7 pairs
  found leave no corner house clear (most beside the main street's 5.25 m keep_out): a smaller
  corner asset or a chamfered corner piece.
- [x] (M5, reach agent: `mount` may be a list; quay names `["door", "side"]`, 9 side doors walked,
  `side_doors` and reach rows in quay's expect) lib/review reach checks the `door` mount only: the corner house's side door (`side`) is not
  walked to. Reach every mount with a door's role, or name both in quay's reach.
- [ ] lib/town kit.py: quay's wall never cuts a bay to fillers (`kit_split` 0, the slope is 0.15);
  rule 6 is tested only in test_kit.py. A world with a run up a steeper slope (monastery wall,
  townland) should show it. After `unrelated` the east wall has no gap where s0 and s1 end 7 m
  inside the district (gaps come from keep_out only): a gate in the wall at a street's end is owed.
- [ ] spec/protocol.md 3.3 now says `sink` comes as f64 metres (the code's answer); `max_relief`
  is not served by `pack.query` at all (stepview.rs has no row): serve it or say so.

### From M5 slice 3, market (2026-11-17; the box kit and its three changes; examples/market)

Lesson: a library that lays a kit run as a step's instances gets none of the core run's
exemptions: 86 false `place.overlap` (sub-millimetre, by rounding to the lattice off the axes, and
on bends) until kit.py kept neighbours apart itself (rule 7). A rule written for a core form must
say whether a library's lowering of that form inherits it.

- [x] (slice 3 fix: a `follows` shape had no edge to what it follows, graph.md 2 says it has; case
  `geometry/follows_moved`; market's four changes build on one store) Core: on a store that holds the base build, a change that moves `main_street` stops with
  `diag.internal`: `shape:row_church_n/v:build` not stable (30ff8482 then e7305d6c, the same pair
  for `move_street` and `move_bend`). `row_church_n` follows `church_street`, which starts at a
  point placed `on` `main_street`: the follows shape's key seems to miss that chain. A fresh store
  builds both cleanly. Repro: examples/market, base then `--change changes/move_street.toml
  --against` on one store.
- [ ] Core or spec: a foot placed `on` a path by `t` slides when the path grows longer (`move_bend`:
  101 of the other streets' 102 pieces move), and the moved church foot then meets the main street
  off its line (`geom.unowned_crossing` 1). A meeting place that keeps its distance from one end,
  or an end that names the path it meets (`starts_at = "main_street"` at the nearest point), is owed.
- [ ] lib/town kit.py: one piece per role (the first id bytewise); the spike's rows mixed a shop and
  a tall shop by weight. Variants by weight, seeded by place, are owed for a street that does not
  read as one long block.
- [ ] lib/town kit.py: a row on the outside of a bend under 10 degrees per vertex (a round join of
  the follows offset) opens one joint to 1.1 m (`max_joint_cm` 113); a wedge filler, or a corner by
  the turn over a piece's length rather than per vertex, is owed.
- [ ] examples/market: the rows stand on the ground as it lies (no terrace): 85 of 189 doors within
  0.3 m of the footway, 0 more than 1 m off. A kit run that returns a terrace per stretch (as
  houses.py does) is owed; then `max_relief` back to 0.6.
- [ ] The real kit: the TODO asks for a box kit then a real one; only the box kit is built.

### From lib/review reach over kind surfaces (2026-11-17, M5; townland, monastery, city, quay)

Lesson: a library lint that needs the engine (`surfaces_at`, `meetings`) runs only as a worker; a
plain command's `NotAWorker` turned into a silent terrain-only walk would have hidden the decks
again, so the lint now says `terrain_only` (info).

- [ ] college, border and example declare reach as a plain command: they walk the terrain only and
  now say `terrain_only`. Make each a worker and re-record (example's crossing deck is a kind
  surface: its reach rows may rise). Not rebuilt here (not this agent's worlds).
- [ ] The terrain's cut (surfaces.md 1.3, 4.4 item 4) is not served to a step: reach walks a cut quad
  as ground (a floor with `cut` under a hill). Serve the cut (a field read, or `surfaces_at` leaving
  the terrain out where cut), then read it.
- [ ] A point feature's surface is not in the feature list (no `z`, no `surface`): reach stands a
  point on the lowest open surface at its nearest open post. A point `on` a deck over dry ground
  stands on the ground under it. Give the feature list `z` (features.json has it), or a `surface`.

### From M5 slice 3 (2026-11-17; the change and on-foot reviews, and the fix pass)

Lesson: two blockers were old: `follows` had no graph edge since M1 (graph.md 2 lists it), and a
scatter's `place.gave_way` was in no key. Both showed only as `diag.internal` on a shared store,
which no case built until market's changes did. A reference kind in graph.md 2's table with no
edge in code, or a diagnostic routed to a node whose key does not hold it, is silent until then.

Closed in the fix pass: the overlap test of two hand instances is its own `overlap` node (graph.md
1), no height reads it (case `assets/parts_two_hosts`: two hosts 5 m apart with a part each gave
`graph.cycle`); a scatter's `rule` value holds what its run said (case `ramp_scatter` `narrow`);
`follows` reads its reference (case `geometry/follows_moved`; market's four changes on one store);
a point `on` a thing past the bound takes the terrain, and within the grace the surface's base in
features.json too (case `point_on_deck`; monastery base builds, `bm_gorge` on the gorge floor and
moved with the bridge); step instances skip `surface.through`, `off_bound` and kind relief by name
(case `through_step`); `surfaces_at`'s returned-owner rule in protocol.md (case `c76`); station's
chest inside its floor; loader bodies under `bodies` (station `up_stair_3`, `in_lantern` arrive);
case `assets/parts_on_set_item`; surfaces.md 2.3 says crossings are rounded to the lattice.

To the M5 close:
- [x] (M5 close, core: via-model test `every_reference_in_graph_md_2_makes_an_edge`, which parses
  the table, so a new row with no probe fails; 22 probed edges over 12 rows, `set` by statement, six step,
  write and give rows by their conformance cases; six changes built warm on one store equal
  clean, and back; case `composition/give_moved`, new; removing the `follows` edge fails it. No
  other edge was missing.) A check that every reference kind in graph.md 2's edge table makes an edge in graph.rs
  (`follows` was missing from M1 to M5): a test per row, a world with that reference, the target
  moved, warm equals clean.
- [x] Decks: outlines notched where they meet the banks (city `on_north_bridge`); walks from a deck
  toward a target to the side leave it sideways and stop `blocked` by water (city
  `on_north_bridge`, townland `on_bridge`): trim the mesh at its bound, aim those bookmarks along
  the deck or wait for parapets (on-foot review 5). (M5 close, look pass: the mesh was inside its
  bound; the notches were the held ground standing over the cambered deck, up to 0.13 m on city.
  lib/ways `crossing`: the hold is `deck_z` only within `crown` of the line, the deck has `lift`
  instead of camber over the hold, and `beneath` keeps the ground under the whole span: terrain
  over a deck vertex 0 on all six decks, from up to 0.41 m on monastery. Both bookmarks look along
  their deck and arrive. city's crossings write `crown = 3` for its 2 m posts.)
- [ ] Decks: a kind default cannot read the world's resolution, so `crown` (1.5, for 1 m posts) is
  restated by a world at coarser posts (city). A formula name for the resolution would let the
  kind say `1.42 * resolution` itself.
- [x] station: `loft_top`'s eye is under the loft floor (all grey, walk blocked at 0.3 m); `outside`
  stops `blocked` at `cottage.floor` (seen on the fix pass's sheet, not reviewed). (M5 close, look
  pass: `loft_top` stands 5.2 m over the ground, 1.7 over the upper floor, and walks down the
  flight, arrived; `outside`'s straight line met the cottage's south side where no door is, so the
  floor's 0.45 m edge stopped it: it follows `walk` in by the front door and stops at the kitchen
  table across the line to the lens, 13.9 m short.)
- [ ] station `outside` (M5 close: still 13.9 m short at the kitchen table): a view whose target no straight line reaches ends `blocked` on the sheet
  however it is aimed; a walk to a place inside (a `along` path through the rooms) or a review that
  reads a blocked walk by furniture as no fault is owed.
- [x] lib/ways parapet.py: a deck shorter than the piece is deep said `narrow` ("as wide as the
  deck"); now `short`, with its own text (test `test_a_deck_no_longer_than_the_piece_is_short_not_narrow`).
- [x] pistol/sidearm: the optic hood is a solid plate, the grip upright; sidearm's barrel shows
  past the slide at rest (on-foot review 8; README and expect.toml are in "From M5 slice 2").
  (M5 close: the hood is two posts and a bar round its window; both grips lean as models, a box
  sheared by a tool with packgen's writer, since a placed part stands upright, assets.md 12.5; the
  barrel's crown 2 mm ahead of the slide, checked by `crown` in sidearm's measure.)

Later:
- [ ] Performance: city's unchanged rebuild 21.5 s (2 s elsewhere); `move_avenue` and
  `move_bridge` 105 to 114 s against a 120 s cold build; quay `unrelated` 19.2 s against 16.3 s
  cold (on-foot review 7, one run each, release, a shared machine). Profile what re-runs.
  M5 close (core; run.json node times and `sample`; one run each, release, 14 threads, HEAD's
  examples and lib copied aside since both were being edited, other agents' builds on the
  machine): the change report searched the whole graph once per changed thing, sorting every
  node's inputs again (`causes_from`); now one indexed graph and a search over the nodes that
  depend on a changed statement (`CauseGraph`): quay `write:changes.json` 3.73 -> 0.39 s, bytes
  equal, `unrelated` 17.3 -> 14.0 s (cold 14.7). The build key's leaves placed the whole world
  only to list files: city's same-directory unchanged rebuild (the null build) 8.4 -> 1.5 s. Left:
  (a) core: an unchanged rebuild into a new directory is no null build (graph.md 6.2 asks the
  directory's own manifest), so it plans: city 16.1 s, of which `prepare` (placing every `where`
  district and network, once per stratum) about 5.5 s, defining 20k nodes about 3 s, store reads
  0.8 s. Plans made lazily, or a null build into a fresh directory, is owed. (b) core: city
  `write:changes.json` still 2.9 s in `move_avenue` (unprofiled). (c) library: every change re-runs
  city's three `houses` steps whole (64 to 80 s each in `move_avenue`, 133 s in all against a
  106 s cold build): each reads `build.features` over every lot, so one lot anywhere re-runs it;
  quay's `houses` likewise (6.7 s of 14 s). A `for_each` per district or block in lib/town is owed.
- [ ] The overlap test reads every other hand instance's `instance` node, O(n^2) edges, as the
  height did before; a grid of hand instances by plan would cut it when a world needs it.

### From M5 close (2026-11-17; the fresh agent, the change and on-foot reviews, and the fix pass)

- [ ] M4 worlds not rebuilt after M5 library changes (found by the M5 sheet): county base
  `town_instances` 324 against [68, 68] (quay, its import, now lays its wall with lib/town kit.py,
  slice 3) and border base `unowned` 2 against [0, 0] (monastery, its import, now has sheer faces;
  the bridge over a face is `geom.unowned_crossing`, open item above). Confirm each cause, then
  re-band with the reason or fix; rebuild every importing world in the slice that changes its
  import (M6 slice 1).

Lesson: a fix closed at the case it was written for and not at the world that found it: the slice-3
target fix covered a point `on` a deck, and station's tower views still looked through their
floors. Close a TODO item from the world's own render, not from the case alone.

- [x] Plan step 6 for M5: a fresh agent builds a small variation from the documents alone. Run at
  the M5 close (`brook`: a beck, a footbridge with parapet, two lanes meeting it, a two-storey laid
  house with a door and a stair, a walk from the lane up to the upper floor; then the house moved
  north and turned 36.87 degrees): both builds 0 errors, 8 meetings at gap 0, the walk stops on
  `lay#house_s1.floor` before (28.9 m) and after (33.3 m). Nine trips; fixed in the documents or
  the messages: the Build blocks' `via config program` (station, coast, county), the gap hint (a
  produced side named with its producer, and setting the ground by an effect), `via query`'s
  chapter (no step runs), a turned house's ends rounded off their floor (`lay.py` `onto`, test),
  `via explain step.*` with no build there, `build.pack_root_far` on one drive (a world named by a
  bare file name), walks judged by `on.stop` and a waypoint before a flight's foot (build.md
  15.1), the produced-name pattern and doors off a way's bed (lib/building README "Using it"), the
  `front` attribute and a door piece. Left: below.
- [x] Closed in the fix pass: targets on floors (instances and path ends, case
  `surfaces/target_on_floor`); a walker falling off a deck says `fell`, never `arrived` in the air
  (walk.gd, test_floor); station `move_stair` moves the living table with the stair and
  `measure.py` `in_flight` (0 in every build; 1 before); quay `move_hill` moves `wedge_house` to a
  wedge of that build; `along` on townland `way_bridge`, monastery `bm_bridge_end`, coast
  `on_bridge` (all arrive, base and change); rules.md 8.1 states the terrain-and-floor level (R40);
  the skirt covers only its kind's cut cells (cases `skirt_neighbour`, `skirt_moved`; decided,
  open to the owner); authority.md's volume lines closed by R37; the kit's `ends` param renamed
  `caps` (rules.md 5 keeps `ends` for the core run); lib/building README's pistol line; the edge
  test compares fields and features after "and back" and wants a change in each case.

To M6:
- [ ] `via query` runs no step (surfaces.md 7 says so now): a step runner for `surfaces_at`,
  `sample` and `meetings` over a laid building, and `relief` over a kind surface.
- [ ] `surface.gap` says "what sets each" only by name: the effect that wins the terrain at the
  end (a lane's `bed` over a door's `sill`) is not named; field provenance at a post is owed.
- [ ] `lay` returns no place name a bookmark can target (a storey, a stair's foot): a mount or a
  documented point per door and stair foot would replace the fresh agent's hand `t`/`offset`
  points. Decide with the produced-name rule (R3).
- [ ] townland `ways_meet` walks from the bridge's east end toward its start and stops at the
  bank: `along` walks a path from its start only; a reversed `along` (or from the nearer end) is
  owed. city `on_central_bridge` and `on_harbor_bridge` look down river from mid-deck and now say
  `fell` off the deck's side (no parapet): aim them along, or run `parapet` (next item).
- [ ] Parapets on monastery's, city's and townland's decks (only coast runs lib/ways `parapet`),
  or say in each README why not (on-foot review 6).
- [ ] Quay's `wedge_house` in `move_hill` is placed by coordinates from that build's houses (R3):
  a view that follows a produced wedge needs a name for it.
- [ ] A lint that furniture stands off a flight's line (station's `in_flight` is a world measure);
  lib/building `check` should report it for any building.

To M7:
- [ ] The loader draws the skirt's cut quads clipped to the strip (monastery `bm_tarn` fringe;
  build.md 15.3 glTF reader, slice 2 item).

Later:
- [ ] The kit param rename `ends` -> `caps` (decided, open to the owner): if the owner prefers
  another word, market's two runs and test_kit change with it.
- [ ] Stations by distance from one end (doctrine 6, "Open (M5 close)"), with
  `identity/c27_stretch_lengthened`.
- [ ] station `close_passage`: the sheet still walks through the closed passage (a leaf, not a
  meeting taken away); `outside` 13.9 m short at the kitchen table; coast `cliff_from_sea` and
  `along_cliff` blocked in every build; townland `from_east_gate` a grass slope; college blocks show
  no door at `*.entry` (on-foot review 8 to 10, all known).

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
      Decided (M4 slice 1, composition.md 6.4, R36): no default by origin; the join content is a
      layer ranked above the import's, and R29 orders the rest.
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

- [x] -> M5: Design the kit contract. (M5 slice 3: lib/town/kit.py and the starter wall kit:
      roles by `role:` tag, `params.pitch`, ends inclusive, corner clearance, fillers, joints
      spread or taken by a gap's edge, `open` naming the lengths that close, steps with `sink` as
      footing; test_kit.py; quay's town wall.) Context: the college spike laid real kit bays in runs. Lessons: a
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
      Box kit done (M5 slice 3, examples/market, all three changes measured); the real kit is owed.
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
