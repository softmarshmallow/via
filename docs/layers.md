# Layers and rings

How this repository is laid out, and the two orders that hold it together. Layers order meaning:
what is written in terms of what. Rings order code: what may depend on what. A path tells you
both.

## Layers

| Layer | What it is | Where it lives |
|---|---|---|
| Data types | The forms content takes: field, shape, graph, asset, instance, mesh | [`docs/ledger/archetypes.md`](ledger/archetypes.md) |
| Archetypes | Families of operation over those forms, each with a contract and no domain word | [`docs/ledger/archetypes.md`](ledger/archetypes.md) |
| Mechanisms | The engine's form of an archetype: a table or a key of the world format, a formula function, a part of the build | `spec/`, the engine crates, `sdk/python` |
| Concepts | A named use of mechanisms: a road, a tarn, a terrace | `lib/`, and a user's own library |
| Worlds | What an author states | `examples/`, and a user's world |

Three rules hold between them.

1. **Each layer is written only in the words of the layer below.** A world uses concepts and
   mechanisms. A concept is kinds as data, and steps that reach a build only through the SDK. A
   mechanism realises an archetype and knows no domain word (principles 5 and 10 of
   [`doctrine.md`](doctrine.md)).
2. **The plain form is always legal.** Whatever a concept says can be said in mechanisms alone.
3. **A thing moves toward the engine only on evidence.** The question that decides it is the
   doctrine's second: can it already be said with what exists? Then it is a library or
   shorthand, not core.

The first two layers are the domain's and are kept in the ledger. The last three are ours and
are kept in the folders named.

## Rings

Code sits in rings, from the centre outward. A ring may depend only on rings nearer the centre.

| Ring | Holds | May depend on | Kept by |
|---|---|---|---|
| substrate | Numbers, diagnostics, documents, formulas, geometry, rasters, the build graph | nothing above it | `tools/layers_check.py` |
| model | The eleven nouns and what they do | substrate | the same; and no domain word in an engine crate (`xtask`, deny-list) |
| door | The step protocol and the SDK: the only way code reaches a build | substrate | `tools/layers_check.py`: the model and the door do not depend on each other |
| command | The `via` program, which joins model and door | model, door | `tools/layers_check.py` |
| library | Concepts: kinds as data, steps as code | the door; a library's program may also link substrate crates | Python under `lib/` imports only the SDK (`xtask`, python-imports); a library's program links neither the model nor the command (`tools/layers_check.py`) |
| world | What an author states; packs bring in assets | libraries and the format | the conformance cases |

Four kinds of thing stand beside the rings and are not depended on by them:

| Beside | Holds |
|---|---|
| contract | `spec/`: the chapters every ring's code answers to |
| proof | Conformance cases and vectors, the runner, the oracle data, the repository checks |
| loader | What reads a build into an engine |
| record | `docs/` and `TODO.md` |

All maths goes through one substrate crate, `via-num` (`xtask`, maths-deps).

Libraries are ordered among themselves, from the ground up: land, terrain, water, vegetation,
ways, town, building, and last the two that read the rest, map and review. A library may name
the kinds of those above it in the table below, never of one below it
(`tools/layers_check.py`).

## Where everything sits

Checked by `tools/layers_check.py`: every crate, spec chapter, library and tracked top-level
folder has a row, and every path exists. For a chapter, "read mostly by" is the crate whose
source cites it most (counted 2026-11-18).

| Path | Layer | Ring | Holds |
|---|---|---|---|
| `crates/via-num` | mechanisms | substrate | The one maths module; numbers as text; named seeds |
| `crates/via-diag` | mechanisms | substrate | Diagnostics: the record and the registry of codes |
| `crates/via-doc` | mechanisms | substrate | Documents: reading, includes, layers and changes merged |
| `crates/via-expr` | mechanisms | substrate | The formula language |
| `crates/via-geom` | mechanisms | substrate | The geometry kernel |
| `crates/via-raster` | mechanisms | substrate | Fields as rasters |
| `crates/via-graph` | mechanisms | substrate | The build graph and its store |
| `crates/via-model` | mechanisms | model | The eleven nouns: fields, features, kinds, authority, rules, surfaces, identity, composition |
| `crates/via-proto` | mechanisms | door | The step protocol |
| `crates/via-step` | mechanisms | door | A step's side of the protocol, for a program written in Rust |
| `sdk/python` | mechanisms | door | The SDK: a step's side of the protocol in Python |
| `crates/via-cli` | mechanisms | command | The `via` program |
| `lib/land` | concepts | library | The ground and what is levelled into it |
| `lib/terrain` | concepts | library | The land settled under a few stated facts |
| `crates/via-terrain` | concepts | library | `lib/terrain`'s program: the erosion model |
| `lib/water` | concepts | library | Running and standing water |
| `lib/vegetation` | concepts | library | Where things grow, by cause |
| `lib/ways` | concepts | library | Lanes, roads and where they cross water or each other |
| `lib/town` | concepts | library | Streets, blocks, lots and houses |
| `lib/building` | concepts | library | A building as storeys, rooms, doorways and flights |
| `lib/map` | concepts | library | What a game reads from the world that changes no field |
| `lib/review` | concepts | library | Where a build is judged from, and the measures beside the eye |
| `lib/packs` | worlds | world | The starter and primitive packs: imported assets with their facts |
| `examples` | worlds | world | The example worlds, each with its change files and its views |
| `spec/vocabulary.md` | mechanisms | contract | The eleven nouns and every key; read mostly by `via-doc` |
| `spec/numbers.md` | mechanisms | contract | Numbers; read mostly by `via-num` |
| `spec/diagnostics.md` | mechanisms | contract | Diagnostics; read mostly by `via-cli` and `via-diag` |
| `spec/documents.md` | mechanisms | contract | Documents; read mostly by `via-doc` |
| `spec/formulas.md` | mechanisms | contract | Formulas; read mostly by `via-expr` |
| `spec/geometry.md` | mechanisms | contract | Geometry; read mostly by `via-model` and `via-geom` |
| `spec/fields.md` | mechanisms | contract | Fields; read mostly by `via-raster` and `via-model` |
| `spec/graph.md` | mechanisms | contract | The build graph; read mostly by `via-cli` and `via-model` |
| `spec/authority.md` | mechanisms | contract | Kinds and authority; read mostly by `via-model` |
| `spec/rules.md` | mechanisms | contract | Placement and the three core rules; read mostly by `via-model` |
| `spec/surfaces.md` | mechanisms | contract | Surfaces; read mostly by `via-model` |
| `spec/identity.md` | mechanisms | contract | Names and identity; read mostly by `via-model` |
| `spec/derived.md` | mechanisms | contract | Derived results; read mostly by `via-cli` and `via-model` |
| `spec/composition.md` | mechanisms | contract | Composition; read mostly by `via-model` |
| `spec/assets.md` | mechanisms | contract | Assets and packs; read mostly by `via-model` |
| `spec/build.md` | mechanisms | contract | The build directory; read mostly by `via-cli` and `via-model` |
| `spec/protocol.md` | mechanisms | contract | The step protocol; read mostly by `via-cli` |
| `spec/conformance.md` | - | proof | What a conformance case is |
| `spec/cases` | - | proof | The conformance cases |
| `spec/vectors` | - | proof | Test vectors |
| `tools/conformance` | - | proof | The conformance runner |
| `tools/packgen` | - | proof | The pack generator for cases |
| `tools/ledger_check.py` | - | proof | The ledger's check and counts |
| `tools/layers_check.py` | - | proof | This page's check |
| `tools/map_render.py` | - | loader | A picture of the player map from its data |
| `fixtures` | - | proof | The oracle data the examples are compared with, and the kernel's fixtures |
| `crates/xtask` | - | proof | The repository checks |
| `engines/godot` | - | loader | The Godot loader, and the tool that renders and walks the views |
| `engines/gltf` | - | loader | A second reader of the build directory |
| `docs/ledger` | archetypes | record | The vocabulary of a game world, the data types and the archetypes |
| `docs` | - | record | Charter, doctrine, plan, challenges, design notes, reports, references |

## The engine's form of each archetype

One row per archetype of [`docs/ledger/archetypes.md`](ledger/archetypes.md): the mechanism that
realises it, and where that is specified. Checked by `tools/layers_check.py`: every archetype has
a row, and every name in backticks exists.

| Archetype | The engine's form | Specified in |
|---|---|---|
| Field | `[field.<name>]` with a `base`; noise by `fbm()` and `ridged()` | `fields.md 1`, `fields.md 3`, `formulas.md 8` |
| Shape | `[feature.<id>]` with `at`, `path` or `region` | `geometry.md 2` |
| Asset | `[asset.<id>]` in a pack, with measured facts | `assets.md 2`, `assets.md 4` |
| Derivation | Local: a formula over `f()`. Focal: `slope()`, `near()`, `smoothed_along()`. Zonal: `under()`. Global: `where`, `holds`, `route` | `formulas.md 8`, `derived.md 1` |
| Simulation | A step under authority | `derived.md 5`, `protocol.md 1` |
| Profile | `[kind.<k>.effect.<id>]`: a `value` and a `mask` over the distance from a feature | `authority.md 2` |
| Network | `[network.<id>]`; a way stated by its ends with `starts_at`, `ends_at` and `route` | `geometry.md 8`, `derived.md 3` |
| Area partition | `[cells.<id>]`; the areas a network encloses | `geometry.md 8` |
| Placement | `[instance.<id>]`, placed at a spot, or on or along another thing with an offset and a facing | `rules.md 2`, `rules.md 3` |
| Linear placement | `[run.<id>]` | `rules.md 5` |
| Scatter | `[scatter.<id>]` | `rules.md 7` |
| Recipe | `[set.<id>]`; a library form that lowers to plain rules | `rules.md 4`, `rules.md 10` |
| Grammar | None in the engine. A library's step does it (`lib/building/lay.py`) | `protocol.md 1` |
| Representation switch | `[kind.<k>.surface.<name>]` with a `cut` | `surfaces.md 1` |
| Meeting | `[kind.<k>.mount.<name>]` with `meets` and `connector`; an `exit` of a world built apart | `authority.md 6`, `surfaces.md 3`, `composition.md 7` |
| Overlay | `[layer.<id>]`, `[change.<name>]`, `[field.<name>.then.<id>]`; hand edits by `[pin.<id>]`, `[clear.<id>]`, `[lock.<id>]` | `documents.md 5`, `documents.md 6`, `identity.md 4` |
| Guidance | None in the engine. A library kind carries a view (`review.bookmark`) | `build.md 9` |
| Measure | A lint step under `checks`; a `[budget]` per chunk | `protocol.md 1`, `build.md 11` |
| Markup | Free attributes and `tags`, handed on unread; a kind's `output` | `documents.md 7`, `build.md 9` |
| Delivery | The build directory, cut by `chunk` | `build.md 2`, `build.md 5` |

## What the layout shows

Each archetype followed through the layers, and the holes that shows. Written by
`python3 tools/layers_check.py --write` and checked to be current.

<!-- shows: begin -->

Counted over 865 ledger rows, 58 kinds, 20 steps, 29 example worlds and 1011 conformance cases.

| Archetype | Ledger rows that are ours: shown, partly, none | Chapters, with their cases | Kinds | Steps | Examples that use it |
|---|---|---|---|---|---|
| Field | 13, 5, 19 | fields 58, formulas 52 | 0 | 1 | 29 |
| Shape | 3, 2, 5 | geometry 85 | 0 | 0 | 29 |
| Asset | 13, 6, 28 | assets 89 | 0 | 0 | 19 |
| Derivation | 10, 9, 28 | formulas 52, derived 33 | 2 | 1 | 19 |
| Simulation | 4, 1, 12 | derived 33, protocol 88 | 3 | 2 | 2 |
| Profile | 14, 11, 10 | authority 57 | 19 | 0 | 20 |
| Network | 8, 3, 12 | geometry 85, derived 33 | 5 | 1 | 19 |
| Area partition | 7, 13, 18 | geometry 85 | 6 | 0 | 3 |
| Placement | 3, 0, 7 | rules 68 | 0 | 0 | 15 |
| Linear placement | 6, 5, 13 | rules 68 | 3 | 4 | 10 |
| Scatter | 10, 13, 18 | rules 68 | 0 | 1 | 11 |
| Recipe | 3, 7, 10 | rules 68 | 0 | 1 | 4 |
| Grammar | 2, 5, 10 | protocol 88 | 10 | 4 | 9 |
| Representation switch | 5, 2, 11 | surfaces 92 | 11 | 0 | 8 |
| Meeting | 22, 36, 38 | authority 57, surfaces 92, composition 87 | 14 | 1 | 15 |
| Overlay | 5, 3, 6 | documents 76, identity 57 | 0 | 0 | 29 |
| Guidance | 1, 2, 61 | build 59 | 0 | 0 | 0 |
| Measure | 7, 5, 18 | protocol 88, build 59 | 0 | 6 | 15 |
| Markup | 3, 2, 7 | documents 76, build 59 | 4 | 0 | 29 |
| Delivery | 5, 6, 23 | build 59 | 0 | 1 | 1 |

**Archetypes with no form in the engine:** Grammar, Guidance.

**Kinds no example writes (11):** `building.footing`, `building.pipe_leg`, `building.platform`, `building.roof`, `building.yard`, `land.firm_pad`, `terrain.terrain_course`, `vegetation.clearing`, `vegetation.green`, `vegetation.wood`, `water.weir`.

**Steps that do in code what the engine has a form for:** `lib/building/row.py` (Linear placement, Grammar, 454 lines); `lib/building/pipe.py` (Linear placement, Grammar, Meeting, 509 lines); `lib/land/uplands.py` (Field, 445 lines); `lib/town/houses.py` (Recipe, 1150 lines); `lib/town/kit.py` (Linear placement, 616 lines); `lib/town/streets.py` (Network, 936 lines); `lib/vegetation/clumps.py` (Scatter, 346 lines); `lib/water/fill.py` (Derivation, 123 lines); `lib/ways/parapet.py` (Linear placement, 161 lines).

**Concepts as data and as code:** 1831 lines of kinds, 13855 lines of steps (tests left out).

**One library naming another's kinds:** `building` names `town.terrace`; `review` names `town.main_street`; `review` names `town.street`; `review` names `town.waterfront`; `review` names `ways.crossing`; `review` names `ways.trail`; `ways` names `water.beck`.

**Keys of the format that no example, library or pack writes (6):** `[lock.<id>]`, `max_grade`, `inputs`, `[defaults]`, `engine`, `repeat`.

<!-- shows: end -->

## How it grows

- **A new term** is a row of the ledger under an existing archetype.
- **A new genre** is rows and a table of substitutions over the same archetypes.
- **A new concept,** ours or a user's, is a kind in a library, written as data. It gets a step
  only where it needs an algorithm, and the step reaches the build through the SDK alone. One
  of ours names its archetypes in section 25 of the ledger
  ([`docs/ledger/names.md`](ledger/names.md)).
- **A new library** is a folder under `lib/` with a `kinds.toml`, and a row above.
- **A new mechanism** is admitted only when a concept cannot be written with what exists
  (doctrine section 4, question 2). It realises an archetype, takes no domain word, and is
  written chapter first, cases next, code last ([`CONTRIBUTING.md`](../CONTRIBUTING.md)).
- **A new archetype** must meet the five criteria of
  [`docs/ledger/archetypes.md`](ledger/archetypes.md), and is logged there.
- **A new crate** is given a ring, and depends only inward.

## Checks

| Check | Holds |
|---|---|
| `cargo run -p xtask -- check` | Everything below, and the rest of the repository checks: no domain word in the engine, libraries import only the SDK, all maths through `via-num`. `cargo test --workspace` runs the same two record checks |
| `python3 tools/layers_check.py` | This page's tables, the rings, the order among libraries, and section 25 of the ledger. With `--write` it refreshes "What the layout shows" and what each kind is made of |
| `python3 tools/ledger_check.py` | The ledger's rows, pointers and counts. With `--write` it refreshes the counts |
