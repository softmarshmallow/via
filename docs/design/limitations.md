# The spike's limitations, sorted

Every known limitation of the spike, with what v1 does about it. The sources are `spike/SPEC.md`
section 11 ("Known limitations and inconsistencies"), section 12.8 ("Limitations added") and the
"Known weak points" list in `docs/design/concepts.md`. One row per source bullet, in source order,
grouped as the source groups them; two bullets with two answers are split (`a`, `b`). A last section
holds limitations recorded elsewhere (`spike/FINDINGS.md`, `TODO.md`, `docs/challenges.md`) that the
rewrite would otherwise repeat. This is the M0 sort the plan asks for (`docs/plan.md`, M0 step 3).

Verdicts:

| Verdict | Meaning | "Where" says | "Proven by" says |
|---|---|---|---|
| fix | v1 removes it. | The spec chapter (`spec/<name>.md`) or the library (`lib/<name>`) that defines the fix, and its milestone. | The conformance case (chapter and case name or number) and the code, where the fix is a diagnostic. For a fix after M1, the plan's acceptance line or world. "Owed" means the chapter has no case yet; it must gain one before code for that milestone starts. |
| accept | v1 keeps it on purpose. | Where it is written down for users, and why it is acceptable. "Owed in <chapter>" when the text is not yet written. | The case that shows the kept behaviour, where one exists. |
| drop | It no longer applies: the mechanism or word behind it is gone. | What replaced it. | The case that rejects the old word, where one exists. |

"Until then" is filled for every fix whose milestone is later than M1. It says one of three things:

- `doc.not_yet`: the fix rests on a key listed in `spec/vocabulary.md` section 5, so the M1 parser
  rejects it and the limitation cannot be met silently.
- not reachable: nothing in M1 can express the case (the library that owns it does not exist yet).
- gap: M1 has the limitation, and the cell says what M1 does meanwhile.

Spike words appear only to identify the bullet. The v1 words are in `spec/vocabulary.md`; section 6
there maps one to the other. Codes and section numbers are those of the chapters on the day of this
sort. Duplicate ids have two candidate codes, `doc.duplicate_id` and `identity.duplicate_id`; which
one survives is (open), decided by the lead before the code registry is written
(`spec/diagnostics.md`, Open). Rows below write "the duplicate-id code".

A workspace test that reads this page and fails when an M1 fix row names no case, or a case its
chapter does not hold: (open), proposed for M0 step 5; decided by the lead.

Row ids: `S11.<group>.<n>` for section 11, `S12.<n>` for section 12.8, `W<n>` for the weak points,
`E<n>` for limitations recorded elsewhere.

## Section 11

### Fields and materials

| # | Limitation | Verdict | Where | Proven by | Until then |
|---|---|---|---|---|---|
| S11.F.1 | At most eight ground materials; the rest are lost with `ground.too_many`. | fix | `fields` 1 and `build` 6.2: no cap in the core; a chunk's weights file holds one plane per material present there. A loader's own layer limit is the loader's to report (`load.too_many_materials`, `build` 15.1), never a silent cut. M1; at scale in M6 (`atlas`, eleven materials). | `build` case 11 (twelve materials). | |
| S11.F.2 | The material field must be named `ground` and the height field `height`; much breaks otherwise. Both are required. | fix | `fields` 2.1: the core finds fields by role (`surface`, `material`, `keep_out`), never by name (principle 5). `surface` is required once anything asks for ground (`field.role_missing`); `material` is optional. M1. | `fields` cases "no surface", "features only". A world with no `material` field: case owed in `fields`. | |
| S11.F.3 | `water` is read by name in six places; `bed` and `hole` are made by the compiler. | fix | `fields` 2.2: water is a field a library finds by tag; `build` 6.4 hands it over as a level, no sentinel. `bed` and `hole` go with the one surfaces model (`surfaces`, R25). M1 for water; M5 for floors. | `fields` case "tag selector". Floors: `station` (M5). | `doc.not_yet` (a second `surface` field, `hosts`). |
| S11.F.4 | `grass` matters only to the viewer; `clear` matters only through shared kinds and the pack's `avoid` lists. | fix | `fields` 2.1: the `keep_out` role replaces the field `clear`; `assets`: `avoid` is gone. The loader reads materials through the pack (`build` 7), not names. M1. | `rules` r17; `fields` case "keep_out max". | |
| S11.F.5 | The tag `vegetation` is hard-coded in the part blend. | drop | The part blend is gone. An import's `blend` is a formula the user writes (`vocabulary`, Imports table; R8). | | |
| S11.F.6 | `unit` is not a unit: any value switches on anchor comparison. | fix | `fields` 2.4: `unit` is a unit only. `authority` 5: anchors are always compared (principle 3). M1. | `authority` cases `agree`, `pond_lake_wide` (`authority.conflict`). | |
| S11.F.7 | A second material field silently overwrites the first one's material list. | fix | `fields` 2.1: one field holds the `material` role; a second is `field.role_twice`; another mix is a `weights` field with no role, handed over as data. M1. `vocabulary`'s roles table still says "more: (open)"; raised with the lead. | `fields` case "roles" (`field.role_twice`). | |
| S11.F.8 | Reading a volume field with `f('name')` in a 2D expression fails. | fix | `fields` 8 and `formulas` (field reads by rate). M5. | `fields` case "volume"; `coast` (M5). | `doc.not_yet` (`type = "volume"`). |
| S11.F.9 | Expressions are Python `eval`; `min` and `max` take two arguments; a parameter can shadow a function. | fix | `formulas`: our own grammar (R6); `min` and `max` take two or more (8.1); shadowing is `formula.shadow` (5). M1. | `formulas` cases 6, 7 (`formula.shadow`). `min` of three: case owed in `formulas`. | |
| S11.F.10 | 2D and 3D noise functions differ, and `pi`, `exp`, `pow` are missing from volume expressions. | fix | `formulas` 11: one function table for every context. The 3D names are (open), decided in `formulas` for M5 (`formulas` Open). | `coast` (M5). | `doc.not_yet` (`type = "volume"`). |
| S11.F.11 | A failed 2D expression becomes 0; a failed volume expression becomes `ground`. | fix | `formulas` 9: every failure is a located error; no non-finite value enters a field (R26). M1. | `formulas` cases 19, 21 (`formula.not_finite`), 22 (`formula.range`). | |

### Kinds, effects and authority

| # | Limitation | Verdict | Where | Proven by | Until then |
|---|---|---|---|---|---|
| S11.K.1 | The kind names `bookmark` and `poi` are hard-coded; a kit run's pad kind defaults to `pad`. | fix | `build` 9.2: library kinds hand their features over through `output`; the core knows neither word, and the deny-list test keeps the nouns out (`vocabulary` 7). M1 (`ford` uses a bookmark). The kit run is a building library form that lowers to a run (`rules` 10.1); its pad kind is the library's. M3 (`city`; `authority` Open). | `build` case 19 ("Outputs"). Kit form: `rules` r25 (M3). | Kit form: not reachable (no building library). |
| S11.K.2 | The number of entries in `over` changes effect order; which ids it names does not. | drop | The spike's feature key `over` is gone (`vocabulary` 6). A clash between anchors is reported as `authority.conflict` (error) or `authority.squeezed` (warning), never silenced (`authority` 5). Order is `authority` 4. | `authority` case `no_over` (`doc.unknown_key`). | |
| S11.K.3 | Among anchors only regions are ordered by area; anchored paths and points come after. | fix | `authority` 4, one order for every shape (R21). The start (`then` rows) comes first, by layer rank, `order`, id (4.1). Effects follow by authority, `order`, nesting, layer rank, feature id, effect id (4.2). Area is no longer a sort key. It enters only through nesting (4.3): a contained, smaller region comes after its container, and paths and points nest inside regions that contain them. M1. | `authority` cases `order_key` (challenge 36, across layers), `nest_inside`, `nest_on`, `owner_holds`. An anchored path inside an anchored region: case owed in `authority`. | |
| S11.K.4 | A kind's `let` that fails is skipped in 2D, raised in a volume, and some errors are not caught. | fix | `formulas` 9: a `let` failure is a located error, in every context; nothing is skipped. M1. | Case owed in `formulas` (a failing kind `let` in a 2D effect). | |
| S11.K.5 | `seam.grade` runs for every scalar field a path's kind touches, not only height. | drop | Seam checks are gone. A grade check is a library lint, and it names its field by role or tag. | | |
| S11.K.6 | `effect.unbounded` is not checked where an effect's window touches the world's edge. | fix | `fields` 5.3: `field.effect_cut` is checked on the ring, which leaves out posts on the extent's edge. An effect cut by the extent is not reported, by design: the world's edge is not a seam. M1. | `fields` cases "cut", "fade", "world edge"; `numbers` `window_equals_whole`. A feature on the edge, equal to a larger extent cut down: case owed in `fields`. | |
| S11.K.7a | `seam.change_exceeds` is an error, but the change is still applied. | accept | `authority` 2.2: `effect.max_change` (error); the entry is applied in full so the report shows what was asked. `diagnostics` 4: a build with an error writes only `diagnostics.json`, so nothing applied reaches a loader. M1. | `authority` case `max_change`; `build` case 16 ("Failed build"). | |
| S11.K.7b | `kit.misfit` is an error, but the bays are still applied. | fix | The fit is a building library lint with its own codes (`rules` 10, 10.1). M3 for the kit form (`city`), M5 for the town kit. | Plan M5: "the town kit's three owed changes". | Not reachable (no building library). |
| S11.K.8 | Effects of an instance's kind placed on a derived feature never reach height. | fix | `rules` 2.1 and `authority` 4.3: `ground = "<kind>"` makes the footprint a feature of that kind, ordered like any other; a footprint of an instance placed `on` A nests in A. M1 (`ford`: the cottage's pad). | `rules` r03, r23; `ford` "Pack swapped: the pad grows". | |
| S11.K.9 | Effects on volume fields ignore authority, `over`, `reach`, `max_change` and tags. | fix | Decided: an effect on a volume field is reached by a role or tag selector (`fields` 2.3) and bounded by its `radius` (`fields` 5.3). Whether it obeys the tiers is (open), decided in `authority` for M5 by `coast` (`authority` Open). M5. | `coast` (M5). | `doc.not_yet` (`type = "volume"`). |
| S11.K.10 | A feature without `path3` whose kind writes a volume field touches every volume box. | fix | `fields` 5.3: an effect is bounded by its `radius` when it has one. With none it covers the whole extent, by statement, and the change report marks the whole field when its feature moves; that is legal, so the fix holds only for effects with a radius. `path3` is (open), decided in `surfaces` for M5. M5. | `fields` case "bounded" (the 2D rule); `coast` (M5). | `doc.not_yet` (`type = "volume"`). |
| S11.K.11 | `along` on a region or a point raises `expr.invalid`. | fix | `formulas` 8.4: `along` is defined on regions (outline fraction); on a point it is `formula.shape` with a hint. M1. | `formulas` case 18 (`formula.shape`). `along` on a region: case owed in `formulas`. | |

### Features

| # | Limitation | Verdict | Where | Proven by | Until then |
|---|---|---|---|---|---|
| S11.Fe.1 | A feature with both `flood` and `on` raises `feature.duplicate`; a flood needs `at`. | drop | Floods are not core. A lake at a level is a water library kind (R14), placed by the one grammar (`at` or `on`) like any feature (`geometry` 2.5). | | |
| S11.Fe.2 | `on.slide` is read for instances but not for features. | drop | `slide` is gone; `t` says the same (`vocabulary` 6). | | |
| S11.Fe.3 | A feature `on` another placed feature must come later in the document. | fix | `graph` 2: every reference is an edge, resolved in graph order; table order never changes a result (`vocabulary` 2.2). A real loop is one `graph.cycle` (2.1). M1. | `graph` cases "shuffle", "cycle two"; `documents` case `shuffle`. | |
| S11.Fe.4 | An opening's `on.offset` moves its point but not the hole cut in the wall. | fix | Openings are a building library kind; its cut reads the feature's resolved place, which `on` sets once (`geometry` 2.5). M5. | Plan M5 (`station`, `college`). | Not reachable (no building library). |
| S11.Fe.5 | A stair needs `at` besides `arc` or `line`, else an exception; `width` and `riser` ignore kind defaults. | fix | `authority` 1: kind `defaults` apply to every attribute. `diagnostics` 1: no failure without a code and a hint. Stairs are a building library kind, M5. | `documents` case `unset_default`; `diagnostics`' first case (every record has a code and a hint). | Stairs: not reachable. |
| S11.Fe.6 | A door's target `outside` is reserved; a room with that id cannot be told apart. | drop | The core has no door or room (principle 5). Reserved: the id `world`, and the place names `start` and `end` (`vocabulary` 2.1). | `identity` case 1 (`world` as an id). | |
| S11.Fe.7 | One overhang per column: `height` is the top, `bed` the lowest floor; the rest is mesh only. | fix | `surfaces` (R25): a column may hold any number of surfaces, and every one is asked the same four questions (2). M5. | `coast`, `station` (M5). | `doc.not_yet` (a second `surface` field). |
| S11.Fe.8 | A volume's box is its region's axis-aligned bounding rectangle. | fix | `fields` 8 and `surfaces`: a volume's extent is its owner's region. The exact bound is (open), decided in `surfaces` for M5. | `coast` (M5). | `doc.not_yet` (`type = "volume"`). |
| S11.Fe.9 | Lights are collected only when the world declares levels. | drop | Levels are gone (`storey` is a building library word). A light is a library attribute handed over through `output` (`build` 9.2). | | |
| S11.Fe.10 | A world's water has no generated geometry: it is the `water` raster. | accept | Documented in `build` 6.4: water is a level field a library writes and a loader draws where depth is above 0. Drawing it is the engine's (principle 12). A water body that needs its own surface is a library surface (R25), shown in `coast` (M5). | | |

### Instances and producers

| # | Limitation | Verdict | Where | Proven by | Until then |
|---|---|---|---|---|---|
| S11.I.1 | `atop` on a host with no floor height puts the item above z = 0, not above the terrain. | drop | `atop` is gone. Height comes from the surface beneath (`surfaces` 4, R25). | | |
| S11.I.2 | `yaw` is ignored with `on`; `face` and `turn` with `at`; `against` overwrites `on.offset`. | drop | Those keys are gone. One grammar: `at` or `on`, and `face`, valid with both (`geometry` 2.5, `vocabulary` Placement). | | |
| S11.I.3 | `[[instance]].on` must be a table and `[[run]].on` a string. | fix | `geometry` 2.5 and `vocabulary` Placement: `on` is always an inline table (`{ ref, t, offset }`; runs `{ ref, span, offset }`, `rules` 5). M1; runs M3. | `documents` case `syntax_forms`; `geometry` case "`on` a path at `t = 1`". | Runs: `doc.not_yet` (`[run]`). |
| S11.I.4 | Instance ids are not checked for uniqueness; the default id depends on the index. | fix | `identity`: every written instance has an id in one namespace; a duplicate is the duplicate-id code, naming both. Produced instances are named by place. M1. | `identity` case 1; `documents` case `dup_layers`. | |
| S11.I.5 | An instance outside the extent is dropped but still serves as a host. | fix | `rules` 8.4: a hand placement whose origin is outside the extent is `place.outside` (error); a produced instance outside it is not made (`run.outside`, warning; a scatter's lattice stops at the extent). Hosting: an instance that is not placed offers no mounts and no surface, and each thing placed `on` it is not placed, with `surface.host_missing` (error) (`surfaces` 4.1). An origin exactly on the extent's edge: (open), decided in `rules` before M1 code, with a vector for `ford`'s lane ends at x = 0 and x = 256. M1. | Cases owed in `rules` (`place.outside`; a thing `on` an instance outside) and the edge vector. | |
| S11.I.6 | A set passes only position, yaw and `in` to its members; `scale`, `tint` and the rest are lost. | fix | `rules` 4: an instance of a set carries its frame, `face`, `scale` and `tint` to every item. M3. | `rules` r04, r05. | `doc.not_yet` (`[set]`). |
| S11.I.7 | Free attributes include `over`, `authority` and kind attributes written on an instance. | fix | `documents` 7: an instance has closed keys; an unknown key is `doc.unknown_key`. The spike's feature key `over` is gone; an instance has no kind. M1. | `documents` case `unknown_key`; `authority` case `no_over`. | |
| S11.I.8 | A round asset's footprint feature is a rectangle; `round` is only for primitives. | fix | `assets` 6: round primitives (`cylinder`, `cone`, `sphere`) get a 16-sided polygon. `assets` 2.4: any asset may declare a polygon footprint. A model's measured footprint stays the plan rectangle of its `bounds` unless declared; a convex hull is (open), decided in `assets` for M2 by the real-pack swap. M1. | `assets` case "Primitive pack". | |
| S11.I.9 | Dress items that find no room are dropped without a diagnostic. | fix | `rules` 10.2: dress lowers to a scatter `within` a region. Every candidate that gives way is counted in `scatter.gave_way` (info, `rules` 7.5); a scatter that places nothing is `scatter.empty` (warning). A target count for dress, if one is wanted, is (open), to be raised in `rules` for M5 (`station`). M1 for the scatter. | `rules` r16. | |
| S11.I.10 | The asset key `clearance` is never read; asset `tags` are read only for `landmark`. | drop | `clearance` is gone (`assets`). Tags have no built-in meaning in the core (principle 5); libraries read them. | | |
| S11.I.11 | Primitive triangle counts are a fixed table; `cylinder` and `tower` count as 12. | fix | `assets` 6: a real table for primitives; `assets` 3.2: measured counts for models. M1. | `assets` cases "Primitive pack", "Triangle count". | |
| S11.I.12 | Scatter avoids only explicit solid instances, with a fixed margin of about 1 m. | fix | `rules` 7.1: density is multiplied by `1 - keep_out`, the `keep_out` role's per-post maximum. `rules` 7.5: a candidate gives way where its solid footprint overlaps that of any hand placement or non-scattered produced instance, and to relief; each is counted in `scatter.gave_way`. M1 (`ford`: no tree in the clearing). `assets` 2.4 also defines an asset `keep_out` distance in metres that `rules` 7.5 does not implement, under the role's name with another unit (principle 9); raised with the lead. This row claims only what `rules` says. | `rules` r16, r17; `ford`'s "no tree in the clearing" lint. | |
| S11.I.13 | Scattered instances have no id and appear in no layer file. | fix | `identity` 2: produced instances are named by place (`<scatter>#<place key>`); `build` 8 lists every instance by name. M1. | `build` case 14 ("Stable names"); `identity` cases 6, 7. | |
| S11.I.14 | Overlap is tested with convex polygons, between explicit instances only. | fix | `geometry` 1: exact integer predicates on any simple polygon. `rules` 8.1: overlap is tested across every producer; imports in M4. M1. | `rules` r16, r21 (`place.overlap`). | Imports: `doc.not_yet` (`[import]`). |
| S11.I.15 | `place.avoid` is skipped for an instance with a `kind`. | drop | An instance has no kind (`ground = "<kind>"` replaces it) and `avoid` lists are gone; `keep_out` applies to every instance (`rules` 7.1, 8.3). | | |
| S11.I.16 | Interior checks cover this world's explicit instances only. | fix | Library: `lib/building`. Interior checks are its lints; a lint reads every instance through the SDK (R19b, no private door; `protocol`). M5. | Plan M5: "indoors reachable through a door". | Not reachable (no building library). |

### Layers and merging

| # | Limitation | Verdict | Where | Proven by | Until then |
|---|---|---|---|---|---|
| S11.L.1 | Lists merge by `id` only when every item has one; `[[expect]]` and effect lists are replaced whole. | fix | `documents` 6.2: every entity is keyed by id; lists are values. M1. | `documents` cases `array_table` (`doc.syntax`), `change_inline`. | |
| S11.L.2 | A layer id clash is detected between layers only, not against the world's own file. | fix | `documents` 3: one world namespace across the world file and every layer (layers are section 5); the duplicate-id code names both, wherever they are. M1. | `documents` cases `dup_layers`, `foreign_patch`. | |
| S11.L.3 | `layer.direction` does not read ids in expressions, a door's or stair's `from` and `to`, or `over`. | fix | `documents` 5.1: direction is checked on every reference, formula reads included (`doc.upward_ref`). Doors and stairs are not core; their references are a building library's (M5). M1, proven by `documents` cases, not by `ford`, which has no layer. | `documents` cases `upward`, `upward_formula`, `kind_rank`. | |
| S11.L.4 | `[[layer]]` rows are read from the world file only; an overlay cannot add a layer. | fix | `documents` 6.1: a change may add or remove layers. M1; first used by a world in M3 (`townland`). | Case owed in `documents` (a change that adds a layer). | |
| S11.L.5 | An unknown variant and a layer id clash stop the run with a message, not a diagnostic. | fix | `documents` 6 and `diagnostics` 1: `doc.unknown_change` and the duplicate-id code are diagnostics with a code, span and hint. M1. | `documents` cases `unknown_change`, `dup_layers`. | |

### Parts

| # | Limitation | Verdict | Where | Proven by | Until then |
|---|---|---|---|---|---|
| S11.P.1 | A part is translated only, never rotated or scaled. | fix | `vocabulary` Imports table: an import takes `at` and `face` (R8 owes rotation). Scale stays out. M4. | Plan M4: "an import moved and turned". | `doc.not_yet` (`[import]`). |
| S11.P.2 | Parts do not nest; a part's own `[[part]]` rows are ignored without a diagnostic. | fix | An import's imports are followed, or rejected with a diagnostic, never ignored. Which one is (open), to be decided in `documents` 9 for M4 by `county`. | `county` (M4). | `doc.not_yet` (`[import]`). |
| S11.P.3 | A part and its master must share `resolution`; posts are copied one to one, unresampled. | fix | `fields` 4.3 (sampling): an import's fields are sampled into the parent's grid. M4. | `strait`, `county` (M4). | `doc.not_yet` (`[import]`). |
| S11.P.4 | A seam part's fields are blended without the lift `dz`; its water level stays at its own heights. | fix | `numbers` (frames) and `geometry` (datum and lift): an import's frame carries height, applied to every height field and height attribute alike. M4. | `border` (M4). | `doc.not_yet` (`[import]`). |
| S11.P.5 | A seam part's volume mesh stays rigid while the ground in the apron blends; a seam part's interior sets no `hole`. | fix | `surfaces` 3 (R25): where an import's surface meets the parent's is a meeting, checked to the millimetre. `hole` is gone. M5. | `coast`, `station` (M5). | `doc.not_yet` (`[import]`, a second `surface` field). |
| S11.P.6 | A portal part's `hole` covers its room polygons without their walls. | drop | Portal parts and `hole` are gone: an interior is an import placed on a mount, and floors are surfaces (R25). | | |
| S11.P.7 | A part's floor plans, levels and layer files are not brought; its bookmarks become the master's. | fix | Handed-over features (`build` 9.2, `output`) come with their place names: M4. The player map per floor, imports included: map library (`lib/map`, `build` 9.4), M6. | Plan M4 (`county`); plan M6: "the player map as data, per floor". | `doc.not_yet` (`[import]`). |
| S11.P.8 | Brought features keep unprefixed references (`from`, `to`, `joins`, `level`, `atop`, `group`). | fix | `identity` and `documents` 3: references inside an import resolve as `<import>.<place>` (R8). M4. | `county` (M4). | `doc.not_yet` (`[import]`). |
| S11.P.9 | A part's effects are computed in the part only; the master's fields are not re-resolved. | accept | A place is built alone and joined at its edge (R11). The parent changes imported ground only through an `after = "imports"` layer (R10; `vocabulary` Imports table; `graph` 9). A fill that must read its parent takes it through `give` (R11; `graph` 9). Written for users: owed in `documents` 9 for M4; the stub does not yet say it. | | |
| S11.P.10 | The master's pack wins on a name clash; a part may be drawn with another asset of the same id. | fix | Whether an import brings its own pack is (open) for M4 (`vocabulary` Imports table). Either way, one asset id bound to two different files is an error; to be stated in `documents` 9 for M4. | `county` (M4). | `doc.not_yet` (`[import]`). |
| S11.P.11 | A part's diagnostics lose their positions, and its infos are dropped. | fix | `diagnostics` 1.1-1.2: each diagnostic keeps its `position` in the world frame and its `chain` through imports. M4. | `border` (M4). | `doc.not_yet` (`[import]`). |

### Outputs

| # | Limitation | Verdict | Where | Proven by | Until then |
|---|---|---|---|---|---|
| S11.O.1 | `minimap.png` is always square; a world that is not square is stretched. | drop | The core writes no minimap. The player map is data from the map library (M6); styling it is the user's (doctrine section 7). | | |
| S11.O.2 | `plan_<level>.png` is written for this world's levels only. | fix | Library: `lib/map`, through `build` 9.2 and 9.4: the player map per floor, imports included. M6. | Plan M6: "the player map as data, per floor" (`atlas`). | Not reachable (no map library). |
| S11.O.3 | A path's points in `world.json` are every eighth sample; the last may be missing. | fix | `build` 9.1: a path's `points` are every vertex of its final curve on the lattice, ends included, not resampled. M1. The SDK's side is E5. | Case owed in `build`. | |
| S11.O.4 | `world.json` holds absolute paths to pack files. | fix | `build` 3.4 and 4: paths are relative; `pack_root` is the one path that may climb out. The bytes match on every system (R26). M1. | `build` cases 4 ("Moved"), 5 ("Nothing absolute"). | |
| S11.O.5 | Instance footprint features and kit-run pads are not in `world.json`. | fix | `build` 8 and 9.1: every feature is handed over, produced and footprint features included. M1. Kit-run pads come with the kit form, M3. | `rules` r03 (the pad region). `features.json` holding it: case owed in `build`. | Kit pads: not reachable. |
| S11.O.6 | When `field.required` fires nothing is written, not even `diagnostics.json`. | fix | `diagnostics` 4 and `build` 13: `diagnostics.json` is always written, error or not. M1. | `build` case 16 ("Failed build"); `fields` case "no surface". | |
| S11.O.7 | Old files in a build folder are not removed. | fix | `build` 13: the directory is replaced whole by rename; with an error it holds only `diagnostics.json` and `.previous`. After a build with fewer outputs, no file of the earlier build remains, and `via check` finds no file the manifest does not list (`build` 14). M1. | `build` cases 7 ("No stale file"), 6 ("Complete"), 16. | |
| S11.O.8 | The map's class table is keyed on fixed material names; an unknown material becomes `grass`. | fix | Library: `lib/map`. Classes come from the pack's taxonomy (`assets` 5); an unknown class is a warning, never a guess. M6. | Plan M6 (`atlas`). | Not reachable (no map library). |
| S11.O.9 | The map's tiles ignore leftover rows and columns when the size is not a multiple of 8. | fix | Library: `lib/map`, over `fields` 6.3: windows cover the whole extent. M6. | Plan M6 (`atlas`). | Not reachable (no map library). |
| S11.O.10 | The map's part labels use the part id and exist only for seam parts with a `footprint`. | fix | Library: `lib/map`. Labels come from `output` features and place names, for every import. M6. | Plan M6 (`atlas`). | Not reachable (no map library). |

### The reach lint

| # | Limitation | Verdict | Where | Proven by | Until then |
|---|---|---|---|---|---|
| S11.R.1 | It reads the top surface only: no caves, overhangs, interiors, stairs, meshes or bridges. | fix | Library: `lib/review`, a lint over `surfaces`: it walks every surface and crosses at meetings (`surfaces` 2.4). Decks: M3, (open) as S12.8. Floors and doors: M5. | Plan M3: "reach as a library lint"; plan M5: "indoors reachable through a door". | Not reachable (no review library). |
| S11.R.2 | Its cell (two posts), snap radius (8 m) and instance approach (2.5 m) are fixed. | fix | Library: `lib/review`. Each is a lint parameter with a stated default, reported with the result (protocol with every number). M3. | Plan M3: "reach as a library lint". | Not reachable (no review library). |
| S11.R.3 | `reach.png` shows the walkable area of the first valid expectation's profile only. | fix | Library: `lib/review`. One result per profile. M3. | Plan M3: "reach as a library lint". | Not reachable (no review library). |
| S11.R.4 | A region or path feature cannot be a start or target unless it has `at`. | fix | Library: `lib/review`, over `identity`: a start or target is any place name, path ends and outline points included. M3. | Plan M3: "reach as a library lint". | Not reachable (no review library). |

### Tools

| # | Limitation | Verdict | Where | Proven by | Until then |
|---|---|---|---|---|---|
| S11.T.1 | The build folder is fixed to `build/` beside the package. | fix | `build` 14: `via build` takes `--out DIR`; the default is `build/<name>` beside the world file. M1. | `build` cases 4 ("Moved"), 17 ("Not a build"). | |
| S11.T.2 | `check.py` skips PNG files. | drop | `check.py` is retired. The conformance runner compares every file the build writes (R20, R26); pictures from the sheet tool are judged by measure and eye, not bytes (R15). | | |
| S11.T.3 | `shoot.sh` does not pass `--shots`; the mode is an extra argument. | drop | `shoot.sh` and the spike viewer are retired. The sheet tool renders ground bookmarks with one command (plan, Verification). | | |
| S11.T.4 | The viewer's walk test is written for one world, with its coordinates. | drop | The spike viewer is retired. The Godot review viewer reads any world's bookmarks (plan, M1 track B; `build` 15.1). | | |
| S11.T.5 | The viewer finds rock by the material name `rock` and sea level by the parameter `sea`. | drop | The spike viewer is retired. The loader reads roles, tags and the pack, never names (principle 5). | | |
| S11.T.6 | A relative `root` in a pack is resolved against the working directory. | fix | `assets`: `root` is relative to the pack file. M1. | Case owed in `assets`. | |

## Section 12.8

| # | Limitation | Verdict | Where | Proven by | Until then |
|---|---|---|---|---|---|
| S12.1 | A step that returns a changed field causes the effects below it to be applied twice. | fix | `authority` 4.1 and 4.4, `fields` 7: a step returns rows and features, never a whole field; its rows sit before every effect and it reads the start view (R21). M2. | `fields` M2 case "step view" (12, not 14); `authority` cases `step_start`, `step_view`. | Not reachable: M1's only step is a lint, which returns no rows. |
| S12.2 | A step's record is one slot; building with and without an overlay by turns reruns it each time. | fix | `graph` 3.2: the store keeps every key it has seen; `protocol` keeps several records per static key. M1. | `graph` cases "toggle", "step slots"; `protocol` case 4. | |
| S12.3 | A step that runs an outside program is only as portable as that program; the check suite needs it built. | accept | Documented in `protocol` (a step's run is recorded, not promised) and `graph` 5.2 (R26). Our own erosion library is built in the workspace, so the suite builds it. | | |
| S12.4 | A step reads only exported fields, and feature attributes only with `feature_attrs`. | fix | `protocol`: a step reads every field and attribute at or below its layer (R19b, R21). `feature_attrs` is gone (`vocabulary` 6). M1. | `protocol` cases 3, 22; `graph` case "outside" (`graph.read_outside`). | |
| S12.5 | A region written as a condition keeps its largest piece and drops the rest silently. | fix | `geometry` 3.7: every piece is kept. A piece under one cell is `geom.mask_tiny` (info); a piece with no area on the lattice is dropped with `geom.mask_dropped` (warning); no piece at all is `geom.mask_empty` (error) (R18). M1 track A. | `geometry` cases "`speckle`; mask" and "a field below 0 everywhere". | |
| S12.6 | A feature must have a shape, so a waterfront ("a path along another feature") is written as a region. | fix | `geometry` 4: `follows` (R18). M1 track A; used in M3. | `geometry` cases "`bend`; `follows`", "`dumbbell` outline; `follows`". | |
| S12.7a | The step cache covers inputs, not outputs. | fix | `graph` 5.2: the engine records what each step read; `graph` 3.2: every stored object is checked against its digest. M2. | `graph` cases "step reuse", "step rerun", "corrupt"; plan M2: "a step re-runs only when what it read changed". | Gap: M1's lint steps have recorded reads too, but M2 is where acceptance proves them. |
| S12.7b | Undeclared inputs go stale unseen. | accept | Documented in `protocol` (no sandbox) and `graph` 5.2: a file a step opens directly and does not list in `inputs` goes stale unseen, as R26 accepts (a user's step is recorded, not promised). Only `input.open` of an undeclared path is an error (`protocol.input_undeclared`). Seeing a worker's file reads is (open) in `graph` for M2. | `protocol` case 17. | |
| S12.8 | A deck's end is not checked against any way; a ramp meets a deck with no merge geometry; a deck is flat across its width. | fix | `surfaces` 3: ways arrive at a deck's mounts and every meeting is checked to the millimetre (3.4). A deck's height is a formula over its owner's frame (1.1), so it need not be flat. Milestone M3, (open): `vocabulary` 5 puts mounts on a kind and a second surface in M5; decided by the lead before M3 (`surfaces` Open 2). | Plan M3, challenges 34, 38, 63. | `doc.not_yet` (mounts on a kind, a second `surface` field). |
| S12.9 | Decks ignore authority and are not seen by the reach lint or the map's walkable layers. | fix | `surfaces` 1: a deck is a surface that holds as firmly as its owner. Reach walks it (`lib/review`); the map reads it (M6). Milestone M3, (open) as S12.8. | Plan M3: "reach as a library lint". | `doc.not_yet` (mounts on a kind, a second `surface` field). |
| S12.10 | A deck's clearance over a region is measured at every station inside it, banks included. | drop | Clearance leaves the core: `headroom` is a ways library attribute checked by that library's lint (`vocabulary` 6; `surfaces` 3.5). The library chooses where to measure. | | |
| S12.11 | Two ways crossing at grade grade the ground in turn; nothing models the junction. | fix | `geometry` 3.1-3.2: the network nodes crossings, and a named thing owns each meeting (R18), a ways library kind. M3. | `geometry` case `cross`; `townland` (M3). | `doc.not_yet` (user syntax for network). |
| S12.12 | A kit run's pieces have no corners, stretch or filler; variation is by weighted choice only. | fix | Library: `lib/building`. Its kit form (lowers to a run, `rules` 10.1) takes the town kit's three owed changes. M5. | Plan M5: "the town kit's three owed changes". | Not reachable (no building library). |
| S12.13 | `along` places a straight run on a curved path without checking how far it strays. | fix | A run of rigid pieces reports each piece's greatest distance from its path against a limit the run states. Not yet in `rules`: owed in `rules` 5 for M3. | `townland` (M3). | `doc.not_yet` (`[run]`). |
| S12.14 | Blocks, lots, districts, local streets and interchanges are not concepts; steps make them as plain features. | fix | `geometry` 3.2 and 3.6: network, areas and cells are core mechanisms (R13); district and lot stay library words (R12). M1 track A; M3 for the town. | `geometry` cases `cross`, `townland_streets`. | User syntax: `doc.not_yet` (M3). |
| S12.15 | A way cannot be interrupted by a deck; it is graded and painted across the bed under the bridge. | fix | `surfaces` 3.5: a path meets things at its ends only; a way that crosses water at a crossing is two path features, ending at the crossing's mounts. Milestone M3, (open) as S12.8. | Plan M3, challenge 34. | `doc.not_yet` (mounts on a kind). |
| S12.16 | A deck's profile is a line between its ends plus `rise`, `ramp` and `settle`; nothing else can be stated. | fix | `surfaces` 1.1: a deck's surface is any height formula over its owner's frame. Milestone M3, (open) as S12.8. | Plan M3, challenge 38. | `doc.not_yet` (a second `surface` field). |
| S12.17 | Piers avoid only what a deck names in `over`; they can stand in a street it does not name. | fix | Library: `lib/ways`. Piers are a ways library form; they keep out of the `keep_out` role like every producer (`rules` 7.1, 8.3). The spike's feature key `over` is gone. M3. | Plan M3 (`quay`). | Not reachable (no ways library). |

## Known weak points (`docs/design/concepts.md`)

| # | Limitation | Verdict | Where | Proven by | Until then |
|---|---|---|---|---|---|
| W1 | Volumes, levels and decks: three unrelated ways to leave the height field. | fix | `surfaces`: one surfaces model (R25). M1 for the chapter and grounding on the terrain; decks M3, (open) as S12.8; floors and volumes M5. | `surfaces` 6.1 (`ford`'s cottage). | `doc.not_yet` (a second `surface` field, `type = "volume"`). |
| W2 | `authority`, feature `order` and effect `order`: three orderings with no stated relation. | fix | `authority` 4: one order (R21), as S11.K.3. Feature `order` is gone; `order` lives on effects and `then` rows. M1. | `authority` cases `tier_order`, `order_key`, `across_layers`, `shuffled`. | |
| W3 | The compiler knows names that belong in a library or a convention. | fix | `vocabulary` 7: the deny-list test on every core crate (principle 5). M0 step 5; enforced from M1. | `build` case 19 (no string `bookmark` in the core). | |
| W4 | `set`, `run`, `kitrun`, `dress` and `flank`: five overlapping ways to place a group. | fix | `rules`: three core rules, `set`, `run`, `scatter` (doctrine section 6). Kit run, dress and flank are library forms that lower to them (`rules` 10). Scatter M1; set and run M3; the kit form M3 (`city`; `authority` Open); the town kit M5. | `rules` r25. | `doc.not_yet` (`[set]`, `[run]`). |
| W5 | An asset's `source` is one file, where a real asset is a model, a texture and copies. | fix | `assets` 2.2 and 3.3: an asset is a set of files (`file`, `detail` copies, and every buffer and image they name). M1; proven with real files by the M2 pack swap. | Case owed in `assets`; `glade` (M2). | |
| W6 | `dist()` and `sdist()` run over the whole grid: unbounded cost. | fix | `formulas` 8.4: each takes a `cap`. It may be left out only inside a kind effect or kind `let`, where it defaults to the effect's `radius`; elsewhere a missing cap is `formula.cap`. A kind effect with no radius: (open), raised with `formulas` before M1 (for example, `formula.cap` as outside a kind effect). M1. | `formulas` cases 14 (`formula.cap`), 24. | |
| W7 | Nothing keys a hand edit to a lowered instance, so regeneration loses it. | fix | `identity` 4: pins and clears are intent at a place and speak to every producer (R3). M1. | `identity` cases 4, 8; `ford` "Spacing 7 to 6, under a clear and a pin". | |

## Recorded elsewhere

Limitations the spike recorded outside SPEC 11 and 12.8 that the rewrite would repeat unless sorted.

| # | Limitation | Verdict | Where | Proven by | Until then |
|---|---|---|---|---|---|
| E1 | A path's frame is discontinuous on its medial axis, so `along()` or `sdist()` far from a path creases the terrain (`spike/FINDINGS.md`, "Frames have two real defects"). | accept | `geometry` 2.5 keeps the nearest point, lower `t` on a tie, so `t` jumps across the medial axis and `d` has a kink there. A radius smaller than the path's least radius of curvature keeps an effect off it. Written for users: owed in `geometry` 2.5, M1 track A. A smoothed frame is (open), decided in `geometry` before M1 track A, by a `ford` vector of the beck and lanes with wide effects. | Vector owed in `geometry`. | |
| E2 | Only whole features can be placed in another's frame; a path's own vertex cannot, so moving a road left a trail's start 31 m away (same FINDINGS entry). | fix | `vocabulary` and `geometry`: a path's end is placed at a named thing with `starts_at` and `ends_at` (R18); `follows` runs a path along another (`geometry` 4). M1. | `authority` case `owner_holds`; `ford`'s lanes ending at the ford. | |
| E3 | A tree can stand at the water's edge with its foot wet (`spike/FINDINGS.md`, "What is still wrong"). | fix | `rules` 7.1: a scatter multiplies density by `1 - keep_out`. Whether the water library writes `keep_out` over wet ground, or a scatter's density reads the water level by tag (`fields` 2.2): (open), decided by `ford`'s bank view in M1. | `ford`, seen "on the bank along the water's edge". | |
| E4 | `atlas` exported no water and nothing said so (`TODO.md`). | fix | `fields` 1 and `build` 6: every field is written, with its role and tags; a loader picks. M1. | Case owed in `build` (a field no loader option names is still in the build). | |
| E5 | The SDK's `points` are every eighth sample and do not say so (`TODO.md`, the fresh agent). | fix | The SDK hands a path's points as `build` 9.1 writes them: every vertex. To be stated in `protocol`. M1. | Case owed in `protocol`. | |
| E6 | Two floods that read each other silently miss an effect (`docs/challenges.md` row 5). | fix | `graph` 2.1: a loop of reads is one `graph.cycle` error. Floods are a water library kind (R14), M2 (`vale`). | `graph` case "cycle order"; `vale` (M2). | |
| E7 | `[[override]]` without `id` crashes (`TODO.md`). | drop | `[[override]]` is dropped (`identity` 6). An array table is `doc.syntax` with a hint. | `documents` case `array_table`. | |
| E8 | A passing lint step is silent (`TODO.md`). | fix | `diagnostics` 7: the summary lists every lint step that ran with its counts, zeros included. M1. | `diagnostics` case "a lint step that finds nothing". | |

## Counts

| Source | Bullets | Rows | fix | accept | drop |
|---|---|---|---|---|---|
| Section 11, fields and materials | 11 | 11 | 10 | 0 | 1 |
| Section 11, kinds, effects and authority | 11 | 12 | 9 | 1 | 2 |
| Section 11, features | 10 | 10 | 5 | 1 | 4 |
| Section 11, instances and producers | 16 | 16 | 12 | 0 | 4 |
| Section 11, layers and merging | 5 | 5 | 5 | 0 | 0 |
| Section 11, parts | 11 | 11 | 9 | 1 | 1 |
| Section 11, outputs | 10 | 10 | 9 | 0 | 1 |
| Section 11, the reach lint | 4 | 4 | 4 | 0 | 0 |
| Section 11, tools | 6 | 6 | 2 | 0 | 4 |
| Section 12.8 | 17 | 18 | 15 | 2 | 1 |
| Concepts, known weak points | 7 | 7 | 7 | 0 | 0 |
| Recorded elsewhere | 8 | 8 | 6 | 1 | 1 |
| **Total** | **116** | **118** | **93** | **6** | **19** |

108 of 108 source bullets sorted, plus 8 recorded elsewhere.
