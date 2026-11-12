# Vocabulary (v1)

Every word a user writes, on one page, with the chapter that defines it (`spec/<chapter>.md`). A
key means one thing in every table it appears in (principle 9); where it appears in several, it has
one row and the others point to it. A chapter may not use a key, table or selector this page does
not list; a new one is added here first. Exempt: formula function names (listed in `formulas`),
diagnostic codes (listed in `diagnostics`), and anything inside `params` (handed on unread).
Authority: `docs/doctrine.md` (principles 1-13, rulings R1-R27, section 6 applied as recommended on
2026-11-13).

Every example is flat TOML keyed by id (R1): `[feature.mill]`, never `[[feature]]`. A list whose
items a change would name is a table of tables with ids (`[kind.lane.effect.bed]`). A list of plain
values (`assets`, `path`) is a value, replaced whole. Which tables merge and which are replaced:
`documents.md` 6.2 (section 2.5). One exception to R1: a pack's taxonomy nodes are keyed by path
(`[taxon."<path>"]`, `assets.md` 5).

Keys use British spelling, as prose does (`colour`, `base_centre`; `assets.md` 1.1).

"(open)" marks what is undecided, with what decides it.

## 1. The eleven nouns

| Noun | v1 meaning | Chapter |
|---|---|---|
| frame | Where coordinates are measured from. The world has one; an import, an instance or a set may have its own. | geometry |
| field | A value at every point, read by the core through its `role`. | fields |
| feature | A point, a path or a region with an id, a kind and attributes. | geometry |
| kind | What a feature does to fields, written as data in a library. | authority |
| authority | How firmly a statement holds: `derived`, `shaped`, `anchored`. | authority |
| asset | An id a pack binds to a shape and its facts. | assets |
| instance | One asset, or one set, in one place. | rules |
| rule | Anything that lowers to plain instances or features: `set`, `run`, `scatter`. | rules |
| layer | One document of a world, with an owner. It refers downward only. | documents |
| change | A named difference over a world, written inline or in a file. Hand edits are changes too. | documents |
| step | The project's own code, run at a declared point (`when`), with declared inputs. | protocol |

Build order in one line: fields from their bases, then features by authority, then rules, then
checks.

Two words for what makes things:

| Word | Meaning | Chapter |
|---|---|---|
| producer | A `run`, a `scatter` or a step: something that makes instances the author did not write one by one. A set is not a producer: its items take the class of what places it, so `by` never names a set (`identity.not_a_producer`). | rules |
| maker | Anything whose output is named `<maker>#<place key>`: a producer, a network, a cells table, or a feature written with `where`. | identity |

## 2. Rules for every document

### 2.1 Ids and place names (`identity`)

- An id matches `[a-z][a-z0-9_]*`, at most 48 bytes. Lowercase only, so two ids never collide as
  build file names on a case-insensitive file system (R20). An id holds no `.`, `:`, `#`, `@` or
  `/`.
- One namespace per world holds every field, feature, instance, set, run, scatter, pin, clear, lock,
  import, step and layer. Two of them with one id is an error that names both. Kinds (in libraries)
  and changes each have their own namespace. A pack's asset ids are the pack's own namespace
  (`assets.md` 2.1). A `let` name that equals a field name is an error.
- Reserved ids: `world` (the world file's own layer). Reserved place names: `start` and `end` (the
  ends of a path) and `ground` (an instance's footprint feature, `rules.md` 2.1). No mount, kind
  surface or set item may use them (`identity.reserved`).
- A place name is ids joined by dots, resolved left to right. The first part is an id in this world.
  After an import, the rest resolves inside the imported world. After a path, `start` or `end`.
  After an instance, `ground`, a mount of its asset or an item of its set. After a feature, a mount
  or a surface of its kind (one namespace, `surfaces.md` 1). One namespace means no part is
  ambiguous. A part that names nothing is an error that names the nearest match.
- Selector prefixes (`kind:`, `role:`, `tag:`) cannot collide with an id, since ids hold no `:`.
- Produced names (`<maker>#<place key>`) are never written in text (R3, `identity.md` 3). A value
  holding `#` in a key that takes a name is `identity.key_in_text`.

### 2.2 Order (`documents`, `numbers`)

The order of tables and keys in a file never changes a result. Every tie that no key settles breaks
by id, compared bytewise as UTF-8 (principle 7, R19's ordered maps). Conformance holds one case for
it: the same world with its tables shuffled builds to the same digest.

### 2.3 Closed keys (`documents`)

Top-level tables, and the keys of every table except a feature and `params`, come from the closed
lists on this page. An unknown one is `doc.unknown_key` with the nearest listed word as its hint
(`documents.md` 7).

A feature's keys are the reserved keys below plus the attributes its kind declares (in `defaults` or
`requires`). An undeclared key is an error that names the nearest reserved key or declared
attribute. A kind that declares an attribute named as a reserved key is an error.

Reserved on a feature: `kind`, `authority`, `at`, `on`, `path`, `region`, `where`, `follows`,
`smooth`, `starts_at`, `ends_at`, `face`, `remove`, `unset`.

### 2.4 Values (`numbers`, `formulas`)

The **Takes** column says what a key accepts:

| Type | Accepts |
|---|---|
| `length` | A plan coordinate or plan distance in metres, at most three decimals, held as integer millimetres on the lattice (`numbers.md` 3.1). |
| `point` | `[x, y]`, two lengths. |
| `number` | A finite double (`numbers.md` 3.3). |
| `integer` | A TOML integer. |
| `bool` | `true` or `false`. |
| `string` | Free text, or a word from a list the row gives. |
| `name` | An id or a place name (2.1). |
| `formula` | A formula string or a number (below). |

Lists and tables of these are written as such ("4 lengths", "list of points").

- Numbers are finite. TOML's `inf` and `nan` are rejected with a code (R26).
- A length is metres with at most three decimals. More is an error, never rounded (R24: integer
  millimetres in the core). Heights are not lengths: they are field values (`numbers.md` 3.1).
- A formula is a string in our closed expression language (R6), always marked by a leading `=`:
  `base = "= 20 + 0.06 * y"`. A number is a constant formula. A key that takes a formula takes no
  other string; an unmarked string there is `formula.unmarked`. A parser knows a formula by its key;
  in a feature attribute, which has no Takes, the mark decides (`formulas.md` 1, `documents.md` 7.1).
  Inside a formula, `t` is the fraction along a path and `d`, `sd` the distance and signed distance
  from a feature. Formulas read fields and `let` names by name, never by role. The function list has
  no `where` (it is a feature key).

### 2.5 Merging (`documents`)

The world file and its layers form **the base**; requested changes merge onto it before the graph
is built (`documents.md` 5, 6).

- **Layers do not merge.** Every entity is declared in one document, and no document writes onto an
  entity declared in another. An id declared in two documents, even if one only adds keys, is
  `identity.duplicate_id`; the hint says to edit it where it is declared, or to write a change
  (`documents.md` 5). A difference over a lower layer's text is a change.
- **Includes do not merge either.** A library adds kinds to the world's one kind namespace; two
  libraries that declare one kind are `identity.duplicate_id`. A world writes onto an included kind
  only through a change (`documents.md` 4).
- Which tables a change merges key by key and which it replaces whole is `documents.md` 6.2's one
  list: `on`, `follows`, `clump` and every list item are values, replaced whole; every other table
  merges to any depth, however it was written.
- `remove = true` deletes an entity or the sub-table it is written in, with no other key beside it.
  `unset = ["key", ...]` deletes keys from the table it is written in. Neither acts on a whole world
  or a top table (`doc.remove_above_entity`); one that names nothing in the base is
  `doc.remove_missing` (`documents.md` 6.1).
- The placement keys `at`, `on`, `path`, `region`, `where` and `follows` exclude each other. Writing
  one in a change deletes the others in that table; writing one other than `path` also deletes
  `starts_at`, `ends_at` and `smooth` (`documents.md` 6.1).
- Changes commute: they apply all at once to the base. Two requested changes whose write sets share
  a key path, or where one path is a prefix of the other, are `doc.change_conflict` naming both, even
  when the values are equal (`documents.md` 6.3).
- What a change body may hold (`documents.md` 6.1):

| In a change body | Result |
|---|---|
| a key on an existing entity; a new entity; `remove`; `unset` | Allowed. |
| top-level `pack`, `seed`, `extent`, `resolution` | Allowed. `pack` swaps the whole pack (the ford pack-swap case). |
| `include`, `name` | `doc.change_fixed`: the include list is settled before any change applies. |
| `[layer.*]` | `doc.not_yet` (M4): adding, removing or re-ranking a layer by change waits for a first pass that settles the layer set. |
| a new kind | `doc.kind_outside_library`. Writing onto or removing an existing kind is allowed. |
| `[asset.*]`, `[material.*]`, `root` | `doc.unknown_key`, hint `pack`: a change reaches a pack only by swapping it. |
| `[change.*]` | `doc.change_nested`. |

- An unknown change name is `doc.unknown_change` with the nearest name. A reference left dangling by
  `remove` is an error that names the table holding it.

## 3. Keys and tables

### Document (`documents`, `numbers`)

| Key | Takes | Meaning | Chapter |
|---|---|---|---|
| `name` | string | The world's name; names its build. | documents |
| `extent` | 4 lengths | `[x0, y0, x1, y1]`, a rectangle in the world frame, `x1 > x0` and `y1 > y0`: the world's on the document, the raster's on a field (Fields). | numbers |
| `resolution` | length | Spacing of field samples. Greater than 0. | numbers |
| `seed` | integer (world), string (rule) | What randomness is drawn from. On the world: an integer `0` to `2^63 - 1`, default `0`, used as an unsigned 64-bit value and mixed into every rule's stream; changing it re-rolls everything derived. On a scatter: a name in the id grammar, default the scatter's id; write one only to change it. A run or a set takes none: `seed` there is `doc.unknown_key` (`numbers.md` 7.1, `rules.md` 4 and 5). Mixed by SplitMix64 (Steele, Lea and Flood, 2014) as in the carried-over seed derivation; byte order in `numbers.md` 7.2. | numbers |
| `include` | list of strings | Libraries, by name, never by path; each adds its kinds to the one kind namespace. A name is looked up in `lib/` beside the world file, then each `--lib` root in order, then the shipped `lib` beside the `via` executable's folder; first match wins (`documents.md` 4). Two libraries that declare one kind are `identity.duplicate_id`. Versions of a library: (open), decided by the first shipped library change that breaks a world, no later than M7. | documents |
| `pack` | string | The pack that binds asset ids: a path if it holds `/` or ends in `.toml`, else a name looked up as `assets.md` 1.2 says. A world never names an asset file (principle 6). | assets |
| `[layer.<id>]` | table | One document of the world: `file`, `owner`, `rank`. The world file's own text is the layer `world`. | documents |
| `file` | string | A path relative to the file that names it: on a layer, an import or a field. In a pack, an asset's or material's `file` (and `detail`) resolves against `root`. A field's `file` is a declared input of the graph (principle 7). Whether principle 6 ("a world never names a file") admits a field's `file` is (open): `fields.md` 4 reads it as forbidding asset files only; a doctrine ruling on principles 6 and 7 decides it. | documents |
| `owner` | string | Who answers for a layer, free text. The build does not check it; it travels in the chain of every diagnostic raised from that layer (`documents.md` 5). | documents |
| `rank` | integer | A layer's place, lowest first, unique. `world` is 0 unless written. A layer names only what is at or below its rank. | documents |
| `[let]` | table of formulas | Named values formulas read. Also `[field.<n>.let]` and `[kind.<k>.let]`. Resolved by dependency; a cycle is an error. | formulas |
| `remove`, `unset` | bool; list of strings | In a change body only: delete the entity or sub-table; delete these keys from it (2.5, `documents.md` 6.1). A layer never writes onto another document's entity, so it has nothing to remove. | documents |
| `chunk`, `[budget]` | length; table | Side of a chunk; limits per chunk (`instances`, `triangles`). | build |

### Fields (`fields`, `formulas`)

| Key | Takes | Meaning | Chapter |
|---|---|---|---|
| `[field.<name>]` | table | Declares a field. Any name. | fields |
| `role` | string | What the core uses it for: `surface`, `material`, `keep_out`, or none. See the roles table. | fields |
| `type` | string | `scalar` (default), `weights` (a weight per material), `volume` ((open): M5 decides its keys). | fields |
| `base` | formula | The value before any row. Default 0. Excludes `file`. On a kind surface, its value (Kinds). | fields |
| `file` | string | Base read from a raster, with `channel`, `range`, `extent`, `outside`, `filter`. Formats: the `.npy` array file (M1) and PNG (M2), `fields.md` 4.1. | fields |
| `channel` | string | PNG only: `grey`, `r`, `g`, `b` or `a`, the one channel read. Default `grey` for grey pictures; required for RGB and RGBA. | fields |
| `range` | 2 numbers | PNG only: `[lo, hi]`, the values level 0 and the format's top level (255 or 65535) stand for, whatever the picture's own lowest and highest levels. Default `[0, 1]`. | fields |
| `extent` | 4 lengths | See Document. On a field with `file`: where the raster's corner samples lie. Default the world's `extent` (Document). | fields |
| `outside` | number | The value beyond the raster's extent. No default: required with `file`. | fields |
| `filter` | string | How the raster is sampled: `nearest`, `linear` (default), or `cubic` (M2). | fields |
| `unit` | string | The unit of the value, nothing more. Closed list: `m`, `deg`, `ratio` (`fields.md` 2.4). | fields |
| `[field.<name>.then.<id>]` | table | A row on the field's starting value: `value`, `mask`, `paint`, `order`. | fields |
| `value` | formula | On a row or effect: the new value where it applies. | fields |
| `mask` | formula | On a row or effect: its weight at a point, 0 to 1. Default 1. | fields |
| `paint` | name | On a row or effect of a `weights` field: the material it adds weight to, by `mask`. | fields |
| `order` | integer | On `then` rows and effects: their place where no earlier key orders them, lower first, default 0 (`authority.md` 4). | authority |
| `taxon`, `tags` | string; list of strings | Where it is filed (one path); marks that cut across (many). Same on assets. | assets |

| Role | Type | How the core reads it | Several fields with it |
|---|---|---|---|
| `surface` | `scalar` | The ground under every `at` and `on`; terrain export. | Exactly one at the top level: the terrain. A second is `surface.two_terrains`, from M1 and always (`fields.md` 2.1, `surfaces.md` 1). Every other surface is a kind surface (Kinds). |
| `keep_out` | `scalar`, 0 to 1 | A scatter multiplies its density by `1 - keep_out` itself; a formula does not repeat it. | The maximum over all of them. |
| `material` | `weights` | Material weights handed to the loader (an engine's terrain layers). | At most one (`field.role_twice`, `fields.md` 2.1). |

Any other combination of role and type is an error. A role the build needs and the world lacks
(`surface`, once anything is placed) is an error, and `diagnostics.json` is still written.

| Word | Meaning | Chapter |
|---|---|---|
| entry | One contribution to one field's value: a `then` row (written or returned by a step), or one (effect, feature) pair on each field the effect writes. A field's value is its base, then its entries in one total order (`authority.md` 4). Not a change: a change is a difference over the world's text. | authority |

How a field's value is built: the base, then every entry, in one total order per field that
`authority.md` 4 fixes. In short:

1. **The head**, every `then` row, written or returned by a step, before every effect
   (`authority.md` 4.1): by layer rank; then source (text rows, then each step's rows, steps by
   id); then `order`; then the row's place name.
2. **Effects** (`authority.md` 4.2): by authority (derived, shaped, anchored) across the whole
   world; then nesting (`authority.md` 4.3); then `order`; then layer rank; then the feature's place
   name, then the effect id.

The placement of the head is ours. Nesting and `order` before layer rank is `authority.md`'s reading
of R21, **pending the lead's ruling** (an amendment to R21 citing challenge 36 and R18); until it is
recorded, `authority.md` 4 is the source and this summary follows it.

### Features (`geometry`, `authority`)

| Key | Takes | Meaning | Chapter |
|---|---|---|---|
| `[feature.<id>]` | table | Declares a feature. Its keys: 2.3. | geometry |
| `kind` | name | Its kind. Without one it is a guide: a shape that does nothing, a line to measure from or an outline to plant in. | authority |
| `authority` | string | `derived`, `shaped`, `anchored`. On a kind: its features' default, itself `derived` by default. On a feature: overrides its kind's. | authority |
| `at` | point | A coordinate in the frame of the table it is in: the world's, or a set's for an item. Plan only: height comes from the surface beneath (`surfaces`). | geometry |
| `on` | inline table | A place on a named or selected thing: see Placement. | geometry |
| `path` | list of points | A polyline. With `starts_at` / `ends_at`, every point other than the ends those give. | geometry |
| `region` | list of points | A closed polygon. Read counter-clockwise; `t = 0` is its first written point. | geometry |
| `where` | formula | A region written as a condition: where the formula is above 0 (a yes/no means where it is yes). Every piece is kept, with its holes; a piece smaller than one grid cell is kept and reported (`geom.mask_tiny`, info). The only piece dropped is one whose rings have zero area after rounding to the 1 mm lattice (`geom.mask_dropped`, warning; `geometry.md` 3.7). | geometry |
| `smooth` | bool | Smooth a path through its written points by centripetal Catmull-Rom (Yuksel, Schaefer and Keyser, 2011). `t` is measured along the final curve. Default `false`; a kind cannot change it. Taken only with `path`, else `geom.smooth_not_path` (`geometry.md` 2.4). | geometry |
| `starts_at`, `ends_at` | name | The place a path begins or ends at. A path meets another thing at its ends or by `follows`, nowhere else. | geometry |

Axes: `x` east, `y` north, metres; heights up. A `face` in degrees is a bearing: clockwise from
north, 0 north, 90 east. The loader maps these to the engine's axes. This convention is ours.

### Kinds, effects, kind surfaces and kind mounts (`authority`, `surfaces`)

| Key | Takes | Meaning | Chapter |
|---|---|---|---|
| `[kind.<name>]` | table | Declares a kind, in a library. The core names no kind. | authority |
| `authority` | | See Features. | authority |
| `defaults` | table | Attribute values its features have unless they write them. Declares each attribute. | authority |
| `requires` | list of strings | Attributes each of its features must write. Declares each too. | authority |
| `radius` | length | Distance beyond which a thing does nothing. On a kind or effect: its effects are not evaluated beyond it. On a clear: nothing beyond it is removed. In `clump`: the disc members fall in, or the reach of `around`. | authority |
| `exit` | bool | `true`: its point features are exits, places where something leaves the world. The core owns this flag and the word (R9); the library owns the kind's name. Which of an exit's attributes the core compares: (open) decided in `documents` for M4 (challenge 55 records `width`). | documents |
| `output` | string | A name under which the build hands this kind's features to the loader (bookmarks, map icons). Any id, chosen by the library; the core gives it no meaning and knows no value of it. Kinds with one output share one list in the manifest's `outputs` (`build.md` 9.2). | build |
| `names` | list of strings | The attributes whose values are place names, each declared in `defaults` or `requires` (`authority.names_undeclared` otherwise). Each value is resolved and direction-checked as `on.ref` is, and written to `features.json` resolved (`authority.md` 1.2). Ford's `bookmark` kind needs it in M1. | authority |
| `[kind.<k>.effect.<id>]` | table | One change to fields near each feature: `field`, `value`, `mask`, `paint`, `radius`, `max_change`, `order`. | authority |
| `field` | string | On an effect: the fields it writes, as `role:<role>` or `tag:<tag>`, never by name, so a library works with whatever the world calls its fields. It writes every world field that matches; never a kind surface. | authority |
| `max_change` | formula | The most an effect may move a value at any point: a per-feature formula, 0 or more, that may read attributes but not `v`, `d`, `sd` or `t` (`formulas.md` 6.1). Past it is `authority.max_change` (error, principle 3), and the entry is still applied in full (`authority.md` 2.2). | authority |
| `[kind.<k>.surface.<name>]` | table | A bounded surface every feature of the kind owns: `base` (its height, a per-point formula), and `bound` on a path owner. Named `<feature>.<name>`. No `then` rows, no tags; no effect writes it. Key words pending `surfaces.md` Open 1. | surfaces |
| `bound` | formula | On a kind surface of a path owner: the strip's half width, metres, a per-feature formula above 0. A region owner has none; a point owner owns no surface (`surface.owner_shape`). | surfaces |
| `[kind.<k>.mount.<name>]` | table | A place every feature of the kind has (a deck's two ends): `t`, `offset`, `face`, `surface`, `meets`, `connector`. No `at`. Named `<feature>.<name>`; shares one namespace with the kind's surfaces. | surfaces |
| `surface` | name | On a kind mount: the owner's kind surface it lies on. Default the terrain. | surfaces |
| `meets` | name | On a kind mount: another mount of the same kind; the pair is one meeting. | surfaces |
| `connector` | string | On a kind mount: what joint it is, a free word compared only for equality (M5). On an asset mount: `doc.not_yet` until M5. | surfaces |

| Word | Meaning | Chapter |
|---|---|---|
| attribute | A key of a feature that its kind declares, in `defaults` or `requires` (`authority.md` 1.1). | authority |
| name attribute | An attribute listed in its kind's `names`: its value is a place name, not text (`authority.md` 1.2). | authority |

On a kind mount, `face` is a number only: degrees clockwise from the owner's direction of travel at
`t` (`surfaces.md` 3.1). The "word without `on`" rule does not reach it; a word there is
`surface.mount_face`.

### Placement (`geometry`, `rules`, `identity`)

One grammar for features, instances, sets, rules and hand edits.

| Key | Takes | Meaning | Chapter |
|---|---|---|---|
| `at` | | See Features. | geometry |
| `on` | `{ ref, t, offset }` or `{ of, class, near, t, offset }` | On an authored thing by name (`ref`), or on a produced thing selected by a point near it (`of`). Never both (`identity.on_both`). Runs take `span` in place of `t`. Scatters take no `on`: they use `within`. | geometry |
| `ref` | name | In `on`: the place name it is placed on (2.1): a feature, an instance, a mount, a path end, a set item, an import's place. Never a produced name (`identity.key_in_text`). | geometry |
| `of` | name | In `on` or `within`: the maker selected from, by its id. | identity |
| `class` | string | In `on` or `within`: one class of the maker's output: `instance`, `node`, `edge`, `area`, `cell`, `rest`, `piece`, `feature` (section 4). Needed when the maker's output has more than one class (`identity.of_ambiguous`). | identity |
| `near` | point | In `on` or `within`: the selection resolves to the candidate of `class` nearest this point, in the frame of the table it is in. | identity |
| `t` | number | Fraction along, 0 to 1. Outside that range is `geom.t_range`. | geometry |
| `span` | 2 numbers | On a run: `[from, to]`, fractions along, `0 <= from < to <= 1`. | rules |
| `offset` | length | Distance to the left of the direction of travel. Default 0. | geometry |
| `face` | number or string | Which way the thing looks: a bearing (Features), or one of `along`, `toward`, `away`, relative to `on` (`geometry.md` 2.6, a closed list). A word without `on` is `geom.face_needs_on`. On a thing with a facing, a number is added to the host's facing (`rules.md` 3.3). | geometry |

| `on` resolves to | `t` | `offset` |
|---|---|---|
| a path, or a produced edge or path feature | Fraction of arc length from its start. | Left of travel from start to end. |
| a region, or a produced area, cell, rest or piece | Fraction of the outline from its first point (a produced region: its least vertex), counter-clockwise. | Left of travel, so inward. |
| a point, an instance, a mount, a path end or a set item | Error: no length (`geom.no_length`). | Error: no length. To stand beside it, place a set on it: the set's items take `at` in its frame (`rules.md` 3.2). |

### Instances and sets (`rules`)

| Key | Takes | Meaning | Chapter |
|---|---|---|---|
| `[instance.<id>]` | table | One asset or set placed by hand: `asset` or `set`, `at` or `on`, `face`, `scale`, `tint`, `ground`, `margin`. | rules |
| `asset` | name | An asset id the pack binds. | assets |
| `set` | name | A set to place, in place of `asset`. On a set item: a nested set. | rules |
| `[set.<id>]` | table | A template: items placed as one unit wherever an instance, run or pin names it. Never placed by itself. Items: `[set.<id>.item.<name>]` with `asset` or `set`, `at` (in the set's frame), `face` (added to the set's), `scale`, `tint` (multiplied by the placing thing's). An item is named `<instance>.<name>`. | rules |
| `scale`, `tint` | number, or 2 numbers on a scatter | On an instance, run or set item, the value (default 1). On a scatter, a number fixes it and `[lo, hi]` draws each instance's value: the pair rule, which covers `scale`, `tint` and `clump.members`; elsewhere a pair is `place.range` (`rules.md` 1.1). | rules |
| `ground` | name | A kind: the instance's footprint is also a feature of that kind, grown by `margin`, named `<instance>.ground`. | rules |
| `margin` | length | How far `ground` grows the footprint. Default 0. Only with `ground`. | rules |

### Rules (`rules`)

| Key | Takes | Meaning | Chapter |
|---|---|---|---|
| `[run.<id>]` | table | An `asset` or `set` repeated along a path or outline: `on = { ref, span, offset }` or `{ of, class, near, span, offset }`, `count` or `spacing`, `ends`, `face`, and `scale`, `tint`, `ground`, `margin` copied to every member. No `seed`. | rules |
| `count` | integer | How many. A literal, never a formula. | rules |
| `spacing` | length | Distance between neighbours. On a scatter: the side of a lattice cell. | rules |
| `ends` | string | On a run: `"half"` (default), a half step free at each end; `"both"`, a member at each end. | rules |
| `[scatter.<id>]` | table | A density turned into instances: `assets`, `spacing`, `jitter`, `density`, `within`, `scale`, `tint`, `clump`, `seed`. | rules |
| `jitter` | number | On a scatter: the share of the cell a candidate may move across, 0 to 1. Default 0.8. | rules |
| `assets` | list | Choices, each an asset id, `@<taxon path>` or `{ id, weight }`. A value, replaced whole. | assets |
| `density` | formula | 0 to 1 at each point. | rules |
| `clump` | inline table | How instances gather by cause (R5). One of two forms: `members`, `radius`, `shrink`; or `around`, `radius` (`rules.md` 7.3). A value, replaced whole. | rules |
| `members` | integer, or 2 integers | In `clump`: how many members each parent gets, 0 or more; a pair is drawn. | rules |
| `shrink` | number | In `clump`: members' scale is multiplied by `1 - shrink`, 0 to below 1. Default 0. | rules |
| `around` | name | In `clump`: a producer whose held output new instances grow beside. | rules |
| `within` | name or `{ of, class, near }` | The region a scatter is confined to: a region feature, a footprint feature, or a selected `area`, `cell`, `rest` or `piece` (`identity.md` 3). Never a produced name. | rules |
| `seed` | | See Document. | numbers |

### Hand edits (`identity`)

| Key | Takes | Meaning | Chapter |
|---|---|---|---|
| `[change.<name>]` | table | A difference shaped like the world, merged on request (2.5). A change file holds the same tables at its top. | documents |
| `[pin.<id>]` | table | An instance at a place, whatever producers do: `asset` or `set`, `at` or `on`, `face`, `by`. | identity |
| `[clear.<id>]` | table | `at` or `on` with `radius`; or `region`; or `on` a region (a region feature, or an `area`, `cell`, `rest` or `piece` selection) without `radius`; and `by`. | identity |
| `[lock.<id>]` | table | `region` and `by`: a replayed record of what producers placed there; a producer change reports what it would do there (R4). | identity |
| `by` | list of names | The producers an instance came from, or a hand edit speaks to: run, scatter or step ids, never a set (`identity.not_a_producer`). On a hand edit, default every producer in its own layer or below; naming one in a higher layer is the upward-reference error (`identity.md` 4.1). | identity |

- A pin places its instance and removes every instance of the producers in `by` whose footprint
  overlaps its own. An overlap with an instance of any other producer is a warning naming both
  (R13: a pin inside a building is reported). A pin inside a clear, or where `keep_out` is above 0,
  is a warning naming both. Two pins whose footprints overlap are an error.
- A clear removes every instance of the producers in `by` whose origin lies within `radius` of its
  place, inside its `region`, or inside the region it is `on` (`identity.md` 4.3). Whether it also
  removes generated features: (open) decided with M3's `quay`.
- A lock's record is text in the repository, written by `via lock`, never by hand (principle 1):
  `locks/<lock id>.toml` beside the file that declares the lock, one table per item keyed by its
  produced name, with exactly `asset`, `at`, `face`, `scale`, `tint` and `recorded_height` (the
  surface height when recorded; compared, never placed) (`identity.md` 4.4).
- Hand edits are intent at a place, never against a generated name (R3). Produced names exist for
  the change report, diagnostics, lock records and the command line.

### Imports and composition (`documents`)

| Key | Takes | Meaning | Chapter |
|---|---|---|---|
| `[import.<id>]` | table | A world built apart, placed in this one: `file`, `at`, `face`, `changes`, `footprint`, `blend`, `give`. Whether it may bring its own pack: (open) decided in `documents` for M4. | documents |
| `changes` | list of names | Changes applied to the imported world. | documents |
| `footprint` | name or list of points | The ground a thing covers. On an asset: its outline in its own frame. On an import: the region of it that is brought. | assets |
| `blend` | formula | The weight the import's ground has. | documents |
| `give` | table of names | `{ <name> = "<ours>" }`: hand down a region, a field or a `kind:<name>` selection (R11). | documents |
| `[given.<name>]` | table | What a world accepts from a parent: `accepts`, `default`, `kind`. | documents |
| `accepts` | string | `region`, `field` or `features`. | documents |
| `after` | string | On a layer only, and only `"imports"`: its rows apply over the ground the imports bring (R10). Within it, R21 order holds; overruling anchored import ground is reported. | graph |

### Steps (`protocol`, `graph`)

| Key | Takes | Meaning | Chapter |
|---|---|---|---|
| `[step.<id>]` | table | Declares a step in the layer it writes to; it reads that layer and those below (R21). | protocol |
| `when` | string | `generate`, `lint` or `export` (R22). | protocol |
| `command` | list of strings | The program and its arguments. Never a shell string, since quoting differs on Windows (R20). An element may be `lib:<library>/<path>` (Selectors). | protocol |
| `worker` | bool | `true`: the step speaks the protocol on stdio and may call the engine; `false` (default): a plain command. | protocol |
| `params` | table | Handed to the step unread. Any keys. On an asset or material: handed to the loader unread. | protocol |
| `inputs` | list of strings | Outside files or folders whose change re-runs it (principle 7). | graph |
| `for_each` | selector | Run once per item, for example `"kind:district"`. | graph |

### Pack (`assets`)

A pack is a document users write. It may name files; a world may not (principle 6).

| Key | Takes | Meaning | Chapter |
|---|---|---|---|
| `root` | string | The folder every file of the pack is found under, relative to the pack file. Default the pack file's folder. | assets |
| `[defaults]` | table | Values for declared facts every asset takes unless it, or a taxon above it, writes them. | assets |
| `[taxon."<path>"]` | table | One node of the pack's taxonomy, keyed by path (the exception to R1). Key: `defaults`. | assets |
| `[asset.<id>]` | table | One asset, with exactly one source marker: `file` (a model, M2), `primitive`, or `engine = true`. Keys per source: `assets.md` 2.2. | assets |
| `engine` | bool | `true`: an engine-made asset, made by the loader from `params`. | assets |
| `primitive` | string | One of `assets.md` 6's shape words. | assets |
| `detail` | list of strings | On a model: lower-detail copies of `file`, finest first, relative to `root` (M2). | assets |
| `size` | 3 lengths | `[x, y, z]` in the asset frame, each above 0. | assets |
| `triangles` | integer | Triangle count, 0 or more. Written on an engine-made asset; fixed or measured otherwise. | assets |
| `front`, `up`, `pivot` | string | On a model: its front and up axes (`"+z"`, `"+y"` by default), and the rule that puts the pivot (`base_centre`, `base_corner`, `origin`) (`assets.md` 2.3). | assets |
| `solidity` | string | `solid` or `none` (`passable` M5): whether the footprint is claimed. | assets |
| `sink` | number | How far an instance is lowered below its placing point, metres. Default 0; may be negative. A height, so not a length (`assets.md` 2.4). | assets |
| `max_relief` | number | The largest difference in surface height under the footprint the asset tolerates, metres, 0 or more. No default: without it nothing is checked (`place.relief`). | assets |
| `colour` | 3 numbers | On an asset or material: linear red, green, blue, each 0 to 1. Required on a material. | assets |
| `repeat` | number | On a material: metres one repeat of its `file` covers, above 0. Only with `file`. | assets |
| `[asset.<id>.mount.<name>]` | table | A named place on an asset: `at`, `face` in the asset's frame. Named `<instance>.<name>`. No `t`, `offset` or `surface`. | assets |
| `[material.<name>]` | table | A material `paint` names: `colour`, `file`, `repeat`, `params`. | assets |

Pack terms (`assets.md`): **declared fact** (written in the pack), **measured fact** (read from a
model by the build), the **facts file** (measured facts kept by `via pack accept`), **stand-in** (a model
asset drawn as a box of its accepted facts when its files are absent, M2). Commands: `via pack check`, `via pack list`, `via pack
accept`.

### Checks (`diagnostics`, `graph`, `identity`)

| Word | Meaning | Chapter |
|---|---|---|
| diagnostic | A record with `severity` (`error`, `warning`, `info`), `code`, `subject` (the thing at fault, as an address), `message` (what is wrong, one sentence), `hint` (what to change), a location (`file`, `line`, `column`, `bytes`, or `argument` for a command-line fault), and where they apply `chain`, `position`, `related`, `data`, `count`, `occurrences`, `failed_on` (`diagnostics.md` 1). | diagnostics |
| code | `group.name`. A step's own codes are `step.<id>.<name>`. `via explain CODE` says what to change; a step declares the text for its code with `explain` on its first `diag.emit` (`protocol.md` 3.3). | diagnostics |
| lint | A step at `when = "lint"`. Every check beyond the core's own is one. | diagnostics |
| change report | For each change: every feature, instance (by hand or produced), footprint feature and field it touched, as added, removed, moved, swapped or changed. Moved: any change in integer millimetres or in facing. Swapped: another asset. Changed: another value that is not position, facing or asset (scale, tint, kind, an attribute). For a field, the region whose samples changed. Each report entry names its cause: the edited key, or the reference it followed. Kind surfaces and kind mounts move with their owner and are not report entries of their own. The graph supplies changed regions and causes (`graph.md` 6.4). | identity |

### Selectors and forms (`identity`)

| Form | Meaning | Chapter |
|---|---|---|
| `kind:<name>` | Every feature of that kind, as one collection. | identity |
| `role:<role>`, `tag:<tag>` | Every world field with that role or tag. Never a kind surface. | fields |
| `@<taxon path>` | Every asset of the pack filed at or under that path. | assets |
| `{ of, class, near }` | The produced thing of that class, from that maker, nearest the point. The only way text points at a produced thing (R3). | identity |
| `<id>.start`, `<id>.end` | The two ends of a path. A point feature is a place by its own id. | identity |
| `<instance>.ground` | An instance's footprint feature (`rules.md` 2.1). | rules |
| `<import>.<place>` | A place inside an import (`a.quay_road.end`, `a.west_gate`). Never its coordinates (R8). | identity |
| `lib:<library>/<path>` | In a step's `command`: a file in a library named in `include`, resolved in that library's folder (`protocol.md` 1). | protocol |
| `<maker>#<place key>` | A produced name. **Never written in text** (R3, `identity.key_in_text`): it appears only in diagnostics, the change report, lock records, the command line and the messages between the engine and a step. | identity |

### Produced names (`identity`)

| Word | Meaning | Chapter |
|---|---|---|
| place key | Where a produced thing is, built from integers that describe its place and the names of what it is made from, never from a counter. Matches `[a-z][a-z0-9_]*`, at most 64 bytes. | identity |
| class | What sort of produced thing it is; the closed list is the `class` key's (Placement, `identity.md` 2.3). | identity |
| ordinal | A key's last part: a thing's position among exactly those that share every other part. Fixed before any filtering. | identity |
| lattice index | The `(i, j)` a scatter candidate was drawn for, which its key holds, so jitter never renames. | identity |
| node | A point where a network's paths meet. | geometry |
| frontage | One source path on one side, along which cells are cut. | geometry |
| stretch | A part of a frontage between corners. | identity |
| anchor | The key of the last node at or before a stretch's start. | identity |

## 4. Owed words (R13, R16, R18, R25)

Mechanisms the doctrine owes in core. Each word is fixed here; where its syntax is still (open), the
chapter named decides it, for M1 track A unless said.

| Word | Meaning | Chapter |
|---|---|---|
| `follows` | A path written along another feature (R18): `follows = { ref, span, offset }` as a sketch. | geometry |
| network | Paths noded where they cross, with the areas between them (R13). A maker. How one is declared, for example from a `kind:` selection: (open). | geometry |
| area | A region bounded by a network's paths, named `<maker>#<place key>`, class `area`. Not `face`, which is orientation. | geometry |
| cell | One part of a region divided along a frontage (R13, R23), named `<maker>#<place key>`, class `cell`; what lies deeper than a cell's depth is class `rest`. The place key of a name is not a cell. | geometry |
| mount on a kind | `[kind.<k>.mount.<name>]` (Kinds): a place every feature of the kind has, such as a deck's two ends, so two surfaces meet at a pair of mounts (R25, `surfaces.md` 3.1). | surfaces |
| kind surface and bound | `[kind.<k>.surface.<name>]` with `bound` (Kinds): a bounded `surface` owned by each feature of a kind, over its strip or region (R25, `surfaces.md` 1). "Owner" is prose only; `owner` is taken by layers. | surfaces |
| datum | Where the world frame's height zero and north lie, declared once per world. Lesson (TODO): `atlas` needed a lift found by eye. | geometry |
| `passes` | The places a main way passes, by name (R16). A ways library attribute, not core; the core gives a route over a cost field (R23). | (library) |
| `hosts` | What a volume yields as floors (R25). Kept or folded into surfaces: (open) decided in `surfaces` for M5. | surfaces |

## 5. Not in M1

The M1 parser rejects each key below with `doc.not_yet`, naming its milestone. It never ignores one.

| Words | Milestone |
|---|---|
| `worker`; `clump` and its `members`, `shrink`, `around`; a model asset's `file` and `detail`; `channel`, `range` (PNG fields); `filter = "cubic"`; class `feature` | M2 |
| `[set]`, `set`, `[run]`, `count`, `span`, `ends`, `[lock]` and its record, `for_each`, user syntax for network, area and cell, classes `node`, `edge`, `area`, `cell`, `rest`; `[kind.<k>.surface.<name>]`, `bound`, `[kind.<k>.mount.<name>]` and its `surface`, `meets` (kind surfaces and mounts at M3 once the lead accepts `surfaces.md` Open 2; the plan's M3 rows 34, 38 and 63 need them) | M3 |
| `[import]`, `changes`, `footprint` on an import, `blend`, `give`, `[given]`, `accepts`, `after`, `exit` | M4 |
| `type = "volume"`, `hosts`, `connector`, `solidity = "passable"` | M5 |
| `chunk`, `[budget]` | M6 |

A second top-level `surface` field is not here: it is `surface.two_terrains` in every milestone.

## 6. Spike word to v1 word

| Spike | v1 | Why |
|---|---|---|
| `[[feature]]`, `[[instance]]`, any `[[x]]` | `[feature.<id>]`, `[instance.<id>]`, `[x.<id>]` | R1 |
| `[world]` keys | top-level keys | One table less. |
| `[params]` | `[let]` | Same word as field and kind `let`. |
| field `clear` | field with `role = "keep_out"` | `clear` is the hand edit. |
| deck `clear` | `headroom` (library attribute) | As above. |
| kind or effect `reach` | `radius` | `reach` is the walking lint. |
| `at` as a fraction along a field row | `t` | `at` is a coordinate only. |
| `on.feature` | `on.ref` | It names things that are not features (principle 9). |
| `on.t` | `t` | Unchanged. |
| run `from` / `to` | `span = [from, to]` | `t` is one number. |
| `slide` | dropped | `t` says the same. |
| step `at` | `when` | `at` is a coordinate only. |
| step `run` | `command`, an argument list | `run` is a rule. |
| step `after` | the layer the step is declared in | R21. |
| field `layers` | `[field.<f>.then.<id>]` | `layer` is a document. |
| field `type = "material"` | `type = "weights"` | `material` is a role. |
| `base = "file"` | `file` alone | One key per source. |
| field `unit` (switched on anchor comparison) | `unit` (a unit only); anchors always compared | Principle 3: a clash between firm decisions is never silent. |
| effect field `#tag` | `field = "tag:<tag>"` | One selector form. |
| `check_where` | dropped | `max_change` is a per-feature formula and an error (`authority.md` 2.2). |
| instance `kind`, `margin` | `ground = "<kind>"`, `margin` | An instance has an asset, not a kind. Its footprint feature is the place `<instance>.ground`. |
| instance `mount` (height above the floor) | dropped | Height comes from the surface beneath (R25). `mount` now names a place on an asset or kind. |
| `in` (a room id) | building library attribute | Principle 5; floors are surfaces (R25). |
| `group` (no overlap test between members) | dropped | Pins and producers settle overlap; (open) if a case needs it, decided in `rules`. |
| `scatter.id` written on instances as `by` | `by` | Same meaning, widened to every producer (run, scatter, step). |
| `labels` | `tags` | `taxon` one place, `tags` many. |
| `[[part]]`, master | `[import.<id>]`, parent | "Part" collides with building part. |
| `after = "parts"` | `after = "imports"`, on a layer only | As above. |
| part seam `apron`, `max_step`, `joins_at` | `blend` and library lints | R8, R10: the join blends by a rule the user may replace. |
| part `expects` | library lint | Principle 5. |
| part `pack` | (open) | Section 3, Imports. |
| `atop`, `beside`, `against`, `along`, `yaw`, `turn` | `on = { ref, t, offset }`, `face` | One placement grammar. |
| `joins` | `starts_at`, `ends_at` | A join is an end at a named thing. |
| `over` | dropped | Principle 3: a clash between anchors is an error, never silenced. Its length also ordered effects, against 2.2. |
| `ellipse` | dropped | Write the polygon, or a library step makes one (principle 10). |
| `path3` | (open) | Decided in `surfaces` for M5. |
| `width` | library attribute | Four meanings in the spike. An exit's `width`: section 3, `exit`. |
| kind `map` | map library, through `output` | Principle 5. |
| kind `measure`, `encloses`, `opening`, `link`, `light`, `module_m` | library attributes (water, building) | Principle 5. |
| `[export] feature_attrs` | dropped | A step reads every attribute (R19b: no private door). |
| `[[kitrun]]` | building library rule | Not core. |
| `[[dress]]` | `[scatter.<id>]` with `within` | A scatter on a thing. |
| `[[flank]]` | an instance of a set, `on` a mount | A set placed on a mount. |
| set `items[].name`, `offset`, `turn` | `[set.<id>.item.<name>]`, `at` in the set's frame, `face` | R1; one placement grammar. |
| `<producer>#<key>` written in `ref` | `on = { of, class, near }` | R3: produced names are never written in text. |
| `[variant.<n>]`, overlay file | `[change.<n>]`, change file | One concept. |
| `--variant`, `--overlay` | `--change C`, repeatable: a path if `C` holds `.` or `/`, else a name (`build.md`, flags) | As above. |
| `[[override]]` | dropped | R3: a hand edit is intent at a place. |
| `[[pin]]`, `[[clear]]`, `[[lock]]` | `[pin.<id>]`, `[clear.<id>]`, `[lock.<id>]` | R1. |
| hand edit `scatter` | `by` | Speaks to every producer, steps included (R13). |
| kinds `poi`, `bookmark` | library kinds with `output` | Principle 5. |
| `[[expect]]`, `[profile.*]`, `--lint reach` | review library lints | Principle 5. |
| `[look]` | the loader's own settings, passed through | Principle 12. |
| field names `height`, `ground`, `water` | any name with a role | Principle 5. |
| `[export] fields` | a loader option | Every field is exportable. |
| `seed = 1` on each rule | default: the rule's id | Fewer lines. |
| pack `[ground.<n>]`, `[surface.<n>]` | pack `[material.<n>]` | `ground` is the instance key; `surface` is the role (R25). |
| `[[level]]` | `storey` (building library) | `level` collides with a map. |
| portal part | an import placed `on` a mount | `portal` means culling in engines. |
| volume host `hosts` | `hosts` (open) | Section 4. |

## 7. Library words and the deny-list

Words that belong to libraries under `lib/`, written as data in the same format a user writes:
road, lane, street, main way, crossing, ford, bridge, deck, river, beck, lake, water, shore,
district, square, axis, ward, lot, block, building, storey, room, door, window, stair, wall, roof,
pad, tree, plant, bookmark, poi, waterfront, expect, profile, reach, headroom.

`exit` is not on the list: the core owns the flag and the word (R9); a library owns the kind's name.

The rule: **no core crate contains these nouns.** If the core must know one to work, the core is
missing a mechanism (principle 5). A core mechanism named after one is renamed, not exempted.

The test (plan, "How every milestone runs"):

- Covers `via-num`, `via-diag`, `via-doc`, `via-expr`, `via-geom`, `via-raster`, `via-graph`,
  `via-model`, `via-proto`, `via-cli` and `via-step`. It excludes `via-terrain`, the erosion
  library, which is library tier (R14).
- Reads identifiers and string literals, not comments. Identifiers split at `_`, `::` and each
  lower-to-upper case change; string literals split at every character that is not a letter. Words
  are lowercased. A hit is a whole word, or its plural in `s` or `es`; a two-word entry matches two
  words in a row.
- The machine-readable list is `spec/vocabulary.toml`, written with the code checks (M0 step 5).
  Until then this section is its source.

Allowed in the core in one sense only. The test passes them; review checks the sense.

| Word | Allowed sense |
|---|---|
| `window` | An evaluation window over a field (doctrine section 7). |
| `axis` | A coordinate axis. |
| `block` | Blocking input and output; a block of code. |
| `wall` | Wall-clock time. |
| `square` | Squaring a number. |
| `tree` | `BTreeMap` (R19's ordered maps); a spatial index tree. |
| `expect` | Rust's `expect` on `Option` and `Result`. |
| `pad` | Padding bytes or text. |
| `lane` | A lane of a vector instruction. |
| `reach` | Reachability in a graph. |

The list grows when a library adds a noun.

## 8. Colliding names

From `docs/ledger.md` section 25, and from review of this page.

| Word | v1 decision | Reason |
|---|---|---|
| part | `import` | "Part" means a piece of a building to everyone else. |
| level | Not a core word; the building library says `storey`. | A level is a whole map in every engine; storey is the building-information term. |
| set | Kept, as a core rule: a template placed by an instance, run or pin. Prose says "collection" or "group" for the everyday sense. | The doctrine names it among the three rules. |
| portal | Dropped: an interior is an import placed on a mount. | Engines use portal for visibility culling. |
| layer | Kept for a document only; field rows are `then`. | An engine's terrain layer is a `weights` field with `role = "material"` here. |
| face | Orientation only. A region between a network's paths is an area. | Doctrine section 6 keeps `face` for orientation. |
| edge | A network edge (class `edge`), beside the world's edge of R9 and R10. The network's is always "edge"; the world's is always "the world's edge". | Two senses that prose must keep apart (`identity.md` 9). |
| cell | One part of a divided region only. Generated names use a place key. | One word, one meaning (principle 9). |
| producer, maker | A producer is a run, a scatter or a step; a maker is anything with produced names (a producer, a network, a cells table, a `where` feature). | `by` names producers only; selection names makers (`identity.md` 1). |
| where | A feature key only. A diagnostic's location is `position`; formulas have no `where`. | One word, one meaning (principle 9). |
| kind | Kept. | One of the eleven nouns; its meaning is stated above, which the ledger found missing. |
| pack | Kept. | It binds ids to files (principle 6); a bought bundle becomes usable by writing a pack for it. |
| deck | Library word (ways library). | A deck is a surface owned by a feature (R25); the core needs no word for it. |
| entry | A field entry (`authority.md` 4) unless qualified. A line of the change report is a "report entry" in prose; `identity.md` 5 says "entry" there, where the context is the report alone. | Two senses; the field's is the core term. |
| build | Kept: what `via build` writes, and the act. | The game's build is the engine's; we say "the game build" when we mean it. |
