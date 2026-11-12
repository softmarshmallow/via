# Names and identity

This chapter says how every thing in a build is named: authored things by the id the user wrote,
produced things by where they are. It also says how an author selects a produced thing by place,
how hand edits (pin, clear, lock) hold while producers change under them, and how a build reports
what changed, by name and by cause.

Words are those of `spec/vocabulary.md`. Authority: `docs/doctrine.md`, chiefly R3, R4, R13 and R21.
Words this chapter adds to the vocabulary are listed in section 9; they are owed to that page in
the same change.

## 1. Names

| Name | Written by | Form | Example |
|---|---|---|---|
| authored id | the user, in text | an id (vocabulary 2.1), at most 48 bytes | `cottage`, `beck` |
| produced name | the core, for a maker's output | `<maker>#<place key>` | `trees#s7000_12_n3` |
| place name | the user, to point into a thing | ids joined by dots (vocabulary 2.1) | `beck.end`, `a.quay_road.end` |

- An authored id belongs to the user. The core never makes, changes or reuses one. A produced name
  always holds `#`, and an id never does, so the two cannot collide.
- A **maker** is anything whose output is named `<maker>#<key>`: a producer (a rule or a step,
  vocabulary section 1), a network, a cells table, or a feature written with `where`. "Producer"
  keeps its narrower meaning: a maker that places instances, so the only thing a hand edit's `by`
  may name. How a network and a cells table are declared: `geometry.md` (open there).
- A set item is `<instance>.<item>`. An item of a set placed by a producer is
  `<producer>#<key>.<item>`.
- A produced name from inside an import is reported as `<import>.<produced name>`:
  `a.trees#s7000_4_7`. This form appears only in diagnostics, the change report and lock records;
  it is never written in text (section 3), so the place-name grammar of vocabulary 2.1 is unchanged.
- Names exist for the change report, for diagnostics and for lock records. They never hold a hand
  edit (R3; section 6).

## 2. Place keys

A place key says where a produced thing is. It is built only from integers that describe its place
and from the names of the things it is made from. It is never built from a counter: no count over a
maker's output, no order of evaluation, no thread, no time.

### 2.1 Ordinals, and why one is not a counter

Where every other part of a key is equal, a key ends in an **ordinal**: the thing's position among
exactly those things that share every other part, in an order fixed below for each class. An
ordinal changes only when one of the things it is counted among changes. A counter over a maker's
whole output changes when anything earlier changes, which is the failure the spike measured
(`lots_979` became `lots_965` when an unrelated district changed). This distinction is ours.

Every ordinal and station is fixed **before** any filtering. Keep, give-way, the extent, `within`,
merging and hand edits remove things; they never renumber what is left. A clump member keeps its
draw index when member 0 is dropped; a run's station `k` is `k` even when member `k` is not made.

### 2.2 Form

- A place key matches `[a-z][a-z0-9_]*`, at most 64 bytes. Its first letter tells a reader which
  class made it (2.3); selection does not read the letter (section 3).
- Integer parts are written in decimal and joined by `_`. A negative integer is written with `n` in
  place of `-`: lattice index (12, -3) is `12_n3`.
- Every position in a key is in integer millimetres in the frame the maker is written in (R24,
  `numbers.md`). No floating-point value enters a key.
- Names that enter a key (sources, junctions, an area) enter through a **digest**, written as 16
  lowercase hex digits of a 64-bit value:

  ```
  digest(class, parts) = stream(0, ["name", class] ++ parts)
  ```

  `stream` is `numbers.md` 7.2's, with seed 0 always, so a name never depends on the world seed. The
  first part, `"name"`, says who draws, as `numbers.md`'s table of users requires (a row owed to
  that table). `class` is the class word of 2.3 (`"junction"`, `"edge"`, …). This is one function with
  the seed derivation, not a second one (principle 10). It is not cryptographic; a collision is
  caught (2.5).

**Byte encoding of parts.** `stream` absorbs each part with its length in front, so a list of parts
is unambiguous once every part has a fixed encoding:

| Part type | Bytes |
|---|---|
| name | Its UTF-8 bytes. A produced name or key is a name. The empty name is zero bytes. |
| integer | 8 bytes, two's complement, little-endian. `-3` is `fd ff ff ff ff ff ff ff`. |
| side | 1 byte: `00` left, `01` right, of the source's direction of travel. A ring source's inside is its left. |
| role | 1 byte: `00` the source starts here, `01` it ends here, `02` it passes through. A ring only passes. |
| pair | Its two members as two consecutive parts. |
| set | One integer part holding the count, then each member, sorted: names bytewise; pairs by first member bytewise, then by second byte. A repeated member is kept (a multiset). |

### 2.3 Classes

Each produced thing has a **class**, a word from this closed list. `class` selects by it (section 3).

| Class | Thing | Key | Parts digested | Ordinal by | Example |
|---|---|---|---|---|---|
| `instance` | a scatter's instance | `s<spacing mm>_<i>_<j>` | none | none | `trees#s7000_12_n3` |
| `instance` | a clump member | parent's key, `_m<draw index>` | none | none | `trees#s7000_12_n3_m2` |
| `instance` | a run's instance | `r<station>` | none | none | `fence#r7` |
| `junction` | a network junction (`geometry.md` 3.2) | `j<digest>_<ordinal>` | set of (source, role) pairs | lowest `t` along the bytewise-lowest source | `streets#j82df4d89dc3cb626_0` |
| `edge` | a network edge | `e<digest>_<ordinal>` | its two junction keys, sorted bytewise; set of its sources | lowest `t` along the bytewise-lowest source | `streets#eedaa5ba9984d5e26_0` |
| `area` | a network area | `a<digest>_<ordinal>` | `sides`: set of (source, side) pairs | `point`, `x` then `y` | `streets#a43c6f2eb9128ebc7_0` |
| `cell` | a cell | `c<digest>_<station>` | source, side, stretch anchor, stretch ordinal | station within the stretch | `lots#c5a52c2461d4ce6d7_3` |
| `rest` | what lies deeper than a cell's depth | `b<digest>_<ordinal>` | the divided region's name | `point`, `x` then `y` | `lots#b97d328fbf7300eb3_0` |
| `piece` | a piece of a `where` region | `p<i>_<j>_<ordinal>` | none | `point`, `x` then `y` | `meadow#p2_5_0` |
| `feature` | a step's returned feature | the step's key (2.4) | as the step builds it | as the step builds it | `village#lot_c5a52c2461d4ce6d7_3` |
| `field` | a step's returned field (open: needs ruling, R21; `fields.md` 7) | the step's key (2.4) | none | none | `erosion#eroded` |
| `row` | a step's returned `then` row | the step's key (2.4) | none | none | `erosion#erosion` |

Each class in more detail:

- **Scatter.** The lattice of `rules.md` 7.1: squares of side `spacing`, anchored at the origin of
  the frame the scatter is written in, never at the extent's corner. The key holds the spacing in
  integer millimetres and the **lattice index** `(i, j)` the candidate was drawn for, not the
  square its final position lies in, so jitter never renames. A change of spacing is a new
  lattice, so it renames every instance: the report shows removals and additions, never moves
  (`trees#s7000_5_3` and `trees#s6000_5_3` are different things). This relies on `rules.md`
  drawing at most one parent per lattice square; if that changes, this row changes.
- **A new seed** leaves the key alone. A lattice index kept in both builds keeps its name, so its
  entry is moved or swapped. Where density, `keep_out` or giving way make `keep` differ (`rules.md`
  7.1 draws it from a stream that holds the world seed), an index is kept in one build only and
  shows as added or removed.
- **Clump member.** Its draw index is the member index of `rules.md` 7.1, whose stream is (world
  `seed`, the scatter's `seed`, purpose, `i`, `j`, member index). Members are class `instance` like
  their parents: `of = "trees"` selects both, which is meant.
- **Run.** Station `k` counted from `span[0]` toward `span[1]`. A run's instances are a sequence by
  intent, so the station is the place.
- **Junction.** `geometry.md` 3.2's junction. Its parts are each source that meets there, with its
  role: `start`, `end` or passing through. A path's two ends and a point where it crosses itself
  all have distinct parts, so a new self-crossing does not rename the path's far end. When a source
  passes one point twice, the lower `t` orders it. Most junctions are unique by their parts, so
  most ordinals are 0. "Node" is not used for this: it is the graph's unit of work (`graph.md`).
- **Edge.** An edge lying on several sources (`follows`, or geometry's `overlap` fixture) has every
  one of them in its set. The ordinal separates edges with the same junctions and sources (a loop).
- **Clip outline.** The outline of a network's `clip` region (`geometry.md` 3.2) is the source named
  `#clip`, a name no id or produced name can be.
- **Area.** Its parts are `geometry.md` 3.2's `sides`, already in the set order above. The outside
  of a network is not an area and has no name (`geometry.md` 3.2 returns enclosed areas only).
- **Cell.** A **frontage** is one source on one side, as `geometry.md` 3.6 returns it. Along it,
  cells are cut per **stretch** between corners. A stretch is keyed by its **anchor**, the key of
  the last junction at or before its start along the source's travel (the empty name if none), and
  by its stretch ordinal counted from that anchor. The station counts cells within the stretch from
  its start. So adding a cell, a corner or a merge changes only the names in that stretch and the
  stretches up to the next junction, inside `geometry.md` 3.6's Local guarantee. A merged cell keeps
  the station of the cell it was merged into. The area is not in the key: the anchor already tells
  apart the blocks along one side of a street.
- **Rest.** `geometry.md` 3.6's `rest`, one name per piece, keyed by the name of the region that was
  divided (an area's produced name or a region feature's id).
- **Piece.** A region written with `where` (every piece kept, vocabulary Features). `(i, j)` is the
  coarse lattice square, side 32 m, anchored at the frame origin, that holds the piece's `point`.
  The ordinal separates pieces whose points share a square.

**The anchor point.** Areas, rest pieces and `where` pieces are ordered by `geometry.md` 3.2's
representative point (`point`): the post of the world grid farthest from the outline, ties to the
lowest `y` then `x`, with the halving search when no post is inside. It is fixed exactly, so every
implementation gives the same ordinal; polylabel is not used (`geometry.md` 3.2 says why). Every
region has one, so no piece lacks an anchor. It only orders things that already share every other
part, so a point that jumps changes an ordinal only among those.

### 2.4 Keys from steps

A step names what it returns. Every table a step returns (`protocol.md` 5) carries a `key`, and
the core forms `<step>#<key>`:

| Returned | Class | Key |
|---|---|---|
| a feature | `feature`, or a class of 2.3 the step states (a `cell` it built as the core would) | a place key the step builds (the SDK's naming calls, below) |
| an instance | `instance` | likewise |
| a field (open: needs ruling, R21; `fields.md` 7) | `field` | the step's own word, such as `eroded`: one thing, not one of many places |
| a `then` row on a field (`fields.md` 7, `authority.md` 4.1) | `row` | the step's own word, such as `erosion` |

A returned item with no class stated is `instance` if it is an instance, `field` if it is a field,
`row` if it is a row, and `feature` otherwise. A key that breaks 2.2, or a class not in 2.3, is
`identity.bad_key`. All classes of one step share one namespace: a field and a row with one key are
`identity.key_collision` (2.5).

A field and a row have no place, so they carry no digest and no ordinal, and they are not
selectable: `class = "field"` or `class = "row"` in a selection is `identity.no_place`. A higher
layer reads a returned field through `f('tag:<tag>')`, using a tag the step sets on its field
(`fields.md` 7, `formulas.md` 7). A returned field's produced name appears only where vocabulary
2.1 allows (section 3). A `then` row's produced name orders the head (`authority.md` 4.1) and is
reported; it is never written.

The SDK will offer each class of 2.3 that has a place as one batch call (R23), with the same results as the engine,
so a library names a lot exactly as the core names a cell (R19b: no private door). Those calls are
owed by the first milestone with a generate step that returns features (M2 at the earliest,
`plan.md`); M1 owes only the digest vectors (section 10; case 45).

A step that numbers its output with a counter is not caught by the core; the change report shows
the cost as removals and additions where moves were expected. A user's step is recorded, not
promised (R26).

### 2.5 Uniqueness

Two things of one maker with one key is `identity.key_collision`, naming both positions. The core
never disambiguates by appending a counter. With `for_each` (`graph.md`), the maker is the step and
the key must already differ between items; keys built from place do. A collision among keys the
core built itself (a digest collision, or a key rule that failed) is a defect of ours and is
raised as `diag.internal`, exit 3.

## 3. Selecting by place

An author points at a produced thing by a point near it, never by its name:

```toml
[instance.bench]
asset = "bench_plain"
on = { of = "streets", class = "edge", near = [131.5, 88.0], t = 0.5, offset = 3 }
```

- `of` names a maker by its id. `class` names one class of 2.3. A maker whose output has one class
  needs no `class`; for any other, leaving it out is `identity.of_ambiguous`, whose hint lists the
  classes. A network has three (`junction`, `edge`, `area`), so it always needs one.
- `near` is a point in the frame of the table it is in.
- The selection resolves to the candidate nearest `near`: distance to a point, to a path, or to a
  region (0 inside it). Distances are compared exactly in integer millimetres (`geometry.md`); ties
  break by name, bytewise. A maker with no candidate of the class is one `identity.near_none` for
  that maker and class, listing every selection it left with nothing. A produced thing that others
  depend on and that is gone is said once, at its maker, not once per dependant.
- `on` takes `ref` or `of`, never both (`identity.on_both`).
- Once resolved, the selection is a place like any other: `t`, `offset` and `face` apply as in the
  placement table of the vocabulary, with these starts and directions for produced things:

  | Class | `t = 0` and direction |
  |---|---|
  | `edge` | The end where travel along its bytewise-lowest source begins; along that travel. |
  | `area`, `cell`, `rest`, `piece` | Its least vertex (`x`, then `y`), counter-clockwise (`geometry.md` 2.5). |
  | `instance`, `junction` | No length; as for a point in the vocabulary. |
  | `feature` | As the vocabulary says for its shape. |

- An edge is cut wherever another source joins it, so `t` on an edge jumps when a side street
  arrives, and the selection then resolves to the part nearest `near` and reports
  `identity.reselected`. To stand a thing along a generated street, select the street itself (a
  step's `feature`, whose `t` runs over its whole length), not an edge of the network.
- **Read point.** A selection reads the maker's output after that maker's hand edits, as every
  reader does (`rules.md` 3.4 and 9), with one exception: a hand edit's own selection reads the
  producer's output before hand edits (4.1). Hand edits never change features, so for every class
  but `instance` the two are the same.
- The selecting thing depends on the maker in the graph (`graph.md`).
- Every build records what each selection resolved to. When it resolves to another name than in the
  baseline, `identity.reselected` names both. This is how a selection that slid to a neighbour is
  seen (principle 4). It depends on the baseline, so it is a run diagnostic, never in
  `diagnostics.json` (`build.md` 11.1, R26).
- `within` (vocabulary, Rules) takes a region feature's id or a footprint feature's place name
  `<instance>.ground` (`rules.md` 2.1), or `{ of, class, near }` resolving to an `area`, `cell`,
  `rest` or `piece`. A produced name is not accepted there. This closes the item
  vocabulary and the earlier draft left open for M3.

**Produced names are never written in text** (R3). A value containing `#` in any key that takes a
name or a list of names, in world text, layers, change files or libraries, is
`identity.key_in_text`, whose hint gives the `{ of, class, near }` form at the thing's current
position. That covers `ref`, `within`, `starts_at`, `ends_at`, `follows`, `by`, `of`,
`clump.around`, `give` and formula selectors (`formulas.md` 7). Produced names appear only where
nothing is placed: diagnostics, the change report, lock records (4.4), the command line and the
messages between the engine and a step (`protocol.md`). A step's own parameters are
opaque to the core and not checked; a step that turns one into a placement is held by its record
(R26), not by this rule.

## 4. Hand edits: pin, clear, lock

Pins, clears and locks are hand edits, stored as intent at a place (R3). The keys are in
vocabulary, Hand edits.

### 4.1 How producers read hand edits

- **Layers.** A hand edit reaches down only, as every reference does (doctrine, layer; R21). Its
  `by` defaults to every producer in its own layer or below. Naming a producer in a higher layer is
  `documents.md`'s upward-reference error. A hand edit from a change belongs to the layer
  `documents.md` merges its table into.
- **Two stages per producer.** A producer's run gives its output unheld. A second stage applies the
  hand edits whose `by` names it and gives its held output. Everything that reads the producer's
  instances reads the held output, except a hand edit's own selection (section 3), which reads the
  unheld output. So a clear `on = { of = "trees", near }` speaking to `trees` is no cycle: the
  clear reads the run stage, and the held stage reads the clear.
- **Views.** A generate step in layer `L`, and every node in its view, reads a producer's output
  held by the hand edits in layers up to `L` only; the build's own output carries them all. This is
  R21's rule for a step's view, applied to instances; its cost (one evaluation per distinct view) is
  R21's accepted cost. A lint or export step reads the build's own output, with every hand edit
  (R29, `graph.md` 4).
- **Steps.** A step's run stage does not read hand edits, so editing a pin or clear re-applies the
  held stage and does not re-run the step. A step that wants to lay out around hand edits declares
  it (a flag owed to `protocol.md`); it is then handed the hand edits visible from its layer, they
  become inputs of its run, and what it returns is held all the same.
- The core applies hand edits; a producer cannot skip one, and a step's output is held exactly as
  a rule's is.

Hand edits apply to each producer's output in this order:

| Order | Hand edit | What it does to that producer's output |
|---|---|---|
| 1 | lock | Inside its `region`, the output is replaced by the lock's record (4.4). |
| 2 | clear | Removes every instance whose origin is inside its place (4.3), recorded items included. |
| 3 | pin | Places its instance once, as a written thing (4.2); removes every instance whose footprint overlaps the pin's, except recorded items: an overlap with one is `identity.lock_pin_clash` (4.4). |

The order is ours. A clear inside a lock refines it: "keep this wood as it is, but empty this spot".
A pin comes last because it is the most specific statement.

Hand edits remove and replace instances only. A hand edit never removes a produced feature (a path,
an area, a cell): other things are built on it, so removing one is a change to what its maker
reads, not to what it returns. Whether a clear may remove a produced feature is (open): decided with
M3's `quay` if a case asks to remove one generated street.

Scatter does not redraw around hand edits. Its draws are unchanged and hand edits only filter them,
so a clear removes what it covers and nothing else moves.

### 4.2 Pin

- A pin's instance is named by the pin's id (world namespace) and is a written thing: table
  `instance`, summary maker `null`, cause `pin.<id>`. It is placed once whatever `by` lists and is in
  no producer's held output; `by` says only whose instances it removes.
- Every instance a pin removes is listed by one `identity.pin_removed` (info) per pin: the count and
  the names. A pin that removes nothing raises none. With the default `by`, a pin inside a cell's
  building removes the building, and this is the message that says so (R13).
- An overlap with an instance of a producer outside `by` is `identity.pin_overlap` (warning, naming
  both). Two pins overlapping are `identity.pins_overlap` (error).
- Where `keep_out` is above 0: `identity.pin_in_keep_out` (warning).
- Inside a clear: `identity.pin_in_clear` (warning, not error; R31). The pin, applied later, wins
  and the warning says so. A clear speaks only to the produced instances of its `by`; a pin is in
  no `by`, so the two do not clash (principle 3). "Empty this spot, then put the well in it" is a
  natural edit; the warning is for a pin dropped into a spot meant to stay empty.

A pin `on` something follows it: "this bench, beside that door" is `on = { ref = "door_mark" }` or
`on = { of, class, near }` with `t` and `offset`. The pin moves with the thing; its intent is the
relation.

### 4.3 Clear

- `at`, or `on` a point or an instance, with `radius`: instances whose origin is at distance
  `<= radius` from that point.
- `on` a path or an edge with `t` and `radius`: from the point at `t`. Without `t`: from the
  nearest point of the whole path, so the clear is a strip.
- `region`: instances whose origin is inside it.
- `on` a region (a region feature, or an `area`, `cell`, `rest` or `piece` selection) without
  `radius`: instances whose origin is inside that region. This is how one cell's building is
  cleared: `on = { of = "lots", class = "cell", near = [x, y] }`. Allowing `on` without `radius`
  here widens the vocabulary row for `[clear]` (section 9).
- A point on an outline is inside, as in `geometry.md`'s point-in-ring test. All tests are exact,
  in integer millimetres.
- A clear that removes nothing is `identity.clear_empty` (warning), on every build, whatever the
  baseline: the same text gives the same `diagnostics.json` (principle 7, R26). It is the message
  the spike lacked: its clear on a generated building did nothing and said nothing (R13). A clear
  kept as a precaution warns until it is deleted; whether an unchanged empty clear should be quieter
  is (open): decided by the first fresh-agent run of M1, and only by a rule that reads text, not
  the baseline.

### 4.4 Lock

A lock keeps a finished area as it is while the producers around it change (R4). `[lock]` arrives
in M3 (vocabulary section 5).

**Recording.** `via lock <world> <lock id>` writes the record; nothing else does, and never a person
(principle 1). The command's exact form: `build.md`. It:

1. builds the world with that lock's replay switched off (so a missing record is no error);
2. refuses if the build has any error: exit 1, with the build's diagnostics, and no record written;
3. writes the record: every instance of the producers in the lock's `by` whose origin is inside
   `region`, after the clears that speak to them and before pins (a pin is a hand placement and is
   never recorded). A set placed by a producer is recorded item by item, with full names
   (`fence#r7.post`);
4. if a record already existed, replaces it and gives `identity.lock_rerecorded` (info) with the
   counts of added, removed, moved, swapped and changed items.

An unknown lock id is `identity.unknown_lock` (error, exit 1, nearest id as hint).

**Record.** A file beside the file that declares the lock, at `locks/<lock id>.toml`. The path is
fixed, so the lock table needs no key for it. Flat TOML, one table per item keyed by its produced
name, sorted bytewise:

```toml
["trees#s7000_12_n3"]
asset           = "tree_oak"
at              = [87.412, -19.006]   # exact millimetres, numbers.md 3.4
face            = 212.5               # exact millidegrees, numbers.md 3.2
scale           = 1.08                # shortest round-trip decimal, numbers.md 3.4
tint            = 1                   # likewise
recorded_height = 23.871              # surface height when recorded; compared, never placed
```

These six keys are the record's whole format. Every value is written so that reading it back gives
the same bits (numbers.md 3.4), so replay is byte-identical (R26).

**Replay.** Each build places the record inside `region` in place of the producers' output there.
Plan position, facing, asset, scale and tint come from the record; height comes from the surface
beneath, as for any instance (`surfaces.md`). A recorded name wins over the live output: a live
instance anywhere in the build whose name is in the record is dropped, so one build never holds
two things with one name. This happens when a new seed or a moved lattice carries a recorded name
outside the region. Derived output gives way to the record (principle 3), and `lock_differs`
counts what was dropped.

**What it would have done.** When the producers' live output inside the region, after the clears
that speak to them, differs from the record after the same clears, `identity.lock_differs` (info)
says, per producer, how many it would now add, remove, move, swap and change there, and how many
live instances outside the region were dropped for a recorded name. It is printed with every
build, so a lock never rots unseen (R4).

**When its ground changes.** The record holds plan, not height, so items settle onto the new
ground. Items whose height differs from `recorded_height` give one `identity.lock_ground_moved`
(warning) per lock on every build, with how many and the largest difference, until the lock is
re-recorded. The change report lists them as moved only on the build where the ground changed, with
the cause that moved it.

**Clashes.** Where a recorded item now stands where `keep_out` is above 0, or overlaps an instance
of a producer outside `by`, the pin warnings apply to it. An item that overlaps a pin is
`identity.lock_pin_clash` (error): two firm decisions disagree (principle 3), and the pin removes
nothing. An item whose asset the pack no longer binds is `identity.lock_asset_missing` (error).
A lock declared without a record is `identity.lock_unrecorded` (error).

## 5. The change report

Every complete build writes a change report against a **baseline**: the last complete build written
to the same build directory, or the one `--against` names (`build.md` 11.2, which also gives the
file). A failed build never becomes a baseline. Which nodes ran and why: `graph.md`. This chapter
fixes how things are matched and what an entry says. Without a baseline the report says so
(`identity.no_previous`, info) and lists nothing.

- **Same thing.** Two builds hold the same thing when they hold the same name in the same table.
  Nothing else matches them: not position, not asset. A footprint feature (`rules.md` 2.1) has no
  id of its own; it is reported under its instance's name in the table `footprint`.
- **Entries.** Every feature, instance (authored or produced), footprint feature and field it
  touched. An entry is added, removed, or one or more of moved, swapped and changed:

  | Entry | When |
  |---|---|
  | added | The name is in this build only. |
  | removed | The name is in the baseline only. |
  | moved | Same name; its position differs in integer millimetres, or its facing differs. |
  | swapped | Same name, another asset. |
  | changed | Same name; another value that is not position, facing or asset: scale, tint, kind, an attribute, or any resolved fact `build.md` 7 records for its asset that differs from the baseline's (`size` for a primitive or engine-made one, `bounds`, `footprint`, `origin`, `sink`, `max_relief`, `solidity`, mounts; `assets.md` 7). |

  An entry that moved and swapped is listed once, carrying both, and is counted under each. An
  entry whose cause includes a differing asset fact carries `facts`, each differing fact old and
  new (`assets.md` 7). So when a pack swap makes a house larger under the same origin, the instance
  is `changed` with `facts`, and its footprint feature is `moved`, since its bounding box differs.
- **Position**, before and after: for an instance, its origin (plan and height) and facing; for a
  path, a region, a footprint feature or a field's changed samples, the bounding box in integer
  millimetres. For a field, the entry is the region whose samples changed (`graph.md` 6.4).
- **Cause.** Every changed statement the thing depends on, sorted bytewise, each in one of these
  forms:

  | Form | For |
  |---|---|
  | `world.<key>` | A top-level key: `world.seed`, `world.extent`. |
  | `<table>.<id>.<key>` | An edited key, with its file and line: `scatter.trees.spacing`. A hand edit is a cause like any key: `clear.well_spot.on`. |
  | `<table>.<id>` | A table added (file and line in this build) or removed (file and line in the baseline). |
  | `asset.<asset>.<fact>` | An asset fact that differs from the baseline's, written with the asset id, never the instance id (`assets.md` 7): `asset.house_low.bounds`. Given beside `world.pack` when a change writes it. |
  | `file:<path>` | A declared outside file (a field's `file`, a step's `inputs`), with its old and new digests. |
  | `step:<id>` | A step whose recorded command or code digest changed (R26). |
  | `select:<change>` | A change selected in one build and not the other, only where no statement it wrote explains the difference. |

  A change selected in one build and not the other is otherwise never a cause in itself: its causes
  are the statements it wrote, in the forms above, with file and line in the change file
  (`feature.ford.on`, `world.pack`).
  Each cause carries `via`: the references the change followed to reach this thing, as the
  shortest chain in the graph, ties by node id (`graph.md` 6.4).
- **Summary.** One line per maker, in one fixed form, with the bounding box of what changed:
  `trees: +12 -30 ~4 swapped 0 changed 0, 247 untouched`. `~` counts moved.

The report's file and JSON fields: `build.md` 11.2.

## 6. Why no override by id

The spike built both designs side by side on `example`: `[[override]]` patched a produced instance
by id; pin and clear held intent at a place. Three trees were moved by hand, then the rule under them
changed (`spike/FINDINGS.md`, "Three questions only doing could settle").

| Rule change | By id | By place |
|---|---|---|
| none | 3 of 3 | 3 of 3 |
| density to 60% | 2 of 3, 1 orphan | 3 of 3 |
| spacing 7 m to 6 m | 0 of 3, 3 orphans | 3 of 3 |
| seed changed | 2 of 3, 1 on a different tree | 3 of 3 |
| the wood retreats north | 2 of 3, 1 orphan | 3 of 3 |
| a new road through a moved tree | 3 of 3, silently on the road | 3 of 3, 1 conflict reported |

After a spacing change the things are different things, and the key says so: the spacing is in it,
so every name changes and the report shows removals and additions; the place survives. After a new
seed, a kept lattice index keeps its name though its tree moved and may be another asset: the
by-id edit silently held a different tree, a failure place-based edits cannot have. Place keys make
names stable enough to review a change; they are not stable enough to hold an edit, and are not
offered for it (R3). `identity.key_in_text` enforces this.

## 7. R13's four obligations

| Spike failure (R13) | Answered by |
|---|---|
| A clear on a generated building did nothing. | Hand edits speak to every producer, steps included (4.1); `identity.clear_empty`. |
| A pin landed inside a building, no message. | `identity.pin_removed` with the default `by`; `identity.pin_overlap` outside it. |
| Nothing could be placed on a generated street. | `on = { of, class, near }` (section 3). |
| Ids shifted when an unrelated district changed. | Place keys (section 2); conformance 25 to 28, and 47 in M1. |

## 8. Worked example

```toml
[scatter.trees]
assets  = ["@plant/tree"]
spacing = 7

[pin.old_oak]
asset = "oak_old"
at    = [40.0, 22.5]

[clear.well_spot]
at     = [61.0, 30.0]
radius = 4
```

The instance drawn for lattice index (5, 3) is `trees#s7000_5_3`. The pin removes `trees#s7000_5_3`
and `trees#s7000_6_3`, whose footprints overlap the oak, and `identity.pin_removed` names both. The
clear removes `trees#s7000_8_4`. Spacing changes to 6: every name becomes `trees#s6000_…`, so the
report lists each old name removed and each new name added, cause `scatter.trees.spacing`. The oak
stands and the well spot stays empty; the report lists neither, because their text did not change.

## 9. Owed to other pages

This chapter needs these edits elsewhere, made by the lead in the same change:

- `rules.md` 7.1: "spacing re-lays everything" now also holds for names.
- `protocol.md`: a returned item's `class` (now with `field` and `row`, 2.4), and a step's flag for
  reading hand edits (4.1). Sections 5 and 9: a higher layer reads a returned field by
  `'tag:<tag>'`, not `f('<step>#<key>')` (2.4, `formulas.md` 7).
- `build.md` 11.1: add `identity.reselected` to the run diagnostics (section 3).
- `geometry.md` 3.2: `identity.md` keys areas by `sides` only in M1; `naming` is (open) here
  (Open). The command-line form for kernel conformance carries the instance fixtures and hand
  edits of conformance 47 to 49.

## 10. Test vectors

Computed with the `stream` of `numbers.md` 7.2, whose own vectors it reproduces. The machine-readable
copy is `spec/vectors/identity.toml`, written with the conformance runner (M0 step 5); until then
this table is its source. Both the engine and the SDK run it.

In the file, `digest`'s `args` (`numbers.md` 10) are `[class, [part, ...]]`. Each part is one
type of 2.2, written as `{ name = "..." }` (`""` is the empty name), `{ int = n }`,
`{ side = "left" | "right" }`, `{ role = "start" | "end" | "pass" }`, `{ pair = [p, q] }` or
`{ set = [p, ...] }`, where `p` and `q` are parts. A set's members may be given in any order;
`digest` sorts them. The first row is
`["junction", [{ set = [{ pair = [{ name = "lane" }, { role = "start" }] }] }]]`.

| Class | Parts | Digest |
|---|---|---|
| `junction` | set {(`lane`, start)} | `f8361e4571a94ed7` |
| `junction` | set {(`lane`, end)} | `9c42c3494176612f` |
| `junction` | set {(`lane`, pass)} | `f5b1f230d9f66a5b` |
| `junction` | set {(`beck`, pass), (`lane`, pass)} | `82df4d89dc3cb626` |
| `junction` | set {(`#clip`, pass), (`lane`, end)} | `2658ccbb34c5aa87` |
| `edge` | `j82df4d89dc3cb626_0`, `jf8361e4571a94ed7_0`, set {`lane`} | `fb7cc1661c923e9b` |
| `edge` | `j82df4d89dc3cb626_0`, `jf8361e4571a94ed7_0`, set {`beck`, `lane`} | `edaa5ba9984d5e26` |
| `area` | set {(`beck`, left), (`lane`, left), (`lane`, right)} | `43c6f2eb9128ebc7` |
| `cell` | `lane`, left, `j82df4d89dc3cb626_0`, 0 | `5a52c2461d4ce6d7` |
| `cell` | `lane`, left, empty name, 0 | `881980ca85a857fb` |
| `cell` | `lane`, right, `j82df4d89dc3cb626_0`, -3 (encoding check only) | `f8c5ecdf6d28f289` |
| `rest` | `streets#a43c6f2eb9128ebc7_0` | `97d328fbf7300eb3` |

Keys without a digest: lattice index (12, -3) at spacing 7 m is `s7000_12_n3`; at 6 m,
`s6000_12_n3`. The two spacings give disjoint names for every index.

## Diagnostics

Raised from a step's return, a diagnostic's `file` and `line` are those of the step's declaring
table, and `position` is the item's place.

| Code | Severity | Says | Hint |
|---|---|---|---|
| `identity.bad_id` | error | An id breaks `[a-z][a-z0-9_]*` or is longer than 48 bytes. | The repaired id: lowercased, each invalid character replaced by `_`, `x` in front if the first character is not a letter, cut to 48 bytes (`mill.a` to `mill_a`, `9x` to `x9x`). |
| `identity.reserved` | error | An id is `world`, or a mount, set item or kind surface is named `start`, `end` or `ground` (vocabulary 2.1, `rules.md` r53, `surfaces.md` 1). | Another name; the nearest free one. |
| `identity.duplicate_id` | error | Two tables in one namespace share an id; names both, with file and line, across layers too. | Rename one, or merge them in one layer. |
| `identity.unknown_place` | error | A part of a place name names nothing. | The nearest match. |
| `identity.unknown_producer` | error | `by` names nothing. | The nearest producer id. |
| `identity.not_a_producer` | error | `by` names a set, or a maker that places no instances (a network, a cells table, a `where` feature). | Name the producers that place on it. |
| `identity.unknown_maker` | error | `of` names nothing. | The nearest maker id. |
| `identity.on_both` | error | `on` has both `ref` and `of`. | Keep `ref` for an authored thing, `of` for a produced one. |
| `identity.bad_key` | error | A step returned a key that breaks 2.2, or an unknown class; says which rule. | The repaired key, as for `bad_id`, or the class list. |
| `identity.key_collision` | error | One step gave two things one key; names both positions. | Build the key from place, with the SDK's naming calls. |
| `identity.key_in_text` | error | A produced name is written in a key that takes a name. | `{ of, class, near }` at the thing's current position. |
| `identity.of_ambiguous` | error | `of` names a maker with several classes and no `class`. | The classes, with a count of each near `near`. |
| `identity.no_place` | error | A selection's `class` is `field` or `row`, which have no place (2.4). | Read a field by name or `'tag:<tag>'` in a formula; select a feature or instance by place. |
| `identity.near_none` | error | A maker has no candidate of the class asked for; lists every selection left with nothing. | The nearest candidate of any class of that maker and its distance, or that the maker made nothing. |
| `identity.reselected` | warning | A selection resolved to another name than in the baseline; names both and the distance moved. A run diagnostic (`build.md` 11.1). | Move `near` onto the thing meant, or select a feature that does not split (section 3). |
| `identity.pin_removed` | info | What a pin removed: count and names. | Narrow `by` to keep them. |
| `identity.pin_overlap` | warning | A pin overlaps an instance of a producer outside its `by`; names both. | Add that producer to `by`, or move the pin. |
| `identity.pin_in_clear` | warning | A pin stands inside a clear; names both. | Leave it if the pin is what the spot is for; otherwise move the pin. |
| `identity.pin_in_keep_out` | warning | A pin stands where `keep_out` is above 0; names the field. | Move the pin, or accept it here. |
| `identity.pins_overlap` | error | Two pins' footprints overlap; names both. | Move one; the overlap's size is given. |
| `identity.clear_empty` | warning | A clear removed nothing (4.3). | The nearest instance of a `by` producer and its distance; delete the clear if it is no longer wanted. |
| `identity.unknown_lock` | error | `via lock` was given an id that is no lock. | The nearest lock id. |
| `identity.lock_unrecorded` | error | A lock has no record. | `via lock <world> <id>`. |
| `identity.lock_rerecorded` | info | A record was replaced; counts of the difference. | None needed; review the record's diff. |
| `identity.lock_differs` | info | What the producers would now do inside a lock, and live names dropped for a recorded one, as counts. | Re-record with `via lock`, or leave it. |
| `identity.lock_ground_moved` | warning | Recorded items stand on ground that moved; count and largest change. | Re-record with `via lock`, or edit the ground's cause. |
| `identity.lock_asset_missing` | error | A recorded item's asset is no longer bound by the pack. | Bind the asset again, or re-record with `via lock`. |
| `identity.lock_pin_clash` | error | A recorded item overlaps a pin; names both. | Move the pin, or re-record with `via lock`. |
| `identity.no_previous` | info | No baseline to report changes against. | None needed; the next build has one. |

## Conformance

Each case is a world, an action, and what must hold, checked through the command line (R20). Each
is tagged with the milestone that runs it. "Exit 1" means the build stops; every diagnostic named
carries its subject and the file and line of the text at fault.

**Ids and text**

1. (M1) `[feature.Mill]`; build; `identity.bad_id`, subject `Mill`, hint `mill`, exit 1.
2. (M1) `[feature."mill.a"]` and `[feature."9x"]`; build; two `identity.bad_id`, hints `mill_a`
   and `x9x`, exit 1.
3. (M1) `[feature.world]`; build; `identity.reserved`, exit 1. (M3) a set item named `end`; the same.
4. (M1) `[feature.x]` in the world file and in a layer; build; one `identity.duplicate_id` naming
   both files and lines, exit 1.
5. (M1) `on = { ref = "lane.middle" }`; `identity.unknown_place`, hint `lane.end`, exit 1.
6. (M1) a clear with `by = ["tres"]`; `identity.unknown_producer`, hint `trees`, exit 1. (M3) `by =
   ["streets"]`, a network; `identity.not_a_producer`, exit 1.
7. (M1) `of = "tres"`; `identity.unknown_maker`, hint `trees`, exit 1.
8. (M1) `on` with `ref` and `of`; `identity.on_both`, exit 1.
9. (M1) a pin `on = { ref = "trees#s7000_5_3" }` and a scatter `within = "trees#s7000_5_3"`; two
   `identity.key_in_text`, each hint the `{ of, class, near }` form, exit 1.

**Selection**

10. (M1) two pins `on = { of = "stones", near }`, where `stones` has density 0; one
    `identity.near_none` at `stones` listing both pins, exit 1.
11. (M1) a pin `on = { of = "trees", near }`; thin `density` so the selected tree goes; build;
    `identity.reselected` names both trees in the run report, not in `diagnostics.json`, exit 0. The
    same world built in a fresh directory gives the same `diagnostics.json`.
12. (M3) `of = "streets"` with no `class`; `identity.of_ambiguous` listing `junction`, `edge`,
    `area`, exit 1. `class = "field"`; `identity.no_place`, exit 1.
13. (M3) a bench `on = { of = "streets", class = "edge", near }`; move its street far away; the
    bench lands on the next street and `identity.reselected` names both in the run report, exit 0.
14. (M3) a scatter `within = { of = "streets", class = "area", near }`; builds; trees only in that
    area.

**Hand edits**

15. (M1) a clear `on = { of = "trees", near }` with the default `by`; builds with no cycle, exit 0;
    the selected tree is gone.
16. (M1) a pin with `by = ["trees"]` overlapping a bush; `identity.pin_overlap` names both, exit 0.
17. (M1) the same pin with the default `by`; the bush is removed; `identity.pin_removed` names it;
    the change report lists it removed, cause `pin.<id>`.
18. (M1) two overlapping pins; `identity.pins_overlap`, exit 1.
19. (M1) a pin inside a clear; `identity.pin_in_clear`, exit 0. A pin where `keep_out` is 0.5;
    `identity.pin_in_keep_out`, exit 0.
20. (M1) a clear on empty ground; build; `identity.clear_empty` (warning), exit 0. Build again
    unchanged; `identity.clear_empty` again, and `diagnostics.json` is byte-identical to a build of
    the same text in a fresh directory. A clear that removes one tree; thin `density` so the tree
    goes; build; `identity.clear_empty`.
21. (M1) a pin in layer 1 and a scatter in layer 2 with `by` naming it; `documents.md`'s upward
    reference error. A pin in layer 2 speaking to a layer-1 scatter read by a layer-1 lint; the
    lint sees the scatter with the pin's removals applied (R29).
22. (M2) a generate step returning instances; edit a pin speaking to it; the step does not re-run
    (run report), and the held output changes.
23. (M3) a clear `on = { of = "lots", class = "cell", near }` with no radius over the grid of case
    25; the report lists exactly one removed instance, the building in that cell, cause the clear.
24. (M3) a pin inside a cell's building with `by = ["trees"]`: `identity.pin_overlap`. With the
    default `by`: the building is removed and `identity.pin_removed` names it.

**Names under change**

25. (M3) Distant edge: a 4 by 4 street grid, cells, and a building per cell placed by a generate
    step through the SDK; move one vertex of an outer street without changing what crosses what;
    every junction, edge and area name is unchanged; no cell name outside that street's frontage and the
    stretches beside it changes. Network and cells tables: `geometry.md` (open there).
26. (M3) Unrelated change: two regions divided into cells; change cell `width` in one; no name in
    the other changes.
27. (M3) A region outline with three stretches; lengthen one; the cells of the stretches not beside
    it keep their names.
28. (M3) A new self-crossing on a path; its far end junction keeps its name.
29. (M1) Extent grown on all sides; every old scatter name keeps its position, facing and asset;
    only new lattice indices add names.
30. (M1) Seed changed, a scatter with density 1 and nothing to give way to: every name is kept, and
    each entry is moved or swapped only. Ford's own scatter: no name is both removed and added.
31. (M1) Spacing under hand edits: ford's trees, spacing 7 to 6, with one pin and one clear; every
    `s7000_` name removed, every `s6000_` name added, the pin's tree present, nothing inside the
    clear.
32. (M1) Masked pieces: a `where` region in three pieces; change the condition so one piece shrinks;
    the other two keep their names.
33. (M3) Clump and run: drop clump member 0 by `keep_out`; member 1 keeps `_m1`. Cut a run by the
    extent; the remaining stations keep their numbers.
34. (M2) Step keys: a step returns two things with one key; `identity.key_collision`, at the step's
    table, exit 1. A step returns `Lot1`; `identity.bad_key`, hint `lot1`, exit 1.

**Locks** (M3)

35. Declare a lock; build; `identity.lock_unrecorded`, exit 1. `via lock`; exit 0. Build; exit 0.
36. `via lock` with an unknown id; `identity.unknown_lock`, exit 1. `via lock` on a world with an
    error; exit 1, no record written. Record twice after a change; `identity.lock_rerecorded` with
    counts.
37. Thin the scatter under a lock; the record is kept whole and `identity.lock_differs` gives the
    counts. Raise the ground under it; items settle, are reported moved on that build only, and
    `identity.lock_ground_moved` is given on that build and the next.
38. A pin onto a locked tree; `identity.lock_pin_clash`, exit 1; the tree is not removed.
39. Unbind a recorded asset in the pack; `identity.lock_asset_missing`, exit 1.
40. Change the seed under a lock so a recorded name falls outside the region; the build holds that
    name once, at its recorded place; `identity.lock_differs` counts one dropped.

**Report**

41. (M1) Order: the same world with tables shuffled; the same names and the same report.
42. (M1) Null: build into `build/a`, then build the same text with `--against build/a` (which
    disables the null build, `build.md` 13); its `changes.json` has `entries: []` and every summary
    row zero.
43. (M1) First build in a fresh directory; `identity.no_previous`; build again; no such message.
44. (M1) Scale range of a scatter changed; every surviving name is `changed` only, cause
    `scatter.trees.scale`. World seed changed; causes read `world.seed`.
45. (M1) Digest vectors: section 10's table through `via vectors` (`digest`, `numbers.md` 10), bit
    for bit. The SDK reads the same `spec/vectors/identity.toml` in its own tests, which are not
    conformance.
46. (M1) Pack swapped: ford with `lib/packs/primitive.toml`, then a change writing `pack` to a
    second primitive pack whose house has a larger `size`, under the same `base_centre` origin. The
    cottage's entry is `changed` only, carrying `facts` with old and new `bounds`, `size` and
    `footprint`, causes `asset.house_low.size` (the asset id, `assets.md` 7) and `world.pack`, in
    bytewise order (`build.md` 11.2);
    its `ground` footprint feature is `moved`, larger, with the same causes.

**Kernel cases** (M1, Track A's gate)

These run as `via kernel names` (`geometry.md` 7.3), with its inputs the members of `geometry.md`
7.1, not through a world, since a network and cells have no user syntax before M3. The inputs are
the kernel fixtures, fixture instances standing for one producer `homes`, and hand edits written as
in section 4; the output is names, held instances and diagnostics. The hand edits are applied by the same code as in a build
(4.1). They close the plan's Track A items "names do not change when a distant edge moves", "a
clear removes exactly one cell's building" and "a pin inside one is reported"; cases 23 to 25 then
check the same through a world in M3.

47. Distant source: `linework_a`; network; record every junction, edge and area name. Move one
    vertex of one source, not a junction, by 1 m, so that no crossing is added or lost and no
    vertex passes another along any source; network. Every junction, edge and area name is
    unchanged. The same with the sources given in reverse order: the same names.
48. Clear one cell: `cross`'s first cells case (10 cells), with one `homes` instance per cell at the
    cell's representative point and the cell as its footprint. A clear `on` the cell at station 2
    of the `cross_ns` side, with no radius; the held output lacks exactly that cell's instance and
    keeps the other nine, names unchanged. A clear `on` the `rest` region; `identity.clear_empty`.
49. Pin in a cell: the same 10 instances; a pin whose footprint lies inside the station 2 cell's
    instance. With the default `by`: that instance is removed and `identity.pin_removed` names it,
    count 1. With `by` naming another producer: nothing is removed and `identity.pin_overlap` names
    the pin and the instance.

## Changed from the spike

- `[[override]]` is dropped (R3, section 6).
- Scatter names `<scatter>:<cell>` become `<scatter>#s<spacing mm>_<i>_<j>`; `:` and `.` would clash
  with place names and selectors, and the spacing makes a new lattice a new set of names.
- Hand edits speak to every producer through `by`, steps included, not to scatters only.
- Produced features (streets, areas, cells, pieces) get names by place; the spike numbered them in
  the order made.
- A lock is recorded by a command, not by the first build, and reports changed ground.
- The change report names each entry's cause; the spike's gave counts per rule.
- A clear that removes nothing says so; a pin says what it removed.

## Open

- Whether a clear may remove a produced feature: decided with M3's `quay`, if a case asks to remove
  one generated street.
- Naming an area after a feature inside it (a district, a square): decided with M3's `quay` and
  the districts library. M1 names areas by their `sides` only.
- The coarse lattice side for pieces (32 m): decided by M1's masked-region fixtures.
- Edits to things that are not points (a path's vertex, a hand-painted field): decided in M3, with
  the first case that needs one (TODO, "From spiking the four", item 2).
- Whether a `near` selection should refuse candidates beyond a distance: decided by cases 11 and 13
  and the first fresh-agent run of M1.
- How a hero or interactive thing is marked, so it keeps one identity among regenerated clutter,
  and whether produced names are exported for a game to query: decided in `build.md` with the
  export of names (TODO, "Decide how hero and interactive things are marked").
- Whether a baseline is kept per selection of changes, rather than one slot per directory (the
  spike's 12.8 trap): decided in `build.md`. Until then `select:<change>` names the difference.
