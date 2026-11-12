# Diagnostics

A diagnostic is what the engine says when something in a world is wrong, doubtful or worth knowing:
one record with a severity, a stable code, the thing at fault, where its text is, and what to
change. This chapter fixes that record, the registry of codes, how diagnostics are counted, grouped,
ordered and written, and what a build does when one is an error (principle 8: the first reader is
an agent).

Authority: `docs/doctrine.md`. Words: `spec/vocabulary.md` (section 3, Checks); where the two differ
on the record's fields, this chapter's section 1 is the full list. Spans: `spec/documents.md` (2.1,
2.2). Number formatting: `spec/numbers.md`. Produced names: `spec/identity.md`. Nodes and
dependencies: `spec/graph.md`. How a step sends a diagnostic: `spec/protocol.md` (3.3). Where
`diagnostics.json` lives and `--fail-on`: `spec/build.md` (11.1, 13, 14). Each chapter's own codes
are in that chapter's Diagnostics table.

## 1. The record

| Field | Takes | Required | Meaning |
|---|---|---|---|
| `severity` | `error`, `warning`, `info` | yes | Section 3. Fixed per code (section 2). |
| `code` | string | yes | Section 2. |
| `subject` | string | yes | The thing at fault, as an address (1.1). |
| `message` | string | yes | What is wrong, in one sentence, with its numbers (1.3). |
| `hint` | string | yes | What to change (1.3). |
| `file`, `line`, `column` | string, integer, integer | one location (1.2) | The text an author would edit to fix it. |
| `bytes` | 2 integers | with `line` | The byte range `[start, end)` of that text in `file`. |
| `argument` | integer | one location (1.2) | For a command-line fault: the index of the argument at fault (7). |
| `chain` | list | no | The includes, layers, changes and defaults the value came through (1.2). |
| `position` | `[x, y]` or `[x, y, z]` | where one exists | Where in the world, in the top world's frame (1.2). |
| `related` | list | when the code names two things | Other things involved: each a `subject`, a location, `position` if any, and a `note`. |
| `data` | table | as the registry declares | The code's facts as values: `nearest`, a limit, a measure. Agents act on these, not on `message`. |
| `count`, `occurrences` | integer, list | aggregated only | Section 5. |
| `failed_on` | `true` | with `--fail-on` only | Section 3. |

**The rule.** A diagnostic cannot be built without a code, a subject, a site and a hint. The engine
(`via-diag`) builds it in two steps, because a node's diagnostics are stored with its value and
replayed, and a stored diagnostic holds no file or line (`graph.md` 3.4):

1. **Construction.** A node builds `Diag { code, subject, site, message, hint, ... }`. The code is a
   generated constant (section 2); the subject is an address with an optional key path (1.1); the
   site is one of the values below, none of which is empty. The one constructor takes all four, so
   a diagnostic without them does not compile.
2. **Emission.** When the build emits the record, the site is resolved against the current merge to
   one location (1.2) and a `chain`. A site that does not resolve (its key has no span in this
   merge) is a defect of ours: the record is replaced by `diag.internal` naming the code and the
   site.

| Site | Resolves to |
|---|---|
| `Key(path)` | The span of that key under the subject (1.2). |
| `Expr(path, range)` | The span of a sub-expression of a formula string, mapped to the file (`documents.md` 2.1). |
| `Header` | The span that creates the subject's entity (1.2). |
| `Selector(path)` | The span of the key that wrote the selector. |
| `File(file, locator)` | The `file` form: a non-text file and the `data` that locates inside it. |
| `Argument(n)` | The `argument` form. |
| `World` | The world file at line 1, column 1, bytes `[0, 0]`. |

A step's diagnostic is checked on arrival (section 6). The conformance runner adds one check to
every case (Conformance).

### 1.1 Subject

The subject is one canonical form: an entity address (`documents.md` 2.2), `<collection>.<id>`,
with an optional key path below it. Every other way of naming a thing is rewritten to it at
construction or on arrival.

| Thing | Subject | Example |
|---|---|---|
| An entity | Its address | `feature.ford`, `kind.lane`, `let.slope`, `field.height`, `step.lints` |
| One key of it | The address and the key path | `scatter.trees.density`, `kind.lane.effect.bed.mask` |
| One element of a list | The key path with a 0-based index | `feature.beck.path[2]` |
| A produced thing | Its producer's address; the produced name goes in `occurrences` (5) | `scatter.trees` |
| A selection | The selector | `kind:district` |
| The world as a whole | `world` | `world` |
| A command-line fault | `cli` | `cli` |

- A bare id maps to its entity address: ids share one namespace per world (`vocabulary.md` 2.1), so
  `ford` is `feature.ford` with no doubt.
- A place name (`lane_west.end`, a mount, a set item) maps to the entity its first part names; the
  rest of the place name goes in `data.place`.
- A produced name (`trees#s7000_12_n3`) maps to its producer (`scatter.trees`) and the name goes in
  `occurrences`, so that aggregation (5) works.
- A subject inside an import is prefixed with the import's id (`a.feature.quay_road`; M4).

A **node's subject** is the address of the statement that made it (`graph.md` 1, "Made from"):
`rule:trees` has `scatter.trees`, `step:lints/i:north` has `step.lints`. A node made from no one
statement (`merge`, `source`, `input`, `select`, `op`, `write`) has none.

### 1.2 Location

A location is one of three forms. Every diagnostic has exactly one.

| Form | Fields | Used for |
|---|---|---|
| span | `file`, `line`, `column`, `bytes` | Anything that came from text: every document, kinds file, pack, change, and a step's returned `document.toml`. |
| file | `file` alone, and `data` locating inside it (a sample, a byte offset, a node of a model) | A file that is not text: a raster, a model. |
| argument | `argument` | A command-line fault, before the world file is known. |

**Files.** `file` is written so that it does not depend on where anything is installed (R26).
`documents.md` 2.1 owns the forms; `build.md` 3.4 uses the same ones. In summary:

| Where the file is | Written as | Example |
|---|---|---|
| Under the world file's folder | the path relative to that folder | `layers/roads.toml`, `../shared/npc.toml` |
| In a library, whichever root found it (`--lib` or the shipped root) | `lib:<library id>/` and the path inside the library | `lib:ways/kinds.toml` |
| A change file outside the world's folder | `change:` and its file name | `change:downstream.toml` |
| In the pack's root (`assets.md` 2) | `pack:` and the path inside the root | `pack:models/cottage.glb` |
| A step's returned document | `step:<id>/document.toml`, or `step:<id>/<item>/document.toml` | `step:infill/document.toml` |
| Any other outside file (a field's `file`, a step input) | `input:` and its file name | `input:dem.tif` |

A library file is named by its library id, never by which root found it or by the order of `--lib`
roots, so the bytes are the same on every machine. Ids hold no `:` and a relative path never starts
with a tag, so the forms never mix. An absolute path is never written. Where this summary and
`documents.md` 2.1 differ, `documents.md` wins.

**Spans.** `line` and `column` count from 1, columns in Unicode scalar values (`documents.md` 2.1).
The span is the text an author would edit:

- the key whose value is at fault, if there is one; inside a formula, the faulty sub-expression;
- a missing key: the span that creates the table that should hold it; a missing top-level key, or a
  fault of the world with no key: the world file at 1:1, bytes `[0, 0]`;
- otherwise the span that creates the entity: its `[header]`, or the dotted key or inline table that
  creates it. Where an entity is written in more than one file, the first in reading order
  (`documents.md` 8: the base document before changes).

A fault in derived output has the span of its cause: the producer's table, the kind effect, the
step's table. `position` then says where in the world.

**Chain.** Where the value at fault was written over, or supplied, the span is the last writer's
text, and `chain` lists how the value got there, in reading order, the last writer last. Each entry
is `{ through, name, file, line }`; a `layer` entry also has `owner`, the layer's `owner` (string),
so the record says who answers for the text (`documents.md` 5). Every diagnostic whose value was
written in a layer file carries that layer's entry, even when nothing wrote over it. The world
file's own text is the layer `world`, which has no owner and no entry.

| `through` | `name` holds | `file`, `line` |
|---|---|---|
| `include` | The library's id | The `include` key that named it. |
| `layer` | The layer's id | Where the layer wrote the value. |
| `change` | The change's name | Where the change wrote the value. |
| `defaults` | The address of the feature that took the default | The feature's header. |

Imports add `import` (M4). A value written in layer `roads` and written over by the change
`downstream`:

```json
"chain": [{"file":"layers/roads.toml","line":9,"name":"roads","owner":"roads team","through":"layer"},
          {"file":"world.toml","line":60,"name":"downstream","through":"change"}]
```

**Position.** Any fault that has a place in the world gives `position`. A message with a place and
no `position` is a defect (TODO lesson: a lint finding with no location).

- `x` and `y` are held as integer millimetres (R24). `z`, where given, is a height: a double, not on
  the lattice (`numbers.md` 3.1).
- In JSON, `x` and `y` are written as lattice values in metres (`numbers.md` 3.4: `64`, not `64.0`;
  `181.5`); `z` as the shortest double that reads back.
- A step sends f64 metres (`protocol.md` 3.3). They enter the lattice as `numbers.md` 3.1 says for a
  computed position: millimetres rounded to the nearest integer, ties to even, once. A non-finite
  value, or one outside the world's extent, is `diag.malformed` (6).

### 1.3 Message and hint

- The message says what is wrong, as fact, with the numbers that show it: the value, the limit, the
  two clashing values. Numbers are written in the form `numbers.md` 3.4 fixes.
- The hint says what to change: a key to write or remove, a value, a name, or a command (`via lock`).
  It names the thing to edit. "Check your input" is not a hint. An info's hint says what to write
  to get another outcome.
- Neither holds a file, line, column or byte offset. They are stored with the node's value and
  replayed (`graph.md` 3.4), and text that moved would make them stale. Another place goes in
  `related`, which is resolved at emission.
- Where a name was not found, the hint gives the nearest name, `data.nearest` holds it (1.4), and
  `related` points at its declaration.
- Messages and hints are English. Their wording is not compared by conformance; code, severity,
  subject, location, position, `count` and `data` are.

### 1.4 Nearest name

Every chapter that says "the nearest name" means this: among the names visible at that place, the
one with the least optimal-string-alignment distance (Damerau, 1964; restricted form as in Boytsov,
2011) to the written name, if that distance is at most `max(1, floor(length / 3))` of the written
name. Ties go to the bytewise-first name. If none is close enough, `data.nearest` is absent and the
hint lists up to five names visible there, bytewise first. The threshold is ours.

## 2. Codes and the registry

- A code is `group.name`, or `step.<id>.<name>` for a step's own code. Each part matches
  `[a-z][a-z0-9_]*`. A code never changes meaning once released; a retired code is never reused.
- One code has one severity. A condition that is an error in one case and a warning in another is
  two codes (the spike's `water.dry` was both). `--fail-on` does not change it (section 3).
- One condition has one code. Two codes for one condition is a registry defect. Where two chapters
  touch one condition, the chapter that defines the thing keeps the code: ids are `identity.bad_id`
  and `identity.duplicate_id`; a non-finite number in text is `num.not_finite`, a formula's
  non-finite result `formula.not_finite`, and any other stored one `num.not_finite_result`
  (`numbers.md` 8); a world or scatter seed of the wrong form is `num.seed_type`, a noise call's
  seed that is not a name `formula.seed`.
- Each group is owned by one chapter. A chapter may own several groups.
- A code whose name is `internal` marks a defect of ours, not of the text (for example
  `geom.internal`). Section 7 gives its exit status.
- A step's own codes are `step.<id>.<name>`. The step sends `<name>` as `code`; the engine adds the prefix
  (section 6). The group `step` holds no other codes.

**Registered** means one of two things:

- a core code: in `spec/diagnostics.toml`, with the severity written there;
- a step code: well formed, `<id>` a step of this world, and one severity for every record of that
  name in the build. Its `explain` text comes with it (section 6).

**The registry** is one file, `spec/diagnostics.toml`, flat TOML keyed by code (R1):

```toml
[formula.unknown_name]
severity  = "error"
chapter   = "formulas"
milestone = 1
summary   = "A name is not visible here."
data      = ["name", "nearest"]
explain   = """
A formula read a name that is not a field, a let name or an attribute of its owner.
Write the name as declared, or declare it in [let].
Bad:   value = "= 2 * hieght"
Fixed: value = "= 2 * height"
"""
```

| Key | Meaning |
|---|---|
| `severity` | The code's one severity. |
| `chapter` | The chapter whose table holds the code's row. |
| `milestone` | The milestone that first raises it (`docs/plan.md`). |
| `summary` | One line: the chapter table's text. |
| `data` | The names of the code's `data` fields. |
| `worst` | The `data` field that ranks occurrences, largest first (section 5). Optional. |
| `covered_by` | `conformance` (default), `reader` (raised by a loader, covered by its tests), or `workspace` (a named workspace test). |
| `explain` | What it means, what to change, one bad and one fixed example. |

The registry's text obeys the deny-list (`vocabulary.md` 7): it is compiled into `via-diag`, and
`via explain` output is core output. Examples use neutral names (`height`, `value`, `a`, `b`).

The registry generates three things, and nothing else lists codes:

1. The code constants in `via-diag`, so the engine cannot raise an unregistered core code.
2. `via explain CODE`: prints the code, severity, chapter, summary and `explain`. With `--json`,
   the registry row as JSON. `via explain` with no code lists every code by group.
3. The reference page of every code, generated into `docs/reference/` (R33), and the SDK's list of
   core codes.

Each chapter's Diagnostics table is the source the registry is written from, and the only one: every
key but `explain` is fixed by the code's row and the rules below, so two registries written from the
same chapters are the same. A table starts with columns `Code | Severity`; the summary is the
column headed `Says`, `Meaning` or `Raised when`. `Subject`, `Kind` and `Hint` columns are allowed;
only `data` reads `Hint`.

| Key | Taken from the row |
|---|---|
| `milestone` | The `M` or `From` column (`1`, `M1`). Without one, a `(Mn)` in the summary. Without either, 1. |
| `data` | Every `data.<name>` the summary and hint name, in order, then `nearest` if they give a nearest name (1.4). None: no `data`. |
| `worst` | The summary's `worst first by data.<name>`, a field of `data`. None: no `worst`. |
| `covered_by` | Not from the row: `reader` for `load.*`; `workspace` for `internal` codes and those the workspace checks below name; else `conformance`. |

A code whose record carries a field its row does not name is a registry defect.

`via explain step.<id>.<name> WORLD` prints the `explain` text the step sent with that name in the
last build of `WORLD` (`protocol.md` 3.3); it runs nothing and does not merge the world, so a
document error does not change its answer.

An unknown code is `cli.unknown_code` (exit 2), with the nearest code. The search covers the core
codes, and, when `WORLD` is given, the step codes recorded in its last build.

The group `place` means placement (`rules.md`). It is not the "place name" of `vocabulary.md` 2.1.

## 3. Severity

| Severity | When | Effect on the build |
|---|---|---|
| `error` | The text cannot be read, names nothing, or contradicts itself. Two firm decisions clash (principle 3). A stated fact fails its check after a simulation (R14). A step fails or sends an error. A defect of ours. | No outputs (section 4). Exit status 1 or 3. |
| `warning` | The build is made, but something gave way in a way the author may not expect, a statement held only in part, or text did nothing (a clear that removed nothing, a dropped piece, R18). | Outputs written, unless failed by `--fail-on`. |
| `info` | A fact: a count, a measure (R15), what was merged, split or skipped and why. Never asks for action. | Outputs written. |

- A world cannot change a code's severity. A check that counts warnings reads `diagnostics.json`.
- Something left out of a build is never silent (R18). A piece dropped where the text did not ask
  for it (a region condition keeping only its largest piece) is a warning. A drop the text asked
  for (a clear, a pin replacing produced instances) or a derived rule giving way (`place.gave_way`,
  `rules.md` 7.5) is an info.
- **`--fail-on CODE`** (`build.md` 14) fails the build on a warning code, for one run. The record
  keeps its registered severity and gains `"failed_on": true`. It counts as an error for the exit
  status and for writing (section 4), but it does not fail its node or make any node `skipped`, so the
  graph and the store are the same with or without the flag. It applies to warning records only. A
  registered code that is not a warning is `cli.usage`, and a name that is not a code is
  `cli.unknown_code` (both exit 2). A step code is accepted as written and applies if a warning of
  that name arrives.
- Waivers, a reason written in the text that quiets one warning at one place: (open), decided in
  `protocol.md` with the lint interface (TODO: lint rules as plugins).

## 4. A build with an error

A build with any error writes no outputs. Decided here: a loader, a sheet or an agent that reads a
half-made build takes a wrong world for the current one, and a warning-only check is skipped in
practice (TODO lesson: budgets).

| Stage | What it checks | An error there |
|---|---|---|
| 1. Reading | `documents.md` 8, steps 1 to 7: syntax, keys, includes, layers, changes, the merge, place names, direction. | Each step reports all it finds; the build stops after the first step with an error (`documents.md` 8). |
| 2. Static checks | Each formula's names, types and rates (`formulas.md`, "before anything runs"); asset ids and taxa against the pack (`assets.md`). | Fails the node of the statement that holds it. The build goes on. |
| 3. Graph construction | Cycles (`graph.md` 2.1). | Every member of a cycle fails, as one group, under one `graph.cycle`. The build goes on. |
| 4. Graph run | Fields, shapes, placement, rules and their checks, authority clashes, generate and lint steps. | The node that raises it fails. The build goes on. |
| 5. Writing | Export steps (R22) and the build files. | Runs only when stages 1 to 4 raised no error and no `failed_on` record. |

**Skipped nodes.** In stages 2 to 4, every node that depends on a failed node, directly or through
others, is `skipped` (`graph.md` 3.4; `skipped` is the one word): it does not run and raises nothing
of its own. Every other node runs. A lint step is a node like any other (`graph.md` 4): a lint whose
reads reach a failed or skipped node is skipped and its findings are dropped; a lint that reads none still runs and reports. A lint
reads the finished build (R29), view `build`, whatever layer declares it (`graph.md` 4). In M1 a lint is a plain command
handed that whole view (`protocol.md` 2.2, `graph.md` 4), so a failed node in it skips every lint
that sees it. A lint that reads only part of the build, and so runs beside a failure, needs the
worker's traced reads (M2): when a read reaches a failed or skipped node, the call is cancelled and
the lint is skipped (`graph.md` 4). So one run
reports every independent error, and no error is reported again as its consequences. Results of
nodes that ran are kept by content (`graph.md`), so the next build does not run them again.

**`diag.skipped`.** For each failed node with skipped dependents, the engine raises one
`diag.skipped` (info). It is the one thing the engine says about skipped readers (`graph.md` 3.4:
readers say nothing of their own).

- Subject: the entity address of the failed node's first error in sort order (5), its key path
  dropped. Location: the span that creates that entity (1.2), or the world file at 1:1 for `world`.
- `count`: the distinct subjects (1.1) of the skipped nodes, the failed node's own excluded.
  Nodes with no subject are not counted. `data.first`: the first ten of them, bytewise.
- Members of a cycle are covered by `graph.cycle`, not by `diag.skipped`. Readers of a cycle outside
  it get one `diag.skipped` under the subject of the `graph.cycle`.

**Writing.** With an error or a `failed_on` record: `diagnostics.json` is written and nothing else
that a loader reads (`build.md` 13); no change report is written. Where the world declares export
steps, one `diag.skipped` with subject `world`, `data.stage = "export"`, says how many did not run.
Without an error: every output, `diagnostics.json` among them.

A warning never makes a node skipped.

## 5. Grouping, limits and order

These apply in this order: once, then aggregation, then order.

**Once.** A record whose canonical JSON equals one already raised in the build is dropped. A node
evaluated in two views (R21) reports once. (TODO lesson: a step's diagnostics printed twice.)

**Aggregation.** Records with a `position`, whose subject is a producer (a run, a scatter or a
step, with no key path), with no `related`, and that agree on code, subject, location, `chain`
(which follows from the location) and `failed_on`, are one record. So 166
instances one step placed on steep ground are one warning with `count = 166`, not 166 (TODO:
`townland`). Records without a position, with a key path, or with `related` are never folded, so
two clashes or two cycles stay two records.

- `count` is the number of occurrences after "once", 1 when not aggregated (and then omitted). Two
  occurrences at one position with different names count twice.
- `occurrences` holds at most five, each `{ name, position, value }`: the produced name if there is
  one, its position, and the value of the code's `worst` field if the code has one. With `worst`,
  the five with the largest value; otherwise the first five by position. Position order is `y`,
  then `x`, then `z`, smallest first; ties by `name` bytewise.
- The record's `position`, `message` and `data` are those of the first occurrence. The others'
  `data` is not kept beyond `value`.
- Hand-written things have their own subjects, so they are never folded together: ten hand
  instances each on steep ground are ten warnings.

**Order.** Records are sorted, never listed in the order raised, since nodes run in parallel. Keys,
in turn:

1. severity: error, warning, info;
2. location form: argument, file, span;
3. `argument`; or `file` bytewise, then `line`, then `column`;
4. code bytewise, then subject bytewise;
5. position: absent before present; then `y`, `x`, `z`;
6. the record's canonical JSON bytes.

The order is total. Given the same text and the same step results (replayed from the store as
`graph.md` 3.4 does), the build writes the same `diagnostics.json`, byte for byte, at any thread
count, clean or incremental (R26). A step that gives different results for the same reads is
recorded as `graph.step_unstable`, not promised (R26: a user's step is recorded); that is a run
diagnostic (section 7), so it is never in the file. The file holds no times and no absolute paths.

**Limits.** `diagnostics.json` holds every record; aggregation bounds it by the size of the text,
not of the world. The terminal prints at most ten records per code, then one line
`... and N more <code>`; and infos only as a count unless `--info` is given.

## 6. Step diagnostics

A step sends diagnostics by `diag.emit` (`protocol.md` 3.3). Each holds `severity`, `code` (the `<name>` part of `step.<id>.<name>`),
`subject`, `message`, `hint`, and optionally `position` (f64 metres), `related`, `data`, and
`explain`, the text `via explain` prints for that code (`protocol.md` 3.3: sent with the first
`diag.emit` of a code in each call, so each call of a `for_each` step sends it;
`protocol.no_explain` and `protocol.explain_differs` are that chapter's).

- The engine records the code `step.<id>.<name>`, where `<id>` is the step's id. Since the prefix is
  always added, a step can never raise a core code or another step's.
- The subject is an entity address, key path, bare id, place name, produced name or selector, mapped
  to the canonical form by 1.1. A produced name becomes its producer, with the name moved into
  `occurrences`. A step never writes file or line: the site is the subject's key path if it has
  one, else its header; for a selector, the key in the step's table that wrote it (`for_each`), else
  the step's header.
- `position` enters the lattice as 1.2 says.
- A step that runs once per item (`for_each`) gives the item as subject; aggregation then works per
  item.
- A step that crashes, times out or answers out of protocol raises `protocol.md`'s codes, with the
  step as subject and its table as location.
- A lint step that finds nothing is not silent: the summary (section 7) lists every lint step with
  its counts, zeros included (TODO lesson: a passing lint step is silent).

**`diag.malformed`.** A step's diagnostic is replaced by one `diag.malformed` (error), subject the
step, location the step's header, with `data.message` (the step's message) and `data.missing`
(what was wrong), when any of these holds:

- the hint is missing, empty or only whitespace; the message is missing, empty or only whitespace;
- the severity is not `error`, `warning` or `info`;
- the subject, or a `related` subject, resolves to nothing in this world;
- the `code` does not match `[a-z][a-z0-9_]*`;
- `data` is not a table, or a value in it is not a string, number, boolean or list of those;
- `position` is not finite or lies outside the world's extent.

When one `code` arrives with two severities in one build, every record of that name is replaced by
one `diag.malformed`. Principle 8 holds for our libraries too (R19b).

## 7. Exit status and output

| Exit status | When |
|---|---|
| 0 | No error and no `failed_on` record. |
| 1 | At least one error or `failed_on` record, and no `internal` error. |
| 2 | The command line is wrong (`cli.*`). No build is attempted. |
| 3 | At least one `internal` error: a defect of ours. |

**Arguments.** `argument` counts from 1, after the program name: in `via build --frobnicate`,
`build` is 1 and `--frobnicate` is 2. `--json` is honoured wherever it appears among the arguments,
so `via build --frobnicate --json` prints the `cli.usage` record as JSON. A misspelt `--json` is
itself `cli.usage`, printed as text. Status 2 never reads, writes or replaces a build directory.

**`via explain`.** Exit 0 when it prints. An unknown code is `cli.unknown_code`, a missing world file
`cli.no_world`; both exit 2. It does not merge the world (section 2), so document errors leave it
at 0.

**Defects of ours.**

- An engine panic is caught and becomes `diag.internal`. Its subject is the subject of the node
  being run, with that node's site; with no node, `world` at the world file's 1:1; before the world
  file is known, `cli` and the argument being read.
- A panicked node fails like any other: its dependents are `skipped` and independent nodes still run.
- Every build profile sets `panic = "unwind"`. The panic hook is silent: it prints nothing, and its
  text goes into `data.message`.
- Documents and formulas have depth limits, `doc.too_deep` and `formula.too_deep`, so deep nesting
  never overflows the stack.
- The one exception: running out of memory ends the process with no diagnostic.
- The inputs of the failing operation are written for a reproduction to `<directory>.repro/<digest>/`,
  beside the build directory, where `<digest>` is the failing node's key. `data.repro` holds
  `<digest>`, never a path. That folder is not part of the build: no loader reads it, and `via build`
  never deletes it (`build.md` 13). Its contents: `geometry.md` 7.4.

**The terminal** (standard error) prints each record as:

```
error[formula.unknown_name] scatter.trees.density: no field, let or attribute named `xx`
  --> changes/bad_typo.toml:2:67
  hint: write `x`
```

with `related`, `chain`, `position`, `count` and `occurrences` on further indented lines when present. The
layout follows the Rust compiler's (rustc). Last comes the summary: one line per lint step, then
`E errors, W warnings, I infos`, with `(N failed by --fail-on)` when there are any. Control
characters in user text are escaped. Colour is off when `NO_COLOR` is set or standard error is not
a terminal. Conformance compares no terminal output but the summary line.

**World and run diagnostics** (`build.md` 11.1). A world diagnostic is about the text: every node's
(`graph.md` 3.4), the reading steps' and the checks'. A run diagnostic is about this run of the
engine, not the world; the codes are those `build.md` 11.1 lists as run codes, the one list
(`identity.no_previous`, `identity.reselected`, five `graph.*` codes, most `build.*` codes). Run
diagnostics are never in `diagnostics.json`, so the same text gives the
same file.

**JSON for agents.** `diagnostics.json` and `via build --json` (to standard output, which then holds
nothing else) have one form, in the canonical JSON form of `documents.md` section 8, and differ in
what they hold:

| Document | Holds | `status` |
|---|---|---|
| `diagnostics.json` | World diagnostics only. | The exit status the world diagnostics alone give. |
| `via build --json` | World and run diagnostics, merged in the order of section 5. | The process's exit status. |


```json
{"counts":{"error":1,"failed_on":0,"info":1,"warning":0},"diagnostics":[...],"format":"via-diagnostics-1","lints":[],"status":1}
```

`format` names this chapter's version. `status` is as the table says. `counts` counts the records
the document holds, not
occurrences; a `failed_on` record counts under `warning` and under `failed_on`. `lints` lists every
lint step: `{ step, ran, error, warning, info }`, counting its records (its own `step.<id>.*` codes
and every record whose subject is the step, its `diag.malformed` and `protocol.*` ones included); a skipped lint has
`ran = false`. With exit status 2 there is no build directory, and the document goes to standard
output (`--json`) and to the `--diagnostics` file only (`build.md` 14).

## 8. Worked examples

`ford` as written in `docs/plan.md`, with `changes/bad_typo.toml`, taken to be exactly the two
lines of its body:

```toml
[scatter.trees]
density = "= smoothstep(0.1, 0.5, fbm(60, 3)) * smoothstep(0, 20, xx)"
```

In `examples/ford`, `via build world.toml --change changes/bad_typo.toml --json` exits 1 and
prints, among others:

```json
{
  "bytes": [82, 84],
  "chain": [{"file": "changes/bad_typo.toml", "line": 2, "name": "bad_typo", "through": "change"}],
  "code": "formula.unknown_name", "column": 67,
  "data": {"name": "xx", "nearest": "x"},
  "file": "changes/bad_typo.toml", "hint": "write `x`", "line": 2,
  "message": "no field, let or attribute named `xx`",
  "severity": "error", "subject": "scatter.trees.density"
}
```

`x` is a built-in, so there is no `related`. The static check fails `rule:trees` (section 4,
stage 2). The lint step `checks` reads the trees, so it is skipped. One `diag.skipped` follows:
subject `scatter.trees`, `world.toml` line 64, column 1, bytes `[1711, 1726]` (its header in the
base text, section 1.2), `count` 1, `data.first = ["step.checks"]`. So the lint does not also
complain that no trees stand. From M2, a lint that reads only the lanes still runs and reports. No
instance file is written.

Not ford, and M2: a generate step `infill` returns 166 instances on ground steeper than their asset allows. Placing a
step's instances is checked by `rules.md` 8.2, so they give one warning, not 166:

```json
{
  "bytes": [2210, 2223], "code": "place.relief", "column": 1, "count": 166,
  "data": {"limit": 0.6, "relief": 2.4}, "file": "world.toml", "line": 77,
  "hint": "move them, give their asset `ground` with a kind that levels, or choose an asset with a larger relief limit",
  "message": "166 instances stand on ground that varies by more than their asset allows; worst 2.4 m against 0.6 m",
  "occurrences": [{"name": "infill#h40", "position": [181.5, 64], "value": 2.4},
                  {"name": "infill#h41", "position": [190, 64], "value": 2.1}],
  "position": [181.5, 64], "severity": "warning", "subject": "step.infill"
}
```

`occurrences` is shortened here to two of its five entries. Positions, names and numbers in this
second example are illustrative.

## Diagnostics

| Code | Severity | Says |
|---|---|---|
| `diag.skipped` | info | A failed node's dependents were not built, or export steps did not run; `count`, `data.first` (the first ten by address), `data.stage`. |
| `diag.malformed` | error | A step's diagnostic was malformed (section 6); `data.message`, `data.missing`. |
| `diag.internal` | error | The engine stopped on a defect of ours; the operation; `data.message` (the panic's text), `data.repro` (the reproduction's digest). Exit 3. |
| `cli.usage` | error | An unknown flag, a missing argument or a bad value; the argument and the nearest flag. Exit 2. |
| `cli.no_world` | error | `via explain` only: the world file given does not exist or is not a file. Exit 2. Every other command reports `doc.file_missing` (`documents.md`). |
| `cli.unknown_code` | error | `via explain` or `--fail-on` was given an unknown code; the nearest code. Exit 2. |

### Code groups of M1

Each group's codes are in its chapter's Diagnostics table.

| Group | Chapter | Covers |
|---|---|---|
| `num` | `numbers.md` | Numbers in text, the lattice, extent, resolution, seeds. |
| `doc` | `documents.md` | Syntax, keys, ids in documents, includes, layers, changes, merging. |
| `formula` | `formulas.md` | Formula syntax, names, types, rates, values. |
| `field` | `fields.md` | Roles, types, rows, raster files, effects cut at their radius. |
| `authority` | `authority.md` | Kinds, effects, clashes between firm statements. |
| `identity` | `identity.md` | Ids and places, produced names, pins, clears, locks. |
| `geom` | `geometry.md` | The kernel: shapes, networks, offsets, cells, masks. |
| `surface` | `surfaces.md` | Surfaces, bounds, mounts, where two surfaces meet. |
| `place`, `set`, `run`, `scatter` | `rules.md` | Placement, the three rules, overlap, relief, `keep_out`. |
| `asset`, `pack` | `assets.md` | Asset ids and facts; pack files, taxa, materials. |
| `graph` | `graph.md` | Cycles, missing inputs, the store, the change report. |
| `build` | `build.md` | The build directory and what is handed to the loader. |
| `check` | `build.md` | What `via check` finds wrong in a build directory: manifest, digests, array headers, names, canonical JSON. |
| `load` | `build.md` | What a loader (the Godot loader) finds wrong. `covered_by = "reader"` (section 2). |
| `protocol` | `protocol.md` | A step's run: start, crash, timeout, bad reply. |
| `diag`, `cli` | this chapter | Above. |
| `step` | none | A step's own codes, `step.<id>.<name>`. |

## Conformance

Each case runs through the command line (R20): input; action; what must hold.

- Every case of every chapter; build; each diagnostic has a registered code (section 2), a subject,
  a location and a non-empty hint. A core code's severity equals the registry's. A step code has one
  severity in the build. `failed_on` appears only on a code `--fail-on` named. The runner reads
  every record, exit 2 included, from the `--diagnostics` file (`build.md` 14, `conformance.md` 3.5).
- `ford` with `changes/bad_typo.toml`; build; exit 1, `formula.unknown_name` at `xx` (section 8),
  `data.nearest = "x"`, no `related` (a built-in has no declaration), no output file but
  `diagnostics.json`.
- `ford` with four independent bad edits: the cottage anchored in the beck, a formula typo, a cycle,
  and an unknown asset on another instance; build; exit 1, all four in one run:
  `authority.conflict`, `formula.unknown_name`, `graph.cycle`, `asset.unknown`. Each has code,
  subject, file, line and hint; the clash and the unknown asset have a position (plan, ford).
- (M2) A formula typo in a scatter that one lint step reads and another does not; build; one
  `diag.skipped` with `count` 1 naming the first lint; no record from it; the second lint's findings
  present; `lints` shows the first with `ran = false`.
- (M2) A generate step returning 166 instances on steep ground; build; one `place.relief`, subject the
  step, `count = 166`, five `occurrences`, largest relief first, names `<step>#<key>`.
- Ten hand instances on steep ground; build; ten `place.relief` records.
- (M6) A world with an export step and an error; build; exit 1, one `diag.skipped` with subject `world`
  and `data.stage = "export"`.
- The same world at 1 and 8 threads, clean and incremental; build; `diagnostics.json` byte-identical.
- `ford` with `changes/bad_anchor.toml`; insert one blank line above the table `related` points at,
  nothing else changed; build incrementally and clean; both `diagnostics.json` identical, `related` at the new
  line.
- A world with errors in three files; move tables within each file; build; with location fields
  removed, the records are the same multiset; each location names the moved text.
- The same world with its library given by `--lib` from two different folders; build;
  `diagnostics.json` byte-identical, library files written as `lib:<id>/...` (`documents.md` 2.1).
- A value set by a change file over a layer; build with an error on it; span is the change file's
  line, `chain` is the layer's entry then the change's.
- A warning code and `--fail-on` naming it; build; exit 1, the record a `warning` with
  `failed_on = true`, no outputs; without the flag, exit 0 and outputs written.
- A lint step that sends a diagnostic with no hint; build; one `diag.malformed`, exit 1.
- A lint step that sends `subject = "feature.nowhere"`; build; `diag.malformed`.
- A lint step that sends a produced name as subject; build; the subject is its producer and the name
  is in `occurrences`.
- A lint step that sends a `position` outside the world's extent; build; `diag.malformed`.
- A lint step that sends `code = "unknown_name"`; build; the code is `step.<id>.unknown_name`.
- A lint step that sends one `code` as warning and as error; build; one `diag.malformed`, no record
  of that code.
- A lint step that finds nothing; build; the summary and `lints` list it with zero counts.
- (M2) A generate step that reports one finding at two views (R21); build; one record.
- `via build --frobnicate`; exit 2, `cli.usage` with `argument = 2`; no build directory touched.
- `via explain formula.unknwn_name`; exit 2, `cli.unknown_code`, nearest `formula.unknown_name`.
- `via explain formula.unknown_name --json`; exit 0; the registry row.
- A fixed corpus of fuzz-found documents (`spec/conformance/diagnostics/fuzz/`); build each; exit 0
  or 1, never 3, never a crash.

**Workspace checks** (repository tests, not conformance):

- The registry and every chapter's Diagnostics table: same codes, same severities, same summary.
- Every registered code up to the current milestone with `covered_by = "conformance"` is raised by
  some conformance case. `reader` codes are covered by the loader's tests; `workspace` codes
  (`internal` codes, `build.dir_busy`, `graph.busy`, `graph.store_unavailable`) by a named test.
- An injected panic in a node, in a test build; exit 3, `diag.internal` naming the node, independent
  nodes still reported, nothing printed by the panic hook.
- Fuzzing documents and formulas (cargo-fuzz); new crashes join the fixed corpus above.
- The deny-list test reads the registry's generated strings like any other (`vocabulary.md` 7).

## Changed from the spike

| Spike | v1 | Why |
|---|---|---|
| `where` rounded to 0.1 m, optional | `position`, integer millimetres in the core; required where a place exists | `where` is a feature key (principle 9); principle 8; R24. |
| Location was a subject only; no file or line | Span with file, line, column, bytes and the chain | Principle 8. |
| `hint` optional | Required, for every severity | Principle 8. |
| A build written even with errors | No outputs with an error; `diagnostics.json` always | Section 4. |
| Order as raised | Sorted, total order; byte-identical across threads | R26. |
| A step's codes `<step id>:<code>`, unchecked, no explain text | `step.<id>.<name>`, checked on arrival, one severity per build, `explain` sent with it | One code form; principle 8. |
| A part's diagnostics lost positions and infos | Kept, subject prefixed by the import's id (M4) | Principle 8. |
| One code could be error or warning (`water.dry`) | One code, one severity | Registry rule. |
| Uncaught exception was exit 1 | Exit 3, `diag.internal` | An agent must tell its fault from ours. |
| `field.required` wrote nothing, not even diagnostics | `diagnostics.json` always written | Spike limitation, section 11. |
| Hundreds of identical warnings | One per producer with a count and the worst five | TODO: 166 relief warnings. |

## Open

- Waivers (a reason in the text that quiets one warning at one place): decided in `protocol.md` with
  the lint interface, by M3.
- Codes of a library's steps. `step.<id>.<name>` takes the id the user gave the step, so one
  library check has a different code in two worlds, and has no registry or reference page.
  Recommended: `lib.<library>.<name>`, declared in the library's own registry file in this TOML
  form, keeping `step.<id>.<name>` for a project's own steps; open to every library, ours or a
  user's (R19b). Decided when `protocol.md` fixes how a library ships a step, by M3.
- Machine-applicable edits in a record (a replacement for a span, as rustc's suggestions): decided
  by M1's fresh-agent test, if agents misapply hints.
- Five occurrences and ten printed records per code: decided by M1's fresh-agent test.
- A watch mode streaming records as JSON lines (blackbox, agents): after M1.
- This chapter answers the machine-readable part of the TODO "agent loop surface". Bookmarks and
  review renders go to the review library and `build.md` at M1 (the sheet tool).
