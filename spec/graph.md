# The build graph

A build is one graph: every statement of the merged document becomes nodes, and every reference
between them is an edge. This chapter says what the nodes and edges are, how a node's result is
keyed and stored by content so that a rebuild runs only what read a change, and what a run reports.

Words are from `spec/vocabulary.md`. The merged document and its canonical JSON are `documents.md`;
posts, windows, halos and the one maths module are `numbers.md`; views of a field are `fields.md`
6.2 and `authority.md` 4.4; produced names, hand edits and the change report are `identity.md`; the
step request, its two transports and trust are `protocol.md`; the build directory, its chunks, its
file names and `.previous` are `build.md`; poisoning and `diag.skipped` are `diagnostics.md` 4.

## 1. Statements and nodes

A **statement** is one table of the merged document (`documents.md`): a field, a `then` row, a kind
and each of its effects, a feature, an instance, a set, run or scatter, a pin, clear or lock, a step,
a `let` name, an import, a `given`, and each asset and material of the pack.

A statement's **digest** is the digest (3.1) of its canonical JSON (`documents.md`) together with its
table path (family and id) and, for a statement of a layer, that layer's rank. It holds no file,
line or other source position. Kinds and packs have no rank (`documents.md` 5.1). So moving a table
within its file, or to another file of the same rank, changes no digest; moving it to a layer of
another rank does, since layer rank is one of the keys of row order (R29).

The world file's top-level keys (`documents.md` 1) make two statements. The **frame**, the
`world` statement, is `extent`, `resolution`, `seed` and `chunk`: the keys that decide geometry
and numerics; every node that works on posts, windows or the lattice is made from it. `pack` is
the statement `world.pack`, made into the nodes that resolve asset names (a hand instance, a pin,
a scatter's rule); an asset's facts are each `asset:<id>` statement. `name` and `include` enter
no node's key: the lookups and sources they decide are in the build key (6.2).

A `let` makes no node. It is inlined where it is read (`formulas.md` 5): its digest enters the key
of every node whose formula reads it, as a statement that node is made from (3.1, item 3).

A **node** is one unit of work with one result, its **value**. A node reads only its inputs, which
arrive along its edges; the graph crate hands a node its inputs and nothing else. A node id is
`<family>:<subject>`, then `/v:<view>`, `/w:<window>` and `/i:<item>` where the family has them; a
family with one node (`merge`) is its own id. A subject may hold `/` and `:` (a path); only those
three suffixes are read as parts of the id.

| Family | Node id | Made from | Value | Kept in the store |
|---|---|---|---|---|
| `source` | `source:<path>` | each text file read: world, layer, change file, kinds file, pack | the file's digest | digest only |
| `input` | `input:<path>` | each declared outside file or folder: a field's `file`, a step's `inputs`, an asset file the pack names | its digest (5.1 for a folder) | digest only |
| `merge` | `merge` | the sources and the requested changes | the merged document, each statement's digest, each key's span | yes |
| `attr` | `attr:<id>` | a feature, instance or rule | its attributes: kind defaults and attribute formulas resolved | digest only |
| `select` | `select:<selector>/v:<view>` | each selector read (`kind:district`, `role:surface`), with the statements that decide what it matches: every field's for a role or tag, every feature's for a kind | the matching ids, in id order | digest only |
| `shape` | `shape:<id>/v:<view>` | a feature, `where` regions included | its geometry on the integer lattice (`geometry.md`) | digest only |
| `instance` | `instance:<id>/v:<view>` | a hand instance | its plan: origin in plan, facing, asset | digest only |
| `rule` | `rule:<id>/v:<view>` | a set, run or scatter | its output before hand edits: instances in plan, with produced names | digest only |
| `edit` | `edit:<id>/v:<view>` | a pin, clear or lock | the place it resolved to; for a pin, its instance in plan | digest only |
| `held` | `held:<producer>/v:<view>` | each rule or step that places instances | its output with the hand edits of `identity.md` 4.1 applied, as the view sees them | digest only |
| `footprint` | `footprint:<instance name>/v:<view>` | each instance with `ground` | its footprint feature's geometry (`rules.md` 2.1) | digest only |
| `height` | `height:<owner>/v:<view>` | each hand instance, pin and `held` node | the height of each of its instances from the surface beneath (R25), and the relief and `keep_out` checks (`rules.md` 8) | digest only |
| `overlap` | `overlap:<instance>/v:<view>` | each hand instance, when the world has two or more | the overlap test of it against every other hand instance (`rules.md` 8.1), its `place.overlap`; it reads the instance's inputs, its `height` and every other hand instance's `instance`. Decided in M5 slice 3: no `height` reads it, so two hosts with a part on a mount each, however far apart, close no loop through the test (the test reads plans only, `rules.md` 8.1) | digest only |
| `field` | `field:<name>/v:<view>/w:<window>` | a field, in one view, over one window | its samples | yes |
| `distance` | `distance:<id or selector>/v:<view>/w:<window>` | a feature or selection that `d`, `sd` or a distance function reads | the distance and signed distance at each post | yes |
| `measure` | `measure:<field>/v:<view>/<feature>.<effect>/<call>` | each `under` or `smoothed_along` call (`formulas.md` 8.5), per feature | the measure, or the smoothed values along the path | yes |
| `op` | `op:<operation>/<digest>` | an engine operation (R23): a network with its areas, cells along a frontage, a route over a cost field | its geometry and place keys | yes |
| `made` | `made:<produced name>/v:<view>` | each area or cell a network or cells table gives a kind (`geometry.md` 8.1), reading its `op` | its kind and region: what a field chunk, a distance or a selector member reads of it, so a chunk its window misses keeps its key when another part of the maker moves | digest only |
| `step` | `step:<id>`, and `step:<id>/i:<item>` under `for_each` | a step; one call per item | its returned document, field files and diagnostics, and its trace (5.1) | yes |
| `facts` | `facts:<asset id>` | each model asset the build uses (M2, `assets.md` 4) | the comparison of the facts file's entry with the asset's measured facts and its pack statement, and its diagnostics; it reads the placements that can hold the asset, for `asset.revised`'s list | digest only |
| `mesh` | `mesh:<owner>` | each generated mesh: a kind surface or a triangulated region (`surfaces.md` 5, `build.md` 10); the terrain is written as a field, not a mesh | the mesh | yes |
| `budget` | `budget:chunks` | the world's `[budget]`, when written (`build.md` 5.1); it reads what the instance columns read, takes the chunk side as a parameter, and holds the `world`, pack and `budget.*` statements | no bytes; its `build.over_budget` and `build.waiver_unused` records | digest only |
| `write` | `write:<build file>` | each file of the build directory (`build.md` 2) | the file's bytes | yes |

- **Paths** in `source` and `input` ids are relative to the world file's folder and use `/`; a file
  above that folder keeps its leading `..` (`../shared/h.npy`), so two files never share an id. A file
  inside a library is `lib:<library>/<path>`, inside the pack `pack:<path>`, with the path relative
  to the library's or the pack's folder. So ids are equal on every machine.
- **Selectors** are written in their canonical form as `documents.md`'s canonical JSON prints them.
- **Views.** `<view>` is `build` for the build's own view; `<layer>` for the view of a generate step
  declared in that layer (section 4); `<layer>.head` and `<layer>.full` for a field that step reads
  in its head or full view (`authority.md` 4.4); `head` for the build's head view of a field: its
  base and every `then` row of every layer, no effect (`authority.md` 2.3), which a `shape` reads
  (a `where` region, a derived feature); `build-nofootprint` for the build's view less the entries of every
  footprint feature, which the `keep_out` test reads (`rules.md` 8.3, `fields.md` 6.2). Nodes in a
  view other than `build` are made only when something reads them.
- **Windows.** `<window>` is `<i0>.<j0>.<i1>.<j1>` in posts, with `i1` and `j1` excluded, as
  `numbers.md` 6 defines a window. The build's own `field` and `distance` nodes are cut by the
  build's chunks (`build.md` 5), one node per chunk. A chunk includes its edge posts, which it
  shares with its neighbours, so chunk `(cx, cy)` with `k` posts per side has the window
  `<cx*k>.<cy*k>.<min((cx+1)*k, nx-1)+1>.<min((cy+1)*k, ny-1)+1>`.
- A node marked "digest only" has its key and its value's digest in the store, and its value is
  computed again when a consumer needs it. The line between kept and digest only is ours: kept is
  what is costly or crosses a process boundary. It is fixed per build format version.
- Rasters are kept at **cut points**: the boundaries of `field`, `distance` and `measure` nodes. A
  raster inside a node (a field's base before its rows, one effect's mask) is never a node.
- A core check belongs to the node that finds it: an overlap is in its `held` node's value.
- Node ids sort bytewise. Every order this chapter gives that no key settles is by node id.

## 2. Edges

An edge runs from a reader to a node it reads. Every reference is an edge, and nothing else orders
work. `rules.md` 3.4 lists the same edges the other way round: its "Edge from" column is what is
read, and its "Edge to" column the reader.

Statements are never edge targets. A statement enters a key only as a digest (3.1, item 3). No node
reads `merge` as an edge except `write:diagnostics.json` and `write:changes.json`, which need its
spans; the engine reads `merge` to build the graph.

| Reference | Written as | Edge from the reader to |
|---|---|---|
| placement | `on.ref`, `starts_at`, `ends_at`, a name in `path` (`derived.md` 3), `follows` | the named thing's `shape`, `instance`, `held` or `edit` |
| name attribute | an attribute listed in its kind's `names` (`authority.md` 1.2) | from the feature's `attr` to the named thing's `shape`, `instance`, `held` or `edit` |
| ends | `starts_at` / `ends_at` naming a path's end or an exit | that path's `shape` |
| selection by place | `{ of, near }` (`identity.md` 3) | the maker's `held`, `op` or `step`; a hand edit's own selection reads the maker's `rule` or `step` (`identity.md` 4.1) |
| step output | a `kind:` selector over features a step returned (`formulas.md` 7); a `tag:` selector resolving to a field a step declared is (open: needs ruling, R21; `fields.md` 7); never a produced name in text (R3) | that step's `step:<id>` node, or its `held` node for an instance |
| mount | a place name through an instance (`cottage.door`); a path's `starts_at` or `ends_at` naming the instance itself meets its default mount (R31) | the instance's `instance` or `held`, and its `height` |
| formula read | a field, attribute, feature or selector a formula names (`formulas.md` reports the names) | the field's nodes in the reader's view (in a `shape`, view `head`) over every chunk the reader's window, grown by the function's halo (`slope`: 1 post), meets, and with no halo the chunk of the reader's own window alone, which holds every post read (a neighbour shares only posts it holds, `build.md` 5); `attr`; `distance`; `select` |
| measure | `under`, `smoothed_along` | the `measure` node. Its own edges: `under(field, stat)` reads that field's nodes over every chunk the feature meets; `under(stat)` and `smoothed_along` read the written field's entries before this effect, computed inside the node and never kept, so its edges are those a `field` node over those chunks would have for those entries only |
| effect | a kind's effect with `field = "role:..."` or `"tag:..."` | from each written `field` node to the feature's `distance` over that chunk and its `attr`, for each feature whose effect window (`fields.md` 5.3) meets the chunk |
| field rows from a step | rows a step returned | from each `field` node of that field to the `step` |
| ground | `ground` on an instance | from `footprint` to the instance's `instance` or `held`; the footprint's kind's effects then read the `footprint` as a feature |
| standing | every placed instance (R25, `rules.md` 6, 8) | from `height` to the instance's plan, to `select:role:surface` and the surface fields' nodes under its footprint, and to `select:role:keep_out` and the `keep_out` fields' nodes under its footprint in view `build-nofootprint` (`rules.md` 8.3) |
| scatter | any scatter (`rules.md` 3.4, 7) | `select:role:surface`, `select:role:keep_out` and those fields' nodes; the `instance` of every hand instance whose solidity is not `none`, in its layer or below, that does not stand on its output; and every non-scattered `held` in its layer or below; `within`'s `shape`; `clump.around`'s `held` |
| hand edits | a producer's `by` (`identity.md` 4.1) | from `held` to the producer's `rule` or `step`, and to each `edit` whose `by` names it |
| set | `set` on an instance or run | the set and its items |
| step, declared | `inputs`, `for_each` | `input`, `select` |
| step, recorded | each read the step made (5.1) | the node that answered it |
| write | a build file | the nodes whose values it holds; `write:manifest.json` also reads every other `write`, `source` and `input` |
| give (M4) | `give = { <name> = "<ours>" }` | the parent's node it names, from the import's `given` node (`composition.md` 12) |

An edge's **label** names the key that made it and, where one reader has several edges from one key,
the statement it reads through: `on.ref`, `base reads b`, `effect lane_a.bed`. A label is part of
the reader's key (3.1), so two edges to one node are two inputs.

Direction between layers is checked by `documents.md` 5.1 on these same edges. The doctrine's build
order (section 3: fields from their bases, then features by authority, then rules, then checks) is
the order these edges usually give. The engine enforces no phase.

### 2.1 Two graphs

The graph is known in two stages.

- **The reference graph** is known once `merge` has run, before any other node. Its vertices are the
  node ids with `/v:`, `/w:` and `/i:` removed, except that a field's `head` view is a vertex of
  its own, plus one vertex per `let` name. Its edges are the references of the table above between
  those vertices. A head vertex has only the edges of the field's base and `then` rows, each field
  they read again by its head vertex (`authority.md` 2.3). Which features' effects write which field
  is known from kinds and roles. A `kind:` selector read above a generate step's layer is an edge
  to that step, which may return what it matches (section 4); a `tag:` selector likewise is
  (open: needs ruling, R21). A step has no edges to its
  recorded reads.
- **The node graph** is expanded as the values it depends on become known: a field's chunk nodes
  and their edges once the `shape` values of the features that write it are known, `for_each`
  calls once their `select` is known, edges to a step's output once the step has run. This is a
  dynamic dependency in Shake's terms (Mitchell, 2012), a monadic build in "Build systems à la
  carte" (Mokhov, Mitchell and Peyton Jones, 2018).

Edges are in the key (3.1), so an edge added or removed changes the reader's key: a feature that
moves into a chunk adds an edge to that chunk's node, and one that moves out removes one.

Every cycle among nodes is a cycle in the reference graph, since each node edge is a reference edge
with view, window and item removed. Three kinds of read cannot close a cycle through an effect: a
step's recorded reads, because a step reads only its view (section 4); `under(stat)`,
`smoothed_along` and `v`, because they read the entries before their own; and a shape's read of a
head view, because no effect is in it. A loop through a head vertex's base or `then` rows is still
`graph.cycle`. A reference cycle that chunks
alone would break is still a cycle: what the text means may not depend on where chunks fall.

### 2.2 One cycle report

Cycles are found on the reference graph by Tarjan's algorithm (Tarjan, 1972), before any other node
runs. Each strongly connected component with more than one vertex, or a vertex with an edge to
itself, is one `graph.cycle` error. Its members are statements: the vertices of one statement are
one member.

The diagnostic names one loop in **source order**:

1. Its first member is the member whose statement comes first by layer rank, then file path
   bytewise, then line. A statement with no rank (a kind, the pack) comes after every ranked one,
   by file and then line. A statement a change wrote has the rank of the layer `documents.md`
   merges it into.
2. The loop is the shortest cycle through that member, found breadth first along edges; ties go to
   the next member first in source order.
3. Each line of the loop is a member: its statement, and the file and line of the key that makes
   the edge to the next member, which it names. Two edges with different labels between one pair
   print the lowest label.
4. Members of the component outside that loop are listed after it, in source order.

The subject is the first member. The hint names every key in the loop and says that removing any
one breaks this loop; when the component has other members, it says they hold other loops. This
one code replaces every per-mechanism cycle message: fields that read each other, a placement on a
thing whose shape reads the placed thing's footprint, a `where` that reads its own feature.

Every member's nodes fail as one group under that `graph.cycle`; their readers are `skipped`, under
one `diag.skipped` (`diagnostics.md` 4). The rest of the build runs and is stored, so a later build
reuses it. With an error, nothing but `diagnostics.json` and `.previous` is written to the build
directory (`build.md` 13).

## 3. Keys and the store

### 3.1 The key

A node's **key** is the digest of a canonical encoding of:

1. the four versions (3.3);
2. the node's family;
3. the digests of the statements it is made from, kind, pack and inlined `let` statements included;
4. its own parameters: window, item, operation arguments, path;
5. for each input edge: its label and the input's value digest.

**Leaves.** A `source` or `input` node has no statement. Its parameter is its path, and the file's
(or folder's) digest enters as an edge labelled `bytes`. So a changed file gives a new key, and the
node's status follows the rules of 6.1 like any other.

**What is not in a key.** Node ids are not, nor are input node ids: two nodes with equal keys have
one value. A view's name is not in any key, only what the view holds: two views with the same
entries are two ids, one key and one evaluation. Source positions are not: moving a table, adding a
comment or a blank line changes no key below `merge`.

**Encoding.** All integers are unsigned 64-bit little-endian. A string is its byte length, then its
UTF-8 bytes. A digest is its 32 raw bytes. A list is its count, then its members.

```
"via-key" · document language · protocol · build format · epoch (integer)
· family
· statement digests, sorted bytewise
· parameters as (name, value) strings, sorted by name
· edges as (label, value digest), sorted by label, then digest
```

Test vector, with test versions `test-doc`, `via-step-1`, `via-build-0`, epoch 1: the node
`source:world.toml` whose file digest is 32 bytes of `0x11` encodes to these 189 bytes, whose
digest is recorded in `spec/vectors/graph/keys.toml` with the conformance runner (M0):

```
07000000000000007669612d6b65790800000000000000746573742d646f630a000000000000007669612d737465702d31
0b000000000000007669612d6275696c642d3001000000000000000600000000000000736f7572636500000000000000
0001000000000000000400000000000000706174680a00000000000000776f726c642e746f6d6c010000000000000005
0000000000000062797465731111111111111111111111111111111111111111111111111111111111111111
```

The **digest** is BLAKE3 at 256 bits (O'Connor, Aumasson, Neves and Wilcox-O'Hearn, 2020), written
as 64 lowercase hex digits. Field files (R20) are named by the same digest (`build.md` 3.3).

Keys are computed from value digests, so a node that runs again and gives the same value leaves
every key above it unchanged: its readers are reused. This is early cutoff, from Shake (Mitchell,
2012); a store of values keyed by their inputs' digests is a constructive trace in the terms of
"Build systems à la carte".

### 3.2 The store

The store is a folder of write-once files named by digest. Where it lives is `build.md` 13
(`.via/store/` beside the world file, or `--store`). `ab` below is the name's first two hex digits.

| Entry | Path | Holds |
|---|---|---|
| object | `objects/ab/<value digest>` | the value's bytes; for a value with diagnostics, `"via-value-1"`, the bytes and the diagnostics in the encoding of 3.1, so the value digest covers them (3.4) and a build file's object is the file itself |
| key record | `keys/ab/<key>` | the value digest and the node's diagnostics |
| trace | `traces/<static key>/<trace digest>` | one trace of a step (5.1) and its value digest |
| build record | `builds/<build key>` | 6.2 |
| memo | `keys/ab/<memo key>` | a key record whose value digest is a read's answer or a plain command's request (5.1, Memo) |

- A file is written to a temporary name in the store and renamed into place without replacing, so a
  reader never sees half a file and nothing is renamed over a file another process has open. If the
  target exists, the temporary file is deleted and the existing file kept: the first writer wins.
  The exceptions are a trace entry a `--rerun-step` replaces (5.2) and a build record, which a
  later build with the same key replaces (6.2).
  Equal names hold equal bytes, so one store may serve many builds and build directories at once.
- An object is checked against its name when first read in a build. A mismatch, or a file that
  cannot be read, is `graph.store_corrupt`: the entry is dropped and its node runs again.
  The spike's cache trusted its outputs (`spike/FINDINGS.md`); this check is the answer.
- An entry written once is not forced to disk before it is renamed into place. A crash may leave
  one torn; the check above drops it on first read and its node runs again, so the store is never
  wrong, only colder. A flush per entry cost more than the work it kept (M2, `vale`).
- A store that cannot be written (no permission, a full disk) is `graph.store_unavailable`: the
  build goes on and keeps nothing more.
- Key records written behind (ours, M6): key records and memos are written by a few writer threads
  of the store, not by the thread that settles nodes, and a read of a key the build queued answers
  from the queue, so a build sees its own records at once. The queue is emptied before a build
  record is written (a build record names a build whose keys are all there), before the run's
  store diagnostics are taken and when the build ends. Objects and traces are written in place,
  since a step or a worker may read them by path. Why: one file creation per settled node, on the
  coordinating thread, was about 15 s of city's change build (M6 slice 1, `sample` of
  move_avenue; protocol in `examples/city/expect.toml`, M6 slice 2). A crash
  loses at most queued records, which only makes the store colder. Case
  `store::key_records_are_written_behind` (crate test).
- Deleting the store is always safe: the next build runs everything and gives the same bytes
  (section 7).
- The store keeps every key it has seen, so building with a change, without it, and with it again
  reuses each. The spike's step record was one slot and re-ran every time.

### 3.3 Versions

Every key holds four versions. A new value of any one changes every key, so nothing is reused.

| Version | What it names | Where it is fixed |
|---|---|---|
| document language | the meaning of the text: keys, forms, merge rules | `documents.md` 2.3: `via-doc-1` |
| protocol | the step request and reply, `via-step-1` | `protocol.md` |
| build format | the layout and encoding of build files and store values | `build.md` |
| numerics epoch | an integer, raised by any engine change that may change the bytes of any value for the same inputs | here |

The numerics epoch covers more than maths: a new pin of the maths library (`numbers.md` 8), a fixed
bug in scatter, a new tie rule. An engine release that changes none of these keeps the epoch and
reuses the store. The engine's version string is in no key except one: it is a parameter of
`write:manifest.json`, which records it (`build.md` 4), so a new release rewrites the manifest and
nothing else.

### 3.4 Diagnostics are part of the value

A node's diagnostics are part of its value and its value digest. On a hit they are replayed as if
the node had run, so an unchanged rebuild writes the same `diagnostics.json`.

- A stored diagnostic holds its subject and site (`diagnostics.md` 1), never a file or line. Its
  location and `chain` are filled in when it is emitted, from the current merge's spans. A table
  moved within its file is reported at its new line with nothing below `merge` run.
- A subject is a statement's id or a name made from one. Since statement digests hold table paths
  (section 1), two nodes with equal keys are made from the same statements and carry the same
  subjects. An `op` value holds place keys, never a producer's name: the reader forms
  `<producer>#<key>` (`identity.md` 2.4), so two producers that ask one operation share its value.
- **Only the build reports.** A node in a view other than `build` keeps its diagnostics in its value
  and emits its errors only, since an error fails its reader. Warnings and infos come from the
  build's own nodes, once (`fields.md` 6.1). The spike printed a step's diagnostics twice (TODO).
- **Graph diagnostics** (`graph.cycle`, `diag.skipped`) are worked out on each run from the
  reference graph and the statuses, and are in no node value. They are a function of the text, so
  they replay the same.
- **Run diagnostics** (`graph.store_corrupt`, `graph.store_unavailable`, `graph.build_file_restored`,
  `graph.step_unstable`, `graph.busy`, `protocol.slow_exit`) are about the run, not the world. They
  are printed and kept in the run report (6.3), never in `diagnostics.json`. A worker killed past
  its exit deadline (`protocol.md` 6.2) is a matter of this run's timing, and a reused build starts
  no worker, so `protocol.slow_exit` could not replay.

A node whose value holds an error is stored like any other and replays its error. Its readers are
`skipped`: they do not run and raise nothing of their own, and `diag.skipped` counts them
(`diagnostics.md` 4 calls them poisoned; `skipped` is the one word). A step that did not complete
(`protocol.md`: it failed, was killed or returned nothing readable) is not stored and runs again next
build.

An error the static checks find in a statement (`diagnostics.md` 4, stage 2) fails that statement's
node before anything runs, whatever its inputs: its value is the error, and its key holds its
statements, parameters and the error, no edge. So it is reported beside an independent failure
upstream, which would otherwise have skipped it.

A field's diagnostics are found over the whole extent, once (`fields.md` 6.1). Each belongs to the
chunk node whose window holds its position, the first in chunk order, or, with no position, to
every chunk node of the field; the build reports it once.

## 4. Strata and views

A step declared in layer L reads its **stratum**: the statements of the layers at or below L, and
the outputs of steps in layers below L. It does not see the outputs of steps in L or above, nor any
statement that names one of them, directly or through others (found on the reference graph). So
two steps in one layer do not see each other's outputs, may run side by side, and nothing orders
them. A step that needs another's output is declared a layer above it. A bookmark in L on a course
the step in L returns is legal: it is outside that step's stratum and built after it.

The step's **view** `<L>` is its stratum as nodes: every node it reads carries `/v:<L>` and is built
from the stratum only. Which entries of a field the view holds is `authority.md` 4.4: the step names
the view in each read of a field, the head view (`/v:<L>.head`) or the full view (`/v:<L>.full`,
the default). Both hold nothing a step at L or above returns. That rule and this chapter's stratum
are one definition; neither adds to the other. Other nodes in view `<L>` read fields in the full
view. (open) Whether they should read a field the step returns rows on in its head view; decided by
M2's `basin`, the first step that returns rows.

A step's field reads are assembled from chunk nodes of its view, whatever window it asks for. So a
view whose entries equal the build's shares keys with the build's nodes chunk by chunk, and costs
nothing. A view that differs costs one evaluation per chunk it reads, per distinct view (R21).

A worker that asks for anything outside its stratum (a feature of a higher layer) is refused with
`graph.read_outside`, naming the thing and the lowest layer a step would need to stand in to read
it (M2).

Generate steps run layer by layer: each layer's steps over the world joined so far, their returns
joined before the next layer's steps read (`protocol.md` 5). The final graph then reuses each of
them, since a stratum's answers do not change when a higher layer joins. A worker that the final
graph runs again and that gives another value is `diag.internal` naming the step's node, never
passed: the world joined the first value, so the build would show one value beside the other's
diagnostics (M3). A plain command is handed the build's files, not its stratum's (`authority.md`
4.4), so it may run again in the final graph; that is owed with its view.

A lint or export step reads the finished build (R29), whatever layer declares it. It runs after
every generate step, and every node it reads is in view `build`, so its reads share keys with the
build's nodes and cost nothing. Which of them run after an error is `diagnostics.md` 4: a lint that
reads a failed node is `skipped`; export steps and `write` nodes are `skipped` when the build has any
error.

A plain command is handed its whole view (`protocol.md` 2.2), so a plain lint reads every field,
feature and instance of the build, and any failed node skips every plain lint. Telling lints apart
by what each reads needs workers, whose reads are traced (M2): a worker lint learns its reads only
by running, so when a read it asks reaches a failed or skipped node, the read is not answered, the
call is cancelled, and the lint is `skipped` with its findings dropped.

**When a step starts.** A plain command (M1) starts once every value in its request is computed. A
worker (M2) starts once its stratum's statements are known; each read it makes is computed when
asked, on the thread the step holds, so a worker never waits for a free thread, and at `--threads 1`
its reads run inline.

## 5. Steps in the graph

### 5.1 Static key and trace

A step's result depends on what it read, which for a worker is known only after it runs. A step
node has two parts.

- **Static key.** The digest of: the four versions (3.3); the step's statement digest (`command`,
  `params`, `worker`, `when`, its id and rank); its trust digest (`protocol.md` 7: `command` and
  every file of its code folder, so `lib:` programs and a script's sibling modules are covered);
  for a `command` of one word naming a configured program (`via config program`, such as the terrain
  library's `["via-terrain"]`), the digest of that program's file, since the program is then the
  step's whole code and a revised one must run the step again; the
  frame (the `world` statement, section 1: `extent`, `resolution`, `seed`, `chunk`); the ids of the world's statements, by which the diagnostics it
  returns are mapped (`protocol.md` 5); its seed; under `for_each`, the item's id and statement digest; the digest of each declared
  input. For a plain command, also the request with its absolute paths left out: blobs are named by
  digest, so the request still names every field it hands over. This is the one definition;
  `protocol.md` 4.4 points here.
- **Trace.** The reads the step made, in the order made, each as the request and the digest of the
  answer. For a worker, a read is one engine call (R23), hand edits handed to it included. A call
  the engine refuses (`-32602`) is a read too: its answer is the digest of the refusal (its code,
  message with every absolute path in it left out, and data), since a step that catches it and goes on read that the call failed. A plain
  command's trace is empty: everything it reads is in its static key.

An input that is a folder (`protocol.md` 1) is digested as the sorted list, bytewise, of every file
under it as (path relative to the folder with `/`, file digest). A symbolic link is entered as its
target's text, not followed. A path that does not exist is `graph.input_missing` (error), raised on
the step; the step does not run and its readers are `skipped`.

**Reuse.** The traces under the static key are tried newest first: each recorded read is answered
again from the current graph, and if every answer has the recorded digest, the stored value is the
step's value. Answering a read computes only what it names, through the store, so checking a trace
costs what the step read and nothing more. When no trace matches, the step runs and its trace is
added. This is a verifying trace (Mokhov, Mitchell and Peyton Jones, 2018), keyed by content rather
than by time. It replaces the spike's one-slot hash of "the whole build below" (`spike/SPEC.md`
12.2, 12.8). An untrusted step built from its committed record (`protocol.md` 7, `gen/<step>/`)
takes that record as its value, with status `reused`.

**Memo.** A read's answer is kept in the store as a key record under its memo key: the digest of
the build key (6.2); every `source` node's text and `input` node's bytes in the graph that answers
it (a stratum's joined world, section 4, has its own); the step node's id; what failed or was
skipped (`diagnostics.md` 4), for a traced lint; and the read's method and request. A plain
command's request, the part of its static key above, is kept the same way without the read. Each
answer a completed run was given is kept as it is made. A check, or a static key, whose memo key is
stored takes the digest from it and makes nothing else: an unchanged world answers every trace
without building. Every answer and request is a function of those parts (the model's build is made
from the sources, the inputs and the flags of the build key alone), so a kept digest is the one
making it would give, and section 7 holds. Ours: the M3 engine made the whole build per stratum to
answer any read, so an unchanged rebuild cost 33% to 53% of a cold one (TODO, "A change costs a
cold build"). A changed world still makes the whole build for the reads its memo keys miss; reading
only what a read names is owed (Open).

What the engine cannot see stays the step's to declare: a file opened directly, the clock, the
network, an interpreter or package installed on the machine. Such a read goes stale unseen, as R26
accepts: a user's step is recorded, not promised.

### 5.2 Forcing a step, and unstable steps

`--rerun-step ID` (repeatable) runs that step even when a trace matches, and compares: a value whose
digest differs from the stored one for the same trace is `graph.step_unstable`, naming both digests,
and the new value is used. It is the way out when something the engine cannot see has changed, and
the one way instability is found. The new value replaces the stored trace entry of the same name,
written to a temporary name and renamed over it, as a build record is (3.2), so later
builds take it. An export step that writes other bytes on a rerun (`build.md`
11.4) is found the same way.

### 5.3 One call per item (M3)

Under `for_each`, each match in the step's view is one node `step:<id>/i:<item>`, whose static key
holds the item, so moving one item re-runs its call alone. The join node `step:<id>` reads every
call in item id order; it is what references to the step's output read, and where
`identity.key_collision` (`identity.md` 2.5) is found. A selector that matches nothing is
`graph.for_each_empty`. M1 keeps room for item ids and the join; the rest is written with M3.

## 6. Running

### 6.1 Statuses

Keys are worked out from the leaves up. For each node, in dependency order:

| Status | When |
|---|---|
| `reused` | Its key is in the store. Its value digest comes from the store; its value is read only if a reader that runs, or the expansion of the graph, needs it. |
| `recomputed` | Its key is in the store, its family is "digest only", and a reader that runs, or the expansion of the graph, needs its value. |
| `ran` | Its key is not in the store. It is computed and stored. |
| `skipped` | It reads a failed node, is a member of a cycle's group, or is gated by `diagnostics.md` 4. |

Among nodes of one build with equal keys, the one with the lowest id carries the status `ran`; the
others are `reused`. Status never depends on which thread got there first.

### 6.2 The build key, the build record and the null build

**The build key** is the digest of: the four versions; the requested changes; each `source` and
`input` path with its digest; each lookup the build made (`documents.md` 4: the name, the path
found, and each candidate before it that was absent); each step's trust digest, and whether this
machine trusts it (a lint trusted since runs); `locations.pack_root` as computed for this
directory (`build.md` 4), which is relative, so it enters by place; and `--lib`,
`--stand-ins` and `--fail-on`. Roots and lookups enter by place and source path only: each root as
`beside`, `--lib <n>`, `VIA_LIB <n>` or `shipped`, each path as its `lib:<id>/` or world-relative
source path (`documents.md` 2.1). No absolute path enters the key. A folder input's digest covers its listing, so a new file in it
changes the key. Which files are read is decided by the text of the files read before them, so
equal digests for every recorded read mean the same reads.

**An import enters by its text**, as `import <file as written>` with its import key: the digest of
its `seed` and `changes`, each of its sources by name and text digest (its change files among
them), its own leaves (pack, the files the pack binds, field files), its lookups, each of its
generate steps' id, trust digest, whether this machine trusts it, its declared inputs and, when
untrusted, its committed result, and each of its own imports' keys in turn. These are what its
build is a function of, as the parent's own are, so the key builds no import: an unchanged world
reads each import's files and runs nothing. Ours (M6 close): the key held each import's built
digest, so atlas's unchanged build spent 36 s building its thirteen imports to learn its key. What
a parent gives a fill is no part of it: that is a function of the parent's text, already in the key.

**The build record**, kept in the store under the build key, holds: the four versions (3.3); every
node's id, key and value digest; the build's world diagnostics as emitted; the digest of each file
of `files` (`build.md` 4); and whether every step completed. The manifest and the reports also
depend on where the directory is and on its baseline, so their digests are per directory, not
per key: they are read from the directory's own manifest, the one its `.previous` names in the
store, never from the record. It also holds each statement's digest by statement id, as `merge`'s value does, each
leaf's `bytes` digest, and each node's input nodes and the statements it is made from, so the
causes of 6.3 need no reading of `merge` and a chain can be found in the baseline's graph (6.4). Per-chunk field digests are
the `field` nodes' value digests, so the change report (6.4) needs nothing more. The build key is written to
the manifest as `run.key` (`build.md` 4). `.previous` (`build.md` 11.2) names the last complete build
by its `id` and manifest digest; this chapter neither reads nor writes it.

**An unchanged rebuild runs nothing.** When the build key equals `run.key` in the build directory's
current manifest (`build.md` 13), the build record for it is in the store, that record says
every step completed, and no `--against` is given, no node is touched: the
run report lists every node of the record as `reused`, and the world diagnostics are the record's.
Run diagnostics (`build.md` 11.1) are never recorded or replayed; each run computes its own, and
on a null build there are normally none, since `.previous` names the recorded build. A
build file whose bytes differ from the record (edited by hand, damaged) is written again from the
store's object, with `graph.build_file_restored`; the manifest and the reports are checked
against the directory's own manifest (above), so another directory built with the same key never
overwrites them. If the object is missing, the null build does not
apply and the build runs as usual. After a build in which a step did not complete, it never
applies. With `--against`, nodes are reused as usual, but `write:changes.json` runs against the
named baseline.

**Into a new directory.** A directory that holds no build (no `manifest.json`, no `.previous`)
has no baseline when no `--against` is given, so the manifest and the reports a build writes into
it are a function of the build key alone: `locations.pack_root` enters the key by place, and
`changes.json` without a baseline is the empty report (`build.md` 11.2). A complete build with no
baseline keeps their digests in the store beside its build record, keyed by the build key. A later
build into a directory holding no build, with that key, its record complete and every object
present, is a null build: no node is touched, the data files, the manifest, the reports and a
`.previous` naming the manifest are written from the store (swapped in as `build.md` 13 writes any
build), and the run report lists every node of the record as `reused` with `null` true. Ours (M6):
a world that imports the others (`atlas`) builds each into its own directory, and city's unchanged
build into a new directory took 16 s of planning against 1.5 s into its own.

Files are always hashed from their bytes. (open) Whether a size-and-time shortcut is allowed.

### 6.3 The run report

Every build that takes the directory's lock writes a run report beside the build directory, as
`<directory>.run.json` (`build.md` 11.3). A build refused with `graph.busy` writes none, so it
cannot overwrite the report of the build that holds the lock. The report is written last, to a
temporary name and renamed over the old one, so a reader never sees half a report. A build that is
killed leaves the previous report in place.

The report is one canonical JSON object (`documents.md` 8). A reader ignores members it does not
know; a `format` it does not know is refused. The **Compared** column says what conformance reads:
"yes" members may be asserted by a case, and must be equal between a build at 1 thread and the same
build at 8 threads from the same store (the `threads` case); "no" members are never compared,
because they differ between runs or machines. The fresh-store build of section 7 runs everything,
so its report is not compared with the incremental one.

| Member | Holds | Compared |
|---|---|---|
| `format` | `"via-run-0"`. Changes when a reader of the old format would misread the new one; added members do not change it. | yes |
| `engine` | `{ "epoch": <integer>, "version": <semver> }`: the numerics epoch (3.3) and the release of the binary. | `epoch` yes, `version` no |
| `threads` | The thread count the build used (`--threads`, section 8). | no |
| `system` | `{ "arch": …, "os": … }`, as Rust's `std::env::consts::ARCH` and `OS` give them (`aarch64`, `x86_64`; `macos`, `linux`, `windows`). | no |
| `build` | `{ "complete", "id", "key", "null", "previous" }`: `key` the build key (6.2); `id` the manifest's `id` (`build.md` 12), or `null` when no manifest was written; `complete`, whether every step completed; `null`, whether the null build applied (6.2); `previous`, the build key of the previous build (below), or `null`. | yes |
| `nodes` | One object per node of the graph, in id order. Members below. | yes, less `ms` |
| `steps` | One object per `step` node, in id order: the record R26 asks for. Members below. | yes, less `log` |
| `diagnostics` | The run diagnostics (3.4, `build.md` 11.1), each a `diagnostics.md` 1 record, in the order of `diagnostics.md` 5. World diagnostics are in `diagnostics.json` and not here. | `code`, `severity`, `subject`, `data` |
| `totals` | `{ "memo_ms", "ms", "nodes", "ran", "recomputed", "reused", "skipped" }`: the count of nodes, the count per status, the build's wall time in whole milliseconds, and `memo_ms`, the whole milliseconds the build spent evaluating the world outside any node's own inputs (a whole-world memo some nodes take their values from; 0 once every node is computed from its inputs alone, 5.1). | yes, less `ms` and `memo_ms` |

A **node** object:

| Member | Holds |
|---|---|
| `id` | The node id (section 1). |
| `family` | Its family, the part of the id before the first `:`. |
| `status` | `reused`, `recomputed`, `ran` or `skipped` (6.1). |
| `key` | Its key, 64 hex digits; `null` when `skipped` before its key could be worked out. |
| `value` | Its value digest; `null` when `skipped`, or for a step that did not complete. |
| `failed` | `true` when its value holds an error, or it is a step that did not complete; else `false`. |
| `cause` | For a `ran` node, the cause object below; else `null`. |
| `needed_by` | For a `recomputed` node, the lowest id among the nodes that read its value and ran or were recomputed (a digest-only reader computed again needs its own inputs), or `"expansion"` when only the expansion of the graph (2.1) needed it; else `null`. |
| `ms` | Whole milliseconds from its start to its value or its status being known. |

A **cause** says why a `ran` node's key was not in the store. The first rule that holds, in this
order, gives it:

| `reason` | When | Other members |
|---|---|---|
| `rerun` | `--rerun-step` named it (5.2). | |
| `no record` | There is no previous build, or its build record is not in the store. | |
| `versions` | The record's versions differ from this build's. | |
| `new node` | Its id is not in the previous record. | |
| `statements` | A statement it is made from (3.1, item 3) has another digest in the previous record's `merge` value. | |
| `edge` | An input edge's node has another value digest than in the previous record, or is not in it. The first such edge by label. A leaf's `bytes` edge, which has no node behind it, comes after its node edges and names the leaf itself. | `edge`: the label; `node`: the input's id |
| `key` | Its key differs from the previous record's for none of the reasons above: an edge removed, a parameter changed. | |
| `trace` | A step whose static key (5.1) equals the previous record's key, whose stored trace no longer matches: the first read whose answer differs now (a refused call's answer is its refusal, 5.1) in the trace the previous record names for the node (its step object's `trace`); when it names none, in the first other stored trace by name, never the one this run added. | `method`: the read's method; `request`: the digest of its normalised request |
| `not stored` | Its key equals the previous record's, but the store does not hold it: dropped as corrupt, a step that did not complete, a store that could not be written. | |

The **previous build** is the one named by `build.key` of the run report this build replaces; when
there is none, or it cannot be read, the one named by `run.key` of the directory's manifest (`build.md`
4); when there is none and the build names `--against DIR`, the one named by `DIR`'s manifest
`run.key`, the baseline (`build.md` 11.2). The report only points into the store: the record it names is checked like any store entry
(3.2). `via explain NODE WORLD` prints a node's cause and the chain of causes back to a leaf (a node
id holds `:`, a code does not).

A **step** object, one per `step:<id>` and `step:<id>/i:<item>` node:

| Member | Holds |
|---|---|
| `id` | The node id. |
| `command` | The command as the step's statement writes it (`protocol.md` 1), a list of strings. |
| `static_key` | Its static key (5.1). |
| `trace` | The digest of the trace whose value it took or that this run added; `null` for a plain command, whose trace is empty, and when it did not run or match. |
| `reads` | Each read in the order made: `{ "answer", "method", "request" }`, with `method` the engine call, and `request` and `answer` the digests of their canonical JSON. A plain command has one read, method `data`: its request without absolute paths (5.1), whose `answer` is the same digest, since what it is handed is that request. |
| `exit` | A plain command's exit status when it ran in this build; `null` when it did not run (reused, skipped, killed by a signal). A worker step's is always `null`: one worker process serves every call of the step and exits only when the build ends (`protocol.md` 3.2), after the record is written; its exit, when abnormal, is a `protocol.*` diagnostic instead. |
| `returned` | `{ "files", "value" }`: the value digest, and each file of `out` as `{ "digest", "path" }` sorted by path (`protocol.md` 5). Files a lint wrote beside its diagnostics are listed here. `null` when it returned nothing readable. |
| `log` | The digest of its log in the store (`protocol.md` 6.2), or `null`. |

A short example, one line per node for reading:

```json
{"build":{"complete":true,"id":"…","key":"…","null":false,"previous":"…"},
 "diagnostics":[],
 "engine":{"epoch":1,"version":"0.1.0"},
 "format":"via-run-0",
 "nodes":[
  {"cause":null,"family":"distance","failed":false,"id":"distance:beck/v:build/w:0.0.65.65","key":"…","ms":3,"needed_by":null,"status":"reused","value":"…"},
  {"cause":{"reason":"statements"},"family":"shape","failed":false,"id":"shape:ford/v:build","key":"…","ms":0,"needed_by":null,"status":"ran","value":"…"},
  {"cause":{"edge":"ends_at","node":"shape:ford/v:build","reason":"edge"},"family":"shape","failed":false,"id":"shape:lane/v:build","key":"…","ms":1,"needed_by":null,"status":"ran","value":"…"}],
 "steps":[],
 "system":{"arch":"aarch64","os":"macos"},
 "threads":8,
 "totals":{"memo_ms":0,"ms":412,"nodes":61,"ran":19,"recomputed":0,"reused":42,"skipped":0}}
```

The run report is about the run, not the world: it is not part of the build's bytes, and section 7
leaves it out. Times are never compared. Conformance asserts the set of `ran` nodes exactly, and
that every `recomputed` node's `needed_by` names a `ran` or `recomputed` node that reads it, or
`"expansion"`.

### 6.4 What the change report gets from the graph

The change report is `identity.md` 5's; its file is `build.md` 11.2. The graph gives it, from this
build's record and the baseline's (found by the baseline's build key):

- **A field's changed region.** Chunks whose `field` value digest differs from the baseline's are
  compared post by post. The region is, per changed chunk, the window of posts that differ. A chunk
  whose digest is unchanged is not read.
- **Causes.** For each changed thing, every statement whose digest differs from the baseline's
  `merge` and that it depends on, each with the shortest edge path from that statement to it (ties
  by node id): in this build's graph, else in the baseline's (from its record) when the dependency
  was only in the baseline, as when a change is undone.

When the baseline's build record is not in the store (`--against` a build made with another store),
whole field files are compared post by post, and causes are not given; `identity.md` says how an
entry shows that.

## 7. Clean equals incremental

For any store this engine at this epoch could have left, building a world against that store
writes the same build directory, byte for byte, and the same `diagnostics.json`, as building it
with a fresh store into a fresh directory. The run report and run diagnostics are left out.

It holds because a value is a function of its key, a key is a function of what the node read, and
a node can read nothing that is not an edge (section 2, implicit reads by role included). It can
fail only through a step whose result depends on something it did not declare (5.1). The
conformance runner checks it on every case: each case is built in its sequence, then again with a
fresh store into a fresh directory, and the two directories are compared.

## 8. Order and parallelism

- Nodes are started in a topological order of the node graph: ready nodes are taken in id order,
  onto as many threads as `--threads` allows (`build.md` 14). The thread count is in no key and no
  build file.
- The traces of the ready steps (5.1) are replayed on as many threads before those nodes are looked
  up (ours, M6): each replay reads only the store and the view, so its match is the one the lookup
  would find, and the lookup takes the matches in id order. Steps sharing a static key and steps
  forced by `--rerun-step` replay at their lookup, so a damaged trace is dropped once and in
  order. Why: replays answered one by one were about 13 s of city's change build (protocol in
  `examples/city/expect.toml`, M6 slice 2).
- Every value is a function of its inputs only. Whatever combines many values (a field's rows, the
  outputs of `for_each` calls, the instances of several producers, diagnostics) does so in an order
  fixed by keys and ids, never in the order work finished. Reductions inside a node follow
  `numbers.md` 8, rule 5.
- Steps run as separate processes, at most one per thread (section 4 for a worker's reads). A step
  writes only to its own output folder (`protocol.md`).
- One build at a time per build directory. The build takes an operating-system advisory lock
  (`flock` on macOS and Linux, `LockFileEx` on Windows) on `<directory>.lock` beside it, which the
  system releases when the process ends, killed or not. A second build is refused with
  `graph.busy`.
- Only the run report's times and the order of progress lines on the terminal may differ between
  two runs. `diagnostics.json` is sorted (`diagnostics.md` 5).

## 9. Imports and `after` (M4)

An import is a graph of its own, its node ids prefixed `<import>.`. The prefix is in no key, so one
world imported twice, or built alone, shares its results in one store; M1 keeps room for the
prefix. A prefixed id is `<import>.<family>:<subject>` (`a.step:make`, `town.step:lay/i:districts.d1`;
`<import>.<nested>.` at depth): the family stays `[a-z]+`, and the run report's `family` holds the
prefix with it (`a.step`). Correction of M4 slice 4: section 1's grammar said nothing of the prefix. A `give` may read any node of the import that does not reach the given node; reading one
that does is `graph.cycle`. So a parent may draw a fill's limits along the import's arterials or
river: a scoped name reads one node per name (`placed:<id>/<name>`), never the whole import. The
families `import`, `placed`, `brought`, `given` and `joincheck`, their edges, and what a moved import
or a moved given thing re-runs are `composition.md` 12. There is no `after`: a layer ranked above
the layer that declares an import reads the joined ground (`composition.md` 6.5). All of this is
`doc.not_yet` in M1 to M3.

## Worked example

```toml
name       = "brook"
extent     = [0, 0, 128, 128]
resolution = 1
include    = ["land", "water", "ways"]
pack       = "primitive"

[field.height]
role = "surface"
base = "= 20 + 0.05 * y"

[feature.beck]
kind      = "beck"
authority = "anchored"
path      = [[90, 128], [100, 0]]

[feature.ford]
kind = "ford"
on   = { ref = "beck", t = 0.5 }

[feature.lane]
kind    = "lane"
path    = [[0, 40]]
ends_at = "ford"

[change.downstream.feature.ford]
on = { ref = "beck", t = 0.8 }
```

At resolution 1 the default chunk is 64 m (`build.md` 5), so the 129 by 129 posts make four chunks:
`w:0.0.65.65`, `w:64.0.129.65`, `w:0.64.65.129` and `w:64.64.129.129`. Only the effects on
`role:surface` are drawn: `beck.bed`, `ford.approach` and `lane.bed` (`docs/plan.md`). The node
graph, in part:

```
shape:ford/v:build    -> shape:beck/v:build                    on.ref
shape:lane/v:build    -> shape:ford/v:build                    ends_at
distance:<f>/v:build/w:<chunk> -> shape:<f>/v:build            for f in beck, ford, lane
field:height/v:build/w:<chunk> -> distance:beck/v:build/w:<chunk>, attr:beck   effect beck.bed
field:height/v:build/w:<chunk> -> distance:ford/v:build/w:<chunk>, attr:ford   effect ford.approach
field:height/v:build/w:<chunk> -> distance:lane/v:build/w:<chunk>, attr:lane   effect lane.bed
write:fields/height.npy -> field:height/v:build/w:<each chunk>
```

Each `field` chunk has the effect edges only of features whose effect window meets it.

| Build | `ran` | Holds |
|---|---|---|
| 1. clean | every node | |
| 2. again | none | the null build: every node `reused` |
| 3. `--change downstream` (`build.md` 14) | `merge`; `attr:ford` (its statement changed; same value); `shape:ford`, `shape:lane`; their `distance` nodes and the `height` chunks that the old or new windows of their effects meet; `write:fields/height.npy`, `write:changes.json`, `write:diagnostics.json`, `write:manifest.json` | `shape:beck` and the other chunks `reused` |
| 4. without the change | `write:changes.json` and `write:manifest.json`: the report is against build 3 now | the store still holds build 1's nodes |
| 5. tables shuffled, a comment added | `source:world.toml`, `merge`, `write:changes.json`, `write:diagnostics.json`, `write:manifest.json` | statement digests unchanged, so nothing else runs; diagnostics carry new lines |

Writing `on = { ref = "lane", t = 0.5 }` on the ford closes a loop:

```
error[graph.cycle] feature.ford: references form a loop of 2 statements
  --> world.toml:18:10
  world.toml:18  feature.ford  on.ref = "lane"   -> feature.lane
  world.toml:23  feature.lane  ends_at = "ford"  -> feature.ford
  hint: remove feature.ford on.ref or feature.lane ends_at; either breaks this loop
```

## Diagnostics

| Code | Severity | Meaning |
|---|---|---|
| `graph.cycle` | error | References form a loop; names one loop in source order, each member with the key that makes its edge. |
| `graph.input_missing` | error | A step's declared input (`inputs`, `protocol.md` 1) does not exist; names the path. The step does not run (5.1). |
| `graph.read_outside` | error | A worker asked for something outside its stratum; names it and the lowest layer that could read it. (M2) |
| `graph.busy` | error | Another build holds this build directory's lock. Run diagnostic. |
| `graph.store_corrupt` | warning | A store entry did not match its name or could not be read; it was dropped and its node ran again. Run diagnostic. |
| `graph.store_unavailable` | warning | The store cannot be written; this build keeps nothing more. Run diagnostic. |
| `graph.step_unstable` | warning | Under `--rerun-step`, a step gave another value for a matching trace; names both digests. Run diagnostic. |
| `graph.for_each_empty` | info | A `for_each` selector matched nothing in the step's view; the step did not run. (M3) |
| `graph.build_file_restored` | info | A build file differed from its record and was written again. Run diagnostic. |

## Conformance

Every case is also built with a fresh store into a fresh directory, and at 1 and 8 threads, and
must give the same build directory and `diagnostics.json` (section 7). Unless said, a case is built
once clean first. `ran` sets name nodes by id; where a case names a fixture, the set is listed there,
one id per line, as the case's `expected/ran.txt` (`conformance.md` 2), derived from sections 1 and 2
for `examples/ford/` in M1; `ford_downstream.ran` is `spec/cases/graph/downstream/expected/ran.txt`,
`ford_vertex.ran` is `spec/cases/graph/vertex/expected/ran.txt`.

| Case | M | Input | Action | Must hold |
|---|---|---|---|---|
| key vector | M0 | the vector of 3.1 | repository test of `via-graph`, not conformance (R20, as `fields.md` 6.4's last-writer test) | the bytes and digest recorded |
| null | M1 | `ford` | build twice | second run: no node `ran` or `recomputed`; same bytes |
| threads | M1 | `ford` | build at 1 and at 8 threads, each clean | same bytes; same `ran` set |
| empty store | M1 | `ford`, store deleted, build directory kept | build | every node `ran`; same bytes as before |
| shuffle | M1 | tables and keys reordered, comments added | rebuild | `ran` is exactly the changed `source` nodes, `merge`, `write:diagnostics.json`, `write:changes.json` (now against the first build, `build.md` 11.2) and `write:manifest.json` (it lists source digests); diagnostics show the new lines |
| early cutoff | M1 | a world `let` read by `height`'s base, from `2 * 3` to `6` | rebuild | `ran` is `source:world.toml`, `merge`, every `field:height/v:build` chunk, `write:diagnostics.json`, `write:changes.json` and `write:manifest.json`; `write:fields/height.npy` `reused` |
| downstream | M1 | `ford` | `--change downstream --out build/ford` (the clean build's directory) | `ran` equals `ford_downstream.ran` |
| vertex | M1 | one vertex of the beck moved | rebuild | `ran` equals `ford_vertex.ran`: the beck's nodes, then those of `downstream` |
| why | M1 | the `downstream` case | read the run report | every `ran` node names a cause; `shape:ford/v:build`'s is "statements" |
| toggle | M1 | `ford` | build with `downstream`, without, with, without | third run: only `write:changes.json` and `write:manifest.json` `ran` (the report is against the second build, where the first had none, and the manifest lists its digest, `build.md` 4, 11.2); fourth run: nothing `ran` (its baseline has the `id` and `run.key` of the second run's, and those are what `write:changes.json`'s key holds of a baseline) |
| cycle two | M1 | `ford` with `feature.ford` `on.ref = "lane_west"`, against `lane_west.ends_at = "ford"` | build | one `graph.cycle`; loop starts at `world.toml:25:8` (`feature.ford` `on.ref`), then line 30 (`lane_west.ends_at`); exit 1 |
| cycle order | M1 | three fields reading in a ring, declared in reverse, `ford`'s sixteen chunks (4 by 4) | build | one `graph.cycle`, first member the first in source order |
| cycle rest | M1 | a cycle and an unrelated feature | build; fix the cycle; build | first: the unrelated feature's nodes `ran` and are in the store; the directory holds `diagnostics.json` and `.previous` only. Second: those nodes `reused` |
| warning replay | M1 | a world with one warning | build twice | second: same `diagnostics.json`, nothing ran |
| error replay | M1 | a formula typo | build twice | second: same error, nothing ran, exit 1 |
| run report | M1 | `ford` | build; `--change downstream` | each report is canonical JSON with `format` `"via-run-0"` and every member of 6.3; `nodes` lists every node once, in id order; `totals` counts equal the statuses; every `ran` node has a cause, every other `null` |
| lint gate | M1 | a world of two layers; a formula typo in an attribute of a layer-2 feature whose effect writes `height`; a plain lint in layer 2 and one in layer 1 | build | both lints `skipped` (each reads the finished build, which holds the failed `attr` node, R29); no `write:` node ran but `write:diagnostics.json`; exit 1 |
| lint reads the build | M1 | a lint in layer 1 reading `height`; a `height`-writing feature of layer 2 moves | rebuild | the lint runs again, its request naming the moved `height` (R29); an unchanged rebuild reuses it |
| step slots | M1 | `ford`'s plain lint `checks` | alternate with and without `downstream`, four times | from the third build, the lint never runs |
| inputs | M1 | a byte changed in the plain lint's `inputs` file; then the file deleted | rebuild each | the lint ran; then `graph.input_missing` |
| folder input | M1 | a file added to the plain lint's `inputs` folder | rebuild | the null build does not apply; the lint ran |
| script | M1 | a sibling module in the plain lint's code folder edited | rebuild | the lint ran without the module in `inputs` |
| rerun | M1 | a plain lint that writes the time beside its diagnostics | build; build with `--rerun-step` | second: `graph.step_unstable` in the run report, not in `diagnostics.json` |
| corrupt | M1 | a byte flipped in the stored object of a `height` chunk the change does not touch | build `--change downstream` | `graph.store_corrupt` in the run report; that chunk ran; same bytes as clean |
| restored | M1 | one build file edited by hand | rebuild | `graph.build_file_restored`; file equal to clean; nothing ran |
| epoch | M1 | the engine with `VIA_TEST_EPOCH_OFFSET=1`, read by test builds only | rebuild | every node `ran`; manifest `files` equal |
| epoch guard | M1 | the examples' recorded `files` maps | build each | equal, or the epoch was raised |
| failed step | M1 | a plain lint that exits 1 | build twice | `protocol.failed` both times; its readers `skipped`; it ran twice |
| null, new directory | M6 | `ford` | build into `build/one`, then into `build/two`, one `--store` | second: nothing `ran`, the run report's `null` true; same files, `id`, diagnostics and output as the first |
| import_key_text | M6 | p imports m, m imports n (a hut at (11, 1)) | build twice into one directory; edit n's world file (the hut 1 m east); build | second: nothing `ran`; third: not null, `m.n.hut` at (12, 1), as a fresh build gives (6.2, "An import enters by its text": a nested import's file reaches the key) |
| two depths | M1 | `ford` | build into two `--out` folders at different depths with one `--store`, then the first again | each manifest's `pack_root` resolves from its own directory (`pack_root_resolves`); the third build: nothing `ran`, no run diagnostic |
| program later | M1 | `ford` with no program configured | build; configure the program; build | first: `protocol.no_program`; second: `step:checks` `ran` (a step that did not start is not stored, 3.4) |
| busy | M1 | the runner holds `<directory>.lock` | build | `graph.busy`, exit 1 |
| killed | M1 | a build killed mid-run | build | builds; no `graph.busy` |
| step reuse | M2 | a plain generate step at layer 1; a `height`-writing feature of layer 2 moves | rebuild | `step:` node `reused` |
| step rerun | M2 | the same; a `height`-writing feature of layer 1 moves | rebuild | `step:` node `ran` |
| same layer | M2 | a generate step and a bookmark `on` its output in one layer | build | builds; the step's request holds no bookmark |
| same views | M2 | a generate step that returns no row on `height`, at a layer above which no entry of `height` lies | build | each chunk of its view and of `v:build` share a key; one `ran`, one `reused` |
| step trace | M2 | a worker reads `height` in one window; a feature outside it moves, then one inside | rebuild each | first `reused` after its trace replays; then `ran`, a new trace added |
| lint gate, traced | M2 | one layer; a formula typo in an attribute of a feature whose effect writes `height`; a worker lint reading `height`, one reading only the features of an unrelated kind | build | the first lint `skipped`, its findings dropped; the second ran and reports |
| outside | M2 | a worker at layer 1 asks for a feature of layer 2 | build | `graph.read_outside` naming layer 2 |
| returned file bytes | M2 | a worker step returning one field file whose bytes follow a param, its document text fixed; the param changed | build twice in one store | the step ran; the field holds the new bytes, as a fresh store gives |
| refused read | M2 | a worker lint reads a field that does not exist, is refused and goes on; a change adds the field | build twice | the lint ran (its trace held the refusal) and read the field |
| causes, let | M2 | a field whose `let` reads `dist` of a point; the point moved | rebuild | the field's entry names `feature.<point>.at` (6.4) |
| causes, density | M2 | a scatter whose density reads a field; the field's base set to 0 | rebuild | every removed instance names `field.<field>.base` |
| causes, of a step | M2 | a feature `on = { of = <step> }`; a feature the step read moved | rebuild | the placed feature's entry names the read feature's key (the step's trace, 5.1) |
| causes, returned feature | M2 | the same | rebuild | the returned feature's entry names the read feature's key |
| causes, returned field | M2 | `returned file bytes`' world | rebuild | the returned field's entry names `step.<id>.params` |
| per item | M3 | three districts, `for_each = "kind:district"`; one moved | rebuild | exactly one `step:<id>/i:` call ran |
| item added | M3 | a fourth district | rebuild | one call ran; the other three `reused` |
| scatter reads step | M4 | a scatter in a layer of rank 1 whose density reads a field a generate step there returns (by tag), an import's footprint and `slope('height')`; a change makes half the ground steep | build; the change `--against` the first, one store | exit 0, no `diag.internal`; the same build as a clean one. The rule's value is computed from the plan's declarations (imports brought, steps' returns), not the world text alone |

## Changed from the spike

| Spike | v1 | Why |
|---|---|---|
| `on`, mounts, `along` and formula attributes each had their own ordering code; adding mounts broke bookmarks on a lane | One graph of references, one cycle report | TODO, "One dependency graph". |
| `field.cycle`, `attr.cycle`, `authority.cycle`, `give.early` | `graph.cycle` | One code for one fault. |
| Fields resolved in the middle of one another; two-phase placement | Dependency order of the graph. The doctrine's build order is what the edges usually give, not an enforced phase | The phases were an accident of the code. |
| A step's record: one hash of command, params, files, frame and the whole build below; one slot | Static key plus recorded reads; many traces per key | SPEC 12.8; FINDINGS: working out the key cost a full build of the layer. |
| The cache trusted its outputs | Store objects and build files checked by digest | SPEC 12.8: a damaged raster was reused. |
| Lint steps always ran | Lint steps are nodes, reused like any other | An unchanged rebuild runs nothing. |
| A part given anything rebuilt whole (60 s; `atlas` 200 s) | `give` is an edge; `for_each` per item | TODO, "Rebuild a part by area". |
| Step `after = "<layer>"` | The layer the step is declared in, and its stratum | R21, vocabulary. |

## Open

- **Pruning the store.** Decided by the store's growth over `ford`'s change scenarios in M1 and
  `basin` in M2. Until then nothing is pruned, every trace is kept, and deleting the store is safe.
- **A size-and-time shortcut before hashing a file.** Decided by hashing time measured on `atlas` in
  M6; until then every file is hashed.
- **Placements kept in the store** (`rule`, `held`). Decided by scatter time on `quay` and
  `townland` in M3.
- **Field nodes cut by the build's chunks or a size of their own.** Decided by `ford`'s build times
  in M1.
- **Which view a shape in a step's view reads a returned field in** (section 4). Decided by `basin`
  in M2.
- **A read answered from the nodes it names** (5.1, Reuse). A changed world misses every memo
  key and makes the whole build per stratum, so a far edit still costs about a cold build.
  Decided by `city` `move_avenue` and the M4 acceptance (a moved square re-runs only its
  district). Ours (M4 slice 3 profile; 1 run each, release, fresh store, load 10 to 15; numbers
  in TODO): on quay `unrelated` and city `move_avenue` no step re-runs
  for a missed memo key; each re-runs because a read's answer changed (the first differing read
  is a whole-extent `build.field`, every `kind:main_street`, every lot), and a step that runs needs
  the build anyway. Node-answered reads pay only once those reads are narrowed to what a district
  uses and the houses steps are split per item (5.3).
- **A route keyed by the corridor it searched.** A `route`'s shape reads its cost's fields over
  the whole extent (section 2), so any ground change re-runs every routed road. A trace of the
  chunks whose cells the search relaxed would key it narrower (a search that relaxes the same
  cells at the same costs runs the same). Not done, since it buys no time yet: every plan's
  placement pass computes every route and `where` region over the whole extent before any node
  runs, and a re-run shape node only encodes that result. Ours (M6 close; atlas `move_market`,
  release, one run, load 45): 10 shape nodes ran, 7 to their old values; 2215 of 2266 field
  chunks and 431 of 508 `distance:kind:road` windows ran to their old values, at 68 ms and 73 s
  of node time; of the 200 s sampled (1 ms) on the main thread, 90 s went to two plans'
  placement passes (two per plan where a fill is given) and 39 s to building the imports. Decided when placement is kept per
  shape across plans and runs (the open item above on reads applies to it too).
