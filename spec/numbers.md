# Numbers

This chapter fixes what every number in a world means: its unit, its axes, how decimal text becomes
an exact value, which frame it is measured in, how a place is measured from a feature, where
randomness comes from, and how arithmetic is done. Its rules are what let the same text build to the
same bytes on every system (principle 7, R24, R26).

Words are from `vocabulary.md`. Keys this chapter owns there: `extent`, `resolution`, `seed`. The
words `post`, `lattice` and `halo` are this chapter's (flagged for `vocabulary.md`, which does not
yet register them). The nouns `frame` and `feature` are defined in `geometry.md`; this chapter gives
the arithmetic of frames and of measures from a feature, and `geometry.md` says where each is used.
Where this chapter and `geometry.md` give the same rule, the rule here is the one to implement, and a
difference is a defect to fix in the other chapter.

## 1. Units and axes

| Quantity | Unit | Notes |
|---|---|---|
| Length, plan and height | metre | Text is decimal metres; the core holds plan geometry in integer millimetres (section 3). |
| Angle in text | degree | The core holds facings in integer millidegrees (section 2). |
| Fraction along (`t`) | none | 0 to 1. |
| Scale | none | A double factor, greater than 0. |
| Field values | as the field's `unit` | `fields.md`. |

Axes: `x` east, `y` north, `z` up. The frame is right-handed. Plan geometry is `x` and `y`; height
is `z`. Where `z = 0` lies is the world's datum (`geometry.md`). The loader maps these axes to the
engine's (`build.md`). This convention is ours.

## 2. Facing

`face` written as a number is a bearing: degrees clockwise from north, so 0 is north, 90 east, 180
south, 270 west. There is no other angle convention in text; the word yaw is not used.

- The text is read exactly (section 3.2) to integer millidegrees, then normalised to
  `[0, 360000)` by Euclidean remainder. `face = -90` and `face = 270` are the same facing.
- A facing computed by the core (for example `face = "along"`, `geometry.md`) is the bearing of a
  direction `(dx, dy)` of integer millimetres:

  ```
  m = round(atan2(dx as f64, dy as f64) * K), then normalised
  ```

  `atan2` is `libm`'s (section 8). `K` is one double constant, `180000.0 / PI` computed as one
  correctly rounded division, with `PI = 0x400921FB54442D18`; its bits are in section 10. `round` is
  to the nearest integer, ties to even. The argument order is `(dx, dy)`: a bearing measures from
  north. A zero direction never reaches this: a path has no zero-length segment (`geometry.md` 2.2,
  `geom.repeated_point`), `t` on a thing of no length is `geom.no_length`, and `toward` or `away`
  with `offset = 0` is `geom.face_side`.
- Two facings are equal when their millidegrees are equal. The change report's "moved in facing"
  means exactly this (`vocabulary.md`, change report).
- Sine and cosine of a facing `m` (millidegrees) are taken by quadrant, so the four axis directions
  are exact: `q = m div 90000`, `r = m mod 90000`, `a = (r as f64) * (PI / 180000.0)`, `s = sin(a)`,
  `c = cos(a)`; then quadrant 0 gives `(s, c)`, 1 gives `(c, -s)`, 2 gives `(-s, -c)`, 3 gives
  `(-c, s)`. Zero results are `+0`. `PI / 180000.0` is one double division; its bits are in section
  10. This reduction is ours.

## 3. Decimal text to exact numbers

`documents.md` hands this chapter the literal text of every number (its normalised text: `_` and a
leading `+` dropped, hexadecimal, octal and binary integers in decimal digits). No number passes
through a TOML library's binary float first. A reader that rejects or alters a literal such as
`1e400` before this chapter sees it is a defect of the reader.

### 3.1 Lengths and the lattice (R24)

A **length** is a plan coordinate or a plan distance in metres: a point's `x` and `y`, an `extent`,
an `offset`, a `spacing`, a `radius`, a `margin`, a `resolution`. The core holds it as a signed
integer number of millimetres. The set of such values is the **lattice**. A key that takes lengths
says so in its Takes column in `vocabulary.md` with the type `length` (or `point` and lists of them),
as against `number`, which is a double (3.3).

A length is read from its text:

1. Reject `inf` and `nan` in any sign (`num.not_finite`).
2. Read sign, digits before the point, digits after it, and the exponent (TOML allows `1.5e2`). The
   exact value is `N * 10^e` for an integer `N`. If `N = 0`, the value is `0` whatever `e` is.
3. Move trailing zeros of `N` into `e`. If `e + 3 < 0`, the value is not a whole number of
   millimetres: `num.too_precise`. It is never rounded. Trailing zeros are allowed: `1.2340` is
   `1234`.
4. If the digit count of `N` plus `e + 3` exceeds 10, the value is out of range
   (`num.out_of_range`), found before any power of ten is formed. Otherwise the millimetres are
   `N * 10^(e + 3)` in 64-bit integers.
5. The result must lie in `[-(2^31 - 1), 2^31 - 1]` millimetres, about plus or minus 2,147 km
   (`num.out_of_range`). The bound leaves room for exact predicates in 128-bit integers
   (`geometry.md`); worlds past a few kilometres are set aside (doctrine section 7) in any case.
6. `-0` and `-0.0` are `0`. The lattice has no signed zero.

| Text | Millimetres | Note |
|---|---|---|
| `12.5` | `12500` | |
| `3` | `3000` | A TOML integer is metres too. |
| `0.001` | `1` | The finest step. |
| `1.005` | `1005` | Read through a double, `1.005 * 1000` is `1004.9999999999999`. |
| `1.2340` | `1234` | Trailing zero allowed. |
| `1_000.25` | `1000250` | |
| `2.5e-3` | rejected | `2.5` mm is not whole: `num.too_precise`. |
| `1e3` | `1000000` | |
| `0.0005` | rejected | `num.too_precise`. |
| `2.0000000000000001` | rejected | `num.too_precise`. Through a double this is `2.0`, so a float reader would accept `2000`. |
| `-0.0` | `0` | |
| `0e-500`, `0.0e999` | `0` | Zero with any exponent. |
| `1e400` | rejected | `num.out_of_range`, without forming `10^400`. |
| `2147483.648` | rejected | `num.out_of_range`. |

**Computed positions.** A position the core computes (a point at `t`, an offset, a point in a turned
or scaled frame, a curve sample) is computed in doubles in millimetres, by the expressions written in
sections 5.1 and 5.5, in the order written. It enters the lattice by one rounding to the nearest
integer, ties to even, at the end of that expression. A rounded result outside the range of step 5 is
`geom.range` (`geometry.md`): `num.out_of_range` is for text, `geom.range` for constructions.

Heights are not on the lattice. A height is a field value or a value computed from one (a double,
section 8). Whether bounded surfaces at fixed levels put `z` on the lattice is (open), below.

### 3.2 Angles

A `face` number is read by the same steps with `10^(e + 3)` giving millidegrees: at most three
decimals of a degree, never rounded (`num.too_precise`), then normalised (section 2). Angles are not
bounded before normalisation, but the integer must fit in 64 bits (`num.out_of_range`; the digit
test of step 4 uses 18 in place of 10).

### 3.3 Other numbers

A number that is not a length or a facing (a `t`, a density, a scale, a constant in a formula) is a
double: the nearest IEEE 754 binary64 value to the literal's exact decimal value, ties to even. No
locale is involved; the decimal mark is always `.`. A TOML integer in such a key is the same double.
A literal whose nearest double is infinite (`1e400`) is `num.out_of_range`; one that rounds to zero
is `+0`. An `integer` key (`count`, `order`, `rank`) takes a TOML integer only; `2.0` there is
`doc.type` (`documents.md`). The world `seed` is section 7.1.

Two such numbers are read exactly instead, as a length is (3.1) but in millionths: a scatter's
`jitter` and an asset choice's `weight` (`rules.md` 7.1 and 7.4). `0.8` is `800000`; more than six
decimals is `num.too_precise`. They are integers from then on, because they feed integer draws.

### 3.4 Numbers written out

Where the build writes a number as text (`build.md` decides which files; `documents.md` for the
merged document):

| Value | Written as | Examples |
|---|---|---|
| A double | Exactly ECMAScript's Number-to-String (ECMA-262, `Number::toString` with radix 10): the shortest decimal that reads back to the same bits, no exponent for magnitudes in `[1e-6, 1e21)`, otherwise `d.ddde+N` or `d.ddde-N`. Rust's own formatting and plain Ryu (`3.0`, `1e21`) differ from it and are not used as they are. | `3.0` as `3`; `0.1` as `0.1`; `1e21` as `1e+21`; `1e-7` as `1e-7`; `5e-324` as `5e-324`; `1.23e-18` as `1.23e-18`. |
| A length | Exact metres from the integer millimetres: the point three digits from the right, trailing zeros and a trailing point dropped. Never through a double. | `1005` as `1.005`; `3000` as `3`; `-250` as `-0.25`; `0` as `0`. |
| A facing | Exact degrees from the integer millidegrees, the same way. | `270000` as `270`; `12345` as `12.345`. |
| An `integer` key | Decimal digits. | `16`. |
| The world `seed` | A JSON string of 16 lower-case hexadecimal digits, never a JSON number, which cannot hold it. | `0` as `"0000000000000000"`. |

The merged document (`documents.md` 8) writes lengths and facings as exact metres and degrees by
this table, as JSON number tokens, and the world `seed` as the string. Where a 64-bit integer is written as
bytes, it is little-endian.

## 4. The world frame and its posts

The world frame is the plane `x`, `y` with `z` up. A world's fields are sampled on a grid in it:

| Key | Takes | Rule |
|---|---|---|
| `extent` | 4 lengths | `[x0, y0, x1, y1]`. `x1 > x0` and `y1 > y0`, else `num.extent_empty`. Any other form is `doc.type`. |
| `resolution` | length | Post spacing `r`, at least 1 mm (`num.resolution`). Default `0.5` (500 mm) when not written (`documents.md` 1). The default is held to the same rules as a written value, so an extent that is not a whole multiple of 0.5 m is `num.extent_not_multiple`. ford writes none and builds at 0.5 m. |

- `x1 - x0` and `y1 - y0` must each be a whole multiple of `r`. Otherwise `num.extent_not_multiple`.
  Its hint keeps `x0` and `y0`, and gives `x1` and `y1` rounded down and up to multiples of `r`; it
  also gives the resolutions nearest `r`, below and above, that divide both widths (divisors of their
  greatest common divisor, in millimetres). Nothing is rounded. The hint is written in metres, and a
  rounded-up `x1` or `y1` beyond the lattice range (3.1) is left out, as is a resolution with none.
- A **post** is a sample point of the grid. There are `nx = (x1 - x0) / r + 1` posts along `x` and
  `ny = (y1 - y0) / r + 1` along `y`. Both edges carry posts.
- `nx * ny` is at most `2^26` (67,108,864) posts, else `num.too_many_posts`, checked before anything
  is allocated. The limit is provisional, to be replaced by the M0 probe (Open).
- Post `(i, j)`, with `0 <= i < nx` and `0 <= j < ny`, is at `(x0 + i * r, y0 + j * r)` exactly, in
  millimetres.
- A formula reads a post's coordinates as doubles in metres: `x = (x_mm as f64) / 1000.0`, one
  correctly rounded division. So `x` at 12,345 mm is the double nearest 12.345.
- Post order, wherever posts are listed or stored, is row by row from the south: `j` outer, `i`
  inner. Index `k = j * nx + i`.
- How a value between posts is read is `fields.md`.

## 5. Frames, and measures from a feature

A **frame** is an origin, a facing and a scale in its parent frame, and nothing else. The world frame
is the root; it has no parent. Every other frame is placed in one parent, so the frames form a tree,
and a reference that would make a cycle is a graph error (`graph.md`). The `t`, `d` and `sd` a
feature gives (5.5) are measures, not a frame.

### 5.1 A placed frame

A placed frame has, in the world, an origin `(ox, oy)` that is a lattice point, a facing `m` in
millidegrees and a scale `k`, a double greater than 0 (default 1). Its axes: `x` to its right, `y`
forward (the direction it faces), `z` up. With `face = 0` and `k = 1`, a placed frame's axes are the
world's.

A point `(lx, ly)` written in the frame, in millimetres, is in the world, with `(s, c)` the sine and
cosine of `m` (section 2):

```
a  = (lx as f64) * k
b  = (ly as f64) * k
px = ox + round((a * c) + (b * s))
py = oy + round((-(a * s)) + (b * c))
```

Each product and sum is one double operation in the order written; `round` is to the nearest
integer, ties to even, once per written point. Scale is applied before the turn. A height `z` written
in the frame is multiplied by `k` (a double, not rounded). A facing written in the frame is `m` plus
the local facing, modulo 360000. At the four axis facings with `k = 1` the result is exact. This is a
plane rotation by a bearing; the quadrant reduction, the order and the rounding rule are ours.

### 5.2 Instance frame

An instance's frame has the instance's resolved place as origin (rounded once, 5.5), its resolved
facing, and its `scale`. An asset's own frame is its instance's frame: an asset's front looks along
its `+y` (`assets.md` says how a pack declares a file whose axes differ). Set items, mounts and
footprints are written in it and taken to the world by 5.1, so a `scale` moves them as `rules.md`
says (r05: `scale = 2` doubles item positions).

### 5.3 Import frame

An import's frame is placed by the import's `at` and `face`; inside it, the imported world's own
world frame holds unchanged. The imported world is built in its own frame, with its own extent,
resolution and world seed, so moving or turning the import never changes what the imported world
makes, its noise included. Lesson (spike FINDINGS, "Parts"): a piece placed in a world took the
world's seed and coordinates, and moving it changed its noise. How the import's fields are
resampled onto the parent's posts is `fields.md`; its datum and lift, `geometry.md`. An import placed
with `on` (an interior on a mount, `vocabulary.md` section 6) resolves to an origin and facing as an
instance does (`composition.md` 3). Whether an import takes `scale`:
no; a world is built at its own size. Not in M1 (`vocabulary.md` section 5).

### 5.4 Nested frames

A frame `G` written inside a placed frame `F` (a set inside an instance, a set inside a set) at
local origin `(gx, gy)`, local facing `g` and local scale `kg` is itself a placed frame:

- its origin is `(gx, gy)` taken to the world by `F` (5.1), so every frame origin is a lattice
  point, rounded once when the frame is placed;
- its facing is `(m_F + g) mod 360000`, exact;
- its scale is `k_F * kg`, one double multiplication, from the root outward.

A point written in `G` is then taken to the world by 5.1 with `G`'s origin, facing and scale: one
rounding per written point, however deep. The order of operations is fixed, so the result is the same
bits everywhere. Ours.

### 5.5 Measures from a feature

Every feature gives each lattice point `p` three measures. Formulas read them as `t`, `d` and `sd`
(`formulas.md` 6.5); `on = { ref, t, offset }` uses the same quantities the other way (`geometry.md`
2.6). All lengths below are millimetres, and a quantity written `X as f64` is converted once, to the
nearest double.

| Feature | `t` | `d` | `sd` |
|---|---|---|---|
| path | Arc-length fraction of the nearest point on the path, from its start. | Distance `e` to the nearest point on the path. | `+e` left of travel, `-e` right. |
| region | Arc-length fraction of the nearest point on the outline, from the first written point, counter-clockwise. On several rings, `geometry.md` 2.6. | 0 inside, outline included; outside, the distance `e` to the nearest point on the outline (any ring). | `+e` inside, `-e` outside, with `e` the distance to the outline. |
| point | 0 in a formula (`formulas.md` 6.5); `geom.no_length` for `on` with `t`. | Distance `e` to the point. | `-e`. |

So `d = abs(sd)` on a path and a point, and `d = max(-sd, 0)` on a region: `d` is the distance from
the feature, and a post inside a region is on it. A formula that wants the depth inside a region
writes `max(sd, 0)`. Where `e = 0`, `sd = +0`. This agrees with `geometry.md` 2.6: a bounded effect
cut at `d > radius` (`fields.md` 5.3) keeps a region's whole interior, and a mask on `d` levels the
whole footprint, not a ring at its edge. A point's `t = 0` lets one kind's formula serve point and
path features alike (`formulas.md` 4.2).

- **One sign.** `sd` is positive on the side a positive `offset` goes to. For a region read
  counter-clockwise, left of travel is inside, so a place at `t`, `offset = o` near the outline has
  `sd = o`. This is the reverse of the usual signed-distance convention for regions (negative
  inside); it is chosen so that `sd` and `offset` are one quantity (principle 9). Ours.
- **Vertices and lengths.** A path is vertices `P0 .. Pn`; a region's ring is read with its closing
  segment, counter-clockwise, starting at its first written point (a ring written clockwise is
  reversed and keeps that point as `t = 0`, `geometry.md` 2.2). Segment `k` runs from `Pk` to
  `Pk+1` with integer direction `(dxk, dyk)`. Its length `lk` is `geometry.md`'s rule: the exact
  squared length `dxk*dxk + dyk*dyk` in 128-bit integers, converted to the nearest double once, then
  `sqrt`. Cumulative lengths are doubles: `C0 = 0.0`, `Ck+1 = Ck + lk`, summed in order. `L = Cn`.
  A smoothed path (`smooth`) is first made a lattice polyline (`geometry.md` 2.3); all of this then
  applies to that polyline.
- **The segment holding `t`.** With `s = t * L` (one double multiplication), the segment is the one
  with `Ck <= s < Ck+1`, compared as doubles; ties go to the later segment, so a vertex is held by
  its outgoing segment. `t = 1` is held by the last segment. A path has no zero-length segment
  (`geometry.md` 2.2), so no segment is skipped.
- **The point at `t`, with `offset`.** On segment `k`, with `o` the offset in millimetres (a length):

  ```
  f  = (s - Ck) / lk                        (f = 1.0 when t = 1)
  qx = (xk as f64) + (f * (dxk as f64))
  qy = (yk as f64) + (f * (dyk as f64))
  nx = (-(dyk as f64)) / lk
  ny = (dxk as f64) / lk
  px = round(qx + ((o as f64) * nx))
  py = round(qy + ((o as f64) * ny))
  ```

  One rounding, at the end, for `t` and `offset` together. `face = "along"` is the bearing of
  `(dxk, dyk)` of the same segment (section 2). The same construction with `s` given directly is the
  point at arc length `s` (used by section 6).
- **The nearest point.** For each segment, the nearest point to `p` is its start vertex
  (`dot(u, p - Pk) <= 0`), its end vertex (`dot(u, p - Pk) >= dot(u, u)`) or its interior, with
  `u = (dxk, dyk)`, decided in integers. The squared distances, `dot(w, w)` to a vertex and
  `cross(u, w)^2 / dot(u, u)` to an interior, are compared exactly as rationals in integer arithmetic
  (`geometry.md` 1, Predicates). A tie goes to the lower `t`: the lower segment index, and on a
  region `P0` as `t = 0`, never `t = 1`.
- **`e` and `t`.** At an interior of segment `k`, with `w = p - Pk`:
  `e = (((abs(cross(u, w))) as f64) / lk) / 1000.0` and `t = (Ck + ((dot(u, w) as f64) / lk)) / L`.
  At a vertex `v = Pi`, with `w = p - v`: `e = (sqrt((dot(w, w)) as f64)) / 1000.0` and
  `t = Ci / L`. `cross` and `dot` are exact in 128-bit integers. `e` is in metres. `d` and `sd`
  follow from `e` and the side by the table above.
- **The side of a path, exact.** The sign of `sd` on a path is found in integers, never with a
  tolerance (R24). At a segment interior: the sign of `cross(u, p - Pk)`. Before the start or beyond
  the end: the sign of `cross(u, p - v)` with `u` the first or last segment and `v` the end vertex.
  At an interior vertex `v` with incoming direction `a` and outgoing `b`: `p` lies in the vertex's
  outer cone, on the convex side of the turn, so the sign is minus the sign of `cross(a, b)`. Where
  `cross(a, b) = 0` (straight on, or a reversal where the path doubles back), the sign of
  `cross(a, p - v)`. A zero sign gives `+d`. The vertex rule is ours; the sum of the two cross
  products, used in an earlier draft, gives the wrong side at sharp turns (vector in section 10).
- **The side of a region.** Inside, outline included, by the exact winding test (`geometry.md`
  1, Predicates); holes are outside. No side test along the outline is used.
- **Several rings.** A region from `where` may have several pieces and holes (`geometry.md` 2.2).
  `t` is undefined on a region of more than one ring (`geometry.md` 2.6: `geom.many_rings`; in a
  formula, `formula.shape`); `d` and `sd` measure to every ring.

## 6. Windows and halos

A **window** is a rectangle of posts: `i0 <= i < i1`, `j0 <= j < j1`, inside the grid. Fields are
evaluated by window from the start (doctrine section 7), so tiling can come later without changing
any result.

**The window rule.** Evaluating a field over a window gives, at every post of the window, the same
bits as evaluating it over the whole extent and cutting the window out. This holds for every window
shape and position, a single post included.

What the rule requires of every operation:

- A value at a post depends on the post's position, never on the window's origin, size or the
  order windows are done in. Noise is evaluated at world (or import frame) coordinates.
- An operation that reads neighbours declares its **halo**: how many posts around it it reads, or,
  for a distance, its cap (R28; `formulas.md` 8.6). A read with no bound is a step. To
  fill a window, its inputs are evaluated over the window grown by the halo, clipped to the grid. At
  the grid's edge, the operation's own edge rule applies, and it is the same rule whole or windowed.
  This is the halo, or ghost-cell, scheme of stencil codes.
- A `radius` cuts by distance from a feature, never by window.

**Samples of a feature.** A reduction over a feature (`under`, the samples `smoothed_along` smooths,
`formulas.md` 8.5) runs over the feature's samples, in this order, whatever window asked for it, and
its result is cached once per feature:

| Shape | Samples | Order |
|---|---|---|
| region | Every post inside the region, outline included, by the exact winding test. | Post order (section 4). |
| path | The points at arc length `s = i * r` (`r` the resolution as a double, `i` from 0) while `s < L`, then the point at `s = L`. Each by 5.5 with offset 0, rounded to the lattice. | By `i`, the end last. |
| point | The point itself. | One. |

A value at a sample that is not a post is read by `fields.md`'s rule between posts. A reduction with
no samples (a region with no post inside) is `num.empty_reduction`, naming the feature; it never
gives 0. Ours.

## 7. Named seeds

Randomness comes only from named seeds (principle 7). There is no global random state and no
generator whose output depends on how many draws came before.

### 7.1 Seeds in text

| Where | Key | Takes | Default |
|---|---|---|---|
| The world | `seed` | integer, `0` to `2^63 - 1` | `0` |
| A `scatter` | `seed` | a name, as an id (`[a-z][a-z0-9_]*`) | the scatter's id |
| A noise function in a formula | `seed` argument | a name (`formulas.md` 8.2) | none: the stream is named by owner and place (`formulas.md` 8.2) |

Only a scatter draws among the rules. A `run` and a `set` draw nothing, so `seed` on either is
`doc.unknown_key` (`rules.md` 4 and 5, case r70).

- The world seed is a TOML integer, used as an unsigned 64-bit value. A negative one is
  `num.seed_range`. One above `2^63 - 1` is not a TOML integer and fails at parse (`doc.syntax`).
- A seed of the wrong form is `num.seed_type`: a world seed that is a string or a float; a scatter
  seed that is a number (hint: a name); a scatter seed that breaks the id grammar, such as `"Trees"` (hint:
  its lower-cased form). A noise seed of the wrong form is reported by `formulas.md` as
  `formula.seed`, which points here for the form.
- A scatter writes `seed` only to change its draws. Two scatters that write one seed name draw the same
  streams, which is how an author says "draw these alike".
- An imported world uses its own world seed (5.3), unless the parent writes `seed` on the import
  (`composition.md` 2).
- Measures are reported over several seeds (TODO, working rules). An ensemble is a set of requested
  changes, each setting the world `seed`, built side by side (`build.md`, `--change`); the text stays
  the only truth.

### 7.2 Derivation

SplitMix64 (Steele, Lea and Flood, 2014) and the root constant `0x5851F42D4C957F2D` come from the
earlier Rust code; `absorb`, `stream` and `draw` are ours. All arithmetic is unsigned 64-bit and
wraps.

```
splitmix64(x):
  x = x + 0x9E3779B97F4A7C15
  x = (x ^ (x >> 30)) * 0xBF58476D1CE4E5B9
  x = (x ^ (x >> 27)) * 0x94D049BB133111EB
  return x ^ (x >> 31)

mix(h, x)        = splitmix64(h ^ x)
absorb(h, bytes) = fold mix over the bytes, starting from mix(h, len(bytes))
stream(seed, parts) = fold absorb over the parts, starting from splitmix64(seed ^ 0x5851F42D4C957F2D)
draw(s, keys)    = fold mix over the keys, starting from mix(s, len(keys))
unit(u)          = (u >> 11) * 2^-53              exact, in [0, 1)
```

A **part** is a byte string, absorbed byte by byte. A part written as a string is its UTF-8 bytes;
other parts are encoded by the chapter that uses them (`identity.md` 2.2 encodes integers, sides,
roles and sets). A **key** is a signed 64-bit integer, mixed as its two's-complement bits. The
length that starts each `absorb` and `draw` keeps `["ab", "c"]` and `["a", "bc"]` apart. `unit` is
the usual 53-bit mapping. `identity.md`'s digest is `stream(0, ["name", class] ++ parts)`: this
function, not a second one (principle 10).

One more derivation is kept, from the earlier Rust code (its own there), for the one model drawn
from it, `lib/terrain`'s program (`protocol.md` 8):

```
sub_seed(seed, label, salt) = mix(fold mix over label's bytes, starting from splitmix64(seed ^ 0x5851F42D4C957F2D), salt)
```

It absorbs no length, so it keeps labels apart less well than `stream`; it stays, bit for bit,
because every settled landscape (basin, atlas and their cases) is drawn from it, and a new user
takes `stream`. It is not in the vector file: its values on the old code's vectors are pinned
where it is used (`crates/via-terrain/tests/carried.rs`), so the null digest of section 10 does
not move. Moved into `via-num` from the retired crate `via-artifact` at the M7 cutover (ours;
decided, open to the owner).

The first part says who draws, so streams of different users never meet:

| Drawn by | Parts | Keys | Defined in |
|---|---|---|---|
| A rule | `["rule", <seed name>, <purpose>]` | Where the draw is for: integers (a cell's `i`, `j`, a member index). | `rules.md` names the purposes (`jitter_x`, `jitter_y`, `keep`, asset, facing, scale, tint) and the keys of each. |
| A noise function | `["noise", "seed", <seed name>]`, or by default `["noise", <class>, <owner>, ...]` naming the owner and the formula's place in it | None: the noise reads its stream as its gradient hash's seed. | `formulas.md` 8.2, which lists the default parts. |
| A name in a key | `["name", <class>]`, then the parts digested, with seed 0 always | None: the stream is the digest. | `identity.md` 2.2. |
| A step | `["step", <step id>]`, then the item id as a third part under `for_each` | As the step chooses. | `protocol.md` hands it over. |

Where a draw is for a thing named by a place key (`identity.md`, a string), the place key is one more
part after the purpose; it is never turned into an integer key.

### 7.3 Draws by place

A draw is keyed by place, never by count: a rule draws `draw(stream, keys)` with keys that name
where the draw is for, as `rules.md` fixes. So a draw at one place does not move when a draw
elsewhere is added or removed, and draws can be made in any order or in parallel. This is a
counter-based generator in the sense of Salmon, Moraes, Dror and Shaw (2011), with SplitMix64 as its
mixing function. Each purpose has its own stream, so adding a purpose moves no other.

| Edit | Streams that change |
|---|---|
| World `seed` | Every stream of this world; none inside an import. |
| A rule's id, with no `seed` written | That rule's. |
| A rule's `seed` name | That rule's. |
| A noise call's seed name; with none, its owner's id or its place in the owner | That noise. |
| Moving a rule's table in its file, or to another layer | None. |
| Adding or removing another rule | None. |
| Moving or turning an import | None inside the import. |

## 8. The one maths module (`via-num`)

All arithmetic that can reach a build goes through the crate `via-num` (R26). The code check "all
maths through the one module" (`docs/plan.md`) enforces it: outside `via-num`, a core crate calls only
the float operations on the allow-list below, and no crate but `via-num` depends on a maths library.

| Allowed anywhere in the core (exact or correctly rounded) | Only through `via-num`, or not at all |
|---|---|
| `+`, `-`, `*`, `/`, `sqrt`, comparisons, `abs`, `copysign`, `floor`, `ceil`, `trunc`, `round_ties_even`, `to_bits`, `from_bits`, `is_finite`, `is_nan`, `is_sign_negative`, `total_cmp`, `as` conversions between integers and floats | `mul_add` (never), `round` (ties away; never), `f64::min`, `f64::max`, `f64::clamp` (never: rule 4), `powi`, `powf`, `exp`, `exp2`, `ln`, `log`, `log2`, `log10`, `sin`, `cos`, `tan`, `sin_cos`, `asin`, `acos`, `atan`, `atan2`, `sinh`, `cosh`, `tanh`, `hypot`, `cbrt`, `exp_m1`, `ln_1p`, and every other method of `f32` or `f64` |

Its rules:

1. **Doubles.** Arithmetic is IEEE 754 binary64, round to nearest, ties to even. `+`, `-`, `*`,
   `/` and `sqrt` are correctly rounded on every target, so they are used directly.
2. **One library for the rest.** Every transcendental comes from the `libm` crate (a Rust port of
   musl's libm), at a version pinned in the engine's lock file, with `default-features = false` (no
   `arch` fast paths) and the feature `force-soft-floats`, so no target-specific code changes bits.
   Never from the platform's maths library, which differs between systems. Changing the pin is an
   engine version change (section 9).
3. **No fused multiply-add.** `mul_add` is never called, and no target feature or flag may let the
   compiler contract `a * b + c`. Every expression is evaluated in the order written.
4. **Defined `min` and `max`.** `min(a, b)` is `a` if `a < b`, `b` if `b < a`, and when they are
   equal: `-0` if either is `-0`, else `a`. `max` mirrors it with `+0`. `std`'s `f64::min` leaves
   signed zero unspecified and is not used. No NaN reaches them (rule 7).
5. **Fixed reduction order.** A sum, product, minimum or maximum over many values is defined over a
   fixed sequence (post order, the sample order of section 6, or id order for lists): split into
   chunks of 1024 in sequence order; reduce each chunk left to right, starting from its first
   element; then reduce the chunk results left to right the same way. A mean is that sum divided by
   the count as a double. An empty sum is `+0` and an empty product `1`; an empty mean, minimum or
   maximum is an error (`num.empty_reduction`). A parallel version must give these bits. No result
   depends on the thread count. The chunk rule is ours.
6. **Integers.** Lattice arithmetic is integer and checked: an overflow is `num.overflow`, naming
   the operation's subject (feature, rule or field) and position, never a wrap. Only seed hashing
   wraps, and says so. No text reaches it in M1: lengths are bounded by 3.1 step 5 and computed
   positions by `geom.range`, so 64-bit sums of them cannot overflow; a named workspace test covers
   it (`diagnostics.md` 2).
7. **No non-finite value enters a build.** A double that is infinite or NaN is never stored in a
   field, an attribute, a position or a build file. `via-num`'s functions return a checked value. A
   formula's non-finite result is `formula.not_finite` (`formulas.md` 9). A non-finite literal in a
   returned `document.toml` is `num.not_finite` (`protocol.md` 5). Any other (a step's column blob
   or returned field file (`protocol.md` 4.2), or a stored feature value) is
   `num.not_finite_result`, naming the step or feature and the row or position. One condition, one code, as `diagnostics.md` recommends.
8. **No negative zero is stored.** A zero leaving `via-num` into a stored value is `+0`.
9. **Narrowing.** Where a stored type is narrower than a double (`fields.md` decides), the double is
   rounded to nearest, ties to even, once, on store.
10. **Ordered everything.** No hash map's iteration order reaches a result (R19), and document
    order never does. A field's entries take R29's order (`authority.md` 4); any other tie no key
    settles breaks by place name, bytewise (`vocabulary.md` section 2.2).

The SDK exposes the formula function list (R7) through `expr.eval` (`protocol.md` 8): the engine
computes, the SDK computes none of it itself. Python's own `math` calls the platform's library, so
a step that wants the engine's bits calls the engine in a batch (R23) rather than reimplementing a
function.

## 9. Same text, same bytes (R26)

**Promised.** Given the same inputs, one engine version builds every file of the build directory to
the same bytes:

- on macOS, Linux and Windows, on x86-64 and on 64-bit ARM;
- at any thread count;
- built clean or incrementally (`graph.md`);
- evaluated whole or by windows of any shape (section 6);
- with the tables and keys of any file in any order (`vocabulary.md` section 2.2).

The inputs are: the world's text and its layers, the included libraries, the pack, the requested
changes, the files declared as inputs (a field's `file`, a step's `inputs`), and the recorded
results of steps. Text inputs are compared after reading, with line endings normalised to LF, so
a checkout that turns LF into CRLF on Windows is the same text (`documents.md` reads files). Build
files hold no wall-clock time, host name, user name, absolute path or thread count (`build.md`).

**Not promised.**

| What | Why |
|---|---|
| Bytes across engine versions | A version may change a definition (a `libm` pin, a formula, this chapter). Its notes say which outputs move. |
| A user's step run again | Recorded, not promised (R26). The record (its outputs by digest, `protocol.md`) is an input; a rebuild that reuses it is same bytes. A step that gives other bytes on a rerun is reported, not hidden. |
| An outside program a step calls | As portable as that program (spike SPEC 12.8). |
| What the engine renders | Past the engine's door (principle 12). |
| Timings and logs | Kept outside the build files. |

Windows is a target from the first day, and is unproven until runs on hosted machines are allowed
(`docs/plan.md`, "Comes back to you later").

## 10. Test vectors

Doubles are written as their 64-bit patterns in hexadecimal, with the decimal for reading only; the
bits are normative. Positions are in millimetres. The machine-readable copy is
`spec/vectors/numbers.toml`, written with the conformance runner (M0 step 5); until then these
tables are its source. Tests in `via-num` read that file and embed no literals, so the deny-list
test (`vocabulary.md` section 7) never meets a stream name. The SDK does not read it: it computes
nothing (`protocol.md` 8), and a step's seed arrives in the request already derived. Computed with `libm` 0.2.16,
`default-features = false`, `force-soft-floats`.

**The vector file.** One table per vector, keyed by its id (`vocabulary.md` 2.1), in the order of
the tables below: `[vector.<id>]` with three keys, `fn`, `args` (an array) and `expected`. `via
vectors` (`build.md` 14) and the `via-num` tests read it; `spec/vectors/identity.toml` (`identity.md` 10)
and the noise vectors of `formulas.md` 8.2 use the same form. Values are written so: a 64-bit word
or a double's bits as a string, `0x` and 16 upper-case hex digits; an integer (millimetres,
millidegrees, a key, an index) as a TOML integer; a decimal read by section 3 (`t`, a scale, a
length's text) as a string of its text, never a TOML float; a point as `[x, y]`; a path or region
as an array of points; a list of parts as an array of strings; several results as an array, in
the order of the table's columns; a rejection as `{ code = "<code>" }`. `fn` is one of:

| `fn` | `args` | `expected` |
|---|---|---|
| `splitmix64` | `[word]` | word |
| `stream` | `[seed, parts]` | word |
| `draw` | `[stream, keys]` | word |
| `unit` | `[word]` | double |
| `facing` | `[millidegrees]` | `[sin, cos]` |
| `bearing` | `[dx, dy]` | millidegrees |
| `frame` | `[frames, point]`, `frames` an array of `[origin, facing, scale]` from the root outward | point (5.1, 5.4) |
| `point_at` | `[path, t, offset]` | `[point, along]` (5.5) |
| `measure` | `[shape, points, point]`, `shape` one of `"point"`, `"path"`, `"region"` | `[t, d, sd]` |
| `samples` | `[path, r]` | array of points (6) |
| `length` | `[text]` | millimetres, or a rejection (3.1) |
| `metres` | `[millimetres]` | double (4) |
| `sum` | `[doubles]` | double, in order (8) |
| `min`, `max` | `[double, double]` | double (8) |
| `write`, `write_length` | `[double]`, `[millimetres]` | text (3.4) |
| `noise` | `[stream, octave, px, py]` | double (`formulas.md` 8.2) |
| `digest` | `[class, parts]` | 16 lower-case hex digits (`identity.md` 10) |

A row given as a reason only (`1.005` read through a double; the Before rounding column) is not a
vector. A gradient index of `formulas.md` 8.2 is the `draw` word's top four bits and is checked
from it. How `digest` writes its parts is (open), decided in `identity.md` 10 with the file.

**Constants.** `PI` is `0x400921FB54442D18`. `PI / 180000.0` is `0x3EF24D1521193474`.
`K = 180000.0 / PI` is `0x40EBF9F8F1C56B6C` (57295.77951308232).

**SplitMix64 and streams.**

| Input | Output |
|---|---|
| `splitmix64(0)` | `0xE220A8397B1DCDAF` |
| `splitmix64(1)` | `0x910A2DEC89025CC1` |
| `splitmix64(0 ^ 0x5851F42D4C957F2D)` (root of seed 0) | `0xBA8894FA3BE59747` |
| `stream(0, ["rule", "alpha"])` | `0x7A0B95CDDB5BE11D` |
| `stream(1, ["rule", "alpha"])` | `0x60D5960CEFC51AC5` |
| `stream(0, ["rule", "alph"])` | `0x9AFDD40726496466` |
| `stream(0, ["rule", "alpha_"])` | `0x08E4899C9F655CAB` |
| `stream(0, ["rule", "ab", "c"])` | `0x9F0B7FFF365137DE` |
| `stream(0, ["rule", "a", "bc"])` | `0x2971A02DF9563DEE` |
| `stream(0, ["rule", "alpha", "keep"])` | `0xEEEB12673BDC4403` |
| `stream(0, ["rule", "alpha", "jitter_x"])` | `0xE72578E3B18692E7` |
| `stream(0, ["noise", "seed", "hills"])` | `0xB2257365AD2F65DB` |
| `stream(0, ["noise", "field", "height", "base"])` | `0x2B7A1CEA060EDEB6` |
| `stream(0, ["step", "relief"])` | `0xDC461A972B032DF7` |
| `stream(0, ["step", "relief", "beta"])` | `0xE4531D06F5242E11` |

**Draws**, with `s = stream(0, ["rule", "alpha", "keep"])`.

| Input | `draw` | `unit(draw)` |
|---|---|---|
| `draw(s, [0, 0])` | `0xBDEB160205197BBA` | `0x3FE7BD62C040A32F` (0.7418683772021718) |
| `draw(s, [1, 0])` | `0x84C4866B3F1AE2B7` | `0x3FE09890CD67E35C` (0.5186237346489224) |
| `draw(s, [0, 1])` | `0x4879AD2673908BEA` | `0x3FD21E6B499CE422` (0.28310663403277847) |
| `draw(s, [-1, 0])` | `0x98B64AE0AC14CF31` | `0x3FE316C95C158299` (0.5965315626636595) |
| `draw(s, [3, 4])` | `0x6238B13CAA1192AB` | `0x3FD88E2C4F2A8464` (0.38367755633414036) |
| `draw(s, [3, 4, 0])` | `0x094A1D9690ADDC3D` | `0x3FA2943B2D215BB0` (0.03628716398150489) |
| `unit(0)` | | `0x0000000000000000` |
| `unit(0xFFFFFFFFFFFFFFFF)` | | `0x3FEFFFFFFFFFFFFF` |

**Facings.**

| Millidegrees | sin | cos |
|---|---|---|
| `0` | `0x0000000000000000` | `0x3FF0000000000000` |
| `30000` | `0x3FE0000000000000` (0.5) | `0x3FEBB67AE8584CAA` |
| `45000` | `0x3FE6A09E667F3BCD` | `0x3FE6A09E667F3BCC` |
| `90000` | `0x3FF0000000000000` | `0x0000000000000000` |
| `120000` | `0x3FEBB67AE8584CAA` | `0xBFE0000000000000` |
| `180000` | `0x0000000000000000` | `0xBFF0000000000000` |
| `-90000` (is `270000`) | `0xBFF0000000000000` | `0x0000000000000000` |
| `359999` | `0xBEF24D15211548FF` | `0x3FEFFFFFFFEB111D` |

**Bearings** (section 2), direction `(dx, dy)` to millidegrees.

| Direction | Before rounding | Facing |
|---|---|---|
| `(0, 1)`, `(1, 0)`, `(0, -1)` | 0, 90000, 180000 | `0`, `90000`, `180000` |
| `(-1, 0)` | -90000 | `270000` |
| `(1, 1)` | 45000 | `45000` |
| `(3, 4)`, `(3000, 4000)` | 36869.89764584402 | `36870` |
| `(1, 2)` | 26565.05117707799 | `26565` |
| `(-5, -12)` | -157380.13505195957 | `202620` |
| `(1000, -1)` | 90057.2957604145 | `90057` |

**Placed frames** (section 5.1, origin `(0, 0)`).

| Local point | Facing | Scale | In the parent |
|---|---|---|---|
| `(0, 10000)` | `90000` | 1 | `(10000, 0)` |
| `(10000, 0)` | `90000` | 1 | `(0, -10000)` |
| `(10000, 0)` | `30000` | 1 | `(8660, -5000)` |
| `(0, 10000)` | `30000` | 1 | `(5000, 8660)` |
| `(3, 4)` | `45000` | 1 | `(5, 1)` |
| `(1234, 5678)` | `12345` | 1 | `(2419, 5283)` |
| `(10000, 0)` | `30000` | 1.5 | `(12990, -7500)` |
| `(1234, 5678)` | `12345` | 0.8 | `(1936, 4226)` |
| `(1000, 2000)` | `0` | 2 | `(2000, 4000)` |

**Nested frames** (section 5.4). An instance at `(8500, 2000)`, facing `90000`, scale 1. In it, a
frame at local `(1000, 2000)`, facing `30000`, scale 1.5: its origin is `(10500, 1000)`, facing
`120000`, scale 1.5. A point at local `(2000, 0)` in that frame is at `(9000, -1598)`.

**Points at `t`** (section 5.5). Path A is `[(0, 0), (8000, 0), (8000, 8000)]`, `L = 16000`.
Path B is `[(0, 0), (1000, 3000), (5000, 2000)]`, `L` = 7285.38328578604.

| Path, `t`, offset | Point | `along` | Shows |
|---|---|---|---|
| A, 0.3, 0 | `(4800, 0)` | `90000` | |
| A, 0.25, 2000 | `(4000, 2000)` | `90000` | Left of travel east is north. |
| A, 0.5, 0 | `(8000, 0)` | `0` | At a vertex, the outgoing segment holds `t`. |
| A, 0.5, 1000 | `(7000, 0)` | `0` | Its normal, not the incoming one. |
| A, 1, 0 | `(8000, 8000)` | `0` | `t = 1`: the last segment, `f = 1`. |
| A, 0, -1000 | `(0, -1000)` | `90000` | |
| `[(0, 0), (1001, 0)]`, 0.5, 0 | `(500, 0)` | `90000` | 500.5 rounds to even. |
| `[(0, 0), (3000, 4000)]`, 0.5, 1000 | `(700, 2600)` | `36870` | |
| B, 0.3, 1500 | `(-732, 2548)`, from (-731.8728027636117, 2547.7980819617337) | `18435` | |
| B, 0.7, -2000 | `(2395, 590)`, from (2394.570763310489, 589.8044963635477) | `104036` | |

**Measures** (section 5.5), at lattice points.

| Feature, point | Nearest | `t` | `d` (m) | `sd` (m) |
|---|---|---|---|---|
| A, `(3000, 2000)` | segment 0 | 0.1875 | 2 | 2 |
| A, `(4000, 4000)` | tie, segment 0 wins | 0.25 | 4 | 4 |
| A, `(9000, -1000)` | vertex 1, outer side of a left turn | 0.5 | 1.4142135623730951 | -1.4142135623730951 |
| A, `(8000, 0)` | vertex 1 | 0.5 | 0 | `+0` |
| A, `(-1000, 0)` | start, on the extension | 0 | 1 | 1 |
| A, `(9000, 9000)` | end | 1 | 1.4142135623730951 | -1.4142135623730951 |
| `[(0, 0), (100000, 0), (99000, 1000)]`, `(101000, 500)` | vertex 1, sharp left turn, unequal segments | 0.9860550753913473 | 1.118033988749895 | -1.118033988749895 |
| B, `(2500, 1000)` | segment 1 | 0.7003835570894763 | 1.5764815627361644 | -1.5764815627361644 |
| Square `[(0, 0), (10000, 0), (10000, 10000), (0, 10000)]`, `(5000, 2000)` | outline segment 0 | 0.125 | 0 (inside) | 2 |
| The same square, `(5000, -2000)` | outline segment 0 | 0.125 | 2 | -2 |
| The same square, `(-1000, -1000)` | `P0`, as `t = 0` not 1 | 0 | 1.4142135623730951 | -1.4142135623730951 |
| The square written clockwise, `[(0, 0), (0, 10000), (10000, 10000), (10000, 0)]`, `(5000, 2000)` | as the first | 0.125 | 0 | 2 |
| Point `(0, 0)`, `(3000, 4000)` | the point | 0 | 5 | -5 |

On the square, `on = { t = 0.125, offset = 2 }` gives `(5000, 2000)`, where `sd = 2`: `offset` and
`sd` agree. At `(101000, 500)` the rejected cross-sum rule says left; the true side is right.

**Path samples** (section 6). Path A with `r = 2000`: nine samples at `s` = 0, 2000, ..., 14000,
16000, the points `(0, 0)`, `(2000, 0)`, `(4000, 0)`, `(6000, 0)`, `(8000, 0)`, `(8000, 2000)`,
`(8000, 4000)`, `(8000, 6000)`, `(8000, 8000)`.

**Text and order.**

| Input | Result |
|---|---|
| Post `x` at 12,345 mm | `0x4028B0A3D70A3D71` |
| `1.005` read through a double, times 1000 | `0x408F67FFFFFFFFFF`: why section 3.1 never does this |
| Sum of `[0.1, 0.2, 0.3]` in order | `0x3FE3333333333334` |
| Sum of `[0.3, 0.2, 0.1]` in order | `0x3FE3333333333333`: why the order is part of the definition |
| Sum of `[1.0, 1e100, 1.0, -1e100]` in order | `0x0000000000000000` |
| `min(-0.0, +0.0)`, `min(+0.0, -0.0)` | `0x8000000000000000` both |
| `max(-0.0, +0.0)`, `max(+0.0, -0.0)` | `0x0000000000000000` both |
| Written out (3.4): `3.0`, `0.1`, `1e21`, `1e-7`, `5e-324`, `1.23e-18` | `3`, `0.1`, `1e+21`, `1e-7`, `5e-324`, `1.23e-18` |
| Written out (3.4): the length `1005` mm | `1.005` |

## 11. Worked example

```toml
name       = "tiny"
extent     = [0, 0, 10, 6]
resolution = 2

[feature.track]
path = [[0, 1.5], [10, 1.5]]          # a guide: no kind

[instance.hut]
asset = "hut"
on    = { ref = "track", t = 0.3, offset = 2 }
face  = 90

[scatter.trees]
assets  = ["@plant/tree"]
spacing = 2
density = 0.5
```

- Extent `[0, 0, 10000, 6000]` mm, `r = 2000`: 6 by 4 posts. Post `(5, 3)` is at
  `(10000, 6000)`; a formula reads it as `x = 10.0`, `y = 6.0`.
- `track` has `L = 10000` mm. `t = 0.3` gives `s = 3000`, `(3000, 1500)`. Left of travel east is
  north, so `offset = 2` puts the hut at `(3000, 3500)`, rounded once, facing 90000 millidegrees,
  east.
- A mount written at `(0, 1)` in the hut's frame (forward 1 m) is at `(4000, 3500)` in the world.
  One written at `(1, 0)` (right 1 m) is at `(3000, 2500)`: right of facing east is south.
- Post `(2, 1)` at `(4000, 2000)` has, from `track`: `t = 0.4`, `sd = 0.5`, `d = 0.5`. Post
  `(1, 0)` at `(2000, 0)` has `sd = -1.5`, `d = 1.5`.
- `trees` writes no seed, so its `keep` stream is `stream(0, ["rule", "trees", "keep"])`,
  `0x31B2A8C62E0994BD`. Its draw keyed `[0, 0]` is `0x6F75A37A63D07B78`, unit
  0.43538877237353 (`0x3FDBDD68DE98F41E`). Renaming the table to `[scatter.oaks]` changes every
  draw it makes; adding a second scatter changes none of them.
- `resolution = 3` would be `num.extent_not_multiple`: 10 is not a multiple of 3; the hint gives
  `[0, 0, 9, 6]`, `[0, 0, 12, 6]` and `resolution = 2`. `offset = 2.0005` would be
  `num.too_precise`.

## Diagnostics

Codes this chapter introduces. All are errors. `diagnostics.md` holds the list and the shape.

| Code | Severity | Meaning |
|---|---|---|
| `num.not_finite` | error | A number in text is `inf` or `nan`. |
| `num.too_precise` | error | A length is not a whole number of millimetres, or a facing not a whole number of millidegrees. The hint names the two nearest values that are. |
| `num.out_of_range` | error | A number in text out of range: a length beyond plus or minus `2^31 - 1` mm, a facing beyond 64 bits, a double whose nearest value is infinite. Computed positions out of range are `geom.range`. |
| `num.extent_empty` | error | `extent` has `x1 <= x0` or `y1 <= y0`. Any other wrong form is `doc.type`. |
| `num.resolution` | error | `resolution` is below 1 mm. |
| `num.extent_not_multiple` | error | The extent's width or height is not a whole multiple of `resolution`. The hint keeps `x0`, `y0` and gives `x1`, `y1` rounded down and up, and the nearest resolutions that divide both widths. |
| `num.too_many_posts` | error | `nx * ny` exceeds the post limit. Names `nx`, `ny` and the limit. |
| `num.seed_range` | error | The world `seed` is negative. |
| `num.seed_type` | error | A seed of the wrong form: a world seed not an integer; a scatter seed not a name, or a name that breaks the id grammar. The hint suggests a name, or the lower-cased form. |
| `num.overflow` | error | A checked integer operation overflowed. Names the subject and the position. |
| `num.empty_reduction` | error | A mean, minimum or maximum over a feature with no samples. Names the feature. |
| `num.not_finite_result` | error | A step or a stored feature value would store an infinite or NaN value. Names the step or feature and the row or position. A formula's is `formula.not_finite`. |

## Conformance

Each case runs through the command line (R20). Every case also checks that a clean build equals the
incremental one (`docs/plan.md`); for a case that must exit 1, that means the same
`diagnostics.json` bytes, clean and incremental. A field over a window is
`via eval WORLD --field F [--window i0,j0,i1,j1] --out FILE`, and the vectors computed by the
binary are `via vectors FILE`, both as `build.md` 14 gives them. The `via-num` unit tests check
the same vector file; those are crate tests, not conformance cases. The SDK computes nothing, so
it has no vector tests.

| Case | Input | Action | Must hold |
|---|---|---|---|
| lattice_read | Each row of the table in 3.1 as an `offset` on a placed instance | `via merge` | Accepted rows give those millimetres, written as exact metres in the merged document; rejected rows give their code, file and line, exit 1. No placement is made, so a large offset tests reading only. |
| lattice_no_float | `offset = 2.0000000000000001`; `offset = 0.0010000000000000001` | `via merge` | `num.too_precise` each. A reader through a double would accept 2000 and 1. |
| text_inf_nan | `resolution = inf`; `spacing = nan` | build | `num.not_finite` each, exit 1. |
| face_normalise | `face = -90`, `face = 270`, `face = 630` | build | One facing, 270000, in the build. |
| face_too_precise | `face = 12.3456` | build | `num.too_precise`. |
| posts_count | `extent = [0, 0, 10, 6]`, `resolution = 2` | build, read field header | 6 by 4 posts; post `(5, 3)` at `(10, 6)`. |
| resolution_default | `extent = [0, 0, 10, 6]`, no `resolution` | build, read field header | 21 by 13 posts at 500 mm; no `doc.missing_key`. |
| extent_not_multiple | `resolution = 3` on that extent | build | `num.extent_not_multiple` naming `[0, 0, 9, 6]`, `[0, 0, 12, 6]` and resolution 2. |
| too_many_posts | `extent = [0, 0, 2000000, 2000000]`, `resolution = 0.001` | build | `num.too_many_posts`, exit 1, before any allocation of the grid. |
| frame_vectors | An asset with a square footprint, placed with `face` at the facings of the section 10 frame table, `scale` 1, 1.5 and 0.8 | build | Footprint corners in the build as section 5.1 gives them. |
| frame_nested | The section 10 nested vector, as a set item in an instance placed on a path | build | Facing `120000`; position `(9000, -1598)`. M3 (sets). |
| sd_sign | Path A and the square, counter-clockwise and clockwise; a formula writing `sd`, `d` and `t` | build | The section 10 measures at those posts; `sd` positive left of the path and inside the region; equal to `offset` at placed points. |
| sd_chapters | The square; `on = { t = 0.125, offset = 2 }`; a formula writing `sd` and `d` | build | The placed point is `(5000, 2000)`; there `sd` is `+2` and `d` is 0, the worked points of this chapter and `geometry.md` 2.6 agreeing. |
| point_t | A point feature of a kind whose effect reads `t` | build | `t` is 0 at every post; no error. `on` with `t` on it is `geom.no_length`. |
| sd_vertex | The sharp-turn path of section 10; Path A | build | `sd` negative at `(101000, 500)` and `(9000, -1000)`, by the exact test. |
| degenerate | A path with a repeated vertex; a one-point path with `t`; `face = "along"` on it | build | `geom.repeated_point`; `geom.degenerate`, and the feature placed on it skipped (`diagnostics.md` 4: it raises nothing of its own); never a facing of north by default. |
| points_at_t | The section 10 points-at-`t` table as instances placed `on` | build | Positions and `along` facings as tabled, `(500, 0)` included. |
| window_equals_whole | ford's height field | `via eval` whole and by windows of 1x1, 7x3, 64x64 and odd offsets | Every post the same bits. |
| halo_edge | A field `slope('height')` (halo 1, `formulas.md` 8.3), windows touching each grid edge | as above | Same bits as the whole. |
| seed_vectors | `spec/vectors/numbers.toml` | `via vectors` | Every vector's bits. |
| seed_default | A scatter with no `seed`; the same with `seed = "<its id>"` | build both | Same instances. |
| seed_rename | Rename the scatter | build, change report | Its instances added and removed; nothing else. |
| seed_neighbour | Add a second scatter elsewhere | change report | The first scatter's instances unchanged. |
| seed_world | World `seed = 1` | change report | At least one scatter instance moves; every hand-written instance and pin is unchanged in plan (`x`, `y`, facing). |
| seed_bad | World `seed = -1`; world `seed = "a"`; scatter `seed = 1`; scatter `seed = "Trees"` | build | `num.seed_range`; `num.seed_type` three times, the last hinting `trees`. |
| shuffled | ford with its tables and keys shuffled | build | Same digest. |
| crlf | ford with CRLF line endings | build | Same digest as with LF. |
| threads | ford at 1 and 8 threads | `via build --threads N` | Same digest. |
| systems | ford on macOS, Linux (and Windows once allowed) | build | Same digest. |
| empty_reduction | `under('mean')` on a region with no post inside | build | `num.empty_reduction` naming the feature. |
| non_finite_formula | A field `base = "= 1 / (x - x)"` | build | `formula.not_finite` naming the formula and a position, never a NaN in the field. |
| non_finite_step | An M2 worker whose call carries an f64 column blob holding one NaN | build | Error -32004 and `num.not_finite_result` naming the step and the row. M2. |
| no_neg_zero | A formula giving `-0` | build, read the field file | The bytes of `+0`. |

## Changed from the spike

| Spike | v1 | Why |
|---|---|---|
| `yaw` counter-clockwise from east | `face`, a bearing clockwise from north, in integer millidegrees | One convention (`vocabulary.md`); exact comparison. |
| Coordinates as doubles | Integer millimetres from the literal text | R24. |
| `nx = round(width / resolution) + 1` | The width must be a whole multiple | Nothing rounded silently. |
| Signed distance negative inside a region | Positive inside, so `sd` equals `offset` | Principle 9. |
| `t = 0` for a region | Fraction along its outline | Same meaning as `on`'s `t`. |
| Noise seeded by the world, in world coordinates, also for parts | Each import keeps its own seed and frame | Moving a part changed its noise (FINDINGS). |
| `seed = 1` on every rule and on noise; `derive(seed, stage, salt)` | A name; a scatter's defaults to its id, a noise's stream to its owner and place; length-prefixed parts; one stream per purpose; draws keyed by place | Doctrine section 6; no stream shifts when a neighbour changes. |
| A failed expression became 0 | An error; no non-finite value is stored | R26, principle 8. |
| numpy and the platform's maths | `via-num`, `libm`, no fused multiply-add, fixed reduction order | R26. |
| Parts translated only, same resolution | Frames turn and scale; import resampling in `fields.md` | R8. |

## Open

- **Height on the lattice.** Whether a surface at a fixed level (a deck, a floor) holds `z` in
  millimetres. Decided in `surfaces.md` for M5, by whether two surfaces meeting at a pair of mounts
  must compare heights exactly.
- **The post limit.** `2^26` is provisional. Decided by the M0 probe: the largest grid that builds
  within the budget the plan sets for ford.
- **Windows bytes.** Decided by the first run on hosted runners, once a push is allowed.
