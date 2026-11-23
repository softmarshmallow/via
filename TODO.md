# TODO

What is open, filed by layer (`docs/layers.md`): the engine and its contract by chapter, our
libraries, the examples, the loaders, the proof, the record. Each item carries the context it
came from and the lesson learnt, and ends with the slice of work that found it. A done item is
removed, not ticked: the history keeps it.

This file was re-filed on 2026-11-18. Until then it was a log by milestone and slice, and it
also held every done item; that list is `git show ed27503:TODO.md`. An item that says
"was -> M4" was first aimed at that milestone of `docs/plan.md`, which is now past: it is
open, and its place here is its destination.

## The owner's to decide

- [ ] Decide what a concept may say as data. Context: seven library steps do in code what the engine
      has a form for (a run, a set, a scatter, a network, a filled hollow): 3,788 lines of Python
      against 1,707 lines of kinds for all nine libraries (`docs/layers.md`, "What the layout
      shows"). Two archetypes, Grammar and Guidance, have no form in the engine at all. Lesson: a
      kind today can write fields, own a surface and offer mounts; whatever it must place, a step
      places, and each world restates that step.
      (From the revisit, 2026-11-18.)
- [ ] Fit other map examples to the spike model (recreations of well-known game maps) and tune the model.
      (From the list before the rewrite, Spikes; was -> none yet (open; owner to confirm at the M0 sheet whether it joins doctrine section 7 beside row 44).)
- [ ] **The terrain library is a Rust program, not a `viasdk` step** (R19a/b). It
  links `via-num`, the seed derivation and the store, and computes its own noise. Recorded in
  protocol.md 8 and challenge 58 ("no private door" not met). Rule: publish `via-step` and friends
  as a door, or port the library to Python. Meanwhile a one-word configured program's file digest
  is in the step's static key (graph.md 5.1), so a revised binary re-runs basin.
      (From the M2 review fixes, 2026-11-14; was -> owner (doctrine).)
- [ ] Pick the dogfood target: which game, which kind of map, at what scale.
      2026-11-23: the owner added a fantasy map as K11 of `docs/plan-kits.md`, after K10. That
      settles the kind of map; the game and the scale are proposed in K11's design note.
      (From the list before the rewrite, Framing; was -> M4 (`county` proposed in docs/plan.md, the owner's to redirect).)
- [ ] Design reference matching: a bookmark solved from an image (horizon, lens, anchor points), a
      render-beside-reference sheet, and measured differences (palette, horizon, silhouette).
      (From the list before the rewrite, Look and reference; was -> none yet (open; in no milestone, like challenges 9 and 10, and the owner decides at the M0 sheet).)
- [ ] The four things only doing can settle (filtered 2026-11-12). Everything else on this list
      is engineering the rewrite can own. These have no known right answer, or depend on judgement
      or on how agents and people behave:
      1. -> done (R3): What "the same thing" means after a rule changes, so a hand edit can hold on to it.
      2. -> M2 (R5 sets the default; the person judges per pack): Whether placement by rule looks good enough unattended, with real assets.
      3. -> M7: Whether an agent that is not the author of the tool can build and iterate from the
         documents alone.
      4. -> M7: Whether a creator can live with text as the only source of truth. Only real users answer
         this; 1 decides how much it hurts.
      (From the list before the rewrite, Surface and delivery; was -> by item below.)

## Mechanisms: the engine and its contract

### `spec/vocabulary.md`

- [ ] Six keys of the format are written by no example, library or pack: `[lock.<id>]`,
      `max_grade`, `inputs`, `[defaults]`, `engine`, `repeat` (`docs/layers.md`, "What the layout
      shows"). Only cases exercise them. Give each a use in a world, or say why it needs none.
      (From the revisit, 2026-11-18.)
- [ ] -> spec/vocabulary.md: Settle the names flagged in ledger section 25: `part`, `level`, `set`, `portal` and `layer`
      collide with established words; `kind`, `pack`, `deck`, `build` are open.
      (From the list before the rewrite, Framing.)
- [ ] Design the core IR v0 against one tiny example world. Candidate primitives, as tested by the spike:
  - [ ] -> spec/geometry.md: Frame: units, axes, extents, chunk grid, and feature-local frames
  - [ ] -> spec/fields.md: Fields: functions over the domain (height, masks, density, custom)
  - [ ] -> spec/documents.md: Features: typed points, paths and regions with attributes (POIs, roads, zones)
  - [ ] -> spec/assets.md: Instances: asset reference plus transform
  - [ ] -> spec/assets.md: Asset contract: opaque ID plus declared facts (bounds, footprint, origin, per-LOD cost)
  - [ ] -> spec/documents.md: Variants: overlays on a shared base (e.g. seasons)
      (From the list before the rewrite, Core model; was -> M1.)
- [ ] -> spec/vocabulary.md (doctrine section 6: `[look]` leaves the world file): Give `look` an owner. Context: sun, sky, fog, surf, water and rock tints all ended in one
      untyped table only the viewer understood. Lesson: split world facts (time of day, sea state)
      from engine hints, and type both.
      (From the list before the rewrite, Look and reference.)
- [ ] Engine gaps, triaged from the ledger reading (2026-11-12). Kept only where a real request
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
      (From the list before the rewrite, Surface and delivery; was -> by item below.)

### `spec/diagnostics.md`

- [ ] **A traced lint cannot see a feature that failed late.** Late feature errors (e.g.
  `identity.near_none`) are routed to `merge`, so a lint reading that feature is not skipped. Lesson:
  a late error should fail its own `shape:` node, as an early one does.
      (From the M2 integration, 2026-11-14; was -> M2.)
- [ ] `import.seam`'s message says "across the edge" for a step inside the blend band (harbour,
  27 m inside a): say where it is.
      (From M4 slice 2, 2026-11-16.)
- [ ] Engine: `place.keep_out` raised on a step's run member is `diag.internal` ("raised at the
  header, which has no span"); seen with lib/ways parapet in a world with a `keep_out` field. And
  the crossing's own `clear` refuses its parapet: posts on the deck should not read the terrain's
  keep_out (or `clear` stops at the deck's edge). coast has no keep_out field meanwhile.
      (From M5 slice 1, 2026-11-16.)
- [ ] `surface.gap` says "what sets each" only by name: the effect that wins the terrain at the
  end (a lane's `bed` over a door's `sill`) is not named; field provenance at a post is owed.
      (From M5 close, 2026-11-17.)
- [ ] **`formula.outside_field` names the library's line** (Sonnet `crossroads`): an import moved
  past the extent gave the error at `lib:ways/kinds.toml:89:12` ("'link' has a sample beyond the
  extent"), not at the road's end or the import that moved. Name the feature's end that lies
  outside and the change that moved it.
      (From M7 close, 2026-11-17.)
- [ ] **An import's own warnings folded** (both families): a default build of `crossroads` printed
  5 `geom.cell_*` warnings (about 10 infos with `--info`) from quay's own cells, none the parent's
  to fix; Sonnet's printed 9 and the college's `place.*` too. README now says they are the
  import's. Fold an import's own records into one line per code, or mark them as the import's.
      (From M7 close, 2026-11-17.)
- [ ] **Knock-on diagnostics after an error** (both families; Sonnet: `college.arival` also gave
  the world's own lint step's "road `link` not found", since steps still ran): a misspelt
  `starts_at` (`town.gat`) also gave `formula.empty_selection` on the ground's `kind:road` mask,
  and not the `import.exit_unanswered` the fixed build will raise. Suppress what follows from a
  failed feature; say what is pending.
      (From M7 close, 2026-11-17.)
- [ ] **`lay` beside its building's step is still quiet on the console** (on-foot review 2): with
  `lay` in the same layer as the step that returns its building, the console reads "0 errors, 0
  warnings" and no mill is built; the hint is in `diagnostics.json` only, since
  `graph.for_each_empty` is an info (most worlds' optional steps match nothing, so a warning there
  is noise). Warn when a step of the same layer returns that kind, which is known only after that
  step runs; or print a `for_each` step that matched nothing in the summary. Lesson: an info that
  explains a missing building is silence to a newcomer.
      (From M7 close, 2026-11-17.)
- [ ] -> spec/diagnostics.md: Design the compile pipeline and diagnostics: stages, determinism, error and warning shape.
      (From the list before the rewrite, Core model.)
- [ ] -> spec/diagnostics.md: Design the agent loop surface: bookmarks, review renders, machine-readable diagnostics.
      (From the list before the rewrite, Surface and delivery.)

### `spec/documents.md`

- [ ] -> spec/documents.md: **Numbers too.** `[params]` are already a world's declared knobs with defaults. A parent giving
  a param is the same idea and is not possible yet; decide whether `given` and `params` are one table.
      (From the city given its limits, 2026-11-12.)
- [ ] **which layer a change file's added entities belong to** is now asked.
  `[pin.shed] by = ["orchard_trees"]` in a change file is `doc.upward_ref` (added in `world`, the
  scatter in `dress`); example's clear and pin are inline changes in `dress` until it is decided.
  Proposal: a change file may write top-level `layer = "<id>"` for what it adds.
      (From building example, 2026-11-15; was -> owner, documents.md 6.)
- [ ] a `doc.change_adds_near` warning on example's `third_cottage` (`cottage_c` near
  `cottage_a`): a typo guard firing on a numbered family of names is noise; consider not warning
  when the two names differ only in a trailing letter or digit.
      (From building example, 2026-11-15; was -> core (documents.md 7).)
- [ ] 6.7's named places miss name attributes, `follows` and `within` (documents.md 5.1).
      (From M4 slice 2, 2026-11-16.)
- [ ] A layer cannot write a row on a field another document declares (`identity.duplicate_id`),
  and a lower row cannot read a higher layer's features, so atlas paints its roads with a step
  (layers/steps/track.py) that only returns a row. Decide whether a layer may add `then` rows to a
  field declared below it (documents.md 4, fields.md 7); then drop the step. (Atlas pass: the
  question is written with two options, a layer's rows on a lower field at its rank, or the step
  kept, and the first recommended; the owner's to rule.)
      (From M6 slice 2, 2026-11-17.)
- [ ] -> spec/documents.md: Design layers: separately owned documents for one world (terrain, buildings, dressing, NPCs,
      gameplay), compiled and checked together, emitted separately. Context: an NPC is not part of
      the map, yet where it stands, patrols and sleeps is planned against the map. Lessons from the
      spike: one file already mixed rooms, furniture, hero props and lint expectations at 800 lines;
      overlays change a base and are the wrong tool for adding a concern beside it. A layer needs
      its own namespace, references to things in lower layers by ID and frame, a declared direction
      (upper layers may read lower ones, never the reverse), per-layer output the game can consume
      or replace, and per-layer variants (a season for terrain, a quest state for NPCs). Layers
      split by concern and parts split by space; a world can use both.
      (From the list before the rewrite, Composition.)

### `spec/formulas.md`

- [ ] **A path's length has no name in a kind's formula.** lib/town's
  bed fades out over a street's ends with `min(t, 1 - t) * length`, `length` an attribute the
  streets step writes; a routed main way has none an author can write, so its ends keep a round
  cap (now a 1:2 bank, not a wall). On quay the main way's cap at `quay_head` fills the harbour
  floor in a 1:2 cone past the waterfront's reach: `field.effect_cut` at (113.5, 22.5). A built-in
  per-feature `length` (metres) would let both kinds drop the attribute. Lesson: a cap is shape, not
  style; what an effect cannot see along its path it cannot stop at.
      (From building quay, 2026-11-15; was -> core (spec/formulas.md 6.5).)
- [ ] A vector case for a noise `base`: the stream is now `["noise", "feature", <fid>, "surface",
  <name>, "base"]` (formulas.md), fixed in the fix pass; no world stored values under the old one.
      (From M5 slice 1, 2026-11-16.)
- [ ] Decks: a kind default cannot read the world's resolution, so `crown` (1.5, for 1 m posts) is
  restated by a world at coarser posts (city). A formula name for the resolution would let the
  kind say `1.42 * resolution` itself.
      (From M5 slice 3, 2026-11-17.)
- [ ] -> spec/formulas.md: Carry formula attributes into the design (2026-11-12). Context: the `pistol` stress test
      needed `= slide.top + hz` to place 17 parts with no coordinates. It answers derived
      dimensions and most of nested frames, and marks a formula apart from a string. Open: the
      same for positions (`at`, `path`), rotation, and whether it replaces `on` and `along`.
      (From the list before the rewrite, Surface and delivery.)

### `spec/geometry.md`

- [ ] **A way that ends on a ford placed by a selector is said to cross the stream.**
      `examples/realm` (K11): `rindle_ford` stands `on = { of = "uplands", class = "feature" }`,
      the stream a step returned, and both roads end on it, yet each is `geom.unowned_crossing`
      with the stream at the ford's own point (439.748, 236.622). In `examples/ford`, whose ford
      stands `on` its beck by name, the lanes raise none. A produced name cannot be written
      (R3), so a ford on a made stream cannot be named onto it: 2.7's "the end of one named onto
      a point on the other" should hold for a point placed `on` the other by a selector.
      (From K11 of docs/plan-kits.md, 2026-11-23.)
- [ ] A path an import brings is in no crossing check (`geometry.md` 2.7): two imported paths,
      or an imported one and one of the world's, cross with nothing said. Older than R71.
      (From K6 of docs/plan-kits.md, 2026-11-23; the independent review.)
- [ ] **Distance to large regions is slow.** A fill lake from marching squares has thousands
  of vertices; exact `project_region` per post made vale take ~6 min. A box bound now prunes
  segments; a grid of segments per region is the next step if it is still slow.
  M3 slice 4, measured: the kernel's fast paths (nearest vertex first in `nearest`, the wide
  product over significant limbs, winding from the edges that span a row) and `distance`'s member
  pruning. Cold `via build` of the working tree's townland and vale (same lib/ and examples/ for
  both binaries), fresh store per run, release, default threads, Apple M4 Pro shared with other
  agents (load average about 9), 3 runs each, alternating: townland 93.1 / 94.3 / 93.2 s at
  4d331d3 against 27.1 / 26.9 / 26.9 s now (3.5x); vale 39.1 / 38.9 / 38.9 s against 32.6 / 32.7 /
  32.5 s (1.2x). Values unchanged by the paths: `crates/via-geom/tests/exact_fast.rs` and
  `graph.rs` `distance_tests` compare each with the plain scan.
      (From the M2 integration, 2026-11-14; was -> M2.)
- [ ] **Pass spur sides as frontage.** via-model's `area_front` still skips spur
  half-edges (geometry.md 8.2, Spurs: Open in text); drop that skip and add a text case. A dead-end
  street inside a block gets no lots until then.
      (From building quay, 2026-11-15; was -> core (M3 slice 2).)
- [ ] -> via-geom: **A block narrower than `depth` gives slivers** (quay: 17 m blocks, depth 24): both far
  sides lie outside, the two frontages' cuts cross it whole, strips merge and near cuts leave 1 m
  slivers (geometry.md 3.6 step 5, Open). Owed: a far side at the middle (skeleton). Meanwhile
  lib/town keeps `depth` under half its narrowest block. Lesson: the far-side guarantee held only
  between `depth` and `2 depth` wide; nobody had tried narrower.
      (From building quay, 2026-11-15.)
- [ ] **Quay-facing lots stop 12.4 m deep** between the main way and `d175` at `depth`
  24 and 30 alike (and `d35`/`d175` lots about 11 m): no house fits. Not explained; the block's shell is
  plain (108 to 198 by 46 to about 96). Find what bounds them at y 58.4.
      (From building quay, 2026-11-15; was -> kernel or core.)
- [ ] `network::representative` scans every post of an area's bounding box against
  every outline segment; the city's 24 wards (400 m across) made it the top of the profile.
      (From From M3 slice 5 (city, 2026-11-15); was -> M3 core.)
- [ ] geometry.md 2.7: a bridge's line over a sheer face is `geom.unowned_crossing` (monastery 2,
  every change): a deck over a face is no meeting, and a crossing names only the water it crosses.
  A kind listing more than one name in `names`, or a face (a path whose kind lays no way) left out
  of the check: decide (open to the owner). R71 (K6) does not settle it: the deck lands on the
  brink, where the face's own surface is at the deck's height, so by heights the two do meet.
      (From M5 slice 3, levels and sheer faces, 2026-11-17.)
- [ ] -> spec/geometry.md: **Main ways are found in the order written,** and join only by a discount. A network search that
  places junctions is the real thing; this is the "network and faces" mechanism again (R13).
      (From the boxiness measure and the town on simulated land, 2026-11-13.)
- [ ] -> spec/geometry.md: **A path along another feature is a shape the core lacks.** The waterfront is written as a region
  and the tool offsets the water's edge. Lesson: "the bank of X", "the kerb of Y" are intent (R13).
      (From walking townland, 2026-11-13.)
- [ ] -> spec/geometry.md: Design frames that nest: a thing placed in a thing, turned and moved as one. Context: the
      `sidearm` stress test moved six parts by hand to draw one slide back; the same gap stops a
      placed part being turned and a set holding a set. See FINDINGS, "A hard-surface stress test".
      (From the list before the rewrite, Surface and delivery.)

### `spec/fields.md`

- [ ] **PNG field rows without cases**: 16-bit, inverted, orientation, channels, palette, interlaced
  and others in fields.md 4.
      (From the M2 close, 2026-11-15; was -> M3.)
- [ ] -> spec/fields.md: Decide which picture formats and depths a field may read (16-bit PNG works; EXR, tiles for
      large worlds and colour profiles do not), and whether a picture can pass through to the
      engine as a texture. Context: the `relief` world; see FINDINGS, "Pictures as fields".
      (From the list before the rewrite, Surface and delivery.)

### `spec/graph.md`

- [ ] **A diagnostic on a feature key the attributes' key does not hold.** `owners()` in
      `crates/via-model/src/graph.rs` gives a feature's diagnostic to its `attr:` node unless its
      key is in the placing list; `route` and `holds` were not, so an empty selection a route read
      and a dry `holds` sat on a node whose key does not hold the step returns they read, and a warm
      build after the step changed found the attributes "not stable" (cases
      `graph/routed_over_returned`, `graph/holds_warm`). Not surveyed: every other key whose check
      reads the shape, a field or another feature (`surface`, `face`, a name attribute's target).
      Lesson: a diagnostic belongs to the node whose key holds what it was judged from.
      (From the fix plan's F1 review, 2026-11-24.)
- [ ] **Warm builds that differ from clean ones, found by K5's review and older than it.** Each
      reproduces on the engine before K5. (1) The debug check `change entry ... has no cause`
      (crates/via-cli/src/changes.rs) fires when a hand instance is added over a run's member,
      when a hand instance a member had given way to is deleted, and when a pack's `solidity` or
      `sink` edit changes what a scatter gives way to: the entry is right, its cause is missing.
      (2) `height:<instance>` is not stable when a terrain instance raises `surface.through`
      and that surface's level or kind then changes (exit 3). (3) A clean build exits 3 for a
      run of an asset with an unmet mount connector: `surface.unmet` on a member is raised at
      the header, which has no span. (4) After a model is exported again a run's
      `place.gave_way` record is missing or stale in the warm `diagnostics.json`, though its
      instance files are right. (5) A removed member of an `assets` run also lists
      `asset.<its own asset>` as a cause when it was that asset's last instance; an `asset` run
      does not. Lesson: the review's fuzz (edits to levels, bounds, kinds, packs and layers,
      warm against clean, 6,500 steps) found these in an hour where the cases found none; a
      fuzz of that shape belongs in the checks. Mended at K5: a scatter's `held` and the
      `overlap` check now read what their diagnostics are decided by (cases `level_crowded`,
      `r90`).
      (From K5 of docs/plan-kits.md, 2026-11-23; the independent review.)
- [ ] A store keeps a lint's diagnostics across a rebuilt binary: after a fix to step subjects
      the old `diag.malformed` came back until the store was wiped (node keys hold no engine version).
      (From M4 slice 1, 2026-11-16, left when `run` and `set` landed.)
- [ ] -> spec/graph.md: **Let the parent see the part before it gives.** The limits were drawn blind, by coordinates,
  because a given thing must exist before the part does. A part could publish a cheap outline
  (its arterials, its river) ahead of its build. This is the one-graph question again.
      (From the city given its limits, 2026-11-12.)
- [ ] -> spec/graph.md: **Rebuild a part by area.** Any change to what is given reruns every step of the part (60 s for
  the city). Lesson: a step's record covers all its inputs or none.
      (From the city given its limits, 2026-11-12.)
- [ ] -> spec/graph.md: **Let a step say what it reads by name** (these kinds, this field), not only by which layer it
  stands above. Today a harmless edit to the facts layer reruns the model.
      (From simulating the land under stated facts, 2026-11-13.)
- [ ] -> spec/graph.md: **A step's record is one slot.** Building with and without an overlay by turns reruns it every
  time. Key the record by what was read, as `given` already does for parts.
      (From simulating the land under stated facts, 2026-11-13.)
- [ ] **Engine version in the build key.** A changed engine with the same `CARGO_PKG_VERSION`
  replays a stored build (vale kept a fixed `diag.internal` until its store was cleared). Lesson:
  the key must name the engine build, or every engine change during a milestone needs a version bump.
  M3 slice 4: the epoch is raised to 2 for the smoothed course held to the extent (`derived.md` 3
  step 6), so stores built before it rebuild; `epoch guard` covers only the null world, so a missed
  raise is caught by nothing. Until the key names the build, raise `EPOCH` with every change that
  moves bytes.
      (From the M2 integration, 2026-11-14; was -> M2.)
- [ ] **A way that ends at a door** (relief A): `ends_at = "bothy.door"` reads the instance's
  height and loops through the trail's own effect (`graph.cycle`). Proposed: the `height` edge
  only for a read of the mount's height (`graph.md` 2, `mount` row). Not done.
      (From the M2 integration, 2026-11-14; was -> M3.)
- [ ] **`facts:` nodes for every model asset of the pack**, not only those the build uses
  (graph.md 1). Which assets a build uses is known only after placement; the nodes are defined
  before. Harmless (an unused asset's node raises nothing) but each runs on a facts-file edit.
      (From the M2 integration, 2026-11-14; was -> M2.)
- [ ] **Incremental builds recompute every field over the whole extent** (`fields_memo`): a vale
  change costs 4 to 8 minutes though the graph reuses most nodes. The change report's chains were
  worked out per instance (basin 30 min+); now once per producer.
      (From the M2 integration, 2026-11-14; was -> M3.)
- [ ] **A worker step's `exit` is always null** (graph.md 6.3 now says so): a pooled worker exits
  when the build ends, after the record. Lesson: per-call status belongs to the call, not the process.
      (From the M2 review fixes, 2026-11-14; was -> M3.)
- [ ] **A change still costs ~90% of a cold build** (vale move_road 49.6 s vs 54.8 s, after cap
  bounding, no per-object fsync, one placement pass). Remaining: the `checks` lint step (~21 s,
  re-runs whole on any features change), the whole-extent fields memo (~9 s; `distance:kind:road`
  re-runs in all 64 chunks though far chunks are capped constant), `pre_edit`'s fourth placement.
  Lesson: profile before guessing; the biggest cost was paid by every build.
      (From the M2 close, 2026-11-15; was -> M3.)
- [ ] **Giving way resolves outside the graph.** When a returned instance's `ground` kind has
  an effect with `max_change`, `prepare` (build.rs `give_way`) resolves those fields in full at the
  build view, outside the graph, once per round, and re-runs the whole Placer each round, warm builds
  included: keys stay right, but the warm-store saving is lost for such worlds (any step-returned
  instance on lib/land `pad`; quay is not one, its terraces are features). Measured on the gave_way
  case world grown to 2048 by 2048 posts (height `0.2 * x`, release, one change build after an
  unrelated edit): 0.49 s with the pad's `max_change`, 0.45 s without; it grows with the cost of the
  fields the pad writes, so measure again on a world with a returned pad on eroded land. Then move
  the decision into a graph node. Lesson: a decision that changes what is
  placed was put before the graph because placing is; it costs every build its cache.
      (From building quay, 2026-11-15; was -> core.)
- [ ] **A selector's members are read whole, per chunk.** via-model `graph.rs`
  `define_all` gives each field chunk an edge from every entry that reads a selector to every
  member of it (and `shape_members` filters and sorts every feature and made thing per call);
  graph.md 8.1 says only the made things whose windows meet. lib/town's `terrace` reads
  `inside('kind:terrace')`, so terraces are quadratic: `city` with terraces under 777 lots passed
  7.5 GB in 1.5 min; without terraces, 1,992 houses build in 154 s at 4.7 GB. Lesson: a library
  kind that reads its own kind costs the square of its count in the core.
      (From From M3 slice 5 (city, 2026-11-15); was -> M3 core.)
- [ ] **A change costs a cold build.** quay `unrelated` ~100% of cold, `clear_one` 73%, unchanged 64%
  (core report, 3 runs, HEAD 43ea46a). On the m3-sheet run the slowest change is 81% (monastery) to
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
  of HEAD f44991c (a worktree, its own city) against the working tree with all of slice 1, a fresh
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
  HEAD 2cc9657 against HEAD + this patch, same lib and examples, load ~8 from other agents,
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
      (From the M3 close, 2026-11-15.)
- [ ] **Selector members read whole per chunk** (`define_all`, graph.md 8.1): terraces are quadratic,
  so the city has none (190 `place.relief`, doors to 0.5 m off). Then terraces in `city`.
      (From the M3 close, 2026-11-15.)
- [ ] The store kept a step's old `diag.malformed` after the binary was rebuilt: node keys hold the
  engine version string, not the engine's code, so a warm store is not clean across two builds of
  one version. Decide: a build id in `Versions`, or say in graph.md 3.3 that a dev rebuild needs a
  fresh store.
      (From M4 slice 1, 2026-11-16.)
- [ ] The key-clash test covers one small world; run `key_clashes` over every example's plan
  (crates/via-model/src/graph.rs, test-only) when a world adds a new per-item family.
      (From M4 slice 1, 2026-11-16.)
- [ ] Cost numbers of slice 2 were HEAD + the cost patch alone; re-measure quay and city on the
  committed tree (its `prepare` builds every import in `leaves()` and each stratum).
      (From M4 slice 2, 2026-11-16.)
- [ ] Incremental builds save little (review on foot): changes 80 to 123% of cold on quay,
  townland, example, city; example `winter` 90%; city unchanged 25% and peaks above cold (6.4 vs
  5.3 GB). Profile unchanged and `winter` first (`distance`, `write:changes.json` 13 s).
      (From M4 slice 2, 2026-11-16.)
- [ ] townland change runs once took 4 to 35 times cold (1041 s wall, 60 s CPU) under load 9 to 18;
  a second fresh store did not repeat it. Re-time on a quiet machine; if it repeats, record what the
  build waits on. M4 slice 3 re-time (release, fresh store, 1 run each, load 10 to 15, `/usr/bin/time -l`):
  cold 22.7 s wall / 34.9 s CPU, `move_crossing` 23.3 s / 35.0 s, 0 swaps, 0 page faults, 0 block
  I/O: not repeated. The engine's own waits do not lock (the build lock never blocks: read in the code, not measured); a wall that
  far over CPU is a child step waiting (a plain step polls `try_wait`, a worker call
  `recv_timeout`, both up to the 600 s deadline) or memory pressure from parallel builds. If it
  repeats: `sample` the via pid and `ps` its children.
      (From M4 slice 2, 2026-11-16.)
- [ ] **A far edit re-runs every step because its reads are whole-extent** (M4 slice 3 profile;
  graph.md Open). city `move_avenue` (1 run, 97.7 s against 101.6 s cold): all 10 steps re-run on a
  changed answer, not a memo miss: houses_old/houses_rows read every `kind:main_street`,
  houses_down every lot, old_streets `height` over [0,0,801,801]. Steps are 139 of the run's
  node-seconds; houses_down alone 49 s. quay `unrelated` (8.2 s against 8.0 s cold): houses and
  streets re-run on changed `build.features`. Fix in lib/town first (read the district's window
  and its own streets), then per-item houses (5.3), then node-answered reads.
      (From M4 slice 2, 2026-11-16.)
- [ ] city unchanged peaks above cold (6.25 vs 5.05 GB) and `write:changes.json` takes 4.7 s
  (known, the slice 2 cost item).
      (From M4 slice 3, 2026-11-16.)
- [ ] A diagnostic the store codec cannot read back is refused silently
  (`via-graph/src/codec.rs` encode): the record then has no world diagnostics and no null build
  follows. Raise `diag.internal` (run) naming the code and subject instead.
      (From the M4 close, 2026-11-16.)
- [ ] Wide reads make a far edit re-run every step (city `move_avenue`, quay `unrelated`): the
  slice 3 profile item above; city unchanged peaks above cold.
      (From the M4 close, 2026-11-16.)
- [ ] A run's members have no node of their own: their change entries carry the cause (the model's
  deps) with an empty `via` chain (case `floor_warm`, `edge#r0`). Give runs `rule`/`height` nodes.
  (Slice 2: the instance file now reads what lays a run, so files are right; the nodes are still owed.)
      (From M5 slice 1, 2026-11-16.)
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
      (From M5 slice 3, 2026-11-17.)
- [ ] The overlap test reads every other hand instance's `instance` node, O(n^2) edges, as the
  height did before; a grid of hand instances by plan would cut it when a world needs it.
      (From M5 slice 3, 2026-11-17.)
- [ ] city `move_avenue` is still 106 s against 128 s cold (target 40 s). From `sample` of a
  change build (move_bridge, after): key records written one by one on the engine's thread, about
  15 s (`Store::put_key`, `write_once`: exists, two `create_dir_all`, a temp file, a hard link,
  an unlink each; batch them, or write off the engine's thread); trace replays answered on that
  thread, 13 s (`stepwork::replay`); 650 `distance` nodes, 29 node-seconds, most in
  `lineref::nearest`; three `prepare` passes, about 2.7 s each, of which `Placer::select`
  evaluating each `where` (its `slope` resolves `height` over windows, fbm noise) 0.9 to 1.2 s and
  `network::network` 0.5 s (HEAD profile, city into a new directory).
  Done in M6 slice 2 (perf): key records written behind (graph.md 3.2), the ready steps' traces
  replayed on --threads (graph.md 8), a stored blob not written again (protocol.md 4.1), the
  nearest place among a window's or 32 m tile's segments (geometry.md 3.8); city cold 103 -> 64 s,
  move_avenue 108 -> 57 s, move_bridge 93 -> 60 s, every build equal to the before binary's.
      (From M6 slice 1, perf, 2026-11-17.)
- [ ] city `move_avenue` 57 s against the 40 s target, and cold 64 s against 60 s. Left on the
  engine's thread (`sample`, M6 slice 2): the whole model built again per generate rank for the
  steps' views and `for_each` items (`CliPlan::new` -> `ModelPlan::build`, about 10 s, most in
  `fields_memo` resolving whole fields); the three `prepare` passes (about 8 s, most in
  `Fields::eval_where` reached from `make_network`); key reads in `lookup` (1 s). Share the
  strata's work where a rank's returns do not reach it, or evaluate `where` over windows.
      (From M6 slice 1, perf, 2026-11-17.)
- [ ] The full view of `height` at rank 1 read another value at post (0, 0) after city
  `move_avenue` than before, while the built field's post is 14.0 in both (houses.py's field
  listing read that post; it now reads one in its block, `probe`). Find why the stratum's view
  moves there; a step reading a corner post re-runs on any far edit.
      (From M6 slice 1, perf, 2026-11-17.)
- [ ] Change builds as slow as cold: city `hand` 116.5 s against 110.5 cold, `move_avenue` 98.6,
  `move_bridge` 110.3; townland `move_crossing` 27.5 against 26.2; quay `unrelated` 21.8 against
  15.0 (on-foot review, one run each, load 22 to 47). The perf items above.
      (From M6 slice 1, 2026-11-17.)
- [ ] **`diag.internal` on a pack found late** (found in this pass): build a world whose pack is not
  found, then build it again in the same store with `VIA_LIB` set: `instance:<id>/v:build` "gave
  another value when computed again". The library roots are not in the node's key, or the stored
  failure is reused. Reproduce with `spec/cases/assets/unknown_asset_far`'s world.
      (From M7 close, 2026-11-17.)
- [ ] **A build at another depth: 153.1 s with one node run** (on-foot review 4, load 38 to 28;
  85.0 s at lower load): memo 18.2 s and the manifest write 21.0 s; about 113 s not broken down,
  likely writing 15216 reused values into the new folder. Let the whole-build shortcut cross depths,
  or time the writes. Lesson: a reuse that runs nothing can still cost the copy.
      (From M7 close, 2026-11-17.)
- [ ] **A count of memo hits or model builds in the run report** (change review 3): `two_depths_reuse`
  passes on the old binary, so R55's claim is measured, not checked (graph.md 5.3 says so now).
      (From M7 close, 2026-11-17.)
- [ ] -> spec/graph.md: One dependency graph. Context: mounts, `on`, `along` and formula attributes each say "this
      depends on that" and each got its own ordering code; adding mounts broke every bookmark on a
      lane until the order was patched. Lesson: the rewrite needs one graph of references,
      resolved once, with one cycle report.
      (From the list before the rewrite, Surface and delivery.)
- [ ] **Two keys that still miss a read** (found fixing K1's graph edges, not mended). The
  `identity.pin_in_keep_out` record lives on a pin's `height` node, which has no edge to the
  `keep_out` fields. An `overlap` node reads another instance's written `surface` but not that
  surface's owner, so the terrain-against-surface clause of the level test for two hand instances
  can go stale on a warm rebuild. Lesson from K1: a record must live in a node that reads
  everything the test behind it reads; two older holes of this kind (a pin's records, and a
  scatter's `rule` not reading the runs and sets it gives way to) failed warm builds with
  `diag.internal` before R65 and were found only when a new check node read them.
      (From K1's fix pass, 2026-11-20.)

### `spec/authority.md`

- [ ] **A name attribute cannot be left out.** An attribute a kind lists in `names` must name
      something on every feature of the kind: an empty default is `identity.unknown_place`. So a
      library cannot say "this flight lies over that one" or "this doorway is of that row" on
      some features and not others, which is what both of K6's library mends for a false
      `geom.unowned_crossing` needed (the third flight of a stair over its first; a doorway
      ending on its row's frontage). R71 and a row written as a point went round it. An empty
      name read as "names nothing" would let a kind state an optional relation.
      (From K6 of docs/plan-kits.md, 2026-11-23.)
- [ ] A run member's pad past its kind's `max_change` is `authority.max_change` (as written),
      not given way: `Decls::produced_instance` and the give-way rounds know scatters and steps only.
      (From M4 slice 1, 2026-11-16, left when `run` and `set` landed.)
- [ ] **A fade that changes surface along its length.** It needs a paint chosen by `t`, or two effects
  with crossed masks, and the output's limit of eight materials makes each one costly.
      (From the city's exits, 2026-11-12; was -> M4.)
- [ ] A run member's pad past its kind's `max_change` raises `authority.max_change` instead of
  giving way: `Decls::produced_instance` and the give-way rounds know scatters and steps only.
      (From M4 slice 1, 2026-11-16.)
- [ ] R29 among brought entries: the order is now the entry's own effect's `order`, but nesting is
  not compared and the tier's list is topological, so the position is approximate
  (`resolve::merge_brought`); a unit test with two effects of different `order` around one import.
      (From M4 slice 2, 2026-11-16.)
- [ ] -> spec/authority.md: Design authority: per feature, whether the world conforms to it (anchored) or it conforms to
      the world (derived), with tiers in between and a blend apron around every anchor.
      Context: a landmark lake needs an exact shape and things placed beside it; a scenery lake can
      emerge from terrain. Same vocabulary, opposite direction. Lesson: the spike ordered effects by
      declaration order and it was not a real answer (a road regraded a pad). Authority tiers replace
      order; two anchors whose aprons overlap is the case that must error or name a winner.
      (From the list before the rewrite, Core model.)
- [ ] -> spec/authority.md: Design nesting for authority. Context: every building on a terrace conflicted with it.
      Lesson: "the more specific anchor wins" (smaller region applied later) removed eleven false
      errors and also ordered paint. Decide whether that rule is enough or containment should be
      declared ("this stands on that").
      (From the list before the rewrite, Core model.)

### `spec/rules.md`

- [ ] `[pin] set` still `doc.not_yet`; `on = { of = "<run>", near }` (select a member) and the
      mount of a member (rules.md 15, "a lamp on a fence post") are not written.
      (From M4 slice 1, 2026-11-16, left when `run` and `set` landed.)
- [ ] **A level test makes every produced instance depend on every owner of a surface that
      cuts** (R70 as built). A change report may then list a changed floor as a cause of an
      instance that changed for another reason, and a node that tests levels runs again when any
      surface owner changes. Narrow the read to the surfaces whose bound holds the thing's
      place. Also: `asset.unknown` for an entry of a run's `assets` points at the key's line, not
      the entry's column; and the assets of a set's items may be unkeyed against a pack's
      revision as a run's were (a warm build kept a stale `z` after a `sink` changed; mended for
      runs).
      (From K5 of docs/plan-kits.md, 2026-11-23; the implementer's own notes.)
- [ ] Run members are not tested against other producers (`place.crowded`); identity.md case
      33's run half (cut by the extent) owed. The `kitrun` fit lint of rules.md 10 is struck: a
      kit's fit is the library's (R67).
      (From M4 slice 1, 2026-11-16, left when `run` and `set` landed.)
- [ ] **Bodies laid edge to edge that meet by the lattice's rounding are touching** (R69, owed).
      Until the overlap test can tell, lib/building lays no floor tile off the axes. Candidates:
      a tolerance of a lattice unit for two things one step returned; rows of tiles as runs.
      (From K5 of docs/plan-kits.md, 2026-11-23.)
      K8 evidence (`examples/court`, `turned`, 16.7 degrees off the axes): it is not only
      tiles. 40 `place.crowded` of pieces that only abut: a corner post and the first panel of
      the run beside it, a hip and the slope beside it, two hips of a short side, a door frame
      and its post. Every wall and roof of a turned kit laid by posts is told so.
- [ ] **Clumps of different parents and scatters still interpenetrate** (rules.md 15). Members now
  give way to their parent and earlier siblings; a scatter giving way to lower layers' scatters is
  the next question, decided at eye height.
      (From the M2 review fixes, 2026-11-14; was -> M3.)
- [ ] **`run` and `set` are still `doc.not_yet`** though their milestone is M3; example's
  row of houses is written house by house and its lamps, stalls and bench are left out.
      (From building example, 2026-11-15; was -> M3 core.)
- [ ] An overlap on a set's item is now at its set placement's header (case
  `assets/parts_two_hosts_deep`; it was `diag.internal`); other records raised at an item's own
  header (`place.keep_out`, rules.rs) still are: route them through `place::declared_at` too, with a
  case each.
      (From M5 slice 2, 2026-11-17.)
- [ ] **A thing on the ground under a surface 4.5 m up is on one level with what stands on it**
  (rules.md 8.1, 3: a kind surface is the ground where its bound meets the footprint of a thing
  on the terrain; spans then compared from their own datums). K9's first pipe support, a post
  4.35 m high on the terrain, was `place.crowded` with every straight piece of the leg over it.
  `pipe.py` now stands a support on the leg's own surface, its model reaching down; a lamp post
  under a gantry, a car under a pipe bridge, will meet the same. With the item below.
      (From K9 of docs/plan-kits.md, second slice, 2026-11-23.)
- [ ] The level test compares spans from two datums (terrain and floor) as one; a ground floor
  above the terrain by more than a piece's height still clashes. Read the floor's `base` at plan
  time once surfaces settle before placement.
      (From M5 slice 2, 2026-11-17.)
- [ ] Core or spec: a foot placed `on` a path by `t` slides when the path grows longer (`move_bend`:
  101 of the other streets' 102 pieces move), and the moved church foot then meets the main street
  off its line (`geom.unowned_crossing` 1). A meeting place that keeps its distance from one end,
  or an end that names the path it meets (`starts_at = "main_street"` at the nearest point), is owed.
      (From M5 slice 3, market, 2026-11-17.)
- [ ] Stations by distance from one end (doctrine 6, "Open (M5 close)"), with
  `identity/c27_stretch_lengthened`.
      (From M5 close, 2026-11-17.)
- [ ] -> spec/rules.md: Design the producer model: how procedural generators emit the explicit core form.
      (From the list before the rewrite, Core model.)

### `spec/surfaces.md`

- [x] **A record raised on a member of a run cannot be located** (`diag.internal`, exit 3).
  Found by K9's `examples/works` for a run a step returned, and by K9's review for a plain
  `[run.<id>]` in the world's text: `surface.unmet` of a member's mount was raised on a feature
  named after the member. Mended in K9's fix pass: a record about a run's member is its run's
  (`place_subject`, `crates/via-model/src/surfaces.rs`), naming the member's mount; case
  `surfaces/run_unmet`. `lay`'s workaround (a port as its own instance) is taken out again.
      (From K9 of docs/plan-kits.md, 2026-11-23.)
- [ ] **No joint across an import.** A mount of an instance inside an import and a mount of the
  parent's at one point are not joined, and each is `surface.unmet` in its own world, naming
  no other (K9's review, parent `q.m` and `town.p3.m`). An import's meetings are found in the
  import's own build and brought; its asset mounts are not, and an import's `surface.unmet`
  would have to be withdrawn in the parent when a parent's piece answers it. Stated in
  `spec/surfaces.md` 3.3. Wanted: asset mounts brought through the import's frame as its
  surfaces are, and joints found in the parent among both.
      (From K9's review, 2026-11-23.)
- [ ] **A leg cannot end on the piece that stands on it** (`graph.cycle`). A riser stands on the
  lower of two legs (its surface) and its top is where the upper leg starts; the lower leg ending
  at the riser's foot too is a loop of two statements, so the riser's foot has no connector and
  nothing holds the lower leg to it (lib/building/pipe.py rule 3). A meeting that is checked
  and gives no height to either side would not be a loop; today every `ends_at` is read as what
  the path depends on.
      (From K9, second slice, 2026-11-23.)
- [ ] A joint (surfaces.md 3.3, R76) is found at one lattice point only. Two pieces whose mounts
  are a millimetre apart are told the miss by `surface.unmet` and are not joined; pieces turned
  off the lattice's axes round their mounts to the millimetre separately, so a ring of turned
  pieces may not join where it closes on paper. No tolerance is given: R24's gap is exact too.
  The evidence wanted is a kit of pieces at 30 degrees (K9's example, or K10's kit).
      (From K9 of docs/plan-kits.md, first slice, 2026-11-23.)
- [ ] A kind mount takes no part in a joint: two features' mounts at one point with one
  connector say nothing to each other, and each is `surface.unmet` unless a path or `meets`
  answers it. Decided so for K9 (two features meet by a path); reopen if a library's kinds want
  to be set against each other as pieces are.
      (From K9 of docs/plan-kits.md, first slice, 2026-11-23.)
- [ ] `surface.unmet`'s nearest mount looks 2 m in plan and at open asset mounts only. A mount
  that missed a kind mount, or one further off, is told nothing more than before. The 2 m is
  ours, with no evidence yet from a kit with long pieces.
      (From K9 of docs/plan-kits.md, first slice, 2026-11-23.)

- [ ] **A connector that starts on a kind surface is not known as one.** A doorway's start is a
      free end, answered by nothing and checked for nothing (3.4). lib/building's second doorway
      behind a colonnade starts on the `platform` and ends on the room's floor (lay.py rule 16):
      the core accepts it and reports one meeting, at its end. That the walk goes platform to room
      is known only to lib/building's `check`, which reads where the path starts. A start could
      take `starts_at = "<platform>.floor"` as a flight's does; it was not tried, since a doorway's
      kind has an effect on the terrain and a flight's has none. Lesson: found by trying, the
      core needed nothing for two surfaces at one height that overlap (R70).
      (From K8 of docs/plan-kits.md, 2026-11-23.)

- [ ] **Facing for every named place, not only exits.** `onward` works because an exit faces somewhere.
  Mounts and path ends still face nowhere (the same gap as challenge 48).
      (From the city's exits, 2026-11-12; was -> M5.)
- [ ] meets.json's write node now reads the meeting features' shape and attr, what their ends
  name, every instance and height, the facts of assets with mounts, and the terrain under ends and
  under each plan's footprint grown by 5 m (`MOUNT_REACH`, an asset mount off its footprint); its key
  holds the statements of those features, their kinds and every instance. Replace the 5 m with the
  mounts' own points, and add warm-equals-clean cases for a connector edit in kinds.toml and a world
  with the crossing removed (review issue 2 (b), (c); the cases were written in slice 3:
  `meets_connector_edit`, `meets_crossing_removed`; the 5 m reach stays).
      (From M5 slice 1, 2026-11-16.)
- [ ] (M5 close: surfaces.md 7 now says what the command does, no step, and where to ask instead;
  the step runner is owed) `via query` builds with `via_model::build` alone: it runs no generate step, though
  surfaces.md 7 says it does, so a world built by a step answers `meetings` empty (the house of
  `test_lay.World` gave `[]`; the test reads `meets.json` from `via build` instead). Also `relief`
  of a kind surface is not answered by `via query` (a usage error) nor by the protocol.
      (From M5 slice 1, 2026-11-16.)
- [ ] A point feature's `surface` z as a meeting's default mount (3.3 now says so); until then the
  code gives the terrain. features.json gives its surface's z since M5 slice 2.
      (From M5 slice 1, 2026-11-16.)
- [ ] The terrain's cut (surfaces.md 1.3, 4.4 item 4) is not served to a step: reach walks a cut quad
  as ground (a floor with `cut` under a hill). Serve the cut (a field read, or `surfaces_at` leaving
  the terrain out where cut), then read it.
      (From lib/review reach over kind surfaces, 2026-11-17.)
- [ ] -> spec/surfaces.md: Design volumes: bounded 3D fields for caves, overhangs and cliffs. Context: the coast spike
      meshed a sea cliff and cave from a density field whose base is the ground, blended back to the
      terrain at its edges. Lessons: the model extended to 3D with no new concept; but standing,
      water and placement still assume one or two surfaces per column, nothing can be placed inside,
      and whether we hand the engine triangles or the density field is undecided.
      (From the list before the rewrite, Core model.)
- [ ] -> spec/surfaces.md: Decide what the core owns for buildings. Working answer from the spike: levels as frames,
      grounding on a named surface (not only terrain), the threshold seam between a door and the
      ground, and the footprint a building claims. Everything inside belongs to the building domain.
      (From the list before the rewrite, Assets and buildings.)
- [ ] Give a mount a direction, a height and a type. Context: built with position only
      (2026-11-12). A path should arrive square to a door, a stair should land at its height, and
      a road should not end at a window.
      (From the list before the rewrite, Surface and delivery; was -> M5.)
- [ ] **A height in `at` (R65), what is owed.** A record of what each height chose, said when it
  changes against the baseline as `identity.reselected` is for a selection. An import's kind
  surfaces are not candidates. The SDK's `Out.instance` and `Out.feature` write two numbers, so
  a step cannot return a place with a height. No case covers a candidate whose `base` fails at
  the point. In `via query`, which runs no step, a height stands its thing on the nearest floor
  the run does have, or the terrain: not always the floor a build chooses. It goes when the
  query runs generate steps (TODO, From M5 close).
      (From K1 and K2 of docs/plan-kits.md, 2026-11-20.)

### `spec/identity.md`

- [ ] -> spec/identity.md: **Derived geometry needs to be a first-class result.** The river's found course is handed back
  as a guide feature and views stand on it; when no water runs, everything on it fails with four
  unrelated-looking errors. Same need as generated streets.
      (From simulating the land under stated facts, 2026-11-13.)
- [ ] **Consequential errors in basin `cannot_hold`.** With the river stated uphill the step
  returns no course, and the five views standing on it said `identity.near_none` each: one fault,
  six reports. Now one `identity.near_none` at the step listing the five (identity.md 3, "said
  once, at its maker"), so two reports. Left: the generate step knows why it made no course, but
  failing it would skip the lint that names the cause. Lesson: a step needs a way to say "made
  nothing, because" that its readers' selections defer to.
      (From the M2 integration, 2026-11-14; was -> M3.)
- [ ] **Lots renamed in unchanged blocks.** `city` `move_avenue` renames 212 lots (106
  with identical outlines) inside 164 blocks kept with the same name and outline (measure.py
  `lot_changed_in_kept_blocks`); `move_bridge` 16. R13 says names elsewhere stay. Suspect the
  cell key's stretch anchor and ordinal (identity.md 2.3) when a long street loses a junction.
      (From From M3 slice 5 (city, 2026-11-15); was -> M3 core.)
- [ ] Quay's `wedge_house` in `move_hill` is placed by coordinates from that build's houses (R3):
  a view that follows a produced wedge needs a name for it.
      (From M5 close, 2026-11-17.)
- [ ] -> spec/identity.md: **A found course cannot be adjusted except by `via`.** Same gap as hand edits for paths.
      (From the boxiness measure and the town on simulated land, 2026-11-13.)
- [ ] -> spec/identity.md: Decide how hero and interactive things are marked. Context: one painting and one book among
      a hundred dressed props are gameplay objects. Two patterns: explicit annotation in the map
      (free attributes that travel untouched), or the game queries by stable ID or tag and keeps the
      meaning itself. Working position: support both, own neither's meaning. What we must own is
      stable identity, so a hero item survives regeneration of the clutter around it, and a rule
      hook so a studio can check "every interactable is reachable" or "no hero prop is hidden
      inside generated clutter".
      (From the list before the rewrite, Assets and buildings.)
- [ ] Decide how hand-placed things stay valid on generated land. Lesson: the lane on `massif` was
      drawn after looking at one seed's terrain; another seed moves the valley, not the lane.
      (From the list before the rewrite, Surface and delivery; was -> M2.)
- [ ] From spiking the four (2026-11-12; see "Three questions only doing could settle" in
      `spike-final:spike/FINDINGS.md`):
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
      (From the list before the rewrite, Surface and delivery; was -> by item below.)
### `spec/derived.md`

- [ ] a route cost of the course's own grade (direction-aware), so a main way can
  take a fall in switchbacks; a slope-only cost cannot.
      (From building quay, 2026-11-15; was -> core (derived.md 3).)
- [ ] **a path's waypoints by name.** R16 says "the places it passes, by name",
  but `path` takes coordinates; monastery's six turns are typed numbers.
      (From building monastery, 2026-11-15; was -> core (derived.md 3).)
- [ ] Way points by name, left: an inline `{ ref, t, offset }` entry in `path` (today a point
  feature `on` it, then its name); `holds` naming a point feature (derived.md Open).
      (From the M3 close, 2026-11-15.)
- [ ] -> spec/fields.md: Design derived features: features computed from fields (flood fill, contour, flow line).
      Context: the spike's lake was an ellipse forced onto the terrain and needed a dam. Lesson: the
      model can turn a feature into a field but not the reverse, and anything that follows terrain
      (shores, rivers) needs the reverse.
      (From the list before the rewrite, Core model.)

### `spec/composition.md`

- [ ] **Check an answer against the exit's facts.** Today any path answers any exit. Lesson: the facts
  are only useful if something compares them; decide which are core (width) and which a library checks.
      (From the city's exits, 2026-11-12; was -> M4.)
- [ ] **Let a parent offer, not only a part ask.** A world built apart cannot say "I need a river in
  from the north"; it can only say where its own leaves. The join contract should run both ways.
      (From the city's exits, 2026-11-12; was -> M4.)
- [ ] **Put exits on the boundary.** They stand 10 m past it to avoid the blended strip. A place on a
  part's edge should be exact by definition, not by a margin.
      (From the city's exits, 2026-11-12; was -> M4.)
- [ ] **Let hand-drawn ways bend to what is given.** The city's arterials ignore `limits` and `urban`;
  they stay straight and stop dead at the edge. Lesson: once a world takes inputs, anything in it
  written as coordinates is the part that will not adapt. Arterials should be rules too.
      (From the city given its limits, 2026-11-12; was -> M4.)
- [ ] **A part whose ground gives way to the master's.** The city cannot have a waterfront: a seam
  replaces the master's ground and never the reverse. `strait`, `border` and now this all want a
  blend that reads the ground, not distance from an edge.
      (From the city given its limits, 2026-11-12; was -> M4.)
- [ ] **An exit should excuse a step in the ground** the way it excuses water at the edge
  (`part.seam` fired where the river crosses the limits).
      (From the city given its limits, 2026-11-12; was -> M4.)
- [ ] **Try `given` on a second kind of world** (a wood given a climate field, a village given a
  road). One use does not show it is the right mechanism.
      (From the city given its limits, 2026-11-12; was -> M4.)
- [ ] `after` on a layer: still `NotYet(4)` in via-doc's schema; R36 and composition.md 6.5 want
  `doc.unknown_key` with the fixed hint (case `after_dropped`).
      (From M4 slice 1, 2026-11-16.)
- [ ] Composition cases owed (composition.md Conformance, the paragraph before "Changed from the
  spike"): 16 rules with no case, each to be written able to fail.
      (From M4 slice 1, 2026-11-16.)
- [ ] Undeclared exits (R9; composition.md Open): `atlas`'s river at the edge decides between
  `import.exit_undeclared` and nothing.
      (From M4 slice 1, 2026-11-16.)
- [ ] Field matching by role as well as name across imports (composition.md Open).
      (From M4 slice 1, 2026-11-16.)
- [ ] Brought `weights` fields: the parent's paint holds only its own materials (strait: `sand`);
  composition.md 6.2 reads each material's weight. Both islands render as sand. No longer silent:
  `import.weights_dropped` (warning, case `weights_dropped`) since the slice 2 fix pass.
      (From building strait, 2026-11-16.)
- [ ] The seam check counts the import's own steep ground inside the blend band as a step at the
  join (strait's road shoulders fading into the sea, 0.7): say so in 6.6, or measure only the
  part of the grade the blend made (`|w' (h_import - h_parent)|`).
      (From building strait, 2026-11-16.)
- [ ] Every field chunk re-runs on a move (180 of 180 in `move_b`, also for `harbour`): the
  `brought` nodes per chunk (composition.md 12) are owed.
      (From building strait, 2026-11-16.)
- [ ] An import's own steps never run: `import::built` finishes with no step returns and no lints
  (composition.md 3, 12). Case `import_step`, pending. Fix: the import's graph in the parent's,
  ids prefixed `<id>.` (section 12).
      (From M4 slice 2, 2026-11-16.)
- [ ] No `brought:<id>/<field>/w:` nodes and one coarse `joincheck:imports` that reads every node
  (skipped when any node fails, so join diagnostics vanish behind any error); every field chunk
  re-runs on a move (strait `move_b` 180 of 180).
      (From M4 slice 2, 2026-11-16.)
- [ ] A change's chain through an import (`through import`, composition.md 11) and `related` on
  scoped diagnostics are dropped.
      (From M4 slice 2, 2026-11-16.)
- [ ] `file` of `[import.<id>]` resolves from the world folder, not the declaring layer's file
  (composition.md 2 says the layer's): spec edit with its reason, or code.
      (From M4 slice 2, 2026-11-16.)
- [ ] A seed override (`[import] seed`) does not reach attribute formulas.
      (From M4 slice 2, 2026-11-16.)
- [ ] `import.overlap` areas in doubles with convex clipping, not on the lattice (composition.md
  6.8 wants exact).
      (From M4 slice 2, 2026-11-16.)
- [ ] `moved_square` (no `town.step:lay/i:` nodes) waits on import steps running (item above), and
  then needs `for_each = "given:<name>"` (graph.rs and steps.rs accept only `kind:`) and a
  `given:<name>` selector in the step view (`stepview.rs` `select_rows`). Shape found: the alone
  build is eager in `import::built` (`finish_from` with no step returns) inside the parent's
  `prepare`, so the import's step nodes must enter the CLI graph as `<id>.step:...` and the
  `import:<id>`/`brought` nodes read them; a nested `build::run` per import would not give the
  parent's ran set or trust (composition.md Open "Trust for an import's steps"). `import_step`'s
  `a` declares no field `tone`, so its return is `make#tone` and the parent's `tone` is not
  brought: fix that case (declare `[field.tone]` in `a`) when it is un-pended.
      (From M4 slice 2, 2026-11-16.)
- [ ] county (M4 slice 3): given features of a kind with effects cannot be given. `give = { roads =
  ["north_road"] }` into quay's `[given.roads] kind = "main_street"` gave `doc.upward_ref` (given
  rank is below the rank-0 fields its kind writes), `formula.type` (`width` not a number: the
  fill kind's defaults are not applied) and `formula.outside_field` (the given path is not clipped
  to the fill's extent). Fix all three, then give county's road to the town.
      (From M4 slice 2, 2026-11-16.)
- [ ] A given region, a brought region and a `footprint` region keep only their first piece's shell:
  holes and further pieces are lost (`geo_to_parent`, `bring`).
      (From M4 slice 2, 2026-11-16.)
- [ ] Given features do not take the parent's authority (composition.md 8): `GivenFeature` has no
  tier.
      (From M4 slice 2, 2026-11-16.)
- [ ] A given field is read by a model-only `Fields::new` over the first placement: a parent field
  returned by a step or brought from another stratum reads wrong, and each give costs a resolve.
  Model fields only until it reads the graph.
      (From M4 slice 2, 2026-11-16.)
- [ ] A turned import notches sheer ground (strait `move_b`, b at 80 degrees: water inside the
  slipway apron). No check sees it; a distortion measure, and resampling that keeps anchored edges.
      (From M4 slice 2, 2026-11-16.)
- [ ] `moved_square` still fails (no `town.step:lay/i:` nodes): M4's acceptance is not met. See
  the slice 2 items (import steps, `for_each = "given:<name>"`).
      (From M4 slice 3, 2026-11-16.)
- [ ] border: water stands 0.2 to 1.0 m in the walks (2188 posts above 0.05 m; max 1.02 m at
  (432, 364); west_opening blocked by 0.69 m). Each place's water is brought over its footprint as
  that place's ground alone, and the join's rank-1 walk beds lower the ground after it. Moving
  `[field.water]` into layers/join.toml changed nothing (2188 both ways: brought water, not the
  base); a layer cannot add a row to a field of another file (`identity.duplicate_id`). Needs a way
  to dry brought water against the joined ground (a kind effect on `tag:water_level` in the trail,
  or a rule in composition.md 6.2), then a `walk_wet` measure.
      (From M4 slice 3, 2026-11-16.)
- [ ] A step other than a generate step at a join that reads `<import>.<field>` gets "no field",
  not a named refusal (composition.md 6.5 now says only a generate step reads it).
      (From M4 slice 3, 2026-11-16.)
- [ ] `ModelPlan::scoped_values` makes a `Fields::new` per read (each window of `<import>.<field>`);
  unmeasured on a large world. Memoise per plan.
      (From M4 slice 3, 2026-11-16.)
- [ ] county: `merge` rebuilds the imported town on every build (2.7 s of 5.4 s node time on
  unchanged, 63% of cold); `move_town` re-runs 1280 of 1280 chunks. Add an unchanged-percent row to
  county's expect.toml once `brought:` nodes exist.
      (From M4 slice 3, 2026-11-16.)
- [ ] county `move_town`: the quay wall scalloped by the lattice and a staircase band along the
  turned town's edge in join_shore/join_west (paint or region edge on cells, not height: heights
  within 0.2 m over 3 m there). Resample paint with the footprint.
      (From M4 slice 3, 2026-11-16.)
- [ ] An import's lint steps do not run in the parent, and an import's fill pre-build (before
  give) runs none of its steps: decide whether a parent's build owes the import's lints
  (composition.md 12 says not). A step's join records (reading faults of a returned document)
  inside an import are dropped (`build/imports.rs`, `recs`).
      (From the M4 close, 2026-11-16.)
- [ ] The import's model nodes are still built in-process by `import:<id>` (leaves() prepares the
  parent and every import to make the build key): county unchanged is 3.5 s of 15.9 s cold (22%,
  six runs over three stores, release, other agents building), all of it that prepare.
      (From the M4 close, 2026-11-16.)
- [ ] `brought:<id>/<field>` nodes per chunk and the import's graph in the parent's: county
  unchanged pays `merge` (2.7 s of 5.4 s node time; unchanged 56 to 63% of cold over three runs of
  the slice), `move_town` re-runs 1280 of 1280 chunks, strait `move_b` all of its chunks. Then an
  unchanged-percent row in county's expect.toml.
      (From the M4 close, 2026-11-16.)
- [ ] A turned import notches sheer ground: county quay wall scalloped, a staircase band along the
  turned town's edge (paint or region edge on cells), seam 14 pairs at 1.61; strait b's slipway
  apron. Resampling that keeps anchored edges, paint resampled with the footprint, a distortion
  measure. Part done M4 slice 4: the wall cell rule (6.2, lengths only after the review; cases
  `turned_wall`, `turned_tone`) takes county `wall_mid` 203 -> 122 after `move_town`; the seam
  stays 1.607; the 122 are slopes under 2 m a cell read bilinear.
      (From the M4 close, 2026-11-16.)
- [ ] Given features cannot be given (county: `doc.upward_ref`, `formula.type`,
  `formula.outside_field`); a `features` given has no `default` selector though the spec says so;
  a given region's default cannot be a named region. Then give county's road to the town.
      (From the M4 close, 2026-11-16.)
- [ ] Vocabulary the border agent could not say: growth from a part's own rule past its footprint,
  names for an outline's sides, two `follows` on one outline ending on each other, a name
  attribute holding several names, `blend` naming a place (composition.md 6.3 against 4).
      (From the M4 close, 2026-11-16.)
- [ ] `scoped_values` per read, a named refusal for a non-generate step reading `<import>.<field>`,
  `anchored_at` by box containment: the slice 3 items above.
      (From the M4 close, 2026-11-16.)
- [ ] An exit on the outline of a turned import is `import.exit_outside` (hamlet: gate at (196, 0)
  on the vale's south edge, `face = 30`; `face = 0` clean). `inside` takes half a millimetre for the
  frame's rounding; the exit test does not, or the footprint's own turned ring rounds inward. Then a
  case `exit_on_turned_edge`.
      (From M4 slice 4, 2026-11-16.)
- [ ] The river of an import leaves at two edges with no exit and nothing is said (composition
  Open "what leaves undeclared"); hamlet's last seam (0.62) is probably there, not checked.
      (From M4 slice 4, 2026-11-16.)
- [ ] (M7 slice 1: not done, 0.286; a road's paint over an import's ground needs an effect, since
  rows come before every effect and the import's ground enters as effects: lib/ways kinds paint
  nothing yet, by lib/land's convention.) Roads inside a place's outline take the place's paint (`road_painted_in` 0.10): the last
  stretch to a gate is grass. Seams: places meet the plain as straight bare scarps with
  triangular pools; the city's freeways and east street stop about 30 m short of its edge; a wild
  tree fills the view at `join_wild_west`; `on_bridge` stands on grass short of a bare deck.
  Atlas pass: `on_bridge` stands on the deck (t 0.25 of `creek_bridge`, z 4.5), which has its two
  parapet runs (30 posts at 4.44 to 4.53 m, written in layers/ways.toml: lib/ways's parapet step's
  `for_each = "kind:crossing"` would also select every import's crossing); `join_wild_west` looks
  from the corner (the west trail starts at the exit now). Not seen on foot (no sheet this pass).
      (From M6 slice 2, 2026-11-17.)
- [ ] A parent's way does not clear an import's scatter: the wild's trees stood on the atlas's
  west trail. Atlas routes round the wild instead (its footprint leaves out its kept corner, the
  roads meet its exits from there; no tree within a metre of any atlas way). Proposed for
  composition.md (owner's to rule): an import's produced instances (a scatter's, `<producer>#key`)
  give way to the parent's `keep_out` where a layer ranked above the import raises it, as a
  produced instance gives way to `max_change` (authority.md 2.2), with one `import.gave_way` per
  import (count, the worst post); written instances never do (`place.keep_out` as now).
      (From M6 slice 2, 2026-11-17.)
- [ ] Fresh agent (`weir`): an import's `doc.file_missing` names `cli` and "the world file" (the
  import table and its span are the subject); `import.seam` and `import.no_datum` give no lift
  hint (`data.lift_hint`: the median height difference across the outline or the held place);
  no check on the height gap where a way ends at another path kind's end (the bridgehead pattern
  as a library rule, or a core meeting tolerance: owner's); `build.over_budget` names an import's
  asset the parent cannot thin (`data.from`, and a hint to ask the import's change); no
  `build.budget_peak` info to set a budget without a first build; a formula cannot read a
  three-part import place (`doc.not_yet`, composition.md 6.3; done M7 slice 1, core: a `blend`
  reads it, case `blend_path_end`; an import's `blend` was never walked for scoped places).
  Done M7 slice 1 (`messages`), but the last: an import's file faults are on `import.<id>`'s
  `file` with `data.file` (composition.md 2, case `import_file_missing`); `data.lift_hint` on
  `import.no_datum` and `import.seam` (composition.md 5, the median step across the edge; cases
  `no_datum_lift`, `seam`), no_datum now made after the ground is built; `build.over_budget`
  `data.from` and the import's change in the hint (build.md 5.1, case 52); no `budget_peak`: build.md
  5.1 says to read the peak from the manifest (decided, open to the owner). The bridgehead: core
  stays exact (surfaces.md 3.4, Open 4 already gives tolerances to libraries); recommended rule for
  lib/ways' `check`: a way's end within 1 m in plan of another path kind's end with no meeting
  between them is `end_unjoined` (warning) with the plan distance and the height gap (`sample` of
  both surfaces), every path kind with a surface, not only `crossing`. Not built (lib/ways was not
  this slice's); owner's to rule. Built M7 close (`worlds`, decided, open to the owner): lib/ways'
  check over `lane`, `road`, `trail`, `crossing` ends (a deck's height its `deck_z`, a way's the
  surface field), at least one end the world's own; joined through a named point both meet,
  except at a deck; tests in lib/ways/test_steps.py. Atlas runs it: its creek roads ended at the
  deck's points, now at its ends. Lesson: a lint over a parent's view sees its imports' features
  under scoped names and in their own frames; judge the world's own, and build a subject from the
  whole scoped name.
      (From M6 close, 2026-11-17.)
- [ ] **A lift at a named place** (`crossroads`): `lift_hint` is the edge's median; the road's join
  wanted 1.66 where the hint said 0.74, and a moved import needs its lift measured again
  (composition.md 5 now says both). Give the lift at the place a parent path ends on, or a lift
  written as "meet the ground at `<place>`".
      (From M7 close, 2026-11-17.)
- [ ] Design the seam model: what a seam is, when the compiler blends, when it errors.
      (From the list before the rewrite, Core model; was -> M4.)
- [ ] Give every world a declared datum (what its zero is: sea level, or nothing). Context: `atlas`
      placed nine worlds whose edges ran from 0 m to 140 m; each needed a `lift` found by eye.
      Lesson: without a shared zero, joining is guesswork and water levels drift.
      (From the list before the rewrite, Composition; was -> M4.)
- [ ] R9): Let a part's footprint be a shape, and let it declare what crosses its edge (a river out at
      this level, a road out there). Context: in `atlas` every flaw was a straight line: scarps
      400 m long, a river that stops at the edge. Lesson: a rectangle with one blend cannot say
      where a place really ends or what continues past it.
      (From the list before the rewrite, Composition; was -> M4 (exits.)
- [ ] Decide which side yields at a join by authority, not by key. Context: `apron` bends the part,
      `outer` bends the master; `atlas` needed the second, `coast` the first. Lesson: this is the
      same question authority already answers inside one world.
      (From the list before the rewrite, Composition; was -> M4.)
- [ ] A step that runs at a join. Context: the skirt is a mix of heights and leaves bare smooth
      ramps. Lesson: "a custom function" for blending means code handed both sides, not a falloff.
      (From the list before the rewrite, Composition; was -> M4.)
- [ ] Make the master a layer above its parts by default, not below. Context: `border` needed
      `after = "parts"` so a ridge on the join was not blended away (7 m kept 4.1 m without it).
      Lesson: the order was an accident of the spike; one graph of layers and parts should set it.
      Decided (M4 slice 1, composition.md 6.4, R36): no default by origin; the join content is a
      layer ranked above the import's, and R29 orders the rest.
      (From the list before the rewrite, Composition; was -> M4.)
- [ ] Name the sides of a part's outline, and "the sides facing part X". Context: `border` writes
      `wild.edge.2`. Lesson: a number breaks when the outline gains a point.
      (From the list before the rewrite, Composition; was -> M4.)
- [ ] Let a master name a point along a part's path, not only its ends. Context: the west walk in
      `border` starts at a bookmark because no road of the college ends on that side.
      (From the list before the rewrite, Composition; was -> M4.)
- [ ] Let a part's own rule grow past its edge, fading. Context: growth across the join in `border`
      is a second scatter written to match the first. Lesson: the cause lives in the part; the
      master should extend it, not copy it.
      (From the list before the rewrite, Composition; was -> M4.)
- [ ] A master feature that overrules a part's anchored ground only warns (`part.overruled`), and
      never clears the part's instances under it. Decide: error, or authority across parts.
      (From the list before the rewrite, Composition; was -> M4.)
- [ ] `max_step` and the edge checks per side. Context: an outline with sides on the world's edge
      had to raise the limit for every side.
      (From the list before the rewrite, Composition; was -> M4.)
- [ ] Filter when a fine part is sampled onto a coarse master, and say when detail is lost.
      (From the list before the rewrite, Composition; was -> M4.)
- [ ] Find why a part's water body (the city's river) draws no water once it is in a master.
      (From the list before the rewrite, Composition; was -> M4.)
- [ ] Decide whether "part" is one concept with two kinds of boundary, or two concepts. A piece
      blended into the terrain around it (the coast case) meets the master along a continuous seam:
      an apron where fields are blended. An interior behind a door (the station case) meets it at
      discrete portals: a footprint, openings and their heights, and nothing is blended. They share
      a frame, a seed, scoped names and building alone. A cave is both at once (a mouth, and rock
      continuous with the terrain), which argues for one concept whose boundary lists seams and
      portals. Naming to settle: other tools say stamp or patch for the first and cell, sublevel or
      interior for the second.
      (From the list before the rewrite, Composition; was -> M4.)
- [ ] Design parts: a piece of map authored and built by itself, then placed in a master map like an
      asset (position, rotation, seed, parameters), with its seams resolved by the compiler.
      Context: two reference scenes were built as standalone worlds and merged into Coast by hand.
      Lessons: every coordinate was retyped; the coastline was welded by editing a vertex list; one
      height formula per world cannot be composed; global `look`, seed and kind names collided; and
      moving a piece changed its noise. A part needs its own frame and seed, contributions written as
      effects inside a footprint with an apron, namespaced IDs, scoped materials, and declared
      expectations at its boundary (sea here, land joining there) that the compiler checks, the same
      contract as sockets on a modular kit. Possibly the feature that matters most.
      (From the list before the rewrite, Composition; was -> M4.)

### `spec/assets.md`

- [ ] **A roof's corner left open over a bevel is said only by the example.** Since the fix plan's F5
      a pack can ask for a piece's measured `plan` (spec/assets.md 3.4), and `check` holds a
      corner's turn by it (rule 16, F5b). Not yet: `lay` or `check` reading the roof pieces' plans
      over a building's outline and eaves, so Kenney's bevelled hip leaves 81 dm2 of its eaves open
      and only `examples/borrowed`'s measure says so. It needs each roof piece's plan turned and
      placed as `lay` places it, and the union read against the outline moved out by the eaves.
      (From K10's review, 2026-11-24; F5 and F5b of docs/plan-fixes.md.)
- [ ] **A bevelled corner set as a post in a notch.** Where a kit has no inside corner piece,
      `lay` sets its outside corner in an L's notch as a post (reach 0, lay.py rule 6). Kenney's
      corner is bevelled, so its bevel then leaves a notch on the room side against the wall
      block beside it, and nothing says so: rule 16 (`kit_turn`) judges a corner only as `lay` sets
      it at an outside corner. (From F5b's review, 2026-11-24.)
- [ ] **A model at another scale** (K10, `examples/borrowed`). Kenney's Modular Buildings is in
      its own units: a building block 1 by 1 by 0.625, a doorway 0.25 by 0.5, about a quarter of a
      person's size. The contract takes model units as metres and leaves another scale open
      (2.3: "between a declared factor on the asset and a fit to a declared height"), so the
      example is a town a quarter of a person's size, and every person's measure says so (four
      `door_narrow`: a block stands over each doorway within a walker's head). Proposed: a factor
      in the pack (a taxon's `defaults` or the asset), applied where the model is measured, so
      bounds, origin, mounts and `sink` are the scaled ones and no library reads an unscaled
      number; an instance's own `scale` multiplies it. lib/building would then need nothing.

- [ ] **A kit is not a unit a second world can name.** A world has one pack (`pack` takes a
      string: a list is `doc.type`), and two kits' packs joined are `doc.syntax` (a taxon
      table twice) and then seven ids bound twice (`base`, `base_corner`, `door`, `post`,
      `roof_gable`, `roof_slope`, `tile`) and a material coloured two ways. `examples/realm`
      writes one pack from three with a script that prefixes every id (`packs/make_pack.py`,
      115 lines). What a kit states travels with its taxon (R66); where its pieces live and what
      they are called does not. For K10's last review: a pack that includes another, or ids
      scoped by taxon.
      (From K11 of docs/plan-kits.md, 2026-11-23.)
- [ ] A mount with a height (spec/assets.md 12.2), half built at K9 (R75): an asset mount has
  `rise`, and a path end or another instance's mount is held to it. Still owed: a thing placed
  `on` a risen mount (`surface.on_rise` today). It needs an instance that stands at its host's
  surface plus the rise and not on a surface, so grounding, relief and `surface.through` each
  need a rule for it; until then parts lie on flat ground only, a part at two heights is two
  assets, and an accessory's asset states its socket's height.
      (From M5 slice 2, 2026-11-17; K9 of docs/plan-kits.md, 2026-11-23.)
- [ ] `rise = 0` and no `rise` are one pack: the facts file, the manifest and `pack.query` write
  `rise` only where it is not 0, so facts files written before K9 stay byte-equal. A reader that
  wants "this mount states its height" cannot tell the two apart.
      (From K9 of docs/plan-kits.md, first slice, 2026-11-23.)
- [ ] -> spec/assets.md: Design for assets that arrive fast and uneven (2026-11-12). Assumption: creators generate
      models and images quickly and replace them often. Consequences: stating an asset's facts must
      be nearly free (measure, do not ask); odd pivots, facings and sizes are the normal case;
      a kit run cannot demand one exact width; a replaced asset must show what it affects.
      (From the list before the rewrite, Framing.)
- [ ] Design per-variant asset and material swaps. Context: the winter variant moved the snowline but
      left green trees and open water. Lesson: parameter overlays alone do not make a season.
      (From the list before the rewrite, Composition; was -> M2.)
- [ ] Design passable footprints. Context: a gatehouse blocked the path through it. Lesson: solid
      or not is too coarse; gates, arcades and cloister walks are buildings you walk through.
      (From the list before the rewrite, Assets and buildings; was -> M5.)
- [ ] -> spec/assets.md: Carry asset shape into placement. Context: dressing an interior. Lessons: footprints need a
      shape (round or rectangular), assets need to declare their top surface and wall-mount side,
      and "against a wall" must respect the wall's curve. Each was patched in the spike after the
      compiler reported a false or missing error.
      (From the list before the rewrite, Assets and buildings.)
- [ ] -> spec/assets.md: Draft the asset contract from the city study (`docs/research/Modular city asset contracts.md`,
      section 5): a written front and up rule, module per axis apart from bounds, a pivot rule per
      kit checked against the model, a kit table of roles and variants, connectors, surfaces and
      clearances. Lesson from studios: the kit is cheap to build and expensive to change.
      (From the list before the rewrite, Surface and delivery.)
- [ ] Decide on a lock of each asset's contract facts, diffed on every compile, so a revised piece
      names every instance it affects. No studio source describes a way to do this.
      (From the list before the rewrite, Surface and delivery; was -> M2.)
- [ ] Idea (2026-11-12): own the asset contract and ship a zoo with it: query a pack by any fact
      from the command line or as data, lint the pack itself, and lay matches out as a world any
      loader can show. Context: a crude `python -m worldc.zoo` did all three in 70 lines, with no
      renderer of its own. Lesson: a pack is already a database; what is missing is the question
      mark. Corrected the same day: managing assets here means a taxonomy, not storage. Added
      `taxon` and `labels` on an asset, a tree view, and `@taxon/path` in a kit run so a world
      asks for a role and the pack answers. Still to design: who defines the tree (the user;
      we ship the mechanism and a starter), selection everywhere a list of assets is taken, and
      usage (which worlds use an asset, which assets nothing uses). Out of scope: storage,
      versioning, import and conversion.
      (From the list before the rewrite, Surface and delivery; was -> M2.)
- [ ] Decide whether "a thing with parts" is one interface at every scale: facts, mounts, states,
      options, for a pack asset, a kit piece and a placed map alike. Proposed in
      `docs/design/ways-and-assets.md`. It would merge `part`, the asset contract, variants and
      overlays under four words. Needs mounts with a direction and a type.
      (From the list before the rewrite, Surface and delivery; was -> M5 (open).)
- [ ] -> spec/assets.md: Make a mount the asset contract's connector. Context: three accessory files attached to
      named mounts with no change to the host. Lesson: a socket is a named place derived from the
      host, and the thing attached refers only to it.
      (From the list before the rewrite, Surface and delivery.)

### `spec/build.md`

- [ ] A material's `file` and `repeat` do not reach the build. `build.md` 7 says the manifest's
      `materials` hold them when written; only `colour` is. No code under `crates/via-cli/src`
      names `repeat`, and `crates/via-model/src/pack.rs` only checks that the two come together.
      Context: found by a reader checking the ledger's claim about ground materials, then seen in
      the code; not confirmed by a build. Either write them, or change the chapter.
      (From the revisit, 2026-11-18.)
- [ ] -> spec/build.md (doctrine section 6: every field is exportable): **`atlas` exported no water and nothing said so.** A master with a `water` field and a viewer with
  none is silent. Lesson: a field every loader needs should not depend on a list someone forgot.
      (From the city's exits, 2026-11-12.)
- [ ] **build.md 12 vs the null vector**: the id digests the canonical JSON without its final
  line feed; build.md should say so. **ford's pack for build.md 29/32** (stand-ins on a pack with no
  model asset) needs a ruling; `c29_flags_in_key_m2` stays pending.
      (From the M2 close, 2026-11-15; was -> owner.)
- [ ] A point feature's surface is not in the feature list (no `z`, no `surface`): reach stands a
  point on the lowest open surface at its nearest open post. A point `on` a deck over dry ground
  stands on the ground under it. Give the feature list `z` (features.json has it), or a `surface`.
      (From lib/review reach over kind surfaces, 2026-11-17.)
- [ ] No case for an import's `chunk` and `[budget]` applying to its own build only (build.md 5,
  5.1); no example writes `[budget]`, so no over-budget or waived chunk was seen on a world.
      (From M6 slice 1, 2026-11-17.)
- [ ] over_budget counted from placement before the per-chunk nodes run (target 30% of cold): not
  reached at M7 close (`core`, time box).
      (From M7 slice 1, 2026-11-17.)
- [ ] -> spec/build.md: Lift the limit of eight ground materials, or make it per chunk. Context: `atlas` had eleven
      and renamed three (`ground_as`), so city asphalt is drawn as rock.
      (From the list before the rewrite, Composition.)
- [ ] -> spec/build.md: Decide compile-time export versus runtime loading.
      (From the list before the rewrite, Surface and delivery.)
- [ ] -> spec/build.md: Design the handoff representation: realisation classes (unique, repeated, field-driven, static),
      spatial partition, asset facts for culling and detail levels, stable identity.
      Context: the spike's first output was one flat instance list per asset and ran at 12 fps;
      grouping by chunk plus engine-side culling took it past 110. Lesson: we don't own draw calls, but
      we do own the structure that makes culling and streaming possible.
      (From the list before the rewrite, Surface and delivery.)

### `spec/protocol.md`

- [ ] The terminal does not print the last 40 lines of a failed step's log (protocol.md 6.2 says it
  does; code and spec disagree since M1). The `protocol.raised` hint now names where the log is
  (`<out>.run.json` `steps[].log`, `<store>/objects/<2 hex>/<digest>`); print the tail, or a
  `via log <step>` command.
      (From M4 slice 4, 2026-11-16.)
- [ ] spec/protocol.md 3.3 now says `sink` comes as f64 metres (the code's answer); `max_relief`
  is not served by `pack.query` at all (stepview.rs has no row): serve it or say so.
      (From M5 slice 3, town kit, 2026-11-17.)
- [ ] -> spec/protocol.md: Design one operator interface that built-ins and user code both use (inputs, outputs, seed,
      determinism). Context: lake shaping could be a hand polygon, an expression, or a simulation.
      Lesson: the escape hatch must enter before validation, never after, so nothing bypasses the
      seam checks.
      (From the list before the rewrite, Core model.)
- [ ] -> spec/protocol.md: Design lint rules as plugins over a read-only view of the compiled world. Context: reach was
      written into the spike's compiler; a studio will have its own rules ("a barrier at a cliff
      edge must be backed by real fence objects"). Split three tiers: structural checks the compiler
      needs to be correct (seams, cycles, conflicts) stay in core; opt-in rules we ship (reach,
      budgets) use the public rule interface; user rules use the same one. Core owns the query
      surface, the diagnostic shape, rule configuration and waivers, and knows nothing of "fence" or
      "player". Rules read declared facts, never meshes, and only emit diagnostics. Consider a
      declarative form for the common pattern "every X must have a Y within d".
      (From the list before the rewrite, Checks.)

### The command (`crates/via-cli`)

- [ ] `via build` asks once per new `--change` to be trusted although every step is trusted and
  unchanged (hamlet trip 11, README "and once per --change"): decide whether a change that adds no
  step needs trust.
      (From M4 slice 4, 2026-11-16.)
- [ ] `via query` runs no step (surfaces.md 7 says so now): a step runner for `surfaces_at`,
  `sample` and `meetings` over a laid building, and `relief` over a kind surface.
      (From M5 close, 2026-11-17.)
- [ ] **`via build --help`** (Sonnet `crossroads`): a subcommand's `--help` is not understood;
  `via --help` lists every flag.
      (From M7 close, 2026-11-17.)
- [ ] **`via step commit --check`**: write nothing, exit 1 when a folder would change (change review
  5, R54's cost): the only staleness check left for a program changed alone. `basin`'s result had
  not been made again since M2. Lesson: an exemption from a key needs its own check beside it.
      (From M7 close, 2026-11-17.)

## Concepts: our libraries

### `lib/land`

- [ ] **`uplands` traces its stream over its own cells.** `derive.descend` and `derive.spill`
  read fields of the build; a step's own land is in none while it runs. Lesson: a library that wants
  the engine's line over its returned land needs a second step above it.
      (From the M2 review fixes, 2026-11-14; was -> M3.)
- [ ] **Land look not finished.** massif: new `uplands` params (`channel`, `creep_steps`) bend the
  streams, but rocks fell to 0 (weather sees about tan/√2 of a diagonal slope), expect.toml not
  re-recorded, uplands docstring lacks the two params. basin: dress/land edits unbuilt (woods
  expected ~1,000), mouth and straight course over the shelf untouched. vale after move_road: not
  started. A wrong `role:surface matches 0 fields` error hides a road end outside the extent.
      (From the M2 close, 2026-11-15; was -> M3.)
- [ ] -> lib/land or a kind: **A wall eased over a few metres is walkable obliquely** along a row of posts
  nearly parallel to it (monastery's gorge: 3 m and 0.6 m eases both leaked). Sheer faces are steps
  from post to post and draw stair-stepped; a cliff or gorge kind that says "sheer" should own this.
      (From building monastery, 2026-11-15.)
- [ ] Design shape character, not only shape position: profiles and cross-sections for cliffs,
      stacks and caves. Context: compositions matched the references in one pass; "barrel versus
      weathered stack" could only be nudged with noise amplitudes.
      (From the list before the rewrite, Look and reference; was -> M5.)

### `lib/terrain`

- [ ] **Rectilinear drainage.** At 12.5 m cells on an even tilt, streams follow the grid. Try a rougher
  starting surface, a coarser model with finer detail added after, or routing that is not D8.
      (From simulating the land under stated facts, 2026-11-13; was -> M2.)
- [ ] **Use what the model knows about the river.** Width and depth from discharge, a wandering line,
  water in the tributaries. The step ignores all of it and draws a ribbon.
      (From simulating the land under stated facts, 2026-11-13; was -> M2.)
- [ ] **The two-ends course is a lattice walk.** Replace with any-angle routing or smooth harder.
      (From simulating the land under stated facts, 2026-11-13; was -> M2.)
- [ ] **The Rust model's defaults assume 200 m cells and uplift.** At game scale four numbers were
  found by trial. Ship presets per scale, or derive them from the cell size.
      (From simulating the land under stated facts, 2026-11-13; was -> M2.)
- [ ] Map: the river narrows from about 25 m to 10 m at the city's south edge, the mouth is a
  stepped blob (M7 close, `worlds`: the course is 10 m wide, one width from source to mouth;
  `width = 30` made the facts lint lose the water in a hollow at the city's edge, (994, 322),
  `step.facts.river_lost`, an error, so reverted; the blob is the terrain step's groove eroded
  under the sea's 2 m and filled by the sea row, 60 to 100 m wide from y 216 down. Owed: the course
  taking the city's river's width where it leaves it, or a hold at the edge (lib/terrain). The
  city avenue said to dead-end past the outline was not found: every city way ends at its limits,
  60 m inside the outline; six of its gates face the plain unanswered); test_minimap's floor check assumes station (atlas has no floors); no atlas
  minimap image, the minimap is in no viewer.
      (From M6 close, 2026-11-17.)
- [ ] **Rectilinear tributaries at 12.5 m cells** are hidden in townland by not painting them.
      (From the boxiness measure and the town on simulated land, 2026-11-13; was -> M2.)

### `lib/water`

- [ ] **A weir a view can see** (Sonnet `mill`): lib/water's `weir` writes ground and water only,
  so a bookmark toward it shows running water; a crest mesh (a kind surface, as a crossing's deck)
  or a pack asset on its line.
      (From M7 close, 2026-11-17.)
- [ ] **An upgrade note for a library that starts shipping a kind** (on-foot review 3): Sonnet's
  `mill` declared its own `weir`; on this tree it is `identity.duplicate_id` with lib/water's. The
  message is clear, but the hint could say which library is newer, and a changelog of shipped kinds
  would say it before the build does.
      (From M7 close, 2026-11-17.)
- [ ] **Bends are a jittered sine.** Good enough to stop the river reading as a canal; a real meander
  model (or the terrain engine at a finer cell) is the proper source.
      (From walking townland, 2026-11-13; was -> M3.)

### `lib/ways`

- [ ] -> lib/ways: **`deck_z` is typed from `via eval`**; a moved crossing must rewrite it. Read the rims'
  ground at the ends (`under` at the ends, or a default from the ends' heights).
      (From building monastery, 2026-11-15.)
- [ ] `city`'s interchanges are at-grade plates and its freeway lies on the ground (lib/ways
  `interchange`, `freeway`: stand-ins); ramps, levels and the crosstown viaduct need kind surfaces.
      (From From M3 slice 5 (city, 2026-11-15); was -> M5.)
- [ ] -> lib/ways: the crossing's head hold is a half disc behind each end, not the arriving way's
  corridor (a kind cannot tell what arrives); on a hillside it cuts a bowl round the head (monastery).
  A per-end corridor needs the arriving way's line in the formula (core: a path end's frame).
      (From From M3 slice 5 (ways, 2026-11-15).)
- [ ] -> lib/ways: a way arriving below `deck_z` with a cut-only bed (lane) still sags behind the ramp
  (example `lane_west`: 0.47 m below the deck 7 m back). The lane's bed could fill near a named end.
      (From From M3 slice 5 (ways, 2026-11-15).)
- [ ] -> lib/ways or core (R18): **the church path cannot reach the door.** Ending it 2.5 or 3.8 m short
  of `church.door` puts the church in the lane's `keep_out` (`place.keep_out`); `ends_at =
  "church.door"` is `doc.upward_ref` (the path is in `world`, the church in `hamlet`). It stays
  7.5 m short. Lesson: a way that ends at a door must own that meeting, not be kept off it.
      (From building example, 2026-11-15.)
- [ ] A railway (row 36) is in no v1 world.
      (From the M3 close, 2026-11-15.)
- [ ] `lib/ways` `slipway` is a point with no direction: a square platform with a 2:1 face, not a
  ramp into the water. A way end with a facing, or a path kind, decides it.
      (From building strait, 2026-11-16.)
- [ ] A crossing's `deck_z` is a number measured from a build (hamlet, lib/ways README): a deck
  height from the ground at its ends.
      (From M4 slice 4, 2026-11-16.)
- [ ] lib/ways: `[kind.crossing.surface.deck]` in place of the deck effect once meshes and the cut
  land; a parapet run; a lint that water passes beneath; bookmark walks along the deck. Then rebuild
  townland and example, `measure --check`.
      (From M5 slice 1, 2026-11-16.)
- [ ] City levels: no slip ramps from the freeway on the ground (a diamond), a loop lands 7 m
  beside the arterial with no merge, no piers or deck edges or parapet; the viaduct climbs at up to
  6.1 % in 300 m. `z1` of each ramp and the viaducts' `z_start`/`z_end` are typed from a build's
  ground (as `deck_z`): a moved freeway end or arterial leaves them (Open 9 again).
      (From M5 slice 3, levels and sheer faces, 2026-11-17.)
- [ ] lib/ways `interchange` now requires `level` (was the ground read): a 56 m plate for the
  ramps' feet took 55 rows houses; the feet hold their own ground (`ramp.foot`) outside the plate
  instead. A plate that is not a disc (only where ramps land) would let them land on it.
      (From M5 slice 3, levels and sheer faces, 2026-11-17.)
- [ ] Parapets on monastery's, city's and townland's decks (M7 slice 1: monastery and city run it; townland not yet) (only coast runs lib/ways `parapet`),
  or say in each README why not (on-foot review 6).
      (From M5 close, 2026-11-17.)
- [ ] **Bridgeheads are raised by a step writing `height`.** A crossing's abutment should be a kind with
  an effect, so the raise is visible in the document and obeys authority.
      (From walking townland, 2026-11-13; was -> M3.)
- [ ] **The deck's parapet is stepped boxes** and the deck is 36 m long over a 13 m river (it laps both
  bridgeheads). A deck with wing walls and a sloped parapet is asset work in `worldc/decks.py`.
      (From walking townland, 2026-11-13; was -> M3.)
- [ ] Decide how junctions are modelled. Context: in the `city` world two ways crossing at grade
      each grade the ground in turn, and a deck's ends are lined up with their roads by hand.
      See `docs/design/ways-and-assets.md`, "Open".
      (From the list before the rewrite, Surface and delivery; was -> M3.)

### `lib/town`

- [ ] -> lib/town (s4 town pass): **Doors level with the footway, stepped rows.** Done: terraces at the
  footway (houses.py rule 8), stepped, aprons; quay doors within 0.3 m 10 -> 103 of 125. Left: a raised
  floor (12 rows, rule 7) has no front steps; a party-wall step reads as `place.relief` (8) because the
  step falls in one post cell. Corner lots: 60 still empty, `fallback` (rule 2b) fills none in quay; a
  corner house facing both streets is owed. Lesson: a level read from the right place (the footway, not
  the ground under the house) fixed 90 doors; the measure from the walk named it.
      (From building quay, 2026-11-15.)
- [ ] -> lib/town: houses.py rule 7 (was 6) predicts the pad's `max_change` on a 2 m lattice so a lot beside a
  deep cut stays empty; the move_hill error it hides was `authority.max_change` 3.15 m at (109, 99.5).
  The core's `authority.gave_way` landed (M3, after slice 2; authority.md 2.2) for returned instances with
  `ground` only: houses grounded one by one can drop the prediction and `max_cut`. Terraces are
  returned features, which stay `authority.max_change`, so for them the prediction stays.
      (From building quay, 2026-11-15.)
- [ ] -> lib/town: on-foot slice-2 items still open: cross street grade limit, banks over 35 degrees
  (cause not found), s0's kink in move_hill, the gate bowl, the main way's grade onto the quay and
  its bed raising the harbour floor (needs a path `length` in formulas), `way_water` still looks
  over a grey bump. Done in the town-lib pass: setback by keep_out (the S-bend house), door aprons
  (fronts on the footway), wall face paint (stone), `wall_deck` aimed along the deck, quay `bank = 3`
  (the anchored deck fade overrode the quay row's terraces: 18 `place.relief`).
      (From building quay, 2026-11-15.)
- [ ] -> lib/town: **The quay reads as an earth apron on foot**; no wall face seen from eye height; main way
  and street `d…` run 9 m apart down to it. Only 30 of 178 lots hold a house (curved, shallow lots).
  Partly done (town-lib pass, 2026-11-15): a down street within `spacing / 2` of a main way is not
  laid (`d105` gone); `lot_depth` keeps the lowest contour `2 * lot_depth` from the shore (a scratch
  quay with `lot_depth = 24`: 5 houses within 30 m of the quay, was 0); houses take the largest
  asset that fits (3 `house_wide`, was 0). The apron and the wall face are not touched.
      (From building quay, 2026-11-15.)
- [ ] -> lib/town: **Thinning is global**: Douglas and Peucker over a whole contour keeps other vertices when
  its far end moves, so a lot near the middle keeps its name but changes shape and loses its house
  (quay `unrelated`: 1 house west of 200 m). Thin per stretch between junctions, or snap kept vertices.
      (From building quay, 2026-11-15.)
- [ ] -> lib/town: **A contour that meets a main way running beside it ends there for good** (quay
  `move_hill`: `s0` ends at x 140, the east half has no streets). Continue past the way on the other side.
      (From building quay, 2026-11-15.)
- [ ] -> lib/town (`enclosure`, new, stand-in): **the wall is ground**, as the deck is; a wall as an asset
  run or a surface, a gatehouse at the gate, is owed with kind surfaces (M5).
      (From building monastery, 2026-11-15.)
- [ ] -> lib/town: quay's `d35`/`d175` still take 0.34 m a metre in their first 2 m: the deck's land fade
  (`bank` 3, a `mix`) is concave under the `ends` band. Make the land fade a hold at `end_grade`.
      (From From M3 slice 5 (ways, 2026-11-15).)
- [ ] -> lib/town: waterfront `end_fade`, `rise` are fractions of `t` (a path length has no formula name);
  metres would not change with the line's length.
      (From From M3 slice 5 (ways, 2026-11-15).)
- [ ] -> lib/town: **junction plates with 1:2 shoulders.** Where two street beds meet, the owner of the
  crossing band holds a level plate and the other way climbs its `bank` at 1:2 (quay `high_street` on
  `s0` g1 0.50 at g10 0.21; townland `east_road` off the market 3.5 m over 8 m). Let the network node
  own the meeting (R18) with the crossing's head hold (`approach` 0.1), and a named place kind own
  the market's plate. Lesson: two beds that each hold level make a step where they hand over.
      (From the M3 slice 5 review on foot, 2026-11-15.)
- [ ] -> lib/town: the door still stands about 0.5 m further back than the 0.5 m fade (mount depth,
  0.5 m push steps, the 2 m city posts). Set the front from the frontage street's own edge instead.
      (From the M3 slice 5 review on foot, 2026-11-15.)
- [ ] -> lib/town lay_grid: two grid corners on the outline still miss by 1.0 m (downtown `c1`/`s2`) and
  2.3 m (rows `c2`/`s0`); snap grid ends the same way.
      (From the M3 slice 5 review on foot, 2026-11-15.)
- [ ] -> lib/town: monastery `approach` steps -0.25 / +0.30 m in its last metre at the `gate`: the yard's
  mean level does not meet the way's bed; the gate, a named point, should own it with an approach hold.
      (From the M3 slice 5 review on foot, 2026-11-15.)
- [ ] **Junction plates with 1:2 shoulders** (high; above, "review on foot"): the node owns the
  meeting with an approach hold; a market place kind owns its plate. Worst on foot in every town.
      (From the M3 close, 2026-11-15.)
- [ ] Door ~0.5 m further back than the 0.5 m gap (quay, townland median 1.05 m behind the footway's
  back edge, city 1.55 m): set the front from the frontage street's own edge.
      (From the M3 close, 2026-11-15.)
- [ ] `raise_max` 0.3 (default) loses the hill-street rows (quay 125 -> 102 houses on the streets
  agent's tree); 1.5 keeps them with doors up to 1.5 m above the footway and no steps. -> owner.
      (From the M3 close, 2026-11-15.)
- [ ] Front steps for raised floors (no room in the 0.5 m before the footway; an asset or a stair).
      (From the M3 close, 2026-11-15.)
- [ ] Streets down to a waterfront (`d*`) have no grade limit (quay `d175` 25%); `d35`, `d175` take
  0.34 m a metre in their first 2 m (the deck's concave `bank` fade).
      (From the M3 close, 2026-11-15.)
- [ ] lay_grid corners miss by 1.0 and 2.3 m (city downtown `c1`/`s2`, rows `c2`/`s0`).
      (From the M3 close, 2026-11-15.)
- [ ] Monastery gate lip (-0.25 / +0.30 m): the gate, a named point, owns the meeting.
      (From the M3 close, 2026-11-15.)
- [ ] The waterfront `ends` hold reads `kind:street`: a world with a waterfront and no streets gets
  `formula.empty_selection`.
      (From the M3 close, 2026-11-15.)
- [ ] Monastery wall: lib/town kit.py would give corners and end piers, but its gaps come from
  keep_out only: the approach's verge opened 15 m at the gate (tried; the run's span gives 5.1 m).
  A gap by place (a gate point) in kit.py, then move the wall to it. The run's bays overlap at
  the corners.
      (From M5 slice 3, levels and sheer faces, 2026-11-17.)
- [ ] lib/town houses.py: rule 2c pairs only lots no row fits by geometry; a pair whose rows fit
  but are dropped by keep_out is not tried (a second batch, or corner tries in rule 4's). 7 pairs
  found leave no corner house clear (most beside the main street's 5.25 m keep_out): a smaller
  corner asset or a chamfered corner piece.
      (From M5 slice 3, town kit, 2026-11-17.)
- [ ] lib/town kit.py: quay's wall never cuts a bay to fillers (`kit_split` 0, the slope is 0.15);
  rule 6 is tested only in test_kit.py. A world with a run up a steeper slope (monastery wall,
  townland) should show it. After `unrelated` the east wall has no gap where s0 and s1 end 7 m
  inside the district (gaps come from keep_out only): a gate in the wall at a street's end is owed.
      (From M5 slice 3, town kit, 2026-11-17.)
- [ ] lib/town kit.py: one piece per role (the first id bytewise); the spike's rows mixed a shop and
  a tall shop by weight. Variants by weight, seeded by place, are owed for a street that does not
  read as one long block.
      (From M5 slice 3, market, 2026-11-17.)
- [ ] lib/town kit.py: a row on the outside of a bend under 10 degrees per vertex (a round join of
  the follows offset) opens one joint to 1.1 m (`max_joint_cm` 113); a wedge filler, or a corner by
  the turn over a piece's length rather than per vertex, is owed.
      (From M5 slice 3, market, 2026-11-17.)
- [ ] The kit param rename `ends` -> `caps` (decided, open to the owner): if the owner prefers
  another word, market's two runs and test_kit change with it.
      (From M5 close, 2026-11-17.)
- [ ] quay, townland: houses per block as in city (quay's houses are 7 s of `unrelated`).
      (From M6 slice 1, perf, 2026-11-17.)
- [ ] **Dead ends tripled once districts arrived (10% to about 35%).** Grid streets are cut off where a
  ward ends. The street tool should end a street at a junction or not lay it. Found by the measure.
      (From the boxiness measure and the town on simulated land, 2026-11-13; was -> M3.)
- [ ] **Local streets and lots ignore the land.** Only main ways are routed. 163 of townland's 166
  warnings are buildings on slopes. Either the lot tool reads slope, or a lot levels its ground.
  Started (lib/town, M3): local streets follow contours above `steep`, houses take `ground = "pad"`.
  Not yet built on real land: needs the engine's `[network]`/`[cells]` first.
      (From the boxiness measure and the town on simulated land, 2026-11-13; was -> M3.)
- [ ] **lib/town has only fake-engine tests.** Build a world with it (quay) once the core parses
  `[network]` and `[cells]`; check every lot holds a house facing a street, on foot (R17). Also run
  `lib/*/test_*.py` in CI: today no workflow runs the library tests. Lesson: a step written against
  a contract is unproven until the engine answers it.
      (From the boxiness measure and the town on simulated land, 2026-11-13; was -> M3.)
- [ ] **lib/town streets: open shapes.** Cross streets run straight down the fall (steep on steep
  land: no zigzag or stair yet); long streets that converge in flat ground can cross; a contour that
  spirals off a summit is dropped, not ended at a junction.
      (From the boxiness measure and the town on simulated land, 2026-11-13; was -> M3.)
- [ ] **More buildings on slopes after the embankment (155 to 193 warnings).** The new frontage along
  the banks and the raised bridgeheads add lots on graded ground. Same fix as "lots ignore the land".
      (From walking townland, 2026-11-13; was -> M3.)
- [ ] **35 dead ends remain, 9 of them stubs.** Five are the gates at the map's edge. The rest are
  thinned fringe streets longer than the join distance. Decide whether a fringe lane is meant.
      (From walking townland, 2026-11-13; was -> M3.)
- [ ] From the city as intent at three strengths (2026-11-13; FINDINGS, same title; challenge 57).
  - [ ] -> M4: Main ways belong to the parent. Context: the city's arterials are its own, so `atlas`
        restates four axes by coordinates. Lesson: the anchored skeleton is handed down like the
        districts are; a fill should own nothing a parent would want to speak of.
  - [ ] -> spec/geometry.md: A network and its faces, and cells in a region, in the core. Context: streets, wards and
        lots exist only inside two tools; a clear did nothing, a pin stood inside a building, an id
        changed when another district did. Lesson: what a tool derives must be nameable by place.
  - [ ] -> spec/identity.md: `[[clear]]` and `[[pin]]` should speak to every producer, not scatters only, or say so.
  - [ ] -> spec/protocol.md: A step cannot ask for the pack; `lots.py` reads the committed file. Lesson: the SDK needs
        the pack and its taxonomy.
  - [ ] -> spec/diagnostics.md: `[[override]]` without `id` crashes (`KeyError`) instead of reporting.
  - [x] -> done (M3, assets.md 6.1): The pack has no small urban building; an old town is 22 m boxes.
        `house_narrow` (5 m front, 8 m deep, two storeys) added to the primitive pack; lib/town's
        houses step prefers it.
  - [ ] -> spec/graph.md: A part given anything rebuilds whole: `atlas` is 200 s. Partial rebuild by district.
      (From the list before the rewrite, Surface and delivery; was -> by item below.)

### `lib/building`

- [ ] **What someone else's kit left** (K10, `examples/borrowed`, lib/building/README.md "Someone
      else's kit"). Kenney's corners with windows (nine corner blocks) have no place: a corner
      has no variants (rule 8). Eight window blocks stand 0.025 to 0.075 m proud of the block's
      face (sills, awnings, a balcony) and rule 8 refuses each as a variant, as K3 decided; a
      sill as a piece of its own, placed on a bay, would take them. Kenney's gable is a whole
      ridge for a building one block deep, not our two slopes and gable halves. Not built: a hip
      over an L (Kenney's inner corner), steps to a door (four step blocks), and a leaf that sits
      in a block's hole (six doors, 22 windows).

- [ ] **What the realm left** (K11, `examples/realm`). Not built: a plot that keeps trees off (it
      has no `keep_out`: a pad's would warn of every piece of a kit); one plot for buildings that
      are not a court; a row on a plot (its houses step, and on the realm's slope the ground
      stands up to 0.26 m over a house's floor on its uphill side, now said by check.py rule 13,
      `ground_over`, R78, and a house in water by rule 14, `in_water`: the fix plan's F2). Not
      said at all: 66 m of road under more than 0.3 m of water in `flood` (lib/review says only
      that the ford is 3.2 m deep); a settlement whose land a new seed changed under it (`new_seed`: every instance
      moves, the floors with the land, and only the roads' crossings of the new stream are
      said, 25). A way that keeps off what stands is written in each route (`+ 60 * (1 -
      smoothstep(6, 12, dist('kind:building', 14)))`); without it the trail went through the
      temple (`through_fane`: 15.2 m of trail inside it, 108 `place.keep_out`).
      (From K11 of docs/plan-kits.md, 2026-11-23.)
- [ ] **The ground's rules skip the kit's.** Since the fix plan's F2 `check` reads the ground along
      every building (rules 13 and 14), so a build whose ground failed skips the whole lint
      (spec/diagnostics.md 4), the kit's rules too, which read no ground: `examples/borrowed`
      `handed_over` lost its two `kit_deck`. The ground's rules (12 to 14) as a lint step of their
      own, beside `check`, would keep the kit's said; every world's walls layer would then
      declare two steps.
      (From the fix plan's F2, 2026-11-24.)
- [ ] **Two plots over one ground are said, not avoided.** Two buildings with `ground =
      "level"` laid in one layer each return a plot, and neither `lay` call sees the other's;
      where they overlap the later by name holds the ground (`examples/realm` `shrine`: the
      temple's plot 0.539 m off its level along its back wall, said by `plot_lost`). A plot for
      several buildings that are not a court, or `lay` reading its layer's plots, would avoid it.
      (From K11's review fix pass, 2026-11-24.)
- [ ] **What pipes leave** (K9, second slice; lib/building/pipe.py, from `examples/works`). Said
      or refused, not built: a leg that climbs along its length (`params`: a kit of straight
      pieces has none that climbs); a turn that is no elbow of the kit; a climb that is no riser
      of the kit; a tee, a valve, a reducer. Not said at all: a pipe's heights are above the
      ground at its first point, so over ground that is not level its ports miss in height and
      the only word is the engine's `surface.gap` at each (`sloped`: 149, 4 and 188 mm; an
      error, so the build stops); a pipe that ends at a plain wall is told only that its flange
      is answered by nothing (`no_port`); a pipe through a wall, a building or another pipe is
      laid, since no piece's body is read against a wall's or a pipe's; a support is one length and its
      foot is checked against the ground under the pipe's first point, not set on it. A pipe is
      a point and its further points because a path over its own legs is
      `geom.unowned_crossing` with each (as a row's frontage was, K6). `check` holds a pipe
      kit only to its pitch (`kit_pitch` of the straight and the slip piece, with the kit's
      taxon in `kits`; `examples/works` `short_pipe`): an elbow's, a riser's, a flange's and a
      cap's `params.reach`, a riser's `params.rise` against its model and its `top` mount's
      `rise`, and a support's `params.height` are read by nothing but the example's own
      measure, which finds loose ends from the models' boxes. `kit_unit` and `kit_stated` are
      for wall kits, and no room kit is read. Two legs that cross at one height are the core's
      `geom.unowned_crossing`. Lesson: the pipe step
      pairs nothing; a flange and a port with one word at one point are the core's to join
      (R76), which is what lets a third kit's port take the same pipe.
      (From K9 of docs/plan-kits.md, second slice, 2026-11-23.)
- [ ] **A ring of whole rooms says where it misses, not which piece is wrong** (K9, second
      slice, `examples/works` `long_room`, `flipped`). The closing join's two doors each name
      the other and the miss in millimetres; a room 128 mm long and a hall of the other hand
      read the same. A room kit has no step of its own: its pieces are written one `on` the
      last, in the layer that makes their floor, and the first by a point and a height (no text
      can name a floor a step made). A generator for a ring, as `row.py` and `court.py` are for
      theirs, would know the pieces' sizes and could say which does not fit.
      (From K9, second slice, 2026-11-23.)
- [ ] **A port's hole is not held to its mount.** `examples/works` `low` restates the shed
      port's `rise` as 3.5 m with the model's hole still 4.5 m up: the engine holds the pipe to
      the mount (`surface.gap`, 1000 mm) and nothing reads the hole. `check` measures a piece's
      box, not its openings (docs/design/kit-contract.md).
      (From K9, second slice, 2026-11-23.)
- [ ] **What posts and bays, and the colonnade, leave** (K8, first slice; lay.py rules 15 and
      16, from `examples/temple`). Each is refused or not laid, and says so:
      an inside corner in a kit laid by posts (`params`: the post in the notch, and the two
      stretches that start from it, are not worked out); bays of two widths on one side (a
      wider middle bay: a run holds one pitch, and a variant must be the bay's size); a
      colonnade on two storeys or on a plan that is no rectangle; steps up a podium (the doorway holds the ground at its
      sill, as K7 filed). `row.py` does not pass `colonnade` to its buildings. Lesson: a line
      of posts outside the walls was one more use of rule 14's moved outline, and `N` bays with
      `N + 1` posts needed no pitch on a run, only two runs over one stretch.
      (From K8 of docs/plan-kits.md, 2026-11-23.)
- [ ] **What the roof beyond the walls leaves** (K8, second slice; lay.py rule 17, from
      `examples/court`). Not built, and each says so where it is asked for or is simply not a
      word yet: layered roofs (a lower roof round a storey set back above it: a roof on rule
      14's terrace); curved eaves (the kit's own modelling); a hipped roof over an L; a gable
      under eaves over an L (`roof_none`); eaves under a flat roof; a building's own `eaves`
      over its kit's. A gable under eaves on a rectangle is built and tested with made-up rows
      only: no kit here has the gable half it needs (the slope's span less the eaves). Lesson:
      eaves belong to the kit, not the building, because a roof piece's span is half of one
      depth of building and the eaves together.
      (From K8 of docs/plan-kits.md, 2026-11-23.)
- [ ] **Only a court knows what a roof stands in.** A roof under `shared.eaves` is the first
      thing a building owns outside its outline. `court.py` says `court_meets` where two of its
      halls' roofs, or a roof and its yard's wall, share ground. Two buildings written by
      hand, two rows, or a row and a court are told nothing by the library; the engine says
      `place.crowded` of two roof pieces, and only where two pieces' boxes overlap
      (`examples/court`, `close`). `row.py`'s party walls and `row_meets` read outlines, not
      roofs. Rule, or a check in `check.py` over every `roof` feature.
      (From K8 of docs/plan-kits.md, 2026-11-23.)
- [ ] **A yard is a wall, not yet a place** (K8, second slice; lay.py rule 18). A building of
      no storeys is its walls on the terrain round a `yard` feature with no surface. Left: its
      ground is the land as it lies (nothing levels a court, and each hall's doorway raises
      the ground to its own sill inside it); on a slope each wall piece stands at the height
      under its middle, so its ends are in and over the ground by half its length times the
      slope, with no base and no word said (`examples/court`, `sloped`: 7.5 cm); a wall along
      an open line that is no outline is not laid; the gate returns no doorway, so nothing
      measures its clear width and no way can end at it by name. `lib/town`'s `enclosure`
      still writes its wall on the terrain as a stand-in: it could return a building of no
      storeys instead.
      (From K8 of docs/plan-kits.md, 2026-11-23.)
- [ ] **What a court leaves** (K8, second slice; `lib/building/court.py`). One yard, one hall
      across the axis, wings along it in pairs. Not built: a second court behind the first; a
      gate house; halls at other angles; a wing's `bays` (a pin on one bay) passed on with its
      place reflected (`fill` is, with sides 1 and 3 changed and every entry mirrored);
      `faces` for a hall, so a `fill` cannot say "the side that faces the court". The mirror
      is the generator's and is measured (`mirror_mm`), not held: a twin edited by a change
      file afterwards is not told.
      (From K8 of docs/plan-kits.md, 2026-11-23.)
- [ ] **`shared.module` and `shared.unit`: two words for two things, to be ruled.** Since K6
      `module` is the kit's bay and `check` holds every `role:wall` piece to it. A classical kit
      takes every size from its column's width, which is smaller than any bay, so K8 added
      `shared.unit` beside it (`kit_unit`, check.py rule 11) and left `module` alone. Studios
      use "module" for the grid unit (a 256 or 384 footprint), which is `unit`'s meaning here,
      not `module`'s. Renaming `module` to `bay` would touch four packs, their facts and the
      kit contract. Context: `examples/temple/README.md`, "Before", third row.
      (From K8 of docs/plan-kits.md, 2026-11-23.)
- [ ] **A plan is not held symmetric.** `mirror` on a `fill` mirrors a wall's pieces about its
      side's middle. Nothing says that a door is off its building's axis, or that two wings
      differ: the temple's door is on the axis because its author wrote 7.2.
      (From K8 of docs/plan-kits.md, 2026-11-23.)

- [ ] **What the wall's foot leaves** (K7, first slice; lay.py rule 13). The base is one piece
  of one depth: `base_short` says where it is too shallow and nothing stacks a second course.
  A door has no steps: the doorway raises or cuts the ground to its sill, a mound on a steep
  slope. Uphill the ground stands against the wall (1.7 m on `examples/village`'s steep street)
  and nothing says so or shapes it: a yard or pad of the building's own. The kits of `kit`,
  `block`, `station` and `college` have no base, so each of their buildings says `plinth_bare`
  (info). And `lay`'s records of one code for the buildings of a row fold into one whose
  numbers are the first building's. Lesson: the gap was in every sheet since M5 and no measure
  looked under a wall.
      (From K7 of docs/plan-kits.md, 2026-11-23.)
- [ ] **A thing inside a house is not the house's.** A world's text cannot name the floor of a
  building a step made, so furniture is written by a point and a height (`surfaces.md` 4.1).
  When the house moves or its ground changes the thing stays: in a wall (`examples/village`
  `wider`: two wall bays gave way to a chest, 6 m open), or on the roof whose eaves are now at
  that height (`steep`). The build names it only once the two disagree. A place inside a row's
  lot that a world can write (a lot's own frame, or a mount the building returns) is the
  question. Lesson: this is the studio's enlarged house that broke a quest.
      (From K7 of docs/plan-kits.md, 2026-11-23.)
- [ ] **What K6's review found and K6 did not mend.** (1) A diagnostic `lay` says of a building a
  row returned is located on `[step.rows]`, and records of one code on one building fold into
  the first: a second storey's `wall_open` is not shown. (2) `check` gives two `kit_variant`,
  both false, for a one-storey building of the city kit (its ground-storey blank bay is held to
  the upper bay). (3) A building past the world's extent makes `lay` raise (`RpcError`), by hand
  or from a row. (4) A party wall is blank to its top: 68 blank bays of `examples/block` look out
  over a lower neighbour's roof, since `faces` says what stands beside a side and not how high.
  (5) A flight that ends on the far wall leaves 1.5 m of wall unbuilt (a lot of the plain kit
  exactly as long as a landing and the run: `row.py` now keeps a landing past the head; `lay`
  written by hand still does it). Lesson: the review's mutation run left 10 of 34 standing, among
  them the whole of one new check; tests written with the code pin what the author thought of.
      (From K6 of docs/plan-kits.md, 2026-11-23; the independent review.)
- [ ] **A block is four statements, not one.** `row.py` (K6) lays buildings in bays along one
  frontage. A block of four rows repeats the kit, the depth and the `fill` on each, and its side
  rows are written a row's depth in from the corners by hand (the one sum left: 12.6 m); two
  rows written to one corner are refused and said (`row_meets`), not solved. A `block` feature
  (a region, its sides' lots) would own its corners. Also from K6: a side only part of which a
  neighbour stands against has one word in `faces`, so where that part is under half its windows
  stay (`examples/block`: 4); a lot is named by its number, so a lot written before others
  renames them; a corner building has one door, on its row's street; and what a row passes on to
  its buildings is eight attributes named one by one. Lesson: twelve buildings by hand were 192
  numbers and 312 unseen faults; the unit an author writes in should be the kit's.
      (From K6 of docs/plan-kits.md, 2026-11-23.)
- [ ] **A storey's own outline, and what does not close on it** (lay.py rule 14, K7). Built on
  a rectangle, one side or two opposite ones, never the stair's. Left: a terrace is whole bays
  less one wall deep, so no whole number of tiles, and its side parapets are two posts short of
  whole pieces (`examples/block` `setback`: 14 tiles and 4.8 m of parapet, said); a jetty
  smaller than the kit's half bay does not close and a deeper storey has no roof piece
  (`examples/village` `jetty`). Lesson: the walls' outer face as the outline makes a storey's
  inside whole tiles and a terrace's not; a kit that wants terraces needs the piece for the
  difference, or its walls on the grid's lines.
      (From K7 of docs/plan-kits.md, 2026-11-23.)
- [ ] **Three of four instances of a city block are floor tiles** (3,719 of 4,949). A floor of
  1.5 m tiles is whole only on the tile's grid and costs a tile each 2.25 m2; a block of twelve
  buildings takes about a minute to build, most of it in `lay`'s calls and their tiles. A floor
  piece a bay square, or a floor as one mesh per storey, is the question.
      (From K6 of docs/plan-kits.md, 2026-11-23; measured on examples/block.)
- [ ] lib/building `lay`: an inner wall between rooms leaves no gap for a flight (the station loft's
  walk up its flight was blocked at 2.89 m with the cut at x = 49.3), and a door may land on a
  rooms' cut (the lint's `door_narrow` said 0 m clear). Refuse or move either (`params`).
      (From M5 slice 1, 2026-11-16.)
- [ ] `lay` returns no place name a bookmark can target (a storey, a stair's foot): a mount or a
  documented point per door and stair foot would replace the fresh agent's hand `t`/`offset`
  points. Decide with the produced-name rule (R3).
      (From M5 close, 2026-11-17.)
- [ ] A lint that furniture stands off a flight's line (station's `in_flight` is a world measure);
  lib/building `check` should report it for any building.
      (From M5 close, 2026-11-17.)
- [ ] **A building placed by `on` and `face`** (the `mill`'s "a mill beside its weir"): a `region`
  holds absolute points, so following another feature took a 20-line generate step, its lint
  (`authority.unchecked`) and views found by an absolute `near`. A region in its own frame placed
  `on`/`at` with `face`, as an instance's footprint is (core or lib/building).
      (From M7 close, 2026-11-17.)
- [ ] **lay's walls on a turned outline**: the kit's pieces are not turned with the edge, so an
  11-degree mill showed a saw-tooth with open corners; it builds clean. lib/building README now
  says to keep kit-walled buildings square.
      (From M7 close, 2026-11-17.)
- [ ] **A view on a produced storey**: `surface = "<storey>.floor"` cannot be written (R3); the
  eye is lifted by hand (`eye_height = storey_h + plinth + 1.7`). A way to stand `on` a produced
  storey by `of` and a level.
      (From M7 close, 2026-11-17.)
- [ ] **A building's own yard or pad**: the doorway's derived `sill` loses to any shaped ground kind
  under it (a terrace gave `surface.gap` 29 mm); `plinth = 0` on a slope is the workaround.
      (From M7 close, 2026-11-17.)
- [ ] Scope a building domain as a sub-kit on top of the core, as vegetation will need its own.
      Context: the interior spike reinvented storeys, spaces, doors and stairs. Lessons: use the
      established vocabulary (building-information modelling defines storey, space, wall, door,
      stair, slab) instead of inventing one; model a stair as a thing, not a hand-made connection;
      keep a non-parametric escape hatch (a room that is one authored mesh, with only its footprint,
      openings and floor heights declared); make the kitbash contract explicit (module sizes and
      sockets constrain room sizes from the start); and carry dressing down to small props on
      furniture, which needs assets to declare their surfaces.
      (From the list before the rewrite, Assets and buildings; was -> M5.)
- [ ] **What a kit states, where it is not yet read.** lib/building lays by a kit's
  `defaults.shared` (R66, K6: `wall`, `stair`, `door`; `module` and `storey` are checked). Left:
  lib/town's rows read each piece's pitch alone and no `shared` (F4 of docs/plan-fixes.md
  moved its kit read into the SDK; reading `shared` before the run lays by it costs its reuse); the kits of `station`, `college`, `market` and `quay` state nothing
  (`kit_unstated`, info). Lesson: the first statement written for the plain kit was wrong (one
  storey height of two) and the check caught it: a stated fact is only worth having with the
  check that holds the pieces to it.
      (From K6 of docs/plan-kits.md, 2026-11-23. A building's `storey_h` and `plinth` come from
      the kit since the fix plan's F3.)
- [ ] **No floor tile on a building turned off the axes.** A tile is an instance on the
  millimetre lattice; off the axes two that abut overlap by the rounding, and two instances of
  one step in one place are `place.overlap`, an error (49 on the annex at 30 degrees). Walls
  escape it because neighbours of one run are never compared. `lay` lays none and says so
  (`slab_open`), and refuses a flat roof there. Candidates: rows of tiles as runs (needs a run
  along a line a step returns, standing on a named surface); a tolerance of a lattice unit in
  the overlap test. Lesson: "touching edges do not overlap" holds on the lattice only.
      (From the review of K4, 2026-11-23.)
- [ ] **A flight that ends within a wall's thickness of the wall ahead cuts a gap in it.**
  `wall_stretches` takes a flight's path for a crossing where it only comes near (a stair run to
  0.3 m of the far wall: 50 mm of that wall opens on both storeys). Older than K4; met again in
  its review.
      (From the review of K4, 2026-11-23.)
- [ ] **A gable over a plan that is neither a rectangle nor an L** is refused (`roof_none`) and
  the flat roof laid (the L came with K7, lay.py rule 10: a hip, and a valley with a reach). The base came with K7's first slice (lay.py rule 13); steps to a door did not. `STUB_MIN` (a stairwell 0.3 m short of a
  corner is refused) was for the wall stub beside it; with a footing no stub exists, and the
  rule now only keeps a thin strip of floor from being cut. Revisit it with the stair above.
      (From K4 of docs/plan-kits.md, 2026-11-23.)
- [ ] **Corners past the plain case.** `params.reach` on an outside corner (a piece with a leg
  along each wall) is covered by a unit test only, no example. A corner that is not a right
  angle still gets a square box turned to the wall after it, as before. A T-junction of walls
  and a wall's free end have no role.
      (From the plain kit's mends, 2026-11-20.)
- [ ] **Two doors on one side are each told alone.** `gap_wide`'s hint finds where one door would
  close with the side's other openings held where they are; two doors both half a metre off are
  each told no place closes, though moving both would. Solve a side's openings together.
      (From the review of K1 and K2, 2026-11-20.)
- [ ] **Variants of the bay only, and only of its size** (lay.py rule 8). Not built: a variant of
  a filler, a corner or a door; a variant of another length (a half-bay window, a double bay),
  which is a fit and not a swap; a weight on a choice; a choice that reads what stands beside
  the wall (a door facing a street). The no-repeat rule holds inside one stretch, not across a
  door or a corner. Lesson: a variant that must be the bay's size is the whole reason a swap
  needs no refit; anything looser is another rule.
      (From K3 of docs/plan-kits.md, 2026-11-22.)
- [ ] **A vertex in the middle of a straight wall gets a corner piece.** `wall_stretches` takes
  every vertex of the outline for a corner and a straight one for an inside corner (`turn > 0`
  is false), so a wall with a point in its middle is cut there and given a `corner_in`. Sides
  are now numbered through it as doors number them; the corner is still laid. Lesson: a corner
  is a turn, not a vertex.
      (From the review of K3, 2026-11-22; older than K3.)
- [ ] **A side is named by its number** (`side`, `at` on a door and a stair). A plan that gains a
  corner renumbers the sides after it, so a door on a later side must be written again
  (`examples/kit`: the annex's door is on side 0 so that `ell` does not move it). A side named
  by a point near it, as a selection is, would hold.
      (From K1 and K2 of docs/plan-kits.md, 2026-11-20.)

### `lib/map`

- [ ] lib/map labels do not avoid footprints; a test of `read_world`'s pack handling (the
  `asset_unknown` warning) with a fake step.
      (From M6 slice 2, 2026-11-17.)

### `lib/review`

- [ ] **A bookmark cannot keep its eye clear** (glade 4): writing keep_out from a bookmark loops
  through the instance it looks at. Lesson: a view's clearance is a placement fact, not a field.
      (From the M2 integration, 2026-11-14; was -> M3.)
- [ ] **Measures beyond `checks.py`.** `water_*`, `trees_on_way` and `crowded_view` landed in
  lib/review (relief 2, vale 5); worlds' `expect.toml` still read their own scripts.
      (From the M2 integration, 2026-11-14; was -> M2.)
- [ ] -> lib/review (reach): **`not_sealed` says no place.** A sealed pair that holds a route names only
  the `to` place; finding the way in took a throwaway walk over the build's fields (twice, as in the
  spike's LOG entry 2). Give the walked route's first post past the `from` side's piece, or the
  narrowest stretch, as `unreached` gives its stop.
      (From building monastery, 2026-11-15.)
- [ ] reach `not_sealed`'s new `narrowest` place is unit-tested only, not exercised in a world.
      (From the M3 close, 2026-11-15.)
- [ ] lib/review reach lint: refuse an unknown params key or a malformed entry (`{ from, to }` at the
  top level ran silently; `sealed` entries without `profile` raised AttributeError) and say one
  info when it checks nothing. Its reference is now in lib/review/kinds.toml (hamlet trip 1).
      (From M4 slice 4, 2026-11-16.)
- [ ] **Give the street-form numbers a reference.** Bearing spread and junction shares have no target.
  The earlier research measured real towns; bring a few of those numbers in as ranges per pattern.
  Lesson: a measure without a reference only ranks our own builds.
      (From the boxiness measure and the town on simulated land, 2026-11-13; was -> M3.)
- [ ] Design the reach lint properly: opt-in, driven by declared movement profiles and stated
      expectations, never by assumed defaults. Context: a first version in the spike found the
      lighthouse and a headland cut off before being told where to look. Needs: several profiles
      (on foot, mount, glider, boat); links and barriers that add or remove connection (ladder,
      door, ferry, locked gate) with conditions for progression; negative expectations (a sealed
      zone must stay sealed); waivers that carry a reason; and honesty about what it reads (terrain
      top surface only). Lesson: visual QA is not ours, but "is this promise about the map true" is.
      (From the list before the rewrite, Checks; was -> M3.)
- [ ] Lint idea: collision that is missing by mistake. Some assets have no collision on purpose
      (grass, small props); a large one without it is probably an error. Context: adding a walking
      body to the spike, where scattered trees and rocks had no colliders at all. Needs the asset
      contract to declare collision (none, box, mesh) so a rule can compare it with size, and a
      waiver for intended cases.
      (From the list before the rewrite, Checks; was -> M2.)
- [ ] Let a check know the state it is checking. Context: the measuring lint reports the pistol
      off its published size when the slide is back or accessories are on, which is true and not
      a fault.
      (From the list before the rewrite, Surface and delivery; was -> M5.)

### Every library

- [ ] Seven kinds are written by no example: `building.roof`, `land.firm_pad`,
      `terrain.terrain_course`, `vegetation.clearing`, `vegetation.green`, `vegetation.wood`,
      `water.weir` (`docs/layers.md`, "What the layout shows"; `terrain_course` is what a step
      returns). Write a world that uses each, or retire it.
      (From the revisit, 2026-11-18.)
- [ ] Names that blur: `lib/land` beside `lib/terrain`, and a lake in both `lib/water` (`lake`,
      `tarn`, `pond`) and `lib/terrain` (`terrain_lake`). Context: a library's name is written in
      every world's `include` and cited by chapters and cases, so a rename is a change to the
      format, not to a folder: it needs the conformance suite and a note for worlds already
      written. Decide the names first; `docs/layers.md` says meanwhile what each holds.
      (From the revisit, 2026-11-18.)
- [ ] Define the standard vocabulary tier between core and user: named kinds and operators we ship,
      keep stable, and map in every exporter. Water bodies are the first candidate (still: lake,
      pond, sea; flowing: river, stream). Context: "does a lake exist?" Lesson: the physical form
      (a water-surface field) is core, but identity and meaning (a named body with a level, a shore,
      inlets) must be vocabulary, or gameplay, minimap and engine water systems have nothing to bind.
      (From the list before the rewrite, Core model; was -> M2.)

## Worlds: the examples and the packs

### `examples/atlas`

- [ ] **`atlas` should take its plain from this**: the river that bends and the land under the city
  were the reason the original library was brought up.
      (From simulating the land under stated facts, 2026-11-13; was -> M6.)
- [ ] Owed to M6 by earlier sections, not done this slice: atlas takes its plain from the erosion
  library and townland's authoring; coast's frame rate after parts; the loader's minimap. (Atlas
  pass: the plain is the erosion library's since M6 slice 2, layers/land.toml's `via-terrain`
  step; closed. Townland's authoring is not done: given land, townland's fixed levels (deck 8.5,
  quay 6.6, its valley written in its own metres) stand in its own zero, so `give = { land }`
  needs those written over the given ground first, and a townland build and an atlas build to
  show it; and its main ways need a `given` of features its districts node with.)
      (From M6 slice 2, 2026-11-17.)
- [ ] Atlas after M7 slice 1, left (`atlas`): college's lift (+2.4, above); `market.top` (above);
  `move_market.ran_imports` re-taken on a fresh store with no import edited between base and
  change (measured ["city", "market"]: city's parapet edit landed between them; done in the fix
  pass by the on-foot review, ["market"]); city's parapets
  seen in atlas; townland's authoring (given land) not started; `road_painted_in` 0.286.
  Lesson: a change of an import's text between a base and its change build shows as that import
  running in the change; freeze every imported world before a timed pair. Lesson: the terrain step
  settles at 16 m cells, so a shelf written into the plain at an import's edge came out 10 m lower
  2 m from it; carry a written slope 30 m past the edge it must meet.
      (From M6 close, 2026-11-17.)
- [ ] On-foot sheets of the atlas changes, on the decks and in the cave; monastery alone to see
  whether its deck fall is its own.
      (From M6 close, 2026-11-17.)
- [ ] Atlas rows banded at the value measured (NOT HOLDING in expect.toml): grade_max and
  road_join_max 0.442 (target 0.25), road_painted_in 0.286, join_step_max 37.24, bookmarks_wet,
  exit_in_grade_max 1.122, exit_out_grade_max 0.375. The places stand on plinths and in pits
  (on-foot review 4: the lift-at-exit trade is the owner's; else a rim-fitted plain).
      (From M7 slice 1, 2026-11-17.)
- [ ] **`coast.in_cave` blocked 1.95 m short in atlas** (on-foot review 1; it arrived on the slice's
  binary, and alone it arrives 0.991 m short against the 1 m radius). Likely R53's wider round-ended
  band cutting the quad [2428, 2430] x [68, 70] at the cave's mouth, 1.4 m from the stop; not proven
  (atlas not rebuilt without it). Re-aim the view off the 9 mm margin, or look at the ground the
  band leaves at the mouth. Lesson: a cut that grows moves every walk that ends near it; re-walk the
  views beside a changed band.
      (From M7 close, 2026-11-17.)
- [ ] **join_college's eye shows a large grey panel in the field beside the approach** (on-foot
  review 6): not identified.
      (From M7 close, 2026-11-17.)
- [ ] **Put townland's authoring into `atlas`.** The city still carries hand-drawn arterials and its own
  river. It needs the parent's land and main ways handed down (`give`), and a build under a minute.
      (From the boxiness measure and the town on simulated land, 2026-11-13; was -> M6.)

### `examples/basin`

- [ ] basin's `measure.py` exits 1 on base (on-foot review 5): ground_tarn_view 44.36 (oracle
  57.35 +-6), ground_middle 55.79 (62.38 +-6), mouth_moves 51.4 m (50). The 2026-11-14 values
  came from a state never committed: a fresh terrain run on a bit-equal `before` gives 44.36 on
  003d19e, d8e8a5c, 0e57aae, f738d84 and this tree (not the cutover, not the river's `width`).
  The committed `gen/terrain/` holds that state's `eroded` and is stale for any rebuilt
  `via-terrain` (`protocol.untrusted_stale`, `param program`), so an untrusted reviewer cannot
  build basin (protocol.md 8 says one can). Re-taken with the reason in expect.toml; owed: which
  state, whether the bands or the land move, and a fresh `via step commit`.
  M7 close (`core`): committed fresh (`via step commit`, release, this tree's `via-terrain`); a
  configured program's digest now stands as recorded for an untrusted step (protocol.md 7.1,
  decided, open to the owner), and README gains the venv step (`uv sync --project sdk/python`)
  and the untrusted build. Proven in a fresh worktree: basin untrusted with its own rebuilt
  `via-terrain` gives the trusted build's `height` bytes, and trusted, running it, the same. Still
  open: measure.py's ground_tarn_view 44.36 and ground_middle 55.79 against the oracle (the state
  question above), and an untrusted build's `findings` 1 (the lint's `protocol.untrusted_skipped`
  warning, counted by measure.py).
      (From M7 slice 1, 2026-11-17.)

### `examples/border`

- [ ] border views: `at_gate` hugs the enclosure wall 0.75 m off (move the eye 5 to 8 m east, out
  of the gate); `south_opening` aims at the gate through the wall corner (aim at a point on the
  south walk); `wild.look_corner` to `wild.west_end` is blocked by the bank (add the reach pair or
  move the bank to the parent side); fringe trees on the steep bank (a slope limit is in the
  density, 22 to 30 degrees: check the bank's slope against it); brought views outside the
  footprint look at grass (drop or flag them).
      (From M4 slice 3, 2026-11-16.)
- [ ] border views and reach (`at_gate`, `south_opening`, `wild.look_corner`, trees on the bank,
  brought views outside the footprint): the slice 3 list above.
      (From the M4 close, 2026-11-16.)

### `examples/city`

- [ ] `city`'s grand avenue strays 39 m off its gate-to-gate line over 1.5 m lumps into the
  rows district: a path way point by name (core item 3) or a stiffer route cost.
      (From From M3 slice 5 (city, 2026-11-15); was -> M3.)
- [ ] -> examples/city: `downtown_at_central` (eye 3 m from a house wall in base) and `old_town_street`
  (looks across lots) face a fixed point; anchor each on a street and face along it.
      (From the M3 slice 5 review on foot, 2026-11-15.)
- [ ] city: 1,465 of 2,779 lots hold no house (1,314 held; no asset fits, or too steep on the hill).
      (From the M3 close, 2026-11-15.)
- [ ] city: bookmarks `downtown_at_central`, `old_town_street` anchored on a street; houses along the
  arterials outside the districts; the hand-edit views never rendered.
      (From the M3 close, 2026-11-15.)
- [ ] city `look_grand_central` is `at = [700, 830]`, so after `move_avenue` the view looks 40 m
  off the avenue: put it `on` `grand_ave_line` (review on foot, PNGs in its scratch `keep/`).
      (From M4 slice 1, 2026-11-16.)
- [ ] Change builds: city move_avenue 63.3 s against 60.5 cold; atlas move_market 58% of cold;
  an unchanged build into a new directory with an export trusted 89.8 s, about 54 s in no node;
  over_budget fails only after 175 s (check the budget before writing instances).
  M7 slice 1 (perf), protocol in examples/city/expect.toml: key records and a batch's input
  objects read ahead on --threads (graph.md 8), the budget ungated (build.md 5.1, case 53: atlas
  over_budget 75% -> 58% of cold, its 387 writes skipped), a `for_each` step's items kept under the
  leaves and the model's build started beside a rank (graph.md 5.3); city's back-to-base build
  47 -> 35 s; move_avenue paired 84/69 -> 61/75 s at load 20 to 36, not shown under 40 s. Left,
  from `sample` of atlas built unchanged into a folder at another depth (102 s, every node
  reused): `prepare` (placement) 59 s over the ModelPlan of each rank, the final graph and the
  imports, and the model's build 25 s for step requests (13 s) and trace answers (12 s), whose
  memos miss because the leaves hold the build key, which holds `locations.pack_root` by place
  (graph.md 5.3, open). An unchanged build into a sibling folder is already null (0.6 s).
  over_budget's remaining time is the change itself (`chunk = 160` re-runs 7693 nodes), so 30%
  of cold needs the budget counted before the per-chunk nodes, from placement alone. Atlas
  move_market was not timed this slice.
      (From M6 close, 2026-11-17.)
- [ ] Revisit the `city` world. Parked 2026-11-12: the point (draw one level, derive the rest in
      user code) is proven, the result still needs work. Known: boxes for buildings, empty block
      middles, no junction geometry, arterials painted across the river bed, five unlabelled
      districts. See FINDINGS, "A city that makes sense".
      (From the list before the rewrite, Surface and delivery; was -> M3.)

### `examples/coast`

- [ ] coast (M5 slice 2): the cave's mouth is a slot the face's full height and the passage has no
  rock (challenge 8: a library places rock round a cave, surfaces.md 5); the sheer face leaves
  0.13 m² a metre open at foot and brink (a cut quad reaches 0.71 of a post past the strip). The
  sheer kinds for the monastery wall, gorge and tarn are not written (`sheer_cliff` is the cliff's).
      (From M5 slice 1, 2026-11-16.)

### `examples/college`

- [ ] college: box collision for the primitive pack, a door at each `entry`, the chapel padded to
  its court (README "Known faults").
      (From M5 slice 1, 2026-11-16.)
- [ ] college, border and example declare reach as a plain command: they walk the terrain only and
  now say `terrain_only`. Make each a worker and re-record (example's crossing deck is a kind
  surface: its reach rows may rise). Not rebuilt here (not this agent's worlds).
      (From lib/review reach over kind surfaces, 2026-11-17.)

### `examples/county`

- [ ] county: the town is bare in the parent: no streets or houses (import steps, above) and no
  paint (`import.weights_dropped`); a turned town notches its quay wall (`wall_face`, seam 1.61).
  `move_town` re-runs `import:town` whole (land given) and all 1280 field chunks.
      (From M4 slice 2, 2026-11-16.)
- [ ] county: the erosion step on the county's land (skipped for the box); views of the town's
  district (its steps run since M4 slice 4; the README is written).
      (From the M4 close, 2026-11-16.)

### `examples/example`

- [ ] -> examples/example: a bush lands on the `at_mill` eye after move_beck; keep scatter off bookmark eyes.
      (From the M3 slice 5 review on foot, 2026-11-15.)
- [ ] example: the bush on `at_mill`'s eye; the church path 7.5 m short of the door.
      (From the M3 close, 2026-11-15.)

### `examples/kit`

- [ ] The six styled kits the owner listed (medieval or fantasy village, modern city, traditional
  East Asian, industrial, classical, terrain-adapting), each for the structure it forces: roofs
  over irregular plans, floors by bays, the roof as the main kit, pieces joined in three
  dimensions, every size from one module, meeting a slope. The research found no account by a
  developer of roofs on irregular plans, of slopes, or of classical or industrial kits: those
  examples test our own design and are to be labelled so.
      (From the owner, 2026-11-19; the plain kit came first.)
- [ ] A kit we did not make. Every piece here is ours, so it fits where we meant it to. A freely
  licensed kit fetched on demand and never committed is the only test on pieces someone else
  shaped (challenge 33).
      (From the plain kit, 2026-11-20; examples/kit and its README.)
- [ ] No rooms, and no furniture on its floors. Variation between bays came with K3 (`fill`,
  `bays`); storeys of two heights, floors and roofs with K4.
      (From the plain kit, 2026-11-20; examples/kit and its README.)

### `examples/market`

- [ ] Challenge 33 (K10): a real kit has now been laid by lib/building (`examples/borrowed`,
      Kenney's Modular Buildings), but not by lib/town's kit run, which `market` uses. Kenney's
      seven sample houses are whole buildings, the shape lib/town's pieces have: owed, a market
      street of them, once a model at another scale has a factor (`spec/assets.md`).

- [ ] examples/market: the rows stand on the ground as it lies (no terrace): 85 of 189 doors within
  0.3 m of the footway, 0 more than 1 m off. A kit run that returns a terrace per stretch (as
  houses.py does) is owed; then `max_relief` back to 0.6.
      (From M5 slice 3, market, 2026-11-17.)
- [ ] Dogfood: the market-street district from the same report, built from a box kit then a real
      one, judged on three changes (piece remodelled, piece's module or pivot changed, street moved).
      Box kit done (M5 slice 3, examples/market, all three changes measured); the real kit is owed.
      (From the list before the rewrite, Surface and delivery; was -> M5.)
- [ ] examples/market `measure.py` divides `instances/z.npy` by 1000 with x, y and face;
  build.md 8 says z is metres (`<f4`). Its door measures (`doors_level`, `doors_off`) so read a
  floor about a centimetre above the ground at each piece's origin. Check them, and the "85 of
  189 doors" figure.
      (Found writing examples/kit's measure, 2026-11-20.)

### `examples/massif`

- [ ] **expect rows OUT at 4d331d3 and now alike** (slice 4 re-check): massif `rocks`
  0 (`scatter.empty` fallen_rock) and every change's counts; basin `ground_tarn_view` 44.4,
  `ground_middle` 55.8, `mouth_moves` (1866.6, 804.2). Not from slice 4: owed a look.
      (From building monastery, 2026-11-15; was -> massif, basin.)

### `examples/monastery`

- [ ] Monastery `no_bridge` floods the floor at the bridge site (seen, not measured).
      (From the M3 close, 2026-11-15.)
- [ ] monastery `bm_turn` (`at = [292, 62]`) stands on grass beside the road: `on` `road_up` near
  `turn_2`.
      (From M4 slice 1, 2026-11-16.)
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
      (From M5 slice 3, levels and sheer faces, 2026-11-17.)
- [ ] **Two water surfaces meet as an upright sheet at monastery's tarn outflow** (on-foot review 5,
  look only): the stream's lighter surface stands over the tarn's in the gorge.
      (From M7 close, 2026-11-17.)

### `examples/quay`

- [ ] -> examples/quay: **Set `lot_depth = 24` and `max_relief` in quay and re-measure expect.toml.**
  The town-lib pass changed only lib/town; quay as written now gives 15 blocks, 174 lots, 10 streets,
  29 houses (expect.toml: 16, 178, 11, 30; HEAD's lib/town on today's crates already gives 181 lots).
      (From building quay, 2026-11-15.)
- [ ] -> examples/quay: on foot the deck still reads earth at the eye (`way_water`, `street_quay`,
  `wall_deck`) though the shore row comes after the street row; check how rows combine. The wall face
  shows only as a grey band from the water.
      (From building quay, 2026-11-15.)
- [ ] Footway hummock numbers (quay max 0.227 m) are the streets agent's, never re-measured.
      (From the M3 close, 2026-11-15.)

### `examples/strait`

- [ ] strait: no step at the join (the ferry is the only crossing; R10's join layer has nothing
  to read yet); `harbour`'s moved slipway leaves `harbour_house` in the lane's keep_out.
      (From building strait, 2026-11-16.)
- [ ] strait: the ferry is not drawn; no view shows the crossing between the imports.
      (From M4 slice 2, 2026-11-16.)

### `examples/townland`

- [ ] -> examples/townland (s4 town pass): lib/review/form.py now counts ends on a district edge or at a
  gate as `edge_ends`, apart from `dead_end_count`; townland expect `base.dead_ends` (18) must be
  re-recorded (18 were 15 at the edge and the rest at gates or inside) and `dead_end_at_edge` read from
  `edge_ends`. Done (townland pass): `base.dead_ends` [0, 0], `base.edge_ends` [16, 16].
      (From building quay, 2026-11-15.)
- [ ] -> examples/townland: **One old-town street ends on the embankment** (`d68`). Step 6 drops the
  rest within `spacing / 2` of `quay_road` and `high_street`; `cross = 30` gave an `authority.max_change`
  of a terrace (3.10 m against 3, at (299, 198)) on the lib/town of that hour. Re-try with the town
  pass's end-on-a-main-way rule landed. `from_east_gate` sees no town over the hill's shoulder; move
  it to the east road's crest. move_hill empties the old town (52 houses): the districts are written
  where the hill was.
      (From building quay, 2026-11-15.)
- [ ] -> examples/townland: one `authority.squeezed` at (202, 327): the embankment's faded wall and the
  crossing's deck edge at partial masks. Fade the wall to 0 before the crossing's `width / 2 + 0.25`.
      (From From M3 slice 5 (ways, 2026-11-15).)
- [ ] townland `move_hill`: the district regions stay where the hill was, so the old town empties
  (56 houses, base 97); districts should follow the hill (a region by name round it).
      (From the M3 close, 2026-11-15.)
- [ ] townland: the river is a written straight course of one width (row 62 regressed from the
  spike's bends and estuary); `from_east_gate` sees no town.
      (From the M3 close, 2026-11-15.)

### The packs (`lib/packs`)

- [ ] The real kit: the TODO asks for a box kit then a real one; only the box kit is built.
      (From M5 slice 3, market, 2026-11-17.)

### Every example

- [ ] **The on-foot review of M2 worlds was never run** (disk full). Run it with M3's first sheet.
      (From the M2 close, 2026-11-15; was -> M3.)
- [ ] -> examples/measure: the "meetings" block is the same 120 lines in four measure.py files (each
  example is one standalone file); move it to one shared module if a fifth world needs it.
      (From the M3 slice 5 review on foot, 2026-11-15.)
- [ ] The "meetings" block in four measure.py files; the probe in make_m3_sheet.py is a fifth copy
  of the same idea. One shared module when the next world needs it.
      (From the M3 close, 2026-11-15.)

## Loaders

### `engines/godot`

- [ ] A kind surface with no material is drawn dark grey: a storey's floor, whose region is the
      outline, square at a corner, shows as a dark triangle through Kenney's bevelled corner
      blocks at every storey of `examples/borrowed` (`docs/reports/borrowed-sheet.png`). Draw a
      kind surface without a material as its ground's paint, or not at all where a piece stands
      over it. (From K10's review, 2026-11-24.)
- [ ] A joint is a row of `meets.json` (surfaces.md 3.3, R76), and the loader makes a
  `NavigationLink3D` of every row (build.md 15): two pipe pieces joined 5 m up get a link no
  walker should take. The loader cannot tell a walk from a pipe: only the connector's word
  differs. Decide who says which connectors are walked (a library, by the word), and have the
  loader read it. It also reads no `rise` from the manifest's `mounts`.
      (From K9 of docs/plan-kits.md, first slice, 2026-11-23.)

- [ ] Sheet walks go straight at `toward`, so no walk climbs a spiral flight (station stair_1 to 3);
  a walk along a path (`toward` a flight's end, following its line) would. Closing a passage takes
  the meeting away, not the way: the walker still crosses `tower_step` with it unset.
      (From M5 slice 1, 2026-11-16.)
- [ ] Spiral walks need `along` (build.md 15.1, lib/review `bookmark.along`, text, not a name
  attribute: decided in slice 3, open to the owner); a lint that every flight is walked by some
  bookmark is not written.
      (From M5 slice 2, 2026-11-17.)
- [ ] station `outside` (M5 close: still 13.9 m short at the kitchen table): a view whose target no straight line reaches ends `blocked` on the sheet
  however it is aimed; a walk to a place inside (a `along` path through the rooms) or a review that
  reads a blocked walk by furniture as no fault is owed.
      (From M5 slice 3, 2026-11-17.)
- [ ] townland `ways_meet` walks from the bridge's east end toward its start and stops at the
  bank: `along` walks a path from its start only; a reversed `along` (or from the nearer end) is
  owed. city `on_central_bridge` and `on_harbor_bridge` look down river from mid-deck and now say
  `fell` off the deck's side (no parapet): aim them along, or run `parapet` (next item).
      (From M5 close, 2026-11-17.)
- [ ] Streaming (M6 slice 1, stream.gd) left: reload only the chunks whose `digest` (build.md 5.2)
  changed after a rebuild; walks streamed and whole agree on outcome and blocker in all 25 coast
  and city walks, but two stop elsewhere along the wall they press (coast `cliff_from_sea` 0.52 m,
  city `old_town_edge` 0.02 m), likely the order bodies enter the space; a headless walk's first
  frame is 133 ms (Godot's 8-step clamp after the start's loading), not the streamer's.
  Lesson: a renderer read-back (`Mesh.get_faces`) on a worker thread waits on the main loop, so a
  threaded chunk load hung on the fixture's `mesh` collision; make chunk data without read-backs.
      (From M5 close, 2026-11-17.)
- [ ] The loader draws the skirt's cut quads clipped to the strip (monastery `bm_tarn` fringe;
  build.md 15.3 glTF reader, slice 2 item).
      (From M5 close, 2026-11-17.)
- [ ] station `close_passage`: the sheet still walks through the closed passage (a leaf, not a
  meeting taken away); `outside` 13.9 m short at the kitchen table; coast `cliff_from_sea` and
  `along_cliff` blocked in every build; townland `from_east_gate` a grass slope; college blocks show
  no door at `*.entry` (on-foot review 8 to 10, all known).
      (From M5 close, 2026-11-17.)
- [ ] Left from the far layers: slivers where a loaded chunk's fine edge meets a coarse one;
  generated meshes (decks, floors) are not drawn past the load radius; a far house is a box, not
  its roof (a model's mean albedo). compare_views.py is not in run.sh (it needs a world build).
      (From M6 slice 1, 2026-11-17.)
- [ ] The headless "real time" walk gives 27,623 frames and 190.7 s in both modes: likely Godot's
  low-processor sleep; set `OS.low_processor_usage_mode = false` or say so.
      (From M6 slice 1, 2026-11-17.)
- [ ] engines/godot/tests/run.sh's reload step: its fixtures hold no chunk `digest`, so the suite
  sees only "made again"; give them digests so one chunk is kept and one made.
      (From M6 slice 2, 2026-11-17.)
- [ ] Far stand-ins pop at the load edge: 11 of 123 atlas views over 1% whole against streamed
  (vale-aerial 6.79%); shaded or rounder far trees, or a looser bound past the load radius
  (owner's).
      (From M6 close, 2026-11-17.)
- [ ] `city.old_town_street` streamed against whole (test_stream_walks, 1 FAIL): not a dropped
  collider (on-foot review 3's guess). Every house it meets has its body in chunk [10, 10], and
  streamed with every chunk loaded first it parts the same 18.54 m. The trails part by 1 mm 14 m
  in (the bodies enter the physics space in another order) and the walk grazes a house corner at
  101 m. Owner's (R48): a bound for walks that graze a corner, or one body order for whole and
  streamed. Lesson: load every chunk first before blaming the streamer.
      (From M7 slice 1, 2026-11-17.)
- [ ] **The sheet stalled once for 3 minutes** while another session's Godot drew, and the
  watchdog did not fire (a `mill` run). Small; time the watchdog per view.
      (From M7 close, 2026-11-17.)
- [ ] **A sheet of the world's own views**: an import's bookmarks are on the sheet too (18 of 21 in
  `crossroads`); an option to draw only the world's own.
      (From M7 close, 2026-11-17.)
- [ ] The loader reads the player map (`exports` by `output`): a minimap in the viewer.
      (From the list before the rewrite, Player map; was -> M6.)
- [ ] Find why the render script hangs on a second or third build in a row.
      (From the list before the rewrite, Surface and delivery; was -> M1 (the sheet tool must not repeat it).)
- [ ] Explain Coast's frame rate after parts were placed. Context: 22 fps in one capture, below the
      30 asked for, with about 9 million source triangles. Lesson so far: a part brings its whole
      budget with it and nothing in the master accounts for that.
      (From the list before the rewrite, Surface and delivery; was -> M6.)

### `engines/gltf`

- [ ] The glTF reader (build.md 15.3) is not written: only engines/godot exists, so nothing to
  skip cut quads in yet (change review 15). Owed with the glTF reader, M7 (engines).
      (From M5 slice 2, 2026-11-17.)
- [ ] glTF export: water (`water_level`) is not carried (sea and beck missing at coast
  `under_deck`); carry it per chunk, or say so in engines/gltf's README. The skirt is in "To M7"
  above.
      (From M6 slice 1, 2026-11-17.)
- [ ] A generated mesh with no `material` has no glTF material, so a reader takes glTF's default
  (a white metal); the glTF reader replaces it (15.3), others may not.
      (From M6 slice 1, 2026-11-17.)

### `tools/map_render.py`

- [ ] map_render's coast registration counts `beck` (water) as a way and skips the under-way
  `sea_cave` silently: rename the measure or list what it skips.
      (From M6 slice 1, 2026-11-17.)

## Proof

### Conformance cases and the runner

- [ ] **The check suite now needs the Rust engine built** for `basin`. Decide whether an outside
  program in a step is allowed in the suite, or whether its output should be recorded.
      (From simulating the land under stated facts, 2026-11-13; was -> M2.)
- [ ] **`d20_basin` stays pending**: the runner configures only `python`, and basin's step is the
  `via-terrain` program. Lesson: a case that needs a configured program needs a runner key for it.
      (From the M2 review fixes, 2026-11-14; was -> M2.)
- [ ] -> spec/cases (M3 slice 2): owed cases `geom.many_rings` on a maker (needs a `where` region),
  `protocol.output_table` for a returned `[network]`/`[cells]`, `of` a cells table without `class`,
  `min_length`/`min_area` merging through text, rest `kind` null; identity 27 and 28 are pending.
      (From building quay, 2026-11-15.)
- [ ] Case details that depend on the implementation: the line numbers in `anchored_clash` and
  `after_dropped`, the node ids in `moved` and `moved_square` (`town.step:lay/i:districts.d1`);
  settle them against the first implementation, not the other way round.
      (From M4 slice 1, 2026-11-16.)
- [ ] `footprint_ref` checks no point of the feature that `follows` `a.footprint`; `change_scoped`
  guards a pre-M4 rule (note it in the row).
      (From M4 slice 2, 2026-11-16.)
- [ ] Not run in the fix pass: a small-world probe that a changed import step code file re-keys
  the parent (review "Left").
      (From M4 slice 4, 2026-11-16.)
- [ ] A derived field read by its offset has no case at a wall cell where the derived field is flat
  (an underwater quay face, review issue 2): the code reads both at one post since the fix pass;
  a case with exact post values is owed.
      (From M4 slice 4, 2026-11-16.)

### The repository checks

- [ ] `xtask check`'s kinds-file rule reads text: a step that builds the name `kinds.toml` another
  way, or reads it with another parser, is not caught (protocol.md 7).
      (From M7 slice 1, 2026-11-17.)

### Runs by an agent that did not write the tool

- [ ] Plan step 6 for M3: a fresh agent builds a small variation from the documents alone. Not run.
      (From the M3 close, 2026-11-15.)
- [ ] pistol/sidearm: a fresh agent, given only the READMEs and spec/assets.md 12, builds a
  variation (a fourth accessory on a new socket, or the sidearm racked half way).
      (From M5 slice 2, 2026-11-17.)
- [ ] **The Sonnet `mill` run's scratch kept its target, store and build** (on-foot review 6,
  748 MB, and a stray `world/orld.toml`), though its report said deleted; not ours to remove.
  Check the outsider's cleanup line against `du` before trusting it.
      (From M7 close, 2026-11-17.)

## Record

### `docs/`

- [ ] Not re-read in the revisit of 2026-11-18: `docs/faq.md`, `docs/challenges.md`, and the three
      design notes dated before the first cut (`docs/design/blackbox.md`, `ways-and-assets.md`,
      `limitations.md`). Fold what still holds into `spec/` or the ledger, and say what does not.
      (From the revisit, 2026-11-18.)
- [ ] Review `spike-final:spike/SPEC.md` section 11 (known limitations and inconsistencies) and sort each line
      into: fix in the design, accept and document, or drop. Why: it is the most complete list of
      what the current model gets wrong, and the rewrite should start from decisions on it.
      (From the list before the rewrite, Spikes; was -> M0.)
- [ ] The review's "remove dead `is_path`" found nothing dead in crates/ (every `is_path` is read).
      (From M5 slice 1, 2026-11-16.)
- [ ] **When `rewrite` is committed, `git add` the untracked files**: `sdk/python/README.md` (README
  links it) and 27 new case folders under `spec/cases` (32 with `trips2`'s `kind_empty`,
  `subject_imported`, `weir_off_lattice`, `weir_off_lattice_r2`, `kindless_returned`); a patch of
  the tree needs `git diff --binary` (basin's `.npy`). Lesson: a fresh-worktree check of an uncommitted tree copies the
  untracked files too, or it tests less than the commit.
      (From M7 close, 2026-11-17.)

### The ledger (`docs/ledger/`)

- [ ] What the ledger still owes is listed in `docs/ledger/README.md`, "Owed" and "Depth of
      evidence": scenes per section, typical sizes, links between rows, the operations an author
      performs, and thirteen subjects still to read from sources.
      (From the revisit, 2026-11-18.)
- [ ] Answer the 22 questions a stylised themed-region open world puts to the model.
      Context: read on 2026-11-13 from one published map and screenshots of it (`docs/ledger/world.md`,
      "Themed regions" and the two subsections after it). Lesson: the genre moves the dials (more is
      specified, less is derived) and keeps the archetypes; each question is a dial with no setting yet.
      1. Is a world one continuous landscape, or a set of regions built and shipped separately? What is the unit?
      2. When a region's biome is the author's decree, what remains of climate and ecology inside it?
      3. What makes a seam between two unrelated regions believable?
      4. How is a finished region kept unchanged while a neighbour is added?
      5. How does a world end, and what is the sea made of?
      6. How are stacked and disconnected spaces described when one height per location is not enough?
      7. How does what the player's body can do (climb, glide, swim) turn into rules about cliffs, gaps and water?
      8. What are the proportions of a compressed world: how big is a mountain, a town, a house, against the player?
      9. How is "one landmark, visible from everywhere in its region" stated and checked?
      10. How are fast-travel points and rewards spaced, and what does that spacing do to paths?
      11. What is shared between two regions' towns, and what is swapped?
      12. How does a place carry more than one state over time?
      13. How much of such a world is specified outright, how much derived, and how much imported?
      14. What does the drawn map need from the world to be produced?
      15. Where does a setpiece end and filler begin, and how do the two join?
      16. Is shelf terrain a kind of terrain generation, or a different thing from erosion-style landscape altogether?
      17. What makes an expanse empty on purpose rather than unfinished?
      18. Why are paths so sparse, and what guides the player where none is drawn?
      19. Are a stylised world's shelves a height field with rock meshes laid against it, or one solid form?
      20. If ground cover is only "top or side", what is left for fields and analysis to decide?
      21. What stays the same and what is swapped when one landscape is given a different style?
      22. Can "helps the picture" (an accent tree, framing, a skyline) be stated as a rule, or only judged?
      (From the list before the rewrite, Genres: a stylised themed-region world; was -> none yet (open; in no milestone).)

## Set aside

- [ ] -> set aside until a world needs it (relief C): `smoothed_along` shrinking its window
  symmetrically at a path's end. It changes ford's bytes and formulas.md 8.5's vectors; the road
  kind's 1.7 m step at an end stays, and `lib/ways` documents keeping 40 m from the edge.
      (From the M2 integration, 2026-11-14.)
- [ ] -> set aside (doctrine section 7): Decide the boundary for sky, clouds and backdrop. Context: asked while matching reference
      photos, where sky and distant land carry much of the picture. Working position: sky and cloud
      rendering are the engine's; time of day and sun direction are world facts we carry; and what
      lies beyond the playable edge (open sea, distant ranges, a skyline as billboards or impostors)
      is world content we should place and could derive from our own neighbouring maps, as we do the
      minimap. Stylised fantasy maps lean on that backdrop heavily.
      (From the list before the rewrite, Look and reference.)
- [ ] -> set aside (doctrine section 7, image-model map styling): Define a registration measure for a restyled map that works on painted water, roads and
      footprints. Context: the spike's colour-based water overlap failed on pale washes; only the
      shore distance was trustworthy. Nothing is published. Needs several seeds before any number
      is quoted.
      (From the list before the rewrite, Player map.)
- [ ] -> set aside (doctrine section 7, image-model map styling): Decide how a single changed map tile is repainted in a consistent style. Context: the spike
      fingerprints 64 tiles and found one changed after a level edit, but never repainted it. This
      is the map's version of hand edits surviving regeneration.
      (From the list before the rewrite, Player map.)
- [ ] Work down the "Owed" table in `docs/challenges.md`: six requests that have only talk or a
      proposal behind them. Lesson (2026-11-12): a request answered in prose leaves no evidence
      for the rewrite; build a crude demonstration first, then analyse.
      (From the list before the rewrite, Surface and delivery; was -> M2, M5 and M6, row by row (docs/challenges.md, "Rows by milestone"); row 41 set aside (doctrine section 7).)
- [ ] -> spec/build.md (R27 rules Godot for the first cut and Unreal set aside; close when `spec/build.md` holds the Unreal mapping table): Design the engine target strategy: Godot and Unreal mapping, and what "first-class" means for each.
      (From the list before the rewrite, Surface and delivery.)

## Lessons and decisions, by the slice that made them

Kept from the log this file was: what each slice of the rewrite learnt or decided, apart
from any one item.

### From the M3 close (2026-11-15; docs/reports/m3-sheet.png, the slice 5 reports)

M3 closes on paper with these open. Detail for most is in the sections above; each line here says
where it goes. Lesson: M3 carried four core items (incremental cost, `run`/`set`, way points by
name, lot names at size) that its worlds needed and did not get; carry them first, not last.

### From M4 slice 1 (2026-11-16; the change, on-foot and gate reviews, and the fix pass)

The fix pass fixed: run members' footprint keys (example `third_cottage`, `move_tarn`), run and
set-item ranks, the duplicate r50, quay `move_hill`'s three name rows (the names fix keeps 18 more
lots), and composition.md's graph (one `placed` node per scoped name, statements without `file`,
`joincheck` nodes), brought-entry order, given feature names and attributes, R13 across imports
(9.1), R36 recording R10's amendment. Lesson: two reviews from different angles (diff, on foot)
found different blockers; the gate alone found none of them.

### From building strait (2026-11-16, M4 slice 2; examples/strait)

Fixed in the slice: a scoped region lost its outline, so `on = { ref = "a.isle", t }` was
`geom.many_rings`; a brought feature's name attribute kept the import's name and frame (`a.at_slip`
looked at (186, 108), A's own point, named `slip`); an import's `changes` path was read from the
process's folder, not the import's; the instance columns and `features.json` did not read
`import:<id>`, so `harbour` built on the base's store lost A's new boathouse (47 against 48 fresh).
Lesson: build every change on the base's store as well as fresh; the fresh build hid all of it.

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

### From the M4 close (2026-11-16; docs/reports/m4-sheet.png, the slice 3 reports)

Lesson: M4 slice 2 took 11 hours. A slice that grows past its box hides what is late; slice 3 ran
five agents in 90-minute boxes and wrote what did not fit here instead of carrying it. Keep that.
M4 closes on paper with its third acceptance unmet: a moved square is not shown (below, first).
The open "-> M4 slice 4" items above are re-sorted here; check them off in both places.

### From M4 slice 4 (2026-11-16; the change, review, fresh-agent and gate runs, and the fix pass)

Lesson: the wall rule was written for ground and applied to every field read; a threshold in metres
must say which fields are metres. A rule's unit is part of the rule.

### From M5 slice 1 (2026-11-16; the change and on-foot reviews, and the fix pass)

Lesson: a write node keyed by its reads' values misses what only a statement says: unsetting
`ends_at` changes no shape, so `meets.json` stayed stale while every node read was unchanged. A
file made from declarations holds those statements in its key.

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

### From M5 slice 3, levels and sheer faces (2026-11-17; examples/monastery, city; lib/land, lib/ways)

Lesson: a world not rebuilt after a library change that moves a surface hides OWED rows for two
slices: monastery and city (decks as surfaces, slice 2) failed `deck_wet` and `reach` at HEAD
already; rebuild every world a library change reaches in the slice that makes it.

### From M5 slice 3, town kit (2026-11-17; corners, wedges, kit runs; examples/quay)

Lesson: a fallback chosen before rule 4 (keep_out) never ran in quay: the rows fitted the lots'
geometry and were then dropped by the footway's keep_out, so `fallback` saw no empty lot. A rule
that answers "nothing stood" must run after every rule that removes, in the same batch.

### From M5 slice 3, market (2026-11-17; the box kit and its three changes; examples/market)

Lesson: a library that lays a kit run as a step's instances gets none of the core run's
exemptions: 86 false `place.overlap` (sub-millimetre, by rounding to the lattice off the axes, and
on bends) until kit.py kept neighbours apart itself (rule 7). A rule written for a core form must
say whether a library's lowering of that form inherits it.

### From lib/review reach over kind surfaces (2026-11-17, M5; townland, monastery, city, quay)

Lesson: a library lint that needs the engine (`surfaces_at`, `meetings`) runs only as a worker; a
plain command's `NotAWorker` turned into a silent terrain-only walk would have hidden the decks
again, so the lint now says `terrain_only` (info).

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

### From M5 close (2026-11-17; the fresh agent, the change and on-foot reviews, and the fix pass)

Lesson: a fix closed at the case it was written for and not at the world that found it: the slice-3
target fix covered a point `on` a deck, and station's tower views still looked through their
floors. Close a TODO item from the world's own render, not from the case alone.

### From M6 slice 1, perf (2026-11-17; city and quay, before and after in examples/*/expect.toml)

Lesson (M6 slice 2): the segment index that sped up nearest-place queries near a shape made a
field's window three times slower (city's distance nodes 18 -> 64 node-seconds), since most posts
lie far from the shape and its ring search walked out to them. Time the node family in the run
report before and after a "faster" search, not the search alone.
Lesson: the houses steps were not the whole of a change's cost; once they ran per block, the
change took as long, held up by the engine's own serial work. Profile the whole change build
before choosing the cut.

### From M6 slice 1 (2026-11-17; the change and on-foot reviews, and the fix pass)

Lesson: a key that names the file a node writes is not the key of what the node computes from it.
`budget:chunks` read `names.txt` because the counts sit beside the names, but the counts come from
positions and assets that file does not hold. Key a derived node on what decides its value, and
write the case that changes only the undecided part.

### From M6 slice 2 (2026-11-17; atlas, export, perf, stream and map; the change and on-foot reviews, and the fix pass)

Lesson: two reviews found five atlas faults that measure.py never saw (no road, no buildings,
dropped surfaces, water at joins, a cliff exit), because each row measured the plan (features,
outlines, medians) and none what stands on the ground. An example's first rows should sample the
finished fields along what a player walks.

### From M7 slice 1 (2026-11-17; the change and on-foot reviews, and the fix pass)

Lesson: a dotted name is not always a scoped one. `from` took any first part as an import, and a
world's own set item (`kitchen_table.candle`) named an import that does not exist; read a scope
against the table that declares it.

### From M7 close (2026-11-17; the outsider runs, `trips`)

Decided in this pass, open to the owner: a stairwell stopping 0.5 mm to 0.3 m short of a corner is
refused by `lay` (`STUB_MIN`, ours); the nearest-name list is nearest first (diagnostics.md 1.4,
ours); a place within a millimetre of a turned import's footprint is on it, so held
(`place_inside`, composition.md 7's edges are inside); a deck under its water stays `low_deck`
(warning), worded "under the water"; a world's diagnostics name an import outside its folder by a
climbing relative path, not an absolute one (`build.md` case 7). Lesson: the trips that cost most
were where a value had to be carried across a seam by hand (a lift, a building's place, an eye's
height); messages were rarely the cost.
Decided in the second run (`trips2`) and its fix pass, open to the owner: rulings R60 to R63 (the
weir's radii, a returned bookmark's `along`, a step's report on an import's feature landing on the
import, an absolute import shown climbing).
From the second run's change and on-foot reviews (fix pass 2, 2026-11-17; fixed: a step's subject
`feature.<import>.<name>` now maps to the import (R62, case `subject_imported`), the weir's radii
at 2 m posts (R60, case `weir_off_lattice_r2`), the `for_each_empty` hint's wording, the
kindless-path chain tested (case `kindless_returned`, SDK test), the "met by both" count (8; the
layer split was Sonnet's alone), the walker's `through` naming far trees (`Walk.inside`,
test_walk), the anchored-beck note in lib/water, spec conformance rows, rulings R60 to R63):
