# Doctrine

The book that decides details nobody has time to review. When a question comes up, it is answered
from here; when an answer is new, it is written here. A ruling stands until a later ruling names it
and replaces it.

Kept by the lead engineer during review. Challenges are welcome from any angle; a challenge changes
a ruling only by showing that a principle below is served better another way.

## 1. The goal

> Decide what matters; the world settles around it. Change your mind, and it settles again.
> You never repair a map by hand.

Who it is for: new games, small teams first, Godot or Unreal, with an agent writing the documents
and a person reviewing them. Assets are assumed cheap and fast to get. Placing them at scale, again
and again, is the hard part, and it is ours.

## 2. Principles

Numbered so a ruling can cite them. Earlier ones win when two disagree.

1. **Text is the only source of truth.** The build is thrown away and made again. Nothing a person
   cares about lives only in a build.
2. **Write intent, not results.** A world says what a thing is and what it belongs to ("on the
   lane, a third of the way along"), not where it ended up. A coordinate is allowed where the
   author truly means that spot.
3. **Say what matters; the rest gives way.** Every statement has a strength. The author's firm
   decisions hold, and everything derived moves around them. A clash between two firm decisions
   is an error, never a silent winner.
4. **Change is the main case.** A feature is judged by what happens when something under it moves,
   not by whether it builds once.
5. **The core knows no domain noun.** Road, room, lake, tree, door are words in libraries, written
   as data in the same format a user writes. If the core must know a word to make it work, the
   core is missing a mechanism.
6. **A world never names a file.** It names an asset id or a place in a taxonomy. A pack binds
   those to files. Swapping the pack swaps the look and nothing else.
7. **Same text, same build.** Randomness comes from named seeds. Outside inputs are declared.
8. **Every failure says where and what to change.** The first reader of a message is an agent. A
   message without a location or a next step is a defect.
9. **One word, one meaning.** A key means the same thing in every table. A new word must say
   something the existing words cannot.
10. **Fewer mechanisms beat more words.** Before adding a table or a key, show that it cannot be
    written with what exists. Convenience forms are allowed only as shorthand that lowers to the
    plain form, and the plain form is always legal.
11. **Readable by review.** Files are flat, keyed by id, and short enough to read. A change to one
    thing is a diff of a few lines in one place.
12. **We stop at the engine's door.** We hand over fields, instances, meshes, chunks and metadata.
    Frame rate, sky, streaming and gameplay are the engine's, and we say so.
13. **A request that does not fit is recorded, not skipped.** It gets a demonstration and a row in
    `challenges.md`. Leaving it out of the product is a decision written down with its reason.

## 3. The mental model

Eleven nouns. Everything else is a library word or shorthand.

| Noun | What it is |
|---|---|
| frame | Where coordinates are measured from. The world has one; a part or a thing may have its own. |
| field | A value at every point: height, water, ground material, anything the project names. |
| feature | A point, a path or a region with an id, a kind and attributes. |
| kind | What a feature does to fields, written as data. |
| authority | How firmly a statement holds: derived, shaped, anchored. |
| asset | An id a pack binds to a shape and its facts. |
| instance | One asset in one place. |
| rule | Anything that lowers to plain instances or features (scatter, run, set). |
| layer | One document of a world, with an owner. It may refer downward only. |
| change | A named difference over a world: a variant, an overlay, a hand edit. |
| step | The project's own code, run at a declared point, with declared inputs. |

The build order, in one line: fields from their bases, then features by authority, then rules,
then checks.

## 4. How a challenge is ruled

Asked in this order. The first one that answers ends it.

1. Does it serve the goal line, or does it only make building once nicer? (1, 4)
2. Can it already be said with the eleven nouns? Then it is shorthand or a library, not core. (5, 10)
3. Does the word already mean something else here? Then rename or merge. (9)
4. Could an agent get it right from the documents alone, and be told how when it does not? (8, 11)
5. Is it on our side of the engine's door? (12)

"Do we need X?" is answered by removing X from the example world and seeing what a user would
then have to write. If the result is no longer or harder to change, X goes.

## 5. Rulings

| # | Date | Question | Ruling | Cites |
|---|---|---|---|---|
| R1 | 2026-11 | File format | Flat TOML, tables keyed by id. | 11 |
| R2 | 2026-11 | Language of user code | Python, through an SDK from the start. A Rust core is a direction, not decided. | 7 |
| R3 | 2026-11-12 | How a hand edit survives a rule change | It is stored as intent at a place (pin, clear), not against a generated id. Ids exist for the diff only. | 2, 4 |
| R4 | 2026-11-12 | Locking a finished area | A lock is a replayed record, and a rule change reports what it would have done there. | 1, 4 |
| R5 | 2026-11-12 | Good placement unattended | Placement by cause (growth reads its neighbours, clumps) is the default to teach. | 3 |
| R6 | 2026-11-12 | What the formula strings are | A small expression language of ours, not Python: arithmetic, comparisons, and a closed list of about twenty functions shipped by the engine, on one reference page. No imports, no loops, no user functions; logic beyond that is a step. The spike's use of Python `eval` is a shortcut and does not carry over. | 7, 8, 10 |
| R7 | 2026-11-12 | Formula or code | Both stay, with a line between them. A formula is for a value at a point from other values at that point; the engine can see its inputs, so it can cache, rebuild part of a world and report errors in place. A step (Python, through the SDK) is for anything that needs neighbours, an algorithm or outside data. The SDK exposes the same function list as formulas, so one can be moved to the other. The SDK writes text and fields; it is never a second source of truth. | 1, 7, 10 |
| R8 | 2026-11-12 | Maps built apart, joined in one | Core. A master places worlds built apart and owns only what connects them, naming places inside them (`a.quay_road.end`), never their coordinates. The join blends by a rule the user may replace. A named place that the blend would alter is reported. Shown in `strait`; rotation, placing `on` a part's feature, and a blend that reads the ground are owed. | 2, 3, 4 |
| R9 | 2026-11-12 | Who owns the edge of a world built apart | Three owners. The world owns its edge: what is local stops inside it, and what leaves is declared as an exit, a named place with its facts. The parent owns the region: it answers each exit by name with something that goes somewhere, or with a deliberate end. The core owns the obligation: an exit nobody answers is an error that says what to write. The core knows the word exit and no word for what leaves. Shown in `city` and `atlas`. | 2, 3, 5, 8 |
| R10 | 2026-11-12 | A join by ground, with extra care | No new concept. The care is master content over the joined ground, kept in its own master layer with its own owner, applied `after = "parts"` so it can read both sides. The join line is named from the parts, a barrier opens where named walks cross it, and expectations prove the funnel. A third map between two is right only when the in-between is a place in itself. Overruling ground a part anchored is reported. Shown in `border`; a part's own growth crossing its edge is owed. | 2, 3, 4, 10 |
| R11 | 2026-11-13 | Places and fills | Two ways to compose. A place (campus, island, interior) is built alone and joined at its edge. A fill (city, forest, farmland) takes its shape from the region: it declares what it accepts with defaults (`[given.*]`) and the parent hands it down (`give`), so it is built in the parent's context and rebuilt when that changes. Opt-in per input. | 2, 3, 4 |
| R12 | 2026-11-13 | How a city is authored | As intent at three strengths: an anchored skeleton and landmarks, shaped districts (a region written as a condition, plus a character), and derived fill that nobody writes. Districts are features in the parent, not maps. The core owns nothing named after a city; `district`, `square` and the patterns are a library. Shown in `atlas`: moving one square settled streets and lots with no other edit. | 3, 5, 10 |
| R13 | 2026-11-13 | What generated streets, blocks and lots are to the core | Owed, and now proven necessary: a network of paths with the faces between them as nameable regions, and a region divided into cells, as core mechanisms. While code fakes them privately, a clear on a generated building does nothing, a pin lands inside one, nothing can be placed on a generated street, and ids shift when an unrelated district changes. All four must hold in v1. | 4, 8 |
| R14 | 2026-11-13 | Simulation under authority | A simulation is a way of deriving, so it obeys authority. What the author states (a lake at a level, a river from here to there) is turned into the simulation's starting conditions, it fills in the rest, and afterwards every stated fact is checked and a failure says where and what would hold instead. Never generate many and pick one. How a fact becomes a condition is the library's knowledge, not the author's: a river is a groove in the starting surface, not held cells, which failed. The simulation is a step with declared inputs; the core does not learn erosion. Shown in `basin` with the earlier Rust terrain code. | 2, 3, 7, 8 |
| R15 | 2026-11-13 | "Looks right" needs a number beside the eye | Where a look can be measured, a library lint reports the measure with the build, and the eye still decides. First case: street form (bearing spread, junction mix). It named what the eye had not: the skeleton stayed a grid after districts, and dead ends tripled. A measure informs; it is never a target to optimise. | 8 |
| R16 | 2026-11-13 | A main way | Written by what it connects: its ends and the places it passes, by name, with the course found over the land. A typed course is allowed and is then anchored. Shape comes from the land first, the skeleton second, districts third; a city on flat ground with straight roads is boxy whatever the street pattern. Shown in `townland`. | 2, 3 |
| R17 | 2026-11-13 | Where a build is judged | From eye height, at the places where two kinds meet (a way and water, a street and an edge, a deck and a bank). A map view accepts the plan only. Every demonstration carries ground-level views at its meetings, and a fault seen there is a fault in a kind or a tool, not in the viewer's taste. Found in `townland`: accepted from the air, wrong on foot. | 4, 8 |
| R18 | 2026-11-13 | Who owns the place where two things meet | A named thing, never whichever of the two got there first. A crossing owns its deck and its two ends, and ways arrive at those ends; a waterfront owns the bank, and streets end on it; a river that comes from beyond the map enters at a declared exit, like a road. A meeting nobody owns shows as a stub, a doubled deck or a blunt end. Shown in `townland`: dead ends 39% to 15%, three overlapping decks to one. Owed in core: a path written as "along another feature", and a region condition that silently keeps only its largest piece must report what it dropped. | 3, 5, 8, 9 |
| R19 | 2026-11-13 | Language of the core | Rust. Replaces the second half of R2. Accepted costs: we write the planar network with faces, robust offsetting and the array evaluator ourselves; ordered maps and fixed reduction order everywhere for principle 7. Conditions: (a) steps and libraries stay Python through a thin SDK over a declared interface; (b) no private door: our own libraries (streets, lots, terrain, rivers) are built only on what a user's step can reach, so the user's ceiling is ours; (c) a step exchanges whole fields and feature lists, never a per-point callback; (d) the network with faces is built first. The user never sees the core's language: they write TOML, formulas and Python. | 5, 7, 10 |
| R20 | 2026-11-13 | Setup | One Rust binary, `via`. `spec/` (chapters, schemas, test vectors, conformance cases) is normative; conformance runs through the command line only. The Python SDK is pure Python and never links the engine: the engine starts a worker process and they speak JSON-RPC on stdio. Fields and large tables never travel as JSON: they are write-once files named by digest, mapped by both sides. Windows is a target from the first day. Follows the reference in `docs/references.md` ("Engine and SDK setup"), whose two recorded failures (Windows, bytes across systems) are designed for. | 7, 8, 12 |
| R21 | 2026-11-13 | Order of a field's rows across layers | By authority across the whole world; layers break ties and decide what a step may read (the layers at or below it). A step returns rows and features, never a whole field, so nothing below it is applied twice. Layer-first order was proposed as cheaper and rejected: a derived lake in a lower layer would stop giving way to an anchored thing above it. Cost accepted: one evaluation per distinct view a step reads. | 3, 4 |
| R22 | 2026-11-13 | Lifecycle points | `generate`, `lint`, `export`. `effect` and `place` are dropped: a generate step at a layer covers both. One request, two transports: a worker (may call the engine) and a plain command (files only). | 10 |
| R23 | 2026-11-13 | Engine calls from a step | Always a batch: columns in, columns out. A loop that would need thousands of calls (faces, cells along a frontage, a route over a cost field) becomes an engine operation with no domain noun. | 5, 10; R19b, R19c |
| R24 | 2026-11-13 | Geometry numbers | Integer millimetres in the core; decimal metres in text. Exact predicates, no tolerance tuning. | 7 |
| R25 | 2026-11-13 | The one surfaces model | A surface is a bounded field with the role `surface`, owned by a feature: terrain over the extent, a deck over its strip, a floor at a level, a cave's floor (M5: there is no volume field; each is a kind surface, `surfaces.md` 1.3). Where two surfaces meet is a pair of mounts. No new noun. Placement, grounding, reach and export all ask the same four questions of it. | 5, 10 |
| R26 | 2026-11-13 | Same text, same bytes | On macOS, Linux and Windows, for one engine version. All maths goes through one module; no non-finite value enters a build. A user's step is recorded, not promised. | 7 |
| R28 | 2026-11-13 | Neighbourhood functions in formulas (asked by `spec/formulas.md` 8.6) | Amends R7. A formula may call a shipped function that reads around a point (`slope`, `under`, `smoothed_along`, `dist`) when the function declares its halo and cap, so the engine still sees every input and evaluates by window. An open-ended neighbourhood or an algorithm stays a step. | 7, 10 |
| R29 | 2026-11-13 | Ties under R21 (asked by `spec/authority.md` 4.2) | Within one tier: nesting first (the smaller anchor inside a larger is applied later), then a stated `order`, then layer rank, then id bytewise. Document order never decides. A step's returned rows and a field read from a `file` are rows like any other and take the same order; a lint or export reads the finished build. | 3, 7 |
| R30 | 2026-11-13 | What a pack swap may change (asked by `spec/assets.md` 1) | Amends principle 6: swapping the pack swaps the look and the facts the look carries (size, footprint, mounts), and what follows from those facts (a pad that grows) settles and is reported. Nothing the author wrote changes. A field's `file` is a declared outside input under principle 7, not an asset. | 4, 6, 7 |
| R31 | 2026-11-13 | Meetings and the terrain's owner (asked by `spec/surfaces.md` 3.3, `spec/identity.md` 4.2) | Amends R25's wording: the terrain surface is owned by the world. Every meeting is a pair of mounts; a path that ends at a place meets that place's default mount. A pin inside a clear is a warning (the later edit wins and says so), not an error. Queued for M2: R14's `checks` key on a simulated fact. | 3, 8 |
| R32 | 2026-11-13 | What a step may return (asked by `spec/fields.md` 7, `spec/protocol.md` 5) | Amends R21. A generate step returns rows on any field below it, features, and fields of its own (new names, declared in its returned layer). It never replaces a field that already exists. R19(c) reads "whole arrays, never per point", for workers and plain commands alike. | 3, 4, 10 |
| R33 | 2026-11-13 | Pre-code placements (asked by the M0 open list) | Function table: `spec/functions.toml`; word list: `spec/vocabulary.toml`; generated reference pages in `docs/reference/`; conformance runner in `tools/conformance/`; machine files (`config.toml`, `trust.toml`) are documented in `spec/protocol.md`, not the vocabulary; network and cell declaration syntax is M3; the smoothed path frame is decided by the `ford_frame` vector in M1 track A; a workspace test fails when a limitation sorted "fix" names a case its chapter lacks. | 10, 11 |
| R34 | 2026-11-15 | A library step not written in Python (asked by M2: the erosion step is a Rust program) | Allowed. "No private door" (R19b) is about reach, not language: a library step may be any program that speaks the step protocol and reaches the build only through it. R19a's "Python" names the SDK we ship for users, not a limit on our own steps. A step that reads build files or engine internals outside the protocol is the private door, whatever its language. | 5, 10 |
| R35 | 2026-11-15 | `collision` on an asset (asked by `spec/assets.md`) | A declared fact the core never reads and hands to the loader (principle 12), so it is pack metadata, not a core mechanism. Reverses the M1 sentence "the core has no collision key". | 12 |
| R36 | 2026-11-16 | Content over a join without `after` (asked by `spec/composition.md` 6.5, M4 slice 1) | Amends R10's wording: the master content over the joined ground is a layer ranked above the layer that declares the import, so its entries follow the import's in the same tier and its steps read the joined ground. `after = "parts"` (spike) and `after = "imports"` (vocabulary) are dropped: rank already orders entries and decides what a step reads (R21, R29). `after` on a layer is `doc.unknown_key` with a fixed hint. The rest of R10 stands. | 10 |
| R37 | 2026-11-17 | No volume: where a thing stands above or under the ground (asked by `spec/surfaces.md` Open 6 to 10 and 1.3, challenges 7 and 8; `coast`, `station`) | Amends R25 (its wording already says so). There is no 3D field and no `hosts`: a floor, a deck, a stair's tread and a cave's floor are each a kind surface, and `type = "volume"` is `field.type_unknown` with a hint naming kind surfaces. The terrain under a floor or across a cave mouth is removed from export by `cut` on the kind surface (a quad is cut when its centre is inside the bound and its lowest post is below the surface plus `cut`); the terrain field itself is unchanged and every rule reads it as before. A kind surface's mesh is the core's, on the world's posts inside the bound, with a pack `material`. `path3` is dropped: a stair, a ramp and a cave are paths whose kind owns a surface, a loop ramp that does not pass over itself in plan is one path, and a full spiral is two. Heights are world metres: a floor's `level` is a number in the world frame, and "so high above the ground" is a step's computation from `sample`. Still open, with its cost seen in M5 (`city`, `monastery`): a surface formula may not read a field (Open 9), so `deck_z`, a storey's `level` and a ramp's ends are typed or step-computed and do not follow a moved end. | 5, 10, 12 |
| R38 | 2026-11-17 | What holds at a meeting (asked by `spec/surfaces.md` 3.2 to 3.4 and Open 4, 11; `spec/assets.md` 2.7, 10) | A meeting is exact in v1: no allowed step (a kerb is a short flight or the walk library's tolerance read from `gap`) and no threshold (a mount's height is always its surface's; a raised sill is the floor's own height with the ground set to it). `connector` is a free word on a kind mount, a kind's path ends and an asset mount, compared only for equality at every meeting; a connector edit in an accepted pack is `asset.revised`. `surface.facing` reads `meets` pairs only; `surface.unmet` reads kind and asset mounts, never a path end. There is no fallback asset for an unmet mount: meetings come after placement, so the library step that left a mount unanswered places its filler. A point's or an instance's face is its default mount's, so `turn` is reported there (report only). A kind surface is a place: a path's end that names it meets it at the end's own point, and outside the bound is `surface.outside`; this is how a door and a stair land on a floor. Mounts read from named glTF nodes are not in v1. | 3, 5, 8, 12 |
| R39 | 2026-11-17 | Which surface a thing stands on, and who is told when it does not fit (asked by `spec/surfaces.md` 4.1, 4.2; M5 slices 2 and 3) | **Owner first** (two items marked "open to the owner" in the spec). An `at` placement, a set, a point feature or a scatter may name the kind surface it stands on with `surface = "<feature>.<surface>"` (never the terrain, never with `on`: `surface.placed_twice`); a run never takes it. A point feature `on` a thing stands where an instance `on` it would, and its `z` is that surface's; laid past the bound by more than one lattice unit it stands on the terrain and nothing is said (an instance there is `surface.outside`): **open to the owner**, cost that a point meant for the deck and laid off it is not reported. `surface.through` is tested for every hand instance on the terrain however it was put there; `surface.off_bound` and `place.relief` on a kind surface are said for hand instances and set items only. A run member laid along its host's line may overhang by design, a scattered candidate gives way silently, and a step's returned instance is never tested by either, whatever hand edits the world holds: a library's own instances are its lint (**open to the owner**; it was tested where a hand edit made it a producer's). | 3, 8, 10 |
| R40 | 2026-11-17 | When two placed things clash (asked by `spec/rules.md` 8.1, `spec/assets.md` 12.1, `spec/graph.md` 1; `station`, `pistol`, `market`) | Records `rules.md` 8.1's M5 level rule. Two things are tested for overlap only when they stand on one level and their height spans above it meet (`-sink` to `-sink` plus the `bounds` height, scaled); the test reads plans and asset facts only, before any field settles, and is its own graph node that no height reads. Two storeys are two levels. The terrain and a kind surface with `cut`, or whose bound the terrain thing's footprint meets, are one level (the slice 2 ruling, case `surfaces/ground_floor_clash`; `rules.md` 8.1 says so since the M5 close). Parts whose chains of `on` reach one root are one thing and are not tested against each other, as one set's items are not. A core form's exemption does not pass to a library's lowering of it: a kit run a step lays is hand instances of the step's document, so the library keeps its neighbours apart itself (`lib/town/kit.py` rule 7); every library form says whether its lowering inherits the core's exemptions. Cost, owed: the terrain and a floor are compared as one datum, so a ground floor raised above the terrain by more than a piece's height still clashes (TODO). | 3, 4, 7, 10 |
| R41 | 2026-11-17 | Passable ground and what a walk may assume (asked by `spec/assets.md` 10, `spec/surfaces.md` 4.4, `spec/build.md` 15.1; `college`, `station`) | **Owner first** for the last sentence. `solidity = "passable"` is a core value: it claims ground exactly as `solid` does in every rule, and only walks read the difference; `collision` stays its own fact. The core has no walker; a walk library may rely on this list and nothing more: one surface at a time, within its bound, changing surface only at a holding meeting (a bound's edge is a wall unless a meeting is on it), a cut quad and a solid footprint are no ground, and `surfaces_at` is asked rather than "the highest surface here". A walk that must follow a route (a spiral flight) names a path in the review library's bookmark attribute `along`, which is text read by the loader, not a name reference: the core neither checks nor follows it, and a wrong name is the loader's `load.bad_target` (**open to the owner**: a name attribute would be renamed with its path and checked at build). | 5, 9, 12 |
| R42 | 2026-11-17 | Things with parts, states and accessories (asked by `spec/assets.md` 12, `spec/formulas.md` 6.7, challenges 46 and 47; `sidearm`, `pistol`) | No new noun, key or code. A thing with parts is placed things in frames that nest: items of a set (nested sets move together) or parts `on` the mounts of the part they ride on (sockets). A part's height in its thing is its asset's `sink`, negative (**open to the owner**; cost stated: a thing with parts must lie on flat ground, a part at two heights is two assets, and an accessory's asset states its socket's height; a mount with a height is owed). An accessory is a change file placing a set on a named socket, writing nothing of the host again. A state is said once, in the arrangement that makes it (another asset, another socket, another item place); a check reads it from the build, never from a flag beside it. Not said in v1, each a model's or a modelling tool's job: holes cut in a part, tilted parts, sizes, places or facings from a formula (a part sized from published numbers is a pack written by a committed tool with a `--check`), and constraints between parts. | 6, 9, 10, 12 |
| R43 | 2026-11-17 | What a step's document may say and how values reach it (asked by `spec/protocol.md` 3.3, 4.3, 5, `spec/identity.md` 2.3; `lib/building`) | Amends R32. Inside a returned document a name resolves to the document's own keys first (`starts_at`, `ends_at`, way points, `on.ref`, `follows.ref`, a kind's `names` attributes and `surface`), so one step lays a building's storeys, flights, walls and furniture in one document: a second step could not name what the first returned (R3), so they are not separate steps. An attribute number reaches a step as f64 whether written `2` or `2.5`; a list or table attribute as canonical JSON text, a kind's default included, with no type converted inside it. `surfaces_at` and `sample` share one test of what a step may read: a kind surface whose owner lies above the step's layer, or was returned by a step at or above the reader's rank, is left out or refused. `sink` comes as f64 metres, as `z` does. A produced name's producer is what stands before its last `#`. | 3, 7, 10 |
| R44 | 2026-11-17 | Budgets: who declares a limit and how it blocks (asked by `spec/build.md` 5.1, challenge 24; `atlas` `over_budget`) | **Open to the owner** (ours, no source). The world declares `[budget]` per chunk (`triangles`, `instances`, `groups`); no default, the pack and the reader declare none for the build. Over a limit is always `build.over_budget`, an error: no warning band (one code, one severity). A waiver names a chunk and raises that chunk's limit for a measure the world limits; it cannot add a limit the world does not set. Triangles per chunk are counted from the source assets (instances times the asset's `triangles`); generated meshes are not counted yet. The record names the dominant asset and its share. | 3, 8, 12 |
| R45 | 2026-11-17 | Export steps: where their files go and what they may be called (asked by `spec/build.md` 11.4, `spec/protocol.md` 1.1; `lib/map`) | **Open to the owner** (ours). An export step writes only under `export/<step id>/`, listed with digests in `files` and in the `id`. A file name outside `[a-z0-9_.#]` is `protocol.output_file` and is not written (a device name is escaped). The manifest has no `exports` key when no export ran, and an untrusted export step has no entry (it did not run). What it writes is the step's (R26). | 1, 7, 12 |
| R46 | 2026-11-17 | Assets an import brings (asked by `spec/composition.md` 10; `atlas`'s college and market) | **Open to the owner** (ours). An import's own pack's assets come into the build with its instances: listed in the manifest, counted by budgets, drawn by the map. The parent's text never resolves them: a parent's `asset`, query or `@taxon` reads the parent's pack only. | 6, 10 |
| R47 | 2026-11-17 | The player map's fixed rules (asked by `spec/build.md` 11.4's map rows, `lib/map`; challenge 42) | **Open to the owner** (ours, library defaults, each a `params` key). Floors whose levels lie within 1.6 m merge into one; a floor's plan cuts walls at 1.2 m over it; the discovery grid is 4 m a side; a label's text is the name's last part with `_` as a space (a produced name gives none), placed at the first of eight positions round its point that overlaps no other label at the label zoom. The map is data (`map.json`, tiles, `labels.json`); style, icons and discovery state stay the game's. | 5, 12 |
| R48 | 2026-11-17 | Streaming: what a streamed reader must equal (asked by `spec/build.md` 5.2, 15.1; challenges 4 and 53) | **Open to the owner** (ours). A walk streamed at a load radius has the same outcome and blocker as the whole build's walk and stops within two walk frames' pace of it (the stated stop bound); a reader reloads a chunk only when its `digest` changed; past the load radius a far layer (coarse ground, far water, instance stand-ins) is drawn and is the reader's, not the build's. | 4, 12 |
| R49 | 2026-11-17 | An import's kind surfaces, cut and meetings (asked by `spec/composition.md` 6.1; `atlas` on foot) | **Open to the owner** (ours). They come with their feature: base plus `lift`, scoped names, brought whole (footprint and blend do not clip them); the parent makes their meshes and the terrain's cut on its own posts; what stood on one inside the import stands on it here, nested too. A brought meeting's terrain side stands at the parent's finished ground and its gap is measured again, in `meets.json` and not as a diagnostic (the seam check says it). | 2, 3, 8 |
| R50 | 2026-11-17 | Water and ground at joins (asked by `spec/composition.md` 6.2, 6.7; `atlas`) | **Open to the owner** (ours). A field derived from one the import brings is laid by its depth over the parent's own ground (its head view): lowered by how far the joined ground lies below it, never raised; an import without the derived field is dry under it. The import's ground is held within 12 m of a place a parent path joins, faded to the blend by 30 m. The head view is a key of the derived field's chunks. | 2, 3, 4 |
| R51 | 2026-11-17 | Where a parent keeps a change it asks of an import (asked by `spec/composition.md` 9; `atlas` `move_square`) | **Open to the owner** (ours). `changes` names a file relative to the import's world folder, so a parent may keep the change in its own folder (`../atlas/changes/market/move_square.toml`): the change is in the import's terms and the import need not know it. No new key. | 9, 10, 11 |
| R52 | 2026-11-17 | A library step and its kinds file (asked by `spec/protocol.md` 7; the M7 slice 1 review) | **Open to the owner** (ours). A library's `kinds.toml` is text, not code: it is out of its steps' trust digest, and its statements key the steps that read them. So a library step never opens it; it receives what the file says through the build (`params`, the features served). `xtask check` refuses a step under `lib/` that imports `tomllib` or names `kinds.toml` outside a docstring. | 4, 7 |
| R53 | 2026-11-17 | The cut band of a narrow brought strip (asked by `spec/composition.md` 6.1; `coast.along_cliff` fell in `atlas`) | **Open to the owner** (ours). On posts coarser than the import's, a cutting path owner's strip narrower than half a parent quad's diagonal cuts as if that wide plus 6 mm (the 5 mm an arc's chords stray, `geometry.md` 3.4, and a millimetre of rounding), round at its ends, so a quad the line ends on at a corner is held; its skirt reaches that much again. Cost: a few more quads are cut round a strip's ends than the strip covers (`coast.in_cave`, 9 mm from such a corner in `atlas`, now stops 1.95 m short; TODO "From M7 close"). | 3, 8 |
| R54 | 2026-11-17 | An untrusted step's committed program digest (asked by `spec/protocol.md` 7.1; `basin` untrusted in a fresh checkout) | **Open to the owner** (ours). A configured program's digest in a committed result stands as recorded: the reader never runs it, and a copy rebuilt from the same source is other bytes. Cost, stated next to the rule: a change to the program alone no longer makes the result stale for an untrusted reader; the check left is a trusted `via step commit`, which prints each folder whose bytes changed (a writing-nothing `--check` mode is open). | 4, 7 |
| R55 | 2026-11-17 | What a build's leaves are keyed by (asked by `spec/graph.md` 5.3; `atlas` built at another depth took longer than cold) | **Open to the owner** (ours). The leaves hold the build key less `locations.pack_root`, which only `write:manifest.json` reads, so a folder at another depth reuses every memo and every node but the manifest's. The reuse is measured (85 s against 276 s cold, and 153 s under load), not checked by a case: no run report member counts memo hits. | 4, 7 |
| R56 | 2026-11-17 | Naming a walk's stop (asked by `spec/build.md` 15.1; `market.top` blocked by `null`) | **Open to the owner** (ours). A walk `blocked` having pressed against nothing steeper than it walks is named by what it stands on: `ground` with the steepest slope over the next metre ahead, or the surface or instance under its feet. A walker with nothing under its feet keeps `by` null. | 8, 12 |
| R57 | 2026-11-17 | How diagnostics word what they name (asked by `spec/diagnostics.md` 1.3, 1.4, `spec/build.md` case 7; the M7 outsider runs) | **Open to the owner** (ours). With no name close enough, the hint lists up to five names visible there, nearest first, ties bytewise. A world's diagnostics name an import outside its folder by a climbing relative path, never an absolute one. Messages and hints are still not compared, but a case written for a hint that misled an agent may read its text from the diagnostics document, saying the trip in a comment; a core hint names no library's attribute (`names = ["<attr>"]`, not a library's word). | 5, 8 |
| R58 | 2026-11-17 | A place on a turned import's edge (asked by `spec/composition.md` 6.7; `crossroads`) | **Open to the owner** (ours). A place within a millimetre of the import's footprint, and inside its extent, is on it, so it is held: a turned footprint's corners are rounded to the millimetre and a place on its edge can fall a fraction of one outside. | 2, 3 |
| R59 | 2026-11-17 | Library choices from the outsider runs (asked by `lib/building/lay.py`, `lib/ways/check.py`; tasks `mill` and `crossroads`) | **Open to the owner** (ours, library defaults). `lay` refuses a stairwell that stops between 0.5 mm and 0.3 m short of a corner (`STUB_MIN`, a corner piece's length). `end_unjoined` judges a way's end within `reach` (1 m) of an end of a path of another of `lane`, `road`, `trail`, `crossing`, with no meeting between them, and only the world's own; two paths of one kind ending together are not judged (a fork is written by name). A deck under its water stays `low_deck` (warning), worded "under the water". | 8, 13 |
| R27 | 2026-11-13 | The first cut | Everything the spike showed, rebuilt as final examples in the v1 vocabulary on the new core; nothing of the spike's core or words carries over. What is left out is in section 7. Milestones in `docs/plan.md`. | 4, 13 |

## 6. Queued for ruling

Found by the lead engineer reading the example world against the principles. Each has a
recommendation. Applied as recommended on 2026-11-13 as the v1 vocabulary (`spec/vocabulary.md`);
kept here as the record of why.

**Words with more than one meaning (9)**

| Word | Meanings today | Recommendation |
|---|---|---|
| `clear` | A field rules avoid; a bridge's headroom; the hand edit that empties a spot. | Keep the hand edit. Field becomes `keep_out`. Headroom becomes `headroom`. |
| `reach` | A kind's effect distance; the walkability lint; `expect.reach = false`. | Kind key becomes `radius`. The lint keeps the word. |
| `at` | A coordinate; a fraction along a row; when a step runs. | Coordinate only. Fraction is `t`. Step uses `when`. |
| `layer` | A document of the world; a row in a field's stack. | Document keeps it. Field rows become `then`. |
| `kind` on an instance | Lends the instance's footprint to a kind. | Rename to `ground = "pad"`; an instance has an asset, not a kind. |
| `tags`, `labels`, `taxon` | Three ways to classify, on fields and on assets. | `taxon` (one place) and `tags` (many). Drop `labels`. |
| `part` | A world joined into another; also everyday "building part". | `import`. |
| `guide` | A point to measure from; an outline to plant in. | Keep: both are "a shape that does nothing". Say that in one line. |

**Too many ways to say one thing (10)**

| Today | Recommendation |
|---|---|
| Where a thing goes: `at`, `on`, `beside`, `atop`, `along`, plus `offset`, `face`, `yaw`, `turn`. | One grammar: `at` (a coordinate) or `on` (a named thing, with `t` and `offset`), and `face`. |
| How paths meet: `joins`, `starts_at`, `ends_at`. | `starts_at` and `ends_at` only; a join is an end at a named thing. |
| Six rules: `set`, `run`, `kitrun`, `dress`, `flank`, `scatter`. | Three in core: `set`, `run`, `scatter`. `kitrun` to the building library. `dress` is a scatter on a thing. `flank` is a set on a mount. |
| `variant` (inline) and overlay (a file). | One concept, `change`, written inline or in a file. |
| Expressions as bare strings; formulas as `"= expr"`. | One convention for both. To be chosen with the expression language. |

**Domain words the core knows (5)**

| Word | Recommendation |
|---|---|
| `poi`, `bookmark` | Library kinds with a declared output; the core knows neither name. |
| `expect`, `profile`, the reach lint | A library on core lints. |
| `[look]` | Leaves the world file. It is the engine's side of the door, passed through. |
| Field names `height`, `ground`, `water` required by name | Declared roles (`role = "surface"`), not names. |

**Smaller**

- `[export] fields`: every field is exportable; the list is a loader option, not world text.
- A list replaced whole on merge forces a variant to restate it. Lists of tables get ids.
- `seed = 1` on every rule: default the seed to the rule's id; write one only to change it.

**Open (M5 close), for the owner**

- Stations along a stretch: `geometry.md` 3.6 step 4 stands them at `k L / n`, so shortening a
  stretch at its far end slides every station and every lot keeps its name but not its shape
  (quay `unrelated`: a lot 83 -> 87 outline points, a house no longer clear). Stations by distance
  from one end (the remainder at the far end) would move only the last cells, and would change
  every cells case's numbers. Ties to the pending case `identity/c27_stretch_lengthened`, which
  waits on the same ruling. Recommendation: by distance from the start, with c27 recorded under it.

## 7. Set aside

Decisions not to do something, with the reason.

| What | Until | Reason |
|---|---|---|
| Unreal (challenge 41) | After the first cut | The owner's call (2026-11-13). The build-directory chapter keeps an Unreal mapping table on paper and a second reader (glTF) so the format does not set around Godot. |
| Worlds past a few kilometres (tiled storage) | After the first cut | The stated users build a few kilometres. Fields are evaluated by window from the start, so tiling can be added without changing results. |
| Sky, clouds, the frame's look (11) | Not ours | The engine's side of the door (12). Time of day and sun direction are carried as facts. |
| Re-creating an existing project's map exactly (44) | After the first cut | Porting is low priority for the stated users. |
| Image-model styling of the player map | Not ours | The user's, outside the build. We hand over the map as data. |
| Testing on Linux and Windows (the cross-system half of R26) | Until the owner asks | The owner's call (2026-11-13): no need to test them now. The code stays portable (via-num, no platform paths), and the CI file is kept, unrun; same bytes is checked on macOS only. |
| A maintained engine plugin beyond the Godot loader | After the first cut | One loader done well first. |
