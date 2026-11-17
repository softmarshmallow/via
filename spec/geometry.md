# Geometry

The geometry kernel is the part of the core that turns written points, paths and regions into exact
shapes and answers questions about them: where paths cross, what areas lie between them, what lies
within a distance, how a region divides along its edge. This chapter defines the nouns `feature`
and `frame`, and fixes what each kernel operation takes, returns and guarantees, how it fails, and
how we choose between writing it and taking it from a crate (R19, R24).

Authority: `docs/doctrine.md`. Words: `spec/vocabulary.md`. Units, axes, the arithmetic of frames,
the decimal text to millimetre mapping and rounding: `spec/numbers.md`. Diagnostic shape:
`spec/diagnostics.md`. Names of junctions, edges, areas, cells and pieces: `spec/identity.md`. How a
step calls an operation: `spec/protocol.md`.

## 1. Numbers

| Rule | Statement |
|---|---|
| Lattice | Every coordinate the kernel holds is an integer number of millimetres in the world frame (R24). Text is decimal metres with at most three decimals, mapped exactly by `numbers.md` 3.1. |
| Range | Each coordinate lies in `[-(2^31 - 1), 2^31 - 1]` mm (about ±2147 km). Text outside it is `num.out_of_range` (`numbers.md`). A point the kernel computes outside it (an offset, a cut, a stroke) is `geom.range`. |
| Predicates | Decided exactly in integer arithmetic. No tolerance, no epsilon, no float in a predicate. Widths below. |
| Crossings | The crossing point of two segments is computed as an exact rational and rounded, per axis, to the lattice point whose hot pixel (3.1) holds it: the nearest, a tie going up. Ties to even would put a crossing on a pixel's high edge in a pixel the segments do not pass through, and noding would not settle (found by M1's noding fuzz test). |
| Other constructions | An offset vertex, an arc sample, a curve sample, a point at `t`, a cut end: computed in doubles through `via-num` (R26) and rounded to the lattice by `numbers.md` 3.1, once, at the end of each construction. |
| Lengths | `numbers.md` 5.5 is the one definition, and this chapter does not restate it: a segment's length `lk` is the exact squared length `dx*dx + dy*dy` in 128-bit integers, converted to the nearest double once, then `sqrt`. Cumulative lengths `Ck` and the total `L` are summed in doubles, in order from the start. Every length, `t`, `d` and `sd` in this chapter uses those expressions, so the kernel and formulas give the same bits. |
| Area | Twice the signed area of a ring is an exact integer (mm², shoelace formula). Every area guarantee in this chapter is stated in that integer. |
| Frames | The kernel works in the world frame only. A shape written in another frame is carried into the world frame by `numbers.md` 5 before any operation. |

Coordinate differences fit in 33 bits. The predicates and the integer width each needs:

| Predicate | Width |
|---|---|
| Orientation of three points (sign of a cross product) | 128-bit |
| Segment crossing, collinear overlap, point in ring (winding number) | 128-bit (built from orientation and dot products) |
| Compare two squared point-to-point distances | 128-bit |
| Compare a point's distances to two segments (squared distances as rationals, cross-multiplied) | 256-bit |
| Corner test of 3.6 (`4 dot²` against `3 |a|² |b|²`) | 256-bit |

Exact integer predicates on a fixed lattice are the standard approach of robust computational
geometry (Shewchuk 1997 for the predicates; the lattice removes his adaptive stage, since integer
inputs need no floating filter). The bounds are ours.

## 2. Features, frames and shapes

### 2.1 Feature and frame

A **feature** is a point, a path or a region with an id, a kind (none for a guide) and attributes
(vocabulary, Features). Its shape comes from its one placement key (vocabulary 2.5):

| Key | Shape |
|---|---|
| `at` | point |
| `on` | point (2.6) |
| `path`, with `starts_at` and `ends_at` | path (2.3) |
| `follows` | path (3.5) |
| `region` | region of one piece without holes (2.3) |
| `where` | region of any number of pieces, with holes (3.7) |

A **frame** is an origin and a facing in a parent frame. The world frame is the root. An import, an
instance and a set each have one. The arithmetic of frames (placing, nesting, rounding) is
`numbers.md` 5. Every feature also gives the plane a local frame, `t`, `d` and `sd`, defined in
`numbers.md` 5.5 and computed here (2.6, 3.8).

**Datum.** The world frame is its own datum: north is `+y` and height zero is the value 0 of the
`surface` field (`numbers.md` 1). A world names what that zero is with `datum` (M4); two worlds
with one `datum` join with no lift (`composition.md` 5). `build.md` 5 writes it in `frame`. Where a
frame sits on a map grid is not declared in v1.

### 2.2 Point, path, region

| Shape | What it is | Valid when |
|---|---|---|
| point | One lattice point. | In range. |
| path | A polyline: two or more lattice points in order, with a direction (start to end). | At least two distinct points; no two consecutive points equal (a repeat is removed with `geom.repeated_point`). It may cross itself; operations that need a simple path say so. |
| ring | A closed polyline: three or more points, the last joined to the first. | Simple: no two edges cross or overlap, and no point is visited twice (so no pinch and no zero-width spike `a, b, a`); no consecutive repeat; non-zero area. |
| region | A set of pieces. A piece is one shell ring and zero or more hole rings. | Shells counter-clockwise, holes clockwise; each hole inside its shell; piece interiors disjoint; two rings may share a vertex but never an edge of positive length. |

- A boundary that touches itself at a vertex is two rings sharing that vertex: two pieces, or a
  shell and a hole. It is never one ring through the vertex twice. This is the one form of a pinch.
- A region is always a set of pieces with holes. A written `region` key gives one piece without
  holes; every operation takes and returns the general form.
- An empty region (no pieces) is a legal value. Whether an empty result is an error is said by the
  operation.
- A region is **settled** when noding it (3.1) and rebuilding it gives it back byte for byte: no
  vertex lies within the snap bound of an edge it is not on. Every region an operation returns is
  settled.

### 2.3 Written shapes

- `path` is read as written. `starts_at` and `ends_at` supply its first and last point (2.7).
- `region` is one ring. A ring written clockwise is read counter-clockwise; its first written point
  stays `t = 0`. A written ring that crosses itself, visits a point twice or doubles back on itself
  is `geom.self_crossing` with the position of the first fault; it is never repaired. Fewer than
  three distinct points, or zero area, is `geom.degenerate`.
- Written shapes keep their vertices as written (collinear vertices included), because `t` is
  measured on them. Canonical form (2.5) applies to what operations return.

### 2.4 Curves

`smooth = true` replaces a written `path`'s points by a centripetal Catmull-Rom curve through them
(Yuksel, Schaefer and Keyser, 2011), then by a polyline sampled from that curve:

- Each end gets a phantom point by reflection: before `p0`, `2 p0 - p1`; after `pn`, `2 pn - pn-1`.
- The span between written points `p_i` and `p_i+1` is sampled at `n = max(1, ceil(c / 250 mm))`
  equal steps of the curve parameter, where `c` is the span's chord length. Samples are rounded to
  the lattice; consecutive repeats are removed. Written points are kept exactly.
- A path of two points stays straight.
- `t`, `d` and `sd` are measured on the sampled polyline, the final curve (vocabulary, `smooth`).
- `smooth` is taken only with `path`. With `region`, `where`, `follows`, `at` or `on` it is
  `geom.smooth_not_path` (error), whose hint says where it is taken.

**Default: `smooth = false`.** What the author writes is where the line goes (principle 2), a
straight lane or a wall line stays straight, and the spike's default of `true` moved every `t` on a
path of three or more points without the author asking. A kind cannot change the default: a kind
defaults attributes only, never a reserved key (`authority.md` 1).

The 250 mm step is the spike's resampling interval, kept because the spike's worlds looked right
with it; it is ours.

### 2.5 Canonical form and order of results

Every operation returns shapes in one form, so equal shapes are equal bytes (R26):

- No consecutive repeat, and no vertex that lies on the straight segment between its two neighbours.
- A ring starts at its least vertex, ordered by `x`, then `y`. Shells counter-clockwise, holes
  clockwise.
- Holes in a piece, and pieces in a region, are sorted by their start vertex, then by their second
  vertex.
- A path returned keeps its direction.

The order of lists does not depend on the order of inputs:

| Output | Order |
|---|---|
| Per-input results (`offset`, `stroke`, `substring`, `project`, `interpolate`, `follows`, one region's cells) | The order of the inputs. Within one input's result: as the operation says. |
| Vertices of a noding or network | By `(x, y)`. |
| Segments | By (least end, other end). |
| Junctions | By `(x, y)`. |
| Edges | By (least end junction, other end junction, second vertex of the edge from its least end). |
| Areas, masked pieces | By the shell's start vertex, then its second vertex. |
| Cells | By (frontage edge, side `left` before `right`, station). |
| Provenance entries | By source id, bytewise, then segment index. |
| Instances and names (`via kernel names`, section 7) | By name, bytewise. |

Section 7 writes these lists as bytes, in this order, for the conformance runner.

### 2.6 The local frame and `on`

`project` (3.8) returns `t`, `d` and `sd` exactly as `numbers.md` 5.5 defines them, for paths,
regions and points: `sd` positive to the left of a path and inside a region, `sd = -d` for a point.
`d` is the distance from the feature: `d = abs(sd)` on a path and a point, and `d = max(-sd, 0)` on
a region, so `d` is 0 inside a region, outline included. A formula that wants the depth inside a
region writes `max(sd, 0)`. A bounded effect's cut at `d > radius` (`fields.md` 5.3) and a mask on
`d` rely on this meaning; where this paragraph and `numbers.md` 5.5 differ, `numbers.md` wins. A
point has no `t`; `project` on a point returns `d` and `sd` only. The kernel computes `d` and `sd` as doubles in
millimetres; a formula sees them in metres, divided by `1000.0` once. `t` is a fraction from 0 to 1.

What `numbers.md` leaves to this chapter:

| Case | Rule |
|---|---|
| `t = 0` on a written region | Its first written point. |
| `t = 0` on a produced region (an area, a cell, a piece) | Its shell's start vertex (2.5). |
| `t` on a region of more than one ring (several pieces, or holes) | Undefined, as in `numbers.md` 5.5: there is no single outline to measure along. One exception: on a region of one piece with holes, `on` and `follows` measure `t` and `offset` along its outer ring, the outline a walker meets from outside, so a thing standing on a lake's shore keeps its place when an island appears (asked by `relief` and `vale`). `on` and `follows` on several pieces, and `interpolate` and `substring` on more than one ring, are `geom.many_rings`; in a formula, `t` is `formula.shape`. `d` and `sd` are defined and measure to every ring. |
| Offset at a vertex | The normal of the segment that holds `t`. A segment holds `[t_i, t_i+1)`; `t = 1` is held by the last segment. So at a vertex the outgoing segment's normal is used, never the bisector. This closes `numbers.md`'s open line: one segment's normal keeps `offset` a true distance from that segment, and the ford cottage on a bend is its case (Conformance). |

A point placed `on = { ref, t, offset }` is the point at `t`, moved `offset` metres along the left
normal of the segment that holds `t`. On a region the left normal points inward (vocabulary,
Placement). The result is rounded to the lattice. A point placed inside near a narrow part or a
concave corner can land outside the region; it is placed as stated and nothing is reported.

The slots of `on`: exactly one of `ref` or `of` is needed (both is `identity.on_both`; neither is
`doc.missing_key` naming `ref`); on a path or a region, `t` is needed where a thing is placed at a
point (a feature, an instance, a pin; a run takes `span`, `rules.md` 5). A clear's `on` takes an
optional `t` on a path and no `t` on a region (`identity.md` 4.3); `offset` is a `length`
(`numbers.md` 3.1) and defaults to 0. A missing slot is `doc.missing_key`, naming `on` and the slot,
with the hint to write the whole table (a change that writes only `t` is this case, `documents.md`
6.2; one condition, one code). `t` or `offset` on a ref with no length (a
point feature, an instance, a mount, a path end, a set item) is `geom.no_length`. A slot `on` does not
take is `doc.unknown_key`. A `t` outside `[0, 1]`, or `nan`, is `geom.t_range`.

**`face` words**, decided here as a closed list of three:

| Word | On a path or region | On a ref with a facing (instance, mount, set item, point feature with `face`, path end) |
|---|---|---|
| `along` | The direction of travel of the segment that holds `t`. | The ref's own facing, so the placed thing turns with it (`rules.md` 3.3). A path end's facing is the direction of travel at that end. A point feature without `face` has facing 0. |
| `toward` | Square to the line, toward it: the direction from the placed point to the point at `t`. | `geom.face_side`: there is no offset to face across. |
| `away` | The reverse of `toward`. | `geom.face_side`. |

`toward` and `away` with `offset = 0` have no direction: `geom.face_side`. A word without `on` is
`geom.face_needs_on`. Any other word is `geom.face_unknown`, with the nearest word as hint. A path
end has a facing for these words only: `face` written as a number on it is a bearing, not added to
its facing (`rules.md` 3.3).

**Where the frame jumps (limitation E1).** The frame is the nearest point, with the lower `t` on a
tie (`numbers.md` 5.5). So it is not smooth everywhere. The **medial axis** of a line is the set of
points with two or more nearest points on it. Across it:

| Measure | On a path | On a region |
|---|---|---|
| `t` | Jumps across the medial axis: at a bend, the bisector on the inside of the turn; at a U-bend, the line midway between the two arms. Beyond each end it is 0 or 1. Continuous elsewhere. | Jumps across the medial axis, inside and outside, and across the normal at the first point, where it falls from near 1 to 0 (the seam). |
| `d` | Continuous. On the medial axis its slope turns by the bend's angle: a kink, not a step. | 0 inside. Outside, continuous, with a kink on the outer medial axis. |
| `sd` | Continuous across the medial axis, with the same kink. Beyond each end it steps from `+e` to `-e` across the end segment's line, extended past the end (the side rule of `numbers.md` 5.5). | Continuous; kinks on the medial axis. |

Size of the jump. At a point at distance `e` on the inside bisector of a bend that turns by `τ`, the
two nearest points lie `2 e tan(τ/2)` apart along the path, while both lie inside their segments.
The jump in `t` is that length over `L`. It reaches one metre at `e = 1 / (2 tan(τ/2))` m. On a
polyline the inside medial axis starts at the vertex itself, so no radius, however small, keeps an
effect clear of it: the jump is small near the line and grows with distance. `smooth` (2.4) makes
each turn small, so the jump near the line is small; it does not remove the medial axis. The rule
and its sizes are geometry of the nearest point; the bound is ours.

What this means for a user (the text E1 owes):

- An effect that reads `t` makes a step where it crosses the medial axis; one that reads `d` makes
  a crease there; one that reads `sd` past a path's end makes a step of twice the distance.
- For a profile across a path (a bed, a verge, a bank), read `d`, or `sd` within the path's span.
- Keep an effect that reads `t` narrow: within `1 / (2 tan(τ/2))` m of a path whose sharpest turn
  is `τ`, the jump stays under one metre of the path's length.
- End a path on something (2.7) or at the extent's edge, so the step in `sd` past its end falls
  where it is covered or outside the world.

**Vector `ford_frame`.** `ford`'s beck and lanes (`docs/plan.md`, milestone one), in the world frame:
the beck `[[150, 256], [140, 170], [160, 90], [150, 0]]`, `L = 259595.40718138084` mm; the ford at
`t = 0.55` on it, `[153.63, 115.48]`; `lane_west` from `[0, 120]` to the ford; `lane_east` from the
ford to `[256, 140]`. The beck turns by 20.67° left at `[140, 170]` and 20.38° right at
`[160, 90]`: one metre of jump at `e` of about 2.74 m and 2.78 m. Each pair is two posts 1 mm apart
across a jump. Points in millimetres; `d` and `sd` in metres; values are `numbers.md` 5.5's doubles,
bit for bit, in shortest form.

| Feature, place | Point | Nearest | `t` | `d` | `sd` |
|---|---|---|---|---|---|
| beck, inside the bend at `[140, 170]` | `[160000, 171293]` | segment 1 | 0.347370404256366 | 19.716448566078615 | 19.716448566078615 |
| | `[160000, 171294]` | segment 0 | 0.31966700518731006 | 19.716689495547183 | 19.716689495547183 |
| beck, inside the bend at `[160, 90]` | `[140000, 88654]` | segment 2 | 0.6648343886445807 | 19.729033859375637 | -19.729033859375637 |
| | `[140000, 88655]` | segment 1 | 0.6375139173642725 | 19.729060418580506 | -19.729060418580506 |
| `lane_west`, 10 m past its end | `[163625, 115186]` | end vertex | 1 | 9.999323027085383 | 9.999323027085383 |
| | `[163625, 115185]` | end vertex | 1 | 9.99935247903583 | -9.99935247903583 |
| `lane_east`, 10 m before its start | `[143905, 113150]` | start vertex | 0 | 10.000226247440605 | -10.000226247440605 |
| | `[143905, 113151]` | start vertex | 0 | 9.999993299997756 | 9.999993299997756 |

At 19.7 m from the beck, 1 mm across the bisector moves the nearest point 7.19 m and 7.09 m along
the beck (`2 e tan(τ/2)` gives 7.19 and 7.09), while `d` changes by less than 0.3 mm. Past a lane's
end `sd` changes sign over 1 mm. The beck's ends lie on the extent's edge, so its own step lies
outside the world; each lane's step past the ford lies inside it, over the beck. The machine copy is
the case `spec/cases/geometry/ford_frame/`: one `via kernel project` input per feature (section 7;
one input holds one shape, so the three features cannot share one vector file), each row of the
table a check on its output, values as written here.

**A smoothed frame: decided in M1 track A (R33): none in v1.** The vector holds bit for bit through
`via kernel project`. A smoothed frame would be a second frame beside the nearest point, one more
mechanism (principle 10), and would move the `t` the author's points give (principle 2), while the
jump it removes is bounded and stated above. So v1 keeps the nearest-point frame, and E1 stays
accepted with the note above. The condition binds the libraries: an effect that reads `t`, directly
or through `smoothed_along` (which reads each point's `t`, `formulas.md` 8.5), keeps its mask
within `1 / (2 tan(τ/2))` m of its line where the line turns by `τ`, and no effect reads `sd` past
a path's end.

Measured when this was decided, on ford built from the text above with `lib/` as written then
(resolution 0.5 m, `via eval --field water` and `--field height`, every pair of neighbouring posts
within 8 m of a point where both posts are wet, one machine, one thread): the largest step in
`water` is 0.091 m inside the bend at `[140, 170]`, against 0.036 m at the bend at `[160, 90]` and
0.034 m on the straight stretch near `[145, 210]`, which is the ground's fall over one post. The
excess at `[140, 170]` is the jump in `t`: `lib/water`'s `level` reads `smoothed_along(4)` with its
mask out to `width / 2 + 4` = 7 m, past the beck's bound of 2.74 m, so it breaks the condition.
The fault is the library's, not the frame's: `level` keeps its `t` read within the bound, or takes
its fall from the ground beneath it (owed by `lib/water`, M1). The lane's `bed` reads
`smoothed_along(24)` on a straight path, which has no inside medial axis; the ford's `approach`,
the bank, the verge and the pads read `d` only. A library effect that breaks the condition with no
such fix reopens this paragraph.

### 2.7 Ends and meetings (R18)

A path meets another thing at its ends or by `follows`, nowhere else. `starts_at` and `ends_at` name
what the end meets. Every meeting is a pair of mounts (R31): the path end is one, and a name that is
not a mount means that place's default mount. The point each kind of name gives:

| `starts_at` or `ends_at` names | The end is at |
|---|---|
| a point feature | Its default mount: its point. |
| a path end (`beck.end`) | Its default mount: that end's point. |
| an instance or a set item | Its default mount: its origin. |
| a mount | Its plan point (`surfaces.md` 3.1). |
| a whole path or a region | `geom.meet_shape` (error): it has no one point. The hint: name an end, or place a point feature on it with `on`. |

The check: the build nodes every path feature that has a kind (guides are left out) and reads the
provenance (3.1). A point where two such paths cross or touch is owned when it is the end of one
named onto the other or onto a point on it (`starts_at`, `ends_at`), when one `follows` the other,
when one names the other in an attribute its kind lists in `names` (one was found from the other,
as a course returned by a step names the river it was followed from), or when both are inputs of
one network, which owns its junctions. Any other is `geom.unowned_crossing`
(warning) at the point, naming both, with the hint to end one on the other or on a named owner.
This is R18's "a meeting nobody owns shows as a stub, a doubled deck"; ford's lanes end on the ford,
so ford raises none.

### 2.8 The extent

The kernel keeps shapes whole, inside or outside the world's `extent`, so a river that comes from
beyond the map keeps its course. It clips only when an operation is given an explicit `clip` region
(3.2). It reports nothing about the extent; a chapter that places or samples by the extent says what
it does with what lies beyond it.

## 3. Operations

Each operation is an engine operation with no domain noun (R23). A step calls it as a batch, columns
in and columns out (`protocol.md`); the conformance runner calls it through `via kernel`
(section 7). Every input shape carries the id of the thing
it came from, its **source**. Two inputs of one batch with one source id is `geom.duplicate_source`.
Every operation is deterministic: the same inputs give the same bytes at any thread count (R26).

A kernel diagnostic carries the source id and a position. A step's caller maps the source id to
subject, file and line (`diagnostics.md`); `via kernel` writes it as 7.1 says.

A parameter outside its stated range (a width or depth not above 0, a mitre limit below 1, a
minimum above what it bounds) is `geom.bad_parameter`, naming the parameter and its value.

### 3.1 Noding

Takes a list of paths and ring outlines, each with a source. Returns **vertices** and **segments**
such that no two segments cross, overlap or touch except at a shared vertex.

- **Mechanism.** Snap rounding on the 1 mm lattice: every segment end and every crossing makes a
  hot pixel; every segment that passes through a hot pixel is routed through its centre (Hobby 1999;
  Guibas and Marimont 1998). The hot pixel of lattice point `(x, y)` is `[x - 0.5, x + 0.5) ×
  [y - 0.5, y + 0.5)` in millimetres, half-open on the high side of both axes. To make the result
  stable we use a variant whose output, rounded again, is unchanged: iterated snap rounding
  (Halperin and Packer 2002), decided in M1 by the `near` and `linework_a` fixtures and a fuzz test
  of 3000 random batches: each fragment is routed again until it passes through no hot pixel but
  its two ends.
- **Provenance.** Each segment carries every source it lies on, as `(source, segment index on the
  source, t interval, same or reversed direction)`. The `t` interval is two doubles, the `t` of the
  segment's two ends on that source, each by `project`'s sequence (3.8).
- **Overlap.** Where two sources share segments, each shared segment names both. One
  `geom.overlap` (info) is given per contiguous shared stretch per pair of sources, with both ids
  and the stretch's two ends. A source that runs back over itself is a pair with itself and is
  reported the same way (challenge 63: the deck that doubled back).

| Guarantee | Statement |
|---|---|
| Noded | No two segments cross; two segments meet only at a vertex. |
| Idempotent | Noding the output again returns it unchanged, byte for byte. |
| Close | Every segment lies within the bound of the chosen variant of its sources; for plain snap rounding that is √2/2 mm. Iterated snap rounding's measured bound on `linework_a` is 0.64 mm; the guarantee checked is 1.5 mm, so a further iteration's drift is allowed for. |
| Faithful | Every point of every source lies within the same bound of some segment that names it. |
| Order-free | Shuffling the input list changes no byte of the output (2.5). |

### 3.2 Network and areas

Takes noding's inputs, optionally a `clip` region, optionally **naming points** (point features, by
id), and the world's post grid (extent and resolution, for representative points). If `clip` is
given, its outline is noded with the rest and every segment outside it is dropped.

The network has two levels:

| Level | Things | What they are |
|---|---|---|
| fine | vertices, segments | Noding's output. |
| coarse | **junctions**, **edges** | A junction is a vertex whose degree is not 2, a vertex where the set of sources on its two segments differs, or the end of a source. An edge is the chain of segments between two junctions, returned as a path from its least end junction. A ring of segments with no junction gets one at its least vertex, and is one edge that starts and ends there. |

`identity.md` names junctions and edges. The word `face` is orientation only (doctrine section 6).

An **area** is a bounded region of the plane enclosed by the network's edges: a region of one piece,
possibly with holes. For each area the kernel returns:

| Output | What it is |
|---|---|
| `region` | The area as a region (one piece, canonical). |
| `sides` | Every `(source, side)` of the edges on the area's shell, side `left` or `right` of that source's travel, sorted by source then side. A ring source's inside is its left. |
| `edges` | The shell's edges in order, counter-clockwise, from its start vertex. |
| `holes` | For each hole ring, its edges: the outline of a component inside the area. |
| `spurs` | Edges with this area on both sides: dead ends and bridges to an inner component. |
| `point` | Its representative point (below). |
| `naming` | The ids of the naming points strictly inside it, sorted. |

`sides` come from the shell only. A spur or a component inside the area (a dead-end path, a pond's
outline) is listed in `spurs` or `holes` and never changes `sides`, so adding one does not rename
the area (R13). A new junction on the shell splits an edge but leaves `sides` the same.
`identity.md` 2.3 names an area by `sides` only in M1. `naming` is returned for M3: whether an
area may be named after a feature inside it is (open), decided with M3's `quay` and the districts
library (`identity.md`, Open).

**Representative point.** The one interior point used by areas, cells and masked pieces, in this
chapter and in `identity.md`. Candidates are the posts of the world grid (`numbers.md` 4) strictly
inside the region, the grid's lattice extended past the extent (a shape beyond it is kept whole,
2.8). The one farthest from the outline (every ring) wins; distances are compared
exactly (section 1); ties go to the lowest `y`, then the lowest `x`. If no post is inside, the
candidate spacing is halved by integer division, anchored at the grid's origin, down to 1 mm, until
one is. If no lattice point is strictly inside, the point is the shell's start vertex. Ours; chosen
over the centroid, which can lie outside a bent region, and over polylabel (Agafonkin 2016), whose
result depends on its queue order and stopping rule.

A network is declared in text by `[network.<id>]` (section 8, decided in M3, R33). Text gives it no
naming points in v1: its areas are named by `sides` alone (`identity.md` 2.3), and the kernel's
`naming` input stays for the case that reopens it (Open).

| Guarantee | Statement |
|---|---|
| Euler | `V - E + A = C` at both levels: vertices and segments, and junctions and edges; `A` areas, `C` connected components. Checked on every build of a network, not only in tests. |
| Closure | The sum of the areas' twice-areas equals the sum, over top-level components only, of the twice-area each encloses, exactly. A component is top-level when it lies inside no area of another. What it encloses is the union of its own areas, holes filled. |
| Cover | Every edge has an area or the outside on each side; no area overlaps another. |
| Dead ends | A junction of degree 1 is a dead end: legal, and counted (`geom.dead_end`, info, with the count). Bridges are not counted. |

A network with no area is `geom.no_area` (warning).

### 3.3 Region booleans

Takes two regions, or a list for union, and one of `union`, `intersection`, `difference`. Returns a
region. Inputs are noded together first, so the pieces of every result come from one arrangement. A
list union uses the non-zero fill rule, so overlapping inputs need not be unioned first.
Symmetric difference (`xor`) has no consumer before M4; it is added when one asks for it.

| Guarantee | Statement |
|---|---|
| Valid | The result is a valid, settled region in canonical form. |
| Closure | On the noded inputs `A'` and `B'`: `area(A' ∖ B') + area(A' ∩ B') = area(A')`, and `area(A' ∪ B') + area(A' ∩ B') = area(A') + area(B')`, exactly. `|area(A') - area(A)|` is at most the noding bound times `A`'s perimeter. |
| Idempotent | For `A` settled and canonical: `A ∪ A = A`, `A ∩ A = A`, `A ∖ A` empty, byte for byte. |

Overlapping regions are never cut by one another in the kernel. Where two features' regions
overlap, each keeps its whole shape; which entry wins on a field is the order of `authority.md` 4,
never the order written.

### 3.4 Offsetting and stroking

**Offset** takes a region, a distance `d` in integer millimetres (positive grows, negative insets), a
join (`round`, `mitre`, `bevel`) and, for `mitre`, a limit (default 2). Returns a region. An inset may
split a piece into several, or leave nothing. `d = 0` returns the input in canonical form.

**Stroke** takes a path, a width to its left and a width to its right (each at least 0, not both 0),
a join and a cap (`butt` or `round`). Returns the region swept. A stroke of a self-crossing path is
the union of its pieces. A `square` cap has no consumer before M5; it is added when one asks.

**Joins.**

- `bevel`: the chord between the two offset segments' ends.
- `mitre`: the two offset lines extended to their meeting point, the tip. The mitre ratio is the
  distance from the input vertex to the tip divided by `|d|`. Where it exceeds the limit, the corner
  is cut by the line square to the corner's bisector at distance `limit × |d|` from the input vertex
  (the `miter-clip` join of SVG 2). The limit must be at least 1.
- `round`: an arc about the input vertex.

**Arcs.** A round join or cap is a polygon whose chords stray at most 5 mm from the true arc. For an
arc of angle `a` radians at radius `r` mm, both doubles: if `r <= 5`, `n = 1`; otherwise
`q = 1.0 - 5.0 / r`, `h = 2.0 * acos(q)`, `n = max(1, ceil(a / h))`, each step an IEEE double
operation in that order and `acos` from `via-num`. The arc has `n` equal chords. The tolerance is
fixed in v1; ours.

| Guarantee | Statement |
|---|---|
| Valid | A valid, settled region; no self-crossing survives, whatever the input's concavity. |
| Distance | Every result edge not made by a join or a cap lies on the line at distance `|d|` from an input segment, parallel to it, within 1 mm. Join and cap vertices lie as defined above, within 1 mm. |
| Empty | An inset that leaves nothing returns an empty region and `geom.offset_empty` (warning). A split is `geom.offset_split` (info, with the piece count). |

### 3.5 A path along another feature

Takes a reference (a path, or a region's outline), a span `[from, to]` of fractions along it, and an
offset in integer millimetres to the left of its travel (inward on a region). Returns one path, in
the reference's direction. This is what `follows` lowers to (section 4).

Mechanism: the substring of the reference over the span (3.8), then its one-sided offset curve: the
boundary of its one-sided stroke at `|offset|` with `round` joins, less the reference itself and the
two ends. Loops an inside bend would make are cut out. This is the method of JTS `OffsetCurve`
(Davis, JTS 1.19, 2022). With `offset = 0` the result is the substring.

- On a region, `from > to` runs through `t = 0`, and `from = to` is the whole outline, starting and
  ending at `from`; the result is then closed.
- On a path, `from >= to` is `geom.span_order`.

| Guarantee | Statement |
|---|---|
| Distance | Every result segment not on a round join lies at `|offset|` from the substring, within 1 mm; round-join vertices within 5 mm (3.4). |
| Simple | The result does not cross or touch itself. |
| One piece | If the offset curve breaks into several pieces or touches itself (an offset wider than a narrow part allows), the result is `geom.follow_broken` (error) with the position of each gap and the widest offset that would hold. That offset is found by bisection over whole millimetres between 0 and `|offset|`, each probe a full offset curve. The result is never joined or trimmed silently. |

A reference that is a point or has no length is `geom.no_length`.

### 3.6 Cells along a frontage

Takes a region and its **frontage**, the part of its outline that cells face. The frontage is either
a list of `(edge, side)` from a network result, each edge on the region's shell or among its spurs,
or `outline`, the whole shell of a region feature. Also takes, in integer millimetres:
`cell_length`, `band_depth`, `min_cell_length`, and `min_cell_area` in square millimetres. Returns
**cells**, regions that divide a band of the region along its frontage, and `rest`, the region minus
the union of the cells.

The user keys for these inputs are section 8.2's: `spacing` (`cell_length`), `depth`
(`band_depth`), `min_length` and `min_area`. `width` is not used for them; it is a library attribute
(vocabulary 6). `frontage` is a core word with the meaning above, and the key that selects it.

Mechanism (ours, after the skeleton strips of Vanegas et al. 2012, "Procedural generation of parcels
in urban modeling", simplified to straight cuts):

1. **Chains.** The region's shell is walked counter-clockwise with the area on the left, and a spur
   in the frontage is walked where it leaves the shell: up its one side, round its tip, down its
   other side, back to its foot. Consecutive frontage edge sides along that walk form a chain. A
   spur side not in the frontage is left out of the walk; where that leaves the walk broken (a spur
   with one side in the frontage), the chain breaks there too. A spur on a component inside the
   area (a hole's) is walked with that component, whose outline is not frontage (Open).
2. **Band.** The union of each chain's one-sided stroke (3.4) toward the region, width
   `band_depth`, join `mitre` with limit 2, caps `butt`, intersected with the region. A chain's
   shell edges are one stroke, and each run along one side of a spur another, butt-capped at the
   spur's foot and tip. Parts of the
   region near outline that is not frontage are not in the band; they fall in `rest`.
3. **Stretches.** Each frontage edge side is split at its corners into stretches. A corner is a vertex
   where the direction turns by more than 30 degrees, decided exactly: with incoming direction `a`
   and outgoing `b`, a corner when `dot(a, b) <= 0` or `4 dot(a, b)² < 3 |a|² |b|²`. Exactly 30
   degrees is not a corner. A junction is a stretch end too: between two frontage edges, and
   where an edge that is a loop meets itself.
4. **Stations.** A stretch of length `L` (section 1) holds `n = floor(L / cell_length)` cells, with
   stations at `k L / n` for `k = 1 .. n - 1`, each point found by `interpolate`. A stretch shorter
   than `cell_length` holds no cell: `geom.cell_none` (warning, with its position). Stations are
   numbered along the edge side from 0, counted from the edge's start in its source's direction,
   across its stretches.
5. **Cuts.** At each station a cut runs into the band, square to the segment that holds it (2.6), to
   its own chain's far side (below) and 2 mm past it: its length is the distance to the first
   crossing of that far side at least `band_depth` less 2 mm away, at least `band_depth` and at most
   `2 × band_depth`, plus 2 mm; `band_depth` plus 2 mm where the far side is not crossed. So a station
   just before a convex bend, whose square cut meets the far side the next segment sets a little past
   `band_depth`, still divides its two cells. A station's cut stops sooner, 2 mm past the first
   stretch-end cut (the bisectors below) it crosses ahead of its station: past that bisector lie the
   next stretch's cells, which a square cut near a convex corner would otherwise divide across their
   depth (an L corner of a 50 m and a 90 m frontage, `cell_length` 11 m, `band_depth` 24 m: the
   long side's cells keep their whole depth; case `cells_corner`). Crossings are found in doubles,
   as the far side's. At a stretch end inside a chain the cut runs along the corner's
   bisector: the direction is the sum `s` of the two segments' unit inward normals, in doubles, and
   the length `min(2 / |s|, 2) × band_depth` plus 2 mm, to the corner of the band's far side. Cut
   ends are rounded to the lattice. Chain ends need no cut: the butt caps bound the band. Each
   chain's **far side**, its frontage offset `band_depth` inward, consecutive segments joined where
   their offset lines cross (bevelled past ratio 2; at a spur's tip, the two offsets' ends joined
   straight, through the tip), is built in with the cuts, so a cut never divides the cells of a
   frontage across a region between `band_depth` and `2 × band_depth` wide: what lies past both
   far sides is `rest`. **A region narrower than `band_depth` (Open).** Its far sides lie outside
   it, so they stop no cut: the cuts of its two frontages cross it whole, every strip between two
   cuts borders a station of each (`geom.cell_merged`), and two near cuts leave a sliver cell (a
   17 m wide rectangle fronted all round, `cell_length` 11 m, `band_depth` 24 m: 11 cells, 3
   merged, two of 12 m²; with `band_depth` 8 m, 12 cells, none merged). Owed: a far side at the
   middle between frontages (the skeleton of Vanegas et al.); until then a caller keeps
   `band_depth` under half the narrowest width it divides.
6. **Cells.** The band's outline and the cuts are built into a network with `clip` = the band (3.1,
   3.2). An area of it that borders the frontage between two stations of one edge side is that
   station's cell; when several areas take one station, they are joined into its one cell. An area
   that borders several such portions is one cell, takes the lowest (edge, side, station), and is
   reported by `geom.cell_merged`. An area that borders none is part of `rest`. An area borders a
   frontage only on the side the frontage faces: a spur's two sides run along one line, and the
   cells of each lie on its own side. A cut that meets another near a concave bend so splits cleanly instead of crossing.
7. **Short cells.** A cell is short when its frontage portion is shorter than `min_cell_length` or its
   area is under `min_cell_area`. Repeat until none can merge: take the first short cell in output
   order that has a neighbour on its edge side, and merge it into the one before it (the one after,
   if it is first); `geom.cell_merged` (info) names it. A short cell with no neighbour stays, with
   `geom.cell_small` (warning).

For each cell the kernel returns its region, its frontage `(edge, side)` or `outline`, its station
number, the `[from, to]` it covers along that edge (fractions of the edge), and its representative
point (3.2). `identity.md` names a cell from the first three.

`min_cell_length` above `cell_length`, or any input not above 0 (`min_cell_*` may be 0), is
`geom.bad_parameter`.

| Guarantee | Statement |
|---|---|
| Fronts | Every cell borders its frontage along at least `min_cell_length`, unless reported by `geom.cell_small`. |
| Closure | The cells' twice-areas plus `rest`'s equal the region's, on the noded band, exactly. |
| Disjoint | No two cells overlap; no cell crosses a network edge. |
| Local | A change to one frontage edge changes only the cells of that edge and of the frontage edges that share a chain corner with it. |

### 3.7 Mask to regions

Takes the values of a formula at the posts of a grid (`numbers.md` 4), as doubles, and returns the
region where the value is above 0. This is what `where` lowers to (vocabulary, Features).
`via-raster` evaluates the formula at the posts; `via-geom` turns the posts into the region. The rule
for the split: an operation whose output is a shape and whose guarantees are shape guarantees is in
`via-geom`; one whose work is a search over posts (3.11) is in `via-raster`.

Mechanism: marching squares at level 0 (Lorensen and Cline 1987, in two dimensions).

- A post exactly at 0 counts as not above.
- The crossing on each grid edge, between its two posts in post order `v0` and `v1`, is at fraction
  `f = v0 / (v0 - v1)` from the first post; its coordinate is `p0 + f * r` in millimetres, doubles in
  that order, then rounded to the lattice. Each grid edge's crossing is computed once and shared by
  the two cells beside it.
- A saddle cell is settled by the mean of its four posts, summed in post order: above 0 joins the
  high corners.
- A piece that reaches the grid's edge is closed along the extent's outline. The result is the same
  whole or windowed.
- The rounded rings are noded and rebuilt as a union (3.3), so the result is valid and settled.

| Guarantee | Statement |
|---|---|
| Every piece | Every connected piece is kept, with its holes. Nothing is kept "largest only" (R18). |
| Tiny pieces | A piece smaller than one grid cell (`resolution²`) is kept and reported: one `geom.mask_tiny` (info) per masked region, with the count and each piece's position. |
| Dropped | The only piece ever dropped is one whose rings have zero area after rounding to the 1 mm lattice: `geom.mask_dropped` (warning) with its position. |
| Empty | No post above 0 gives an empty region and `geom.mask_empty` (warning), with the formula's text and its least and greatest value over the posts. |
| Window | Masking a window gives the same rings, clipped to the window, as masking the whole (`numbers.md` 6). |

An empty mask is a warning, not an error: an edit elsewhere (land lowered, a condition no longer
met) can empty a derived region, and change is the main case (principle 4). It is still said
(principle 3). A feature that needs a shape from the empty region reports its own error (for
example, `on` it is `geom.no_length`).

Outlines are not simplified. A `where` region's outline is exactly the mask's.

### 3.8 Linear referencing

| Operation | Takes | Returns |
|---|---|---|
| `project` | A path, region or point, and points. | Per point: `t` (none on a point or a region of more than one ring), `d`, `sd` (2.6), and the nearest point on the line as a double pair in millimetres. |
| `interpolate` | A path or region, and `t` values. | Per `t`: the point, rounded to the lattice, and the bearing of travel there (2.6). |
| `substring` | A path or region, and `[from, to]`. | The path between them, in the source's direction (3.5 for `from >= to`). |
| `reverse` | A path. | The same points in the other order. |

`project`'s nearest point, `t`, `d` and `sd` are `numbers.md` 5.5's expressions, not restated here.
In short: the nearest segment `k` and whether the nearest point is a vertex or an interior are
decided exactly in integers (ties to the lower `t`); at an interior `t = (Ck + dot(u, w) / lk) / L`,
at a vertex `Pi` `t = Ci / L`, with `lk`, `Ck` and `L` as in section 1. Where this summary and
`numbers.md` 5.5 differ, `numbers.md` wins. `t` is the fraction, never the arc length.

`interpolate` follows `numbers.md` 5.5. `t` outside `[0, 1]`, or `nan`, is `geom.t_range`.
`interpolate(project(p))` lies within 1 mm of the exact nearest point; `project(interpolate(t))`
returns `t'` with `|t' - t| × L` at most 1 mm.

**How the nearest is found** (ours). The definition above names one segment: the least exact
squared distance, ties to the lower `k`. How an engine finds it is not part of the definition. A
field's posts are many points against one shape, so the engine files the segments by square cell
of the shape's box and searches outward from a point's cell in rings, stopping once the best exact
distance is strictly below the gap to every cell not yet searched; a segment beyond that gap is
strictly farther, so no tie is missed. Any search must return the scan's segment, foot and exact
distance, so `t`, `d`, `sd` and the nearest point are the same bits, and no key or vector changes
with it. Since M6 the engine takes many points by box instead: a `distance` field's window, or a
square tile of 32 m (ours) for any other field or measure over many posts. It keeps only the
segments that can be nearest to some point of the box and scans those for each point. The distance
to a segment is convex, so over the box it is greatest at a corner; `u`, the least over segments
of that greatest exact squared distance rounded up, bounds the nearest distance of every point of
the box, and a segment whose box lies farther than `u` from the box is strictly farther from every
point of it. The ring search searched outward to the far posts of a long shape, and for a window
cost more than the plain scan (city's 650 `distance` nodes, 18 node-seconds scanned, 64 by rings,
7 by box; M6, protocol in `examples/city/expect.toml`, M6 slice 2). The network's representative point (3.2) is likewise exact whatever is skipped: an
estimate below the floor of the greatest estimate so far is below the final floor too, so it is
left unmeasured.

### 3.9 Simplification (pointer)

No operation in M1 to M3 needs a simplifier: outlines are kept as made (3.7) and written shapes as
written. A topology-preserving one (Douglas and Peucker 1973, refused where a removal would make a
crossing, as JTS `TopologyPreservingSimplifier`) is specified when a case needs it. The likeliest is
the waterfront's wriggle (section 4); decided in M3.

### 3.10 Triangulation

Takes a region; returns triangles, for a region exported as a mesh (`build.md` 10). Needed from the
milestone in which `build.md` first exports a region as a mesh; its candidates are judged then
(section 6).

| Guarantee | Statement |
|---|---|
| Cover | The triangles' twice-areas sum to the region's, exactly, and no two overlap. |
| Vertices | Every vertex is a vertex of the region; no point is added (no Steiner points). |
| Order | Each triangle is counter-clockwise and starts at its least vertex; triangles are sorted by their three vertices in that order. |

### 3.11 Routing over a cost field (pointer)

Finding a least-cost course over a field is a search over posts (R23): it lives in `via-raster` and
its specification belongs with fields. Its answer enters the kernel as a path on the lattice, with the
source of the operation's caller. Where its chapter is written is (open): decided when M2 or M3 first
needs a found course (`fields.md` is the default). TODO's main ways "found in the order written"
need it with the network (3.2): a search that places junctions rather than joining by a discount.
That search is decided in M3, with the user syntax for networks.

### 3.12 Other forward pointers

| Item | Where it goes |
|---|---|
| A profile swept along a path (walls, rims, hedges) | (open): decided by the first milestone world with a wall along a path, M3 at the earliest. Likely a stroke (3.4) carrying a cross-section, owned with `surfaces.md`. |
| The waterfront that wriggles with every bend of the river (FINDINGS) | (open): whether `follows` takes a smoothing or a tolerance; decided by the M3 waterfront case. |

## 4. `follows`

`follows` is the written form of 3.5 (R18: "a path written along another feature"). It is accepted
from M1 (track A). Its syntax, decided here:

```toml
[feature.quay_walk]
kind    = "lane"
follows = { ref = "beck", span = [0.2, 0.7], offset = -9 }
```

| Key | Takes | Meaning |
|---|---|---|
| `ref` | name | The path or region it runs along, named whole. |
| `span` | 2 numbers | `[from, to]` along `ref`. Default `[0, 1]`. On a path `0 <= from < to <= 1`; on a region `from > to` wraps through `t = 0` and `from = to` is the whole outline (3.5). |
| `offset` | length | Metres to the left of `ref`'s travel, read by `numbers.md` 3.1; inward on a region. Default 0. |

`span` inside `follows` is accepted from M1, although `span` on a run waits for M3 (vocabulary 5).
The feature is a path, in `ref`'s direction. It moves when `ref` moves (`graph.md`). `follows`
excludes `at`, `on`, `path`, `region` and `where` (vocabulary 2.5), and does not take `smooth`
(2.4). A missing `ref` is `doc.missing_key` (`documents.md`). Whether `starts_at` and `ends_at` may be written with
it is (open): decided by the M3 waterfront case.

## 5. Worked example

Three guides (features without a kind), noded into a network. World extent `[0, 0, 100, 100]`,
resolution 0.5.

```toml
[feature.frame_a]
region = [[0, 0], [100, 0], [100, 100], [0, 100]]

[feature.cross_ew]
path = [[0, 50], [100, 50]]

[feature.cross_ns]
path = [[50, 0], [50, 100]]
```

Fine level: 9 vertices (four corners, four midpoints of the frame, the centre) and 12 segments
(8 on the frame, 2 on each path). Coarse level: 5 junctions (the four midpoints, where degree is 3,
and the centre, degree 4; the corners have degree 2 and one source) and 8 edges (4 on the frame, each
round a corner, 2 on each path). The network has 4 areas and 1 component: `9 - 12 + 4 = 1` and
`5 - 8 + 4 = 1`. Each area's twice-area is `2 × 50 000 mm × 50 000 mm = 5 000 000 000`; the four sum
to the frame's `20 000 000 000`, exactly.

| Area, in order (start vertex) | Representative point | `sides` |
|---|---|---|
| [0, 0] | [25, 25] | `cross_ew` right, `cross_ns` left, `frame_a` left |
| [0, 50] | [25, 75] | `cross_ew` left, `cross_ns` left, `frame_a` left |
| [50, 0] | [75, 25] | `cross_ew` right, `cross_ns` right, `frame_a` left |
| [50, 50] | [75, 75] | `cross_ew` left, `cross_ns` right, `frame_a` left |

Moving `cross_ns`'s top end to `[60, 100]` moves the crossing to `[55, 50]`: all four regions and
representative points change, and all four `sides` lists stay the same.

Adding a ring `pond_a`, `[[10, 10], [20, 10], [20, 20], [10, 20]]`, inside the first area: 13
vertices, 16 segments, 6 junctions (the ring's own at `[10, 10]`), 9 edges, 5 areas, 2 components;
`13 - 16 + 5 = 2` and `6 - 9 + 5 = 2`. The first area gains a hole and its twice-area falls to
`4 800 000 000`; the ring's area is `200 000 000`. The sum stays `20 000 000 000`, the frame's alone,
since the ring's component is not top-level. The first area's `sides` are unchanged.

## 6. Bake-off: crates or ours

R19 accepted that we write the network with areas and robust offsetting ourselves; the plan allows a
pure-Rust integer crate for region booleans and offsets where one passes on our fixtures. The
bake-off covers those two jobs only. Each candidate is judged by the guarantees of section 3,
byte-identical results on macOS and Linux (Windows pending until a push is allowed, as the plan
says), no non-finite value, licence (MIT or Apache-2.0), and time: the booleans and offsets of every
case below, together, in under 1 second in a release build on one thread on the M0 machine.

| Job | Candidate | What must be verified | Fallback |
|---|---|---|---|
| Booleans (3.3) | `i_overlay` 9.x (integer core, fill rules) | Exact on integer input; canonical output reachable; same bytes on both systems; idempotence; closure. | Ours, on our noding. |
| Offset and stroke (3.4) | `i_overlay` 9.x outline and stroke | Negative offsets on concave and holed regions; the three joins, the `miter-clip` limit as 3.4; arc count as 3.4; no self-crossing output. | Ours: raw offset then union with the non-zero rule, after Chen and McMains (2005). |
| Test oracle | `geo` | Used only in tests: area, length, Hausdorff distance, validity, line locate. Not for booleans: `geo`'s boolean operations are built on `i_overlay` since 0.29, so they would check the candidate against itself. | |

Booleans are checked against an independent oracle of our own: exact point-in-region tests at a grid
of lattice points and at every input vertex, slow and simple.

Written by us whatever the bake-off: exact predicates, snap-rounding noding with provenance, the
network and its areas, representative points, linear referencing and the local frame, curves,
`follows` (3.5), cells (3.6), the mask (3.7), canonical form. Triangulation's candidates (`i_triangle`,
`earcutr`, or ours by ear clipping on exact predicates) are judged when 3.10 is first needed.

**Fixtures** live in `spec/vectors/geometry/`, in integer millimetres, with names free of library
words, so no core crate names a library noun in a string (vocabulary 7). The conformance runner reads
the case list; no core crate names a fixture file.

`linework_a` and `course_a` are made from the committed `fixtures/kernel/townland/` (`streets.geojson`
and `river.geojson`) by a kept script, `spec/vectors/geometry/import.py`, never from `spike/out` or
`spike/build`. The import rule, for this oracle data only: each coordinate in metres is converted to
millimetres and rounded to the nearest integer, ties to even, once. Text a user writes is never
rounded (`numbers.md` 3.1).

| Fixture | From | Exercises |
|---|---|---|
| `linework_a` | `streets.geojson`, as paths with source ids. | Noding, network, Euler, closure, order-freedom, cells. |
| `course_a` | `river.geojson`, one path. | Stroke both sides with round joins and caps. |
| `cross` | Section 5. | Euler, closure, sides, cells. |
| `cross_pond` | Section 5 with `pond_a`. | Closure with a component inside an area. |
| `near` | Segments 1 mm apart, a vertex 1 mm off a segment, a crossing at a non-lattice point, a crossing on a hot pixel's edge. | Snap rounding, idempotence, the bound, the pixel convention. |
| `overlap` | Two paths sharing a stretch, one reversed; a path that doubles back on itself. | Shared segments naming both; `geom.overlap`, self-pairs included. |
| `dumbbell` | Two 20 m squares joined by a 4 m neck. | An inset that splits; `follows` that breaks. |
| `comb` | A ring with deep narrow teeth, and holes; settled and canonical as written. | Offsets both ways, mitre limit, booleans. |
| `bend` | A sine path, amplitude 10 m, wavelength 30 m. | `follows` on the inside and outside of bends. |
| `speckle` | A field with single posts above 0, a saddle, and a piece touching the grid's edge. | Every piece kept, tiny reported, saddle rule, edge closing. |

**Measures** against `fixtures/kernel/townland/reference.json`, under the fixtures README's rule
(read and explained, never tuned to): the area count and total area of `linework_a`'s network against
`faces` and `total_area_m2`; the area of `course_a` stroked at 8.5 m and 17 m on both sides, round
joins and caps, against the buffers there. The reference used 16 segments per quarter circle and we
use 5 mm chords, so the stroke areas differ by a small, explained amount.

## 7. The kernel command

Conformance runs through the command line only (R20). Networks and cells have no user syntax before
M3, so the kernel's cases, and `identity.md` cases 47 to 49, run through one command **(new)**, owed
to `build.md` 14:

```
via kernel OP --in FILE --out FILE
```

It runs one operation on the shapes in `--in` and writes the result to `--out`. It reads no world,
no store and no library, and writes nothing else, except a reproduction (below). It is a test
command: no build uses it.

### 7.1 Files

Both files are one JSON object. The input may be in any member order and spacing; the output is
canonical JSON (`documents.md` 8: members ordered bytewise, no whitespace, one closing line feed).

| Value | Written as |
|---|---|
| A coordinate, a length, a distance, a width, a depth | A JSON integer, millimetres. |
| A twice-area | A JSON integer, in full. It can pass 2^63 (section 1), so a reader parses it as a 128-bit integer. |
| A point | `[x, y]`, two integers. |
| A path | A list of points. |
| A ring | A list of points, the closing point not repeated. |
| A region | A list of pieces; each piece a list of rings, shell first, then holes. |
| A `t`, a `span` end, a field value, `d`, `sd`, a nearest point's coordinates | A double, as `numbers.md` 3.4 writes it. |
| A facing or bearing | A JSON integer, millidegrees (`numbers.md` 2). |
| No value (`t` on a point) | `null`. |

An input shape is an object with `source` (an id) and exactly one of `point`, `path`, `ring` or
`region`. Output lists are in 2.5's order; output shapes are in canonical form (2.5).

The input object:

| Member | Holds | Needed by |
|---|---|---|
| `@format` | `"via-kernel-1"`. | every OP |
| `shapes` | Input shapes, each with its source. | every OP but `mask` |
| `params` | The operation's parameters, named as in section 3 (below). | as the OP says |
| `grid` | `{ "extent": [x0, y0, x1, y1], "resolution": r }`, integers in millimetres (`numbers.md` 4). | `network`, `cells`, `mask`, `names` |
| `values` | The field's values at the grid's posts, doubles, in post order (`numbers.md` 4). | `mask` |
| `formula` | The formula's text, for `geom.mask_empty`. | `mask` |
| `points` | Points to project. | `project` |
| `t` | `t` values. | `interpolate` |
| `instances`, `footprints`, `edits` | Fixture instances, asset footprints and hand edits (7.3). | `names` |

The output object holds `@format` (`"via-kernel-1"`), `op`, the OP's result members, and
`diagnostics`: every record, in the form and order of `diagnostics.md`'s `--json` document. A
source id is not an entity address, so each record, and each of its `related` entries, has subject
`cli`, the `argument` location (`diagnostics.md` 1.2, 7) giving the index of the input file's path
after `--in`, `position` where there is one (in metres, as every record writes it, `diagnostics.md`
1.2), and the source id in `data.source` (no
member when the record has no source). On an error the result members are absent and `diagnostics`
is still written.

### 7.2 Operations

| OP | Section | `params` and other input | Result members |
|---|---|---|---|
| `read` | 2.3, 2.4 | One shape, `path` or `ring`, as written; `params.smooth` (default false). | `shape`: the shape as stored. |
| `noding` | 3.1 | `shapes`. | `vertices`; `segments`, each `{ "ends", "provenance" }`, an entry `{ "source", "segment", "t": [a, b], "reversed" }`. |
| `network` | 3.2 | `shapes`, `grid`; `params.clip` (a source id among `shapes`), `params.naming` (point sources). | `vertices`, `segments` as `noding`; `junctions` (points); `edges`, each `{ "ends", "path" }`; `areas`, each with the members of 3.2's table; `components` (a count). |
| `boolean` | 3.3 | Regions in `shapes`; `params.op`: `union`, `intersection` or `difference` (the first shape less the rest). | `region`. |
| `offset` | 3.4 | Regions; `params.distance`, `join`, `limit`. | `results`: one `region` per input. |
| `stroke` | 3.4 | Paths; `params.left`, `right`, `join`, `cap`, `limit`. | `results`: one `region` per input. |
| `follow` | 3.5 | One path or one-ring region; `params.span` (two doubles), `params.offset`. | `path`. |
| `cells` | 3.6 | `shapes` and `grid` as `network`; the region as `params.area`, an area's start vertex, or `params.region`, a region source; `params.frontage`, a list of `{ "source", "edge": [a, b], "side" }` (the edge by its two end junctions, least first; the side of that source's travel, as in `sides`) or `"outline"`; `params.cell_length`, `band_depth`, `min_cell_length`, `min_cell_area`. | `cells`, each `{ "region", "frontage", "station", "span", "point" }`; `rest`. |
| `mask` | 3.7 | `grid`, `values`, `formula`; `params.window` (`[i0, j0, i1, j1]`, posts `i0 <= i < i1` as `numbers.md` 6, default the whole grid). | `region`. |
| `project` | 3.8 | One shape; `points`. | `results`: per point `{ "t", "d", "sd", "nearest" }`, `d` and `sd` in metres, `nearest` in millimetres. |
| `interpolate` | 3.8 | One path or one-ring region; `t`. | `results`: per `t` `{ "point", "bearing" }`. |
| `substring` | 3.8 | One path or one-ring region; `params.span`. | `path`. |
| `reverse` | 3.8 | One path. | `path`. |
| `triangulate` | 3.10 | One region. From the milestone that needs it. | `triangles`. |
| `names` | `identity.md` 2, 4 | 7.3. | 7.3. |

`params.join` is `round`, `mitre` or `bevel`; `params.cap` is `butt` or `round`; lengths in
millimetres. A parameter missing where its section gives no default, or one the OP does not take,
is `cli.usage` naming its member path.

### 7.3 Names and hand edits

`names` serves `identity.md` cases 47 to 49. It runs `network` or `cells` and names what they make,
then holds the fixture instances by the hand edits, through the same code as a build
(`identity.md` 4.1).

| Member | Holds |
|---|---|
| `params.of` | `network` or `cells`; its other `params` are that OP's. |
| `params.producer` | The name of the producer of the network or cells, for selections (`on = { of, class, near }`). |
| `instances` | Fixture instances: `{ "name", "producer", "asset", "origin", "face", "footprint" }`; `footprint` a region in the world frame. |
| `footprints` | Each asset a pin may name, by name: its footprint as a region in the asset's own frame, placed by the pin's `at` or `on` and `face` (`numbers.md` 5.1). |
| `edits` | `{ "pin": { id: table }, "clear": { id: table } }`, each table exactly as `via merge` prints a `[pin.*]` or `[clear.*]` table (`documents.md` 8): lengths in metres as written. All edits, instances and producers are in one layer. There is no baseline, so `identity.clear_empty` is a warning. |

The result: `junctions`, `edges`, `areas` (and `cells` for `of = "cells"`), each entry its place
in that OP's output (a point, an edge's ends, an area's start vertex, a cell's frontage and station)
with its `name` by `identity.md`; and `held`, the instances after hand edits, each `{ "name",
"producer", "origin" }`, by name.

### 7.4 Status and reproduction

Exit status is `diagnostics.md` 7: 0 with no error; 1 with an error (`geom.*`, `num.out_of_range`
for an input coordinate out of range, `identity.*`); 2 for an unknown OP, a missing or unreadable
file, an input that is not this form, or a wrong `@format` (`cli.usage`, naming the argument or the
member path; nothing is written); 3 for `geom.internal`, which is an internal error.

`geom.internal` writes a reproduction in `diagnostics.md` 7's place and form. In a build the folder
is `<directory>.repro/<digest>/`, beside the build directory, `<digest>` the failing node's key.
Under `via kernel` it is `<out>.repro/<digest>/`, beside the `--out` file, `<digest>` the digest
(`graph.md` 3.1) of the input file's bytes. `data.repro` holds `<digest>`, never a path. The folder
holds one file, `<op>.json`: a complete `via kernel` input for the failing call, its whole batch
with sources and parameters, so that

```
via kernel <op> --in <folder>/<op>.json --out FILE
```

reproduces it. This closes the contents left (open) in `diagnostics.md` 7.

## 8. Networks and cells in text (M3)

Decided in M3 (R13, R33). Two tables declare the two makers of section 3 that text could not reach
before: `[network.<id>]` runs 3.1 and 3.2 over features the author selects, and `[cells.<id>]` runs
3.6 over regions. Neither knows a domain noun: a network is made of whatever paths and region
outlines its `from` selects, and what its areas and cells are for is said by the kind a library
gives them (`kind`). Both ids are in the world namespace (vocabulary 2.1), so `of` can name them
(`identity.md` 3). Both are makers, not producers: they place no instance, and a hand edit's `by`
never names one (`identity.not_a_producer`). Keys reused from elsewhere keep their one meaning
(principle 9): `within` is the region a thing is confined to, `spacing` the distance between
neighbours, `kind` what a feature does to fields.

### 8.1 `[network.<id>]`

| Key | Takes | Meaning |
|---|---|---|
| `from` | list of names and `kind:` selectors | Required. The features noded together, each one source (3.1). A name is a path or region feature's id or place name. A `kind:<k>` selector takes every path and region feature of that kind, written or returned by a step, and leaves the kind's point features out, without a diagnostic, as a selector leaves out what does not apply. A point feature named by id is `geom.network_point` (error). A name that is declared but is not a path or region feature (an instance, a pin, a step, a scatter, a network or a cells table) is `identity.unknown_place` naming it, at the key `from` (error). A feature selected twice is one source. A region feature is its outline, as a ring source (3.2: its inside is its left); a region of more than one ring (two pieces, or a hole) is `geom.many_rings` at the key (error): its rings have no one source each to be named by. |
| `within` | name | Optional. A region feature whose outline clips the network: 3.2's `clip`. Its outline is the source `#clip` (`identity.md` 2.3). A region of more than one ring is `geom.many_rings`; a name that is not a region feature is `identity.unknown_place`. |
| `kind` | name | Optional. The kind every area takes: each area is then a region feature of that kind in `features.json`, at the kind's authority, and `kind:<k>` in a cells table's `within` selects it. Its effects apply to each area as to a step's returned feature (Effects of a made region, below), and `kind:<k>` in a formula's shape selector counts the areas. Default none: the areas are guides. |

- **Sources by name.** Each source carries the selected feature's name: its id, or, for a feature
  a step returned, its produced name `<step>#<key>`. Junction, edge and area names digest those
  names (`identity.md` 2.3), so a step that keys its paths by place keeps every area name when a
  distant part changes, and a step that numbers its paths renames every area they bound.
- **What it makes.** Junctions, edges and areas, 3.2's guarantees held on every build, named
  `<id>#<key>` (`identity.md` 2.3). Its areas are region features in `features.json` under those
  names, of the kind `kind` gives (`build.md` 9.1). Junctions and edges are not features: they are
  selected (`on`, `within`) and reported, never written to `features.json`.
- **Meetings.** Every two inputs of one network own their crossings (2.7), so the network's own
  linework raises no `geom.unowned_crossing`.
- **Empty.** A network whose `from` selects nothing, or whose linework encloses nothing, is not an
  error: `geom.no_area` (warning), and any selection of its areas finds none
  (`identity.near_none`).
- **Effects of a made region (M3).** A made area or cell with a kind stands where a step's
  returned feature stands. It writes fields through its kind's effects at its table's layer and its
  kind's tier and defaults (`authority.md` 4), ordered among the other entries as any feature is
  (R21; its name orders it where names do). A shape selector counts it: `dist('kind:<k>')`,
  `inside`, `near` and the rest take every area and cell a table gives kind `k`, and a produced
  name (`blk#a…`) selects the one. Rest pieces take no kind (8.2), so they write nothing.
  It is outside the views of every step at its layer, as returned features are
  (`authority.md` 4.4, What counts as returned: the views leave out a step's or a maker table's
  `#` names of rank L and above, as members of a shape selector too: a field of layer L whose
  formula reads `kind:<k>` holds the made things in the build, and not in the view of a step of
  layer L, case `n14`). So no step reads what a maker of its layer makes, and the order
  is fields' head views, steps, makers, then the fields' build views: no cycle. A `where` region or
  a route's cost reads the head view, which holds no effect, and is placed before the makers run,
  so it does not see made things through a selector either.
  In the graph (`graph.md` 1) each made thing that writes is a `made` node reading its table's
  `op` node; a field chunk, a distance and a selector read the `made` nodes whose windows meet
  them. So moving one street re-runs its network and the chunks of the areas it bounds, and a
  chunk only unchanged areas reach is reused (case `n13_made_effects_local`). Cases `n12` to
  `n14`.

### 8.2 `[cells.<id>]`

| Key | Takes | Meaning |
|---|---|---|
| `within` | name, `kind:` selector or `{ of, class, near }` | Required. The regions divided, each on its own by 3.6: a network's id (every area of it); a region feature's id; `kind:<k>` (every region feature of that kind, a step's included, and every area or cell a table gives that kind); or one selected `area`, `cell`, `rest` or `piece` (`identity.md` 3). A region of more than one piece is `geom.many_rings`: divide its pieces one by one through a selection. |
| `frontage` | list of names and `kind:` selectors | Optional. The sources whose sides the cells face. On a network's area, every edge side on its shell or among its spurs whose edge lies on a selected source (an edge on several sources, by `follows` or overlap, when any of them is), the side facing the area. On a region feature, its whole outline when it is itself selected. Default: every source, so every side and every outline. |
| `spacing` | length | Required. 3.6's `cell_length`: a stretch of length `L` holds `floor(L / spacing)` cells, evenly. |
| `depth` | length | Required. 3.6's `band_depth`: how deep the cells reach from their frontage. |
| `min_length`, `min_area` | length; number | Optional, default 0. 3.6's `min_cell_length` (metres) and `min_cell_area` (square metres, read exactly in square millimetres): a cell shorter or smaller merges into its neighbour. |
| `kind` | name | Optional. The kind every cell takes, as on a network. Rest pieces are guides. |

- Each divided region gives its cells and its rest (3.6), named `<id>#c…` and `<id>#b…`
  (`identity.md` 2.3). Both are region features in `features.json`.
- `spacing`, `depth` not above 0, a minimum below 0, or `min_length` above `spacing`, is
  `geom.bad_parameter`, naming the key.
- A divided region whose outline encloses more than one area (a returned or selected region that
  touches itself) is `geom.self_crossing` at the key `within`, naming it; the others are divided.
- **Spurs (Open in text).** The kernel walks a spur's sides when they are in the frontage it is
  given (3.6 step 1; `via kernel cells` takes them, case `cells_spur`). The core does not pass them
  yet: its frontage of a network's area is the shell's edge sides only, so a dead-end line inside
  an area still gets no cells on its sides through `[cells]`. Owed in the core (M3 slice 2).
- A `frontage` that selects no side of any divided region divides nothing: `geom.frontage_none`
  (warning), naming the table. A single region with no frontage side is all rest, and says nothing:
  a region nothing faces is not a fault.
- Varied lengths along one stretch (a list in place of `spacing`) stay (open): Open.

### 8.3 In the graph, in layers and in steps

- **Nodes.** A network or a cells table is one `op` node (`graph.md` 1), depending on every feature
  its keys resolve to. A selection from it (`on`, `within`) depends on that node (`rules.md` 3.4).
  A distant edit re-runs it; the names it gives do not change (`identity.md` 2.3).
- **Layers.** A network or cells table belongs to the layer that declares it and reads downward, as
  any reference does (`documents.md` 5). It is not a step, so it reads the features steps of its own
  layer return; a step reads its output only from a layer above every step it depends on
  (`protocol.md` 3.3's stratum; else `graph.read_outside`).
- **Steps read them.** `build.features` with `select` naming a network or a cells table returns its
  areas, or its cells and rest, as a feature list with a `class` column (`protocol.md` 3.3). So a
  library places one thing per cell by reading the cells, never by cutting its own (R19b, R23).
- **Steps do not return them.** A step's document may not hold `[network]` or `[cells]`
  (`protocol.output_table`): a maker must have an id text can write in `of`, and a returned table's
  name would be produced. A library returns paths and regions of its kinds, and the world selects
  them by `kind:`. Reopened if `quay` shows the two tables are a burden on every town.

### 8.4 Example

A test library declares one kind, `rib`, with no effects. A step returns eight `rib` paths keyed by
place (`x20`, `y70`): at 20, 70, 120 and 170 m each way, each from 10 to 180 m across.

```toml
[network.grid]
from = ["kind:rib"]

[cells.plots]
within   = "grid"
frontage = ["kind:rib"]
spacing  = 25
depth    = 20

[instance.lamp]
asset = "lamp_post"
on    = { of = "grid", class = "edge", near = [45, 71], t = 0.5, offset = 3 }

[clear.one]
on = { of = "plots", class = "cell", near = [37.5, 30] }
```

The network has 32 junctions (16 crossings, 16 ends), 40 edges and 9 areas, one component:
`32 - 40 + 9 = 1`, and 16 dead ends (`geom.dead_end`, count 16). Each area is a 50 m square whose
four sides are one chain; each side holds 2 cells, cut at its middle and at the corners' bisectors,
each a trapezoid of 300 m², and the rest is the 10 m square at the centre: 72 cells and 9 rest
pieces. The lamp stands on the edge of `ribs#y70` between the junctions at `[20, 70]` and
`[70, 70]`, at its middle moved 3 m to the left of travel: `[45, 73]`. The clear removes every
instance whose origin lies in the cell holding `[37.5, 30]`, whichever producer in its layer or
below placed it (`identity.md` 4.3). Bending `ribs#y170` between two crossings renames no area,
rib, junction or edge, and no cell, rest piece or per-cell instance, the bent area's included: with
the vertex at `x = 45` lifted 2 m, its north side's station falls 19 mm before the vertex, and the
cut there, square to its segment, runs on to the far side the next segment sets (3.6 step 5), so
that side keeps its two cells and nothing merges. 3.6's Local guarantee allows the bent area's
cells to change shape; here their names hold.
The cases are `spec/cases/geometry/n*` and `identity.md` 12, 13, 23 to 26.

## Diagnostics

Each code carries the source id and a position where there is one; the caller adds subject, file
and line (section 3, `diagnostics.md`), and `via kernel` writes them as 7.1 says.

| Code | Severity | Meaning | Hint |
|---|---|---|---|
| `geom.range` | error | A point the kernel computed (offset, cut, stroke, join) is outside ±(2^31 - 1) mm. | Move the shape nearer the origin or reduce the distance. |
| `geom.repeated_point` | warning | A point repeats the one before it; it is removed. | Delete the repeat. |
| `geom.degenerate` | error | A path with fewer than two distinct points, or a ring with fewer than three or no area. | Add points or remove the feature. |
| `geom.self_crossing` | error | A written ring crosses itself, visits a point twice or doubles back; the first fault. | Reorder the points or split the region. |
| `geom.no_length` | error | `t`, `offset` or `follows` on a point, a zero-length thing or an empty region. (A run's is `run.no_length`, `rules.md` 5.) | Use `at`, or place on a path or region. |
| `geom.t_range` | error | A `t` or a `span` end outside 0 to 1, or `nan`. | Write a fraction from 0 to 1. |
| `geom.span_order` | error | A `span` on a path whose `from` is not less than its `to`. | Swap the two, or reverse the path. |
| `geom.many_rings` | error | `on`, `follows`, `interpolate` or `substring` on a region of more than one ring, which has no `t`. | Place on one piece's outline, or use `at`. |
| `geom.face_side` | error | `face = "toward"` or `"away"` with `offset = 0`, or on a ref with no length. | Give an offset, or use `along` or a bearing. |
| `geom.face_needs_on` | error | A `face` word without `on`. | Use a bearing in degrees. |
| `geom.face_unknown` | error | A `face` word that is not `along`, `toward` or `away`. | The nearest word. |
| `geom.smooth_not_path` | error | `smooth` on a feature without `path`. | Remove `smooth`. |
| `geom.meet_shape` | error | `starts_at`, `ends_at` or a way point in `path` names a whole path or a region. | Name an end, or place a point feature on it. |
| `geom.unowned_crossing` | warning | Two paths with kinds cross or touch with no owner of the meeting (2.7); names both. | End one on the other or on a named owner, or name it in an attribute the kind lists in `names`. |
| `geom.duplicate_source` | error | One batch holds two inputs with one source id. | Give each input its own source. |
| `geom.bad_parameter` | error | An operation parameter outside its range; names it and its value. | The range. |
| `geom.overlap` | info | Two sources, or one with itself, run along one another; the stretch and the ids. | |
| `geom.dead_end` | info | A network has junctions of degree 1; the count. | |
| `geom.no_area` | warning | A network encloses no area. | Close the linework, or add a `clip`. |
| `geom.offset_empty` | warning | An inset left nothing. | Use a smaller distance. |
| `geom.offset_split` | info | An inset split a piece; the count. | |
| `geom.follow_broken` | error | A path along another feature breaks into pieces; each gap and the widest offset that holds. | Use at most that offset, or a shorter span. |
| `geom.cell_none` | warning | A stretch of frontage is shorter than one cell. | Lower `spacing` (`cell_length`, 8.2), or lengthen the frontage. |
| `geom.cell_merged` | info | A cell was merged into its neighbour, or bordered several stations. | |
| `geom.cell_small` | warning | A short cell had no neighbour to merge into; kept. | Lower the minimum, or lengthen the frontage. |
| `geom.network_point` | error | A network's `from` names a point feature; a network is made of paths and region outlines (8.1; M3). | Name a path or region, or place the point `on` one. |
| `geom.frontage_none` | warning | A cells table's `frontage` selects no side of any region it divides, so nothing is divided (8.2; M3). | Select the sources the cells should face, or leave `frontage` out for every side. |
| `geom.mask_tiny` | info | A masked region has pieces smaller than one grid cell; the count and positions; kept. | |
| `geom.mask_dropped` | warning | A masked piece had no area on the lattice and was dropped. | |
| `geom.mask_empty` | warning | A mask is above 0 nowhere; the formula's text and its range of values. | Check the condition against the range. |
| `geom.internal` | error | A guarantee check failed inside the kernel (Euler, closure, validity). A defect of ours; exit 3. | Report it with the reproduction folder `data.repro` names. |

`geom.internal` names the operation in `data.op` and writes its inputs for a reproduction to
`<directory>.repro/<digest>/<op>.json`, beside the build directory, never inside it (`diagnostics.md`
7, `build.md` 13): a `via kernel` input file (7.4). `data.repro` holds `<digest>`.

## Conformance

Each line: input; action; what must hold. Positions in metres. Every error case exits with status 1.
An action that is a kernel operation, or `read` of a written `path` or `region`, runs through
`via kernel` (section 7), its output compared byte for byte, doubles bit for bit (R26). `placed`,
`merge`, `build` and a `read` of `on`, `face`, `starts_at` or `ends_at` run through a world with
`via build` or `via merge`.

- `via kernel frobnicate`; `cli.usage`, exit 2, nothing written. An input with `@format`
  `"via-kernel-0"`; `cli.usage` naming `@format`, exit 2.
- `cross`; network twice, the second input with members in another order and spaced; the two output
  files are identical and canonical (`documents.md` 8).
- `ford_frame` (2.6); project; every row of the table, bit for bit.

- `cross`; network; 9 vertices, 12 segments, 5 junctions, 8 edges, 4 areas, Euler at both levels,
  closure exact, areas, points and sides as in section 5.
- `cross_pond`; network; counts and twice-areas as in section 5; the first area's `sides` unchanged.
- `cross` with a path `spur_a` from `[0, 25]` to `[20, 25]`; network; the first area's `sides`
  unchanged, `spur_a` in its `spurs`; `geom.dead_end` with count 1.
- `cross` with a path `east_a` from `[100, 80]` to `[130, 80]`; move its end to `[130, 95]`; network;
  the first area is unchanged, byte for byte.
- Two crossing paths and no ring; network; `geom.no_area`.
- `linework_a`; network; Euler and closure hold; every area valid; no segment crosses another.
- `linework_a` noded; noded again; byte-identical.
- `linework_a` shuffled, and `cross` shuffled; network; same bytes as unshuffled.
- `linework_a`; network; area count and total area reported as measures against `reference.json`.
- `near`; noding; no crossing; every segment within the bound of its sources; idempotent; the point on
  a hot pixel's high edge snaps to the pixel above it.
- `overlap`; noding; shared segments name both sources with directions; one `geom.overlap` per
  stretch, one for the self-pair.
- A noding batch with two inputs of source `a`; `geom.duplicate_source` naming `a`.
- A ring written clockwise; read; stored counter-clockwise, `t = 0` at the first written point.
- A bow-tie ring; read; `geom.self_crossing` at the crossing.
- `region = [[0, 0], [10, 0], [10, 10], [10, 0], [0, 5]]` (a zero-width spike); read; `geom.self_crossing` at `[10, 0]`.
- `path = [[0, 0], [0, 0], [10, 0]]`; read; one `geom.repeated_point` at `[0, 0]`, a two-point path.
- `path = [[5, 5], [5, 5]]`; read; `geom.degenerate`. `region = [[0, 0], [10, 0], [20, 0]]`; read;
  `geom.degenerate`.
- A 10 m square at `[2147470, 0]`; offset +20 m; `geom.range`.
- `comb`, `A ∪ A`, `A ∩ A`, `A ∖ A`; booleans; `A`, `A`, empty, byte for byte.
- `comb` and a copy shifted by `[0.7, 0.3]`; booleans; both closure identities exact on the noded inputs.
- `comb`; offset -1.5 m with each join; valid region, no crossing; every edge not from a join on a
  line 1.5 m from an input segment, within 1 mm.
- A 10 m square; offset +2 m, `mitre`, limit 1.2; each corner cut square to its bisector at 2.4 m from
  the corner.
- `dumbbell`; offset -2.5 m; two pieces and `geom.offset_split` with count 2.
- A 4 m square; offset -3 m; empty and `geom.offset_empty`.
- A path with a 90-degree turn; stroke 3 m left and right, each join, `butt` and `round` caps; valid,
  arcs within 5 mm.
- A path with a 90-degree turn; stroke 0.001 m left and right, `round`; one chord per arc, no
  non-finite value.
- `course_a`; stroke 8.5 m and 17 m both sides, round; areas reported as measures against `reference.json`.
- `cells` with `cell_length = 0`; `geom.bad_parameter` naming it.
- `bend`; `follows` with offset +5 m and -5 m; one simple path each, every segment off the round joins
  at 5 m within 1 mm.
- `dumbbell` outline; `follows` with offset 3 m inward, `span = [0.3, 0.3]`; `geom.follow_broken`
  naming each gap and 1.999 m as the widest offset that holds.
- A path; `follows` with `span = [0.7, 0.2]`; `geom.span_order`.
- A point feature; `follows` it; `geom.no_length`.
- `cross`'s first area, frontage `(cross_ns edge [50, 0]-[50, 50], left)` and
  `(cross_ew edge [0, 50]-[50, 50], right)`; `cell_length` 10 m, `band_depth` 10 m,
  `min_cell_length` 5 m, `min_cell_area` 40 m²; cells; 10 cells, 5 per edge side, stations 0 to 4;
  8 squares of twice-area 200 000 000 and 2 triangles of 100 000 000 split by the bisector from
  `[50, 50]`; `rest` is `[0, 0]-[40, 40]` with twice-area 3 200 000 000; closure exact.
- Same, `min_cell_area` 60 m²; cells; 8 cells; two `geom.cell_merged`, each corner triangle merged into
  station 3 of its edge side.
- Same as the first cells case; move `cross_ew`'s west end to `[0, 52]`; only the cells of the
  `cross_ew` edge side and of the `cross_ns` edge side change.
- `linework_a`'s network; cells on each area in turn, frontage every `(source, side)` in its
  `sides` (each edge side on its shell, facing it), `cell_length` 20 m, `band_depth` 30 m,
  `min_cell_length` 8 m, `min_cell_area` 100 m²; Fronts holds (the gate's "every cell touches a
  street"): every cell borders a frontage edge along at least `min_cell_length`, or carries
  `geom.cell_small`; closure exact per area; areas too short for a cell give `geom.cell_none`.
- A frontage of 6 m with `cell_length` 10 m; cells; none, and `geom.cell_none`.
- `cross`'s first area, frontage `(cross_ns edge [50, 0]-[50, 50], left)` only, `cell_length` 50 m,
  `band_depth` 10 m, `min_cell_area` 600 m²; cells; one cell of 500 m², kept, `geom.cell_small`.
- `speckle`; mask; every piece kept, one `geom.mask_tiny` with the count of single-post pieces and
  their positions, the saddle joined by its mean, the edge piece closed along the extent.
- `speckle` masked by window, including windows on the grid's edge; compared with the whole, clipped;
  equal.
- Resolution 0.001, one post at `1e-9` and its neighbours at `-1`; mask; `geom.mask_dropped` at it.
- A field below 0 everywhere; `where`; empty region, `geom.mask_empty` with the formula and its range,
  exit status 0. A feature `on` that region; `geom.no_length`.
- A path; `project(interpolate(t))` for 1000 seeded `t`; `|t' - t| × L` at most 1 mm.
- A path; `interpolate(project(p))` for 1000 seeded points; within 1 mm of the exact nearest point.
- How the nearest is found (3.8): seeded walks of 20 to 2,000 segments, a comb whose teeth tie,
  one long segment among short ones and a 400-point ring; points on every vertex, inside the box
  and far outside it; the indexed search gives the scan's segment, foot and exact distance, and
  `project` the same bits (`crates/via-geom/tests/nearest_index.rs`). A window's segments: seeded
  walks of 2 to 400 segments (one in four on a 1 m lattice, so distances tie) and boxes of 0 to
  126 m near, across and far from them; for each corner and posts inside, the nearest among them
  is the scan's segment, foot and exact distance (`crates/via-geom/tests/exact_fast.rs`,
  `window_segments_keep_the_nearest`). Seeded stars, a square and a
  sliver; the representative point equals the unpruned search's.
- A 10 m square region and the point `[5, 2]`; `project`; `t = 0.125`, `sd = +2`, `d = 0`. The point
  `[5, -3]`; `sd = -3`, `d = 3`. The point `[10, 5]` on the outline; `t = 0.375`, `sd = +0`, `d = 0`.
- `on` that square at `t = 0.125`, `offset = 2`; placed; `[5, 2]`.
- A 3-point path with `smooth = true`; read; written points kept, each span in `ceil(c / 250 mm)`
  equal steps of the curve parameter (2.4; a step's chord can pass 250 mm where the curve bows).
- `smooth = true` on a region; read; `geom.smooth_not_path`.
- `on` a path at `t = 1`; placed; offset along the last segment's normal.
- ford's cottage `on` the beck at the `t` of its vertex `[140, 170]` (found by `project`), offset
  12 m; placed along the normal of the outgoing segment, toward `[160, 90]`, not the bisector.
- `on = { ref = "a", t = 1.5 }`; read; `geom.t_range`. `t = nan`; read; `num.not_finite`, since a non-finite number in text is refused at read (`documents.md` 2, `numbers.md` 3.3) and never reaches the kernel.
- A change writing only `on = { t = 0.8 }`; merge; `doc.missing_key` naming `ref`.
- A region from `where` of two pieces; a feature `on` it; `geom.many_rings`. `project` on it; `d`
  and `sd` to the nearer ring.
- `on` a point feature with `t = 0.5`; read; `geom.no_length`.
- `face = "toward"` with `offset = 0`; read; `geom.face_side`. `face = "awya"`; `geom.face_unknown`
  with hint `away`. `face = "along"` with `at`; `geom.face_needs_on`.
- An instance `on` a mount with `face = "along"`; placed; facing equal to the mount's.
- `ends_at` naming a region feature; read; `geom.meet_shape`.
- Two paths with kinds that cross, neither ending at the other; build; `geom.unowned_crossing` at the
  crossing, naming both. ford; build; none.
- (`unowned_crossing_hint`, M7) The same; the hint names both ways to own it, an end named on the
  other and an attribute the kind lists in `names` (read from the diagnostics document).
- (`weir_owns_crossing`, M7) A kind of the world's own (a fresh agent's weir) across lib/water's
  beck, `names = ["crosses"]` and `crosses = "river"`; build; no `geom.unowned_crossing`, the crest
  and the level pond upstream exact; without `names`, `geom.unowned_crossing`.
- `comb`; triangulate (when 3.10 is first needed); twice-areas sum exactly, no added vertex,
  counter-clockwise, in the order of 3.10.
- Every case above; on macOS and Linux (Windows once a push is allowed), at 1 and 8 threads; same bytes.

**Networks and cells in text (M3, section 8).** Through `via build`; the grid is 8.4's, the fixture
`spec/vectors/cells/grid/`. Case folders `n01` to `n11`.

- (`n01_step_paths`) A step returns the eight `rib` paths; `[network.grid] from = ["kind:rib"]`;
  build; 9 area features named `grid#a…`, the 8 paths under `ribs#`; `geom.dead_end`; no
  `geom.unowned_crossing`, no `geom.no_area`.
- (`n02_grid_euler`) The same linework as `via kernel network` input; 32 junctions, 40 edges, 9 areas,
  1 component; Euler at both levels and closure exact.
- (`n03_area_names_distant`) Section 5's cross and a ring `far` east of it, by name in `from`; move
  one vertex of `far`; 5 areas before and after, every area name unchanged.
- (`n04_cells_counted`) A 100 by 60 m region feature divided with `spacing = 20`, `depth = 15`,
  frontage left to its default; 16 cells (5, 3, 5, 3) and 1 rest piece; no `geom.cell_none`.
- (`n05_frontage_subset`) Section 5's cross, `within` its first area by place, `frontage` the two
  paths only, `spacing = 10`, `depth = 10`; 10 cells and 1 rest piece, as the kernel's first cells case.
- (`n06_frontage_none`) A `frontage` naming a path on no side of any divided area;
  `geom.frontage_none` naming the table, no cell, exit 0.
- (`n07_network_point`) `from` naming a point feature; `geom.network_point`, exit 1.
- (`n08_on_generated_edge`) On the grid, an instance `on = { of = "grid", class = "edge", near = [45, 71],
  t = 0.5, offset = 3 }` stands at `[45, 73]`; one `on = { of = "ribs", class = "feature", near =
  [100, 121], t = 0.5, offset = -2 }` stands at `[95, 118]` (R13's third obligation).
- (`n09_within_made_kind`) A region 80 by 40 m and a path across its middle, `[network.zz]` with
  `kind = "zone"`, `[cells.c] within = "kind:zone"`, `spacing = 20`, `depth = 10`; 2 areas written
  with kind `zone`, 16 cells and 2 rest pieces (8.2: a table's areas taken by kind).
- (`n10_network_clip`) Two crossing paths clipped by a 60 m square; 4 areas. The same paths with no
  `within`; one `geom.no_area` for that network only (8.1, Empty).
- (`n11_maker_faults`) `spacing = 0`, `depth = -1`; two `geom.bad_parameter`, one per key. `from`
  naming an instance; `identity.unknown_place` naming it. A network `kind` with effects is no
  fault: no `doc.not_yet` (8.1, Effects of a made region). Exit 1.
- (`n12_made_kind_effects`) A region 80 by 40 m and a path down its middle, `[network.blk]` with
  `kind = "block"` (paints `paved` within 2 m), `[cells.c] within = "blk"`, `spacing = 20`,
  `depth = 10`, `kind = "lot"` (writes `tag:lot` = 1), and a field `gap = dist('kind:block', 20)`:
  the ground at `[30, 30]` is all `paved` (plane 1 is 255) and at `[5, 5]` all grass; `gap` is 0
  at `[30, 30]` and 5 at `[5, 30]`, `[95, 30]` and `[50, 55]`; the `tag:lot` field is 1 at
  `[15, 30]` (a cell) and 0 at `[30, 30]` (the rest); no `doc.not_yet` (8.1).
- (`n13_made_effects_local`) A region `[4, 188]` by `[4, 60]` cut by streets `a` (x = 70) and `b`
  (x = 150) into three `block` areas, 64 m chunks: the first build runs `op:blk` and chunk 0 of the
  ground; moving `b` to x = 154 runs `op:blk` and the chunks at 64 and 128, and not chunk 0, which
  only the unchanged west area reaches (8.1, graph.md 1 `made`).
- (`n14_made_outside_own_view`) The `n12` areas at the world's layer, `gap = dist('kind:block',
  20)`, and a worker step at the same layer reading `gap`: the build's `gap` is 0 at `[30, 30]`;
  the step reads 20 there, since the areas stand outside its view (8.1, `authority.md` 4.4).
- Owed (M3 slice 2): `geom.many_rings` on a `from`, `within` or divided region (only a `where`
  region can have two rings), `protocol.output_table` for a returned `[network]` or `[cells]`,
  `of` a cells table with no `class`, `min_length` and `min_area` merging through text, and a rest
  piece's `kind` null in `features.json`.

`geom.internal` cannot be reached from valid input; it is tested by unit tests with an injected fault.

## Changed from the spike

- The spike used Shapely (GEOS) in floating point with repairs (`buffer(0)`); v1 uses integer
  millimetres and exact predicates, and rejects a self-crossing ring instead of repairing it.
- Paths were resampled every 0.25 m always; v1 keeps written points and samples only a smoothed path.
- `smooth` defaulted to `true`; v1 defaults to `false`.
- `on` a region offset outward; v1 offsets inward, left of counter-clockwise travel (vocabulary).
- A `where` region kept its largest piece silently and was simplified to two posts; v1 keeps every
  piece, reports tiny ones, and does not simplify.
- Areas between streets and lots existed only inside two step tools, numbered in the order made;
  v1 has the network, areas and cells in the kernel, with provenance for names by place (R13).
- A waterfront was written as a region and offset by a tool; v1 has `follows` (R18).
- `ellipse` is gone (vocabulary 6).

## Open

- **`i_overlay` for booleans and offsets**: decided by the bake-off (section 6). In M1 the
  fallback (ours) is built and passes the section's cases; `i_overlay` was not fetched (no download
  was approved), so the bake-off itself is still owed.
- **Triangulator**: decided when `build.md` first exports a region as a mesh.
- **`starts_at` and `ends_at` with `follows`**, and smoothing on `follows`: decided by the M3
  waterfront case.
- **Naming points** for a network declared in text (an area named after a feature inside it):
  none in v1 (3.2, 8.1); reopened by `quay` and the districts library if a case needs one
  (`identity.md`, Open).
- **Cell lengths from a list** (varied cells along one stretch) rather than even division: decided by
  the M3 `quay` world.
- **A profile swept along a path**: decided by the first world with a wall along a path (3.12).
- **Where routing over a cost field is specified**, and the junction-placing search for main ways:
  decided when M2 or M3 first needs a found course (3.11).
