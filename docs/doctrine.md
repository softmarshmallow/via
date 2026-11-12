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
| R25 | 2026-11-13 | The one surfaces model | A surface is a bounded field with the role `surface`, owned by a feature: terrain over the extent, a deck over its strip, a floor at a level, the floors a volume yields. Where two surfaces meet is a pair of mounts. No new noun. Placement, grounding, reach and export all ask the same four questions of it. | 5, 10 |
| R26 | 2026-11-13 | Same text, same bytes | On macOS, Linux and Windows, for one engine version. All maths goes through one module; no non-finite value enters a build. A user's step is recorded, not promised. | 7 |
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

## 7. Set aside

Decisions not to do something, with the reason.

| What | Until | Reason |
|---|---|---|
| Unreal (challenge 41) | After the first cut | The owner's call (2026-11-13). The build-directory chapter keeps an Unreal mapping table on paper and a second reader (glTF) so the format does not set around Godot. |
| Worlds past a few kilometres (tiled storage) | After the first cut | The stated users build a few kilometres. Fields are evaluated by window from the start, so tiling can be added without changing results. |
| Sky, clouds, the frame's look (11) | Not ours | The engine's side of the door (12). Time of day and sun direction are carried as facts. |
| Re-creating an existing project's map exactly (44) | After the first cut | Porting is low priority for the stated users. |
| Image-model styling of the player map | Not ours | The user's, outside the build. We hand over the map as data. |
| A maintained engine plugin beyond the Godot loader | After the first cut | One loader done well first. |
