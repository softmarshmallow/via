# Rules: placement and the three core rules

This chapter says how one asset or one set is placed (an instance), and how the three core rules
(`set`, `run`, `scatter`) turn a few lines of text into many instances. It also fixes what the
build checks about placed things, and how library forms such as kit runs lower to these.

Words come from `vocabulary.md`. Where a key's meaning is another chapter's, this chapter points
there and does not restate it. Section 1.1 lists the words this chapter adds there.

Milestones: each section and each conformance case carries the milestone that needs it. Instances,
placement, grounding and scatter are M1 (`ford`); `clump` is M2 (`glade`); sets and runs are M3
(vocabulary section 5); the library forms are M3 and M5.

## 1. Terms

| Term | Meaning |
|---|---|
| placement | The keys that say where a thing goes: `at` or `on`, and `face`. One grammar for features, instances, sets, rules and hand edits (`vocabulary.md`, Placement). |
| hand instance | An instance written as `[instance.<id>]`, and every item of a set it places. The author placed it one by one. Pins are hand edits (`identity.md`), not hand instances, and are checked there. |
| producer | A `run`, a `scatter` or a step: something that makes instances the author did not write one by one (`vocabulary.md`, section 1). |
| produced instance | An instance a producer made, and every item of a set it places. Named `<producer>#<key>`, an item `<producer>#<key>.<item>` (`identity.md`). |
| scattered instance | A produced instance whose producer is a scatter. |
| footprint | The asset's outline in its own frame (`assets.md`), scaled and turned by the instance's frame. An asset without one has a point footprint: its origin. |
| solidity | An asset fact: `solid`, `passable` (M5) or `none` (`assets.md` 2.4). `solid` and `passable` claim their footprint, alike in every rule of this chapter; `none` claims nothing. Only a walk reads `passable` (`surfaces.md` 4.4). |
| instance frame | Origin at the placed point, turned by the resolved bearing, scaled by `scale` (`numbers.md`, frames). A set's items and an asset's mounts are measured in it. |

A set is a rule in the doctrine's sense: placed, it lowers to plain instances. It is not a producer.
Its items take the class of the thing that places it: items of a hand instance are hand instances;
items of a produced instance are produced by the same producer. So `by` never names a set (that is
`identity.not_a_producer`); it names the producer that places it.

### 1.1 Words to add to `vocabulary.md`

These are added there with this chapter, as part of its acceptance (vocabulary: a chapter may not
use a key that page does not list):

- `jitter` on a scatter (M1); the `clump` keys `members`, `radius`, `shrink`, `around` (M2).
- `ends` on a run, taking `"half"` or `"both"`; `scale`, `tint`, `ground`, `margin` on a run;
  `set`, `scale`, `tint` on a set item (M3).
- `on` takes `{ ref, t, offset }` or `{ of, class, near, t, offset }` (`identity.md` 3); a run
  takes `span` in place of `t`.
- The pair rule: on a producer that draws (a scatter), a key that takes a number also takes
  `[lo, hi]`, meaning "drawn from this range per instance". It covers `scale`, `tint` and
  `clump.members`. Elsewhere a pair is `place.range`.
- The place-name part `ground` (`<instance>.ground`, section 2.1), reserved beside `start` and
  `end`: no mount or set item may use it.
- Ownership: `ground` and `margin` are this chapter's, not `surfaces`'.
- The `on` table row for a thing with no length: its remedy is "place a set on it" (section 3.2),
  not "write `face` and `at` in its frame".

The asset facts this chapter reads are `assets.md`'s: `footprint`, `solidity` and `max_relief`.
M1 reads them as declared by the primitive pack; measured facts arrive in M2 (plan). There is no
asset fact for a distance kept around a footprint (`assets.md` 2.4).

## 2. An instance (M1)

`[instance.<id>]` places one asset, or one set, by hand.

| Key | Takes | Required | Default | Meaning |
|---|---|---|---|---|
| `asset` | name | one of `asset`, `set` | | An asset id the pack binds (`assets.md`). |
| `set` | name | one of `asset`, `set` | | A set to place as one unit (section 4, M3). |
| `at` | point | one of `at`, `on` | | A coordinate in the world frame. Plan only. |
| `on` | `{ ref, t, offset }` or `{ of, class, near, t, offset }` | one of `at`, `on` | | A place on a named or selected thing (section 3). |
| `face` | number or word | no | see 3.3 | Which way the instance's front looks. |
| `scale` | number | no | `1` | Uniform scale of the asset, or of the set's frame. Greater than 0. |
| `tint` | number | no | `1` | Multiplier on the asset's colour. 0 or greater. |
| `ground` | name | no | none | A kind. The footprint, grown by `margin`, is also a region feature of that kind (2.1). |
| `margin` | length | no | `0` | How far `ground` grows the footprint. 0 or greater. Only with `ground`. A length is read exactly to millimetres (`numbers.md` 3.1). |

Exactly one of `asset` and `set`, and exactly one of `at` and `on`. An instance has an asset, never
a kind: it lends its footprint to a kind through `ground`. `authority` is not an instance key
(`doc.unknown_key`): a hand instance is already the author's firm decision, and its footprint
feature takes its kind's authority (2.1). To make the ground under it firmer, change the kind, or
write a separate region feature with its own `authority`.

A height is never written. The instance stands on the surface beneath its origin (section 6).

### 2.1 `ground`: an instance lending its footprint

`ground = "pad"` (any kind a library declares) makes the instance's footprint also a region feature
of that kind, so the kind's effects shape fields under the instance: a pad flattens and clears the
ground under a house. The feature, its **footprint feature**:

- is the footprint, in the world frame, grown outward by `margin` (`geometry.md`, offsetting);
- for an instance of a set, is one region per item footprint, unioned; every piece is kept;
- takes its kind's `authority` (`authority.md`);
- is named `<instance>.ground`, or `<producer>#<key>.ground` for a produced instance. It is a place:
  it is in every `kind:<k>` selection, `within` may name it (so dress lowers, section 10), a `for_each`
  item carries the name. The change report and `features.json` key it by its instance's name, in
  the table `footprint` (`identity.md` 5, `build.md` 9.1). Its diagnostic subject is the key
  path `instance.<id>.ground`, located at the `ground` key's line; for a produced one, the
  producer's address, with `<producer>#<key>.ground` in `occurrences` (`diagnostics.md` 1.1, 5);
- moves when the instance moves, so a moved cottage moves its pad and everything the pad shaped;
  and follows the asset's footprint, so a pack swap that grows the footprint grows the pad, which
  settles and is reported (R30).

A produced instance whose footprint feature would move a field past its kind's `max_change` gives
way: it is not placed, with `authority.gave_way` (warning); a written instance past it stays
`authority.max_change` (error). `authority.md` 2.2 gives the rounds. So a producer offers places
and need not predict the ground.

`ground` on an asset with a point footprint is `place.no_footprint`. `ground` naming no kind is the
unknown-kind error of `authority.md`. `margin` without `ground` is `place.margin_alone`.

## 3. Placement (M1)

### 3.1 The grammar

Every placed thing in this chapter takes the one grammar:

- `at = [x, y]`: a coordinate in the frame of the table it is in. On an instance, run or pin that
  is the world frame; on a set item, the set's frame (section 4).
- `on = { ref, t, offset }`: a place on an authored thing. `ref` is a place name (`identity.md`).
- `on = { of, class, near, t, offset }`: a place on a produced thing, selected by a point near it
  (`identity.md` 3). This is the only way to stand on a generated street, cell or instance (R3,
  R13). `on` takes `ref` or `of`, never both (`identity.on_both`).
- `t` and `offset` are resolved by `geometry.md` 2. A run takes `span` in place of `t` (section 5).
  `span` on an instance, or `t` on a run, is `place.on_form`. `t` or `offset` on a thing with no
  length is `geometry.md`'s `geom.no_length`.
- `face`: a bearing in degrees, or a word from `geometry.md`'s list (`along`, `toward`, `away`).

### 3.2 What `on` may name

| `on` names | The placed origin | `t`, `offset` |
|---|---|---|
| a path feature, or a produced edge or path feature | The point at `t` of arc length, moved `offset` to the left of travel. | As `geometry.md` 2. |
| a region feature, or a produced area, cell, rest or piece | The point at `t` of the outline, moved `offset` inward. | As `geometry.md` 2. |
| a point feature, a path end (`<path>.start`, `.end`), an instance (authored or selected), a mount, a set item | That point. | Not accepted: no length (`geom.no_length`, `geometry.md` 2.6). To stand beside it, place a set on it: the set's items take `at` in its frame (section 4). |
| an import's place (`a.west_gate`) | As the thing it names inside the import (M4). | As above. |

A produced name written in `ref` (`ref = "trees#s7000_8_18"`) is `identity.key_in_text`, whose hint
gives the `{ of, class, near }` form. Produced names are for the change report, diagnostics and
selection results only (R3; `identity.md` 3).

**Mounts.** Every placed instance exposes its asset's mounts as places under its name: `cottage.door`,
and on a set item `cottage_lamps.l.<mount>` (`identity.md` for the grammar). A mount of a produced
instance has a name (`fence#r3.<mount>`) that text may not write; how text reaches the mount of a
selected instance is (open), section 15.

### 3.3 Facing

The resolved bearing is the instance frame's turn, in integer millidegrees (`numbers.md` 2).

| Placement | `face` written as a number | Default |
|---|---|---|
| `at` | The bearing. | `0` (north). |
| `on` a path, region, or produced edge, path, area, cell, rest or piece | The bearing. | `"along"` at `t` (`geometry.md` 2). |
| `on` a path end | The bearing. | The direction of travel at that end: `along` at `t = 0` for `start`, `t = 1` for `end`. |
| `on` a thing with a facing: an instance (authored or selected), a mount, a set item, a point feature with `face` | Added to the host's facing, so the placed thing turns with its host. | `0`: the host's own facing. `"along"` gives the same. |
| `on` a point feature without `face` | The bearing. | `0`. `"along"` gives the same: its facing is 0 (`geometry.md` 2.6). |

On a thing with a facing, `face` is relative, as it is for a set item (section 4): one meaning,
"turn from the frame it is placed in" (principle 9). The words are `geometry.md` 2.6's. On a thing
with no length, `face = "along"` is the host's own facing, so the placed thing turns with it;
`toward` and `away` need a line and an offset, and are `geom.face_side`.

### 3.4 How `on` resolves through the graph

Every reference in this chapter is an edge in the one graph (`graph.md`). The placed thing waits
for what it names, and moves when that moves.

**Plan and height are two nodes.** An instance's placement is a plan node (origin, facing, frame)
and a height node (its `z`). The footprint feature hangs from the plan node; the height node reads
the final surface. So a padded house writes the surface through its pad and stands on the result,
with no cycle: plan, then pad, then surface, then height. These are `graph.md`'s `instance` (or
`held`) and `height` nodes.

| Reference | Edge from | Edge to |
|---|---|---|
| `on.ref` | the named thing's resolved shape, or the plan node of the instance or set item it names | the placed thing's plan |
| `on = { of, class, near }` | the producer's held output (`identity.md` 4.1); for a network or a cells table, its node (`geometry.md` 8.3) | the placed thing's plan |
| `set` on an instance or run | the set table | each item's plan |
| `ground` | the instance's plan and its asset's footprint | the footprint feature, and so every field its kind writes |
| (height) | the final surface | each instance's height |
| `within` on a scatter | the named region or selection | the scatter |
| `clump.around` | the named producer's held output | the scatter |
| giving way, non-scattered producer | the plans of hand instances in its layer or below, whose solidity is not `none`, that do not stand on its own output | its output before hand edits |
| giving way, scatter | the same, plus the held output of every non-scattered producer in its layer or below, solidity not `none`; the `surface` and `keep_out` fields; every field its `density` reads | the scatter's output before hand edits |
| any producer | the pins, clears and locks whose `by` names it (`identity.md` 4.1) | its held output |

A cycle is `graph.md`'s cycle error, naming every edge in it. Two that the rules above make:

- An instance `on` a feature whose shape reads the instance's own footprint feature, as a `where`
  reading `inside('cottage.ground')` under the cottage. A feature that only reads a field the
  footprint's kind writes is no cycle: those entries are effects, outside the head view
  (`authority.md` 2.3).
- A step in a scatter's layer, or below it, that reads the scatter's output and places instances
  whose solidity is not `none`: the scatter gives way to the step, and the step reads the scatter.
  The hint names three ways out: make the step's assets `solidity = "none"`, move the step to a
  layer above the scatter, or run it at `lint`, which reads the finished build (R29).

**Giving way reads down only.** A producer gives way to things in its own layer or below, as every
reference does (doctrine, layer: "may refer downward only"; `identity.md` 4.1 for hand edits). A
hand instance or a produced instance in a higher layer that overlaps a lower layer's produced
instance is reported (`place.crowded`, 8.1), never silently resolved; the hint is a clear or a pin
in the higher layer with `by` naming the lower producer. Whether R21's reasoning should extend to
placement, so lower derived things give way to firmer things above, is (open), section 15.

## 4. Set (M3)

`[set.<id>]` is a named group in a local frame, placed as a unit wherever an instance, a run or a
pin names it. A set is never placed by itself.

| Key | Takes | Meaning |
|---|---|---|
| `[set.<id>.item.<name>]` | table | One item. `<name>` is an id; not `start`, `end` or `ground` (`identity.reserved`). |
| `asset` or `set` | name | On an item: what it places. A set inside a set is a nested frame (`numbers.md`). |
| `at` | point | On an item: its origin in the set's frame. Default `[0, 0]`. |
| `face` | number | On an item: a bearing added to the set's bearing. Default 0. |
| `scale`, `tint` | number | On an item: multiplied by the placing instance's. Default 1. Greater than 0, and 0 or greater (`place.range`). |

Rules:

- The placing thing gives the set its frame: origin, bearing and `scale` (the items' positions are
  scaled too). Its `tint` multiplies every item's.
- An item is named `<placing instance>.<name>`; a nested item `<placing instance>.<name>.<name>`.
  An item is a place, so other things may stand `on` it.
- Each item stands on the surface beneath its own origin, not the set's (section 6).
- Items of one placement are not checked for overlap against each other: the author arranged them
  as one unit. This replaces the spike's `group`.
- A set with no items is `set.empty`. A set that contains itself, at any depth, is the graph's
  cycle error. A set that nothing places is `set.unused` (info).
- A set draws nothing, so it takes no `seed` (section 5).

## 5. Run (M3)

`[run.<id>]` repeats one asset or one set along a stretch of a path or of a region's outline.

| Key | Takes | Required | Default | Meaning |
|---|---|---|---|---|
| `asset` or `set` | name | one | | What each member places. |
| `on` | `{ ref, span, offset }` or `{ of, class, near, span, offset }` | yes | `span = [0, 1]`, `offset = 0` | The stretch. What it names must have a length (3.2). |
| `count` | integer | one of `count`, `spacing` | | How many members. A literal, never a formula. |
| `spacing` | length | one of `count`, `spacing` | | Wanted distance between neighbours along the line. Greater than 0. |
| `ends` | word | no | `"half"` | `"half"`: a half step free at each end. `"both"`: a member at each end. |
| `face` | number or word | no | `"along"` | As 3.3, per member. |
| `scale`, `tint`, `ground`, `margin` | as on an instance | no | as on an instance | Copied to every member. A pair is `place.range`: a run draws nothing. |

A run draws nothing at random, so it takes no `seed`: `seed` on a run or a set is
`doc.unknown_key` (`numbers.md` 7.1).

### 5.1 Where members stand

All lengths are integer millimetres (R24, `numbers.md`).

1. `Lmm = round_half_even(L)`, once per line, where `L` is `numbers.md`'s arc length (a double).
2. `a = round_half_even(span[0] * Lmm)` and `b = round_half_even(span[1] * Lmm)`, the product in
   doubles through `via-num`. `L' = b - a`; zero is `run.no_length`.
3. `n` and each member's arc length `s_k` (`k` from 0 to `n - 1`), in integers:

| | `ends = "half"` | `ends = "both"` |
|---|---|---|
| `n` from `count` | `count`, at least 1 | `count`, at least 2 |
| `n` from `spacing` (`sp`, mm) | `max(1, round(L' / sp))` | `max(2, round(L' / sp) + 1)` |
| `s_k` | `a + floor((2k + 1) * L' / (2n))` | `a + floor(k * L' / (n - 1))` |

`round` is half away from zero, done in integers: `round(L' / sp) = floor((2L' + sp) / (2sp))`.
With `spacing`, the step is stretched or shrunk so the members fill the stretch evenly; the run
never leaves a remainder. A library that needs one exact step (a kit of fixed modules) checks the
fit itself (section 10).

On a region with `span = [0, 1]` the outline is closed: `"both"` places `n = count` (or
`max(1, round(L' / sp))`) members at `s_k = a + floor(k * L' / n)`, so the first point is not used
twice.

**The point at arc length `s` mm.** Members are placed by arc length, never through a `t`. With the
segments' cumulative lengths `c_0 = 0 < c_1 < ... < c_m` (doubles, as `numbers.md` sums them), the
segment holding `s` is the `k` with `c_k <= s < c_(k+1)`, or the last segment when `s >= c_m`.
The point is `p_k + (p_(k+1) - p_k) * min(1, (s - c_k) / (c_(k+1) - c_k))`, moved `offset` along
that segment's left normal, rounded to the lattice once (ties to even). A member on a vertex is
held by the outgoing segment, as `geometry.md` 2 holds `t`. This is `numbers.md` 5.5's point at
arc length, with `s` in whole millimetres.

Members are named `<run>#r<k>` (`identity.md` 2.3). A member whose origin falls outside the extent
is not made, with one `run.outside` warning for the run.

A member gives way as any produced instance (3.4): one whose solid footprint, or a solid item of the
set it places, overlaps a solid hand instance of its layer or below is not made, with its items,
counted in one `place.gave_way` for the run. Neighbours of one run are never tested. A `span` value
outside 0 to 1 is `geom.t_range`; an `ends` word other than `"half"` or `"both"` is `doc.type`.
Ours.

## 6. Grounding (M1)

A placement is plan only. Height comes from the surface beneath the origin, by `surfaces.md`'s four
questions (R25), in the height node (3.4). This chapter fixes only:

- Every instance, set item, run member and scattered instance grounds at its own origin.
- Text never writes a height for any of them. A thing that should stand higher stands on a surface
  that is higher (a deck, a floor, a table top), which `surfaces.md` defines.
- Instances stand upright. Tilting to the slope is (open), section 15.

## 7. Scatter (M1; `clump` M2)

`[scatter.<id>]` turns a density into instances on a lattice fixed to its frame.

| Key | Takes | Required | Default | Meaning |
|---|---|---|---|---|
| `assets` | list | yes | | Choices: an asset id, `@<taxon path>`, or `{ id, weight }` (7.4). |
| `spacing` | length | yes | | Side of a lattice cell. Greater than 0. One candidate per lattice cell. |
| `jitter` | number | no | `0.8` | Share of the lattice cell the candidate may move across, 0 to 1. 0 puts it at the lattice cell's centre. Read exactly (7.1). |
| `density` | formula | no | `1` | The chance a candidate is kept, at its point. 0 to 1. |
| `within` | name or `{ of, class, near }` | no | the extent | The region the scatter is confined to: a region feature, a footprint feature (2.1), or a selected area, cell, rest or piece (`identity.md` 3). |
| `scale`, `tint` | number or `[lo, hi]` | no | `1` | A number fixes it; a pair draws each instance's value (the pair rule, 1.1). `lo <= hi`. |
| `clump` | inline table | no | none | Placement by cause (7.3, M2). |
| `seed` | string | no | the scatter's id | The name its draws come from (`numbers.md` 7). Write one only to change it. |

A scatter takes no `face`: facing is drawn (7.1). See section 15.

### 7.1 The lattice and the keep test

The lattice is anchored at the origin of the frame the scatter is written in (`identity.md` 2.3),
never at the extent's corner or the region's: lattice cell `(i, j)` covers `[i*s, (i+1)*s)` by
`[j*s, (j+1)*s)` in millimetres, with `s` the spacing. Moving the extent or the region therefore
moves no candidate; it only adds or removes lattice cells. This anchoring is ours.

**Which lattice cells.** With `[x0, x1] x [y0, y1]` the bounding box of the extent intersected with that of
`within`, `i` runs from `floor(x0 / s)` to `floor(x1 / s)` inclusive, and `j` likewise, by integer
floor division. Lattice cells are visited in any order; nothing depends on it.

**Jitter is read exactly**, as a geometry number is (`numbers.md` 3.1), to an integer `Jm` in
millionths: `0.8` is `800000`. More than six decimals is `num.too_precise`. Weights (7.4) are read
the same way (`numbers.md` 3.3).

For each lattice cell:

1. **Candidate.** `w = floor(s * Jm / 10^6)` mm. Per axis,
   `x = i*s + floor((s - w) / 2) + ((r * w) >> 64)`, with `r` the draw `jitter_x` (likewise `y` with
   `jitter_y`), all in 128-bit integers, every division a floor. A jitter of 0.8 keeps the candidate
   between 10% and 90% of the lattice cell, as the spike did.
2. **Confine.** Kept only if its origin lies in the extent and in `within`. Both are closed: a point
   on the edge is inside, by `geometry.md`'s exact point-in-ring test.
3. **Keep.** Kept when `unit(u) < p`, with `u` the draw `keep` and
   `p = density * (1 - keep_out) * near`, evaluated at the candidate's point: `density`
   as `fields.md` defines a formula at a point, `keep_out` the maximum of every field with that role
   (0 if none), `near` from `clump.around` (1 without it). The scatter applies `keep_out` itself; a
   formula does not repeat it (`vocabulary.md`, roles). A density outside 0 to 1 is `formula.range`
   (error), as `formulas.md` 6.1 says: the range is the key's, and it is never clamped (principle
   3). A density formula that reads a `keep_out`
   field is `scatter.keep_out_twice` (info).
4. **Choose.** Asset (7.4), facing, scale and tint, each from its own draw.
5. **Clump members**, if `clump` has `members` (7.3, M2).
6. **Give way** (7.5).
7. **Hand edits**, in `identity.md` 4.1's order: locks, then clears, then pins. What they remove is
   reported through `identity.md`, not counted in `place.gave_way`.

**Draws.** Each purpose has its own stream, `stream(world seed, ["rule", <seed name>, <purpose>])`
(`numbers.md` 7.2). A parent's draw is `draw(stream, [i, j])`;
a clump member's is `draw(stream, [i, j, m])`, `m` its member index. `u` is the draw; products are
in 128-bit integers; doubles go through `via-num`.

| Value | Purpose | Formula |
|---|---|---|
| position | `jitter_x`, `jitter_y` | Step 1. |
| keep | `keep` | `unit(u) < p`. |
| asset | `asset` | 7.4. |
| facing | `face` | `(u * 360000) >> 64` millidegrees. |
| scale, tint from `[lo, hi]` | `scale`, `tint` | `lo + unit(u) * (hi - lo)`, in doubles, in that order. A fixed value draws nothing. |
| member count from `[lo, hi]` | `clump_count` | `lo + ((u * (hi - lo + 1)) >> 64)`. |
| member position | `member_r`, `member_a` | `rr = radius_mm * sqrt(unit(u_r))`; bearing `m = (u_a * 360000) >> 64` millidegrees; with `numbers.md` 2's sine and cosine `(sn, cs)` of `m`, the member is the parent's candidate plus `(round_half_even(rr * sn), round_half_even(rr * cs))`. |
| member keep | `member_keep` | `unit(u) < 1 - keep_out` at the member. |
| member facing, scale, tint | `member_face`, `member_scale`, `member_tint` | As the parent's. Scale is then multiplied by `1 - shrink`. |

Draws never move: no edit changes a draw except an edit to the world `seed`, the seed name, or
`spacing` (which makes a new lattice and renames every instance, `identity.md` 2.3). So:

- lowering `density` only removes instances; it never moves or adds one;
- changing `assets`, or a pack swap (R30), swaps assets in place, and changing `scale` or `tint`
  rescales or recolours in place, except where giving way now decides differently (a larger footprint, a lower
  `max_relief`); those are counted in `place.gave_way`;
- changing `spacing` or `seed` re-lays everything, which is expected. Hand edits hold by place
  (R3), not by name.

### 7.2 Densities by cause

R5: placement by cause is the default to teach. Most causes are written in `density`, as a formula
over fields and distances (`formulas.md`): a band along a bank, stones by slope, nothing on the
lane. `keep_out` carries every "not here" that kinds write. Clumps and growth beside another scatter
are the two causes a formula cannot say. R7, as amended by R28, lets a formula read around a point
only through a shipped function with a declared halo and cap (`formulas.md` 8.6), and those read
fields and features; a clump reads other candidates' draws and a producer's instances.

R7 puts such reads in a step. `clump` is the exception, kept in core because R5 makes placement by
cause the default to teach, so it must work with no code, and because it reads only draws and a
named producer's output within `radius`, both of which the graph already holds (principle 10: one
key, no new mechanism). It stays in core: `glade` writes its clumps as a step as a demonstration,
not a gate.

### 7.3 `clump` (M2)

`clump` takes one of two forms. The keys and their behaviour are ours, after the spike's `clump`
and `around()` (`glade`, `spike/FINDINGS.md`, "Placement by cause").

| Form | Keys | What it does |
|---|---|---|
| members | `members` (integer or `[lo, hi]` integers, 0 or more), `radius` (length, > 0), `shrink` (0 to < 1, default 0) | Each kept candidate is a parent. It gets `members` members (drawn, inclusive), each at a point in the disc of `radius` around the parent's candidate (7.1, draws), with the parent's asset, its own facing, its own tint, and its own scale draw times `1 - shrink`. A member is kept when its origin is in the extent and `within` (closed) and its `member_keep` draw passes; density is not applied again. Members then give way as 7.5 says. |
| around | `around` (a producer id), `radius` (length, > 0) | `near = max(0, 1 - d / radius)`, where `d` is the distance from the candidate to the nearest origin among the named producer's held output. Growth beside what is already there. |

- Members are drawn from the parent's candidate after the keep test (step 3), whatever happens to
  the parent later: a parent that gives way or is cleared does not take its members with it.
- A member's index `m` runs from 0 to its count minus 1, fixed before any keep or giving way. Its
  name is `<scatter>#<parent key>_m<m>`, for example `trees#s7000_8_18_m2` (`identity.md` 2.3). A
  member that gives way leaves its siblings' names unchanged.
- Writing keys of both forms in one `clump`, a missing `radius`, a `radius` not above 0, `members`
  below 0, `shrink` outside `[0, 1)`, or `around` naming something that is not a producer, is
  `scatter.clump`. `radius` keeps its one meaning: beyond it, nothing.

**Around another scatter.** `around` names any producer in the scatter's layer or below
(`documents.md` 5.1): usually another scatter, also a run or a step. It reads that producer's held
output (`identity.md` 4.1), so a cleared tree takes its bushes' chance with it and a thinned wood
thins its undergrowth (r43). A pin whose written `by` names the producer counts as one of its
instances: a pinned tree is a tree bushes grow beside. A pin with `by` left to its default counts
for none, since it would make every pin in the layer count for every producer.

**Two causes on one thing** are two scatters, never one `clump`: undergrowth that grows beside the
trees and gathers in tufts is `bushes` with `clump = { around = "trees", radius = 5 }`, and
`tufts` with `clump = { around = "bushes", radius = 1.6 }` (`glade`). Each radius keeps its one
meaning, and each cause is a line a reviewer reads. A chain reads down the graph, so a loop is
`graph.cycle` (r44).

### 7.4 Assets by taxon

`assets` is a value, replaced whole (`vocabulary.md`, 2.5). Each entry is:

- an asset id, weight 1;
- `@<taxon path>`: every asset of the pack filed at or under that path (`assets.md`), weight 1 in
  all;
- `{ id, weight }`, where `id` is either of the above and `weight`, read exactly in millionths
  (7.1), is greater than 0 (`scatter.range`).

**The choice.** Entries are taken in written order, each with its weight `W_e` in millionths;
`T` is their sum. `target = (u * T) >> 64` picks the first entry whose running sum exceeds
`target`. Within an `@` entry with `k` matches sorted bytewise by id, the match is index
`((target - start_e) * k) / W_e`, floored, where `start_e` is the running sum before the entry.
An asset named by two entries is chosen through either; its weights are not merged.

So an `@` entry's weight is shared equally among its matches, and adding an asset to the pack under
one path changes only which asset that entry's instances take: an instance that drew another entry
keeps its asset. Shared weights and this choice are ours. A path with no match is `assets.md`'s
unknown-asset error.

### 7.5 Giving way

A scatter is derived (principle 3). After choosing, a candidate or clump member is dropped when:

- its footprint overlaps the footprint of a hand instance, or of a non-scattered produced instance,
  in the scatter's layer or below, and neither asset's solidity is `none` (`assets.md` 2.4; as 8.1 tests, and as 3.4's edges read only solid hand instances).
  Overlap is 8.1's: interiors meet, so touching edges pass. v0 keeps no distance around a footprint
  (`assets.md` 2.4); a spacing fact, if one is wanted, takes a new word through `assets.md` and
  `vocabulary.md`, never `keep_out`, which is a field role (principle 9). Hand instances that stand on this scatter's output (3.4) are excluded; or
- `relief(surface, footprint)` (`surfaces.md` 2.3), `high - low`, exceeds its asset's `max_relief`.

A clump member is also dropped when it is solid and its footprint overlaps the footprint of its
parent (when the parent was kept) or of an earlier kept member of the same parent, by member index
`m` (7.3), in 8.1's sense. So a clump never grows through itself, the order is fixed by the draws
and every member keeps its name. Other scattered instances are not tested against each other
(section 15). Each producer reports what gave way in one `place.gave_way` info: counts by cause (overlap, relief) and their bounding box. A
producer from which nothing gave way raises no `place.gave_way`.

**Which producer places first.** Hand instances first. Then runs and steps, each giving way to hand
instances as above (8.1). Then scatters, giving way to both. Runs and steps do not give way to each
other, and scatters do not give way to each other: there is no order between them, and overlaps
between runs and steps are reported (8.1). This settles the question `assets.md` leaves here.

## 8. What is checked (M1)

All checks run after fields are resolved and every producer has placed, on the held instances
(hand edits applied). Overlap and point-in-polygon tests are exact on the millimetre lattice
(`geometry.md`); no tolerance is tuned (R24). Pins are checked by `identity.md` (4.2) only, never
here, so no case has two codes.

### 8.1 Overlap

Two footprints overlap when their interiors meet; touching edges do not overlap, so kit pieces
that abut pass. Only instances whose solidity is not `none` are tested.

**Levels (M5, decided).** Two things are tested only when they stand on one surface (the terrain,
or one kind surface: `surfaces.md` 4.1, a set's items by their placement's, a run's members by its
`on`) and their height spans above it meet. A thing's span is `-sink` to `-sink` plus its
`bounds` height, both scaled (`assets.md` 2.4); spans meet when their insides do, so a piece
standing on another's top passes, and a span of no height meets one it touches. This holds for
`place.overlap`, `place.crowded` and every giving way (7.5, runs and steps). Reason: the test runs
on plans (`surfaces.md` 4.3, step 1), before any field settles, so it may read only declarations
and asset facts; two storeys are two surfaces, so a thing on one never clashes with a thing on the
other (station: ten shell pieces of the ground floor gave way to furniture upstairs), and a roof
lifted onto its walls by its `sink` clears the furniture beneath it. Whether a tall piece on one
storey reaches through the floor above is a library lint over `surfaces_at`, not this test. Ours.
The terrain and a kind surface count as one level where the surface stands on the ground: one
whose kind writes `cut`, or whose bound meets the thing's footprint on the terrain (R40, owner's
ruling, M5 slice 2 fix): a ground floor and the yard outside it are one floor to stand furniture
on, so a chest on the floor and a barrel on the ground reaching onto it clash as on one surface; their
spans are still compared from their own datums (TODO, the level test, Later). Cases: `surfaces.md`
Conformance, `storey_overlap`, `ground_floor_clash`.

| Between | Result |
|---|---|
| two hand instances | `place.overlap`, error: two firm decisions clash (principle 3). |
| a hand instance and a produced instance of a producer in its layer or below | The produced one gave way before it was placed (7.5 for scatters; the same overlap test for runs and steps), counted in `place.gave_way`. A hand end piece beside its run therefore replaces the bay it overlaps. |
| a hand instance and the instance it stands on (`on` it), or any two whose chains of `on` reach one root (M5, `assets.md` 12.1) | Not tested: it was placed there on purpose; parts on parts of one root are one thing, as one set placement's items are. |
| neighbours `k` and `k + 1` of one run, and the last and first on a closed outline | Not tested. Kit bays measure a little over their pitch, and on a curve abutting bays overlap on the inner side even at the exact pitch: the spike's college gave 22 false overlaps this way (`spike/FINDINGS.md`, college). A step's instances are hand instances of its document, so this exemption does not reach a kit run a library lays: the library keeps its neighbours apart itself (`lib/town/kit.py` rule 7, M5 `market`: 86 false `place.overlap` before it, by rounding to the lattice and on bends). |
| two items of one set placement | Not tested (section 4). |
| any other two non-scattered produced instances, of one producer or two | `place.crowded`, warning, naming both. |
| a lower layer's produced instance and anything in a higher layer | `place.crowded`, warning, naming both (3.4: giving way reads down only). |
| two scattered instances | Not tested (section 15). |

### 8.2 Relief under an asset

For every placed instance whose asset declares `max_relief`, relief is `surfaces.md` 2.3's
`relief(surface, footprint)` on the final surface, `high - low`. This chapter owns the comparison
and its code; `surfaces.md` 4.1 and `assets.md` point here.

- A hand instance, run member, step instance or set item over the limit is `place.relief`,
  warning, with the measured relief, the limit and the position.
- A scattered candidate over the limit gives way (7.5).

A pad that levels the ground under its own instance counts, because relief is taken on the final
surface. `relief`'s `outside` share is `surfaces.md`'s (`surface.off_bound`).

### 8.3 `keep_out`

A hand instance, run member, step instance or set item stands in `keep_out` when the field is above
0 at any post inside its footprint or at its origin. That is `place.keep_out`, warning, naming the
field and the position.

The test reads `keep_out` in one view without the entries (the kind's effects) of any footprint feature (`fields.md` 6.2, `graph.md` view `build-nofootprint`): one extra
evaluation per world (R21's accepted cost), not one per instance. So no pad reports its own house,
and two houses whose pads touch or overlap do not report each other. A thing placed on another
house's pad, beyond its footprint, is therefore not reported; the overlap test covers the house
itself. Scatters do not report: they multiply by `1 - keep_out` (7.1).

### 8.4 Extent

A hand instance or set item whose origin is outside the extent is `place.outside` (error, not
placed): nothing stands outside the extent, and the extent is the terrain's bound. `surfaces.md`
4.1 and `build.md` 5 point here; `surface.outside` is `surfaces.md`'s, for a plan point outside a
kind surface's bound. A produced instance outside the extent is not made and raises no
`place.outside` (`run.outside`
for a run; a scatter's lattice stops at the extent).

The extent is closed: an origin on its edge or at its corner is inside. The test is on the origin
in integer millimetres (`numbers.md`), `x0 <= x <= x1` and `y0 <= y <= y1`, the same closed rule as
scatter's confine step (7.1) and the clump's member test (7.3). One rule serves every placer: hand
instances, set items, run members and scattered candidates. Only the origin is tested; a footprint
that reaches past the edge is placed whole, as `geometry.md` 2.8 keeps shapes whole. Our own
choice: `ford`'s lanes end on the edge (x = 0 and x = 256), and a thing placed at a lane's end must
stand. Case r17 holds it.

## 9. Hand edits

Pins, clears and locks are intent at a place, never against a produced name (R3). `identity.md`
defines them, their order (lock, then clear, then pin) and their two stages. This chapter fixes only
that every producer, rules and steps alike, has its hand edits applied after it gives way and
before anything reads its instances (`clump.around`, a step, the checks).

On generated content (R13): a network or a cells table (`geometry.md` 8) is a maker, not a
producer, so a hand edit never acts on it and `by` never names it. A hand edit acts on what
producers place there: a clear `on` a selected cell removes the instances whose origin lies in it,
from every producer its `by` reaches, a step's included; a pin inside a cell removes what its
footprint overlaps and says so (`identity.md` 4.2, 4.3). The cells themselves stay, since other
things are built on them (`identity.md` 4.1).

## 10. Library forms that lower to rules (M3, M5)

The spike had six rules; the core has three (doctrine section 6). The others are library forms.

**The lowering rule.** A library form is a feature of a library kind (or a library step's
`params`) that a library step, run at `when = "generate"` and usually `for_each = "kind:<k>"`,
turns into plain features, instances, sets, runs and scatters. The step reaches only what a user's
step reaches (R19b). For every form:

- the lowered text is legal plain text: written into the world in place of the form, it builds the
  same instances and features, with the same assets, frames and footprint features, differing at
  most in their names;
- what the form checks that plain text does not (a kit's fit) is a lint the library declares, with
  its own codes (`step.<id>.<name>`).

Each form, in a line; their cases are the libraries':

- **Kit run** (building library, M3): a `run` with `count`, `ends = "half"`, and one pad feature
  under the row. The fit (`count * module` against the length) is the library's lint. The town
  library's kit run (M5, `lib/town/kit.py`) lowers to plain instances: end pieces centred on a
  path's ends and either side of each gap `keep_out` opens, a corner piece at each turn, bays at
  `params.pitch` and fillers for the rest, what is left spread over the joints or taken by a
  gap's edge, each piece level at its origin with `sink` as its footing; a stretch that cannot
  close is the step's error naming the lengths that would. With `caps = "inside"` (a row of
  houses, `market`; the step's param was `ends` until the M5 close, renamed so `ends` is only the
  core run's key) an end piece stands wholly inside its stretch and a free end takes what is
  left; neighbours whose chords turn, or that would meet once rounded to the lattice, stand apart.
- **Dress** (M5): a `scatter` with `within` naming the region to dress, often a footprint feature
  (`within = "cottage.ground"`). A table top that is a surface is `surfaces.md`'s.
- **Flank** (M5): a set of two items placed `on` the host's mount, turning with it (3.3).

## 11. Worked example

A lane 100 m long running east, a cottage on a pad, lamps at its door (M3), a fence along the far
side (M3), and trees in clumps (M2) with bushes growing near them.

```toml
[feature.lane]
kind = "lane"
path = [[0, 120], [100, 120]]

[instance.cottage]
asset  = "house_low"
on     = { ref = "lane", t = 0.6, offset = 12 }
face   = "toward"
ground = "pad"
margin = 1.5

[set.door_lamps.item.l]
asset = "lamp_post"
at    = [-1.8, 0.9]

[set.door_lamps.item.r]
asset = "lamp_post"
at    = [1.8, 0.9]

[instance.cottage_lamps]
set = "door_lamps"
on  = { ref = "cottage.door" }

[run.fence]
asset   = "fence_post"
on      = { ref = "lane", span = [0.1, 0.4], offset = -4 }
spacing = 3
ends    = "both"

[scatter.trees]
assets  = ["@plant/tree"]
spacing = 7
density = 0.6
scale   = [0.8, 1.2]
clump   = { members = [2, 4], radius = 3, shrink = 0.25 }

[scatter.bushes]
assets  = ["@plant/bush", { id = "fern_low", weight = 2 }]
spacing = 2
density = 0.5
clump   = { around = "trees", radius = 6 }
```

What follows, with world `seed = 0`:

- The cottage's origin is `[60, 132]` (60% of 100 m, 12 m to the left of travel, which is north).
  `toward` turns it to face the lane: bearing 180. Its footprint grown by 1.5 m is the `pad`
  feature `cottage.ground`. It stands on the surface after its own pad (3.4).
- The lamps are `cottage_lamps.l` and `cottage_lamps.r`, at the mount `door` of `house_low`. Their
  default face is 0 relative to the mount (3.3), so they turn with the cottage. Each grounds at its
  own origin.
- The fence: `Lmm = 100000`, `a = 10000`, `b = 40000`, `L' = 30000`,
  `n = max(2, floor((60000 + 3000) / 6000) + 1) = 11`, posts at `10000 + floor(k * 30000 / 10)` mm:
  every 3 m from x = 10 to x = 40, 4 m south of the lane, named `fence#r0` to `fence#r10`.
- Trees: the cottage's origin is in lattice cell `(8, 18)` (`floor(60000 / 7000)`, `floor(132000 / 7000)`)
  of the lattice anchored at the world frame's origin, the frame `trees` is written in. `w = 5600`,
  so the candidate's x lies in `[56700, 62300)` mm. The draws give `(60683, 130893)`: keep
  `unit = 0.357854873215093 < 0.6`, so kept; it then lies under the cottage and gives way
  (`place.gave_way`). Its four members (`members` drew 4), `trees#s7000_8_18_m0` to `_m3`, are
  drawn all the same (r41); all four also fall on the cottage's 8 m by 6 m footprint and give way.
  Where the pad's kind writes `keep_out`, fewer trees are kept.
- Bushes stand only within 6 m of a held tree, most likely right beside one.
- Moving the lane moves the cottage, its pad, the lamps and the fence; the trees' lattice does not
  move, so only trees the pad newly covers or uncovers change, and the change report says so.

## 12. Diagnostics

Codes this chapter introduces. Unknown names, cycles and bad `on` tables are `identity.md`'s,
`graph.md`'s and `geometry.md`'s; unknown assets are `asset.unknown`; an unknown kind is
`authority.md`'s; pins are `identity.md`'s;
`authority` or `seed` where not allowed is `doc.unknown_key`.

| Code | Severity | Says | Hint |
|---|---|---|---|
| `place.no_position` | error | An instance or pin has neither `at` nor `on`; a run has no `on`. | Write `at` or `on`. |
| `place.asset_or_set` | error | A placement writes both `asset` and `set`, or neither. | Keep one of `asset`, `set`. |
| `place.on_form` | error | `span` in an instance's `on`, or `t` in a run's `on`. (`t` or `offset` on a thing with no length is `geom.no_length`.) | Write `t` on an instance, `span` on a run. |
| `place.outside` | error | A hand instance or set item whose origin is outside the extent; not placed (8.4). | Move it inside the extent, or widen the extent. |
| `place.range` | error | On an instance, run or set item: `scale` not above 0, `tint` below 0, `margin` below 0, or a `[lo, hi]` pair (only a scatter draws). | The key and its allowed range. |
| `place.margin_alone` | error | `margin` without `ground`. | Add `ground = "<kind>"`, or remove `margin`. |
| `place.no_footprint` | error | `ground` on an asset with no footprint; only a model measures a point footprint, `assets.md` 2.4 (M2). | Declare the asset's `footprint`, or remove `ground`. |
| `place.overlap` | error | Two hand instances' footprints overlap; names both. | Move one; the overlap's size is given. |
| `place.crowded` | warning | Two produced instances overlap (8.1), or one overlaps a thing in a higher layer; names both. | Change one producer's `on` or `span`, or add a clear or pin in the higher layer with `by` naming the lower producer. |
| `place.relief` | warning | Relief under an instance exceeds its asset's `max_relief`; gives both numbers and the position. | Move it, give it `ground` with a kind that levels, or choose an asset with a larger `max_relief`. |
| `place.keep_out` | warning | An instance stands where `keep_out` is above 0; names the field and the position. | Move it, or change the kind whose effect writes `keep_out` there. |
| `place.gave_way` | info | A producer's candidates dropped by overlap and by relief, as counts, with their box. | Widen the space or change the hand instances it gives way to; nothing if intended. |
| `set.unknown` | error | `set` names no set (M3). | The nearest set id. |
| `set.empty` | error | A set has no items (M3). | Add `[set.<id>.item.<name>]`, or remove the set. |
| `set.unused` | info | A set nothing places (M3). | Place it with an instance or run, or remove it. |
| `run.count_or_spacing` | error | A run writes both `count` and `spacing`, or neither (M3). | Keep one. |
| `run.count` | error | `count` below 1, or below 2 with `ends = "both"` on an open stretch (M3). | The least allowed `count`. |
| `run.no_length` | error | The run's `on` names a thing with no length, or its stretch is zero millimetres (M3). | Name a path or region, or widen `span`. |
| `run.outside` | warning | Members fell outside the extent and were not made; gives the count (M3). | Narrow `span` to the part inside the extent. |
| `scatter.range` | error | A `[lo, hi]` with `lo > hi`, a `scale` not above 0, a `tint` below 0, a `jitter` outside 0 to 1, a `spacing` not above 0, or a `weight` not above 0. | The key and its allowed range. |
| `scatter.within` | error | `within` names a path or a point. | Name a region feature, a footprint feature, or select an area, cell, rest or piece. |
| `scatter.clump` | error | `clump` breaks 7.3: both forms, no `radius`, a value out of range, or `around` naming a non-producer (M2). | The key to remove or change. |
| `scatter.keep_out_twice` | info | `density` reads a `keep_out` field, which the scatter already applies. | Remove the `keep_out` factor from `density`. |
| `scatter.empty` | warning | A scatter placed nothing; names the stage that removed the last candidate. | The key behind that stage (`density`, `within`, `keep_out`, a hand edit). |

## 13. Conformance

Each case runs through the command line (R20). The runner also checks that a clean build equals
the incremental one. Unless a case says otherwise: world `seed = 0`, extent `[0, 0, 256, 256]`,
flat terrain at height 20, the primitive pack with `house_low` an 8 m (x) by 6 m (y) solid box,
no `max_relief`. Fixtures are `spec/cases/rules/<case>/`.

**M1: instances, placement, grounding, checks**

| Case | Input | Action | Must hold |
|---|---|---|---|
| r01 at_point | `at = [10.5, 20.25]` | build | Origin `(10500, 20250)` mm, `z = 20.0`, facing 0. |
| r02 on_path | Lane `[[0, 0], [100, 0]]`; `on = { ref = "lane", t = 0.25, offset = 2 }` | build; then move the lane's end to `[100, 10]` | `(25000, 2000)`, facing 90000; then moved, cause `feature.lane`. |
| r03 pad | `ground = "pad"`, `margin = 1.5`, at `[50, 50]` | build | `cottage.ground` is an 11 m by 9 m region in `kind:pad`. |
| r04 pad_on_slope | r03 on terrain `20 + 0.1 * x`, a pad kind that levels | build | No cycle; `z` equals the final surface at the origin, not 25.0. |
| r05 faces | Instances `at`; on a point feature without `face`; on `lane.end`; on an instance with `face = 180` | build | Facings 0, 0, 90000, 180000. |
| r06 face_side | `face = "toward"` on a point feature; then `face = "along"` on an instance facing 90 | build | `geom.face_side`, exit 1; then placed, facing 90 (`geometry.md` 2.6). |
| r07 on_form | `on = { ref = "well", t = 0.5 }` on a point feature; then `span = [0, 1]` in an instance's `on` a path | build | `geom.no_length`, exit 1; then `place.on_form`, exit 1. |
| r08 key_in_text | `on = { ref = "trees#s7000_8_18" }` | build | `identity.key_in_text`; hint holds `of = "trees"`. |
| r09 overlap | Two hand instances 4 m apart | build | `place.overlap` naming both, exit 1. |
| r10 cycle | `cottage` with `ground = "pad"`, `on` a `where` region reading `inside('cottage.ground')` | build | `graph.cycle` naming the edges; exit 1. |
| r11 relief | `max_relief = 0.6`, terrain `20 + 0.1 * x`, face 0 | build | `place.relief`, relief 0.8, limit 0.6, a position. |
| r12 own_pad | A pad kind writing `keep_out = 1` | build | No `place.keep_out`. |
| r13 pads_side | Two padded houses 9 m apart, pads overlapping | build | No `place.keep_out`, no `place.overlap`. |
| r14 in_keep_out | A region feature writing `keep_out = 1`; a hand instance in it | build | `place.keep_out` naming the field. |
| r15 outside | `at = [300, 10]` | build | `place.outside`, not placed, exit 1. |
| r16 bad_keys | Each of: no position; neither `asset` nor `set` (`set` itself is M3, `doc.not_yet` in M1); `ground` on a point asset; `margin` alone; `scale = 0`; `authority = "anchored"` on `cottage` | build | `place.no_position`, `place.asset_or_set`, `place.no_footprint`, `place.margin_alone`, `place.range`, `doc.unknown_key` with subject `instance.cottage.authority`; each exit 1. |
| r17 edge | Lanes `lane_west` `[[0, 120], [100, 120]]` and `lane_east` `[[156, 140], [256, 140]]`; instances `on = { ref = "lane_west", t = 0 }`, `on = { ref = "lane_east", t = 1 }`, `at = [256, 256]`, `at = [256.001, 10]` | build | First three placed at `(0, 120000)`, `(256000, 140000)`, `(256000, 256000)`, no `place.outside`; the fourth `place.outside`, not placed, exit 1. |

**M1: scatter**

| Case | Input | Action | Must hold |
|---|---|---|---|
| r20 draws | `trees`, spacing 7, jitter 0.8, density 0.6, `scale = [0.8, 1.2]` | build | Lattice cell `(8, 18)`: `jitter_x` `0xB61EE2389F06E35A`, `jitter_y` `0xBFB242685F562515`, origin `(60683, 130893)`; keep `0x5B9C60812C4D1041`, unit 0.357854873215093, kept; facing 338068; scale 1.027638703368402; name `trees#s7000_8_18`. |
| r21 centre | r20 with `jitter = 0` | build | Lattice cell `(8, 18)` at `(59500, 129500)`. |
| r22 negative | r20 on extent `[-20, -20, 20, 20]` | build | Lattice cell `(-1, -1)` at `(-4105, -6285)`, named `trees#s7000_n1_n1`. |
| r23 threads | r20 | build at 1 and 8 threads | Same bytes. |
| r24 thin | r20 | `density` 0.6 to 0.3 | Only removals. |
| r25 swap | r20, flat, no other instances | change `assets` | Only swaps; no instance moved or removed. |
| r26 rescale | r20, flat, no other instances | `scale = [1, 1.5]` | No instance moved, added or removed. |
| r27 within | Scatter `within` a region | grow the region | Only additions. |
| r28 extent | r20 | shift the extent by one resolution step | No instance kept in both builds moved. |
| r29 seed | r20 | rename the scatter | As `numbers.md`'s `seed_rename`. |
| r30 give_way | r20 with `cottage` at `[60, 132]` | build | No tree footprint overlaps the cottage's; `place.gave_way` counts `trees#s7000_8_18`. |
| r31 footprint_give_way | A solid hand instance inside a scatter of solid assets; one candidate's footprint touches it edge to edge | build | No scattered footprint overlaps it; the touching one is kept; the rest that overlapped are counted in `place.gave_way`. |
| r32 field_keep_out | A region writing `keep_out = 1` | build | No candidate kept inside it. |
| r33 relief_give | Trees with `max_relief = 0.3` on terrain `20 + 0.1 * x` | build | No tree placed whose footprint spans more than 3 m in x; counted as relief. |
| r34 taxon | `["@plant/tree", "rock_a"]`; add an asset under `plant/tree` | rebuild | Every instance that drew `rock_a` keeps it and is unmoved. |
| r35 density_range | `density = 60` | build | `formula.range` (`formulas.md` 6.1), hint `clamp`, exit 1. |
| r36 keep_out_twice | `density = "= 0.6 * (1 - keep_out)"` | build | `scatter.keep_out_twice`. |
| r37 scatter_bad | Each of: `scale = [1.2, 0.8]`; `within` a path; `density = 0` | build | `scatter.range` exit 1; `scatter.within` exit 1; `scatter.empty` naming `density`. |
| r38 ford_spacing | `ford`, a clear and a pin with `by = ["trees"]` | spacing 7 to 6 | Every tree name changes (`s7000` to `s6000`); the clear's place holds no tree; the pin stands; both hand edits unchanged. |

**M2: steps** (M1's only step is one lint, as a plain command)

| Case | Input | Action | Must hold |
|---|---|---|---|
| r39 step_same_layer | A generate step in `trees`' layer that reads `trees` and places solid benches | build | Graph cycle error; hint names `solidity = "none"`, a higher layer, or `lint`. |
| r40 step_above | The same step one layer above `trees`, a bench over a tree | build | No cycle; the tree stays; `place.crowded` naming both. |

**M2: clump**

| Case | Input | Action | Must hold |
|---|---|---|---|
| r41 members | r20 with `clump = { members = [2, 4], radius = 3, shrink = 0.25 }` | build | Parent `(8, 18)`: `clump_count` gives 4; members at `(61918, 129124)`, `(62906, 132568)`, `(63278, 132342)`, `(63502, 131558)` before keep; `member_keep` units 0.38020872430545594, 0.3787660718847049, 0.677276160385895, 0.859956877106416. |
| r42 member_gives | r41 with a hand instance over member `_m1` only | build | `_m1` gone; `_m0`, `_m2`, `_m3` keep their names and positions. |
| r43 around | `bushes` around `trees`, radius 6 | thin `trees` | Only removals of bushes, each within 6 m of a removed tree; graph edge recorded. |
| r44 around_cycle | Two scatters each `around` the other | build | Graph cycle error, exit 1. |
| r45 clump_bad | Each of: both forms; no `radius`; `shrink = 1`; `around` a feature | build | `scatter.clump`, exit 1. |
| r46 around_pin | r43 with `trees` thinned to nothing; a pin with `by = ["trees"]` at `[60, 60]`; then the same pin with no `by` | build each | Bushes only within 6 m of `[60, 60]`; then no bush. |
| r47 around_chain | r43 plus `tufts` around `bushes`, radius 1.6 | thin `trees` | Only removals, of bushes and of tufts; each removed tuft within 1.6 m of a removed bush. |
| r48 around_up | `around` naming a scatter of a higher layer | build | `doc.upward_ref` naming both layers, exit 1. |
| r49 around_step | A generate step returning instances in a layer below; a scatter `around` the step | build; then the step's output moved | Candidates beside the step's instances only; then the scatter changed only near the moved ones. |
| r50 member_gives_to_parent | r31 without the crate, `clump = { members = 2, radius = 0.2 }` | build | Every member stands inside its parent's trunk and gives way (`place.gave_way`); the parent of cell (8, 18) stands at (59.5, 129.5) and no member of it is left within 1 m. |

**M3: sets and runs**

| Case | Input | Action | Must hold |
|---|---|---|---|
| r62 set_mount | Set of two items `on` a mount | turn the host 90 | Both items turn with it; names `<instance>.<item>`. |
| r63 set_scale | Set placed with `scale = 2` | build | Item positions and sizes doubled. |
| r64 set_cycle | Set containing itself | build | Graph cycle error naming the set, exit 1. |
| r65 set_bad | Unknown set; unused set; empty set; item named `ground` | build | `set.unknown`, `set.unused`, `set.empty`, `identity.reserved`. |
| r66 run_half | `count = 4`, `ends = "half"`, 100 m path | build | Members at 12500, 37500, 62500, 87500 mm. |
| r67 run_both | `spacing = 3`, `ends = "both"`, 30 m stretch | build | 11 members, at 10000 to 40000 mm every 3000. |
| r68 run_vertex | Path `[[0, 0], [30, 0], [30, 30]]`, `count = 3`, `ends = "both"`, `offset = 1` | build | Members at `(0, 1000)` facing 90000, `(29000, 0)` facing 0, `(29000, 30000)` facing 0. |
| r69 run_closed | Region `[[0, 0], [10, 0], [10, 10], [0, 10]]`, `count = 4`, `ends = "both"` | build | Members at the four corners, none twice. |
| r70 run_bad | Each of: `count` and `spacing`; `count = 1` with `"both"`; `on` a point; a run past the extent; `t` in `on`; `seed = "x"` | build | `run.count_or_spacing`, `run.count`, `run.no_length`, `run.outside`, `place.on_form`, `doc.unknown_key`. |
| r71 kit_curve | 12 bays at exact pitch, abutting, on a curve | build | No `place.crowded`. |
| r72 end_piece | A hand end piece overlapping the run's last bay | build | That bay gave way, `place.gave_way` 1; no error. |
| r73 clear_member | A clear over `fence#r3` | build | That member removed; neighbours unmoved. |
| r74 member_keys | A run of four members on pads; then a hand instance added, built again in the same store | build; add; build | No `diag.internal`; the warm build equals a clean one (each member's nodes are keyed by its name, `graph.md` 3.1). |

## 14. Changed from the spike

- Every table keyed by id (R1). Instance ids are checked for uniqueness (`identity.md`).
- One placement grammar. `atop`, `beside`, `against`, `slide`, `yaw`, `turn`, `mount` (height) and
  `in` are gone. A height is never written; surfaces decide it (R25).
- `kind` and `margin` on an instance became `ground` and `margin`; the footprint feature now keeps
  every piece, takes its kind's authority, and has a name.
- A set passes scale and tint to its items, may hold a set, and its items are places. `group` is
  gone: items of one set placement are one unit.
- A run takes `spacing` and `ends` beside `count`, and `span` for `from` and `to`. Members are
  placed by integer arithmetic on arc length. Neighbours in one run are not tested for overlap.
- `kitrun`, `dress` and `flank` left the core; each lowers to plain text that is legal on its own.
- The scatter lattice is anchored at its frame's origin. Each draw has its own stream and an exact
  integer formula. Tint defaults to 1: no variation unless written.
- A producer gives way by solid footprint overlap to every hand instance, and
  a scatter also to every non-scattered instance, in its layer or below, not by a fixed 1 m to
  explicit ones; a scatter also gives way to relief. What gave way is counted, never dropped
  silently.
- An asset's `avoid` list is replaced by fields with the `keep_out` role. The spike's fixed 1 m
  distance is gone; no asset fact replaces it (`assets.md` 2.4).
- `around('<id>', r)` in a formula became `clump.around`: a graph edge, not file order.
- A hand instance outside the extent is an error, not dropped-and-still-a-host.

## 15. Open

| Question | What decides it |
|---|---|
| Whether a scatter may take `face` (fixed, or along a feature). | A case that needs it; none in M1. |
| Whether scattered instances of different parents or different scatters are tested against each other (a clump's members are, 7.5, decided by M2's measures: 342 interpenetrating footprints in `basin`, most within clumps), for example a scatter giving way to the held output of scatters in lower layers; and whether any producer keeps a distance beyond footprint overlap (a new asset fact through `assets.md`, never `keep_out`). | `glade` and `basin` at eye height: logs through trunks, rocks inside trunks. |
| Whether lower derived things give way to firmer things in higher layers (R21 extended to placement). Today they do not, and the overlap is `place.crowded`. Queued for the doctrine. | `townland`'s layers in M3: how many `place.crowded` a natural layering gives. |
| How text reaches a mount of a selected produced instance. | A case in M3 (a lamp on a fence post of a run). |
| Whether instances tilt to the slope. | `surfaces.md`, with the four questions. |
| How dress keeps items inside its host's edge (the spike's `margin`). | `station`'s dressing in M5. |
| Whether `ground` on a set unions its items or takes their hull. | The town kit in M3. |
