# Fields

A field is a named value at every post of the world grid: a height, a material mix, a mask, or
anything a project names. This chapter says how a field is declared, how the core and libraries
find it by role or tag, how its value is built from a base and entries, how it is read between posts,
and what reaches a loader.

Words are from `spec/vocabulary.md`. Posts, post order, windows, halos, `d` and the maths module are
`numbers.md` (sections 4, 5.5, 6, 8); the order of a field's entries and the views a step reads are
`authority.md` (section 4); formulas, their marker and their field reads are `formulas.md`; node
keys and the store are `graph.md`; the array file is `build.md` 3.2.

**Milestones.** Sections 1 to 3, 4 for an array file, 5, 6.1, 6.3, 6.4 and 7.1 are M1. A PNG base
(4.1), the `cubic` filter, step views (6.2) and what a step returns (section 7) are M2 (`relief`,
`massif`, `basin`): M1's only step is a lint, which returns no rows. Their cases are in the M2
block of the conformance table, so the M1 runner does not owe them.

## 1. What a field is

- A field is declared by `[field.<name>]`. Its name is an id in the world's one namespace
  (`identity.md`). The core never reads a field by its name; it reads by `role` (section 2).
  Formulas read a field by name or by `'tag:<tag>'` through `f`, `slope` and `under`
  (`formulas.md` 7).
- A field is stored at posts (1.1) and read between them by 1.2.
- A field never holds a non-finite number (R26). A formula that yields one is an error in
  `formulas.md`; a file sample that is one is `field.file_nonfinite`.
- Every field is exportable. Which fields a loader takes is the loader's option (`build.md`), not
  world text.

| `type` | A post holds | Takes `base` / `file` | Row keys |
|---|---|---|---|
| `scalar` (default) | One 32-bit float (1.1). | Yes, one of them. | `value` (required), `mask`, `order` |
| `weights` | One 32-bit float per material, each 0 to 1, summing to at most 1 (5.2). | No. | `paint` (required), `mask`, `order` |
| `volume` | Forward pointer only (section 8). Rejected in M1 with `doc.not_yet`. | | |

Another `type` is `field.type_unknown`, with the nearest type as hint. The spike's
`type = "material"` gets the hint `type = "weights"` with `role = "material"`.

Keys of a field: `role`, `type`, `unit`, `taxon`, `tags`, `base`, `file`, `channel`, `range`,
`extent`, `outside`, `filter`, `let`, and its rows `[field.<name>.then.<id>]`. Any other key, on
the field or on a row, is `doc.unknown_key` with the nearest word as its hint (`documents.md`).

- `taxon` files the field in one place, as on assets. The core gives it no meaning; the build's
  index lists it for loaders and tools (`build.md` 6).
- `[field.<name>.let]` names sub-expressions visible in the field's own `base` and its own `then`
  rows, and nowhere else (`formulas.md` 5, 6.3, 6.4).

### 1.1 Precision

This settles `numbers.md`'s open item "stored field precision", at the size the hello-step probe
uses (a 6.5 MB field of 32-bit floats). The probe confirms the size and time; if it fails, the
array file's `descr` (`build.md` 3.2) lets storage change without a new layout (`build.md` 6.1).

- Every formula, row and filter is evaluated in doubles through `via-num` (`numbers.md` 8).
- `v` between rows is a double. A field's value at a post is narrowed to a 32-bit float once,
  after its last entry in the view being resolved, round to nearest, ties to even (`numbers.md` 8,
  rule 9). A weights field narrows each weight the same way.
- Every reader of a resolved field (`f`, `slope`, `under`, 1.2, a step, export) sees the narrowed
  value widened to a double, which is exact.
- A finite double beyond the 32-bit float's finite range becomes infinite on narrowing. That is
  `num.not_finite_result`, naming the field and the post.

### 1.2 Reading a field between posts

A field is read at a point `(x, y)` in integer millimetres (R24). A point beyond the extent has no
value; the reader that asks for one reports the error its own chapter names.

With `r` the resolution in millimetres and `nx` posts along `x`:

1. `a = x - x0`, an integer. The cell is `i = min(floor(a / r), nx - 2)`, in integer arithmetic.
   A point on an edge two cells share therefore uses the cell to its east (or north). Only a point
   on the last column (or row) is clamped, and uses the cell to its west (or south). The value is
   the same either way, since the interpolant is continuous; the gradient is not, and `sample`
   (`surfaces.md` 2.2) takes its cell by this same rule.
2. The fraction is `fx = (a - i * r) as double / r as double`: one correctly rounded division,
   0 to 1, exactly 1 only on the last column. Likewise `j` and `fy`.
3. With `p00` the post `(i, j)`, `p10` its east neighbour, `p01` its north one and `p11` the
   north-east one, in doubles and in this order, with no fused multiply-add:
   - `s = (1 - fx) * p00 + fx * p10`
   - `n = (1 - fx) * p01 + fx * p11`
   - value `= (1 - fy) * s + fy * n`

Each line is exact when its fraction is 0 or 1, so a point on a post reads that post's value bit
for bit. The mapping and order are ours; bilinear interpolation is the textbook rule.

**A formula at a point** (a scatter's `density` at a candidate, `rules.md` 7.2) is evaluated at the
point itself: `x` and `y` are its coordinates (`numbers.md` 4, millimetres divided by 1000), and
each field it reads through `f` gives that field's value at the point by the rule above. It is
never the interpolation of the formula's own values at posts. A function whose halo is in posts
(`slope`) is read at a point as the interpolation of its values at the four posts around it. Ours.

## 2. Roles and tags

A role is what the core uses a field for. A tag is what a library or a project uses it for. The
core reads roles only; it never gives a tag a meaning.

### 2.1 The three roles

| Role | Type | `unit` | How the core reads it | How many |
|---|---|---|---|---|
| `surface` | `scalar` | `m` | The ground under every `at` and `on`; terrain export (`surfaces.md`). | Exactly one unbounded in M1, required once anything asks for ground (below). A second is `surface.two_terrains` (`surfaces.md`). |
| `material` | `weights` | `ratio` | Material weights handed to the loader as the engine's terrain layers. Every `paint` names a pack `[material.<name>]` (`assets.md` 2.6). | At most one. Optional: a world without one hands over no material. |
| `keep_out` | `scalar` | `ratio` | A scatter multiplies its density by `1 - keep_out` (`rules.md` 7.2). A hand placement, run member, step instance or set item where it is above 0 is `place.keep_out`, warning (`rules.md` 8.3); a pin there is a warning naming both (`vocabulary.md`, hand edits). | Any number. Read as one (below). |

- **Units on roles.** A role field may leave `unit` out. An absent unit is then the role's unit,
  everywhere: in the build's index, in `field.file_coarse`, and for every other reader. Any other
  unit is `field.role_unit`.
- **Units on weights.** A `weights` field's unit is `ratio`, written or not, stored as `ratio`.
  Another is `field.weights_unit`.
- A field with no `role` is legal with any type. The core then reads it only where a formula, a
  selector or a step names it.
- Any other role is `field.role_unknown`; a role on a field of the wrong type is `field.role_type`.
- Two `material` fields are `field.role_twice`. Why one: an engine has one set of terrain layers.
  A second mix (vegetation classes, say) is a `weights` field with no role, handed over as data.
  So no further `[material.*]` keys are owed for several material fields; `assets.md` 2.6's open
  item closes here.
- **What asks for ground.** An instance from any producer (hand placement, set, run, scatter,
  step), and a step or loader that asks for `role:surface`. A world with any of these and no
  `surface` field is `field.role_missing`, naming the first that asked; `diagnostics.json` is still
  written. A world of features alone needs no surface.
- **Reading `keep_out`.** At each post the core takes the maximum over every `keep_out` field (0 if
  none), in the view being read. A value outside 0 to 1 is clamped there, with one
  `field.role_range` warning per field naming its first such post. Between posts the core reads
  that per-post maximum by 1.2: the maximum is taken first, then interpolated.

### 2.2 What libraries and projects name: tags

Water level, wetness, snow line, a district's density: these are library or project words
(principle 5). They are not roles. A library names the tag it reads in its own documentation, and
a world puts that tag on its field:

```toml
[field.lake_level]
tags = ["water_level"]
```

Water level is therefore `tag:water_level` by the water library's convention, not a core role.
A plain mask a rule or step reads by name needs neither a role nor a tag.

### 2.3 How a field is found

| Who | Finds a field by | Where defined |
|---|---|---|
| A kind effect | `field = "role:<role>"` or `"tag:<tag>"`, never a name. It writes every field that matches. | `authority.md` 2 |
| A formula, including an effect's `value` and `mask` | By name, or `'tag:<tag>'` matching exactly one field, through `f`, `slope`, `under`. Never by role. | `formulas.md` 7 |
| A step | The same selectors through the SDK, or a name the world hands it in `params`. | `protocol.md` |
| A loader | Role for `surface` and `material`; any field by name or tag as its own option. The build's index lists every field with its `role`, `type`, `unit`, `taxon` and `tags`. | `build.md` |

So a library's effect reads other fields by tag, not by the world's names: a water library's mask
writes `f('tag:water_level')`. The library and a user's step use the same selectors (R19b).

- **Selector checked at the feature's rank.** An effect's selector is a reference, checked at the
  rank of each feature of its kind (`documents.md` 5.1). A match on a field declared in a higher
  layer than the feature is `doc.upward_ref`.
- **Type mismatch.** Where one selector matches fields of both types, the entry on a field whose
  type its keys do not fit (`value` on a weights field, `paint` on a scalar one) is
  `field.row_key`, naming the effect and the field (`authority.md` 2). It is never skipped.
- **No match.** A selector that matches no field is `authority.no_field` (info), owned by
  `authority.md` 2. Lesson from TODO: a parent's field that nothing read was silent.

### 2.4 Units

`unit` is the unit of the value and nothing more. The list is closed:

| `unit` | Meaning |
|---|---|
| `m` | Metres. |
| `deg` | Degrees. |
| `ratio` | A number without dimension (a weight, a fraction, a density). |

A field with no role may leave `unit` out, and it stays absent. Another value is
`field.unit_unknown`, nearest unit as hint. Anchors are always compared, whatever the unit
(vocabulary section 6).

## 3. The base

The base is the value at every post before any row. A `scalar` field has exactly one of:

- `base`, a formula evaluated at each post. Default 0.
- `file`, a raster read and sampled to the posts (section 4).

Both is `field.base_and_file`. A `weights` field has neither: it starts with every weight 0 (no
material) and is built by its rows. `base` or `file` on it is `field.weights_base`. A picture that
should paint a material is read as a `scalar` field and used as a row's `mask`.

## 4. A base from a file

A field's `file` is a path relative to the document that names it, and a declared input of the
graph (principle 7, `graph.md`). Whether principle 6 ("a world never names a file") admits it is
(open): `vocabulary.md` reads the principle as forbidding asset files only, and the doctrine does
not yet say so. A doctrine ruling on principles 6 and 7 decides it.

### 4.1 Formats

| Format | Depths | Channels | Read as | Milestone |
|---|---|---|---|---|
| Array file, `.npy` (`build.md` 3.2) | 32-bit float, `descr = "<f4"` | 2-D, `shape = (ny, nx)` | Stored values, widened. | M1 |
| PNG | 8-bit, 16-bit | grey, grey + alpha, RGB, RGBA | Stored integer levels, mapped by `range`. | M2 |

The array file is the format the build writes and a step returns, so a step's output and an
author's float raster are one format.

- **Array file.** The `.npy` array file of `build.md` 3.2. Its header's `shape` gives rows and
  columns, and it holds no extent. Where it lies is the field's `extent` key alone. A `shape` that is
  not 2-D, or a `descr` other than `<f4`, is `field.file_format`. Its first row is south
  (`build.md` 3.2).
- **PNG.** Its top row is north. Adam7 interlaced pictures are read. 16-bit levels are read as
  stored. Gamma, colour profiles and a `tRNS` chunk are not applied: levels are read as stored, and
  `field.file_profile` (info) names what was ignored.
- **Refused.** Any other format, a palette PNG, a 1-, 2- or 4-bit PNG, or a raster with either
  dimension under 2 samples is `field.file_format`. A missing or unreadable file is
  `field.file_missing`, naming the resolved path.
- **Size.** A raster is checked from its header before its samples are read. More samples than the
  post-count limit of `numbers.md` is `field.file_too_large`. The limit is (open) until the M0
  probe sets it (`numbers.md`, Open).

### 4.2 Keys

| Key | Applies to | Default | Meaning |
|---|---|---|---|
| `channel` | PNG | `grey` for grey and grey + alpha; required for RGB and RGBA | `grey`, `r`, `g`, `b` or `a`. The one channel read. |
| `range` | PNG | `[0, 1]` | `[lo, hi]`: the values level 0 and the format's top level (255 or 65535) stand for, whatever the picture's own lowest and highest levels. |
| `extent` | both | the world's `extent` | `[x0, y0, x1, y1]`, lattice values in the world frame: where the raster's corner samples lie. |
| `outside` | both | none; required with `file` | The value at posts beyond the raster's `extent`. |
| `filter` | both | `linear` | `nearest`, `linear`, or `cubic` (M2). |

- `file` without `outside` is `field.file_outside`.
- `channel`, `range`, `extent`, `outside` or `filter` without `file`, `range` or `channel` on a
  array file, a channel the picture lacks (`grey` on an RGB picture, `a` without alpha), and an
  unknown `channel` or `filter` value are `field.file_key`. The spike's `channel = "luma"` gets the
  hint to name one channel.
- A level `L` stands for `(1 - t) * lo + t * hi`, with `t = L / top` (one division, in doubles):
  exact at both ends. `lo > hi` is legal and inverts the picture (dark is high). `lo == hi` is
  `field.file_range`: a constant is a `base`.
- A raster `extent` follows `numbers.md` 4's rule for an extent (`num.extent_empty`). One that
  lies wholly outside the world's is `field.file_offworld` (warning): every post takes `outside`.

### 4.3 Sampling

Samples of a raster are posts of its own grid: its `W` columns run from `x0` to `x1` of its
`extent`, its `H` rows from `y0` to `y1`, corners included. This corner alignment is ours.

For a post at `X` millimetres, inside the raster's extent (edges included), with `D = x1 - x0`:

1. `N = (X - x0) * (W - 1)`, an exact integer (128-bit).
2. **`linear`.** Cell `c = min(floor(N / D), W - 2)` in integer arithmetic; fraction
   `(N - c * D) as double / D as double`. Likewise along `y`. The value is 1.2's three lines over
   the four samples around, each first mapped by `range`.
3. **`nearest`.** Sample `floor((2 * N + D) / (2 * D))`: the nearest, a tie going to the higher
   index (east or north). Ours.
4. **`cubic`** (M2). Keys cubic convolution with `a = -0.5` (Keys, 1981) over the 4 by 4 samples
   around the cell of step 2, weights from the fraction in doubles, summed west to east for each
   row, then the four rows south to north. A sample index past the raster's edge reads the edge
   sample. The result may overshoot `range`; it is not clamped. Its weights are 1, 0, 0, 0 at a
   fraction of 0, so it too reads a sample exactly.

- A post beyond `extent` takes `outside`. The step at the edge is the author's to see.
- So a raster written from a field's own posts over the world extent reads back bit for bit with any
  filter, and a raster at half the grid's resolution gives its samples exactly at the posts it lies
  on (the spike's relief check).
- A raster whose level step, `abs(hi - lo) / top`, exceeds 0.1 m on a field whose unit is `m`
  (written or implied by its role) is `field.file_coarse` (warning), at any depth. From the spike:
  256 levels over 66 m showed as terraces of 0.26 m. The threshold is ours.

Tiled rasters for worlds past a few kilometres are set aside (doctrine section 7).

## 5. Entries

An **entry** is one change to a field (`authority.md` 4). Entries come from two places, and both are
applied by one rule:

- the field's own **rows**, `[field.<name>.then.<id>]`, written in the world or a layer, or returned
  by a step. "Row" in this chapter means a `then` row and nothing else;
- kind effects, one entry per (effect, feature) pair (`authority.md` 2).

The order of every entry on a field is `authority.md` 4.1 (the head: the `then` rows) and 4.2 (the
effects, after the head). This chapter does not restate it. A `then` row has no
authority key: it is derived by design, and a firm author decision is written as an anchored
feature, not as a row (principle 3). This chapter defines only what one entry does.

`value` is required on a scalar entry and `paint` on a weights entry, effects included; a missing one
is `field.row_missing`. `mask` defaults to 1. `formulas.md` 6.1 checks that `mask` lies in 0 to 1
at every post where it is evaluated (`formula.range`, never clamped), so `m` below is in range.

### 5.1 One entry on a scalar field

```
m = mask
v = (1 - m) * v + m * value
```

In doubles, in that order, with no fused multiply-add. The form is GLSL's `mix` (Khronos GLSL 4.60,
section 8.3), chosen because it is exact at `m = 0` and `m = 1`: an anchored entry with mask 1
gives its own value bit for bit before narrowing. `v` is the field's value so far at that post.
`value` and `mask` are formulas evaluated at the post; both may read `v` (`formulas.md` 6.4,
6.5), so a row can raise ground by 2 m with `value = "= v + 2"`. `paint` on a scalar entry is
`field.row_key`. The rule is the spike's; the evaluation order is ours.

### 5.2 One entry on a weights field

```
m = mask
w[k] = (1 - m) * w[k]              for every material k other than paint
w[paint] = (1 - m) * w[paint] + m
```

In doubles, materials in id order. If the weights summed to at most 1 before, they do after, up to
rounding. Every check on a sum (here, `field.unpainted`, the conformance cases) adds the narrowed
weights in doubles in material id order and allows `1e-6`: "at most 1" is `sum <= 1 + 1e-6`. Nothing
is renormalised. The tolerance is ours.

`value` on a weights entry is `field.row_key`. On the `material` field, a `paint` the pack does not
declare is `pack.unknown_material` (`assets.md`). On a weights field without a role, `paint` is any
id: a class name the project chooses. The rule is the spike's, kept.

A post of the `material` field whose weights sum to less than `1 - 1e-6` has an unpainted
remainder. `field.unpainted` (warning) gives the first such post and the count. How such a post is
written is `build.md` 6.2. The usual fix is a first row with no mask.

### 5.3 Bounded effects

An effect with a `radius` (its own, else its kind's) does nothing at a post where the feature's `d`
(`numbers.md` 5.5) exceeds `radius`. There neither `value` nor `mask` is evaluated, so no error can
arise there.

- The effect's window is the smallest rectangle of posts that covers the shape grown by `radius`,
  cut at the extent. Its formulas are evaluated at the posts of that window with `d <= radius`.
- **The ring** is the posts with `radius - r < d <= radius`, `r` the resolution, leaving out every
  post on the extent's edge (`i = 0`, `i = nx - 1`, `j = 0`, `j = ny - 1`). A mask above 0.02 at a
  ring post is `field.effect_cut` (warning), at the first such post in post order: the effect stops
  in a visible step there. A shape cut by the world's edge is not a seam and is not reported. The
  threshold is the spike's.
- An effect with no radius on it or its kind is evaluated over the whole extent. It is legal; the
  change report then marks the whole field when its feature moves.

## 6. Resolving a field

### 6.1 On demand, once

A field is resolved when something first reads it: a formula, a placement on the surface, a
scatter's `keep_out`, a step's request, or export. A field read by a formula is resolved, with all
its entries in the reader's view (6.2), before the post that reads it is evaluated. A cycle between
fields is `graph.cycle` (`graph.md`).

Before any window of a field is resolved, every feature shape its entries use is resolved over the
whole extent, `where` regions included (`geometry.md`), so an effect's outline and `d` never depend
on the window. Each (field, view, window) is a `field` node; its key, early cutoff and reuse
across builds are `graph.md` 3.1. Two views with the same entries are one node.

Field diagnostics (`field.role_range`, `field.unpainted`, `field.effect_cut`, `field.file_*`) are
computed on the build's own view, over the whole extent, once per build, never on demand. "First
post" always means the least post in post order (`numbers.md` 4: rows from the south, each west to
east).

### 6.2 Views

A view is the subset of a field's entries a reader sees, applied in the field's order
(`authority.md` 4). Every view is fixed by declared facts before the reader runs, never by what it
returns.

| Reader | View | Defined in |
|---|---|---|
| The build | Every entry. | |
| A step at layer L reading with `view = "head"` (M2) | Its head view: the base; the head entries of layers below L; the `then` rows written in layer L's text. | `authority.md` 4.4 |
| A step at layer L reading with `view = "full"`, the default (M2) | Its full view: every entry from layers at or below L, except what steps at L return. | `authority.md` 4.4 |
| `v`, `under(stat)`, `smoothed_along` inside an entry | The entries before this one, in the view being resolved. | `formulas.md` 6.4, 6.5, 8.5 |
| The `keep_out` test of placements | One view per world: the build's view without the entries of any footprint feature (the effects its kind contributes; a footprint feature has no `then` rows). | `rules.md` 8.3 |

- A view applies to every field resolved while answering that reader. A step names the view in each
  read (`authority.md` 4.4), never by what it returns; inside a view of the build, other fields are
  read in the build's view.
- A step that read a field's full view and returns rows on it is `authority.step_view`.
- Each view is a node of its own, in the key (`graph.md` 3.1, 4). R21 accepts the cost.

### 6.3 Windows and halos

A window of a field equals the same cut of the whole field (`numbers.md` 6). Applying entries needs
no halo: every entry is evaluated at one post from values at that post (R7), shapes are resolved
over the whole extent (6.1), and a bounded effect contributes nothing beyond its radius. A raster's
filter reads the raster, never a neighbouring window.

A formula that calls a neighbour function (`slope`, `under`, `smoothed_along`) grows the window of the
field it reads by that function's halo (`formulas.md` 8, `numbers.md` 6). A reader that needs
neighbours (a step computing slope, a lint, export normals) asks for its window with a halo; the
core resolves the grown window, cut at the extent, and never invents posts beyond it. How a step
asks is `protocol.md`.

Weights are held sparse: a window keeps one plane per material that some row painted in it, and no
plane for any other.

### 6.4 The last writer at a point

A diagnostic that must say what set a field's value at a point (M1: `surface.gap`, `surfaces.md`
3.4) asks for the field's **last writer** there. This section is how the core finds it.

- **Rule.** At a post, the last writer is the last entry, in the field's order (`authority.md` 4)
  in the build's view, whose `m` is above 0 at that post. `m` is the mask in doubles, before
  narrowing. An effect beyond its radius (5.3) applies nothing there, so it is never a writer. At a
  point, the posts are the four of the cell 1.2 picks, and the last writer is the latest in the
  field's order over those four. Where no entry has `m` above 0 at any of them, the writer is the
  field's base: its `base` key, or its `file` key.
- **Named as.** A row by its id, file and line; an effect by its kind effect's span and the feature
  (`diagnostics.md`, chain); the base by its key's file and line.
- **Recomputed on demand, never stored.** The core keeps no per-post record. When a diagnostic asks,
  it resolves the field again over the window of those four posts, grown by any halo its entries
  need (6.3), in the build's view, keeping at each post the index of the last entry with `m`
  above 0. The replay is not a `field` node: it is never stored, keyed or reused (`graph.md` 3.1).
  Its narrowed values equal the build's at those posts bit for bit, since a window equals the same
  cut of the whole (`numbers.md` 6); a difference is a fault in the core, never in the world.
- **Cost.** Nothing on a build where no diagnostic asks. Per point asked: one resolution of a 2 by 2
  window plus halo, so (entries) × (window posts) evaluations of `value` and `mask`. Shapes are
  already resolved over the whole extent (6.1). Storing it instead would cost one 32-bit entry index
  per post per field (6.5 MB beside a 6.5 MB terrain at the probe's size), written on every build
  for a reader that only runs when a meeting fails. Ours.

## 7. What a step returns (M2)

Reading R19(c) and R21 together: a step exchanges whole fields as data, but enters a field's order
only as rows, so it may return a whole field only as a new field that its own rows read. This
reading is ours; the lead is asked to record it in the doctrine.

In the document it returns (`protocol.md` 5) a step may write:

| May | May not |
|---|---|
| `then` rows on any field in its stratum, with `value` and `mask` formulas. They take the step's layer rank in the head (`authority.md` 4.1). | A row on a field of a higher layer: `field.step_layer`. |
| A new field of its own, `base` or `file`, for its rows to read. Its `role` follows 2.1 like any field: a `keep_out` is legal; a second terrain is `surface.two_terrains`; a second `material` is `field.role_twice`. | `base`, `file`, `role`, `type`, `unit` or `tags` on a field declared outside its document: `field.step_base`. |
| Features, whose kinds write fields by their effects. | |

A returned row's id is named `<step>#<id>` by the build (`identity.md` 2.4), so it cannot collide
with a row the world wrote.

Example: an erosion step at layer 1 reads `height` with `view = "head"` (the base, layer 0's `then`
rows and layer 1's written `then` rows), returns `[field.eroded]` with `file` set to its output, and
`[field.height.then.erosion]` with `value = "= f('eroded')"`. That row sorts after every `then`
row it read and before every effect (`authority.md` 4.1), so a world `then` row cannot overwrite it, and every effect of
every layer, anchored grooves included, is applied once, on top of it. The facts an erosion must
honour reach it as features (`authority.md` 7), not as effects in the field it reads.

Fields travel as array files named by digest, never as JSON (R20). A stored array file is checked
against its digest when first read (`graph.md` 3.2, `graph.store_corrupt`).

### 7.1 Handing materials over

The `material` field goes to the loader per chunk (`build.md` 5): for each chunk, the materials with
a weight above 0 anywhere in it, in id order, and one plane of weights for each (`build.md` 6.2).
A world may use any number of materials; a chunk names only its own. A loader that draws fewer per
chunk than a chunk names raises `load.too_many_materials` itself, naming the chunk and its
materials (`build.md` 15.1); it is the loader's message, not the build's.

## 8. Volume fields

`type = "volume"` is a bounded 3D field (caves, overhangs, cliffs), owned by a feature like a
bounded surface (R25). Its keys, its rows and how it yields floors are decided for M5 in
`surfaces.md`. M1 rejects it with `doc.not_yet`.

## Worked example

```toml
[field.height]
role = "surface"
base = "= 20 + 0.05 * y"

[field.height.then.terrace]
value = 24
mask  = "= smoothstep(30, 50, y)"

[field.cover]
role = "material"
type = "weights"

[field.cover.then.grass]
paint = "grass"

[field.cover.then.rock]
paint = "rock"
mask  = "= smoothstep(22, 26, f('height'))"

[field.woods]                  # a PNG base: M2
tags    = ["woods"]
file    = "woods.png"
channel = "g"
extent  = [0, 0, 64, 32]
outside = 0
```

At the post (10, 40), with the pack declaring `grass` and `rock`:

| Entry | Result |
|---|---|
| `height` base | 20 + 0.05 × 40 = 22 |
| `terrace`: mask smoothstep(30, 50, 40) = 0.5 | (1 - 0.5) × 22 + 0.5 × 24 = 23 |
| `cover.grass`: mask 1 | grass 1 |
| `cover.rock`: mask smoothstep(22, 26, 23) = 0.15625 | grass 0.84375, rock 0.15625 |
| `woods` | (10, 40) lies north of the raster's extent (`y1 = 32`), so 0, its `outside`. At (10, 20) it would be the green level there, mapped by `range` [0, 1]. |

Reading `f('height')` inside `cover` resolves `height` first, with its one row. A kind effect on
`role:surface` would sort after `terrace` (`authority.md` 4).

## Diagnostics

| Code | Severity | Says | Hint |
|---|---|---|---|
| `field.type_unknown` | error | A `type` not on the list. | Nearest type; for `material`: `type = "weights"`, `role = "material"`. |
| `field.role_unknown` | error | A role that is not `surface`, `material` or `keep_out`. | Nearest role; a library word belongs in `tags`. |
| `field.role_type` | error | A role on a field of the wrong type. | The role's type, e.g. `type = "weights"`. |
| `field.role_unit` | error | A role on a field with another unit. | Remove `unit`, or write the role's. |
| `field.role_twice` | error | A second `material` field; names both. | Remove `role` from one; it is handed over as data. |
| `field.role_missing` | error | No `surface` field, and something asked for ground; names the first that asked. | Add a field with `role = "surface"`. |
| `field.role_range` | warning | A `keep_out` value outside 0 to 1, clamped; the field and its first post. | Wrap the row or base in `clamp(..., 0, 1)`. |
| `field.unit_unknown` | error | A `unit` not on the list. | Nearest unit. |
| `field.weights_unit` | error | A unit other than `ratio` on a `weights` field. | Remove `unit`. |
| `field.base_and_file` | error | A field with both `base` and `file`. | Keep one. |
| `field.weights_base` | error | `base` or `file` on a `weights` field. | Read the raster as a scalar field and use it as a row's `mask`. |
| `field.row_missing` | error | A scalar row or entry with no `value`, or a weights one with no `paint`; names the row or effect. | Add the key. |
| `field.row_key` | error | `paint` on a scalar entry, or `value` on a weights entry; names the row or effect and the field. | Use the key of the field's type, or narrow the effect's selector. |
| `field.file_missing` | error | The file is missing or unreadable; the resolved path. | Check the path, relative to the document. |
| `field.file_format` | error | An unsupported format, depth or palette; an array file whose `shape` is not 2-D or whose `descr` is not `<f4`; a raster with a dimension under 2. | Save as 8- or 16-bit grey PNG, or an array file. |
| `field.file_too_large` | error | More samples than the post-count limit; the header's size. | Crop or downsample the raster. |
| `field.file_key` | error | A file key without `file`, one that does not apply, an unknown `channel` or `filter`, or a channel the picture lacks. | The keys that apply; for `luma`, name one channel. |
| `field.file_outside` | error | `file` without `outside`. | Add `outside`, the value beyond the raster. |
| `field.file_range` | error | `range` with `lo == hi`. | A constant is a `base`. |
| `field.file_offworld` | warning | A raster `extent` wholly outside the world's. | Correct `extent`. |
| `field.file_nonfinite` | error | A non-finite sample; its row and column. | Repair the file. |
| `field.file_coarse` | warning | A level step above 0.1 m on a field in `m`; the step. | Use 16-bit, or a narrower `range`. |
| `field.file_profile` | info | A PNG's gamma, colour profile or `tRNS` was not applied. | None needed; levels are read as stored. |
| `field.unpainted` | warning | Posts of the `material` field with weights summing below 1; the first and the count. | Add a first row with no mask. |
| `field.effect_cut` | warning | An effect's mask is above 0.02 on its ring; the effect, the feature and the first post. | Raise `radius`, or fade `mask` to 0 by it. |
| `field.step_base` | error | A step's document sets `base`, `file`, `role`, `type`, `unit` or `tags` on a field it did not declare. | Return a new field and a row that reads it. |
| `field.step_layer` | error | A step's row on a field of a higher layer. | Move the step up, or the field down. |

Codes of other chapters used here: `doc.unknown_key`, `doc.not_yet`, `doc.upward_ref`,
`num.extent_empty`, `num.not_finite_result`, `formula.range`, `authority.no_field`,
`authority.step_view`, `surface.two_terrains`, `pack.unknown_material`, `graph.cycle`,
`graph.store_corrupt`, `load.too_many_materials`.

## Conformance

Each case runs through `via build` (R20) unless it says otherwise. Pictures and array files for the
cases live in `spec/vectors/fields/`, written with the cases. Worlds have `extent = [0, 0, 8, 8]`
and `resolution = 1` unless the row says otherwise. Every error case also checks exit 1.

**M1**

| Case | Input | Must hold |
|---|---|---|
| base formula | `base = "= 2 * x"` | post (3, 0) holds 6 exactly |
| one row | base 10, row `value = 20`, `mask = 0.25` | 12.5 at every post |
| mask 1 exact | base 0.1, row `value = "= 1 / 3"`, mask 1 | every post holds the f32 nearest 1/3 |
| mask out of range | row `mask = 1.5` | `formula.range` at post (0, 0), hint `clamp(...)` |
| row reads `v` | base 5, row `value = "= v + 2"` | 7 |
| row order | rows `b` (order 0) value 1, `a` (order 0) value 2, `c` (order -1) value 3, all mask 1 | 1 (c, then a, then b) |
| value required | a scalar row with only `mask`; a weights row with no `paint` | `field.row_missing` at each row's line |
| unknown row key | row `colour = 1` | `doc.unknown_key`, hint `value` |
| type unknown | `type = "material"` | `field.type_unknown`, hint `type = "weights"`, `role = "material"` |
| weights rule | grass mask 1, rock mask 0.25 | grass 0.75, rock 0.25 |
| weights sum | `spec/vectors/fields/masks.toml`: ten rows of three materials, `mask = "= 0.5 + 0.5 * sin(37 * x + 11 * y + <k>)"` | every post sums to at most 1 + 1e-6 |
| unpainted | one rock row, mask 0.5 | `field.unpainted`, first post (0, 0), count 81 |
| painted | grass mask 1, rock mask 0.3 | no `field.unpainted` |
| roles | two `material` fields; `role = "water"` | `field.role_twice`; `field.role_unknown` |
| role type | `role = "material"` on a scalar field | `field.role_type` |
| role unit | `role = "surface"`, `unit = "deg"` | `field.role_unit` |
| unit unknown | `unit = "metres"` | `field.unit_unknown`, hint `m` |
| weights unit | `type = "weights"`, `unit = "m"` | `field.weights_unit` |
| weights base | `type = "weights"`, `base = 1` | `field.weights_base` |
| implied unit | surface, no `unit` | the build index lists `unit = "m"` |
| no surface | an instance and no `surface` field | `field.role_missing`; `diagnostics.json` written |
| features only | features, no instance, no `surface` | builds, exit 0 |
| keep_out max | two `keep_out` array files from the fixture: one 1 at even `i` and 0 at odd, the other the reverse; a scatter with `density = 1` | no candidate kept anywhere: the per-post maximum is 1 at every post, so 1 between them (interpolating each first would give 0.5) |
| keep_out range | `keep_out` base 1.5 | `field.role_range` at (0, 0); read as 1 |
| between posts | `height` base `"= x * x"`, an instance `at = [2.5, 0]` | its ground is 6.5 (bilinear), not 6.25 |
| tag selector | effect on `tag:woods`, two tagged scalar fields | both changed |
| type mismatch | effect `value = 1` on `tag:x`, matching a scalar and a weights field | `field.row_key` naming the effect and the weights field |
| upward selector | a layer-0 feature whose kind writes `tag:x`, the field declared in layer 1 | `doc.upward_ref` |
| no match | effect on `tag:water_level`, no such field | `authority.no_field` once |
| tag in formula | effect `mask = "= f('tag:wet')"`, one field tagged `wet` | reads it; with two such fields, `formula.selector_count` |
| bounded | effect radius 5 on a point at (4, 4), extent `[0, 0, 16, 16]`, `value = "= 1 / (d - 6)"` | posts with `d > 5` unchanged; no error, since `d = 6` lies beyond the radius |
| cut | as "bounded", `value = 1`, mask 1 | `field.effect_cut` at post (1, 1) |
| fade | as "cut", `mask = "= 1 - smoothstep(0, 5, d)"` | no `field.effect_cut` |
| world edge | radius 6 on a point at (4, 4), extent `[0, 0, 8, 8]`, mask 1 | no `field.effect_cut`: every ring post lies on the extent's edge |
| edge equals cut | base `"= 2 * x"`; effect radius 3 on a point at (8, 4), `value = 1`, `mask = "= 1 - smoothstep(0, 2, d)"`, 0 on the ring; built once with extent `[0, 0, 8, 8]` and once with `[0, 0, 16, 8]` | every post of the first equals the second's post at the same place, bit for bit; no `field.effect_cut` in either |
| no material | an instance, a `surface` field, no `material` field | builds, exit 0; the build's index lists no `material` field and no material is handed over; no `field.unpainted` |
| cell on an edge | `height` base `"= x * x"`; `sample` (`surfaces.md` 2.2) at (2, 0) and at (8, 0) | at (2, 0) the cell is 2 to 3, `gx = 5`; at (8, 0), the last column, the cell is 7 to 8, `gx = 15` |
| last writer | core test, not `via build`. `height` base 0; row `a` `value = 1`, mask 1; row `b` `value = 2`, `mask = "= smoothstep(4, 5, x)"`; an effect `value = 5`, radius 1, on a point at (0, 7) | at (2.5, 2.5) and (3.5, 2.5) the writer is `a`; at (4, 2.5), an edge two cells share, and at (4.5, 2.5) it is `b`; at (0.5, 6.5) it is the effect; the replay's values equal the build's bit for bit |
| last writer, base | core test. `height` base 3, no rows | at (2.5, 2.5) the writer is the `base` key's line |
| array file base | `file` a 9 by 9 array file, `outside = 0` | every post equals its sample bit for bit |
| corner alignment | write `height` to an array file; a second world reads it with `file`, each filter | every post equal, bit for bit |
| half resolution | a 5 by 5 array file over the extent | posts (2i, 2j) equal sample (i, j) bit for bit |
| orientation, array file | a 2 by 2 array file with 1 in its first row's first value, 0 elsewhere, `filter = "nearest"` | post (0, 0) (south-west) reads 1 |
| nearest tie | world `extent = [0, 0, 1, 1]`, resolution 0.5; a 2 by 2 array file, `nearest` | post (1, 0) at x = 0.5 takes sample column 1 |
| file extent | an array file with `extent` half the world, `outside = -1` | posts beyond read -1; the header's own size never moves it |
| outside missing | `file` without `outside` | `field.file_outside` |
| file key | `range` without `file`; `channel` on an array file; `filter = "smooth"` | `field.file_key` each |
| offworld | raster `extent = [100, 100, 110, 110]` | `field.file_offworld`; every post `outside` |
| array file format | `planes.npy`, `shape = (2, 9, 9)`; `double.npy`, `descr = "<f8"` | `field.file_format` each |
| missing | `file = "none.npy"` | `field.file_missing` with the resolved path |
| too large | a header asking for more samples than the post-count limit | `field.file_too_large`, before any sample is read |
| non-finite | array file with one NaN | `field.file_nonfinite` at its row and column |
| both | `base` and `file` | `field.base_and_file` |
| narrowing overflow | base `"= 1e39"` | `num.not_finite_result` naming the field and post (0, 0) |
| window = cut | — | `numbers.md`'s `window_equals_whole`; no case here |
| memo | `height` read by three formulas | the run report (`graph.md` 6.2) holds one `height` node per window with status `ran` |
| volume | `type = "volume"` | `doc.not_yet` naming M5 |

**M2**

| Case | Input | Must hold |
|---|---|---|
| PNG 16-bit | 65535-level ramp, `range = [0, 100]` | top level reads 100, level 0 reads 0 |
| range default | 8-bit picture, no `range` | level 255 reads 1 |
| inverted range | `range = [10, 0]` | level 0 reads 10 |
| flat range | `range = [3, 3]` | `field.file_range` |
| orientation, PNG | a 2 by 2 grey PNG, its top-left sample 255, the rest 0, `nearest` | post at (x0, y1) reads 1 |
| channel required | RGB picture without `channel` | `field.file_key` |
| grey on colour | RGB picture, `channel = "grey"` | `field.file_key` |
| grey + alpha | grey + alpha picture, `channel = "a"` | the alpha levels |
| luma | `channel = "luma"` | `field.file_key`, hint to name one channel |
| coarse | 8-bit, surface without `unit`, `range = [0, 66]` | `field.file_coarse` |
| coarse 16-bit | 16-bit, unit m, `range = [0, 10000]` | `field.file_coarse` |
| profile | PNG with `gAMA` and `tRNS` | `field.file_profile` naming both |
| interlaced | Adam7 PNG | same values as the plain one |
| too small | a 1 by 4 PNG | `field.file_format` |
| palette | palette PNG | `field.file_format` |
| cubic | a 4 by 4 array file with one spike, `filter = "cubic"`, resolution half the raster's | values below 0 next to the spike, not clamped; edge posts read the edge sample repeated (vector in the fixture) |
| step view | base 10; a layer-0 kind effect `value = "= v + 2"`; a layer-1 step reads `height` with `view = "head"`, writes it as field `seen`, returns `value = "= f('seen')"` | 12, not 14: the effect is applied once |
| step two rows | a layer-1 step returns rows `p` (order 1) and `q` (order -1) on `height` | both read the same head view; `q` applies before `p` |
| step full view | the step of "step view" reads `height` with `view = "full"` and returns rows on `height` | `authority.step_view` |
| step neutral | step reads `height`, returns a row `value = "= v"` | equal to the build without the step |
| step base | step returns `base` on `height`; `tags` on `height` | `field.step_base` each |
| step layer | a layer-0 step returns a row on a field declared in layer 1 | `field.step_layer` |
| step keep_out | step returns a new field with `role = "keep_out"` | builds; a scatter reads it |

## Changed from the spike

| Spike | v1 | Why |
|---|---|---|
| `height` and `ground` required by name; `water`, `bed`, `hole` read by name | Roles `surface`, `material`, `keep_out`; everything else by tag | Principle 5. |
| Eight ground materials, a hard cap | No cap; sparse per window and per chunk | TODO: `atlas` needed eleven. |
| One material field, `ground`; a second overwrote the first | One `material` role; other `weights` fields as data | Principle 9; the overwrite was silent. |
| `unit` switched anchor comparison | `unit` is a unit; anchors always compared | Principle 3. |
| `layers` | `then` rows | `layer` is a document. |
| `base = "file"` | `file` alone | One key per source. |
| `channel = "luma"` default | No luma; a colour picture names its channel | A mix of channels is a silent guess. |
| Picture edge carried on without `outside` | `outside` required | Vocabulary. |
| `.f32` raw, no header, exact grid only | The build's array file, sampled like a picture | One format for steps and authors. |
| Mask clamped to 0 to 1 | A mask outside 0 to 1 is `formula.range` | Principle 3. |
| `v + (value - v) * m` | `(1 - m) * v + m * value`, f64, then f32 once | Exact at mask 1; R26. |
| A step's returned field had lower effects applied again | A step returns rows; the view it reads is fixed by its stratum before it runs, never by what it returns (`authority.md` 4.4) | R21; SPEC 12.8. |
| A failed expression became 0 | An error; no non-finite value in a field | R26, principle 8. |
| `effect.unbounded` not checked at the world's edge | `field.effect_cut` checked on the ring, off the extent's edge | The world's edge is not a seam. |

## Open

- **Principle 6 and a field's `file`.** See section 4. Decided by a doctrine ruling.
- **Float pictures beyond our array file** (EXR, float TIFF). Decided by the first world that needs
  one; challenge 31, `relief` in M2.
- **A picture painting a material directly**, without a scalar field between. Decided by `relief`
  in M2: does the extra field cost a reader anything.
- **A picture passed through to the engine as a texture.** Decided in `build.md`.
- **How a formula reads one material's weight.** Decided in `formulas.md` (a function, since a
  bare weights name has no single value).
- **More units.** Decided by the first library value none of `m`, `deg`, `ratio` describes.
- **Thresholds** 0.02 (mask on the ring), 0.1 m (coarse level) and 1e-6 (weights sum). Decided by
  M1 measures on `ford`.
- **The post-count limit** for rasters (4.1). Decided by the M0 probe (`numbers.md`).
- **Derived features** (a flood fill, a contour, a flow line made from a field). `authority.md` 8
  holds the rules; whether a step or an engine operation makes them is decided in M2 by `vale`
  and `basin`.
- **Whole-map distance** (the spike's ten-minute compile). `formulas.md`'s `dist` cap bounds it per
  formula; whether a whole-extent distance field is an engine operation, and how it is computed per
  window, is decided by M1's `ford` timings.
- **Routing over a cost field** (`geometry.md` 3.11 names this chapter as its default home).
  Decided when M2 or M3 first needs a found course.
- **Resampling an import's fields onto the parent's posts** (`numbers.md` 5.3). Decided in M4 by
  `strait`.
- **Cross-references.** Section numbers in other chapters cited here are re-checked by the lead
  once every chapter has landed.
