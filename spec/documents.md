# Documents

This chapter says which files make up a world, how they are read into one data model, and how
layers, includes and changes merge into the **merged world** the build graph is made from. It also
fixes namespaces, closed keys, removal, shared constants, the source spans every diagnostic points
back to, and the canonical JSON that conformance compares.

Words are from `vocabulary.md`. Here a **document** is always one file. The union of the world file
and its layers is **the base**; the base with the requested changes applied is **the merged
world**. "(open)" marks what is undecided, with what decides it.

## 1. File kinds

Every file is TOML 1.0.0 (toml.io), UTF-8. A leading UTF-8 byte-order mark is skipped. Line ends may
be LF or CRLF; both read the same. Every file is flat and keyed by id (R1): `[feature.mill]`, never
`[[feature]]`.

- The `[[...]]` header form is `doc.syntax` anywhere. If every item of that array carries
  `id = "<y>"`, the hint is the exact rewrite `[x.<y>]`, and `data.rewrite` holds the first one;
  otherwise the hint names the form `[x.<id>]`. This serves agents moving spike files (principle 8).
- A value that is an array of inline tables (`assets = [{ id = "a", weight = 2 }]`) is a list. It
  is legal where the key's Takes allows a list of tables, and `doc.type` elsewhere.
- Bytes that are not valid UTF-8 are `doc.syntax`, located by file and byte offset (`diagnostics.md`
  1.2). A UTF-16 or UTF-32 byte-order mark is `doc.syntax`, with the hint to save as UTF-8.
- TOML 1.1 forms (a newline or trailing comma in an inline table, `\e`, `\xHH`) are `doc.syntax`.
- Nesting is at most 64 deep. Depth counts tables and lists in the tree (section 2), however they
  were written: the file's root table is depth 0, and each table, inline table, list or header part
  below it adds one; a string, number or bool adds nothing. So `[a.b.c]` is depth 3, and
  `x = [[1]]` at the top is depth 2. An item deeper
  than 64 is `doc.too_deep`, at the span of the first value or key part past the limit, and that
  file is read no further. The reader counts before it descends, so no input overflows the stack
  (`diagnostics.md`, defects of ours). The limit is the same as `formulas.md`'s `formula.too_deep`,
  which counts inside a formula string separately. Ours: no world in M1 nests deeper than about 8.

| File kind | Holds at its top | Named by | Found at |
|---|---|---|---|
| world | `name`, `extent`, `resolution`, `seed`, `include`, `pack`, `chunk` (M6), `[budget]` (M6), `[layer.<id>]`, and any world table (below) | the command line (`build.md`) | the path given |
| layer | world tables only | `[layer.<id>] file` in the world file | relative to the world file |
| kinds file | `include`, `[kind.<name>]` | a library name in an `include` list | section 4 |
| pack | the keys `assets.md` 2.1 lists (`root`, `[defaults]`, `[taxon."<path>"]`, `[asset.<id>]`, `[material.<name>]`) | `pack` | `assets.md` |
| change file | a change body (section 6) | the command line (`build.md`), by path | the path given |

World tables are: `[let]`, `[field.*]`, `[feature.*]`, `[instance.*]`, `[set.*]`, `[run.*]`,
`[scatter.*]`, `[pin.*]`, `[clear.*]`, `[lock.*]`, `[import.*]`, `[step.*]`, `[given.*]` and
`[change.*]`. The vocabulary's section 5 says which M1 rejects with `doc.not_yet`.

Required keys, and what an absent optional key means:

| Table | Required | Optional, and its absence |
|---|---|---|
| world file | `name`, `extent` | `resolution`: 0.5 (`numbers.md` 4). `seed`: 0 (`numbers.md` 7). `include`: no libraries. `pack`: none; what an asset id with no pack gives is `assets.md`'s. |
| `[layer.<id>]` | `file`, `owner`, `rank` | `after` (M4). |
| `[layer.world]` | nothing | `owner`: none. `rank`: 0. `file` is `doc.unknown_key`: the world file is its file. |

A required key that is absent is `doc.missing_key`, naming the table and the key, located at the
table's header span (2.1), with the hint naming the key and what it takes. Other chapters name their
own required keys; an absent one is `doc.missing_key` too.

Rules on where things may be written:

- The top-level keys of the world file, and `[layer.*]`, are written in the world file only. In a
  layer they are `doc.world_key_in_layer`. Layers do not nest.
- A kind is declared in a kinds file only. `[kind.*]` in a world or layer file is
  `doc.kind_outside_library`, with the hint to move it to a library folder (section 4).
- `[change.*]` is written in a world or layer file. In a kinds file, pack or change file, or inside
  another change's body, it is `doc.change_nested`.
- `remove` and `unset` are written in change bodies only. Anywhere else they are
  `doc.remove_outside_change`: the base says what is, not what is not.

## 2. The data model behind the syntax

The engine's first act is to turn each file into a plain tree. Everything after works on the tree,
never on TOML, so a second front end (a JSON form, an editor's own store) needs only to produce the
same tree with spans. The model is ours.

| Item | Holds |
|---|---|
| table | A map from key (a string) to item. Key order carries no meaning: the map is ordered by key, bytewise UTF-8 (`vocabulary.md` 2.2). |
| list | Items in written order. List order carries meaning (a path's points). |
| string | UTF-8 text, after TOML's escapes are applied. |
| number | The literal's source text; its normalised text, as `numbers.md` 3.1 step 1 reads it (`_` dropped, a leading `+` dropped, a hexadecimal, octal or binary integer converted to decimal digits); and whether it was written as a TOML integer. Never a binary float. How the normalised text becomes a value, and millimetres for geometry, is `numbers.md`. |
| bool | `true` or `false`. |

Every item carries its **origin**: its spans (2.1) and the chain of includes, layers and changes it
came through (diagnostics print the chain, `diagnostics.md` 1.2).

- TOML dates and times have no tree item. Any one is `doc.type`.
- Non-finite numbers (`inf`, `nan`) are rejected at read as `num.not_finite` (R26).
- A key that takes `integer` accepts only a number written as a TOML integer. A key that takes
  `number` accepts both forms; `7` and `7.0` are the same value.
- Dotted keys, standard tables and inline tables that say the same thing build the same tree. So
  `on = { ref = "beck", t = 0.5 }` and a `[feature.ford.on]` table are one item. Whether a table item
  merges key by key or is replaced whole is set by its key (6.2), never by how it was written.

**The parser.** Standard TOML readers drop what this model needs. The reader must give: each key's
and each value's byte range; the raw text of each number literal; a map from each character of a
string's value back to the file; and whether a table came from a `[x]` header, a `[[x]]` header, a
dotted key or an inline table. It must accept TOML 1.0.0 exactly. It is a reader of our own
(`crates/via-doc/src/read.rs`), decided in M1 by the conformance cases `syntax_v11`, `array_table`
and `formula_span`: the maintained crates (`toml_edit` 0.25, `toml_parser` 1.1, `toml` 1.2) all
read TOML 1.1, so `syntax_v11` passes on them where it must fail, and none maps a string's
characters back to the file.

### 2.1 Spans

A span is `file`, `line`, `column`, and the byte range `[start, end)`.

- **Key span**: the key as written, dotted parts included. **Value span**: the value as written,
  quotes and brackets included. Every table entry stores both.
- **Header span**: what created a table first in the file, by byte offset: its `[...]` header, the
  first dotted key that names it, or the opening brace of its inline table.
- `line` and `column` count from 1. A column counts Unicode scalar values, not bytes; a tab is one
  column. CRLF is one line end. A skipped byte-order mark is not a column, but counts in bytes.
- A string that holds a formula keeps a map from each character of its value back to its place in
  the file, through escapes and across the lines of a multi-line string. A position `formulas.md`
  reports inside a formula becomes a file line and column through this map.
- A value a kind's `defaults` supplies keeps the default's span, with the feature in its chain.
- A key a change removed has no value; the change report cites the `remove` or `unset` line.

Which span a diagnostic gives (`diagnostics.md` 1.2 says the key at fault, else the header):

| Fault | Span |
|---|---|
| In a key's value (type, range, unknown name, upward reference) | The key span. |
| The key itself (`doc.unknown_key`, `doc.not_yet`, `doc.world_key_in_layer`, ...) | The key span. |
| Inside a formula | The faulty sub-expression, through the map. |
| About a table as a whole (`doc.missing_key`, `identity.duplicate_id`, `doc.placement_twice`) | The header span. |

**`file`** never depends on the machine, so `diagnostics.json` is the same bytes everywhere (R26).
Its forms are `diagnostics.md` 1.2's, the one definition: a path under the world file's folder, or
a tagged form (`lib:<library id>/`, `pack:`, `change:`, `step:<id>/`, `input:`). A file in a
library is written by its library id, never by which root found it (section 4), so the order of
`--lib` roots does not change the bytes. A change file's stem is unique (section 3), so `change:`
and its file name names one file. No span ever holds an absolute path.

A file that fits several forms takes the first that fits: a step's returned document, `step:`;
the pack file and every file it binds, `pack:` and the path relative to the pack file's folder, so
`lib/packs/primitive.toml` is `pack:primitive.toml` and a file under `root = "../art"` is
`pack:../art/models/cottage.glb`; a file in a library, `lib:`; a file under the world file's folder,
its path; a change file, `change:`; anything else, `input:`.

### 2.2 Entities

The top-level tables keyed by id are **top tables**: `feature`, `kind`, `layer`, `let`, and the
rest of section 1's lists. An **entity** is one id directly under a top table: `[feature.ford]`,
`[kind.lane]`. Its address is `<top table>.<id>`. Each `[let]` name is an entity too (address
`let.<name>`), whose value is a formula, not a table. Everything inside an entity (its keys, and
sub-tables such as `[kind.lane.effect.bed]` or `[field.height.then.r1]`) is part of it and has a
key path below its address. The base is formed entity by entity (section 5); key paths are what
changes write and conflict on (section 6).

### 2.3 The document-language version

The document language is named by one string, **`via-doc-1`**. It is the "document language" version
that enters every key (`graph.md` 3.3), beside `via-step-1` for the protocol.

- It names the meaning of the text: the file kinds and their keys and what each takes (this chapter,
  `vocabulary.md` and the chapters that list closed keys), the tree and its spans (section 2),
  library lookup (section 4), merge and change rules (sections 5 and 6), the reading order (section
  8), the limits (section 1), and the formula grammar and function list (`formulas.md`, which has no
  version of its own). What a function computes, to the last bit, is the numerics epoch's, not this.
- It is raised, to `via-doc-2` and on, by any engine release in which one set of files can merge to
  a different merged world, print different canonical JSON under one `@format`, or give a different diagnostic code,
  subject, span or `data`. A release that changes none of these keeps it, and the store is reused.
- It is not `@format` (section 8). `via-merged-1` names the layout of the printed JSON; `via-doc-1`
  names what the text means. A new key raises the language and leaves the format; a new `@` member
  raises the format.

An entity is **declared in** exactly one document: the one that holds its table.

## 3. Ids and namespaces

The form of an id and of a place name is `vocabulary.md` 2.1; how a place name resolves, and the
codes for a bad, reserved or duplicate id, are `identity.md` (`identity.bad_id`,
`identity.reserved`, `identity.duplicate_id`). This chapter fixes which namespace each id lives in.

| Namespace | Holds | Scope |
|---|---|---|
| world | fields, features, instances, sets, runs, scatters, pins, clears, locks, imports, steps, layers | One per world, across the world file and all its layers. |
| let | top-level `let` names | One per world, across the world file and all its layers. |
| kind | kinds | One per world, across all its included libraries. |
| change | inline changes from every layer, and each change file requested in this build | One per world. |

- An id is unique across the whole world namespace, whatever its top table: `feature.mill` and
  `instance.mill` are `identity.duplicate_id`. The check is on the id, never on the address.
- There is no namespace per layer. Two layers declaring one id is `identity.duplicate_id`, naming
  both places. The spike let a step's output and a feature in another layer share an id unseen
  (`spike/FINDINGS.md`); here it is an error, and no name is ever ambiguous.
- A `let` name is not in the world namespace: `[let] ford` beside `[feature.ford]` is legal. A `let`
  name that equals a field name, or any other name visible where formulas read it, is
  `formula.shadow` (`formulas.md` 5), raised once by that chapter. This chapter raises nothing for it.
- An import is its own world with its own namespaces. Its ids are reached from the parent only
  through the import's id, as `<import>.<place>` (R8). Same ids, kinds or change names in a parent
  and an import never collide. (M4.)
- A change file's name is its file stem, which must be an id (`identity.bad_id` otherwise, as for
  `no_cart.v2.toml`). A change file must end in `.toml` (`doc.path_form`). Change files are known
  only when requested, so an inline change and a change file of one name are
  `identity.duplicate_id` only when both are requested. How a command-line request tells a name from
  a path is `build.md` (`--change`).
- Reserved: the id `world` names the world file's own layer. `[layer.world]` is its only legal use;
  any other entity with the id `world` is `identity.reserved`. A layer whose rank equals another's,
  `world`'s default 0 included, is `doc.layer_rank`.

## 4. Includes

`include` is a list of library names. A **library** is a folder named by its id. It may hold a
kinds file `kinds.toml`, and it may hold code that steps run, reached by a `command` element
`lib:<library>/<path>` (`protocol.md` 1). A library with no `kinds.toml` adds no kinds; one with
code only (streets, terrain: R19b) is still included by name. Steps are declared in a world or a
layer only, never in a kinds file (`protocol.md` 1). A name is never a path.

A name is looked up in these roots, first match wins:

1. the folder `lib/` beside the world file;
2. the library roots given to the build, in the order given (`--lib`, `build.md`);
3. the library roots in the environment variable `VIA_LIB`, in the order written;
4. the shipped root: the folder `lib` beside the folder that holds the `via` executable
   (`<executable's folder>/../lib`).

`VIA_LIB` holds a list of folders separated by the system's path-list separator (`:` on macOS and
Linux, `;` on Windows), as `PATH` does. Empty entries are skipped. Each entry is read exactly as one
more `--lib` argument placed after those on the command line, so everything said of `--lib` holds
for it: its place in the build key (`graph.md` 6.2), spans written `lib:<id>/` (2.1), and no
absolute path in any output. Unset or empty, it adds no root.

A name found in more than one root is `doc.include_shadowed` (info), naming the one used. A name
found nowhere is `doc.include_missing`, naming the roots searched and the nearest name found
(`diagnostics.md` 1.4). Both name roots by their place in the order, never by path: `beside`,
`--lib <n>`, `VIA_LIB <n>` (each counting from 1) and `shipped`. So the diagnostic is the same bytes
on every machine (R26). The library folder and `kinds.toml` must match the name on disk exactly,
letter case included, by the algorithm of 7.2 (`doc.path_case`).

- A kinds file may `include` other libraries, resolved the same way. Each library is read once,
  however often it is named. A cycle is `doc.include_cycle`, naming the loop.
- Libraries add their kinds to the world's one kind namespace. A world never writes keys onto an
  included kind; it changes one only through a change (section 6). Two libraries that declare one
  kind are `identity.duplicate_id`, naming both files. Order in `include` changes nothing.
- An import resolves its own `include` from its own world file. (M4.)
- Each file read is a declared input of the build, by digest (`graph.md`), so a library edit
  re-runs what read it.
- Versions of a library: (open). Decided when the first shipped library makes a change that breaks a
  world, and no later than M7, when outsiders build from the documents.

### 4.1 The repository's `lib/` in development and conformance

A binary built by `cargo build` sits at `target/<profile>/via`, so its shipped root is
`target/lib`, which does not exist; and `examples/ford/` holds no `lib/`. The repository's own
`lib/` is reached through `VIA_LIB`, never through a path baked into the binary:

| Who | How `lib/` is found | How its code is trusted |
|---|---|---|
| A developer at the shell | `VIA_LIB=<repo>/lib`, set once in their shell, or `--lib <repo>/lib`. | `via trust <world>` once; again after each edit to a library's code (`protocol.md` 7). |
| `cargo run` and `cargo test` | The repository's `.cargo/config.toml` sets `[env] VIA_LIB = { value = "lib", relative = true }`, which Cargo turns into the absolute path of `<repo>/lib`. | As the conformance runner. |
| The conformance runner | It sets `VIA_LIB=<repo>/lib` for every case, so `via build examples/ford/world.toml` (`docs/plan.md`, Verification) runs as written, with no `--lib`. A case that names its own roots passes `--lib`, which is searched first. It also sets `VIA_CONFIG_DIR={work}/.config`, empty. The runner never trusts a world or records a program itself (`conformance.md` 5.2): those are the case's own actions. A `ford` case runs `via config program python {python}` (its lint step's `command` starts with `python`; without it, `protocol.no_program`), then `via trust world.toml --yes` with each change it requests, before it builds (`conformance.md` 3.6). The cases that test trust itself (`protocol.md` cases 6 to 8, 38) write their own. |
| A released binary | Its shipped root, `<executable's folder>/../lib`, installed beside it. | By the digests it carries. |

- **Only a release carries digests.** The release process builds the binary with the code folder
  digest (`protocol.md` 7) of each library in the repository's `lib/` at the release commit, and
  installs those folders as its shipped root. A binary built any other way (`cargo build`,
  `cargo test`, a developer's own build) carries none, so no library, wherever found, is trusted
  without `via trust`. A binary cannot tell a stale checkout from a release's folder except by
  digest, and development builds have none to compare.
- **No committed `trust.toml`.** Trust is by digest, so a trust file committed under `fixtures/`
  would go stale with every library edit. Each case makes it from the current code, through the
  same `via trust` a user runs. That the code is ours is what the case's trust says;
  it is never a rule inside the engine that our libraries are trusted (R19b: no private door).
- Trust changes whether a step runs, never what a build contains (`protocol.md` 7), so a released
  and a development binary of one engine version give the same bytes for one world.

## 5. Layers and the base

`[layer.<id>]` in the world file declares a layer with `file`, `owner` and `rank` (section 1).

- `file` is relative to the world file. A file not found is `doc.file_missing`. Two layers naming
  one file, or a layer naming the world file, is `doc.layer_file_twice`.
- `owner` is free text. The build does not check it; it travels in the chain of every diagnostic
  raised from that layer, so a message says who answers for the text.
- `rank` is an integer, unique across the world's layers (`doc.layer_rank`). The world file is rank
  0 unless `[layer.world] rank` says otherwise. Ranks may be negative.

**The base** is the union of the entities of the world file and every layer file. An entity is
declared in one document, and no document writes onto an entity declared in another:

- An id declared in two documents is `identity.duplicate_id`, even if one only adds keys. The hint
  names the declaring file and says: edit it there, or write a change.
- This settles the vocabulary's question: a layer may not write keys on a table declared in a lower
  layer. A difference over someone else's text is a change, which is the doctrine's noun for it;
  allowing both would be two ways to say one thing (principle 10), and would spread one thing over
  several files (principle 11).

**What a step returns** (M2) joins its step's layer (`protocol.md` 5): declared in that layer, at
its rank, with its owner in the chain of every diagnostic, and spans in `step:<id>/document.toml`
(2.1). So a layer's owner answers for its text and for what its steps make, and no output of a
build is without an owner (challenge 21). A step's own layer is where it is declared, never where
its output is read.

Output per layer, that a game may load or replace on its own (TODO, Layers): every output record
carries its `layer` (`build.md` 9.1); a separate file per layer is `build.md`'s, (open) until M6.

### 5.1 Direction

A layer refers downward only. Direction is checked on **named references**: a name written in a
name key (`on.ref`, `of` in `on` and `within`, `starts_at`, `ends_at`, `follows`, `within`, `set`,
`by` when written, `clump.around`, `give`, and every attribute a kind lists in `names`, checked at
the feature's rank, `authority.md` 1.2), a name a formula reads (`formulas.md` reports the names
it reads), or an effect's `field` selector (`role:`, `tag:`), checked at the rank of each feature of
its kind (`fields.md` 2.3, case `upward selector`). Each is an edge in the graph (`graph.md` 2). `ground` names a kind, which has no rank:
only that the kind exists is checked (`identity.md`). For each:

- the referring entity has the rank of the layer it is declared in; a key a change wrote onto an
  entity has that entity's rank;
- the entity referred to has the rank of the layer it is declared in; a thing a step produces has
  its step's;
- the referred rank must be at or below the referring rank. Otherwise `doc.upward_ref`, naming both
  entities, both layers and their owners. When a change wrote the reference, the change is in the
  chain.

What is not a named reference, and so not checked for direction:

- **A read selector** (`kind:`, `tag:` or a shape selector in a formula, `for_each`). It resolves only to
  entities at or below the referring rank (R21's view, `formulas.md` 7, `graph.md` 4); members
  above are left out without a diagnostic. If that leaves it empty, the reading chapter's own code
  applies (`formula.empty_selection`, `graph.for_each_empty`).
- **An implicit relation**: a hand edit whose `by` is not written speaks to every producer in its
  own layer or below (`identity.md` 4.1); a producer gives way to things in its own layer or below
  only (`rules.md` 3.4), and an overlap with a thing in a higher layer is reported, not avoided
  (`place.crowded`). Both reach the referring layer and below, as a named reference does, but are
  resolved by the build, not checked as `doc.upward_ref`. The one relation that crosses ranks
  upward is the ordering of a field's rows from every layer by authority (R21).
- **A field's rows.** A formula in text reads a field's finished value, the build's own view, every
  row from every layer (`formulas.md` 8.3 `f`, `fields.md` 6.2, `graph.md` 1). Direction checks only
  the rank of the layer that declares the field. Only a step reads a narrower view: its stratum
  (`graph.md` 4).

Kinds and packs have no rank. A name read inside a kind (a formula in an effect, a mount) is checked
at the rank of each feature of that kind. A fault there is one record per kind and name read:
subject the kind's key path, span in the kinds file, each feature that uses it in `related`, capped
as `diagnostics.md` 5 caps. `let` names are entities and have the rank of their layer.

## 6. Changes

A change is a named difference over a world (doctrine noun). It is written inline as
`[change.<name>]` in the world file or a layer, or as a change file. Its body has the shape of a
world: `[change.downstream.feature.ford]` writes onto `feature.ford`; a top-level key written in the
body (`pack = "winter"`) writes that key.

No change applies unless it is requested, on the command line (`build.md`) or by an import's
`changes` (M4). An unknown name is `doc.unknown_change`, naming the nearest. A change requested
twice, by one name or by two paths to one file, applies once. An empty change body is
`doc.change_empty` (warning).

An inline change may write onto any entity of the world, whichever layer declares it. Its owner is
in the chain of everything it writes, and nothing applies until someone requests it by name. One
change namespace means one name is one difference: a season across terrain and NPCs is one change
`winter` that writes into both layers' entities. The TODO lesson asked for per-layer variants (each
layer its own `winter`, requested together); that is answered differently here, since one name
spread over several files is the opposite of principle 11, and two layers each declaring `winter`
is `identity.duplicate_id`.

### 6.1 What a change body may do

| Written in the body | Effect on the base |
|---|---|
| a key on an existing entity | Sets it, replacing the value there. |
| a table for an id that does not exist | Adds the entity. If an existing id in the same top table is its nearest name (`diagnostics.md` 1.4), `doc.change_adds_near` (warning) names it: a typo would otherwise add a guide silently. |
| `remove = true` | Deletes the entity, or the sub-table it is written in. No other key may sit beside it (`doc.remove_with_keys`). |
| `unset = ["k", ...]` | Deletes those keys from the table it is written in. In `[change.<c>.let]` it deletes `let` names. A key named in `unset` and also written in the same table is `doc.remove_with_keys`. |
| one placement key (`at`, `on`, `path`, `region`, `where`, `follows`) | Sets it and deletes the other placement keys of that table. If the key is not `path`, it also deletes `starts_at`, `ends_at`, `smooth` and `route`, which only a path takes (`geometry.md` 2.2, 2.3, `derived.md` 3); if it is not `where`, it deletes `holds` (`derived.md` 2). |
| the top-level keys `pack`, `seed`, `extent`, `resolution` | Sets them. `pack` is the ford pack-swap case: a change file holding `pack = "<other>"` leaves the world's text untouched and swaps the look, the facts it carries (size, footprint, mounts) and what follows from them, which settles and is reported (R30, `assets.md` 1). |

What a change body may not hold:

| Written in the body | Code | Why |
|---|---|---|
| `include` or `name` | `doc.change_fixed` | Changes write onto the kinds `include` brings, so the list is settled before any change applies, in one pass. `name` names the world whose differences the changes are. |
| `[layer.*]` | `doc.not_yet` (M4) | Adding, removing or re-ranking a layer by change makes reading re-entrant: the base and the change namespace would depend on the changes. No ford case needs it. When one does, the layer set is resolved in a first pass, before the base is formed. |
| a new kind | `doc.kind_outside_library` | A kind is declared in a library. A change may write onto, or remove, an existing kind; kinds have no rank. |
| `[asset.*]`, `[material.*]`, `root` | `doc.unknown_key`, hint `pack` | A pack is not reachable by changes in M1; a change swaps it whole with `pack`. |
| `[change.*]` | `doc.change_nested` | A change has no changes. |
| `remove` or `unset` at the body's root or directly under a top table (except `unset` on `let`) | `doc.remove_above_entity` | Removal acts on an entity or inside one, never on a whole world or top table. |

TOML has no null, so removal is said by `remove` and `unset`, never by a sentinel value (the spike's
`-1e9`). Unsetting a feature attribute falls back to its kind's default. Unsetting a required key
leaves `doc.missing_key`. `remove` or `unset` that names nothing in the base is
`doc.remove_missing`: a change that has gone stale says so.

**Layer of what a change writes.** A key a change writes onto an existing entity takes that entity's
layer. An entity an inline change adds is declared in the layer that declares the change; one a
change file adds is declared in `world`. Which layer a change file's added entities belong to, other
than `world`: (open), decided by the first example whose change file must add a table that names
something above rank 0.

**Unrequested changes are checked too** (principle 4; challenge 33: change scenarios that are never
run rot). After the merged world is formed, every inline change not requested in this build is
applied alone to a copy of it, and checked by reading steps 4, 6 and 7 (section 8), conflicts
excepted. Each fault found is `doc.idle_change` (warning), with the fault's own code in `data.code`
and its span. A change file not requested is not read.

### 6.2 Merge rule

Changes merge onto the base, entity by entity. Whether a table merges or is replaced is set by its
key, never by whether it was written inline or with a header. This is the one list:

- **Value tables**, replaced whole like every list: `on`, `follows`, `clump`, and every item of a
  list. Writing only `t` in a change's `on` replaces `on` with a table that lacks `ref`:
  `doc.missing_key`, whose hint says to write the whole `on`.
- **Every other table merges** key by key, to any depth: top tables, `let`, `defaults`, `then`,
  `effect`, `mount`, `item`, `params`, `give`, `budget`, and every entity.
- A chapter that adds a table-valued key says here which list it joins.

This supersedes the sentence in `vocabulary.md` 2.5 that makes inline tables values. It governs
merging text by changes only; how a pack's defaults are looked up is `assets.md` 2.5's own rule.

### 6.3 Changes commute

Requested changes apply all at once to the base, so their order never matters: the merged world is
a function of the set of changes requested. `remove` and `unset` act on the base only, so a change
cannot remove what another change adds.

Each change has a write set of key paths:

- every key it writes;
- for an entity it adds, the entity's address;
- for `remove`, the address of what it removes; for `unset`, each key named;
- for a placement key, all six placement keys of that table, and `starts_at`, `ends_at`, `smooth`,
  `route` and `holds` when it deletes them.

Two requested changes conflict when a path of one equals, or is a prefix of, a path of the other.
That is `doc.change_conflict`, naming both changes, the path and both spans, even when the two values
are equal. So two changes that add one new entity conflict, whatever keys each writes.

### 6.4 Shared constants

A value that several things share is a name in `[let]` (top level), `[field.<n>.let]` or
`[kind.<k>.let]`, read by formulas (`formulas.md`). Each top-level `let` name is an entity, declared
in one document; a layer reads the names at or below its rank. A change varies a shared value by
writing that one name: `[change.steep.let] grade = 0.12`. A library has no top-level `[let]`: its
constants live in its kinds, so including a library adds no world names.

A key that takes `number` or `integer` takes a literal only; it cannot name a constant. Whether it
should: (open), decided by the first example where one change must move several such literals
together.

### 6.5 Seasons and variants (M2)

A season, a variant or a "what if" is a change: one name, written once, over any layer's entities
(6). Nothing else is needed (challenge 40):

```toml
[change.winter.let]                  # in the world file; the layers read the name
snowline = 34

[change.winter.field.grass]          # a field the dressing layer declares
base = 0

[change.winter.step.weather.params]  # a step's params merge key by key (6.2)
snow = true
```

- **What it may write** is 6.1's list: `let` names, field bases and rows, features, rules, a step's
  `params`, and `pack` for a winter pack (R30).
- **One build per set of changes.** Each set requested is its own build directory. The game chooses
  which to load (principle 12); a world never holds two seasons at once.
- **What it costs.** Keys are by content (`graph.md` 3.1), so a second build shares every node the
  change does not reach. A step runs again only when an answer in its trace differs, so a change
  that writes above a step's layer never re-runs it, and building with and without a change by
  turns runs each step once per distinct reading (`protocol.md` case 22).
- **Checked unrequested.** An inline season not asked for is checked all the same (6.1,
  `doc.idle_change`), so it does not rot between winters.

## 7. Closed keys and free attributes

No key is ever ignored. Every key in every file is one of:

- a key or table `vocabulary.md` lists for that place;
- on a feature, an attribute its kind declares in `defaults` or `requires` (`authority.md`);
- inside `params` of a step, anything: it is handed to the step unread (`protocol.md`);
- in a pack, an asset fact `assets.md` allows.

Anything else is `doc.unknown_key`, naming the nearest listed key or declared attribute as its
hint. A removed or forbidden word has a fixed hint instead, which replaces the nearest-word hint:

| Word | Where | Fixed hint |
|---|---|---|
| `over` | a feature | "`over` was dropped; change one decision, see `authority.md` 5" |
| `authority` | an instance | "the footprint takes its `ground` kind's tier; ground with a kind of the tier you want" (`authority.md` 1.3) |
| `feature` | `on` | "write `ref`" |
| `joins` | a feature | "write `starts_at` or `ends_at`" |
| `[params]` | top level | "write `[let]`" |
| `[variant.<n>]` | top level | "write `[change.<n>]`" |

The other dropped words of `vocabulary.md` section 6 take the nearest-word hint. A word the
vocabulary lists for a later milestone is `doc.not_yet`, naming the milestone. A
kind that declares an attribute named as a reserved feature key is `doc.reserved_attribute`. A table
holding two placement keys is `doc.placement_twice`, naming both.

Keys are checked on the merged world, so a feature's attributes are checked against its kind as the
changes left it. A value of the wrong form for its key's **Takes** is `doc.type`, naming the key,
what it takes and what was written. A string where a key takes `number`, `integer` or `point` gets
the hint that this key takes no formula, when the string starts with `=`.

### 7.1 Formula-valued keys

A formula string starts with `=`, everywhere (`formulas.md` 1); a string without it where a formula
is taken is `formula.unmarked`. What decides that a value is a formula is never its text:

- For a key the vocabulary lists, its Takes says whether it takes a formula.
- For a feature attribute, its kind decides: a default that is a number or a formula makes the
  attribute take a formula; a string default makes it text, even if it starts with `=`
  (`formulas.md` 6.7). An attribute named only in `requires` takes its type from what the feature
  writes (`authority.md`).

A formula string is handed whole, with its span map (2.1), to `formulas.md`. This chapter never
parses formula text.

### 7.2 Paths in text

A `file` value is relative to the file that names it and uses `/` on every system. Its form:

- An absolute path is `doc.path_form`: a leading `/`, a drive letter (`C:`, with or without a
  slash), or a leading `//`. So is any `\`. `~` has no meaning; it is a name like any other.
- Each part is printable ASCII, other than `\ : * ? " < > |`; it does not end in a space or a `.`
  (other than `.` and `..`), and is not a Windows device name (`CON`, `PRN`, `AUX`, `NUL`, `COM1` to
  `COM9`, `LPT1` to `LPT9`, any case, with or without an extension). Otherwise `doc.path_form`. These
  are the names that work on all three systems (Microsoft, "Naming Files, Paths, and Namespaces"),
  and ASCII needs no Unicode normalisation, which file systems treat differently (R20).
- `..` parts are resolved by text against the naming file's folder before anything is opened, so a
  symbolic link never changes where `..` leads.

**Exact case on every system** (`doc.path_case`), so a world that builds on macOS or Windows builds on
Linux (R20). To open a path, the engine walks it part by part from its folder. For each part it lists
the folder: an entry equal bytewise is followed; else an entry equal after ASCII case folding gives
`doc.path_case`, naming the name on disk; else `doc.file_missing`, with the nearest entry
(`diagnostics.md` 1.4). The walk runs on every open, found or not, since a case-blind file system
finds the file anyway. A symbolic link is compared by its own name, then followed.

The same rules apply to library folders and `kinds.toml` (section 4), to the world file and change
files given on the command line, and to every other `file` in text. A command-line path may be
absolute and may use the system's separators; its faults are located by argument.

A file found but not readable is `doc.file_unreadable`.

## 8. Reading order, and the canonical form

1. Read and parse the world file into the tree. Check its top-level keys and `[layer.*]` tables
   (section 1).
2. Resolve `include` from the world (section 4); read every kinds file.
3. Read every layer file; form the base (section 5).
4. Read each requested change: inline by name from the base, change files by path. Run these checks
   only: `doc.syntax`, `doc.too_deep`, `doc.change_nested`, `doc.change_fixed`, `doc.not_yet` for
   `[layer.*]`, `doc.kind_outside_library`, `doc.remove_with_keys`, `doc.remove_above_entity`,
   `doc.remove_missing`, `doc.change_empty`, ids, then `doc.change_conflict` (6.3). Apply all the
   changes (6.1, 6.2).
5. Read the pack the merged `pack` names (`assets.md`), so a change's pack swap is honoured.
6. Check the merged world: closed keys against the final kinds, required keys, Takes, placement
   (section 7). All closed-key checks are here, none earlier.
7. Resolve names (`identity.md`) and check direction (5.1). A name left with no target by a
   `remove` is `doc.dangling`, naming the table that holds it and the change that removed its
   target.

Every step reports all it finds before stopping; the build stops after the first step with an
error. Unrequested changes are checked after step 7 (6.1). `diagnostics.json` is written either way
(`diagnostics.md`).

**Canonical JSON.** `via merge` (`build.md`) prints the merged world as one JSON object, after step
7 ends with no error; on an error it prints nothing to standard output and exits 1. The object holds:

| Member | Holds |
|---|---|
| `@format` | `"via-merged-1"`. |
| `@changes` | The requested changes, as `build.md` lists them in the build record. |
| `name`, `extent`, `resolution`, `seed`, `pack`, `include` | As merged, where written. `include` sorted bytewise; it changes nothing by order. |
| `layer` | Every layer by id, `world` included, with its keys; `world` always shows `rank`. |
| `kind` | Every kind of every included library, after changes, each with `@library` naming its library. |
| one member per other top table (`field`, `feature`, `instance`, `let`, `step`, ...) | Each entity by id, after changes, with `@layer` naming the layer it is declared in. A `let` entity is `{ "@layer": ..., "value": ... }`. |

- `[change.*]` tables are not printed: requested ones are applied, unrequested ones are not part of
  the merged world.
- Nothing is filled in. A key appears only where text wrote it; a kind's default stays in the kind.
  Filling in defaults is the `attr` node's work (`graph.md` 1).
- Members starting with `@` are ours; no key can start with `@` (ids match `[a-z][a-z0-9_]*`).
- Numbers: a length is written as exact metres and a facing as exact degrees, from the integer
  millimetres and millidegrees the core holds, by `numbers.md` 3.4, never through a double (R24:
  the core holds millimetres, text holds decimal metres). An `integer` key is a JSON integer. The world `seed` is a
  string of 16 lowercase hexadecimal digits (`numbers.md` 3.4). Any other number is a double in the
  shortest form of `numbers.md` 3.4. A number inside `params`, which has no type, is its normalised
  text (section 2).
- Strings are raw UTF-8. Only `"`, `\` and U+0000 to U+001F are escaped: `\"`, `\\`, and `\u00xx`
  with lowercase hexadecimal. Nothing else is escaped. A formula string is printed as written,
  `=` included.
- Object members are ordered bytewise by their UTF-8 key; this is not RFC 8785, which orders by
  UTF-16 code units. Lists keep their order. No whitespace outside strings; the output ends with one
  line feed.

Because each entity carries its layer and every layer its rank, two worlds that print the same JSON
build the same graph. `diagnostics.json` uses the same rules for strings, numbers and ordering.

**Origins.** `via merge --origins` prints a second object of the same shape, in the same form, with
each value that is not a table replaced by `{ "file", "line", "column", "chain" }` of its key span
(2.1). Conformance checks spans through it, or through a diagnostic's location. The flag is added to
`build.md`'s list.

**The spike oracle.** The plan asks exact agreement "only for the merged document, exit status and
findings on bad edits" (`docs/plan.md`). The spike prints in its own words (`[[x]]`, variants,
`on.feature`), and no spike word carries over (R27). So the spike's merged output is compared after a
declared translation into this form, following `vocabulary.md` section 6. Where that tool lives:
(open), decided in M0 step 7, when the oracle data is exported.

## 9. Imports (M4)

What is decided: an import is a world built apart, with its own files, namespaces and includes
(section 3). The parent names its places, never its coordinates (R8), and requests its changes by
name in `changes`. A parent's change may write the parent's `[import.<id>]` table, never the
imported world's own tables: a difference inside an import is one of the import's changes. A place
is built alone and joined at its edge (R11): the parent changes imported ground only through a layer
with `after = "imports"` (R10), and a fill that must read its parent takes it through `give`. An
import's own imports are followed or rejected with a diagnostic, never ignored; which: (open),
decided here in M4 by `county`. Whatever is decided about an import's pack, one asset id bound to
two different files is an error. Every import key is `doc.not_yet` in M1.

`exit` (a kind key, R9) marks a kind's point features as places where something leaves the world;
the core owns the flag and the word. Which of an exit's attributes the core compares when two
worlds meet: (open), decided here in M4 by the first import whose exits must match (challenge 55
records `width`). `exit` is `doc.not_yet` in M1.

## 10. Worked example

`mill/world.toml`:

```toml
name       = "mill"
extent     = [0, 0, 128, 128]
resolution = 1
include    = ["ways"]                  # lib/ways/kinds.toml declares kind lane, attribute width
pack       = "primitive"

[layer.dressing]
file  = "layers/dressing.toml"
owner = "props"
rank  = 1

[field.height]
role = "surface"
base = 20

[feature.lane]
kind = "lane"
path = [[0, 40], [128, 60]]

[change.wide.feature.lane]
width = 6
```

`mill/layers/dressing.toml`:

```toml
[instance.cart]
asset = "box_small"
on    = { ref = "lane", t = 0.3, offset = 4 }
```

`mill/changes/no_cart.toml`:

```toml
[instance.cart]
remove = true
```

Built with `wide` and `changes/no_cart.toml` requested, the merged world holds no `instance.cart`,
and `via merge` prints `feature` as:

```json
"feature":{"lane":{"@layer":"world","kind":"lane","path":[[0,40],[128,60]],"width":6}}
```

`--origins` gives `width` the span `world.toml` line 21, column 1, chain change `wide` alone: the
world file's own text has no chain entry (`diagnostics.md` 1.2).
Requesting the two in the other order prints the same bytes. Writing `starts_at = "cart"` on
`feature.lane` is `doc.upward_ref`: `lane` is rank 0 and `cart` rank 1.

## Diagnostics

Ids that are bad, reserved or declared twice use `identity.md`'s codes. Faults found in an
unrequested change are reported as `doc.idle_change`.

| Code | Severity | Says |
|---|---|---|
| `doc.syntax` | error | Not TOML 1.0.0, not UTF-8 (the byte offset in `data.offset`), or a `[[x]]` header (its exact rewrite in `data.rewrite`, section 1); position from the parser. |
| `doc.too_deep` | error | Tables and lists nested over 64 deep (section 1); the file is read no further. |
| `doc.type` | error | A value of the wrong form for its key, or a TOML date or time. |
| `doc.unknown_key` | error | A key or table not listed for its place; the nearest listed word, or a removed word's fixed hint (section 7). |
| `doc.missing_key` | error | A required key is absent; the table, the key and what it takes. |
| `doc.not_yet` | error | A word of a later milestone; names the milestone. |
| `doc.reserved_attribute` | error | A kind declares an attribute named as a reserved feature key. |
| `doc.placement_twice` | error | One table holds two placement keys; names both. |
| `doc.world_key_in_layer` | error | A world-file-only key or `[layer]` in a layer. |
| `doc.kind_outside_library` | error | `[kind]` in a world or layer file, or a kind a change adds. |
| `doc.change_nested` | error | `[change]` in a kinds file, pack, change file or change body. |
| `doc.remove_outside_change` | error | `remove` or `unset` outside a change body. |
| `doc.remove_above_entity` | error | `remove` or `unset` at a change body's root or on a top table. |
| `doc.file_missing` | error | A file not found: world, layer, change file, library folder. |
| `doc.file_unreadable` | error | A file found but not readable. |
| `doc.layer_file_twice` | error | Two layers name one file, or a layer names the world file. |
| `doc.path_form` | error | An absolute path, a `\`, a part not portable, or a change file not ending in `.toml`. |
| `doc.path_case` | error | A path whose case differs from the name on disk. |
| `doc.include_missing` | error | A library name found in no root; roots searched, by place not path; nearest name. |
| `doc.include_cycle` | error | Libraries that include each other; the loop. |
| `doc.include_shadowed` | info | A library name found in several roots; the one used. |
| `doc.layer_rank` | error | Two layers with one rank, `world`'s included. |
| `doc.upward_ref` | error | A named reference to something in a higher layer; both entities, layers, owners. |
| `doc.unknown_change` | error | A requested change that does not exist; the nearest name. |
| `doc.change_fixed` | error | A change writes `include` or `name`. |
| `doc.change_empty` | warning | A change body that writes nothing. |
| `doc.change_conflict` | error | Two requested changes write or remove one key path; both changes and spans. |
| `doc.change_adds_near` | warning | A change adds an entity whose id is near an existing one; the nearest id. |
| `doc.remove_missing` | error | `remove` or `unset` names nothing in the base. |
| `doc.remove_with_keys` | error | `remove = true` beside other keys, or a key both written and unset. |
| `doc.dangling` | error | A name whose target a change removed; the holding table and the change. |
| `doc.idle_change` | warning | An unrequested inline change has a fault; `data.code` holds its code. |

## Conformance

Each case runs through the command line only (R20): input, action, what must hold. "JSON" is the
canonical JSON of section 8; "origins" is `via merge --origins`.

| Case | Input | Action | Must hold |
|---|---|---|---|
| shuffle | A world, and the same with tables and keys reordered | `via merge` both, then build both | Same JSON; same build digest. |
| syntax_forms | One feature written with an inline `on`, then with `[feature.x.on]` | merge | Same JSON. |
| crlf_bom | The world with CRLF line ends and a BOM | merge, origins | Same JSON; same origins. |
| syntax_v11 | An inline table over two lines | merge | `doc.syntax`. |
| too_deep | `x = ` 65 nested lists; then 64; then a header `[a.b. ...]` of 65 parts | merge | `doc.too_deep` at the 65th, no crash; 64 gives no `doc.too_deep`; the header is `doc.too_deep`. |
| bad_utf8 | A layer holding byte `0xFF` | merge | `doc.syntax` with the byte offset. |
| array_table | `[[feature]]` items each with `id`, the first `id = "mill"` | merge | `doc.syntax`, exit 1, `data.rewrite` `[feature.mill]`. |
| inline_list | `assets = [{ id = "a", weight = 2 }]` on a scatter | merge | No diagnostic. |
| unknown_key | `[feature.lane] widht = 6` | merge | `doc.unknown_key`, line and column of `widht`, hint `width`. |
| not_yet | `[import.a]` in M1 | merge | `doc.not_yet` naming M4. |
| chunk_not_yet | `chunk = 64` in the world file | merge | `doc.not_yet` naming M6. |
| datetime | `seed = 1979-05-27` | merge | `doc.type`. |
| integer_key | `[layer.d] rank = 1.0` | merge | `doc.type`. |
| hex_literal | `[layer.d] rank = 0x10` | merge | JSON `rank` is `16`. |
| missing_key | A layer without `owner`; a world without `extent` | merge | `doc.missing_key` for each, at the header. |
| dup_layers | One feature id in the world and a layer | merge | `identity.duplicate_id` naming both files. |
| dup_cross | `feature.mill` and `instance.mill` | merge | `identity.duplicate_id`. |
| dup_let_field | `[let] height` and `[field.height]` | merge | `formula.shadow`, once; no `doc` or `identity` code. |
| dup_kind | Two libraries declaring `kind.lane` | merge | `identity.duplicate_id` naming both kinds files. |
| world_id | `[feature.world]` | merge | `identity.reserved`. |
| foreign_patch | A layer writing one key on a world feature | merge | `identity.duplicate_id`; hint says edit there or write a change. |
| upward | A rank 0 feature with `starts_at` on a rank 1 thing | merge | `doc.upward_ref` naming both owners. |
| upward_formula | A rank 0 formula reading a rank 1 `let` name | merge | `doc.upward_ref`. |
| kind_rank | A kind formula reading a rank 1 `let`, used by three rank 0 features | merge | One `doc.upward_ref`, subject the kind's key path, three features in `related`. |
| selector_rank | A rank 0 formula with `'kind:lane'`; lanes at rank 0 and rank 1 | merge, build | No `doc.upward_ref`; the selection holds the rank 0 lane only. |
| layer_rank | Two layers with rank 1 | merge | `doc.layer_rank`. |
| layer_rank_zero | A layer with rank 0 and no `[layer.world]` | merge | `doc.layer_rank`. |
| layer_file_twice | Two layers naming one file | merge | `doc.layer_file_twice`. |
| include_lookup | `lib/ways` beside the world, and `ways` under a `--lib` root | merge | The local one is used; `doc.include_shadowed` info. |
| include_env | `ways` under a `--lib` root and under a `VIA_LIB` root; `land` under the `VIA_LIB` root only | merge | The `--lib` `ways` is used, `doc.include_shadowed` info; `land` found; `file` begins `lib:<id>/` for both. |
| include_missing_roots | `include = ["wyas"]`, one `--lib` and one `VIA_LIB` root, `ways` in the latter | merge | `doc.include_missing` naming `beside`, `--lib 1`, `VIA_LIB 1`, `shipped`, nearest `ways`; no absolute path in `diagnostics.json`. |
| include_code_only | `include = ["streets"]`; `streets/` holds code and no `kinds.toml` | merge | No diagnostic; no kinds added. |
| include_case | `include = ["ways"]`, folder on disk `Ways/` | merge | `doc.path_case` on every system. |
| include_cycle | Libraries `a` and `b` including each other | merge | `doc.include_cycle` naming `a`, `b`. |
| include_once | Two libraries both including `land` | merge | `land` read once; no duplicate. |
| span_portable | One world and `--lib` root, from two checkout paths, with a fault in a library | build both | Byte-identical `diagnostics.json`; `file` begins `lib:<id>/`. |
| change_inline | The worked example with `wide` | merge, origins | `width = 6`; its origin line 21, chain ends with change `wide`. |
| change_file | The worked example with `changes/no_cart.toml` | merge | No `instance.cart`. |
| change_twice | `--change wide --change wide` | merge | Same JSON as once; no diagnostic. |
| change_empty | `[change.x]` holding nothing | merge | `doc.change_empty` warning. |
| change_nested | `[change.a.change.b]` | merge | `doc.change_nested`. |
| change_layer | A change writing `[layer.extra]` | merge | `doc.not_yet` naming M4. |
| change_kind_add | A change adding `[kind.new]` | merge | `doc.kind_outside_library`. |
| change_pack_table | A change writing `[asset.cart]` | merge | `doc.unknown_key`, hint `pack`. |
| change_fixed | A change writing `include`; another writing `name` | merge | `doc.change_fixed` each. |
| remove_root | `[change.c] remove = true` | merge | `doc.remove_above_entity`. |
| commute | Changes `a` and `b` on different keys | merge `a b`, then `b a` | Same JSON. |
| conflict | Changes `a` and `b` writing one key, equal values | merge | `doc.change_conflict` naming both. |
| conflict_add | `a` and `b` each add `feature.new`, different keys | merge | `doc.change_conflict`. |
| conflict_remove | `a` removes `feature.x`; `b` writes `feature.x.kind` | merge | `doc.change_conflict`. |
| conflict_place | `a` writes `at`; `b` writes `on` on one feature | merge | `doc.change_conflict`. |
| placement_clear | A change writes `on` on a feature with `path`, `ends_at` and `smooth` | merge | `on` present; `path`, `ends_at`, `smooth` gone. |
| placement_twice | `at` and `on` in one feature, in the base | merge | `doc.placement_twice`. |
| unset_default | A change unsets `width` on a feature whose kind defaults it | merge | JSON: the feature has no `width`; the kind's `defaults.width` is present. |
| remove_missing | `unset = ["nope"]` | merge | `doc.remove_missing`. |
| remove_keys | `remove = true` with `kind = "lane"` beside it | merge | `doc.remove_with_keys`. |
| remove_base | `remove = true` in a world file | merge | `doc.remove_outside_change`. |
| dangling | A change removes a feature an instance is `on` | merge | `doc.dangling` naming the instance and the change. |
| adds_near | A change writes `[feature.frod]` beside `feature.ford` | merge | `doc.change_adds_near` warning, `data.nearest` `ford`. |
| on_partial | A change writes only `t` in `on` | merge | `doc.missing_key` naming `ref`. |
| idle_change | An unrequested inline change writing `widht` | merge | `doc.idle_change` warning, `data.code` `doc.unknown_key`; exit 0. |
| pack_swap | A change file holding `pack = "other"` | merge, build | JSON `pack` is `other`; every other member as without the change; builds. |
| unknown_change | Requesting `dowstream` | merge | `doc.unknown_change`, hint `downstream`. |
| change_name | Change files `Winter.toml`, `no_cart.v2.toml` | merge | `identity.bad_id` each. |
| change_ext | Change file `winter.tml` | merge | `doc.path_form`. |
| path_case | A layer `file` written `Layers/d.toml`, on disk `layers/d.toml` | merge | `doc.path_case` on every system. |
| path_form | A layer `file` written with `\`; another `C:x.toml`; another `aux.toml` | merge | `doc.path_form` each. |
| file_missing | A world file path that does not exist | merge | `doc.file_missing`, located by argument. |
| formula_span | A formula with an unknown name, in a basic string with an escape before it | merge | The diagnostic's column is the name's column in the file. |
| params_free | A step with nested `params` of any keys | merge | No diagnostic; `params` in the JSON as written. |
| json_escape | A `name` holding `é`, a tab and `"` | merge | `é` as raw UTF-8; `\u0009`; `\"`. |
| season_layers (M2) | 6.5's `winter` over a world and a layer `dressing` (owner `props`) | merge with `winter` | `let.snowline` 34, `field.grass` `base` 0 with `@layer` `dressing`, `step.weather` `params` merged; origins' chain ends with change `winter`. |
| season_reuse (M2) | 6.5's world, its generate step `weather` in `dressing`, and a generate step in the world layer that reads neither `snowline` nor `grass` | build, `--change winter`, build, `--change winter` | The world layer's step ran once; `weather` ran twice; the third and fourth builds run no step. |
| step_owner (M2) | A step in `dressing` returning a feature of an unknown kind | build | `authority.unknown_kind`, `file` `step:<id>/document.toml`, chain naming layer `dressing` and owner `props`. |
| placement_clear_derived (M2) | A change writes `at` on a feature with `where` and `holds`; another writes `region` on a path with `route` | merge | `holds` gone; `route`, `starts_at`, `ends_at` gone. |

## Changed from the spike

- Unknown keys were ignored or passed through; now every key is closed or declared (section 7).
- `[[x]]` lists merged by `id` when every item had one, else replaced; now every entity is keyed by
  id and lists are always values.
- Layers appended to lists and merged tables over each other; now a document never writes onto
  another's entity, and a difference is a change.
- Variant and overlay are one concept, `change`; overlays applied in command-line order, now
  changes commute and a clash is an error.
- `include` took file paths and was not followed from an included file; now it takes library names,
  followed to any depth, read once.
- Direction was checked on placement references only; now on every named reference, formulas
  included.
- Removal used sentinel values; now `remove` and `unset`.
- An unknown variant or a layer clash stopped the run with a bare message; now each is a diagnostic
  with a code, span and hint.
- The merged output held absolute paths (SPEC 11, Outputs); now no span or output does (2.1).
- `[params]` is `[let]`.

## Open

| Question | Decided by |
|---|---|
| Where the spike-to-v1 translation for oracle comparison lives. | M0 step 7. |
| Versions of a library. | The first shipped library change that breaks a world; no later than M7. |
| Which layer a change file's added entities belong to, other than `world`. | The first example whose change file must add a table naming something above rank 0. |
| Changes that add, remove or re-rank layers (`doc.not_yet` until then). | M4, with a first pass that settles the layer set before the base. |
| Whether a number key may name a shared constant. | The first example where one change must move several literals together. |
| How a world keeps an included kind with a local difference, other than an always-requested change or a copy under a new name. | The first example that needs it. |
| A separate output file per layer, for a game to load or replace. | `build.md`, M6. |
| Building or swapping one layer alone (a spike lesson). | The first case that needs it; not before M6. |
| Whether an import may bring its own pack; whether an import's imports are followed or rejected; which exit attributes the core compares (challenge 55 records `width`); change files requested by an import. | M4, in this chapter, by `county`. |
