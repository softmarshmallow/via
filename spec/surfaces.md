# Surfaces

A surface is what things stand on: the terrain, a deck, a floor. This chapter defines the one
surfaces model (R25), the four batch questions every user of it asks, and how two surfaces meet.

Words come from `vocabulary.md`. Keys and codes this chapter proposes that other chapters do not yet
list are marked **(new)**; section 8 lists every edit this chapter needs elsewhere, and none is
read by code before the lead makes it.

What lands when (plan, "Order notes": the surfaces chapter is M1, connectors M5):

| Milestone | What of this chapter |
|---|---|
| M1 | The terrain only. The four questions on the terrain. Meetings of path ends with places on the terrain (`ford`). `surface.two_terrains`, `surface.empty_region`, `surface.host_missing`. Kind surfaces and kind mounts are `doc.not_yet`. |
| M2 | The four questions through the SDK, in a step's view (2.6). |
| M3 | Kind surfaces, kind mounts, `meets`, the place, height (`surface.gap`: in M1 every meeting is a path end on the terrain, so the gap is always 0, `fields.md`) and namespace checks. Assumes Open 2. |
| M5 | `connector`, facing, `turn`, `surface.unmet`; asset mounts with connectors; floors and stairs. |

## 1. What a surface is

**(M1 for the terrain; M3 for the rest.)**

A surface is a field with `role = "surface"` (`fields.md`) that has a **bound**. It gives one
height `z` (metres, world frame, up; `numbers.md`) at every point of its bound and nothing outside
it. No new noun (R25). "Owner" in this chapter is prose only, never a key: `owner` is the layer key
(`vocabulary.md` section 4).

| Surface | Owner | Bound | Height comes from | How it is evaluated |
|---|---|---|---|---|
| terrain | the world (R31): a world field | the extent | the field's `base`, `then` rows and kind effects (`fields.md`, `authority.md`) | sampled at the posts, interpolated between them as `fields.md` defines |
| a kind surface on a path feature (a deck, a ramp) | the feature | the strip of 1.2 | the surface's `base` formula (1.1) | from the formula, at the point asked |
| a kind surface on a region feature (a floor, a platform) | the feature | the region | as above | as above |
| the floors a volume yields | (open): item 6 | (open) | (open) | (open) |

The terrain is owned by the world, not by a feature (R31).

Rules:

1. **One terrain.** A world declares exactly one surface field at its top level. It is the terrain.
   A second is `surface.two_terrains`, from M1 and always: every other surface is a kind surface,
   so a second top-level one is never supported. This replaces `doc.not_yet` for "a second
   `surface` field" in `vocabulary.md` section 5 and `fields.md` (section 8). Kind surfaces and
   kind mounts are `doc.not_yet` until their milestone.
2. **A kind surface is declared on its kind**, never as a world field, so every feature of the kind
   has one (principle 10: a hand-written bounded surface is a feature of a kind with a surface):

   ```toml
   [kind.<k>.surface.<name>]      # (new)
   base  = "= <height formula>"    # metres, world frame (1.1)
   bound = "= <half width>"        # (new) path owners only (1.2)
   ```

   `base` keeps its field meaning, the value before any row; a kind surface has no rows, so its base
   is its value. It has no `then` rows. The surface is the place `<feature>.<name>`.
3. **One namespace.** A kind's mounts and surfaces share one namespace, so `<feature>.<name>` names
   exactly one. A surface and a mount with one name is `surface.name_clash`. `start`, `end` and
   `ground` are reserved for surfaces as for mounts (`identity.reserved`).
4. **Owner shape.** A path owner needs `bound`; a region owner must not have one (the region is the
   bound); a point owner cannot own a surface. Any other combination is `surface.owner_shape`. A
   bound that varies along `t` is (open), item 3.
5. **A kind surface is its owner's alone.** Selectors (`role:`, `tag:`, and field selectors in
   formulas) match world fields only. A kind surface carries no tags, no effect writes it, and no
   formula reads it with `f()` (Open 9). So an effect on `role:surface` writes the terrain. To change
   a deck, change its owner (R18).
6. **A height function over its bound.** One `z` per point. An overhang, a cave or a room under a
   room is two surfaces, never one (Open 8, 10).
7. **Overlap is legal.** Two anchored kind surfaces whose bounds overlap are not a core error: a
   deck over a deck and a floor over a floor are both meant. A doubled deck (R18) is a library lint.
   Where the owner also writes the terrain (a crossing raising its banks), that effect is ordered by
   R21 like any other.
8. **Change.** A kind surface and its mounts move with their owner and are not change-report entries
   of their own. Moving the owner reports the owner, every path whose end lies at its mounts, every
   instance grounded on it, and the field regions that changed (`vocabulary.md`, change report).
9. **Values.** A surface never holds a non-finite value (R26). A formula that fails at a point is
   handled by 2.7; it never becomes 0 silently.

### 1.1 Kind surface formulas

`formulas.md` section 6.1 has no rows for these keys; section 8 asks for them. Until then this table
is the source.

| Key | Type | Rate | Names visible |
|---|---|---|---|
| surface `base` | number, metres | per point **(new)**: at any lattice point asked, not only at posts | `x`, `y`, `t`, `d`, `sd` of the owner at that point (`numbers.md` 5.5); the owner's attributes; the kind's per-feature `let` names; the functions of `formulas.md` 8.1 and 8.2 |
| surface `bound` | number, metres, > 0 and finite | per feature | the owner's attributes; the kind's per-feature `let` names; the functions of 8.1 |

- Every name in the `base` row is defined at any point of the plane, so "per point" changes nothing
  in their meaning: `d` at a point between posts is the distance at that point, and a noise function
  is a function of `(x, y)`.
- Field reads (`v`, `f`, `slope`, `under`, `smoothed_along`), shape functions (`dist`, `sdist`, `near`,
  `inside`, `along`) and per-post `let` names are not visible: `formula.unknown_name`. Reading a
  field from a surface formula is Open 9.
- `t`, `d`, `sd`, `x` or `y` in `bound` is `formula.rate`. A bound that is not above 0 or not finite
  is `surface.bound`.

### 1.2 The bound of a path owner

- The bound is `stroke(path, bound, bound, join = round, cap = butt)` (`geometry.md` 3.4): an exact
  region on the millimetre lattice (R24). `bound` is converted to millimetres once,
  `round_half_even(bound * 1000)`, by `numbers.md` 3.1's rule for computed positions; a result of 0
  is `surface.bound`.
- A point is inside when the exact polygon test on the lattice puts it inside or on the edge. The
  polygon decides, not `d`: on a round join the chords stray up to 5 mm, so the formula may see `d`
  up to 5 mm above `bound`.
- **Fold.** Where the strip overlaps itself (a bend whose inner radius is less than `bound`), `t`
  jumps inside the strip and `z` would have a cliff. This is `surface.fold` (error), found as
  `geometry.md` 3.5 finds `geom.follow_broken`: the curve at `bound` on either side is not one piece.
  It gives the position and the largest bound that holds.
- A path of zero length is not a valid path (`geometry.md` 2.1), so a strip is never empty.
- A bound reaching beyond the extent is clipped to it, with `surface.clipped` (info) giving the share
  lost. Nothing stands outside the extent.

## 2. The four questions

**(M1 on the terrain; M2 through the SDK; M3 on kind surfaces; M5 for the facing columns.)**

Placement, grounding, reach and export ask only these four questions of surfaces (R25). Each is a
batch: columns in, columns out (R23). The same four are engine operations a step calls through the
SDK (`protocol.md`, `geom.op`), so our libraries use exactly what a user's step can (R19b). Their
names are used nowhere else (principle 9).

Points in are plan coordinates on the millimetre lattice. Heights out are f64. Wherever a height
becomes geometry (an instance transform, a mesh vertex, a meeting test) it is rounded to the lattice
by `numbers.md`'s rule.

### 2.1 `surfaces_at(points)` (new)

Every surface at each point.

| In | Out, one row per (point, surface) |
|---|---|
| `x`, `y` | `point` (row index in), `surface` (place name), `z` |

- A surface is listed at a point when the point is inside its bound (1.2). Points on a bound's edge
  are in.
- Rows for one point are ordered by `z`, lowest first; ties by surface place name, bytewise.
- A point outside the extent has no rows.

### 2.2 `sample(surface, points)` (new)

The height and gradient of one surface.

| In | Out, one row per point |
|---|---|
| `surface` (one place name), `x`, `y` | `inside` (bool), `z`, `gx`, `gy` |

- `gx`, `gy` are `dz/dx` and `dz/dy`, unitless.
- A point outside the bound gives `inside = false` and `z = gx = gy = 0`. It is never an error by
  itself; the caller decides.

**Terrain.** `z` is `fields.md`'s bilinear interpolant. The gradient is that interpolant's, exactly:

- The cell is `fields.md` 1.2's, the one definition: `i = min(floor((x - x0) / r), nx - 2)`, and
  `j` likewise with `y0` and `ny`, in integer millimetres. So a point on a line between two cells
  takes the cell to its east, then to its north, and a point on the extent's east or north edge
  takes the last cell.
- With `u = (x - x_i) / r`, `v = (y - y_j) / r` (integer millimetre differences, one f64 division
  each, `r` in millimetres) and the four posts `z00`, `z10`, `z01`, `z11` read as f64, and `R` the
  resolution in metres:

  ```
  gx = ((z10 - z00) * (1 - v) + (z11 - z01) * v) / R
  gy = ((z01 - z00) * (1 - u) + (z11 - z10) * u) / R
  ```

  evaluated left to right in f64 through `via-num` (`numbers.md` section 8).
- `slope(field)` in `formulas.md` 8.3 uses central differences over posts, so on the same ground it
  can differ from `sample`'s gradient. Reach and grounding use `sample`; `slope` is for densities.

**A kind surface.** `z` is the `base` formula at the point. The gradient is a central difference, ours:
a formula of `t` and `d` has no simple closed derivative, and a fixed step gives the same bytes
everywhere (R26).

- The step is `h_mm = max(1, floor(r_mm / 2))` millimetres, so samples stay on the lattice; `H` is
  `h_mm / 1000` and `H2` is `(2 * h_mm) / 1000`, each one f64 division.
- On x, with `p+ = (x + h_mm, y)` and `p- = (x - h_mm, y)`: both inside, `gx = (f(p+) - f(p-)) / H2`;
  only `p+` inside, `gx = (f(p+) - f(p)) / H`; only `p-` inside, `gx = (f(p) - f(p-)) / H`; neither
  inside (a strip narrower than `2 * h`), `gx = 0`. Likewise `gy`.

### 2.3 `relief(surface, regions)`

How much a surface varies under a footprint. This is the one definition of relief: `rules.md` 8.2
measures relief by calling it (section 8).

| In | Out, one row per region |
|---|---|
| `surface`, a list of regions (polygons on the lattice) | `low`, `high`, `outside` |

Let `R` be the region and `B` the surface's bound (the extent for the terrain).

- **Terrain: exact.** `low` and `high` are the least and greatest `z` over `R ∩ B`, taken at the
  posts inside it, its vertices, each crossing of its edges with a grid line, and on each edge piece
  within one cell, the turning point of the interpolant along it (a quadratic in the edge's
  parameter), where it falls strictly inside the piece. That point is not rounded to the lattice.
  Bilinear has no extreme inside a cell, so this set holds both extremes. Ours.
- **A kind surface: sampled, not exact.** The same set less the turning points, over `R ∩ B`. A
  formula such as `fbm` has extremes anywhere; the result is the extremes over that set.
- `outside = (area(R) - area(R ∩ B)) / area(R)`, with both areas exact integer square millimetres
  from the kernel (`geometry.md` 3.3) and one f64 division, so half is exactly 0.5.
- `R` wholly outside `B` gives `low = high = 0`, `outside = 1`. A region of zero area is
  `surface.empty_region` (error).

### 2.4 `meetings()` (new)

Every meeting in the caller's view (2.6). Takes no input; it reads the meetings section 3 defines.

| Column | Type | Meaning |
|---|---|---|
| `a`, `b` | string | The two mounts, in the order 3.3 gives. |
| `x`, `y` | i64 mm | The meeting's plan point (`a`'s). |
| `z_a`, `z_b` | f64 | Each side's height. |
| `gap` | i64 mm | `abs(round_mm(z_a) - round_mm(z_b))`. |
| `face_a`, `face_b` | i64 millidegrees, or null | Each side's world bearing; null where a side has no face. |
| `turn` | i64 millidegrees, or null | 3.4. Null for a `meets` pair, or where the place has no face. |
| `connector_a`, `connector_b` | string, or null | Each side's connector. |

Rows are ordered by `a`, then `b`, bytewise.

### 2.5 Who asks what

| User | Asks | What it does with the answer |
|---|---|---|
| Placement (`rules.md`) | `surfaces_at` | Finds which surfaces lie at a plan point (4.1). |
| Grounding | `sample`, `relief` | Sets each instance's `z`; checks its footprint against its asset's facts and its surface's bound. |
| A reach check (a library) | `surfaces_at`, `sample`, `meetings` | The core rule it relies on: meetings are the only edges between surfaces. How it walks, and what it calls walkable, is the library's. |
| Export (`build.md`) | `surfaces_at`, `sample`, `meetings` | The terrain as a field; every kind surface as a mesh; the meetings as data. |
| A library lint | any | Headroom under a deck, depth on a ford, a doubled deck (R18). The core knows none of these words. |

### 2.6 Which view a question answers for

- A step's questions answer over the view R21 gives it: features and fields of its own layer and
  below, the terrain as that view settles it (`fields.md` 6.2, one evaluation per distinct view). A
  kind surface whose owner lies above the step's layer is absent, as a name above it is absent
  from `build.features` (`protocol.md` 3.3).
- `meetings()` is available at `lint` and `export` only, which read the finished build (R29).
  Meetings are checked after grounding, so at `generate` it is `surface.too_early` (error), naming
  the step.
- The build itself asks every question over the whole world.

### 2.7 A formula that fails at a point

A kind surface's `base` that gives a non-finite value at a point asked:

- In a build: `formula.not_finite` (error) on the owner, at the first failing point in input order;
  the build fails.
- In a query (2.1 to 2.4, through the SDK or `via query`): the whole call fails with that
  diagnostic and that point. No partial output. Through the SDK this is the error of
  `protocol.md` 3.4.

## 3. Where two surfaces meet

**(M1 for path ends and places on the terrain; M3 for kind mounts; M5 for connectors and facing.)**

### 3.1 Mounts on a kind

A mount is a named place (`vocabulary.md`). A kind declares the mounts every one of its features
has:

```toml
[kind.<k>.mount.<name>]
t         = 0             # where on the owner, as in `on`
offset    = 0             # metres to the left of travel
face      = 0             # degrees, clockwise from the owner's direction of travel at `t`
surface   = "<name>"      # (new) the owner's surface it lies on; default the terrain
meets     = "<mount>"     # (new) another mount of the same kind: the pair is one meeting
connector = "<word>"      # (new, M5) what joint it is; free word, compared only for equality
```

- **Position.** The point `on = { ref = <owner>, t, offset }` gives (`geometry.md` 2.6). There is no
  `at` form on a kind mount.
- **Face.** A number only: degrees clockwise from the direction of travel of the segment that holds
  `t` (`geometry.md`'s `along`). So 0 looks along travel and 180 against it, at any offset. The world
  bearing is that direction's bearing plus `face`, normalised (`numbers.md` 2). No face means none.
  A face word is `surface.mount_face`, since a kind mount has no `on`. This is how a set item's
  `face` is added to its set's (`vocabulary.md`). Ours.
- **Height.** Never written: it is `sample(surface, its plan point).z`.
- **`meets`.** Names a mount of the same kind. One `meets` makes one meeting; both mounts naming
  each other is the same one meeting. A name the kind lacks is `surface.unknown_mount` (listing the
  kind's mounts); naming itself is `surface.meets_self`; a mount in two pairs is
  `surface.meets_twice`. `surface` naming a surface the kind lacks is `surface.unknown_surface`.

### 3.2 Mounts on an asset

`[asset.<id>.mount.<name>]` has `at` and `face` in the asset's frame (`assets.md`). Its plan point is
`at` through the instance's transform. It lies on the surface its instance stands on (4.1), and its
height is `sample` of that surface there. Its `at` and `face` are pack facts, so a pack swap may move
it (R30): its meetings are checked again and what follows is reported. It has no `surface` key and,
until M5, no connector: a
door's "no path reaches it" check is a library lint until then (section 8, challenge 48).

### 3.3 Meetings

A meeting is a pair of mounts (R31). There are two ways to make one, and no other:

| Pair | Made by | `a`, `b` |
|---|---|---|
| Two mounts of one owner | `meets` | the two mounts, ordered bytewise |
| A path end and a place | `starts_at` or `ends_at` (`vocabulary.md`) | `<path>.start` or `<path>.end`, and the place's default mount |

- **Default mount.** A path end is a mount of its path. A mount or a path end is its own default
  mount; a point feature's is its point on the terrain, with no face and no connector; an
  instance's or a set item's is its origin, on the surface it stands on (4.1), with no face and no
  connector; a place inside an import's is that of the thing it names (M4).

- A path end lies on its path's own kind surface when its kind owns exactly one, on the terrain when
  it owns none, and is `surface.which` (error) when it owns two or more. So a ramp whose end rests on
  another feature's deck is a path ending at a mount of that feature: a meeting between two owners,
  with no new key.
- The place may be a mount of any feature, a mount of an instance, a path end, a point feature, an
  instance or a set item, or a place inside an import (R8, M4). A point feature lies on the terrain, with no face and no
  connector.
- Two surfaces never meet anywhere else. Bounds that touch or overlap make no meeting. A path meets
  anything at its ends only (`vocabulary.md`, `starts_at`), so a path never meets a surface along
  its length.

### 3.4 What must hold at a meeting

| Check | Rule | Fails as | From |
|---|---|---|---|
| Same place | Both sides have one plan point on the lattice. A path end is put there by construction; two mounts must be written so. | `surface.apart` | M3 |
| Same height | `gap = 0`. | `surface.gap` | M3 |
| Same joint | Where both sides have a connector, they are equal. A path end takes its kind's `connector` **(new)**. | `surface.connector` | M5 |
| Direction | Two mounts with faces face opposite ways: `(face_a - face_b)` mod 360 000 is 180 000 millidegrees. | `surface.facing` | M5 |
| Answered | A mount with a connector is the place of at least one path end, or meets a mount with the same connector. A `meets` pair whose other side has no connector does not count. | `surface.unmet` | M5 |

- **`turn`.** For a path end at a place with a face. The arrival bearing is the end tangent's for
  `ends_at` and the reversed start tangent's for `starts_at`. With `e = (arrival - face)` mod
  360 000, `turn = min(e, 360 000 - e)`: unsigned, 0 to 180 000 millidegrees. The core reports it and
  sets no limit; a library lint judges it (R15).
- The height rule is exact, with no tolerance (R24). An allowed step, such as a kerb, is (open),
  item 4.
- **What sets each height**, for `surface.gap`: for a kind surface, its `base` key (file and line);
  for the terrain, the last row or effect in R21 order with mask above 0 at any of the four posts
  around the point, else the field's `base`, found by `fields.md` 6.4 (recomputed on demand, never
  stored).

### 3.5 A path interrupted by another owner's surface

The core rule: a path meets things at its ends only, and meetings are the only edges between
surfaces. So a path that should not pass across another feature's surface is written as two paths,
one ending at a mount of that feature and one starting at another. Everything else here is the ways
library's convention, shown in 6.3 and illustrative only:

- A crossing owns its deck and its two ends, and ways arrive at those ends (R18). A way that crosses
  water at a crossing is two path features, so it is never graded across the bed under the bridge.
- A main way written by what it passes (R16, `passes`) is lowered to those paths by the ways
  library. The plain form is always legal (principle 10).
- The crossing's kind holds four mounts: on each bank one on the deck and one on the terrain, paired
  by `meets`. If the crossing raises the ground there, it does so with its own effect, in the
  document, under R21 (TODO lesson: bridgeheads raised by a step writing height were invisible and
  ignored authority).
- A way that passes under a deck is not a meeting. Whether there is room is the library's headroom
  lint, from `surfaces_at`.

## 4. Grounding

**(M1 on the terrain; M3 on kind surfaces.)**

### 4.1 Which surface a thing stands on

| Placement | Stands on |
|---|---|
| `on` a thing that owns exactly one surface | that surface |
| `on` a thing that owns two or more | `surface.which` (error), listing them. Hint: place it `on` a mount. |
| `on` a mount | the mount's surface (3.1, 3.2) |
| `on` a place inside an import (M4) | as this table, for the thing the place names |
| `on` anything else (a path, a region with no surface, an instance) | the terrain |
| `at` | the terrain, always |
| a scatter (`within`) | the terrain; a scatter on a kind surface is (open), item 5 |

- `at` has one meaning, the terrain. Where another surface at the origin lies above the instance's
  `z` and below its top (`z` plus the asset's `bounds` top, scaled; `assets.md`), the thing pokes
  through it: `surface.through` (warning), naming that surface. Hint: "place it `on` <owner> to
  stand on that surface, or move it". Both fixes clear it. An instance well under a high deck does
  not warn. How an `at` names another surface (furniture on an upper floor) is (open), item 5.
- **Outside a bound.** Two codes, one per bound, so no condition has two:
  - An origin outside the extent, the terrain's bound, is `place.outside` (error, not placed),
    owned by `rules.md` 8.4. It applies whatever the thing stands on, and it is checked first. From
    M1.
  - A plan point inside the extent but outside the bound of the kind surface it stands on is
    `surface.outside` (error); the instance is not placed. From M3.
- **A host that was not placed.** An instance that is not placed, for any reason (`place.outside`,
  `surface.outside`, `surface.host_missing`, any other error of its own), offers no mounts and no
  surface. Each thing placed `on` it or on its mounts is not placed either, with `surface.host_missing` (error) on
  each, naming the host in `related`. The rule repeats down a chain: a thing on a thing on a missing
  host is `surface.host_missing` too, each naming its own host. From M1, since M1 already places a
  thing `on` an instance (limitations S11.I.5). A host that is missing because of an error raises
  the host's own code once and `surface.host_missing` on each dependant, never a second code on the
  host.

### 4.2 Height and footprint

- An instance's pivot is at `sample(surface, origin).z - sink`, with `sink` scaled (`assets.md`).
  Nothing else lifts it: the spike's `mount` height and `atop` are gone (`vocabulary.md` section 6).
- `ground = "<kind>"` makes the footprint, grown by `margin`, a feature of that kind (`rules.md`
  2.1). Its effects write the terrain only (rule 5). So `ground` on an instance that stands on a kind
  surface is `surface.ground_off_terrain` (error). Scattered instances carry no `ground` (`rules.md`
  7).
- After grounding, `relief(surface, [footprint])` is taken for every instance. `outside > 0` warns
  `surface.off_bound` with the share. `high - low` is compared with the asset's `max_relief`, and
  past it is `rules.md`'s `place.relief` (a scattered candidate gives way instead). This chapter
  measures; `rules.md` owns the code and its wording (section 8).

### 4.3 Order

The doctrine's line holds: fields from their bases, then features by authority, then rules, then
checks. For surfaces it reads:

1. **Plan positions.** Every placement is plan only (`rules.md` 6), so each hand instance, pin, run
   member, set item and step instance has its plan point and footprint before any height is read.
   The footprint feature of a `ground` is therefore an ordinary feature, ordered with the rest. Rule
   instances enter where `rules.md` 2.1 and 3.4 put them.
2. **Fields settle.** Every row and effect in R21 order, footprint features' effects included. A
   field's value is final only after this. A levelling kind reads the ground with `under('mean')`,
   which reads the field as written before its own effect (`formulas.md` 8.5), so it is not a cycle.
3. **Kind surfaces.** Their formulas read no field (1.1), only their owner's shape and attributes,
   so they are ready once features are.
4. **Grounding.** Every `z` from the final surfaces; then relief.
5. **Meetings, then checks.**

A footprint whose position depends on that footprint's own shape, or on a `then` row that reads it,
is `graph.cycle`. Its kind's effects on a field a shape reads are no cycle: shapes read the head view,
which holds no effect (`authority.md` 2.3). Scatter density reads fields, so scatters come after step 2 and carry no `ground`.

## 5. Ours and the engine's

Principle 12: we stop at the engine's door.

| Thing | Ours | The engine's |
|---|---|---|
| Terrain | The height field, its gradient, its materials (`build.md`). | Its mesh, its level of detail, its collision, how it is drawn. |
| A kind surface | A mesh of its top over its bound, on the lattice, with its owner and surface name; marked as one that collides. The mesh's spacing and material: (open) in `build.md`. | Drawing it, its collision at run time, navigation baked from it. |
| What dresses a surface | Instances placed by rules: a deck's parapet as a run, its piers as a library rule, assets from the pack. | Their look. |
| Meetings | The rows of `meetings()` in the build, so a loader or navigation can join surfaces. | Joining navigation across them. |
| Thickness, underside | Library attributes read by the library's rules. The core makes the walking surface only. | |

## 6. Worked examples

Kinds here are library text (principle 5); their attributes are illustrative. Formula syntax follows
`formulas.md`.

### 6.1 A cottage on terrain (`ford`, M1)

```toml
[field.height]
role = "surface"
base = "= 20 + 0.06 * y + 5 * fbm(110, 3, seed = 'hills')"

[kind.pad.effect.level]                   # library text
field  = "role:surface"
value  = "= under('mean')"
radius = 3

[instance.cottage]
asset  = "house_low"
ground = "pad"
margin = 1
on     = { ref = "lane_west", t = 0.6, offset = 12 }
```

The lane owns no surface, so the cottage stands on `height`. Its footprint plus 1 m becomes a `pad`
feature (step 1 of 4.3). Its effect sets `height` under it to the mean of the ground as written
before the effect (step 2). Grounding asks `sample("height", [P])` for the pivot and
`relief("height", [footprint])`, which gives `high - low = 0` where the pad covers the footprint.
Moving the lane moves `P`, the pad and the cottage; the change report lists the lane, the cottage,
its footprint feature and the region of `height` that changed.

### 6.2 A ford: one surface (M1)

`lane_west` ends at `ford`, a point on the beck; `lane_east` starts there. The ford owns no surface.
`meetings()` gives two rows, (`lane_east.start`, `ford`) and (`lane_west.end`, `ford`), both on the
terrain with `gap = 0`. `surfaces_at` at the ford gives one row, `height`. The water is a library
field, not a surface: the depth on the ford is `water - z`, measured by the water library's lint.
Moving the beck moves the ford, both lane ends and both meetings.

### 6.3 A bridge deck over a river (M3; connectors M5)

```toml
# ways library
[kind.lane]
connector = "way"                        # (new, M5)

[kind.crossing]
authority = "anchored"
requires  = ["deck_z"]
defaults  = { width = 5, camber = 0.02 }

[kind.crossing.surface.deck]
base  = "= deck_z - camber * d"
bound = "= width / 2"

[kind.crossing.effect.heads]             # raises the ground at each bank to the deck
field  = "role:surface"
value  = "= deck_z"
mask   = "= max(1 - smoothstep(0.05, 0.15, t), smoothstep(0.85, 0.95, t)) * (1 - smoothstep(3, 6, d))"
radius = 8

[kind.crossing.mount.west_deck]
t = 0
surface = "deck"
face = 180                               # against travel: out of the deck
meets = "west_head"
[kind.crossing.mount.west_head]
t = 0
face = 0                                 # along travel: into the deck
connector = "way"
[kind.crossing.mount.east_deck]
t = 1
surface = "deck"
face = 0
meets = "east_head"
[kind.crossing.mount.east_head]
t = 1
face = 180
connector = "way"

# world
[feature.mill_bridge]
kind   = "crossing"
path   = [[132, 168], [152, 168]]
deck_z = 24.5

[feature.lane_west]
kind    = "lane"
path    = [[0, 170]]
ends_at = "mill_bridge.west_head"

[feature.lane_east]
kind      = "lane"
starts_at = "mill_bridge.east_head"
path      = [[256, 172]]
```

- `surfaces_at` at the middle of the river gives two rows: `height` (the bed) and
  `mill_bridge.deck` at 24.5. The headroom lint reads both.
- **The levelled patch.** `t` is clamped at the path's ends (`numbers.md` 5.5, `geometry.md` 2.6), so the mask is 1
  for `t <= 0.05` (the first metre of the deck) and behind the west end. With the `d` factor, each
  end gets the ground at `deck_z` within 3 m of the deck's line and of the end point, fading out by
  6 m: a strip under the deck's first metre plus a half-disc behind the bank end. Likewise at the
  east end.
- **Why `gap = 0` at each head.** 132 and 168 are multiples of `ford`'s resolution, so the head lies
  on a post and its height is that post's value. There `t = 0` and `d = 0`, so the mask is 1. The
  crossing is anchored and the lane is not, so `heads` comes after the lane's effects in R21 order
  and sets the post to `mix(v, 24.5, 1)`, stored as f32 24.5 (exact). The deck at the mount is
  `24.5 - 0.02 * 0 = 24.5`. Both round to 24 500 mm. Likewise at the east head.
- `meetings()` gives four rows: the two pairs, and (`lane_east.start`, `mill_bridge.east_head`),
  (`lane_west.end`, `mill_bridge.west_head`). Every `gap = 0`. The lanes travel into and out of the
  heads, so each `turn` is about 0 (the lanes' end segments are near east-west).
- Each lane is graded up to its head and stops there; neither is graded across the bed.
- Delete `heads` and `meetings()` keeps four rows, two with a gap: `surface.gap` twice, each giving
  both heights, the gap, the deck's `base` line and the terrain's last writer there. The deck would
  otherwise float above the banks.
- Writing the crossing as a point on the beck, with its path found square to the water, is ways
  library work that lowers to this text. So is `deck_z` from the water level plus headroom (Open 9).

### 6.4 An upper floor (forward pointer, M5)

```toml
# building library
[kind.storey.surface.floor]
base = "= level"
```

A storey is a region feature; its floor is a surface over the region at its level. A stair is a path
feature whose kind owns one sloping surface. It starts at a mount of the lower storey and ends at a
mount of the upper one, so by 3.3 its two ends are meetings between three owners, and a reach check
climbs only by the stair. That this suffices is (open), confirmed by `college` in M5. `surfaces_at`
inside the house gives the terrain, the ground floor and the upper floor. How furniture names the
upper floor (item 5), whether a floor is measured from the terrain or from the world's datum, and
the floors a volume yields (item 6) are decided in M5.

## 7. The command line

**(M1.)** Conformance runs through the command line only (R20), so the questions have a command
**(new)**, for `build.md` section 14:

```
via query WORLD surfaces_at|sample|relief|meetings --in FILE --out FILE
```

- Flags besides `--in` and `--out`, as `build.md` 14 defines them: `--change`, `--lib`, `--store`,
  `--sdk`, `--diagnostics`.
- It builds `WORLD` with its changes (reusing the store) and answers over the whole world. It runs
  generate steps but no lint or export step. It writes no build directory, run report or
  `.previous`. `meetings` takes no `--in`.
- Both files are canonical JSON (`documents.md` 8): one object whose keys are column names, each a
  list. `surface` in `sample` and `relief` is a key holding one string; `regions` in `relief` is a
  list of rings, each a list of `[x, y]` in integer millimetres.
- Types: `x`, `y`, `point`, `gap` integers; `face_a`, `face_b`, `turn` integers in millidegrees;
  `z`, `z_a`, `z_b`, `gx`, `gy`, `low`, `high`, `outside` doubles written as `numbers.md` 3.4
  writes them; `inside` true or false; `surface`, `a`, `b`, `connector_a`, `connector_b` strings. A
  missing value is JSON `null`.
- Comparison: integers, strings and booleans exactly; doubles bit for bit (R26).
- Exit 0 with `--out` written, or 1 with its diagnostics on standard error and in the
  `--diagnostics` file (`build.md` 14; 2.7 here); no `--out` file is written.

## Diagnostics

All in the group `surface`. `diagnostics.md` fixes the form. Position is given wherever the fault
has a place.

| Code | Severity | Subject | Says | From |
|---|---|---|---|---|
| `surface.two_terrains` | error | the second field; the first in `related` | Two surface fields at the top level. Hint: make one a kind's surface. | M1 |
| `surface.empty_region` | error | the caller (the instance, or the step) | A region given to `relief` has zero area. | M1 |
| `surface.too_early` | error | the step | `meetings()` called at `generate`. | M2 |
| `surface.owner_shape` | error | the kind's surface | A surface on a point kind, a path owner without `bound`, or a region owner with one. | M3 |
| `surface.bound` | error | the feature | Its `bound` is not above 0 or not finite; gives the value. | M3 |
| `surface.fold` | error | the feature | Its strip overlaps itself; position and the largest bound that holds. | M3 |
| `surface.clipped` | info | the feature | Its bound reaches beyond the extent; the share lost. | M3 |
| `surface.name_clash` | error | the kind | A surface and a mount share a name. | M3 |
| `surface.unknown_mount` | error | the mount | `meets` names a mount the kind lacks; lists the kind's mounts. | M3 |
| `surface.unknown_surface` | error | the mount | `surface` names a surface the kind lacks; lists the kind's surfaces. | M3 |
| `surface.meets_self` | error | the mount | `meets` names the mount itself. | M3 |
| `surface.meets_twice` | error | the mount | The mount is in two `meets` pairs; names both others. | M3 |
| `surface.mount_face` | error | the mount | A face word on a kind mount. Hint: a number, 0 along travel, 180 against. | M3 |
| `surface.which` | error | the instance, or the path | `on` a thing, or a path end on a kind, with two or more surfaces; lists them. Hint: place it `on` a mount. | M3 |
| `surface.outside` | error | the instance | Its plan point lies inside the extent but outside the bound of the kind surface it stands on. An origin outside the extent is `place.outside` (`rules.md` 8.4), never this code. | M3 |
| `surface.host_missing` | error | the instance; its host in `related` | What it is placed `on`, or the owner of the mount it is placed on, was not placed (4.1). | M1 |
| `surface.through` | warning | the instance | Its height reaches through another surface above its origin; names it. | M3 |
| `surface.off_bound` | warning | the instance | Part of its footprint lies outside its surface's bound; the share. | M3 |
| `surface.ground_off_terrain` | error | the instance | `ground` on an instance that stands on a kind surface. | M3 |
| `surface.apart` | error | the meeting's `a`; `b` in `related` | Two paired mounts lie at different plan points; gives both. | M3 |
| `surface.gap` | error | the meeting's `a`; `b` in `related` | The two sides differ in height; both heights, the gap in millimetres, and what sets each (3.4). Hint: "make the two sides equal: change <a> or <b>, or add an effect of <owner> that sets the ground to <z>". | M3 |
| `surface.connector` | error | the meeting's `a`; `b` in `related` | Different connectors; names both. | M5 |
| `surface.facing` | error | the meeting's `a`; `b` in `related` | Two paired mounts do not face opposite ways; both bearings. | M5 |
| `surface.unmet` | warning | the mount | A mount with a connector is answered by nothing (R18: a stub). Hint: "end a path whose kind has connector <c> here: `ends_at = \"<place>\"`". | M5 |

## Conformance

Each case runs through the command line (R20), with `via query` for the questions (section 7).
Every case also checks that a clean build equals the incremental one (`graph.md`). Each case's
world, inputs and expected output live under `spec/vectors/surfaces/<case>/`. The cases share one
test library, the folder `spec/vectors/surfaces/lib/surfaces_test/`, whose `kinds.toml`
(`documents.md` 4) holds the `lane`, `crossing` and `pad` kinds of section 6. M1 cases run on
`ford` or a world of their own; each M3 case's world has `include = ["surfaces_test"]` and runs
with `--lib spec/vectors/surfaces/lib` (`build.md`). The
folder's contents are (open): written with the code checks (plan, M0 step 5).

| Case | Milestone | Input | Action | Must hold |
|---|---|---|---|---|
| terrain_only | M1 | `ford`; 100 points in `points.json` | `surfaces_at` | One row each, `height`; same bytes at 1 and 8 threads. |
| sample_edge | M1 | `ford`; a point on a cell edge, one on a post, one on the extent's east edge, the extent's north-east corner | `sample` | Equal to the vector, bit for bit; the edge points use the west and south cell. |
| relief_exact | M1 | a ramped terrain; a footprint whose extreme lies on an edge between posts | `relief` | `low`, `high` equal to the vector. |
| empty_region | M1 | a region of zero area | `relief` | `surface.empty_region`, exit 1. |
| two_terrains | M1 | two top-level surface fields | build | `surface.two_terrains`, exit 1. |
| host_outside | M1 | `ford`; instance `shed` `at = [300, 10]`; instance `barrow` `on = { ref = "shed" }`; instance `pail` `on = { ref = "barrow" }` | build | `place.outside` on `shed` only; `surface.host_missing` on `barrow` (related `shed`) and on `pail` (related `barrow`); none of the three placed; no `surface.outside`; exit 1. |
| ford_meetings | M1 | `ford` | `meetings`; then move one vertex of the beck, `[160, 90]`, 4 m east to `[164, 90]` | Two rows as in 6.2, `gap = 0`, at the ford's point (about `[153.63, 115.48]`); after the move, both rows at the new point (about `[156.40, 115.34]`; the vector holds the lattice values). |
| sdk_batch | M2 | a step calling each question with 10^5 points | step | One engine call per question; answers equal `via query`'s. |
| step_view | M2 | a generate step in layer 1; a crossing in layer 2 | step | `surfaces_at` on the deck gives `height` only. |
| too_early | M2 | a generate step calling `meetings()` | step | `surface.too_early` naming the step. |
| over_order | M3 | two crossings over one point with equal `deck_z`, ids `b_deck` and `a_deck` | `surfaces_at` | `height`, then `a_deck.deck`, then `b_deck.deck`. |
| sample_outside | M3 | 6.3; a point 4 m off the deck's line | `sample` on the deck | `inside = false`, all zeros, no diagnostic. |
| sample_narrow | M3 | a crossing with `width = 0.002` at resolution 0.5 | `sample` on its line | `gx = gy = 0` where both steps leave the strip. |
| relief_outside | M3 | a footprint half off the deck | build | `surface.off_bound`, share exactly 0.5. |
| owner_shape | M3 | a crossing kind on a point feature | build | `surface.owner_shape`, exit 1. |
| fold | M3 | a crossing whose path bends with inner radius 1 m, `width = 5` | build | `surface.fold` with the position and the largest bound. |
| name_clash | M3 | a kind with surface `deck` and mount `deck` | build | `surface.name_clash`. |
| meets_errors | M3 | `meets` to a missing mount, to itself, and a mount in two pairs | build | `surface.unknown_mount` listing the mounts, `surface.meets_self`, `surface.meets_twice`. |
| effect_terrain_only | M3 | 6.3 plus a region feature under the deck whose kind's effect on `role:surface` is `value = "= v - 1"` | build, `sample` | `height` falls 1 m there; `sample` on `mill_bridge.deck` at the same points is unchanged. |
| bridge | M3 | 6.3 | build, `meetings` | Four rows, every `gap = 0`; no post with `0 < t < 1` on the crossing's path changes when both lanes are removed. |
| bridge_gap | M3 | 6.3 without `heads` | build | `surface.gap` twice, each naming the deck's `base` line and the terrain's last writer; exit 1. |
| bridge_moved | M3 | 6.3, crossing path moved 20 m north | build | The change report names `mill_bridge`, `lane_west`, `lane_east` as moved, and `height` with its changed region; no mount or kind surface entry; nothing else. |
| apart | M3 | `meets` between mounts at `t = 0` and `t = 0.1` | build | `surface.apart`. |
| through | M3 | an instance `at` mid-river whose `bounds` top reaches above 24.5 | build | Grounded on `height`; `surface.through` naming `mill_bridge.deck`. |
| on_deck | M3 | an instance `on = { ref = "mill_bridge", t = 0.5, offset = 1 }` | build | `z = 24.48` to the millimetre. |
| outside | M3 | the same with `offset = 4` | build | `surface.outside`, instance not placed. |
| ground_on_deck | M3 | the same with `offset = 1` and `ground = "pad"` | build | `surface.ground_off_terrain`. |
| which | M3 | `on` a feature whose kind owns two surfaces | build | `surface.which` listing both. |
| deck_divide | M3 | a crossing with `base = "= 1 / d"` | `sample` on its line | Exit 1, `formula.not_finite` at that point; no output file. |
| connector | M5 | a lane kind with connector `rail` ending at a `way` head | build | `surface.connector`. |
| unmet | M5 | 6.3 with `lane_east` removed | build | `surface.unmet` on `mill_bridge.east_head`, exit 0. |
| facing | M5 | 6.3 with `east_head` `face = 0` | build | `surface.facing` with both bearings. |
| turn | M5 | 6.3 with `lane_west` arriving from the north-west | `meetings` | `turn` equals the vector; null on the two pair rows. |

## Changed from the spike

- Decks were a special kind key (`deck = {...}`) making a mesh outside every field. A deck is now a
  kind's surface: a field with an owner and a bound, with no special code.
- Floors came from `[[level]]` and `in`; the `bed` and `hole` rasters stood for a second surface.
  Floors are library surfaces (`storey`), and `surfaces_at` lists every surface at a point.
- `mount` (height above a floor), `atop` and `from_on` / `to_on` are gone. Height comes from the
  surface beneath. A path whose kind owns a surface and ends at another feature's mount is a meeting
  between two owners.
- A mount had a position only. It now has a surface, a face and (M5) a connector, and two mounts can
  be a pair.
- A way could not be interrupted by a deck (SPEC 12.8). A way now ends at the crossing's head, and
  the head's height is checked against the deck's end.
- A deck's ends were lined up with roads by hand and unchecked. Every meeting is checked to the
  millimetre.
- The reach lint read the top surface only. A reach library can now walk every surface and cross at
  meetings.
- `place.off_surface` becomes `surface.off_bound`, measured as an exact area share.
- Deck clearance (`clear`, measured at the banks too) leaves the core: headroom is a library lint
  over `surfaces_at`. Piers are a library rule.
- A wrong mount name gave `mount.unknown` with the mounts the owner has (FINDINGS). That lesson is
  kept as `surface.unknown_mount`.
- An asset mount loses the spike's "no path reaches its door" check until M5 (3.2). A regression,
  recorded in section 8.

## Open

1. **Key words.** `[kind.<k>.surface.<name>]`, `bound`, on a mount `surface`, `meets`, `connector`
   (also on a kind), and the questions `surfaces_at`, `sample`, `meetings`. Decided by the lead
   adding them, or other words, to the vocabulary before any code reads them.
2. **Milestone.** `vocabulary.md` section 5 puts kind surfaces and mounts in M3, as `docs/plan.md`
   rows 34, 38 and 63 (bridges, a loop ramp, a deck that meets the ground) need, once the lead
   accepts this point. Decided by the lead before M3 starts; this chapter assumes M3.
3. **A bound that varies along `t`** (a ramp that widens). Decided by challenge 38, the loop ramp.
4. **An allowed step at a meeting** (a kerb). Decided by the first case that needs one, in M3 or M5;
   until then a meeting is exact.
5. **Naming a surface other than the terrain from `at` or a scatter** (furniture on an upper floor,
   trees on a roof garden). Decided in M5 with `hosts`.
6. **The floors a volume yields** (`hosts`): kept as a word or folded into a kind's surface.
   Decided in M5 (`vocabulary.md` section 4).
7. **The terrain under a floor or a deck.** Whether export cuts it away (the spike's floor showed the
   terrain through it). Decided in `build.md` for M5.
8. **`path3`.** Recommended dropped: a path in three dimensions is a path whose kind owns a surface,
   and 3.3 lets its end rest on another deck. Confirmed or reversed by challenge 38.
9. **Reading a field from a surface formula** (a deck's height from the ground at its ends).
   Decided in `formulas.md`; until then 1.1 forbids it and the example writes `deck_z`.
10. **Caves and overhangs** (challenge 7). Bounded 3D fields or kind surfaces under the terrain.
    Decided in M5 with `coast`.
11. **A threshold.** A mount a fixed height above its surface (a raised door sill); today a mount's
    height is always its surface's. Decided by the first door in M5.
12. **Two region owners resting on each other** (a deck on a deck that is not a path's end). Not
    expressible today; decided by challenge 38 if it needs it.

## 8. Edits this chapter needs elsewhere

For the lead; this chapter does not edit other files.

| Chapter | Edit |
|---|---|
| `docs/doctrine.md` R25 | R31 settled the owner and the meetings. Still asked: a kind surface is a bounded field that no selector, row, effect or `f()` reaches (1, rule 5); R25 should say so, or "field" invites the reading that it does. |
| `vocabulary.md` (R31) | "Default mount" as 3.3 defines it. |
| `vocabulary.md` | Add the keys and questions of Open 1. Widen `base` to a kind surface. Selectors (`role:`, `tag:`) match world fields only. A kind mount's `face` is a number relative to travel; the "word without `on`" rule does not reach it. Change report: kind surfaces and mounts move with their owner and are not entries. Section 5: a second top-level surface is `surface.two_terrains`, not `doc.not_yet`; kind surfaces and mounts M3 (Open 2). 2.1: after a feature, a mount or a surface of its kind, one namespace. |
| `identity.md` | Kind surfaces are places; `start` and `end` are reserved for them too. |
| `fields.md` | The surface role row: a second surface field is `surface.two_terrains`. |
| `formulas.md` | Rows in 6.1 for surface `base` (rate per point) and `bound` (per feature), as 1.1. |
| `rules.md` | 8.2: measure relief with `surfaces.md`'s `relief()`, not the vertex, midpoint and origin samples; `place.relief` reads `max_relief`. |
| `assets.md` | "Sink and relief": `surfaces.md` measures, `rules.md` owns `place.relief`. |
| `geometry.md` | None needed: kind mount faces are numbers, so the word list is unchanged. |
| `build.md` | Section 14: `via query`'s row and the owner-flags line list its flags as section 7 does (`--in`, `--out`, `--change`, `--lib`, `--store`, `--sdk`, `--diagnostics`); it writes no build directory. |
| `docs/challenges.md` | Row 48: asset mounts have no connector until M5, so "no path reaches its door" is a library lint until then. |
