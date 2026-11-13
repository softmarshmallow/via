# via: the rewrite plan

## Context

The spike did its job: 19 worlds, 63 recorded requests, a doctrine with rulings R1 to R19. The last
irreversible choice is made (Rust core, R19). The pre-kickoff list has one gate left: define milestone
one. This plan defines it, and the road from there to a first cut.

The owner's three answers (2026-11-13) are built in:

- **Where:** branch `rewrite` in this repo. The spike and docs are committed there first as the
  record. `main` is untouched until you review. Nothing is pushed.
- **First cut:** everything the spike showed. Its worlds come back as the final examples, rewritten
  in the new vocabulary on the new core. Nothing of the spike's core or its words carries over.
- **Engines:** Godot only for the first cut. Unreal is set aside until after it, in writing.

## What is already fixed

Flat TOML keyed by id (R1). Formulas are our own closed language (R6, R7). Python for user code,
through an SDK (R2). Hand edits are intent at a place (R3). The core knows no domain noun
(principle 5). The setup follows the reference you named (`docs/references.md`, "Engine and SDK
setup"): one engine binary, an SDK that never links it, user code in its own process, and a
language-neutral `spec/` that is the contract.

## Rulings this plan adds

Recorded in `docs/doctrine.md` on approval. Each stays open to challenge.

| # | Ruling | Why |
|---|---|---|
| R20 | **Setup.** One Rust binary, `via`. `spec/` (chapters, schemas, test vectors, conformance cases) is normative, and conformance runs through the command line only. The Python SDK is pure Python and never links the engine: the engine starts a worker process and they talk JSON-RPC on stdio. Fields and large tables never travel as JSON: they are write-once files named by digest, memory-mapped on both sides. It must work on Windows. | R19a, R19b. The reference's two recorded failures were Windows and cross-platform bytes; both are designed for from day one. |
| R21 | **Order.** A field's rows are ordered by authority across the whole world. Layers break ties and decide what a step may read (the layers at or below it). A step returns rows and features, never a whole field. | Settles the open "what does a returned field mean". The design review proposed layer-first order because it is cheaper; rejected, because a derived lake in a lower layer would stop giving way to an anchored thing above it (principle 3). Cost accepted: one extra evaluation per distinct view a step reads. |
| R22 | **Lifecycle points are generate, lint, export.** `effect` and `place` are dropped: a generate step at a layer covers both. One request, two transports: a worker (may call the engine) and a plain command (files only). | Fewer mechanisms (10). |
| R23 | **Every engine call from a step is a batch:** columns in, columns out. Loops that would need thousands of calls (faces, cells along a frontage, a route over a cost field) are engine operations with no domain noun. | R19c in both directions; keeps Python libraries fast without a private door. |
| R24 | **Geometry in the core is integer millimetres.** Text stays decimal metres. | Exact tests, no tolerance tuning, same result on every machine. |
| R25 | **A surface is a bounded field with the surface role, owned by a feature** (terrain over the extent, a deck over its strip, a floor at a level). Where two surfaces meet is a pair of mounts. No new noun. | The "one surfaces model" the list calls the largest open question. |
| R26 | **Same text, same bytes, on macOS, Linux and Windows,** for one engine version. All maths goes through one module. A user's step is recorded, not promised. | Principle 7, made checkable. |
| R27 | **First cut and what is set aside** (doctrine section 7): Unreal (#41), until after the first cut; worlds past a few kilometres (tiled storage); sky and clouds (#11, the engine's); re-creating an existing project's map (#44, after the first cut); image-model map styling (the user's, outside the build). | Your answers; each with its reason. |

Also on approval: the queued renames in doctrine section 6 become the v1 vocabulary as recommended,
and `CONTRIBUTING.md`'s "Rust, whole hog, no Python rim" is superseded by R19.

## Shape

```
spec/            normative: chapters, schemas, vectors, conformance cases
crates/          the engine: num, diag, doc, expr, geom, raster, graph, model, proto, cli (+ step, erosion)
sdk/python/      thin port: pure Python, never links the engine
lib/             our libraries: kinds as TOML + steps in Python, through the SDK only
                 (land, water, ways, town, building, review, map)
engines/godot/   loader addon and the review viewer (eye-height sheets)
examples/        the final examples, each with change files and ground bookmarks
docs/            doctrine, challenges, design; pre-reboot research moves to docs/archive/
```

Mechanisms that carry the goal line:

- **One graph.** Every reference (`on`, mounts, formula reads, ends, `give`) is an edge, with one
  cycle report. Results are stored by content: an unchanged rebuild runs nothing, and a change
  re-runs only what read it. The engine records what each step actually read. A step can run once
  per item (`for_each = "kind:district"`), which is what turns a 200 s rebuild into a local one.
  The conformance runner adds one check to every case: a clean build equals the incremental one.
- **Geometry kernel (R19d).** We write: noding, faces that remember which path bounds them, cells
  along a frontage, a path along another feature, a masked region that keeps every piece. Region
  booleans and offsets come from an existing pure-Rust integer crate if it passes a bake-off on our
  fixtures; otherwise ours.
- **Formulas.** Typed (number or yes/no; constant, per feature, per post). Names checked before
  anything runs. A field never holds a bad number: a failure names the formula, the place and the
  fix. One function table generates the evaluator, the reference page and the SDK's list.
- **Names by place.** Generated things get ids from where they are, never from a counter. Authors
  select by place. Pins and clears speak to every producer.
- **Diagnostics.** The code cannot construct one without a code, a subject, a file and line, and a
  hint.
- **Build directory.** Its chapter is written before any loader, with a mapping table per engine
  (Godot now, Unreal on paper), and a second reader (a glTF export) keeps it from setting around
  Godot.

Carried over from the old workspace: `crates/via-terrain` only, as an erosion library that a step
calls (its `run_given` / `Given` entry, the seed derivation in `crates/via-artifact/src/seed.rs`,
and the thread-count determinism test pattern in `crates/via-terrain/tests/determinism.rs`).
Retired: `via-ecology`, `via-suitability`, `via-viz`, the old `via` CLI, `experiments/`, `.vrast`.

## Milestones

Each is closed by change scenarios, not by "it builds". Each ends with a sheet for you.

| | Goal | Proven by | Worlds | Rows closed |
|---|---|---|---|---|
| M0 | Decisions written; the two cheapest risks probed | Rulings recorded; vocabulary page; every line of SPEC section 11 and 12.8 sorted fix / accept / drop; a hello step (a 6.5 MB field out and back, a call back, a killed worker); one digest for a null case | none | 25-27, 37 |
| M1 | One decision settles and resettles, on foot | Below | `ford` (new) | 1, 3, 19, 49 (points) |
| M2 | Land and user code through one public door | A moved river mouth resettles; a new seed moves the valley and what stands on it; a step re-runs only when what it read changed; a revised asset names every instance it affects | `vale`, `glade`, `relief`, `massif`, `basin` | 2, 5, 6, 16, 21, 28-32, 40, 45, 50, 58 |
| M3 | A town that holds on foot and under edits | The moved hill; R13's four obligations; dead ends and decks no worse than R18's numbers; reach as a library lint | `quay` (new, 256 m), `townland`, `example`, `monastery`, `city` | 13, 34-36, 38, 39, 43, 49, 51, 59-63 |
| M4 | Worlds built apart compose by name | An import moved and turned; a forgotten exit says what to write; a moved square re-runs only its district | `strait`, `county` (new: the town as a fill on simulated land), `border` | 12, 22, 52, 54-57 |
| M5 | More than one surface; buildings as a library | The town kit's three owed changes; a building moved and turned keeps its walk; indoors reachable through a door; the measured object matches its published numbers | `coast`, `station`, `college`, `market`, `pistol`, `sidearm` | 7, 8, 14, 15, 18, 20, 33, 46-48 |
| M6 | Whole, and handed over | Every example joined in one map; chunks load and unload on a walk; an over-budget chunk fails and names the asset; the player map as data, per floor | `atlas` | 4, 23, 24, 42, 53 |
| M7 | Outsiders build with it; first cut | Agents from two model families build from the documents alone; `spike/` deleted; `main` moves at your review | all | 51 |

Order notes. `atlas` imports ten other worlds, so it is the capstone, not a composition test. The
change report, pins and clears, a lint step through the SDK, and the surfaces chapter all move into
M1, because every later acceptance check depends on them. Asset facts (measured, not asked) arrive
in M2; connectors in M5.

### Milestone one: `ford`

Two tracks after a short shared base (numbers, the integer lattice, the path frame), one gate.

**Track A, the kernel**, gated through the command line: faces of townland's street linework close
and count correctly; names do not change when a distant edge moves; the river's bank offsets without
crossing itself; every cell touches a street; a clear removes exactly one cell's building; a pin
inside one is reported; every piece of a masked region is kept; same bytes on every system.

**Track B, the slice:** documents, the graph, formulas, fields with roles, kinds as library data,
authority, `at` / `on` / `face`, scatter with pin and clear, diagnostics and `via explain`, build
directory v0, the change report, one lint step through the SDK, the Godot loader, the sheet tool.

`examples/ford/world.toml` is exactly this block, byte for byte, LF line endings and one final
newline. Cases that cite its lines and bytes read this text:

```toml
name    = "ford"
extent  = [0, 0, 256, 256]
include = ["land", "water", "ways", "review"]   # kinds as data; the core knows none of these words
pack    = "primitive"

[field.height]
role = "surface"
base = "= 20 + 0.06 * y + 5 * fbm(110, 3, seed = 'hills')"

[field.clearing]                          # the pad and the beck write it (role:keep_out)
role = "keep_out"
base = "= 0"

[field.water]                             # the beck writes it (tag:water_level); dry, it is the ground
tags = ["water_level"]
base = "= f('height')"

[feature.beck]
kind = "beck"
authority = "anchored"
path = [[150, 256], [140, 170], [160, 90], [150, 0]]

[feature.ford]                            # the named thing that owns where lane and water meet (R18)
kind = "ford"
on = { ref = "beck", t = 0.55 }

[feature.lane_west]
kind = "lane"
path = [[0, 120]]
ends_at = "ford"

[feature.lane_east]
kind = "lane"
starts_at = "ford"
path = [[256, 140]]

[feature.at_ford]                         # on the lane short of the ford; eye height is the review library's default
kind = "bookmark"
on = { ref = "lane_west", t = 0.93 }
toward = "ford"

[feature.on_bank]                         # on the bank along the water's edge
kind = "bookmark"
on = { ref = "beck", t = 0.45, offset = 4 }
toward = "ford"

[feature.to_door]                         # from the lane to the door
kind = "bookmark"
on = { ref = "lane_west", t = 0.6 }
toward = "cottage"

[feature.in_trees]                        # along the lane through the trees
kind = "bookmark"
on = { ref = "lane_east", t = 0.4 }
toward = "lane_east.end"

[instance.cottage]
asset = "house_low"
ground = "pad"
margin = 1
on = { ref = "lane_west", t = 0.6, offset = 12 }
face = "toward"

[scatter.trees]
assets = ["@plant/tree"]
spacing = 7
density = "= max(smoothstep(0.1, 0.5, fbm(60, 3)), near('lane_east', 16))"   # a wood along lane_east; the scatter applies keep_out itself

[step.checks]                             # the one lint step, through the SDK
when = "lint"
command = ["python", "-m", "viasdk", "lib:review/checks.py"]

[change.downstream.feature.ford]          # the change scenario is one line
on = { ref = "beck", t = 0.8 }

[change.hand.pin.oak]                     # the hand edits the spacing row holds
asset = "tree_oak"
at = [200, 60]
by = ["trees"]

[change.hand.clear.gap]
at = [208, 208]
radius = 6
by = ["trees"]

[change.denser.scatter.trees]
spacing = 6
```

**Ford's libraries.** `lib/land`, `lib/water`, `lib/ways` and `lib/review`, each a `kinds.toml`
(and `lib/review/checks.py`), are written first in M1's shared base: before `geometry.md` 2.6's
frame decision reads them, so before track A. From then their text is a fixture every ford case
reads; changing it is a spec change. Until written, this table binds them. Every effect has a
`radius`, its own or its kind's, so every entry has a window (`fields.md` 5.3), and every mask fades
to 0 one post spacing inside it, so no ring post is `field.effect_cut`.

| Kind | Library | Tier | Attributes | Effects: id, field | Radius |
|---|---|---|---|---|---|
| `beck` | water | `shaped` (ford's beck writes `anchored`) | `width` 6, `depth` 1.2, `flow` 0.3, `bank_grade` 0.2 | `bed`, `role:surface` (lowers the ground across `width`, its banks cut at `bank_grade` until they meet the ground); `bank`, `role:keep_out`; `level`, `tag:water_level` (the water surface in the channel) | 16 |
| `ford` | ways | `anchored` | `grade` 0.25 | `approach`, `role:surface`: `min(v, bed + grade * d)`, so it cuts the banks down to a slope of `grade` from its bed and asks `v` wherever the beck's bank already lies below that slope (`authority.md` 6); the beck's `bank_grade` at or below `grade` keeps the two in agreement | 21 (its mask fades by 20) |
| `lane` | ways | `shaped` | `width` 4, `camber` 0.02 | `bed`, `role:surface` | 12 |
| `pad` | land | `shaped` | none | `level`, `role:surface`, `under('mean')` (`surfaces.md` 6.1); `clear`, `role:keep_out` | 4 (its mask fades by 3) |
| `firm_pad` | land | `anchored` | none | As `pad`. The firm pad of `authority.md` 1.3; only the bad edit grounds with it | 4 |
| `bookmark` | review | `derived` | `toward` (required, a name), `eye_height` 1.7 | none; `output = "bookmark"` (`build.md` 9.2) | none |

So the pad keeps trees off the cottage's footprint, the lane's verge keeps them off the lane, and the
scatter's density thickens to a wood along `lane_east` (`near`, `formulas.md`), so `in_trees` looks
down a lane through trees in every build. The beck keeps trees off its bed and writes
`tag:water_level`, the tag the water library documents (`fields.md` 2.2, `build.md` 6.4). Without
`clearing` and `water` those effects would match nothing (`authority.no_field`). `toward` is the
review library's key for the place a bookmark looks at (`build.md` 9.2, 15.1); its name beside
`face = "toward"` is (open: needs ruling), principle 9. `tree_oak` is an asset
`lib/packs/primitive.toml` files under `plant/tree`. The lint step is restated in the world because
`protocol.md` 1 leaves open how a library offers a lint without it: (open), decided by
`lib/review`'s first lint in M1; until then `ford` declares it.

Until written, this table binds `checks.py` too. Each row is one `info` record per subject, code
`step.checks.<name>` (`diag.emit` sends `<name>`), the number in `data.value`, with `position` where
the measure is taken. The first record of each code in a call carries `explain` (`protocol.md` 3.3),
so a ford build raises no `protocol.no_explain`.

| Name | Subject | `data.value`, unit | `position` | Ford's value |
|---|---|---|---|---|
| `lane_gap` | `feature.lane_west`, `feature.lane_east` | Lane end to the ford, metres | The lane end | 0 |
| `approach_grade` | `feature.ford` | Steepest grade on `approach`, rise over run | Where it is steepest | Measured |
| `ford_depth` | `feature.ford` | `water` less `height` at the ford, metres | The ford | Measured |
| `water_step` | `feature.beck` | Largest jump in `water` between neighbouring posts along the bank, metres | Where it is largest | Measured |
| `cottage_relief` | `instance.cottage` | Highest less lowest `height` under the footprint, metres | The cottage | Measured |
| `trees_in_clearing` | `scatter.trees` | Trees where `clearing` holds, a count | None | 0 |

**Change files**, in `examples/ford/changes/`, each a change file (`documents.md` 6), requested with
`--change changes/<name>.toml`. Each file is exactly its body, byte for byte: each table header
line, then each key on its own line in the order shown, tables in the order shown, no blank or
comment lines, LF line endings and one final newline. Cases that cite their lines and bytes read
this text.

| File | Body |
|---|---|
| `vertex.toml` | `[feature.beck]` `path = [[150, 256], [140, 170], [164, 90], [150, 0]]`: the vertex `[160, 90]` moved 4 m east |
| `bigger.toml` | `pack = "packs/primitive_large.toml"`, the pack of `assets.md` 6.2, copied to `examples/ford/packs/` |
| `bad_anchor.toml` | Below: an anchored pad across the anchored beck's bed |
| `bad_typo.toml` | Below: `xx` at line 2, column 67, bytes `[82, 84]` (`diagnostics.md` 8) |
| `bad_cycle.toml` | `[field.loop_a]` `base = "= f('loop_b')"` `[field.loop_b]` `base = "= f('loop_a')"` |
| `bad_asset.toml` | `[instance.shed]` `asset = "shed_gabel"` `on = { ref = "lane_east", t = 0.5, offset = -10 }` |

`bad_anchor.toml`:

```toml
[instance.cottage]
ground = "firm_pad"
on = { ref = "beck", t = 0.3, offset = 2 }
```

`bad_typo.toml`:

```toml
[scatter.trees]
density = "= smoothstep(0.1, 0.5, fbm(60, 3)) * smoothstep(0, 20, xx)"
```

Each bad edit touches nothing the others need, so all four are reported in one run.

| Change | Must follow, and be reported | Must hold |
|---|---|---|
| Nothing | No node runs | Every byte the same, at 1 or 8 threads, on every system |
| `downstream` | `identity.md` 5 entries, each with its cause: `ford` (the edited entity), `lane_west`, `lane_east`, `at_ford`, `on_bank`, `to_door`, `in_trees` where their places follow the moved features, `cottage`, footprint `cottage`, the trees removed where the new pad and lanes give way and those added and removed where the wood follows `lane_east` (the scatter's density reads it), and the changed regions of `height`, `clearing` and `water` | No post of any field changes outside the union of the effect windows (`fields.md` 5.3) of every entry whose feature moved, old shape and new |
| `changes/vertex.toml` | `beck` and its regions of `height`, `clearing` and `water`; then every entry of `downstream` | As `downstream`, the beck's entries among those moved |
| `changes/bigger.toml` | The pad grows; the cottage named with old and new facts (`assets.md` 7) | World text untouched |
| `hand`, then `hand` and `denser` (spacing 7 to 6, under a clear and a pin) | Every `s7000_` tree removed, every `s6000_` tree added | The pin stands; nothing inside the clear; both hand edits unchanged |
| The four `bad_*` files together | `authority.conflict`, `formula.unknown_name` (nearest `x`), `graph.cycle`, `asset.unknown` (nearest `shed_gable`): code, subject, file and line, hint; `authority.conflict` and `asset.unknown` also a position | Exit status 1 |

Measured by the lint step, as its table above gives. Build times, cold and unchanged, come from the run report
(`<directory>.run.json`, `graph.md` 6.3), never from a lint, under a stated protocol: machine,
thread count, cold store against warm. Seen at 1.7 m, before and after, from the four bookmarks:
`at_ford` on the lane short of the ford, `on_bank` along the water's edge, `to_door` from the lane to
the door, `in_trees` along the lane through the trees. `build.md` case 27 lists all four in
`outputs.bookmark`.

## How every milestone runs

1. Chapter first, in `spec/`, with a tiny example. Mechanisms name their source; one without a
   source is marked as ours.
2. Conformance cases and vectors, failing.
3. Code until they pass. Checks on the code itself: no domain noun in core crates (deny-list test);
   library code imports only the SDK; all maths through the one module.
4. The final examples for that milestone, each with change files and ground bookmarks at its meetings.
5. A sheet for you (PNG: plan, eye-height views, before and after, numbers with protocol), the
   `docs/challenges.md` rows, and any new ruling.
6. A fresh agent, given only the documents, builds a small variation. What it trips on is fixed in
   the documents or the messages.
7. The matching spike module is retired from the oracle list.

The spike stays an oracle, never a source: it runs from a worktree pinned at tag `spike-final`, and
only exported data crosses (diagnostics, counts, measures, change reports, bookmark positions,
timings). Exact agreement is asked only for the merged document, exit status and findings on bad
edits; never for raster bytes, scatter positions, or found courses (compare measures, R15).

## First steps after approval (M0, in order)

1. Private-name check, then branch `rewrite`; commit the record (the spike, the docs, and the
   terrain patch as its own commit); tag `spike-final`. No push.
2. Record R20 to R27 and the set-aside list in `docs/doctrine.md`; write `spec/vocabulary.md` (one
   page, section 6 applied); rewrite `CONTRIBUTING.md` and `README.md`; move pre-reboot research
   docs to `docs/archive/`; re-file every open line of `TODO.md` to a milestone or the set-aside list.
3. Sort SPEC section 11 and 12.8, line by line.
4. Write the chapters M1 needs: documents and merge; numbers and the lattice; formulas and the
   function table; fields and roles; kinds, authority and order; the graph; surfaces; the kernel and
   names by place; build directory v0; diagnostics; the step protocol.
5. Workspace skeleton, the conformance runner, the three code checks. Owed from `conformance.md` 7
   at M0's close, before M1's cases are written: every Conformance row of every chapter has a case
   folder whose `milestone` equals the row's (xtask holds the rest of section 7: manifests read, no
   copy from `spike/`, code coverage by `xtask check --milestone N`).
6. The hello-step probe and the null digest: macOS, and Linux in a container.
7. Export the oracle data and the kernel fixtures from the spike.
8. M0 sheet to you; then M1's shared base, then its two tracks.

## Files

- **Created:** `spec/`, `crates/via-{num,diag,doc,expr,geom,raster,graph,model,proto,cli,step}`,
  `sdk/python/viasdk/`, `lib/`, `engines/godot/`, `examples/ford/`.
- **Changed:** `docs/doctrine.md`, `docs/challenges.md`, `TODO.md`, `CONTRIBUTING.md`, `README.md`,
  `Cargo.toml`, `.gitignore`; `crates/via-terrain` becomes the erosion library.
- **Retired:** the other old crates and `experiments/` at M0; `spike/` at M7 (the tag keeps it).
- Private material stays under the gitignored local folder and is never exported into fixtures.

## Verification

- `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test --workspace` (units, property
  tests and fuzzing for the kernel, vectors).
- The conformance runner against the built binary: `via build examples/ford/world.toml`, then each
  change; it asserts diagnostics, the change report, which nodes ran and which were reused, and that
  a clean build equals the incremental one.
- The Godot loader renders the ground bookmarks to a sheet with one command; I read the images
  before you see them.
- The same suite in a Linux container; Windows on hosted runners once you allow a push.
- Measures compared with the oracle's exported numbers, each with its protocol.
- The private-name check before every commit.

## Comes back to you later

- **Windows proof needs the branch pushed to hosted runners.** Until you say so, checks run on
  macOS and Linux only and Windows stays unproven. Asked at the M0 sheet.
- **A redistributable asset pack,** so example renders can be committed. I will propose candidates
  and ask before downloading anything.
- **The dogfood map.** Proposed: `county`. Yours to redirect.
- **Each milestone gate:** one sheet, your challenges.
