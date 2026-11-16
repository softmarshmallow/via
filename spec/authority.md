# Kinds and authority

A kind says, as data, what a feature does to fields; authority says how firmly each statement
holds. This chapter fixes the keys of a kind and an effect, the three tiers, the one order in which
every entry reaches a field, what happens when two firm statements meet, and what a step may not
move.

Authority: `docs/doctrine.md` (principles 3, 5, 10; R7 as amended by R28, R14, R18, R21, R29,
R31). Words: `spec/vocabulary.md`. What one entry does to a field is `fields.md`; the names a
formula sees are `formulas.md`; distance, `d`, `sd` and `t` are `numbers.md`; ids and place names
are `identity.md`.

Formulas are written `"= ..."` (`formulas.md`).

## 1. Kinds

A kind is a table in a kinds file of a library (`documents.md`: `[kind]` anywhere else is
`doc.kind_outside_library`). The core names no kind (principle 5). It reads a kind's keys below and
nothing else; a library's own steps read every attribute (R19b).

| Key | What the core does with it | Chapter |
|---|---|---|
| `authority` | The tier of the kind's features unless a feature writes its own. Default `derived`. | this |
| `defaults` | Declares attributes and their values. A table: a change merges it key by key (`documents.md`, merging). | this |
| `requires` | Declares attributes every feature must write. | this |
| `radius` | A `length`, > 0 (`numbers.md` 3.1). Bounds every effect of the kind that has no `radius` of its own. | `fields.md` 5.3 |
| `let` | Named formulas its effects read. | `formulas.md` |
| `names` | Lists the attributes whose values are place names (1.2). | this |
| `[kind.<k>.effect.<id>]` | One effect (section 2). | this |
| `exit` | Marks its point features as exits (R9): `true`, or the attributes an answer must match. M4. | `composition.md` 7 |
| `output` | Hands its features to the loader under a name. | `build.md` |

A kind may also own a bounded surface and mounts (R25). Those tables are `surfaces.md`'s, listed in
`vocabulary.md` once that chapter adds them, and none is in M1.

A feature without `kind` is a guide: a shape that does nothing. It has no effects and no tier.

A feature's `kind`, or an instance's `ground` (`rules.md` 2.1), that names no kind in the world's
kind namespace (`documents.md` 3) is `authority.unknown_kind` (error), with the nearest kind as hint.

### 1.1 Attributes

- An attribute is declared once: in `defaults` (with its value) or in `requires` (without one).
  Declared in both is `authority.attribute_twice`. Named as a reserved feature key is
  `doc.reserved_attribute` (`documents.md`); named as a formula built-in is `formula.shadow`.
- A feature's attribute is its own value, else its kind's default. A feature that lacks a required
  attribute is `authority.requires`, naming the attribute and the kind's `requires` line.
- An attribute named only in `requires` takes its type from what each feature writes: a number or a
  formula is a number, a bool is a yes/no, a string is text. A formula that reads it is checked per
  feature, so one feature that writes text where an effect reads a number is `formula.type` at that
  feature. This answers the question `formulas.md` 6.7 leaves here.
- A kind defaults attributes only, never a reserved feature key. So `smooth` and every placement
  key are read from the feature's own text, and a reader sees how `t` is measured without opening
  the library (principle 11). This answers the question `geometry.md` leaves here.
- An attribute listed in `names` is a name (1.2), not text.

### 1.2 Name attributes

A name attribute holds a place name: the thing a feature points at that is not where it stands (a
bookmark's view, a sign's destination). A text attribute would hide that reference from the core.
So the kind lists such attributes in `names`, and the core treats each value as a named reference.
The mechanism is ours.

```toml
[kind.bookmark]
output   = "bookmark"
requires = ["toward"]
names    = ["toward"]

[feature.at_ford]                    # the plan's ford world
kind   = "bookmark"
on     = { ref = "lane_west", t = 0.93 }
toward = "ford"
```

- Each attribute in `names` is declared in `defaults` or `requires` (1.1). One that is in neither is
  `authority.names_undeclared` (error) at the `names` line, nearest declared attribute as hint.
- Its value is a string in a form `on.ref` accepts: an id, an end (`lane.start`), a mount
  (`cottage.door`), a place name through an import (`identity.md` 1). Any other type, a formula
  included, is `doc.type`.
- It is resolved by `identity.md`, as `on.ref` is. A name that names nothing is
  `identity.unknown_place`, with the nearest match.
- It is a named reference: an edge in the graph from the feature's `attr` node to the named
  thing's `shape`, `instance`, `held` or `edit` (`graph.md` 2, as placement), and checked for
  direction (`documents.md` 5.1). So when the target moves, the feature is in the change report with
  the target's move as its cause (`identity.md` 5).
- A formula that reads a name attribute is `formula.type`: a name is not a number.
- It is written to `features.json` resolved (`build.md` 9.1): `{ "name": "<place name>", "at": [x,
  y, z] }`. `at` is the point the name gives: a point feature's `at`; an end's or a mount's point;
  for an instance or set item, its pivot (`build.md` 8), `z` its pivot height after `sink`; for a
  path, its point at `t = 0.5` (`geometry.md` 3.8); for a region, its representative point
  (`geometry.md` 3.2). Otherwise `z` as `build.md` 9.1 gives it; a mount's own height for a mount. The choice
  of point for a path or a region is ours. A loader reads `at` and never resolves names itself.

The lead is asked to add `names` to `vocabulary.md`'s kind keys, the row "name attribute" to
`graph.md` 2 and the key to `documents.md` 5.1's list of name keys, and the written form to
`build.md` 9.1, closing `build.md`'s open question on attributes that are place names.

### 1.3 A kind lent by `ground`

An instance with `ground = "<kind>"` has a footprint feature of that kind (`rules.md` 2.1).

- The footprint feature has the kind's tier and defaults, and no attributes of its own.
- `authority` on an instance is `doc.unknown_key` (an instance's keys are closed, `rules.md` 2),
  with the fixed hint: "the footprint takes its `ground` kind's tier; ground with a kind of the
  tier you want". A library that offers a firm and a loose pad declares two kinds.
- A `ground` kind with `requires` is `authority.ground_requires` (error) at the instance's `ground`
  line, with the kind's `requires` line in `related`. The footprint cannot write the attribute. The
  hint: give the attribute a default in the kind, or ground with another kind.

So `ford`'s bad edit "cottage anchored in the beck" (plan, milestone one) is the cottage grounded
with an anchored kind and moved into the anchored beck: `authority.conflict` (section 5).

## 2. Effects

An effect is one change a kind makes to fields near each of its features. Each (effect, feature)
pair is one entry on each field it writes (section 4), applied by `fields.md` 5.

| Key | Takes | Meaning |
|---|---|---|
| `field` | string | `role:<role>` or `tag:<tag>`. The entry is made on every field that matches. Never a field name, so a library works whatever the world calls its fields. |
| `value` | formula | Scalar fields: the value the entry moves toward. |
| `mask` | formula | The entry's weight at a post, 0 to 1. Default 1. |
| `paint` | name | Weights fields: the material the entry adds weight to. |
| `radius` | `length`, > 0, read by `numbers.md` 3.1 | Beyond this distance from the feature the entry does nothing (`fields.md` 5.3). Default: the kind's. |
| `max_change` | formula (per feature), >= 0 | Scalar fields: the most the entry may move a value at any post (2.2). |
| `order` | integer | Its place among the entries of one tier that nesting leaves free (4.2). Default 0. |

| Fault | Code |
|---|---|
| `field` not of the form `role:<role>` or `tag:<tag>` (a bare `"height"`, the spike's `"#water"`) | `authority.selector` (error). The hint shows both forms; where the bare word is a field's name, it gives that field's role or tags (`height` has role `surface`: write `role:surface`). |
| `role:<r>` with a role not in the closed list (`fields.md` 2.1) | `authority.selector`, nearest role. |
| A well-formed selector that matches no field | `authority.no_field` (info), once per (kind, effect), and only when the kind has at least one feature in the build. Nearest tag of the world, if any. The effect does nothing. This is the one code for the fault; `fields.md` 2.3 points here. |
| `value` missing on a scalar entry, `paint` missing on a weights entry | `field.row_missing` (`fields.md` 5). |
| A key that does not fit a matched field's type (`value` or `max_change` on a weights field, `paint` on a scalar one) | `field.row_key` (`fields.md`), naming the field. |
| `radius` at or below 0 | `authority.range` (error): the key, the value, the range allowed. |
| `max_change` below 0 for a feature | `formula.range` (`formulas.md` 6.1), at that feature. |
| `order` not an integer | `doc.type` (`documents.md`). |

Effects on `volume` fields are M5 (open): whether they obey the tiers is decided with `coast`.

### 2.1 The names an effect sees

The full list, with rates, is `formulas.md` 6.5 and 8.5. In short:

| Name | Meaning |
|---|---|
| each attribute | The feature's attributes (1.1). |
| the kind's `let` names | Formulas of the kind, read like names. |
| `d`, `sd`, `t` | Distance, signed distance and fraction along, in the feature's local frame (`numbers.md`). |
| `v` | The field at the post before this entry, in the order of section 4 and the view being resolved. Scalar fields only. |
| `under(stat)` | A measure of this field before this entry, over the feature. |
| `under(field, stat)` | A measure of another field's finished value in the view being resolved, over the feature. |
| the path measure of `formulas.md` 8.5 | Paths only: this field before this entry, smoothed along the path. |

- `v` reads the field at this entry's place in the order. It is a value at the post, so R7 allows
  it. Everything in this chapter that depends on reading an earlier entry (nesting, 4.3; the way out
  of a clash, section 5) uses `v` only.
- `under` and the path measure read neighbours. R28 allows them in a formula, since each declares
  its halo and cap (`formulas.md` 8.5, 8.6). They read the field at the entry's place in the order,
  which no step could.
- Measures run over the feature's posts inside the extent, never over one window, so a window of a
  field still equals the same cut of the whole (`numbers.md` 6). A measure over no posts (a
  feature wholly outside the extent; an `'edge_min'` band past the edge) is `num.empty_reduction`,
  registered by `numbers.md` 6 (`formulas.md` 8.5).
- A per-feature value (`under`, the path measure, `max_change`) is worked out only when at least one
  post of the effect's window lies within `radius` (`fields.md` 5.3). Otherwise the entry touches no
  post, nothing is evaluated, and no error arises.
- Reading another field makes a dependency between fields. A cycle across fields is `graph.cycle`.

### 2.2 `max_change`

At every post where the entry's mask is above 0, `abs((value - v) * mask)`, the amount the entry
moves the value, is compared with `max_change`. Past it is `authority.max_change` (error),
reported once per (effect, feature, field) at the post of the largest move (ties: post order of
`numbers.md`), with the amount and the limit. The hint names the effect's `value` and `max_change`
lines and gives `v` and `value` at that post. The entry is still applied in full, so the build
shows what was asked.

**A produced instance gives way (M3).** When the entry past its limit is the footprint feature
(`rules.md` 2.1) of a produced instance, one a step returned or a producer placed
(`<producer>#<key>`, `identity.md` 2), the instance gives way: it is not placed, its footprint
writes nothing, and the build says `authority.gave_way` (warning), subject the instance, with the
field, the amount, the limit and the post of the largest move. A producer offers places; a limit
the author set on the ground decides which hold, so the producer need not predict the field (a
library that did would read the ground twice). Rounds, before anything is grounded: the fields
are resolved with every placed footprint, every produced instance whose pad passes its limit
gives way at once, and the fields are resolved again without them, until a round gives way to
none. A round is the whole set found, so the outcome follows no name order. Nothing written
stands on a produced instance by name: text never writes a produced name (`identity.key_in_text`),
a step's document names only the stratum in `on`, and a selection `on = { of = "<producer>",
near }` chooses among the instances that stand, so it never falls on one that gave way. Should an
engine path still reach one, what stands on it, and on that, is not placed either and says
nothing; the instance's `authority.gave_way` is the cause. A written
instance, and a written or produced feature, past its limit stays `authority.max_change`
(error): it is what the author wrote, and only the author moves it.

The ruling between error and clamp is error (principle 3: a limit the author set is a firm
statement, and a clamp would be a silent winner); a produced instance gives way rather than clamp,
so the limit still holds wherever anything stands. `max_change` takes a per-feature formula, worked
out once per feature, so a kind can tie it to an attribute (the spike's pad read `max_cut`); it
reads no `v`, `d`, `sd` or `t` (`formulas.md` 6.1). The spike's `check_where` is dropped.

### 2.3 A shape reads the head

A feature whose shape reads a field (a `where` region, or a derived feature, section 8) conforms to
that field's **head view**: its base and its head (4.1), every `then` row of every layer, never an
effect, its own or another's (`formulas.md` 6.6).

- This is the doctrine's build order, "fields from their bases, then features". It is also what
  keeps regions from being cycles: the order of effects depends on region shapes (nesting, 4.3), so
  a region that read the finished field would depend on itself whenever any region of its tier
  wrote that field.
- "Reads" is transitive: through a field's base, its `then` rows' formulas and `let` names. A
  `where` that reads `slope`, whose base reads `height`, reads `height`'s head view.
- Its kind's entries on the fields its shape reads are made as usual. No cycle arises, since no
  effect is in what the shape reads. A region that lowers `height` where `height` is below 10 lowers
  the ground the base and the `then` rows put below 10; adding a `then` row that raises the ground
  moves the region (`formulas.md` case 34).
- So shapes are known before any field's effect order is formed (4.3), and a feature placed `on`
  such a shape, `follows` it, or grounded on it, may write a field the shape reads: its entries are
  effects, outside the head. A cycle remains only through the head itself, a `then` row that reads
  the feature whose shape reads that field: `graph.cycle`.
- An effect's own formulas read the finished field (2.1); only a shape reads the head view. A
  feature a step returns has the shape the step computed in its own view (4.4).

This answers the spike's known limitation that effects on a field a flood region read never reached
the field (`spike/SPEC.md` 11): they are now applied.

## 3. Authority

| Tier | What it says | Holds against |
|---|---|---|
| `derived` | Conforms to the world. Its values follow from others; it gives way to everything. | Nothing. |
| `shaped` | The author's loose statement. It shapes the world and gives way to anchored features. | Derived. |
| `anchored` | The author's firm decision. The world conforms to it. | Derived and shaped. Another anchored feature that disagrees with it is an error or a warning (section 5). |

- `authority` is written on a kind (its features' default) or on a feature (overrides the kind).
  Default `derived`. Any other value is `authority.unknown`, with the nearest tier as hint.
- `authority` on a guide is `authority.no_kind`: it has nothing to hold.
- A tier says how firmly a feature's entries on fields hold, through the order of section 4, and
  how its meetings with other anchored features are judged (section 5). Shapes and placements
  resolve as `geometry.md` says, whatever the tier.

## 4. The order of a field's entries (R21)

An **entry** is one change applied to one field: a `then` row (`fields.md` 5), or one (effect,
feature) pair. A field's value is its base, then every entry on it, one after another, by the rule
of `fields.md` 5. This section fixes the sequence. It is one total order per field, across the
whole world. The lead is asked to list "entry" in `vocabulary.md` with this meaning.

Declaration order never counts: tables keyed by id are unordered (`vocabulary.md` 2.2). Every tie
below that no key settles is broken by place name (`identity.md`), bytewise. In M1 a feature's place
name is its id; from M4, a feature of an import is named through it (`a.lake` against `b.lake`), so
imports do not change the order of what is already there.

Entries are applied in doubles and narrowed to a 32-bit float once, after the last entry
(`fields.md` 1.1). Every comparison in this chapter is made in doubles, before narrowing.

### 4.1 The head

The **head** is the field's own derived beginning: every `then` row, written in text or returned by
a step (`fields.md` 7). It comes before every effect, in this order:

1. `order`, lowest first;
2. layer rank, lowest first. A step's rows take the rank of the step's layer;
3. the row's place name, bytewise: its id for a text row, its produced name for a step's row
   (`<step>#<id>`, `identity.md` 2.4). Two `for_each` calls that return one produced name are
   `graph.md`'s error.

This is R29 with no nesting, since rows have no shapes: a step's rows are rows like any other, one
order, no second pass. A field's `file` takes the place of `base` (`fields.md` 3); the field it
gives is built and ordered like any other. That a step's rows cannot sort before a row it read is
checked after it runs (4.4).

### 4.2 Effects

Every (effect, feature) entry follows the head, ordered by:

1. **authority**: derived, then shaped, then anchored, across the whole world, whatever layer the
   feature is in (R21);
2. **nesting** (4.3): what comes later in a nesting relation is applied after the other;
3. **`order`**, lowest first (challenge 36: every way lays its pavement at 0 before any lays its
   roadway at 1);
4. **layer rank**, lowest first;
5. **place name** of the feature, then **effect id**.

Keys 2 to 5 are one step. Among the entries of one tier, take them in a topological order of the
nesting relation (every entry of the earlier feature on the field before every entry of the later
one), choosing at each point, among the entries whose predecessors are placed, the least by
(`order`, layer rank, place name, effect id). So `order` places only entries that nesting leaves
free, and a library's `order` cannot break a nesting: a pad on a lane comes after the lane's
roadway whatever their `order`.

This is R29: within one tier, nesting, then `order`, then layer rank, then id bytewise; document
order never decides. Authority stays first, so R21's case holds: a derived lake in a lower layer
gives way to an anchored thing above it.

### 4.3 Nesting

Nesting is a relation between two features with entries on one field, both of one tier. It is
formed per field, among those features only. One side comes later:

| Relation | Written as | Comes later | Why |
|---|---|---|---|
| B stands on A | B's `on.ref` names A, an end of A, or a mount of A; or B is the footprint of an instance placed `on` A | B | B refines A: a pad on a lane. |
| B follows A | B's `follows.ref` names A | B | B is written along A and refines it (R18). |
| B arrives at A | B's `starts_at` or `ends_at` names an end of A, a mount of A, or A when A is a point feature or an instance; or names the instance, or its mount, whose footprint A is | **A** | R18: the owner of a meeting holds it against what arrives (section 6). |
| B lies inside A | Every point of B's shape is inside or on A's region, and B's area is smaller | B | The smaller is applied later: a pad on a terrace. |

- The first three are written; containment is found. Containment is not tested for a pair that a
  written relation already links, so a lane that ends at a mount of a quay and lies inside the
  quay's region comes before the quay, as arrival says.
- Containment is exact on the integer lattice (R24); areas are compared in square millimetres.
  Paths and points have no area and lie inside a region that contains them. Shapes are known before
  the order is formed (2.3), `where` regions included: a shape reads head views only, which hold no
  effect.
- Across tiers there is no nesting; authority decides alone.
- A cycle in the relation is `authority.order_cycle`, naming the field, every feature in the cycle,
  the key that made each link and which side that link puts later. The hint: state the meeting once.
  A cycle needs links that disagree, at least one of them written. Two paths that each start at
  the other's end are not one: each shape reads the other, a `graph.cycle` before any order is
  formed (`graph.md` 2.2). A feature that both follows and arrives at another waits on `geometry.md`
  4 (M3). One that M1 builds: a path that ends at an instance, and a second path that starts at the
  first's end and lies inside the instance's footprint. A ford `on` a beck with lanes arriving at
  the ford is not one: the beck comes first, then the lanes, then the ford.

Entries whose windows (`fields.md` 5.3) do not overlap commute, so an implementation may test
containment only between features whose windows overlap. The result is the same.

### 4.4 What a step reads (M2)

A step at layer L reads a field through one of two views (`fields.md` 6.2). Both are fixed by its
stratum (`graph.md` 4) before it runs, and neither holds anything a step at L or above returns: the
rows such a step returns, or the entries of features it returns.

| View | Holds | Use |
|---|---|---|
| **head view** | The base; the head entries of layers below L; the `then` rows written in layer L's text. | For a field the step returns rows on. |
| **full view** | Every entry from layers at or below L, in the order above, except what steps at L return. | For any other field. |

- The step names the view in each read: `view = "head"` or `"full"` on `build.field`, default
  `"full"`. The lead is asked to add this parameter to `protocol.md` 3.3, the node keys
  `v:<layer>.head` and `v:<layer>.full` to `graph.md` 4, and this table to `fields.md` 6.2.
- By 4.1, a step's rows sit before every effect. A step that returns a row sorting before a row of
  its head view is `authority.step_view`, naming both rows: the row it read would be applied again
  on top of its own. The hint: give the returned rows an `order` above every row read. So no entry
  it read is applied again (R21: "nothing below it is applied twice"). Rows outside its view may
  sit before or after its rows; it never read them.
- A step that read a field's full view and returns rows on it is `authority.step_view`: those rows
  would sit before effects the step already saw. The hint: read the head view, or return features.
- A step that read a field's full view may return features whose kinds write that field. Their
  entries were not in its view, and they take their place in the order by section 4.
- **The full view is the build's order, filtered (M3).** Its entries are those of the finished
  field, in the order section 4 gives them over the whole world (R21, R29: one order across the
  layers), keeping each row and effect of rank L or below and leaving out every entry of a higher
  layer and every entry of what a step at L or above returned. The kept entries are never ordered
  again among themselves: nesting with a left-out feature still places them as it does in the
  build. A footprint (4.5) has its instance's rank, so a `ground` pad of an instance in a layer
  above L, written by hand or produced by a step, is outside the view: the step at L reads the
  ground as it lies below that layer, and runs (case `step_full_view`). A step's own returns are
  outside its view too, though they join its layer (case `step_own_returns`). A worker's formula calls
  (`expr.eval`, `derive.*`) read each field in the view the call names, as `build.field` does.
  A plain command is handed the build's fields, not a view; one at a layer below an entry that
  changes a field it is handed stays `doc.not_yet` until the files it is handed are its view's.
  For a plain command its own returns are not such an entry: they join its layer, and it reads
  nothing after returning, so a plain step at the only layer that returns a feature whose kind
  writes a field it is handed runs (case `step_plain_own_returns`).
- **What counts as returned.** "What a step returned" is every name `<step>#…` whose maker is a
  generate step: its returned rows, features and instances, and their parts (an instance's
  `ground` pad, `<step>#<key>.ground`). A `#` name another maker made (a scatter's instance,
  `identity.md` 2) has its maker's rank and is in the views of its layer like a written entry.
  The head view, the full view and the step's view of features and instances apply this one rule.
  What a `[network]` or `[cells]` table made (`<table>#…`, `geometry.md` 8.1) counts as returned
  too: it stands where a step's return stands, at its table's layer, and is outside the views of
  every step at that layer (a step of a higher layer sees it). The rule holds for the members of a
  shape selector too: a field whose formula reads `kind:<k>` has the made things in the build and
  not in such a step's view (`geometry.md` case `n14`). A plain command is handed the build's fields, in which a
  made thing of its own layer is part of its layer, as its own returns are (above).

### 4.5 Footprint features

An instance with `ground` has a footprint feature (1.3), whether the instance is written by hand or
produced by a rule or a step.

- Its entries join each field's order like any feature's: its kind's tier, the layer of the
  instance (or of the rule or step that produced it), and the instance's place name.
- Its shape is plan only: the footprint at a plan position, read from no field. So it nests like
  any region (4.3), and its own placement on a feature is a "stands on" link.
- A rule's own formulas, such as a scatter's or run's `density`, read fields in the build's view. A
  field read that way which takes entries from the rule's own footprints is a cycle, `graph.cycle`
  (`rules.md` 3.4).
- A shape a rule stands on or is confined to (an `on` feature, a `within` region written as
  `where`) reads the head view (2.3), which holds no effect, so footprint entries never feed back
  into it.
- How high a placed thing stands reads the final surface, its own footprint included (`rules.md` 6,
  8.2). That is not a cycle: height never moves a plan position. The `keep_out` test reads the view
  of `rules.md` 8.3.

So the doctrine's build order (fields, features by authority, rules, checks) is the order of
dependence, not of finishing: a field is finished only when the footprints that write it are
placed, and a rule that reads it waits for them.

## 5. Anchored features that meet

Two anchored features A and B, each with an entry on one scalar field, A's entries before B's in
section 4. They are compared at every post where B's mask and the mask of at least one entry of A
are above 0.

- `b` is what B's entry asks: its `value` at the post. `a` is the field's value just after A's last
  entry on the field, in the build's view. So a B that asks `v` where nothing lies between them
  agrees with A wherever A reaches, even where A's mask fades.
- Where B has several entries on the field, each is compared; the results join in one diagnostic.
- They agree when `abs(a - b) <= 0.001` in the field's unit (one millimetre on a field in `m`).
- A mask of 1 means exactly 1.0 in doubles, as the one maths module computes it (`numbers.md` 8);
  there is no tolerance. `1 - smoothstep(0, bank, d)` at `d = 0` is exactly 1.

| Where they disagree | Code | Severity | Post and count taken over |
|---|---|---|---|
| At a post where B's mask is 1 and some entry of A has mask 1 | `authority.conflict` | error | Posts where both masks are 1. |
| Only at posts where at least one mask is below 1 | `authority.squeezed` | warning | Posts where both masks are above 0. |

- One diagnostic per pair of features and field: a conflict if any post qualifies, else a squeeze.
  Its subject is B, the later; A is in `related`. A footprint feature has no id of its own, so it
  is named by its instance's address (`instance.cottage`, as `identity.md` 5 names it). It gives
  the file and line of each one's `authority` (the feature's, or its kind's if inherited, as a
  footprint's always is), `a` and `b` at the post of the largest
  difference (ties: post order), and the number of posts that disagree. Its `data` holds the field
  (`field`), both counts: posts where both masks are 1 (`both_full`), and posts where both are
  above 0 (`both_partial`), and at the reported post what the earlier leaves (`leaves`) and the
  later asks (`asks`).
- Diagnostics are computed over the whole of both features, whatever window was asked for.
- The hint offers the three ways out, in this order: make one `shaped`, so the other holds; move one
  off the other; or let the later one read the earlier (`value = "= v"` where they overlap), so they
  agree. Nesting puts the later one where it can read the earlier.
- A conflict is never silenced. The spike's feature key `over`, which named the anchored features a
  feature beat and silenced both codes, is dropped (principle 3, `vocabulary.md` section 6). A clash
  between two firm decisions is settled by changing one decision, in text, where review sees it.
  `over` is `doc.unknown_key` with the fixed hint: "`over` was dropped; change one decision, see
  `authority.md` 5". The lead is asked to give `documents.md` this fixed hint for removed spike words.
- Two effects of one feature are never compared: the kind's author ordered them.
- Weights fields are not compared: two anchored paints layer by the order (open: whether two
  anchored paints that disagree where both masks are 1 are a conflict; decided in M3 with the court,
  roadway and sidewalk of `quay`).
- Whether 0.001 is right for every field, or a field needs its own tolerance, is (open): decided by
  `ford`'s conformance in M1.

Anchored features are always compared. The spike compared them only on a field with `unit`.

**Hidden entries.** An entry whose every post with mask above 0 is later set by entries with mask 1
leaves no trace in the field. `authority.hidden` (info) says so once per (feature, field), naming
the features that cover it. This is the spike's surprise of a lawn inside a paved court that
vanished; a library lint may judge whether it matters (R15).

## 6. Meetings (R18)

Where two things meet, a named thing owns the meeting, never whichever got there first. In this
chapter's terms:

- The owner is a feature whose kind's entries shape the place (a ford, a crossing, a waterfront).
- What meets it names it: a path with `starts_at` or `ends_at` (its end, its mount, or the owner
  itself when it is a point feature or an instance), a
  thing placed with `on`. Every meeting is a pair of mounts; a path that ends at a place meets that
  place's default mount (R31, `surfaces.md` 3.3). A path meets another thing at its ends or by `follows`, nowhere else
  (`geometry.md`).
- Within one tier, the owner comes after the paths that arrive at it (4.3). So at equal strength
  the owner holds at the meeting. An owner written weaker than what arrives does not hold there;
  that is what the author stated.
- An owner that stands on an anchored feature (a ford `on` an anchored beck) at anchored strength
  asks `v` where they overlap and so agrees (section 5), or is written weaker. Otherwise the two
  disagree and section 5 reports it.
- A meeting nobody owns is two features whose entries overlap with no relation between them. At
  anchored strength, if they disagree, section 5 reports it, and the hint adds: name a feature that
  owns the meeting, and end both at it. At lower strengths the core says nothing; a library lint
  measures it (R15, R17).

## 7. Steps under authority (R14; M2)

A simulation is a way of deriving, so it obeys authority. The contract on the step's side:

1. **Facts in.** The step reads the stated features (their shapes, tiers and attributes) and the
   views of 4.4. How a fact becomes a starting condition is the library's knowledge, not the
   author's and not the core's: a river is a groove in the starting surface, a lake's shore is held.
   The core never promises that "anchored means held" (the `basin` lesson).
2. **One result out.** It returns rows and features (`fields.md` 7, `protocol.md`). It returns one
   result per run; the core has no way to generate many and pick one.
3. **What it returns cannot move a statement.** Its rows sit in the head (4.1), before every
   effect, so every derived, shaped and anchored entry of every layer is applied after them. A
   feature it returns is `derived`, or `shaped` if the step writes so; `anchored` is
   `authority.step_anchored`. A step may not return a feature with an id the world already has
   (`identity.md`), so it cannot rewrite a stated one.
4. **Facts checked after.** The stated values hold in the field by the order. Whether the step's
   result honours them (a lake that is not a dam, a river that keeps to its line) is a library's
   knowledge, so the check is a lint step, which reads the finished build (R29), whose codes are
   `step.<id>.<name>`, each saying where and what would hold instead (principle 8). So that R14
   holds whatever a library writes, a lint step lists the generate steps it checks in a key
   `checks` (open: queued for M2 by R31, decided with `basin`). A generate step that read stated features (by its read
   trace, `protocol.md` 4.4) and returned rows or features, which no lint step names, is
   `authority.unchecked` (error), naming the step and its `when` line. The hint: add a lint step
   that checks it.

The core learns no erosion: the simulation is a step with declared inputs (R14).

## 8. Derived features (M2)

A derived feature is a shape computed from a field: a flood, a contour, a flow line. Its tier is
`derived`, and its shape reads head views by 2.3. Whether it is made by a step or by an engine
operation with no domain noun (R23) is (open): decided in M2 with `vale` (the tarn and the mill
pond) and `basin`.

## 9. Example: a lake beside an anchored pond

A water library declares two kinds (a sketch; the library owns these words):

```toml
[kind.lake]                                # derived: the default
requires = ["water_height"]
radius   = 10

[kind.lake.defaults]
depth = 2.0
bank  = 8

[kind.lake.effect.bed]
field = "role:surface"
value = "= water_height - depth"
mask  = "= 1 - smoothstep(0, bank, d)"

[kind.lake.effect.wet]
field = "tag:water"
value = "= 1"
mask  = "= 1 - smoothstep(0, 1, d)"

[kind.pond]
authority = "anchored"
requires  = ["water_height"]
radius    = 6

[kind.pond.defaults]
depth = 1.2
bank  = 4

[kind.pond.effect.bed]
field = "role:surface"
value = "= water_height - depth"
mask  = "= 1 - smoothstep(0, bank, d)"

[kind.pond.effect.wet]
field = "tag:water"
value = "= 1"
mask  = "= 1 - smoothstep(0, 1, d)"
```

The world:

```toml
name       = "tarns"
extent     = [0, 0, 128, 128]
resolution = 1
include    = ["water"]

[field.height]
role = "surface"
base = "= 30 + 0.05 * x"

[field.wet]
tags = ["water"]

[feature.lake]
kind         = "lake"
region       = [[20, 30], [64, 30], [64, 70], [20, 70]]
water_height = 31.0

[feature.pond]
kind         = "pond"
region       = [[70, 40], [90, 40], [90, 60], [70, 60]]
water_height = 33.0

[change.firm.feature.lake]
authority = "anchored"

[change.wide.feature.lake]
authority = "anchored"
region    = [[20, 30], [76, 30], [76, 70], [20, 70]]
```

The lake asks 29.0 (31.0 - 2.0); the pond asks 31.8 (33.0 - 1.2). The lake's bank reaches 8 m east
of x = 64, into the pond's reach from x = 67 to 71.

| Build | Entries on `height`, in order | Result |
|---|---|---|
| As written | base; `lake.bed` (derived); `pond.bed` (anchored) | Inside the pond, the 32-bit float nearest 31.8 at every post: the lake gave way. No diagnostic. |
| `firm` | base; `lake.bed`; `pond.bed`. Both anchored; no nesting (the pond is not inside the lake); `order` 0; one layer; `lake` < `pond` by place name. | Nowhere are both masks 1. Where the lake's bank fades, the ground it leaves is not 31.8: `authority.squeezed` (warning), 133 posts, largest at (71, 37), where the lake leaves 33.35 and the pond asks 31.8 (1.55 m). |
| `wide` | As `firm`. | Both masks are 1 for x from 70 to 76, y from 40 to 60: `authority.conflict` (error), 147 posts, each 2.8 m apart, reported at the first in post order, (70, 40). Posts where both masks are above 0 and they disagree: 457. |

On `wet`, both kinds ask for 1, so they agree in every build and nothing is reported.

The error in `wide` has `pond` as its subject (the kind's `authority` line, inherited) and `lake` in
`related` (its `authority` line in the change). The hint: make `lake` shaped so the pond holds; or
move it off the pond; or let the pond's bed read the ground the lake leaves (`v`). A flooded lake
that conforms to the ground (section 8) is the same `lake` written as a derived feature; its order
would not change.

## Diagnostics

| Code | Severity | Raised when |
|---|---|---|
| `authority.attribute_twice` | error | An attribute declared in both `defaults` and `requires`. |
| `authority.requires` | error | A feature lacks an attribute its kind requires; names both lines. |
| `authority.names_undeclared` | error | An attribute in a kind's `names` is in neither `defaults` nor `requires`; nearest declared attribute. |
| `authority.ground_requires` | error | An instance grounds with a kind that has `requires`; the instance's `ground` line and the kind's `requires` line. |
| `authority.selector` | error | An effect's `field` is not `role:<role>` or `tag:<tag>`, or names an unknown role; both forms, nearest role, the role of a field named bare. |
| `authority.no_field` | info | A well-formed selector matches no field, and the kind has a feature; once per kind and effect. |
| `authority.range` | error | `radius` at or below 0; the key, value and range. |
| `authority.max_change` | error | An entry moves a value more than `max_change`; post of the largest move, amount, limit, `v` and `value`. |
| `authority.gave_way` | warning | A produced instance's footprint would move a value more than its kind's `max_change`, so the instance is not placed (M3, 2.2); the instance, field, amount, limit and post. |
| `authority.unknown` | error | `authority` is not `derived`, `shaped` or `anchored`; nearest tier. |
| `authority.unknown_kind` | error | A feature's `kind` or an instance's `ground` names no kind in the world's kind namespace; nearest kind. |
| `authority.no_kind` | error | `authority` on a guide. |
| `authority.order_cycle` | error | Nesting on one field forms a cycle; the field, every feature, each link and which side it puts later. |
| `authority.conflict` | error | Two anchored features disagree where both masks are 1; subject the later; values, post, counts: `data.field`, `data.both_full`, `data.both_partial`, `data.leaves`, `data.asks`. |
| `authority.squeezed` | warning | Two anchored features disagree only where a mask is below 1; as above: `data.field`, `data.both_full`, `data.both_partial`. |
| `authority.hidden` | info | An entry left no trace: later entries with mask 1 cover all of it; once per feature and field. |
| `authority.step_view` | error | A step returned rows on a field whose full view it read, or a row sorting before a row of its head view. M2. |
| `authority.step_anchored` | error | A step returned an anchored feature. M2. |
| `authority.unchecked` | error | A generate step that read stated features and returned rows or features is named by no lint step's `checks`. M2, (open: queued by R31). |

## Conformance

Each case runs through the command line (R20). Cases that check an order write each entry as
`value = "= v * 10 + k"` with mask 1 on a field whose base is 0, `k` a digit naming the entry; the
field's value at the post, read from the field file, spells the order (`123` is entries 1, 2, 3).
Six entries fit a 32-bit float exactly.

| Case | M | Input | Action | Must hold |
|---|---|---|---|---|
| default_tier | M1 | A kind with no `authority`, and a shaped one | build | The derived entry's digit first. |
| feature_overrides | M1 | A feature writes `shaped` on an anchored kind, beside a shaped and an anchored feature | build | Its digit between them, as shaped. |
| tier_order | M1 | Derived, shaped and anchored effects on one field, ids reversed against tiers | build | Digits in the order derived, shaped, anchored. |
| across_layers | M1 | A derived lake at rank 0, an anchored pad at rank 1, overlapping | build | The pad holds; the lake gives way (R21's case). |
| order_key | M1 | Two crossing ways in one tier, each with pavement (`order` 0) and roadway (1) | build | Both pavement digits before either roadway digit, in one layer and across two. |
| shuffled | M1 | The same world with its tables and keys shuffled | build | Same digest. |
| nest_inside | M1 | An anchored pad inside an anchored terrace whose bank fades; pad `value = "= v"` | build | Pad digit after terrace digit; no diagnostic. |
| nest_where | M1 | A shaped `where` region reading `wet`, inside a shaped written region, both writing `height` | build | The `where` region's digit after the region's. |
| nest_path_in_region | M1 | An anchored path `a_walk` wholly inside an anchored region `yard`, each entry written as the order cases are, so the two disagree; then the same with one point of `a_walk` moved outside `yard` | build | Inside: the path's digit after the region's (containment, 4.3), and `authority.conflict` with subject `a_walk`, the later. Moved: no nesting, so place name decides: the path's digit first and the conflict's subject `yard` (the spike put every anchored path after every anchored region). No `authority.order_cycle`. |
| nest_on | M1 | A shaped pad `on` a shaped lane, its id sorting before the lane's | build | The pad's digit after the lane's. |
| pad_order | M1 | A pad (`order` 0) `on` a lane with pavement (`order` 0) and roadway (`order` 1) | build | The pad's digit after both of the lane's; the pad holds. |
| follows | M1 | A shaped path that `follows` a shaped beck, its id sorting first | build | The path's digit after the beck's. |
| owner_holds | M1 | Two shaped lanes ending at a shaped ford, ids sorting after the ford | build | The ford's digit after both lanes'. |
| ford_meeting | M1 | A shaped ford `on` a shaped beck, two shaped lanes arriving at the ford | build | Digits: beck, then lanes, then ford; no `authority.order_cycle`. |
| arrive_inside | M1 | A shaped lane ending at an instance `yard` grounded with a shaped kind whose footprint contains the lane | build | The footprint's digit after the lane's; no cycle. |
| order_cycle | M1 | A shaped path `lane` ending at an instance `yard` grounded with a shaped kind; a shaped path `spur` starting at `lane.end` and lying wholly inside `yard`'s footprint (4.3) | build | `authority.order_cycle` naming the field and the three (`lane`, `spur`, `yard.ground`) and each link's key and direction; exit 1. |
| ford_on_anchored | M1 | An anchored ford asking `value = "= v"` on an anchored beck whose bank mask fades | build | No diagnostic. |
| pond_lake | M1 | Section 9 as written | build | Pond interior the 32-bit float nearest 31.8; no authority diagnostic. |
| pond_lake_firm | M1 | Section 9 with `firm` | build | `authority.squeezed`, subject `pond`, 133 posts, (71, 37); exit 0. |
| pond_lake_wide | M1 | Section 9 with `wide` | build | `authority.conflict`, subject `pond`, `lake` related, both lines, 147 posts, 2.8 m, (70, 40), hint; exit 1. |
| agree | M1 | Two anchored features asking values 0.0005 apart where both masks are 1 | build | No diagnostic. |
| disagree_near | M1 | As `agree`, 0.0015 apart | build | `authority.conflict`. |
| mask_exact | M1 | Two anchored regions sharing an edge, masks `1 - smoothstep(0, 4, d)`, values 1 m apart | build | `authority.conflict` at the shared edge (`d = 0`, mask exactly 1). |
| same_feature | M1 | One anchored feature with two effects that disagree | build | No diagnostic. |
| hidden | M1 | A shaped lawn paint wholly inside an anchored court paint, both with mask 1 | build | `authority.hidden` naming the lawn and the court. |
| no_over | M1 | A feature with `over = ["pond"]` | merge | `doc.unknown_key` with the fixed hint naming this chapter's section 5. |
| instance_tier | M1 | An instance with `ground = "pad"` and `authority = "anchored"` | merge | `doc.unknown_key` with the fixed hint of 1.3. |
| ground_requires | M1 | An instance grounding with a kind that requires `water_height` | build | `authority.ground_requires` at the `ground` line. |
| max_change | M1 | A pad on a slope past its `max_change` | build | `authority.max_change` at the largest move; entry applied in full; exit 1. |
| range | M1 | An effect with `radius = 0` | build | `authority.range`. |
| max_change_formula | M1 | `max_change = "= max_cut"` on a pad kind, two pads with `max_cut` 0.5 and 2 on one slope | build | `authority.max_change` for the first pad only. |
| max_change_each | M2 | Two pads of one kind on one slope, both past their `max_change` | build | `authority.max_change` twice, one per pad (2.2: once per effect, feature and field). |
| gave_way | M3 | A step returns two `pad` houses on a slope, one past `max_change`, and the world writes a third past it by hand | build | `authority.gave_way` for the produced one past its limit, which is not placed; the other produced one stands; `authority.max_change` for the written one; exit 1. |
| max_change_negative | M1 | `max_change = "= -1"` | build | `formula.range` at the feature. |
| bare_field | M1 | An effect with `field = "height"` | build | `authority.selector`, hint `role:surface`. |
| bad_role | M1 | An effect with `field = "role:surfce"` | build | `authority.selector`, nearest `surface`. |
| no_field | M1 | An effect on `tag:snow` in a world with no such tag, its kind with one feature | build | `authority.no_field` once. |
| where_head | M1 | A `where = "= f('height') < 10"` region whose kind lowers `role:surface` by 2, on a base rising through 10 | build | No cycle; the region is where the base is below 10, and the ground is lowered by 2 there; adding a `then` row that raises the ground moves the region (`formulas.md` case 34). |
| where_head_through | M1 | A `where` region reading `slope`, whose base reads `height`; another region's effect steepens `height` inside it | build | The region is the same with and without that effect; its own entries on `height` are made. |
| where_head_cycle | M1 | A `where` region `r` reading `height`, and a `then` row on `height` with `mask = "= if(inside('r'), 1, 0)"` | build | `graph.cycle`; exit 1. |
| on_where | M1 | A point feature `on` a `where` region reading `height`, its kind writing `role:surface` | build | No cycle; the point's entry applied after the base and head. |
| name_attr | M1 | The plan's `ford` with the bookmark `toward = "ford"` | build | `features.json` gives `toward` as `{ "name": "ford", "at": [...] }`, `at` the ford's point; the graph holds the edge `attr:at_ford` to `shape:ford`. |
| name_unknown | M1 | As `name_attr`, `toward = "frod"` | build | `identity.unknown_place`, hint `ford`; exit 1. |
| name_moved | M1 | As `name_attr`, a change moving the ford | build | The change report lists `at_ford` with the ford's move as cause. |
| name_upward | M1 | A bookmark in layer 0 whose name attribute names a feature in layer 1 | build | `doc.upward_ref`. |
| names_undeclared | M1 | A kind with `names = ["toward"]` and no `toward` in `defaults` or `requires` | build | `authority.names_undeclared`. |
| measure_outside | M1 | Extent `[0, 0, 16, 16]`, resolution 1; a region `[-4, 4, -1, 8]` (no post inside the extent) whose effect, `radius` 3, reads `under('mean')` | build | `num.empty_reduction` naming the region: its window meets the extent within `radius`. |
| requires | M1 | A pond without `water_height` | build | `authority.requires` naming `water_height` and the kind's line. |
| guide_tier | M1 | A guide with `authority = "anchored"` | build | `authority.no_kind`. |
| unknown_kind | M1 | A feature with `kind = "pnod"`, and an instance with `ground = "pnod"`, in section 9's library | build | `authority.unknown_kind` at each line, hint `pond`; exit 1. |
| window | M1 | Section 9 `wide` | evaluate `height` whole and by windows, one cut through the overlap (as `numbers.md`'s window_equals_whole) | The window equals the same cut of the whole; diagnostics equal the whole build's. |
| step_head | M2 | A generate step that reads `height`'s head view and returns a row; an anchored pond below it | build | The pond's value holds over the row; a clean build equals the incremental one. |
| step_view | M2 | A step that reads `height`'s full view and returns a row on it | build | `authority.step_view`. |
| step_order | M2 | A step that reads `height`'s head view, holding a text row with `order` 1, and returns a row with `order` 0 | build | `authority.step_view` naming both rows. |
| step_features | M2 | A step that reads `height`'s full view and returns a feature whose kind writes `height` | build | No diagnostic; the feature's entry in its tier's place. |
| step_anchored | M2 | A step that returns an anchored feature | build | `authority.step_anchored`. |
| step_full_view | M3 | A worker in layer 0 reading `height`'s full view; a feature of layer 0 whose kind adds 1 at one place; an instance in layer 1 with `ground = "pad"` whose kind adds 10 at another; the step returns what it read as its own field | build | No `doc.not_yet`; the returned field holds 1 at the feature and 0 under the pad; `height` holds 1 and 10. |
| step_own_returns | M3 | A worker in layer 0 reading `height`'s full view and returning a feature whose kind adds 1 at one place | build | No `diag.internal` on a cold store; the returned field holds 0 at the feature; `height` holds 1. |
| unchecked | M2 | A generate step that reads anchored features and returns rows, with no lint step naming it | build | `authority.unchecked`. |

## Changed from the spike

- Default tier is `derived`, not `shaped`: a kind that says nothing gives way.
- Order: the spike sorted by tier, `over` count, area of anchored regions, kind position and
  feature position. Now: tier, nesting, `order`, layer rank, place names. Declaration order is gone.
- Nesting covers every tier and four relations (on, follows, arrival, inside), not anchored regions
  by area only. Anchored paths no longer all come after anchored regions.
- `over` is dropped; anchored features are always compared, not only on a field with `unit`.
- A conflict is judged where both masks are 1 and values differ by more than 0.001, not at the
  spike's 0.5 mask and 0.05 difference; the later feature is compared with what the earlier left.
- `order` lives on effects and `then` rows only; the spike's feature `order` is dropped.
- `reach` is `radius`; `check_where` is dropped; `max_change` stays an error but is now reported
  per feature at its worst post.
- `field = "#tag"` is `field = "tag:<tag>"`, and the old form is an error with the new one as hint;
  an effect on no field says so.
- A step's rows sit in the head, before every effect, so effects below it are no longer applied
  twice (SPEC 12.8).
- A shape that reads a field reads its head view, so effects of a feature whose shape reads the
  field, or that is placed on such a shape, are applied, not silently lost (SPEC 11). The spike's
  flood read the finished field.
- `hosts`, `encloses`, `opening`, `link`, `light`, `measure`, `map`, `module_m` leave the core for
  libraries (`vocabulary.md` section 6). `authority.cycle` becomes `graph.md`'s one cycle report,
  and the flood's own rule becomes 2.3 for every shape that reads a field.

## Open

| Question | Decided by |
|---|---|
| Whether two anchored paints that disagree where both masks are 1 are a conflict. | M3, the court, roadway and sidewalk of `quay`. |
| Whether 0.001 suits every field or a field states its own tolerance. | `ford`'s conformance, M1. |
| Whether a step may pass an author's anchored tier to the features it lowers (a library rule written as a step). | `protocol.md`, by M3 (the kit run as a building library rule; `city`). |
| Whether a derived feature is made by a step or by an engine operation. | M2, `vale` and `basin`. |
| Whether effects on `volume` fields obey the tiers. | M5, `coast`. |
| The lint key `checks` and `authority.unchecked` (section 7). | Queued for M2 by R31; `basin`. |
