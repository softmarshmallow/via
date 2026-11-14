# The step protocol (via-step-1)

A step is the project's own code, run at a declared point with declared inputs. This chapter fixes
how a step is declared, how the engine runs it, what crosses between them and how, what a step may
return, and when a step is allowed to run at all.

Words are from `vocabulary.md`. Authority: `docs/doctrine.md`, chiefly R7 (as amended by R28), R19, R20,
R21 with R29 and R32, R22, R23 and R26. "(open)" marks what is undecided, with what decides it. The setup follows the reference in
`docs/references.md` ("Engine and SDK setup"): one engine binary, an SDK that never links it, user
code in its own process. Its two recorded failures, Windows process handling and bytes that differ
across systems, are what sections 6 and 4 are written against.

What arrives when:

| Milestone | Part of this chapter |
|---|---|
| M0 | A throwaway probe of 3.1, 4.1 and 6.2: a 6.5 MB field out and back, a call back, a killed worker (`plan.md`). It runs from a test harness outside the parser and is not the M2 implementation. |
| M1 | Declaring a step, plain commands (2), field files and columns (4), running a process (6), trust (7), and one lint step as a plain command through the SDK's launcher (8). Of section 5, only `diagnostics.json`: M1's only step is a lint, which returns no document (`fields.md`, Milestones). `worker = true` is `doc.not_yet`. |
| M2 | Returned documents (5), with `fields.md` 7; the worker (3) and its SDK side, with `derived.md` 4's operations; `for_each` (1.2); `checks` (`derived.md` 5); committed step results (7.1). |
| Later | The forward pointers of section 10. |

## 1. Declaring a step

A step is a table `[step.<id>]` in the layer it writes to (vocabulary, Steps). There is no `after`
key on a step.

| Key | Takes | Required | Meaning |
|---|---|---|---|
| `when` | string | yes | `generate`, `lint` or `export` (R22). No default. |
| `command` | list of strings | yes | The program and its arguments (section 6.1). Never a shell string. |
| `worker` | bool | no, `false` | `true`: the step speaks this protocol on stdio and may call the engine (section 3). `false`: a plain command (section 2). |
| `params` | table | no, `{}` | Handed to the step unread. Any keys (`documents.md` section 7). |
| `inputs` | list of strings | no, `[]` | Outside files or folders, relative to the declaring document. A folder is digested as the sorted list of its files (`graph.md` 5.1). A path that does not exist is `graph.input_missing` (`graph.md` 5.1). Their change re-runs the step. |
| `for_each` | selector | no | One call per selected item (1.2). The item reaches the step in its request. |
| `checks` | list of strings | no | On a lint step only: the generate steps whose stated facts it checks (`derived.md` 5). |

The word `worker` is kept (vocabulary left it open): it names what the process is, and a bool
cannot be misspelt into a third transport.

**Which layer.** A step declared in a layer file or the world file belongs to that layer. A step a
change adds belongs to the layer `documents.md` 6.1 gives an added entity: an inline change's
declaring layer, or `world` for a change file.

**What it reads.** A generate step reads its stratum, exactly as `graph.md` section 4 defines it:

- every statement of the layers at or below its layer;
- every output of a step in a layer below its layer;
- not the outputs of steps in its own layer or above, and not any statement that reads one of
  those, directly or through others.

So two steps in one layer do not see each other's outputs. A lint or an export step reads the
finished build, view `build` (R29, `graph.md` 4), whatever layer declares it. Whether it runs after
an error is `diagnostics.md` 4 (1.1).

**Libraries.** Steps are declared in a world or a layer only. A kinds file does not declare steps: a
step's layer decides what it reads and the rank of what it returns (R21, `documents.md` 5.1), and a
library has no layer. A library ships code; any element of `command` may name it in the form
`lib:<library>/<path>`, which the engine resolves in that library's folder as `documents.md`
section 4 finds it. Only a library named in `include` may be named this way
(`protocol.unknown_library`). The form is ours; `vocabulary.md` lists it (Selectors). A library
offers a lint by documenting the `[step]` table a world restates (decided by `lib/review`'s first
lint in M1, principle 10): a lint's place needs no layer, but any new way for a library to declare a
step would be a second mechanism for what three lines of world text already say, and the restated
table shows a reviewer what runs.

### 1.1 The three points

| `when` | Runs | Reads | Returns |
|---|---|---|---|
| `generate` | At its layer's place in the graph (`graph.md`), once per item under `for_each`. | Its stratum (above); a field in the view `fields.md` 6.2 gives. | A document (section 5), field files, diagnostics. |
| `lint` | After every generate step. A lint that reads a failed or skipped node is `skipped` (`diagnostics.md` 4); a plain lint is handed the whole build, so any failed node skips it (`graph.md` 4). | The finished build, view `build` (R29, `graph.md` 4). | Diagnostics. Other files it writes are kept in the store and named in the run report, not in the build directory (open, below). No document. |
| `export` | After lints, and only when the build has no error (`diagnostics.md` 4, stage 5). | The finished build, view `build`. | Files under `export/<step id>/` (`build.md` 11.4). No document. |

`effect` and `place` do not exist (R22): a generate step at a layer covers both.

### 1.2 `for_each` (M2)

`for_each = "kind:<name>"` on a generate step calls it once per feature of that kind in its
stratum, the **item**; any other form is `protocol.for_each_form`. Each call is its own graph node
with its own seed (`graph.md` 5.3, section 2), so moving one item re-runs its call alone. What the
calls return is joined in item order; keys must already differ between items (`identity.md` 2.5),
which a key built from the item's name does. A worker serves every call (3.2). A plain command is
handed its whole view on every call (2.2), so any change in it re-runs every call: per-item reuse
needs a worker. No match is `graph.for_each_empty`.

## 2. The request, and the plain command

One request, two transports (R22). The request is a JSON object. A plain command receives it as a
file; a worker receives it as the params of `step.call`. Its members:

| Member | Holds |
|---|---|
| `protocol` | `"via-step-1"`. |
| `step` | The step's id. |
| `when` | As declared. |
| `for_each_item` | Under `for_each`, the selected item's name; otherwise `null`. |
| `params` | The declared `params` as JSON (2.1). |
| `grid` | `extent_mm` (4 integers), `resolution_mm`, `nx`, `ny` (`numbers.md` section 4). The world frame's grid; a step in another frame is (open), decided with imports in M4. |
| `seed` | `stream(world seed, ["step", <id>])`, or `stream(world seed, ["step", <id>, <item>])` under `for_each` (`numbers.md` 7.2), as 16 lower-case hexadecimal digits. A test vector for each form is owed to `numbers.md` 10. |
| `out` | An absolute path to an empty folder the step writes into. |
| `objects` | An absolute path to the store's folder of objects (`graph.md` 3.2), where blobs live (section 4). |
| `checks` | A lint with `checks`: per named step, sorted by id, `{ "step", "facts" }` (`derived.md` 5); otherwise `null`. |
| `data` | Plain command only: what it may read (2.2). |

Paths in every message are absolute, UTF-8, with `/` as separator on every system. No path, and
neither `out` nor `objects`, enters a key (4.4).

### 2.1 Params

A number in `params` has no type, so it travels as its normalised text (`documents.md` 2): `_`
dropped, a leading `+` dropped, a hexadecimal, octal or binary integer written in decimal. That text
is a valid JSON number. An integer keeps every digit; the 2^53 limit of 4.3 is for columns, not
params. `inf`, `nan` and dates never reach a step: they are refused at read (`num.not_finite`,
`doc.type`).

| TOML | In `params` |
|---|---|
| `1_000.5` | `1000.5` |
| `+1.0` | `1.0` |
| `0xff`, `0o17`, `0b101` | `255`, `15`, `5` |
| `9007199254740993` | `9007199254740993` |
| `6.0`, `6` | `6.0`, `6` |
| `inf` | refused: `num.not_finite` |
| `1979-05-27` | refused: `doc.type` |

### 2.2 The plain command

A plain command is started with the request written to `<call folder>/request.json` and the
environment variable `VIA_STEP` set to that path (section 6.3 places the call folder). Its standard
input is the null device. It makes no engine calls. It reads only what `data` hands it and its
declared `inputs`, by path:

| `data` member | JSON form |
|---|---|
| `fields` | An object keyed by field name, one entry per field in its view. Each entry: `type` (`"scalar"` or `"weights"`), `role` (string or `null`), `tags` (strings, sorted bytewise), `unit` (string or `null`), `blob` (the field file's digest, 4.2), `path` (its absolute path, `<objects>/ab/<digest>`, 4.1), and `i0`, `j0`, `i1`, `j1` (integers: the whole grid, `0`, `0`, `nx`, `ny`). A `weights` entry adds `materials` (names, in id order, one per plane). A bounded field (M3) adds 3.3's `cover` and `posts`. |
| `features` | One feature list (4.3): an object keyed by column name, each member a column in 4.3's form. Every feature in its view, every attribute included (R19b: no private door). Footprint features are features: each is a region row named `<instance>.ground` (`rules.md` 2.1), with its kind, authority, layer and pieces, as a `kind:` selection holds it. |
| `instances` | One table (4.3): an object keyed by column name, holding 3.3's `build.instances` columns `name` to `tint`. A footprint is read from `features`, never from this table. |
| `inputs` | A list, sorted by `path` bytewise, of objects `{"path": <as written>, "absolute": <absolute path>, "digest": <64 hex>}`. |
| `pack` | (M2) `pack.query`'s table (3.3) of every asset the pack holds, every fact. |

Vector, a lint handed `height` over a 3 by 2 grid, no features, instances (so no footprint features) or inputs:
`{"features": {"authority": {"dtype": "str", "values": []}, "coords": {"dtype": "i32", "shape": [0, 2], "values": []}, …}, "fields": {"height": {"blob": "<digest>", "i0": 0, "i1": 3, "j0": 0, "j1": 2, "path": "/…/objects/ab/<digest>", "role": "surface", "tags": [], "type": "scalar", "unit": "m"}}, "inputs": [], "instances": {"asset": {"dtype": "str", "values": []}, …}}`.
The full bytes are owed to `spec/vectors/protocol/data/`, with case 18.

Its view is section 1's: its stratum for a generate step, the finished build for a lint or export. Every
blob in `data` is named by digest, so the request, less its absolute paths, names everything the
command is handed; that is why it is the command's static key and its trace is empty (4.4).

It writes into `out` (section 5) and exits. Exit status 0 is success; any other is
`protocol.failed` (6.2). Its standard output and error are both its log. A plain
command is handed everything in its view, so it runs again when anything in its view changes.
Reading less is what a worker is for.

## 3. The worker (M2)

### 3.1 Wire

- Messages are JSON-RPC 2.0 (jsonrpc.org, 2010 specification). Each message is one request, one
  response or one notification. JSON-RPC's batch arrays are not used: a batch in this chapter is
  always a batch of columns inside one call (R23).
- Each message is framed as in the Language Server Protocol's base protocol (Microsoft, LSP 3.17): a
  header `Content-Length: <n>\r\n\r\n`, then exactly `<n>` bytes of UTF-8 JSON. No other header is
  allowed. A frame over 16 MiB is `protocol.message_too_large`; data that large travels as blobs.
- Both sides open the protocol descriptors in binary mode. On Windows the SDK sets them so with
  `msvcrt.setmode`; text-mode translation of line ends is the failure this rule exists for.
- Numbers are parsed correctly rounded on both sides: the engine's JSON reader must round-trip every
  double (for `serde_json`, its `float_roundtrip` feature). The SDK writes JSON with
  `allow_nan=False`, so a non-finite number never reaches the wire as `NaN`.
- Each side numbers its own requests with integers from 1. A response is matched among the requests
  of the side that receives it. Calls from the worker may be in flight while the engine's
  `step.call` is pending; that is how a step calls back.
- **The protocol takes the standard descriptors before user code is imported.** A Python worker is
  started through the SDK's launcher, `command = ["python", "-m", "viasdk", "tools/soften.py"]`. The
  launcher duplicates descriptor 0 as the protocol's input and descriptor 1 as its output, marks
  both duplicates non-inheritable, points descriptor 0 and `sys.stdin` at the null device and
  descriptor 1 and `sys.stdout` at standard error, then imports the script. A `print` in user code
  reaches the log and never the wire; a subprocess the user starts (`cat`, with no `stdin=`
  argument) reads the null device and never a protocol frame. A frame that is not a valid header is
  `protocol.framing`, whose hint names this rule.
- The SDK reads the protocol on a thread of its own and ends the process when its input ends, which
  happens when the engine dies.
- Standard error is read continuously by the engine, so a chatty worker never blocks. It is the
  step's log (6.2).

### 3.2 Engine to worker

| Method | Kind | Params | Result |
|---|---|---|---|
| `initialize` | request | `protocol` (`"via-step-1"`), `engine` (`name`, `version`), `objects`, `capabilities` (`methods`: the worker-to-engine methods offered) | `protocol`, `sdk` (`name`, `version`), `capabilities` (`when`: the points it can dispatch) |
| `step.call` | request | The request (section 2), without `data` | `document` (TOML text, or `{ "blob": <digest> }` for a large one, or `null` when the step wrote `out/document.toml` itself), or `null` for lint and export |
| `$/cancelRequest` | notification | `id` of the `step.call` to stop | none |
| `shutdown` | request | none | `null` |
| `quit` | notification | none | none; the worker exits with status 0 |

`$/cancelRequest` is LSP's name. `quit` is LSP's `exit` under our own name, since `exit` is a core
word with one meaning (R9).

- **Version check is strict.** If the worker's `protocol` is not exactly `"via-step-1"`, the engine
  sends `quit`, stops the worker and reports `protocol.version`, naming both versions. A worker that
  receives another version answers `initialize` with error -32001 and exits. There is no
  negotiation; capabilities describe what is offered, never versions.
- `capabilities.when` lists the points the worker program can dispatch, not the functions one
  script defines: `viasdk` and `via-step` dispatch all three and always list all three. A worker
  whose list lacks the step's `when` is `protocol.when_unserved`, reported at `initialize`; only a
  worker program built for fewer points raises it. A script with no top-level function named after
  `when` (for `via-step`, a step whose `when()` lacks it) is `protocol.no_entry`, raised by the
  worker as error -32601 to `step.call` with `data.code` `"protocol.no_entry"`. One condition, one
  code: the program's reach is `when_unserved`, the script's entry is `no_entry`.
- A cancelled call is answered with error -32800 (as LSP's `RequestCancelled`). Its output is
  discarded.
- A worker takes one `step.call` at a time. It serves every call of its step in one build (every
  item of a `for_each`); workers do not outlive a build. A step must not carry state from one call to
  the next: an item's output would then depend on which items ran and which were reused. The core
  does not prevent it; `graph.step_unstable` reports it on a clean build. A fresh process per call is
  (open), decided by the first case that needs it.

### 3.3 Worker to engine

Every call is valid only while a `step.call` is pending, and carries `call`, that request's id.
Every call is a batch: columns in, columns out (R23). A loop that would make thousands of calls is
an engine operation instead, with no domain noun.

**Stratum.** The engine answers a generate step only from its stratum (section 1); a lint or export
worker, from the whole finished build (view `build`, R29). A read by name of anything
outside it (a feature of a higher layer, a feature another step of the same layer returns, a field
declared in a higher layer) is refused with `graph.read_outside`, naming the thing and the lowest
layer a step would need to stand in to read it (`graph.md` 4). On the wire it is error -32003 with
`data` holding `code`, `subject` and `layer`. A selector resolves within the stratum only and leaves
out what lies beyond it, without a diagnostic, as every selector does (`documents.md` 5.1). The
engine reports `graph.read_outside` itself when it refuses; if the step then ends by the exception,
that one diagnostic is the call's only report (principle 8: one fault, one message).

**Traced reads.** What the step sees of the build. Each is recorded in the call's trace (4.4).

| Method | Params | Result |
|---|---|---|
| `build.field` | `fields` (names, or `role:`/`tag:` selectors, each matching one field), `view` (`full` or `head`, default `full`; `authority.md` 4.4), `window` (`i0`, `j0`, `i1`, `j1`, `numbers.md` section 6, as an object or a list in that order; default the whole grid), `halo` (posts, default 0) | Keyed by each entry as asked: the field's `name`, its window grown by the halo and clipped to the grid, with its own `i0`, `j0`, `i1`, `j1`, as below. |
| `build.features` | `select` (a selector, a list of names, or `"*"`), `within` (a region feature's name, or a region in millimetres), `columns` (attribute names, or `"*"`) | A feature list (4.3) with `name`, `kind`, `authority`, `layer`, and one column per attribute asked. |
| `build.instances` | `select` (`"*"`, a producer's name, or an `@<taxon path>`), `within`, `columns` | A table: `name`, `asset`, `by` (the producer), `x`, `y` (i32 mm), `z` (f64 metres: heights are not on the lattice, `numbers.md` 3.1), `face` (i32 millidegrees), `scale`, `tint` (f64); footprints as a feature list when asked. |
| `input.open` | `path`, as written in `inputs` | A raw blob of the file's bytes. A path not declared is `protocol.input_undeclared`. |
| `pack.query` (M2) | `select` (`"*"`, asset ids, or an `@<taxon path>`), `columns` (fact names, or `"*"`) | A table by asset id: `id`, `source`, `taxon`, `tags`, and one column per resolved fact asked (`assets.md` 2.4): lengths as i32 millimetres, `bounds` `(n, 6)`, `origin` `(n, 3)`, `colour` f64 `(n, 3)`, words as str, `mounts`, `params` and `footprint` (a list of `[x, y]` millimetres in the asset frame: a table's column holds one value per row, so not a feature list) as canonical JSON text. Read-only (R19b; `assets.md` 5). |

`build.field` by field type:

| Type | Result |
|---|---|
| `scalar` | One field file (4.2): f32, 1 plane, the window's rows and columns. |
| `weights` | `materials`: the names of the materials some row painted in the window, in id order; and one field file of f32 with one plane per material, in that order (`fields.md` 6.3: sparse per window). Each value is the weight, 0 to 1. |
| bounded (R25, M3 and M5) | The value field file over the posts of the bound's box clipped to the window, the cover as a u8 field file (1 inside the bound, 0 outside), and `posts`, the box's `i0`, `j0`, `i1`, `j1` (`build.md` 6.3). |

A step that reads a field's full view and returns rows on it is `authority.step_view`; a step that
returns rows on a field reads it with `view = "head"`. Windows bound the cost of a read; they never
make it per-point: R19(c) reads "whole arrays, never per point" (R32).

**Computes.** Pure functions of their arguments, answered by the engine with its own maths
(`numbers.md` section 8), so a step gets the engine's bits (R26).

| Method | Params | Result |
|---|---|---|
| `expr.eval` | `formula`, and either `x`, `y` (i32 mm columns of one length n) or a `window`; `view` (`full` or `head`, default `full`, as `build.field` takes it: a field the formula names is read, and traced, in that view) | One column, f64 or bool (`formulas.md` section 11). For points: n values in the order given. For a window: `(i1 - i0) * (j1 - j0)` values in post order (`numbers.md` 4: rows from the south, each west to east). Values are doubles, not yet narrowed to a field's f32 (`fields.md` 1.1). |
| `derive.where`, `derive.route`, `derive.descend`, `derive.spill` | `derived.md` 4 | `derived.md` 4: shapes and levels found over a field, the text forms' bytes (R19b, R23). |

A compute that names a field, feature or selector reads it, and that read is traced as if made by
the read methods above (a field: a `build.field` read of its whole grid in the compute's view). The
compute's own call is traced as well, since its answer also depends on what is no read: a `let` its
formula names.

**Says.** Not traced; never change a result.

| Method | Kind | Params |
|---|---|---|
| `diag.emit` | request | Columns `severity`, `code` (the part after `step.<id>.`), `subject`, `message`, `hint`, and optionally `position` (f64 metres, `(n, 2)`), `related` and `data` (each a str column holding one canonical JSON text per row, `""` for none: `related` a list of `{"note", "subject"}`, `data` an object), `explain`. The members are `diagnostics.md` section 1's record. |

`explain` is how a step declares the text `via explain step.<id>.<code>` prints (the vocabulary left
this open): the first `diag.emit` of a code in a call carries it, the step result keeps it (4.4),
and `via explain` reads the latest stored result without running anything. A code sent without it
is `protocol.no_explain` (warning); two texts for one code is `protocol.explain_differs` (warning).

### 3.4 Errors on the wire

| Code | Meaning |
|---|---|
| -32700, -32600, -32601, -32602, -32603 | JSON-RPC 2.0's own: parse error, invalid request, unknown method, invalid params, internal error. `data` names the param at fault. |
| -32001 | Protocol version differs. |
| -32002 | A blob is missing, its bytes do not match its name, or its size does not match its message. |
| -32003 | `graph.read_outside`: the name lies outside the step's stratum. |
| -32004 | A non-finite number in a column (R26). |
| -32005 | A call outside a pending `step.call`, or for another call. |
| -32010 | The step raised an exception it did not catch. Sent by the SDK as the answer to `step.call`, with `data.exception` and `data.traceback`. Reported as `protocol.raised`; the engine writes the traceback to the log (6.2). The worker stays up for the next call. |
| -32800 | Cancelled. |

An error returned to the worker is not by itself a diagnostic; the SDK raises it as an exception.

## 4. The data plane

Fields and large tables never travel as JSON (R20). They travel as blobs.

### 4.1 Blobs

- A **blob** is an object of the store (`graph.md` 3.2): a write-once file in the `objects` folder,
  named by the BLAKE3 digest (O'Connor, Aumasson, Neves and Wilcox-O'Hearn, 2020) of its whole
  content, as 64 lower-case hexadecimal digits.
- Either side writes a blob to `<objects>/tmp/<any unique name>`, then moves it to
  `<objects>/ab/<digest>` (`graph.md` 3.2; `ab` the digest's first two hex digits; `tmp/` is the one
  flat entry) by a move that never replaces: `link` then `unlink` on macOS and Linux,
  `MoveFileExW` without `MOVEFILE_REPLACE_EXISTING` on Windows. If the name exists, the move fails
  and the new copy is deleted; the old one is kept. A blob is never replaced, appended to or deleted
  while a build runs. This keeps Windows working: a mapped file there cannot be replaced or deleted.
  A `tmp/` entry older than one day is deleted at the start of a build; a fresh one may belong to
  another build sharing the store.
- Both sides map blobs read-only. Every blob is checked against its name when the engine first reads
  it in a build (`graph.md` 3.2): from a step it is `protocol.blob_digest`, in the store
  `graph.store_corrupt`. So a blob damaged in the store after it was written is caught by the next
  build that reads it.
- A **raw blob** is bytes with no header (`input.open`, documents, column data).

### 4.2 Field files and column data

A field window, a field a step returns, and a field an author gives as a `file` are one format: the
array file of `build.md` 3.2. It is a NumPy `.npy` file, format version 1.0 (NumPy Enhancement
Proposal 1), in the subset below, with extension `.npy`. This chapter adds no other array format.
A field file is an array blob: the store names it by the digest of its whole content, header
included.

| Part | Rule |
|---|---|
| Magic and version | The six bytes `\x93NUMPY`, then the bytes 1 and 0 (version 1.0). |
| Header length | A little-endian u16: the length of the header text. |
| Header text | `{'descr': '<f4', 'fortran_order': False, 'shape': (513, 513), }` as NumPy writes it, padded with spaces and ended by one LF, so that magic, version, length and text together are a multiple of 64 bytes. A writer of ours pads to the smallest such multiple; a reader accepts any multiple of 64, since some NumPy releases pad further. |
| `descr` | `<f4` for a scalar field and for weights; `\|u1` for a cover. |
| `fortran_order` | `False`: C order. |
| `shape` | `(rows, columns)` for a scalar field or a cover; `(planes, rows, columns)` for weights, one plane per material. |
| Data | The values, little-endian, rows from the south (`build.md` 3.2), no trailing bytes. |

- NumPy reads one with no library of ours: `numpy.load(path, mmap_mode="r")`. `numpy.save` of a
  C-order `float32` or `uint8` array of two or three dimensions writes one.
- A returned field file outside the subset (another version, `descr`, order or number of
  dimensions, or a size that does not match its header), or inside it but of the wrong `descr` or
  `shape` for its field (5), is `protocol.field_file`, whose hint is `step.out.field`. A `.npy` as
  such is not refused: it is the format.
- Where a blob holds a field window, the message beside it gives the window; the header gives only
  type and shape.
- **Column data** is a raw blob: the values, little-endian, C order, with dtype and shape given in
  the message (4.3). `bool` is one byte, 0 or 1.
- **No non-finite value and no negative zero** enter a build (R26, `numbers.md` 8). The SDK writes
  `-0.0` as `+0.0` before it digests a blob. The engine reads every `-0.0` a step sends as `+0.0`, so
  a plain command in another language need not. A NaN or an infinity in a returned field file or
  column blob is `num.not_finite_result` (`numbers.md` 8 rule 7), naming the step and the post or
  row; in a call it is error -32004. A diagnostic's columns (`diag.emit`, `diagnostics.json`) are
  exempt: a non-finite `position` there replaces the record by `diag.malformed` (`diagnostics.md` 6).

### 4.3 Columns, tables and feature lists

A column in a message is one of:

```json
{"dtype": "f64", "values": [0.5, 1.25]}
{"dtype": "i32", "shape": [4096, 2], "blob": "9f2c…", "path": "/…/objects/9f/9f2c…"}
{"dtype": "str", "values": ["beck", "ford"]}
{"dtype": "str", "blob": "<utf-8 bytes>", "offsets": "<i64 blob, n + 1 entries>"}
```

`dtype` is `u8`, `bool`, `i32`, `i64`, `f32`, `f64` or `str`. `shape` is the array's dimensions,
and no other thing. The sender chooses inline or blob; a column of more than 4,096 values must be a
blob (R20: large tables never travel as JSON). An inline JSON integer never exceeds 2^53 in
magnitude; an i64 that might is sent as a blob.

A **table** is a set of columns of one length, one row per thing.

**Attribute columns.** Attribute values are resolved, as the `attr` node holds them (`graph.md` 1):
kind defaults and formula attributes applied (`spike/FINDINGS.md`: a step could not read a street's
width because kind defaults were not carried). An attribute's column type follows its type
(`authority.md`):

| Attribute | Column |
|---|---|
| a lattice length | `i32`, millimetres |
| a facing | `i32`, millidegrees |
| an integer | `i64` |
| another number | `f64` |
| a bool | `bool` |
| a string or a name | `str` |
| a list or a table | `str`, one canonical JSON text per row (`documents.md` 8) |

An attribute that two selected kinds give two types is `protocol.column_mixed`, naming both kinds;
the hint is to read each kind by its own `kind:` selector. Beside each attribute column is a `u8`
column `<attribute>?`: 1 where the feature has the attribute, 0 where it has not, and the value
there is 0 or `""`. Ids hold no `?`, so the name cannot collide.

A **feature list** is columnar, after the layout of GeoArrow (geoarrow.org, 0.1). With F features,
P pieces, R rings and V vertices:

| Column | dtype | Length | Meaning |
|---|---|---|---|
| `coords` | i32, shape `(V, 2)` | V | Every vertex, `x`, `y` in integer millimetres in the world frame (R24). |
| `geometry` | u8 | F | Per feature: 0 point, 1 path, 2 region. |
| `pieces` | i64 | F + 1 | Feature f's pieces are `pieces[f]` to `pieces[f+1]`. A point or a path has one; a region from `where` may have several. |
| `rings` | i64 | P + 1 | Piece p's rings are `rings[p]` to `rings[p+1]`. A point or path piece has one ring; a region piece's first ring is its outline, the rest holes. |
| `ring_starts` | i64 | R + 1 | Ring r's vertices are `coords[ring_starts[r]]` to `coords[ring_starts[r+1]]`. A point's ring has one vertex. |
| `name`, `kind`, `authority`, `layer`, attributes | per 4.3 | F | One row per feature. |

Regions are counter-clockwise outside and clockwise for holes, rings not closed by a repeated vertex
(`geometry.md`). Features are ordered by name, bytewise (`vocabulary.md` 2.2).

Vector: a point `a` at (0, 0); a path `b` from (0, 0) to (1000, 0); a region `c`, the square
(0, 0)-(4000, 0)-(4000, 4000)-(0, 4000) with the hole (1000, 1000)-(1000, 3000)-(3000, 3000)-
(3000, 1000); a region `d` in two triangles, (10000, 0)-(11000, 0)-(10000, 1000) and
(20000, 0)-(21000, 0)-(20000, 1000). Then `geometry = [0, 1, 2, 2]`, `pieces = [0, 1, 2, 3, 5]`,
`rings = [0, 1, 2, 4, 5, 6]`, `ring_starts = [0, 1, 3, 7, 11, 14, 17]`, and `coords` has 17 rows.

### 4.4 Keys, traces and step results

The step node's static key and trace are `graph.md` 5.1's, and are defined there only. This chapter
adds what the request carries beyond the step statement, and the canonical form.

- **What the static key holds** is `graph.md` 5.1's list. Its trust digest covers the code folder
  (section 7), so an edit to a module beside the script re-runs the step; its frame gives `grid`;
  its seed is the request's `seed`.
- **A plain command** has the request in its static key, less `out`, `objects` and the absolute
  path of each input (its path as written and digest stay). Everything in `data` is there by blob
  digest, so its trace is empty (`graph.md` 5.1). It is reused only when the static key matches;
  any change in its view gives a new key, and it runs again (2.2).
- **A worker's trace** is its traced calls (3.3), in the order made, each as its normalised request
  (window clipped to the grid, names and columns sorted, selectors as written, `call` left out) and
  the digest of its answer. A plain command and a worker that read the same things therefore keep
  different records: one key with no trace, or one key with a trace. Neither is converted to the
  other.
- **Canonical form.** Every part of a key or a trace is canonical JSON (`documents.md` 8): members
  sorted bytewise, paths as written relative to the declaring document, every blob by digest. No
  absolute path, no `out`, no `objects` and no call folder enters a key. So a key is the same on
  every machine and in every folder (R26).

Vector, a plain command handed only `height` over a 3 by 2 grid, inputs `tools/a.csv`: its static
key covers `{"command": [...], "data": {"fields": {"height": {"blob": "<digest>", ...}}, ...},
"grid": {"extent_mm": [0, 0, 2000, 1000], "nx": 3, "ny": 2, "resolution_mm": 1000}, "inputs":
[["tools/a.csv", "<digest>"]], "seed": "<16 hex>", ...}`; its trace is empty. The same read by a
worker gives a static key without `data` and a trace of one read, `{"method": "build.field",
"fields": ["height"], "halo": 0, "view": "full", "window": [0, 0, 3, 2]}` with the answer's digest.
The bytes are owed to `spec/vectors/protocol/`.

A **step result** is the step node's value as `graph.md` stores it: the returned document, the files
of `out` as objects, the diagnostics with their `explain` texts, and the trace. Reuse is
`graph.md` 5.1's. A call that did not complete keeps no result (`graph.md` 3.4).

A step is recorded, not promised (R26): the same result gives the same bytes; a fresh call of the
same code may not. Reads the protocol cannot see (a file not in `inputs`, the clock, the network)
are not prevented and go stale unseen. The protocol has no sandbox; the trace is honest about what
passed through it and no more.

## 5. What a step returns (M2; `diagnostics.json` M1)

A generate step writes into `out`:

| File | Holds |
|---|---|
| `document.toml` | Its document, TOML text. A worker may instead return the text in `step.call`'s result; returning both is `protocol.output_twice`. A generate step that returns neither is `protocol.no_output`; one with nothing to return writes an empty document. |
| field files it names | Field files (4.2), named by `file` relative to `out`. One outside 4.2's subset, or whose `descr` is not `<f4`, or whose `shape` is not the grid's `(ny, nx)` (or `(planes, ny, nx)` for weights), is `protocol.field_file`. |
| `diagnostics.json` | Plain command only: a JSON list of objects, one per diagnostic (below). Not a JSON list of objects: `protocol.output_json`. No file, or `[]`: no diagnostics. |

Each object of `diagnostics.json` is one row of `diag.emit`, written as JSON: `severity`, `code`,
`subject`, `message` and `hint` as strings; and where given, `position` as `[x, y]` in metres,
`related` as a list of `{"note", "subject"}`, `data` as an object, and `explain` as a string. As
with `diag.emit`, the first object of each code carries `explain`, or the code is
`protocol.no_explain`. Each object is then checked as `diagnostics.md` 6 checks a step diagnostic;
a member not listed, or of another JSON type, is `diag.malformed` there. Vector:
`[{"code": "steep", "data": {"slope": 0.62}, "explain": "The bank is steeper than a ford allows.", "hint": "Lower the bank or move the ford.", "message": "Slope 0.62 exceeds 0.5.", "position": [64, 181.5], "severity": "warning", "subject": "feature.ford"}]`.

After the call the engine stores every file in `out` as an object. A lint or export step writes no
`document.toml` (`protocol.output_table`).

TOML only: the spike's lesson that generated documents should be JSON, since Python's standard
library has no TOML writer (`spike/FINDINGS.md`), is overruled by principle 8: a returned document
is read by the same reader as authored text, before any check, so every error in it names a line of
`document.toml`. The SDK carries its own writer for the subset it emits. A plain command in another
language writes TOML itself.

The document may hold only these tables, and never a top-level key:

| May | Rule |
|---|---|
| `[field.<n>]` of its own (R32) | `file` (with the keys `fields.md` 4.2 requires) or `base`, a new name. Not on a field it did not declare (`field.step_base`). |
| `[field.<n>.then.<id>]` | A row on a field in its stratum (`fields.md` section 7). On a field of a higher layer: `field.step_layer`. On a field outside its stratum by any other way: `graph.read_outside`. |
| `[feature.<key>]`, `[instance.<key>]` | Features and instances. |
| `[set.<key>]`, `[run.<key>]`, `[scatter.<key>]` | What a library form lowers to (`rules.md` section 10). |

Anything else (`[step]`, `[layer]`, `[pin]`, `[clear]`, `[lock]`, `[import]`, `[given]`,
`[change]`, `[kind]`, `[let]`, `remove`, `unset`) is `protocol.output_table`.

- **What R32 allows.** Rows on any field below it, features, and fields of its own: new names,
  declared in the document it returns. Nothing below the step is applied to a field of its own, so
  nothing is applied twice; a field that already exists is never replaced (`field.step_base`). A
  returned `then` row is a new row under a lower layer's table, which hand-written text may not add
  (`documents.md` 5): R32 allows it to a step only.
- **Its layer.** The returned document joins the step's layer: everything in it is declared in that
  layer, with its rank and its owner in the chain of every diagnostic (`documents.md` 5), and spans
  in `step:<id>/document.toml` (`documents.md` 2.1). It lies outside the stratum of every step of
  that layer and inside that of every step above (`graph.md` 4).
- **Names.** Every table the document returns is named `<step>#<key>` in the build (`identity.md`
  2.4): a field, a row, a feature, an instance, a rule. Inside the document, a name resolves first
  to the document's own keys, then to the stratum, so a returned row reads its own field as
  `f('<key>')`. Outside it, a formula in a higher layer reads a returned field by a tag the step
  sets on it (`tags` on a field of its own, `fields.md` 7), as `f('tag:<tag>')`; `'<step>#<key>'`
  in a formula is `identity.key_in_text` (R3, `formulas.md` 7). A key equal to a name the step can see is
  `protocol.key_shadows`. Under `for_each`, keys must already differ between items (`identity.md`
  2.5).
- **Rank.** Everything returned has the rank of the step's layer, and refers downward only
  (`documents.md` 5.1).
- **Files.** A `file` that leaves `out`, or names nothing, is `protocol.output_file`. So is a
  symbolic link anywhere under `out`, on any step: the engine neither follows nor reads it, and
  stores nothing through it.
- **Values.** Number rules are the text's: `num.not_finite`, `num.too_precise` at the line in
  `document.toml`. The SDK writes lattice values exactly from millimetres.
- **Holds.** The core applies holds to what is returned, after the call (`identity.md` 4.1).
- **Order.** Returned rows sit in the field's head (`authority.md` 4.1): `order`, then layer rank
  (the step's layer), then produced name `<step>#<id>`, bytewise (R29); never by their place in
  `document.toml`. A field of its own by `file` is its base, before every row.

On an error in the document, nothing from it is applied, the build stops after generation
(`diagnostics.md` section 4), and no result is kept for reuse.

## 6. Running a process

### 6.1 The program

`command[0]` is never looked up on `PATH`. It is one of:

| Form | Resolves to |
|---|---|
| A name: no `/` in it, such as `python` | The program of that name in the user's `config.toml` (6.3). Absent: `protocol.no_program`, whose hint is `via config program <name> <path>`. |
| A relative path: a `/` in it | A program shipped with the world, relative to the declaring document. It must be a program the system can start: on Windows an `.exe`. A world meant for several systems names an interpreter and a script. |
| `lib:<library>/<path>` | A program shipped with a library (section 1). |

The other elements are passed as written, as an argument vector, never through a shell; a `lib:`
element among them is resolved to its absolute path first. An absolute path in `command` is
`protocol.absolute_path`: it would not build on another machine. A `.bat` or `.cmd` file is refused
as a program (`protocol.no_program`), because Windows passes its arguments through `cmd.exe`
quoting. The working folder is the folder of the declaring document, or the library's folder for a
`lib:` program.

The environment is the engine's, with:

| Variable | Why |
|---|---|
| `VIA_STEP` | Plain command: the request's path. |
| `VIA_PROTOCOL=via-step-1` | The protocol version. |
| `PYTHONHASHSEED=0` | Python's order of string sets does not vary between calls. |
| `PYTHONDONTWRITEBYTECODE=1` | No `__pycache__` is written into the code folder (section 7). |
| `PYTHONSAFEPATH=1` | The working folder is not put on Python's search path, so a `viasdk.py` or `numpy.py` beside the world cannot shadow the installed one and run untrusted. The launcher puts the script's own folder, inside the trusted code folder, on the path. Needs Python 3.11 or later. |
| `PYTHONPATH` | The SDK's folder (8.1) first, then the engine's own `PYTHONPATH` if it has one. So `python -m viasdk` finds the SDK that came with the engine, and nothing earlier on the path can shadow it. |
| `PYTHONUTF8=1` | Standard error is UTF-8 on Windows too. |

### 6.2 Lifecycle

| Phase | Deadline | On miss |
|---|---|---|
| Start to `initialize` answered | 10 s | `protocol.deadline`; the process tree is killed. |
| `step.call` answered, or a plain command exits | 600 s, or `--step-deadline SECONDS` on `via build` | `$/cancelRequest` (worker), then as below; a plain command is killed. |
| `$/cancelRequest` to the call's answer | 5 s | Killed; `protocol.deadline`. |
| `shutdown` and `quit` to process exit | 2 s | Killed; `protocol.slow_exit` (info). |

The deadlines are ours, defaults until measured (open, below). A worker that exits while a call is
pending is `protocol.crashed`. A process that cannot be started is `protocol.start`.

**The log.** A step's standard error (and a plain command's standard output) is its log, decoded as
UTF-8 with each invalid byte replaced by U+FFFD. A process that cannot be started has as its log the
system's reason. Each stream is kept up to 1 MiB; what follows is read and dropped, and the log ends
with a line saying how much. A stream still open at the deadline (a descendant that left the process
group holds it) is abandoned, and the call is `protocol.deadline`. The log is kept in the store and named by digest in the run report's `steps[].log`
(`graph.md` 6.3, `build.md` 11.3); it is not part of the build, and the terminal prints its last 40
lines under the diagnostic. These diagnostics are world diagnostics, so `diagnostics.json` holds only
facts that are the same on every system (R26, `build.md` rule 6): `protocol.failed` has `data.exit`,
the exit status; `protocol.start` has `data.kind`, `not_found`, `denied`, `other`, or `empty` for a
`command` with no program (located at `command`);
`protocol.raised` has `data.exception`, the exception's type name; `protocol.crashed` has none. The
message is made from these facts only. No log text, traceback or digest of a log enters it.

**No signals.** The protocol never uses a signal to ask a step anything: a worker is asked to stop
by `$/cancelRequest` or `shutdown`, and otherwise its whole process tree is killed at once.

| System | How a step is started and killed |
|---|---|
| Windows | Created suspended (`CreateProcessW`, `CREATE_SUSPENDED`, `CREATE_NEW_PROCESS_GROUP`), assigned to a Job Object with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` and no breakaway, then resumed. Killed with `TerminateJobObject`. If the engine dies, the job's handle closes and the tree dies with it. |
| Linux | Started by fork and exec from one long-lived engine thread (the parent-death signal fires when the spawning thread ends, not the process). Between them (Rust's `pre_exec`): its own process group, `prctl(PR_SET_PDEATHSIG, SIGKILL)`, then `getppid()` checked against the engine's id, exiting if it differs (the engine died first). Killed by `SIGKILL` to the group. |
| macOS | Its own process group, as Linux. macOS has no parent-death signal: a worker ends when its protocol input ends (3.1). Killed by `SIGKILL` to the group. |

What this does not catch: on Linux and macOS, a grandchild that leaves the group (`setsid`,
`setpgid`) escapes the group kill; when the engine dies, the parent-death signal is not inherited by
grandchildren, and a plain command on macOS has no protocol input to lose. These are (open), below.

A terminal's Ctrl+C reaches the engine and not the step, which is in its own group; the engine then
cancels and kills as above, removes each call folder (6.3), and ends by the same signal (`SIGINT`, or
`SIGTERM` when that was sent), writing no build directory and no run report.

### 6.3 Folders and files of the machine

**The call folder** is `<store>/calls/<nonce>/`, holding `request.json` (plain command) and `out/`.
It is deleted once the step result is stored; nothing in it enters a key.

**The user config folder** is `%APPDATA%\via` on Windows, `~/Library/Application Support/via` on
macOS, `$XDG_CONFIG_HOME/via` (default `~/.config/via`) on Linux. `VIA_CONFIG_DIR` replaces it, for
CI and for conformance. It holds two files, both written only by the command line:

| File | Tables | Keys |
|---|---|---|
| `config.toml` | `[program.<name>]` | `path`: the program's absolute path on this machine. |
| `trust.toml` | `[trusted.<trust digest>]` | `step`, `world`, `code_folder`, `files`: what `via trust` showed, kept for review only. Trust is the digest alone. |

These are files of the machine, not of a world. This chapter documents them; the vocabulary does
not list them (R33).

**Commands.** Owed to `build.md` 14's table:

| Command | Does | Exit |
|---|---|---|
| `via trust WORLD [--change C]... [--yes] [--json]` | Lists every step of the world, its layers and the given changes: id, `command`, code folder (or "none: installed code is not covered"), file count and trust digest. Asks once for all; with `--yes`, asks nothing. On yes, writes them to `trust.toml`. `--json` prints the list as canonical JSON. | 0 when every step is trusted, 1 when declined or on an error |
| `via config program NAME PATH` | Writes `[program.NAME] path = PATH` to `config.toml`. `PATH` must be absolute and exist. | 0, or 1 |

## 7. Trust

A world from a stranger must not run programs unasked (`docs/design/blackbox.md`, Open). Trust
changes whether a step can run, never what a build contains.

- **Text never executes.** Reading TOML runs nothing, and a formula is evaluated by the engine's
  closed evaluator (R6). `via build`, `via merge` and `via explain` run no program that is not
  trusted.
- **The code of a step** is `command` as written, plus every file under its **code folder**: the
  library's folder for a `lib:` element, otherwise the folder holding the first element of `command`
  that names an existing file relative to the declaring document. A command where no element names
  a file (`["python", "-m", "mypkg"]`) has no code folder: only `command` is digested, and the
  installed package is the user's own, as a named program is. `via trust` says so in its list. A
  code folder that is, or holds, the folder of the world file is `protocol.code_folder`, whose hint
  is to move the script into a folder of its own: otherwise every edit to the world would need
  trusting again.
- **The code folder's digest** covers its files in path order, each by relative path and content.
  Left out: `__pycache__` folders, names starting with `.`, and symbolic links, so a call, an editor
  or the system does not change it. In a file that is valid UTF-8 with no NUL byte, each CRLF is
  read as LF, so a checkout on Windows with `core.autocrlf` digests as on the other systems. The
  digest is cached by size and modification time. `graph.md` 5.1's digest of a command file is owed
  the same rule.
- **The trust digest** is the digest of `command` and the code folder's digest. A step runs only if
  its trust digest is in `trust.toml`. `params`, `inputs` and the world's text are data and need no
  trust. Programs and modules installed on the machine are the user's own and are not covered.
  A `command` of one word naming a configured program is not trusted again when the program's file
  changes, but that file's digest is in the step's static key (`graph.md` 5.1), so the step runs
  again; `via trust` lists such a step as "none: installed code is not covered". The terrain
  library ships so today (`["via-terrain"]`); whether a library may be a program and not a `lib:`
  folder of Python is open (section 8, R19a).
- **Libraries.** Trusting is by digest, so a library trusted once is trusted in every world, and any
  edit to its code asks again. A library shipped inside the `via` binary is trusted when its code
  folder's digest equals the one the binary carries. A library found elsewhere, such as `lib/` named
  by `--lib` while we develop it, is trusted like any other code, by `via trust`.
- **An untrusted generate step** with no committed result is `protocol.untrusted`, whose hint is
  `via trust <world>`, after reading the code folder it names. The program is never started.
- **An untrusted lint or export step** is skipped with `protocol.untrusted_skipped` (warning).

### 7.1 Committed step results (M2)

So a world whose steps a reader has not trusted still builds, with the bytes a trusted build gives
(`basin`: an erosion step a reviewer need not run).

- A committed step result is the folder `gen/<step id>/` (`gen/<step id>/<item>/` under `for_each`)
  beside the declaring document: `document.toml`, the files it names, and `result.json`, canonical
  JSON holding `protocol`, `static_key`, `key`, `trust_digest`, `trace` (a list of
  `{ "request", "answer" }` as 4.4 normalises them), `facts` (the stated facts it read, `derived.md`
  5, so a lint that `checks` it is handed them), `files` (path in `out` to digest), `diags` and
  `diagnostics` (the diagnostics the step raised itself, with `diag.emit` or its
  `diagnostics.json`: `diags` as the build holds them, hex of the store's encoding, which the build
  replays; `diagnostics` each as `{ "code", "message", "severity", "subject" }` for the reviewer)
  and `explains` (each code's explain text as `[code, text]`). A trace's request names its large
  columns by blob digest (4.2); each such blob is committed beside it as `result.blobs/<digest>` and put
  in the reading machine's store before the trace is checked, so the result reads the same on any
  machine. `result.blobs/` is not one of `files`.
- `static_key` is the digest of `key`, the parts of `graph.md` 5.1's static key that do not depend
  on this machine's trust: the step's statements, its parameters (less `trusted`), its direct
  digests and its inputs' values, each by name. So a stale result names the part that differs.
  A configured program the reading machine does not have keeps the recorded digest: a reviewer who
  never runs the program cannot compare it; one who has it must match.
- An untrusted generate step builds from it only if its static key equals the current one and every
  read of its trace gives the recorded answer now. The check runs nothing: each read is answered from
  the current graph, as reuse is (`graph.md` 5.1). Then the build uses it with
  `protocol.untrusted_result` (info), and its bytes equal a trusted build's: the step's own
  diagnostics and explain texts are replayed from `diags` and `explains`, so they are reported as
  a trusted build reports them. The folder's digest is
  in the build key and in the step's key, so a new commit is read.
- On any difference it is `protocol.untrusted_stale`, naming the first part of the key or the first
  read that differs; the hint is `via trust`, or to commit a fresh result from a trusted machine.
- **`via step commit WORLD [--change C]... [--step ID]...`** builds as `via build` does, then writes
  the result of every generate step (or of each `--step`) to its `gen/` folder, replacing the folder
  whole, by a temporary folder renamed over it. A build with any error writes nothing:
  `protocol.commit_refused`, exit 1. `result.json` is text, so the trace is kept and reviewed in the
  repository with the document. Exit 0 otherwise, printing the folders whose bytes changed.

## 8. The SDK, `viasdk`

Pure Python, 3.11 or later, with NumPy as its one dependency. It never links or embeds the engine
(R20); it speaks section 3, or reads section 2's request. Our libraries use it and nothing else
(R19b).

One library does not yet: `lib/terrain`'s step and lint are the Rust program `via-terrain`, which
links core crates no user step can reach (`via-num`, the seed derivation, the store) and computes
its own noise. Nothing in this chapter admits a Rust door, so R19a and R19b are not met for it. Open
for the owner: publish `via-step`, `via-num` and the seed derivation as a supported door, or move
the library to Python on this SDK. Until then its code is covered by the static key only (7).

| Part | What it is |
|---|---|
| `python -m viasdk <script>`, plain command (M1) | With `VIA_STEP` set and no `worker`: gets `step` as `viasdk.request()` does, puts the script's folder on the path, imports the script and calls its top-level function named after `when`: `generate(step)`, `lint(step)` or `export(step)`. When the function returns, it writes what `step.out` and `step.error(...)` and the like hold into `out` (section 5) and exits 0. A missing function, or an exception the script does not catch, is written to standard error with its traceback and exits 1, which the engine reports as `protocol.failed`. It speaks no protocol. |
| `python -m viasdk <script>`, worker (M2) | With `worker = true`: as 3.1. Takes the standard descriptors, answers `initialize`, imports the script and calls the same function for each `step.call`. A missing function is `protocol.no_entry`; an exception, `protocol.raised`. |
| `viasdk.request()` | For a plain command: reads `VIA_STEP` and gives the same `step` object. Reads are answered from `data`; computes raise `viasdk.NotAWorker`. A script with its own `main` may call it directly instead of using the launcher. (M1) |
| `step.params`, `step.grid`, `step.for_each_item`, `step.seed` | From the request. |
| `step.fields(names, view="full", window=None, halo=0)` | `build.field`: read-only mapped arrays, each with its window; weights with their material names. |
| `step.features(select, within=None, columns=())` | `build.features`: a feature list of NumPy columns, coordinates in millimetres, with a helper to metres. |
| `step.instances(...)`, `step.input(path)`, `step.pack(select, columns=())` | The other traced reads. |
| `step.eval(formula, x, y, view="full")` or `step.eval(formula, window=..., view="full")` | `expr.eval`, one call per batch. |
| `step.derive_where(...)`, `step.route(...)`, `step.descend(...)`, `step.spill(...)` | `derived.md` 4's operations, one call per batch. |
| `step.checks` | A lint's `checks` member (2): each checked step and the stated facts it read. |
| `step.error(...)`, `step.warning(...)`, `step.info(...)` | `diag.emit`. `hint` is a required argument. |
| `step.out` | A document builder: `feature(key, kind, ...)`, `instance(key, asset, ...)`, `row(field, id, value=None, mask=None, paint=None, order=None)`, `field(name, array)`. It writes `document.toml` with lattice values exact from millimetres, and each field as a field file in `out` with `-0.0` written as `+0.0`. It writes text and fields and holds nothing else (R7). |

The function list `expr.eval` takes is generated from the function table (`formulas.md` section 11),
neighbourhood functions included (`formulas.md` 8.6, R28): over a window, `expr.eval` reads each
function's declared halo as a formula does. So a formula and a worker give the same bytes and one
can be moved to the other (R7). A plain
command has no function list: it reads files only (R22), and the SDK computes nothing itself. So
R7's "one can be moved to the other" holds for workers; the lead is asked whether R7 means that.
The SDK's version is recorded in each step result and never checked; the protocol version is (3.2).

The launcher tells the two transports apart by `VIA_STEP` alone: the engine sets it for a plain
command and never for a worker (6.1). So the same script runs both ways, and moving a step from
plain to worker is one key, `worker = true`, with no change to the script.

### 8.1 Where `viasdk` comes from (M1)

The engine puts the SDK on the configured interpreter's path; nothing is installed into the
interpreter. The `via` binary carries the files of `sdk/python/` as it carries a shipped library.
Before the first Python step of a build it writes them, once, to `<store>/sdk/<digest>/`, where
`<digest>` is the SDK folder's digest by section 7's rule, and sets `PYTHONPATH` to that folder
(6.1). `--sdk DIR` replaces the carried copy with a working copy such as the repository's
`sdk/python/`, for developing the SDK, as `--lib` does for libraries.

- **Why not install it.** An editable install changes the user's interpreter, needs `pip` and a
  writable environment, and leaves the SDK's version decoupled from the engine's. A path set per
  call changes nothing outside the call and gives every step the SDK that matches its engine.
- **Keys and trust.** The carried SDK is part of the engine: its digest is fixed by the engine's
  version, which every key already holds (`graph.md` 5.1). With `--sdk`, the folder's digest is
  added to every step's static key and trust digest, as a library's code folder is; `via trust`
  lists it once.
- **What the user still installs.** Python 3.11 or later, named once with `via config program`,
  and NumPy in that interpreter. A missing NumPy is the script's import error, so
  `protocol.failed`, its message in the log.

Shipping `viasdk` to PyPI, and whether a project's virtual environment may serve as a named program,
are M2 packaging (Open).

## 9. Worked example

A generate step in the layer `shaping` softens the ground by a row it computes, reading a field of
its own (M2, R32).

```toml
[step.soften]
when    = "generate"
worker  = true
command = ["python", "-m", "viasdk", "tools/soften.py"]
params  = { strength = 0.3 }
```

```python
# tools/soften.py
import numpy as np

def generate(step):
    (h,) = step.fields(["height"], view="head")        # whole grid; it returns a row on height
    p = np.pad(h, 1, mode="edge")                      # a scalar field is (rows, columns)
    soft = (p[:-2, 1:-1] + p[2:, 1:-1] + p[1:-1, :-2] + p[1:-1, 2:]) / 4
    step.out.field("soft", soft.astype(np.float32), tags=["soft"])
    s = step.params["strength"]
    step.out.row("height", "soften", value=f"= mix(v, f('soft'), {s})")
```

The row reads its own field as `f('soft')` (section 5, Names) and the value before it as `v`; it
does not read `f('height')`, which inside a row of `height` is a cycle (`formulas.md` 8.3). It reads
`height` in its head view, since it returns a row on it (`authority.md` 4.4). In the build the
field is `soften#soft` and the row `soften#soften`; a formula in a higher layer reads the field by
its tag, `f('tag:soft')`. The row sits in `height`'s head at `shaping`'s rank (`authority.md` 4.1).

On the wire (frames abbreviated; `Content-Length` counts bytes):

```
engine → Content-Length: 118\r\n\r\n{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocol":"via-step-1",…}}
worker → {"jsonrpc":"2.0","id":1,"result":{"protocol":"via-step-1","sdk":{"name":"viasdk",…},…}}
engine → {"jsonrpc":"2.0","id":2,"method":"step.call","params":{"step":"soften","when":"generate",…}}
worker → {"jsonrpc":"2.0","id":1,"method":"build.field","params":{"call":2,"fields":["height"],"view":"head"}}
engine → {"jsonrpc":"2.0","id":1,"result":{"height":{"blob":"4be1…","i0":0,"j0":0,"i1":513,"j1":513,…}}}
worker → {"jsonrpc":"2.0","id":2,"result":{"document":"[field.soft]\nfile = \"soft.npy\"\ntags = [\"soft\"]\noutside = 0\n…"}}
```

The trace holds one read: `build.field` of `height`, head view, whole grid, answer `4be1…`. Editing
a layer above `shaping` leaves that answer unchanged, so the step is reused. Adding a `then` row on
`height` below `shaping` changes it, so the step runs again.

## 10. Forward pointers

Not specified until the milestone that needs them. Each is decided by the case named.

| What | Milestone | Decided by |
|---|---|---|
| `build.holds`: pins, clears and locks that speak to a step (`identity.md` 4.1), and `data.holds` | M3 | The first step that returns instances under a pin (`quay`). |
| `name.place`: a place key computed by class (`identity.md` 2.3, 2.4), shapes in, keys out, so a library names a lot as the core names a cell (R19b); and the document key that carries a returned item's class | M3. M2's steps key what they return by the names of the facts or items it came from (`identity.md` 2.2), which no distant edit shifts. | `identity.md` 2.4's owed calls; R13's "ids do not shift". |
| More array operations than `derived.md` 4's (sample at points, a distance field) | when `expr.eval` with R28's `slope` and `dist` will not do | The first step that needs one. |
| `geom.op` and `handle.release`: operations of `geometry.md` 3 and questions of `surfaces.md` 2, with results kept in the engine | M3 | The street and lot libraries (R13, R23). |
| `progress`: a notification shown on the terminal and kept nowhere | M2 | `basin`'s 3.5 s model. |
| More than one `step.call` at a time per worker | after M2 | A step whose calls are measured to wait on each other. |
| Capabilities for dtypes and message size | when either varies | A second SDK. |
| The output name of the player-map export step (`build.md` 9.4) | M6 | The player map. |

## Diagnostics

| Code | Severity | Says |
|---|---|---|
| `protocol.unknown_library` | error | A `lib:` element names a library not in `include`; the nearest name. |
| `protocol.no_program` | error | `command[0]` is no configured program and no file, or is a `.bat`/`.cmd`; the hint gives `via config program`. |
| `protocol.absolute_path` | error | `command` holds an absolute path. |
| `protocol.code_folder` | error | The code folder is or holds the world's folder; move the script into its own folder. |
| `protocol.untrusted` | error | A generate step is not trusted and has no committed result; the hint is `via trust` (M2). |
| `protocol.untrusted_result` | info | An untrusted step was built from its committed result, which matched (M2). |
| `protocol.untrusted_stale` | error | A committed result's static key or a read of its trace differs from the current build; names the first difference (M2). |
| `protocol.untrusted_skipped` | warning | An untrusted lint or export step was skipped. |
| `protocol.start` | error | The process could not be started; `data.kind` (6.2). |
| `protocol.version` | error | The worker speaks another protocol version; both versions, `data.engine` and `data.worker` (M2). |
| `protocol.when_unserved` | error | The worker's `capabilities.when` lacks the step's `when`, `data.when` (M2). |
| `protocol.no_entry` | error | The script has no top-level function named after `when` (M2). |
| `protocol.framing` | error | A frame was not `Content-Length` and JSON; likely output written to the protocol descriptor (M2). |
| `protocol.message_too_large` | error | A frame over 16 MiB, `data.bytes` long; send the data as a blob (M2). |
| `protocol.deadline` | error | No answer by the deadline; which phase, `data.phase` (`initialize`, `step.call` or `cancel`), and the deadline in seconds, `data.deadline`. |
| `protocol.crashed` | error | The worker exited during a call (M2). |
| `protocol.raised` | error | The step raised an exception it did not catch; `data.exception` (M2). |
| `protocol.failed` | error | A plain command exited non-zero; `data.exit`. |
| `protocol.slow_exit` | info | A worker had to be killed after `shutdown` (M2). |
| `protocol.input_undeclared` | error | `input.open` named a path not in `inputs` (M2). |
| `protocol.blob_digest` | error | A blob's bytes do not match its name (M2). |
| `protocol.blob_header` | error | A column blob whose header or size does not match the `dtype` and `shape` its message gives. A non-finite sample is `num.not_finite_result` (4.2) (M2). |
| `protocol.column_mixed` | error | One attribute has two types across the selected kinds; names both kinds. |
| `protocol.no_output` | error | A generate step returned no document (M2). |
| `protocol.output_twice` | error | A worker returned a document and wrote `document.toml` (M2). |
| `protocol.output_json` | error | A plain command's `out/diagnostics.json` is not a list of diagnostic objects. |
| `protocol.field_file` | error | A returned field file is outside 4.2's `.npy` subset, or has the wrong `descr` or `shape` for its field; the hint is `step.out.field` (M2). |
| `protocol.output_table` | error | A returned document holds a table or key a step may not return, or a lint or export returned one. |
| `protocol.output_file` | error | A returned `file` leaves `out` or names nothing (M2). |
| `protocol.key_shadows` | error | A returned key equals a name the step can see (M2). |
| `protocol.for_each_form` | error | `for_each` is not `kind:<name>` (M2). |
| `protocol.commit_refused` | error | `via step commit` found errors in the build; nothing written (M2). |
| `protocol.no_explain` | warning | A step sent a code with no `explain` text. |
| `protocol.explain_differs` | warning | A step sent two `explain` texts for one code. |

Codes of other chapters used here: `graph.read_outside`, `graph.input_missing`,
`graph.step_unstable`, `graph.store_corrupt`, `graph.for_each_empty`, `field.step_base`, `field.step_layer`,
`authority.step_view`, `num.not_finite`, `num.not_finite_result`, `num.too_precise`, `doc.type`, `doc.not_yet`,
`identity.key_collision`.

## Conformance

Each case runs through the command line (R20), with `VIA_CONFIG_DIR` set as `conformance.md` 5.2 sets it (an empty `{work}/.config`; programs and trust are the case's own actions); the
M0 probe alone runs from its own harness. Worlds and scripts live in `spec/vectors/protocol/<case>/`,
written with the cases. A case that asserts no process remains uses a fixture that writes its own
process id and its children's to `out/pids`; the runner checks after the build that none is alive.
Every case on macOS, Linux and Windows gives the same build `id` and the same diagnostics, compared
by code, severity, subject, location and `data`.

| # | M | Input; action | Must hold |
|---|---|---|---|
| 0 | M0 | The probe: a 6.5 MB f32 field file out of a worker and back, a call back, a worker killed mid-call | Bytes equal; the call back answered; no process remains. |
| 1 | M2 | A plain-command generate step returning one feature; build | The feature is `<step>#<key>` with its step's rank. |
| 2 | M1 | A plain lint, built again unchanged, then in another folder and on another machine with the store copied | The step does not run; its result is reused. |
| 3 | M1 | A plain lint; a byte of one of its `inputs` changed; then a file added to a folder named in `inputs`; then an input deleted | Step ran; step ran; `graph.input_missing`. |
| 4 | M2 | `extent = [0, 0, 1280, 1280]`, `resolution = 1`; a plain step returns field `wave` from `i + 0.5 * j` as f32 and the row `[field.height.then.w]` with `value = "= f('wave')"` | `height`'s bytes equal those of the same world with the row `value = "= x + 0.5 * y"`. |
| 5 | M1 | A plain command exiting 3 | `protocol.failed` with `data.exit = 3`; `diagnostics.json` byte-identical on every system. |
| 6 | M1 | A plain lint not in `trust.toml`, no `gen/` | `protocol.untrusted_skipped`, exit status 0, the program never started. |
| 7 | M1 | `via trust WORLD --yes`; build twice | Both builds trusted: no `__pycache__` or dot-file changes the digest. |
| 8 | M1 | Same; then one file in the code folder edited | `protocol.untrusted_skipped`. |
| 9 | M1 | The code folder checked out with CRLF line ends | Same trust digest as with LF. |
| 10 | M1 | `command = ["python", ...]`, no configured program, `python` on `PATH` | `protocol.no_program`. |
| 11 | M1 | A `numpy.py` beside the world that writes a file when imported | The file is never written. |
| 12 | M2 | A step added by an inline change in layer `shaping`, and one by a change file | Ranks of `shaping` and `world`. |
| 13 | M2 | A returned document with `[pin.x]` | `protocol.output_table` at its line in `document.toml`. |
| 14 | M2 | A returned row on a field of a higher layer | `field.step_layer`, at its line. |
| 15 | M2 | A returned `file = "../x.npy"`; then a returned `.npy` of `<f8`, written by `numpy.save`; then one of `<f4` with shape `(ny - 1, nx)` | `protocol.output_file`; then `protocol.field_file`; then `protocol.field_file`. |
| 16 | M2 | A field file written big-endian (`>f4`), in Fortran order, as `.npy` version 2.0, or with a wrong magic; then one written by `numpy.save` from a C-order `float32` array of the grid's shape | `protocol.field_file` for each; then the build accepts it. |
| 17 | M2 | A plain step returning a field holding `-0.0` | The build holds the bytes of `+0.0`. |
| 18 | M1 | A plain lint handed the four features of 4.3's vector; then 2.2's `data` vector | Its columns equal the vector; its `data` equals the vector's bytes, less absolute paths. |
| 19 | M1 | Each row of 2.1's table in `params` | The request's `params` text equals the table. |
| 20 | M1 | A plain lint step with a diagnostic and `explain`; `via explain step.<id>.<code> WORLD` | Prints the text, runs no program. |
| 21 | M2 | A worker reading `height` in a layer; edit a layer above it; then add a `then` row below it | Not re-run; then re-run. |
| 22 | M2 | Build with and without a change by turns, three times | The step runs twice in all. |
| 23 | M2 | `expr.eval` of a formula over a window | Each value, rounded to f32 ties to even, equals the field the formula builds; length and post order as 3.3. |
| 24 | M2 | A worker that prints at import time | The build succeeds; the text is in the log. |
| 25 | M2 | A worker that does not use the launcher and writes `hello\n` to descriptor 1 after `initialize` | `protocol.framing`. |
| 26 | M2 | A worker that runs `cat` as a subprocess with no `stdin` argument | The build succeeds; `cat` reads nothing. |
| 27 | M2 | A worker answering `initialize` with `via-step-2` | `protocol.version`, exit status 1. |
| 28 | M2 | A worker that sleeps past `--step-deadline 2` | `protocol.deadline`; no process remains. |
| 29 | M2 | A worker with a child, killed by the engine (deadline) | No child remains, on every system. |
| 30 | M2 | A worker with a child; the engine itself killed | The worker is gone on every system; a child that stayed in its group is gone on Windows (others: open). |
| 31 | M2 | A worker killed by the fixture mid-call | `protocol.crashed`. |
| 32 | M2 | A step raising `ValueError` | `protocol.raised` with `data.exception = "ValueError"`; the traceback in the log. |
| 33 | M2 | A worker reading through `input.open` a file not in `inputs` | `protocol.input_undeclared`. |
| 34 | M2 | A worker reading by name a feature of a higher layer | `graph.read_outside` naming that layer, and no other diagnostic. |
| 35 | M2 | Two workers in one layer; one reads the other's returned feature by name | `graph.read_outside`. |
| 36 | M2 | A worker reading `kind:x` where one `x` feature lies above its layer | That feature is absent; no diagnostic. |
| 37 | M2 | A worker reading `height`'s full view and returning a row on it | `authority.step_view`. |
| 38 | M2 | A committed `gen/<id>/` for an untrusted step; build; then edit a param | `protocol.untrusted_result` and the trusted build's `id`; then `protocol.untrusted_stale`. |
| 39 | M2 | A worker with `for_each = "kind:x"` over three features; one moved | Exactly one call ran; seeds equal `numbers.md`'s vectors. |
| 40 | M1 | `command = ["python", "-m", "viasdk", "lib:review/checks.py"]`, no `worker`, a configured Python with no `viasdk` installed in it | The launcher runs `lint(step)`; its diagnostics are in the build. |
| 41 | M1 | Same, with a script that has no `lint` function; then one that raises | `protocol.failed` both times, the log naming the missing function, then holding the traceback. |
| 42 | M1 | A world of two layers; a failed `attr` node in layer 2; a plain lint in layer 1 and one in layer 2 | Both `skipped`: each reads the finished build (R29, `graph.md` case "lint gate"). |
| 43 | M2 | A plain generate step not in `trust.toml`, no `gen/`; then trusted, and one file in its code folder edited | `protocol.untrusted`, exit status 1, the program never started; both times. |
| 44 | M2 | Case 39 as a plain command | All three calls ran. |
| 45 | M2 | `for_each = "tag:x"`; then `kind:x` with no `x` feature | `protocol.for_each_form`; then `graph.for_each_empty` and the step did not run. |
| 46 | M2 | Two items of one `for_each` returning one key | `identity.key_collision` naming both items. |
| 47 | M2 | A worker reading `height` only through `derive.route`; a row on `height` below its layer edited; then a feature above its layer moved | Ran; then reused. |
| 48 | M2 | A trusted generate step; `via step commit`; then an untrusted copy of the world built | `gen/<id>/` holds `document.toml` and canonical `result.json`; the copy builds with `protocol.untrusted_result` and the first build's `id`. |
| 49 | M2 | `via step commit` on a world with an error | `protocol.commit_refused`, exit 1, `gen/` unchanged. |
| 50 | M2 | A returned feature with an unknown attribute, its step in a layer whose `owner` is `land` | `doc.unknown_key` at its line in `step:<id>/document.toml`, the chain naming the layer and `land`. |
| 51 | M2 | A step returning `[field.height]` with `file`, where `height` exists | `field.step_base`. |
| 52 | M2 | A worker lint reading `pack.query` of `@plant/tree`, columns `bounds` and `collision`; then the pack's `colour` of one tree edited; then its `collision` | Columns as `via pack list` gives them; the lint reused; then ran. |

## Changed from the spike

| Spike (SPEC 12.2) | v1 | Why |
|---|---|---|
| `[[step]]` with `id`, `at`, `run`, `after` | `[step.<id>]` with `when`, `command`; its layer is where it is declared | R1, vocabulary section 6, R21. |
| `python` meant the compiler's interpreter | A named program from the user's config; never `PATH` | One interpreter, chosen once, the same on Windows. |
| Files only, the whole build folder handed over | Two transports; a worker reads what it asks for | R22, R23. |
| `.f32` rasters with no header | `build.md` 3.2's `.npy` field files, named by digest in the store | R20; orientation and size travel with the bytes. |
| `layer.json`, JSON or TOML shaped; FINDINGS asked for JSON | `document.toml`, TOML only, read by the authored-text reader | Principle 8: lines for diagnostics; validation before use. |
| A returned field applied lower effects again | Rows; a field of its own, never one that exists | R21, R32; SPEC 12.8. |
| Feature attributes only with `[export] feature_attrs`; kind defaults missing | Every attribute, resolved (R19b) | No private door; FINDINGS. |
| One hash of inputs, one slot | `graph.md` 5.1's static key and traces | SPEC 12.8. |
| `step.no_output` | `protocol.no_output` | One group per chapter. |
| Codes `<id>:<code>`, unregistered | `step.<id>.<code>`, with `explain` | `diagnostics.md`. |
| A world's steps ran unasked | Trust by digest | Blackbox, Open. |
| Lint steps always ran | Reused like any step (`graph.md` 5.1), and skipped only when they read a failed node | Same text, same work; `diagnostics.md` 4. |

## Open

| Question | Decided by |
|---|---|
| Whether a lint step's report files belong in the build directory. | R15's first library lint in M1, with `build.md`. |
| The deadlines' values. | The M0 probe and M2's `basin` (a 3.5 s model). |
| Whether a plain command may declare what it reads, to be handed less. | The first plain step whose request is too large to write; not before M2. |
| Shipping `viasdk` outside the engine (PyPI), and whether a project's virtual environment may serve as a named program. M1's route is decided (8.1). | M2 packaging, with the trust rule unchanged. |
| A grandchild that leaves the step's group, and grandchildren and plain commands when the engine dies on Linux and macOS: a subreaper (`PR_SET_CHILD_SUBREAPER`) or a cgroup on Linux; nothing yet on macOS. | Case 30 in the M0 probe that kills the engine. |
| Windows behaviour (Job Object, binary stdio, mapping, no-replace move). | Hosted runners, once a push is allowed (plan, "Comes back to you later"). |
| Waivers for lint findings (`diagnostics.md`, Severity). | The first library lint whose finding a world must accept at one place; M3. |
| A fresh process per call. | The first case where carried state matters. |
| A step in a frame other than the world's. | Imports, M4. |

## Owed by other chapters

Asked of the lead, so the chapters agree:

| Chapter | What |
|---|---|
| `doctrine.md` | R7: whether "moved" holds for workers only. (R32 settled the rest.) |
| `graph.md` 5.1, 5.3 | `checks` and the request's `checks` member in a lint's static key; 5.3 is M2, with `for_each`'s one form (1.2). |
| `build.md` 14 | `via step commit` and `protocol.commit_refused` (7.1). |
| `graph.md` 4 | That a selector filters to the stratum and a name is refused (3.3). |
| `graph.md` 5.1 | The canonical form of 4.4; the CRLF and ignore rules of section 7 for command files; with `--sdk`, the SDK folder's digest in every step's static key (8.1). |
| `build.md` 14 | `via trust`, `via config program`, `--step-deadline`, `--sdk DIR`; step logs, call folders and `<store>/sdk/` in the store. |
| `vocabulary.md` | `message` in the diagnostic's members; `item` noted in Colliding names (the request says `for_each_item`). |
| `diagnostics.md` 6 | `code` in place of `name` for a step diagnostic's tail. A `diagnostics.json` member not listed in section 5, or of another JSON type, is `diag.malformed`. |
| `graph.md` Conformance | Cases "step reuse" and "step rerun" use a plain generate step in M1; generate steps are M2 here and in `fields.md`. |
| `conformance.md` 5.3 | Compare each record less `message` and `hint` only: `data.log_tail` is gone (6.2). |
| `numbers.md` 10 | Vectors for `stream(seed, ["step", id])` and `stream(seed, ["step", id, item])`. |
