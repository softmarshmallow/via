# Conformance

This chapter fixes how a conformance case is stored and run. It gives one root for every case, one
manifest per case (`case.toml`), the checks the runner adds to every case, and how a case waiting on
a later milestone is marked. Each chapter's Conformance section says what its cases test; this
chapter says how any case is written down, so the runner (plan, M0 step 5) reads one form.

Authority: `docs/doctrine.md`, R20 above all ("conformance runs through the command line only").
"(open)" marks what is undecided, with what decides it. Milestone: M0, with the runner.

## 1. What a case is

A case is one row of a chapter's Conformance table (or one numbered item of its list), written as a
folder of input files and a manifest. The runner copies the inputs into a fresh working folder,
runs the manifest's actions in order, and compares what the `via` binary did with what the manifest
expects. It never links the engine and never uses the SDK; it reads build files with Python's
standard library and numpy, as `build.md`'s Conformance section already requires.

A case tests the binary through its command line only (R20). Tests of code (units, property tests,
fuzzing, the SDK's own tests, the readers' suites in `build.md`) are not cases and do not live here.

## 2. Where cases live

**One root.** Every case is a folder `spec/cases/<chapter>/<case>/`, where `<chapter>` is the
chapter's file stem (`build`, `rules`, `surfaces`) and `<case>` is the case's id.

**The case id.** It matches `[a-z][a-z0-9_]*`, at most 48 bytes, and is unique within its chapter.
It is the chapter's name for the row, in snake case (`pack_by_name`, `early_cutoff`). A chapter that
numbers its cases keeps the number as a prefix, a letter and two digits, so the folders sort in the
chapter's order: `build.md` case 1 is `c01_null`, `identity.md` case 25 is `c25_distant_edge`,
`rules.md` case r02 is `r02_on_path`.

**One row, one case.** A row whose input names several worlds is still one case: its folder holds
each world in its own subfolder. A row that spans two milestones (`build.md` case 29's "(M2:)",
`identity.md` case 3's "(M3)") is split into two cases, the second with the suffix `_m<N>`
(`c29_flags_in_key_m2`), because a case has one milestone (section 4).

**Inside the folder.**

| Path | Holds | Copied into the working folder |
|---|---|---|
| `case.toml` | The manifest (section 3). | No. |
| `expected/` | Files the checks compare against: recorded manifests, `id.txt`, ran-node lists, merged JSON. | No. |
| `later/` | Files an action writes into the working folder part-way through (section 3.3). | No. |
| anything else | The case's inputs: worlds, layers, change files, packs, step code, corpora. | Yes, at the same relative path. |

**Vectors and cases.** `spec/vectors/` keeps machine-readable vector files that more than one case,
or a code test, reads: `numbers.toml`, `graph/keys.toml`, `identity.toml`, the geometry fixtures and
`import.py`. A case reads them by copying them in (section 3.2). A file one case alone uses lives in
that case's folder, never under `spec/vectors/`.

**Where the chapters' paths go.** Several chapters named a case path before this chapter existed.
Until each owner updates its Conformance section to point here, this table wins:

| Chapter | Path it names | Becomes |
|---|---|---|
| `build.md` | `spec/vectors/build/null/` (`world.toml`, `manifest.json`, `id.txt`) | `spec/cases/build/c01_null/`, the recorded files under `expected/`. `graph.md`'s epoch guard reads them there. |
| `diagnostics.md` | `spec/conformance/diagnostics/fuzz/` | `spec/cases/diagnostics/fuzz/`, the documents under `corpus/` (section 3.4, `for_each`). |
| `fields.md` | `spec/vectors/fields/` "pictures and array files for the cases" | The case's own folder; `masks.toml` too unless a second case reads it. |
| `graph.md` | ran-set fixtures "under `spec/vectors/graph/`" | The case's `expected/ran.txt`. `keys.toml` stays a vector. |
| `protocol.md` | `spec/vectors/protocol/<case>/` | `spec/cases/protocol/<case>/`. The M0 probe keeps its own harness, as `protocol.md` says. |
| `rules.md` | `spec/cases/rules/<case>/` | Unchanged. |
| `surfaces.md` | `spec/vectors/surfaces/<case>/`, one shared test library | `spec/cases/surfaces/<case>/`. The library root `spec/vectors/surfaces/lib/` (library `surfaces_test`, a folder with `kinds.toml`, `documents.md` 4) stays; a case copies it as a folder and passes `--lib` to it. |
| `assets.md`, `formulas.md`, `geometry.md`, `identity.md`, `numbers.md` | vector files under `spec/vectors/` | Unchanged: they are vectors, not cases. |

The sentence each chapter's Conformance section gains: "Cases are stored and run as
`conformance.md` says: one folder per case under `spec/cases/<chapter>/`, with a `case.toml`."
Where that section already says "the runner also checks that a clean build equals the incremental
one", or "at 1 and 8 threads", it restates section 5 and may point to it instead.

## 3. The manifest: `case.toml`

Flat TOML keyed by id (R1), read with Python's `tomllib`. It is a test file, not world text: its
keys are not in `vocabulary.md`, and the deny-list (`vocabulary.md` 7) does not apply to it. An
unknown key, a missing required key or a wrong type is a case error (section 6), never ignored.

### 3.1 Top level

| Key | Type | Required | Meaning |
|---|---|---|---|
| `format` | string | yes | `"via-case-1"`. |
| `summary` | string | yes | One line: what the case shows, in the chapter row's words. |
| `milestone` | integer, 0 to 7 | yes | The milestone that first runs the case. Equals the chapter row's milestone (a workspace check, section 7). |
| `actions` | list of action ids | yes | The order the actions run in. Every id names an `[action.<id>]` table, and every such table is listed once. |
| `pending` | string | no | The case is not run, for this reason, whatever the milestone. For a wait that is not a milestone: a fixture not yet written, hosted runners. |
| `one_system` | string | no | The case's results are not compared across systems (section 5.3), for this reason. |

Actions are listed rather than ordered by id, because table order means nothing in TOML (R1's
shuffle cases depend on that).

### 3.2 `[copy.<id>]`: inputs from the repository

Copies a file or folder from the repository into the working folder before any action. Copies run in
id order, then the case's own files are laid over them, so a case file replaces a copied one.

| Key | Type | Required | Meaning |
|---|---|---|---|
| `from` | string | yes | A path relative to the repository root: `examples/ford`, `lib/ways`, `spec/vectors/surfaces/lib`. |
| `to` | string | no | A path relative to the working folder. Default: the same as `from`. `"."` copies a folder's contents into the working folder's root. |

A case never copies from `spike/`. Private material is never a case input.

A copy takes the fixture, never what a build left beside it: it skips every `.via/` store
(`graph.md` 3.2), `build/` directory (`build.md` 14), `*.run.json` run report and `*.lock` lock
(`graph.md` 6.3), and every file git ignores and does not track. So every case starts cold, whatever
was built by hand in `examples/`. A case's own files skip only `.via/` and git-ignored files, so a
case may plant a `build/` folder itself.

### 3.3 `[action.<id>]`: what the runner does

Each action holds exactly one of these keys, which says what it is:

| Key | Value | Does |
|---|---|---|
| `via` | list of strings | Runs the binary with these arguments (the program name left out). |
| `replace` | path | Replaces text in a file: `find` (string) with `with` (string). `find` must occur exactly once, or the case is in error. |
| `write` | path | Writes a file: from `text` (string), or from `from` (a path under the case's `later/`). |
| `delete` | path | Deletes a file or a folder. |
| `flip` | path | Inverts every bit of the byte at `at` (integer, a byte offset from the start). |
| `flip_object` | path of a run report | Inverts every bit of the byte at `at` of the store object holding the value the report names for node `node` (`graph.md` 6.3's `value`), in the store folder `store`: `<store>/objects/ab/<value>` (`graph.md` 3.2). For a store damaged after a build, which the case cannot name in advance (`graph.md`'s `corrupt`). A node with no value, or no such object, is a case error. |
| `rerecord` | path of a file in a build directory | Rewrites that file's record in the build's `manifest` (a path): its entry in `run.reports`, else in `files`, gets the file's `bytes` and BLAKE3 `digest` as it now stands. Nothing else of the manifest changes; it stays canonical. A file the manifest does not record is a case error. For `build.md` case 10's "its record in `run.reports` updated". |
| `truncate` | path | Cuts the file to `bytes` (integer) bytes. |
| `derive` | path | Writes a `via kernel` input (`geometry.md` 7.1) made from the kernel output `from`, so one call feeds the next. `as = "segments"`: noding's segments, each a two-point `path` with source `s<index>` (`geometry.md` 3.1 Idempotent). `as = "cells"`: a folder of one `cells` input per area of the network output `from`, with `input`'s shapes and grid, `params` added, the area by its start vertex and as frontage every `(source, edge, side)` on its shell, the side facing the area. `as = "interpolate"`: the shapes of `input` with `t` each `project` result's `t`, in order. `as = "project"`: the shapes of `input` with `points` each `interpolate` result's `point`. `as = "params"`: the kernel input `from` with its `params` replaced by the action's `params` (a fixture asked with other parameters). Nothing is computed but this rewriting. |

Every path is relative to the working folder. Only `via` actions carry the keys below and the
checks of section 3.4. The others are how a case edits text between builds ("then move the lane's
end"), and how `build.md` case 9 damages a build.

| Key on a `via` action | Type | Required | Meaning |
|---|---|---|---|
| `exit` | integer, or list of integers | yes | The exit status (`diagnostics.md` 7). A list allows any of them: the fuzz corpus's `[0, 1]`. |
| `out` | string | on `build` and `lock` | The build directory the action writes, relative to the working folder (`build/ford`). The runner checks the binary wrote there, and reads the run report as `<out>.run.json` (`build.md` 11.3). Written even when it is the default, so the case also tests the default. |
| `env` | table of strings | no | Variables added for this action alone: a test switch such as the one `build.md` case 24 needs. Placeholders allowed. |
| `holding` | string | no | A path the runner holds an operating-system advisory lock on while the action runs (`graph.md`'s `busy` case). |
| `for_each` | string | no | A glob relative to the working folder. The action runs once per match, in bytewise order of path, with `{item}` replaced by the match. Its checks apply to every run. |
| `diagnostics` | string | no | How the world diagnostics are compared: `exact` (default), `includes` or `any` (section 3.5). |
| `untouched` | list of strings | no | Paths the action must not create, delete or change (bytes and modification time). `build.md` case 3's "writes nothing", case 23's "the files untouched". |
| `pids_gone` | string | no | A file of process ids, one per line, written by the case's step fixture; after the action none of them is alive (`protocol.md` Conformance). |
| `no_rerun` | string | no | Leaves this action out of the re-runs of section 5.1, for this reason. For a lock held by the runner, a killed process, a store deleted on purpose. |

**Placeholders.** In `via` arguments and `env` values only, the runner replaces:

| Placeholder | Becomes |
|---|---|
| `{work}` | The working folder's absolute path. |
| `{case}` | The case folder's absolute path. |
| `{repo}` | The repository root's absolute path (`--lib {repo}/lib`). |
| `{python}` | The absolute path of the Python running the runner, for a step's program (`via config program python {python}`). |
| `{item}` | Under `for_each`, the current match. |

Under `for_each`, `{item}` is also replaced in the string arguments of the action's named checks
(3.4), so each run's check reads that run's files.

Nothing in a copied or written file is replaced. A case that needs an absolute path in a file, such
as a program path in `config.toml`, gets it by running the command that writes that file.

### 3.4 Checks on a `via` action

Checks are sub-tables of the action. Each is keyed by an id of the case's choosing, so a failure
names it. A `via` action with no check still gets the checks of section 5.

**`[action.<a>.diag.<id>]`: an expected diagnostic.** Section 3.5.

**`[action.<a>.file.<id>]`: a file, a folder or standard output.** It names one of `path` (relative
to the working folder) or `stream = "stdout"`, then any of:

| Key | Holds when |
|---|---|
| `exists` | `true`: the path exists. `false`: it does not (`build.md` case 11's "is gone"). |
| `listing` | The folder holds exactly these entries, by name, at its top level (`build.md` case 21: `["diagnostics.json", ".previous"]`). |
| `equals` | Byte-identical to this file under `expected/`. |
| `ignore` | With `equals`, on JSON: these members, as JSON Pointers (RFC 6901), are removed from both sides first, and both sides must be canonical (`documents.md` 8). `build.md` case 1's `["/run", "/locations"]`. |
| `text` | The file's whole contents equal this string. |
| `size` | The file has this many bytes. |
| `pointer` | A JSON Pointer into the JSON file, or into line `line` (1-based) of a JSON Lines file. Compared with `value`, `value_file` or `includes`. |
| `value` | Equal to the pointed value. Numbers compare by exact value (a TOML integer and float are equal when their values are); strings and booleans exactly; lists element by element; tables member by member, with no extra member. |
| `value_file` | A file under `expected/` whose text, less one final newline, equals the pointed string (`build.md` case 1's `id.txt`). |
| `includes` | The pointed table has these members with these values, or the pointed list holds these elements. Extras allowed. |
| `dtype`, `shape` | On a `.npy` file: its header's `descr` and `shape`. |
| `post` | On a `.npy` file: `[i, j]`, column `i` then row `j` as the chapters write a post, read as `array[j, i]`; for a three-dimensional array, `plane` (integer) selects the first index. Compared with `value` or `bits`. |
| `all` | On a `.npy` file: every element equals this value. |
| `bits` | The element's exact bits, as hexadecimal (`"0x3EAAAAAB"`), for a value no decimal names safely. |
| `tolerance` | An absolute tolerance for `value` and `all` on array elements. Default 0. A case uses one only where its chapter's row states one. |

A `value` compared with a 32-bit float element is first narrowed to 32 bits, ties to even
(`numbers.md` 8), then compared exactly: "the 32-bit float nearest 31.8" is `value = 31.8`.

**`[action.<a>.nodes]`: the ran-node sets** (`graph.md` 6.3). Read from the action's run report.

| Key | Holds when |
|---|---|
| `ran` | The set of `ran` nodes is exactly this list. An entry ending in `*` matches one or more node ids with that prefix. |
| `ran_file` | As `ran`, from a file under `expected/`, one node id per line (`graph.md`'s fixtures). |
| `ran_includes` | These nodes ran; others may have. |
| `reused` | These nodes are `reused`. |
| `not_ran` | None of these nodes ran. |

**`[action.<a>.same.<id>]`: the same as an earlier action.** For "same JSON", "same digest", "same
bytes as before".

| Key | Meaning |
|---|---|
| `action` | An earlier `via` action of this case. |
| `what` | `id` (the build's `id`), `files` (every file of `files` byte-identical, as section 5.1), `diagnostics` (`diagnostics.json` byte-identical), `stdout`, `ran` (the same `ran` set), or a path relative to the working folder. |
| `ignore` | As in a file check: JSON members removed from both sides first. |
| `differ` | `true`: the two must differ instead (`build.md` case 4's "`run.key` and `run.inputs` differ"). |

**`[action.<a>.check.<id>]`: a named check**, for a property that is not a value: a sum, a count of
crossings, an identity between areas. `fn` names it; its other keys are its arguments. A named check
is a function in the runner, listed in this table with the chapter that asks for it. A case naming a
check not listed is a case error. Rows are added with the cases that need them.

| `fn` | Arguments | Holds when | Asked by |
|---|---|---|---|
| `weights_sum_at_most` | `path`, `limit` | Every post of a weights file sums to at most `limit`. | `fields.md` "weights sum" |
| `post_step_at_most` | `path`, `resolution`, `window`, `limit`; `origin` (default `[0, 0]`) | Between every two neighbouring posts, east or north, both inside `window` (`[x0, y0, x1, y1]`, metres, edges included), the scalar field file differs by at most `limit`: no step where a profile should run on a slope (a frame read past its bound, `geometry.md` E1). | `geometry.md` 2.6 |
| `weights_sum_255_or_0` | `path` | Every post of a weights chunk sums to 255 or 0. | `build.md` cases 17, 18 |
| `no_absolute_path` | (none) | Section 5's U5, on demand inside a sequence. | `build.md` case 7 |
| `changed_within_windows` | `before`, `after` (build directories), `fields`, `kinds`, `world` | Every differing post of each named field lies in the union of the 5.3 windows of every moved feature or footprint, old shape and new, as `fields.md` defines it. | `fields.md` "change windows"; plan, ford rows `downstream`, `vertex` |
| `no_origin_inside` | `build` (a build directory); `at` and `radius`, or `points` (a closed polygon); `prefix` (optional) | No row of `instances/x.npy`, `y.npy` whose name in `names.txt` starts with `prefix` has its origin at distance `<= radius` from `at`, or inside `points` (`identity.md` 4.3). | plan, ford row `hand`; `identity.md` 31; `rules.md` r38 |
| `changes_by_prefix` | `path` (a `changes.json`), `prefix`, `what`; `names` (optional, a `names.txt`) | At least one entry's `name` starts with `prefix`, and every such entry's `what` equals `what`. With `names`: every line of it starting with `prefix` is such an entry. | plan, ford rows `hand`, `denser`; `identity.md` 31 |
| `array_window` | `whole`, `part` (two 2-D array files), `window` (`[i0, j0, i1, j1]`, `i1`, `j1` excluded) | `part` equals that window of `whole`, element by element, bit for bit (`numbers.md` 6). | `numbers.md` window_equals_whole, halo_edge; `build.md` case 38 |
| `json_count` | `path` (a JSON file), `pointer`, `count` | The list or object at `pointer` has exactly `count` members. | `geometry.md` (counts of vertices, segments, junctions, edges, areas, cells, pieces); `identity.md` 47 to 49 |
| `run_node` | `path` (a run report), `node` (a node id), `includes` (a table) | The report's node object with that id exists and holds these members with these values (3.4 `includes`). | `graph.md` why, lint gate, failed step |
| `run_report_valid` | `path` (a run report) | `graph.md` 6.3: canonical JSON, `format` `"via-run-0"`, every member of 6.3; `nodes` lists every node once, in id order; `totals` counts equal the statuses; every `ran` node has a cause, every other `null`. | `graph.md` run report, why |
| `pack_root_resolves` | `build` (a build directory) | `build.md` 4: the manifest's `locations.pack_root`, read from that directory, names an existing folder holding every file `assets.files` lists. | `graph.md` two depths |
| `json_list` | `path`, `pointer`; `from`, `at_least`, `includes`, `starts`, `last`, `nulls` (optional) | The list at `pointer` has at least `at_least` (default 1) elements from index `from` (default 0); each of those includes `includes` (3.4 `includes`), has each member of `starts` (a table of strings) beginning with that string, and has each member named in `nulls` (a list of strings) present with the value `null`, which TOML cannot write; its last element equals `last` (3.4 `value`). | `build.md` 11.2, plan ford row `downstream` (the trees past the written entries; a cause's `via` ends at the thing's node); `graph.md` 6.4 (`causes` null against a build of another store) |
| `instance_at` | `build`, `name`, `at` | The build holds exactly one instance named `name`, its plan origin exactly `at` in integer millimetres (`build.md` 8). | plan, ford row `hand` ("the pin stands"); `identity.md` 31 |
| `changes_inside` | `path`, `prefix`, `at`, `radius`, `what`, `cause` (a table) | At least one entry whose name starts with `prefix` has its `before` or `after` `at` at distance `<= radius` from `at`; every such entry's `what` equals `what` and one of its causes includes `cause`. | `identity.md` 4.3, 5, 31 (what a clear removed, and its cause) |
| `changes_absent` | `path`; `entry_names`, `causes` (lists, optional) | No entry's `name` is in `entry_names`, and no entry has a cause whose `cause` starts with a string of `causes`. | plan, ford rows `downstream` (`beck` untouched) and `hand` ("both hand edits unchanged") |
| `vectors_match` | `path` (a vector file of `numbers.md` 10's form); `fns` (optional, a list of `fn` names) | Every vector of the file, or of those whose `fn` is in `fns`, is printed by the action (`via vectors`, `build.md` 14) as `{"got", "id"}`, and its `got` equals the vector's `expected`: bit patterns exactly, numbers by value. | `build.md` 14 ("the runner compares `got`"); `numbers.md` seed_vectors; `formulas.md` 29, 52; `identity.md` 45 |
| `network_valid` | `path` (a `network` output), `input` (its input) | Recomputed from the segments and the input's path ends: Noded (`geometry.md` 3.1); `components` the count of connected pieces; Euler at both levels; the junctions of 3.2's definition, by `(x, y)`, and as many edges as chains between them; as many areas as bounded faces, each area's shell one of them, canonical (2.5), its holes clockwise, its `twice_area` its rings'; Closure exactly. | `geometry.md` 3.2, `linework_a` network |
| `network_measures` | `path`, `reference` (`fixtures/kernel/townland/reference.json`) | Always: a note on the outcome gives the area count and total area beside the reference's (a measure, never a failure; `fixtures/README.md`). | `geometry.md` 6 Measures |
| `noding_valid` | `path` (a `noding` output), `input`; `bound` (mm, default 1.5) | Noded; each segment's ends within `bound` of every source segment its provenance names (Close); every source vertex within `bound` of a segment naming its source, and each path's `t` intervals covering 0 to 1 (Faithful). | `geometry.md` 3.1, `near`, `linework_a` |
| `noding_same` | `first`, `second` (two `noding` outputs) | The same `vertices` and the same segment `ends`, byte for byte (provenance differs by construction under `derive`). | `geometry.md` 3.1 Idempotent |
| `names_same` | `first`, `second` (two `names` outputs) | The same junction, edge and area names, as sets; none empty. | `identity.md` 47 |
| `cells_valid` | `path` (a `cells` output), `input` (its input), `network` (the network the area came from); `bound` (mm, default 1.5) | Fronts: each cell borders its frontage edge along at least `min_cell_length`, ends within `bound` of the edge, or a `geom.cell_small` lies within 1 m of it; one cell per frontage and station; Closure: cells and `rest` sum to the area's twice-area within `2 × bound × perimeter`, what noding may move an outline; no cell gives `geom.cell_none`. | `geometry.md` 3.6, `linework_a` cells |
| `course_valid` | `path` (a `features.json`), `feature`, `start`, `end` (metres), `min_segment` (metres); `passes` (optional, points), `within` (optional, `[x0, y0, x1, y1]` metres) | The named path starts at `start` and ends at `end` exactly, has each of `passes` as a vertex, turns by no more than 90 degrees at any vertex, and has no segment shorter than `min_segment`. With `within`, every vertex lies in the box, edges included. | `derived.md` 3 step 6, d22, d25 |
| `entry_causes` | `path` (a `changes.json`), `table`, `prefix`; `named` (optional, cause strings) | At least one entry of `table` has a name starting with `prefix`; every such entry's `causes` is a non-empty list holding each of `named` as a `cause`. | `build.md` 11.2 ("never `[]` for an entry"), `graph.md` 6.4 |
| `sample_terrain` | `path` (a `sample` answer), `input` (its question), `field` (the built terrain field), `manifest` | Each row is `surfaces.md` 2.2 computed from the field's posts by `fields.md` 1.2's cell and order, in f64, bit for bit, `inside` true. | `surfaces.md` sample_edge |
| `roundtrip_point` | `path` (an `interpolate` output), `input` (the `project` input whose results fed it, `derive as = "interpolate"`); `bound` (mm, default 1) | One result per point of `input`, each within `bound` of the exact nearest point of `input`'s one path or ring to that point, computed in rationals, ties to the first along the line. | `geometry.md` 3.8, Conformance |
| `roundtrip_t` | `path` (a `project` output), `input` (the `interpolate` input whose results fed it, `derive as = "project"`); `bound` (mm, default 1) | One result per `t` of `input`, each with `|t' - t| × L` at most `bound`, `L` the line's length in doubles. | `geometry.md` 3.8, Conformance |
| `stroke_measures` | `path` (a `stroke` output), `reference` (`fixtures/kernel/townland/reference.json`), `distance` (metres) | Always: a note on the outcome gives each result region's area (shells less holes) beside the reference's `river_offsets` area at that distance (a measure, never a failure). | `geometry.md` 6 Measures, `course_a` |
| `names_count` | `path` (a `features.json`, or a text file of one name per line such as `instances/names.txt`), `prefix`, `count` | Exactly `count` names in the file start with `prefix`. | `geometry.md` 8 (areas, cells and rest pieces of a table counted); `identity.md` 23 |
| `names_kept` | `first`, `second` (each a `features.json` or a names file), `prefix`; `except_within` (optional, `[x0, y0, x1, y1]` metres) | The names starting with `prefix` are the same set in both, and not empty. With `except_within`, a name whose feature has a vertex strictly inside the box (edges excluded, so a neighbour sharing the box's edge stays checked) in either file is left out of both first. In an `instances/names.txt` each name's one vertex is its origin, from `x.npy` and `y.npy` beside it (`build.md` 9.2). | `geometry.md` 8 (area names under a distant edit); `identity.md` 25, 26 |
| `id_rule` | `build` (a build directory) | `build.md` 12: `manifest.json` is canonical, and its `id` equals the BLAKE3 digest of the manifest less `id`, `run` and `locations`, written as canonical JSON without its closing line feed (the form the null vector's `id.txt` fixes, `build.md` 12). | `build.md` case 10 |

### 3.5 Expected diagnostics

Where the records come from:

- The runner passes `--diagnostics {work}/.diag/<action id>.json` to every `via` action and reads
  both kinds of record from that document, on every exit status, 2 included (`build.md` 14). It
  never adds `--json`: on `via merge`, `via pack list` and `via vectors` that is `cli.usage`. An
  action whose `exit` allows only statuses above 3 (a process a test switch stops, `build.md` case
  24's 9) is not asked for the document: the stopped process writes none.
- **Run diagnostics** are the records whose codes `build.md` 11.1 lists as run codes, the ones
  never in `diagnostics.json`. An expectation for one says `from = "run"`.
- **World diagnostics** are all the others.

Records whose code starts `step.` (a lint's measures, R15) are left-over records allowed in every
mode, unless the action writes an expectation with a `step.` code; then they are compared by the
action's mode like any other world record.

An expectation is a table. Only `code` is required. Every other key, when written, is compared; a
key not written is not compared. Messages and hints are never compared (`diagnostics.md` 1.3).

| Key | Compared with |
|---|---|
| `code` | The record's code. |
| `severity` | Its severity. Usually left out: section 5's U2 already checks a core code's severity against the registry. |
| `subject` | Its subject (`diagnostics.md` 1.1). |
| `file`, `line`, `column`, `bytes` | Its span or file location (`diagnostics.md` 1.2). |
| `argument` | Its argument location. |
| `position` | `[x, y]` or `[x, y, z]`, metres, exactly. |
| `position_within` | `[x0, y0, x1, y1]`, metres: the record has a position, and its `x`, `y` lie in this box, edges included. For a position the chapters fix only to a region (a post of largest difference over measured ground). |
| `count` | Its `count`. |
| `occurrences` | The number of its `occurrences`. |
| `data` | A table: each member written equals the record's `data` member. Members not written are not compared. |
| `data_has` | A list of names: each is a member of the record's `data`, and not `null`. For a value the chapters require but do not fix (a failed fact's "what would hold", R14). |
| `data_below` | A table `{ a = "b" }`: the record's `data` members `a` and `b` are both numbers and `a` is less than `b`. |
| `related` | A list of tables `{ file, line }`: each is among the record's `related`. |
| `chain` | A list of tables `{ through, name, file, line }`: as long as the record's `chain`, in its order, and each entry includes the members written (3.4 `includes`), so an unwritten `owner` is not compared. |
| `failed_on` | Its `failed_on`. |
| `records` | How many records match this expectation. Default 1. `diagnostics.md`'s "ten `place.relief` records" is `records = 10`. `0`: no record agrees with the expectation, whatever the mode ("no `build.baseline_missing`"). |
| `from` | `world` (default) or `run`. |

**Matching.** The runner finds an assignment in which each expectation takes `records` distinct
records that agree with every key it writes, and no record serves two expectations. Then, by the
action's `diagnostics` mode:

| Mode | Holds when |
|---|---|
| `exact` | The assignment exists and no world diagnostic is left over, `step.` codes aside (above). The default, so an unexpected warning fails the case. |
| `includes` | The assignment exists. Left-over records are allowed. |
| `any` | No expectation is compared; only section 5's checks run. For the fuzz corpus. |

Run diagnostics are always compared as `includes`: a first build's `identity.no_previous` belongs to
the sequence, not to the row being tested.

### 3.6 An example

`build.md` case 1, the null world:

```toml
format    = "via-case-1"
summary   = "The null world builds to the recorded manifest and id, and passes via check."
milestone = 0
actions   = ["build", "check"]

[action.build]
via  = ["build", "world.toml"]
out  = "build/null"
exit = 0

[action.build.file.manifest]
path   = "build/null/manifest.json"
equals = "expected/manifest.json"
ignore = ["/run", "/locations"]

[action.build.file.id]
path       = "build/null/manifest.json"
pointer    = "/id"
value_file = "expected/id.txt"

[action.check]
via  = ["check", "build/null"]
exit = 0
```

`build.md` case 21, the failed build, with `ford`'s change `changes/bad_typo.toml` (`docs/plan.md`):

```toml
format    = "via-case-1"
summary   = "A formula typo in ford: one error at the typo, no file but diagnostics.json and .previous."
milestone = 1
actions   = ["program", "trust", "good", "typo"]

[copy.ford]
from = "examples/ford"
to   = "."

[action.program]
via  = ["config", "program", "python", "{python}"]
exit = 0

[action.trust]
via  = ["trust", "world.toml", "--yes", "--change", "changes/bad_typo.toml"]
exit = 0

[action.good]
via  = ["build", "world.toml", "--out", "build/ford"]
out  = "build/ford"
exit = 0

[action.typo]
via  = ["build", "world.toml", "--change", "changes/bad_typo.toml", "--out", "build/ford"]
out  = "build/ford"
exit = 1

[action.typo.diag.skipped]
code    = "diag.skipped"
subject = "scatter.trees"
count   = 1
data    = { first = ["step.checks"] }

[action.typo.diag.typo]
code    = "formula.unknown_name"
subject = "scatter.trees.density"
file    = "changes/bad_typo.toml"
line    = 2
data    = { name = "xx", nearest = "x" }

[action.typo.file.nothing_else]
path    = "build/ford"
listing = ["diagnostics.json", ".previous"]
```

### 3.7 The raw worker

A case that needs a worker breaking a rule the SDK keeps (`protocol.md` cases 57 to 61) uses the
raw worker, `tools/conformance/raw_worker.py`: a Python program that speaks `via-step-1` on
descriptors 0 and 1 itself, without the SDK, and misbehaves in the one way its first argument names.
It is a step program like any other (R34), copied in as an input (3.2) and declared by the case's
world: `command = ["python", "steps/raw.py", "<fault>"]`, `worker = true`. It runs under the
program the case records (`via config program python {python}`) and is trusted by the case's own
`via trust`.

| Fault | The worker |
|---|---|
| `none` | Breaks nothing: lists every point in `capabilities.when`, answers each `step.call` with an empty document, exits on `shutdown` and `quit`. |
| `when_unserved` | Lists only `export` in `capabilities.when` (`protocol.md` 3.2). |
| `message_too_large` | Answers `step.call` with one frame of 16 MiB and one byte, 16,777,217 bytes (3.1). |
| `blob_digest` | Returns its document as a blob named by another digest: the bytes of `# raw` and a line feed, stored at the BLAKE3 name of `# other` and a line feed (4.1). |
| `blob_header` | Calls `expr.eval` with an `i32` column `x` of shape `[2]` whose blob, named by its own digest, holds 4 bytes, not 8 (4.2, 4.3); then fails the call with -32010, as the SDK does when an engine error is not caught. |
| `slow_exit` | Serves as `none`, then answers neither `shutdown` nor `quit` and stays alive 6 s, past the 2 s deadline (6.2). |

The faults that name a blob need BLAKE3: such a case also copies the runner's
`tools/conformance/via_conformance/blake3.py` beside the worker, as `steps/blake3.py`. The worker
writes its engine's answers to standard error, the step's log. A fault it does not know ends it
with status 2 before `initialize` is answered. Faults are added with the cases that need them.

## 4. Milestones and pending cases

- The runner is given the milestone being worked on: `--milestone N`. A case whose `milestone` is
  above `N` is **pending**: it is parsed and listed with its milestone, not run, and not a failure.
  This is what `build.md` means by "M1's conformance runner treats a case marked with a later
  milestone as pending".
- A case with `pending` is pending at every milestone, and the runner prints the reason. A pending
  case is still parsed, so a malformed one is an error now, not later.
- A case at or below `N` that fails is a failure. There is no way to mark one "expected to fail".
- A row whose case folder is not yet written is not pending; it is missing, and the workspace check
  of section 7 reports it.

## 5. What the runner adds to every case

These checks need no line in `case.toml`. Each comes from the chapter named.

| # | Check | On | Source |
|---|---|---|---|
| U1 | The exit status is the action's `exit`. | every `via` action | `diagnostics.md` 7 |
| U2 | Every diagnostic has a registered code, a subject, a location and a non-empty hint; a core code's severity equals the registry's; a step code has one severity in the build; `failed_on` appears only on a code `--fail-on` named. | every `via` action | `diagnostics.md` Conformance |
| U3 | With exit status 2, nothing under the working folder is created, deleted or changed, but the `--diagnostics` file (3.5). | every `via` action | `diagnostics.md` 7 |
| U4 | `via check <out>` exits 0. | every `build` or `lock` action that exits 0 | `build.md` case 8 |
| U5 | No JSON or text file in the build directory (`out`), `<out>.run.json` or the `--diagnostics` file holds a path starting `/`, `\`, `//` or a drive letter, the working folder's path, the test user name, or the host name; no `..` but in `locations` and in source paths the text wrote so; no absolute path but in the run report's `steps[].command`, which repeats the text as written (`graph.md` 6.3; such text is `protocol.absolute_path`), and in a diagnostic's `data.pointer`, a JSON pointer into a model file (RFC 6901), not a path. `.config`, `.home` and the store are exempt: `config.toml` and `trust.toml` hold absolute paths by design (`protocol.md` 6.3), and `via config program` and `via trust` are checked by their own cases. | every `via` action | `build.md` case 7 |
| U6 | Clean equals incremental, and 1 thread equals 8 (5.1). | every `build` or `lock` action, unless `no_rerun` | `graph.md` 7, 8; plan |
| U7 | The same results on every system (5.3). | every case, unless `one_system` | R26 |

### 5.1 The re-runs

- The runner adds `--threads 8` to every `build` and `lock` action that does not give `--threads`.
- After such an action, it copies the working folder's inputs as they stand (not the build
  directories, the run reports or the store) into a fresh sibling folder with the same layout, and
  runs the same arguments there with `--threads 1`: a fresh store into a fresh directory. A build
  directory the action reads, named by `--against` or holding a `file` input, is an input and is
  copied too, as it stands. The
  re-run keeps the same environment (5.2), so the same `VIA_CONFIG_DIR`, programs and trust;
  `.config`, `.home` and `.diag` are never copied and never compared.
- It then compares: the same exit status; `diagnostics.json` byte-identical; and, when the action
  exited 0, the same `id` and every file of the manifest's `files` byte-identical. `changes.json`,
  `.previous` and the run report are left out: they depend on the baseline or the run
  (`build.md` 12).
- One extra build thus checks both clean against incremental and 1 thread against 8. When it fails,
  the runner builds once more, clean at 8 threads, and says which of the two broke.
- A case that tests threads or the store itself (`graph.md`'s `threads`, `empty store`) writes those
  actions explicitly, with `--threads` given and `no_rerun` where the re-run would test nothing.

### 5.2 The environment

Each case runs in its own **working folder**, a fresh folder whose name holds a random token, so U5
can find a leaked path. Every action runs with the working folder as its current folder. Each `via`
process starts from an empty environment plus:

| Variable | Value |
|---|---|
| `PATH` | The runner's own. On Windows also the runner's `SYSTEMROOT`, `WINDIR`, `COMSPEC` and `PATHEXT`: machine settings, not the user's, without which Windows and many Python packages a worker imports fail. |
| `VIA_LIB` | `{repo}/lib` (`documents.md` 4.1), so `via build examples/ford/world.toml` runs as written. A case's own `--lib` roots are searched first. |
| `VIA_CONFIG_DIR` | `{work}/.config`, empty at the start. So no case reads the machine's real `config.toml` or `trust.toml` (`protocol.md` 6.3). |
| `HOME`, `USERPROFILE`, `XDG_CONFIG_HOME`, `APPDATA`, `TMPDIR`, `TMP`, `TEMP`, and `LOCALAPPDATA` on Windows | Folders under `{work}/.home`. |
| `USER`, `USERNAME` | `via-test-<token>`, unique per run, for U5. |
| `NO_COLOR` | `1`. Terminal output is never compared but for the summary line (`diagnostics.md` 7). |
| the action's `env` | As written. |

The runner never trusts a world or records a program itself: those are the case's own actions
(`documents.md` 4.1). A `ford` case runs `via config program python {python}`, then `via trust
world.toml --yes`, with each change it requests, before it builds (3.6). The cases that test trust
itself (`protocol.md` cases 6 to 8, 38) write their own.

The host name cannot be changed without privileges on every system, so U5 searches for the real
one. In the Linux container the runner sets a unique host name as well.

### 5.3 Across systems

The runner writes one record per run, `<os>-<arch>.json`, canonical JSON: for every case and every
`via` action, the exit status, the build `id`, the digest of each file of `files`, and each
diagnostic record of `diagnostics.json` less `message` and `hint` (`diagnostics.md` 1.3), never the
digest of the whole file. `compare A B` checks two records agree on
every case without `one_system`. A system with no record makes U7 pending for every case, with the reason; Windows stays so until hosted
runners are allowed (`docs/plan.md`, "Comes back to you later").

## 6. Running

- **Location and language.** The runner is Python, using the standard library and numpy only (this
  is `build.md`'s "read with numpy or the standard library, never through the SDK"). BLAKE3, which
  neither has (`build.md` 3.3), is the runner's own pure-Python copy of the reference algorithm,
  `via_conformance/blake3.py`, used on small files only (a manifest, a blob a raw worker writes). It lives in
  `tools/conformance/` (R33). It is not under `spec/`, because it is a tool, not part of the
  contract.
- **Command.** `run --via PATH --milestone N [--chapter C] [--case ID] [--jobs N] [--record DIR]`,
  and `compare A B`. Cases run in bytewise order of chapter, then case id; `--jobs` runs cases side
  by side, never the actions of one case.
- **Outcomes.** Each case is `pass`, `fail` (a check did not hold; the message names the action, the
  check's id and both values), `pending` (with its milestone or reason), or `error` (the case itself
  is malformed: an unknown key, a `find` not found once, an unknown named check, an action id not
  listed). The runner exits 0 when no case is `fail` or `error`, 1 otherwise.
- **Kept on failure.** A failed case's working folder is kept and its path printed; a passing one is
  deleted.

## 7. Workspace checks on the cases

Repository tests, not conformance (as `diagnostics.md` and `build.md` separate them):

- Every `case.toml` parses with no case error, pending ones included.
- Every row of every chapter's Conformance table, and every numbered item of a Conformance list, has
  a case folder, and its `milestone` equals the row's. Rows without one are listed as missing.
- `diagnostics.md`'s coverage check reads the codes of every `diag` table: each registered code up
  to the current milestone with `covered_by = "conformance"` appears in some case at or below it.
- No case copies from `spike/` and no case file names a path outside the repository.

## 8. What stays open elsewhere

The case form does not depend on these; each is an ordinary `via` action once its owner decides it.

| Open | Owner, decided by |
|---|---|
| The test switch that stops a build after its first rename (`build.md` case 24) | `build.md`, with the case. |
