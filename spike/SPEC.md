# worldc: the contract the spike implements

This document states what the code in `spike/` does. It is a reference for the format, the rules, the
diagnostics and the outputs. It describes behaviour as implemented, including behaviour that is
limited or inconsistent; section 11 lists those cases.

## 1. Scope

`worldc` reads one world document written in TOML (plus the files it includes, its layers, overlays
and variants) and one asset pack, also TOML. It writes a build folder: binary rasters of the world's
fields, generated meshes, a JSON manifest that lists every placed instance, and JSON reports with
diagnostics, budgets and water measurements. A Godot 4 viewer in `viewer/` loads a build folder and
either renders its bookmarks or lets a person walk it.

## 2. Concepts

**Units and axes.** All lengths are metres. `x` is east, `y` is north, `z` is up. Angles are degrees.
A yaw is the direction an asset's front looks, counter-clockwise from east.

**Frame.** A frame is a coordinate system. There are four.
The *world frame* is the rectangle `extent = [x0, y0, x1, y1]` with a square sample grid of spacing
`resolution`. A grid sample is a *post*. There are `nx = round((x1 - x0) / resolution) + 1` posts
along x and `ny` along y. Post `(i, j)` is at `(x0 + i * resolution, y0 + j * resolution)`.
A *feature's local frame* gives, for any post, the distance to the feature (`d`), the signed distance
(`sd`) and, for a path, the fraction along it (`t`). A place in a local frame is written
`on = { feature, t, offset }`.
A *level* is a named floor height. Rooms, and what is placed in rooms, stand on a level.
An *instance's frame* has its origin at the instance, one axis to the instance's right and one axis
forward (the direction of its yaw). `atop`, `beside` and set items use it.

**Field.** A field is a named function over the world frame, resolved to an array.
A *scalar field* has one float per post. A *material field* has one weight per post for each of an
ordered list of ground materials. A *volume field* has one float per cell of a 3D grid, exists only
inside the box of a feature whose kind hosts it, and is solid where positive.

**Feature.** A feature is a point, a path or a region with an `id`, an optional `kind`, and free
attributes. A path is a polyline. A region is a closed polygon. A feature whose shape is computed from
a field (a flood) is a *derived feature*.

**Kind and effect.** A kind is a named table. It holds *effects*, default attribute values, and flags
that other compiler stages read (`hosts`, `encloses`, `opening`, `link`, `light`, `measure`, `map`).
An effect changes one field near every feature of the kind: `v = mix(v, value, mask)` for a scalar
or volume field, or "paint this material with weight `mask`" for a material field.

**Authority.** Authority is one of `derived`, `shaped` (the default) or `anchored`. It sets the order
in which effects reach a 2D field: derived first, shaped next, anchored last, so the last word
belongs to anchored features. A feature with `flood` is always `derived`. When two anchored features
have effects on the same scalar field that declares a `unit`, the compiler compares the values the two
effects ask for where both masks are above 0.05. Where the two differ by 0.05 or more it reports
`authority.conflict` if both masks exceed 0.5 at such a post, otherwise `authority.squeezed`. Neither
is reported when one feature names the other in `over`.

**Instance.** An instance is one asset at one position with a yaw, a uniform scale and a tint.
An *explicit* instance comes from the document (`[[instance]]`, a set, a run, a kit run, a dress row
or a flank row). A *scattered* instance comes from `[[scatter]]` and has no id.

**Asset and pack.** An asset is an opaque id. A pack is a TOML file that binds each asset id to a
primitive shape, a model file or an engine-side recipe, and declares the facts the compiler needs:
size, footprint, triangle count, and placement tolerances. A pack also declares ground materials and
surface materials.

**Producer.** A producer lowers a compact form to plain instances.
A *set* is a named group of items in one local frame, placed as a unit by an instance.
A *run* repeats one instance spec along a stretch of a feature's path or outline.
A *kit run* lays bays of one asset end to end between two points and adds one pad feature under them.
A *dress* row strews small assets at random on top of a placed instance.
A *flank* row places a pair of one asset either side of the front of each named host instance.
A *scatter* row turns a density expression into scattered instances on a jittered grid.

**Variant and overlay.** Both are documents of the same shape as a world, merged over it. An overlay
is a separate file named on the command line. A variant is a `[variant.<name>]` table inside the world.

**Layer.** A layer is a separate TOML file listed by the world in `[[layer]]`. Layers are ordered,
lowest first. Lists in a layer are joined to the world's lists and every item remembers its layer.
An item may refer to items in its own layer or a lower one.

**Part.** A part is another world placed inside this one. The world that places it is the *master*.
The part is compiled alone in its own frame and then translated. It meets the master at a boundary of
one of two types. A *seam* part brings terrain: inside its footprint the master's fields are blended
to the part's over an apron. A *portal* part brings an interior: nothing is blended, and one of the
part's doors is put at a point in the master.

**Bookmark.** A bookmark is a point feature of the kind named `bookmark`. It becomes a camera
position, a target and a field of view in the build.

**Expectation and profile.** An expectation is an `[[expect]]` row: from a start, a list of targets
that must be reachable (or must not be) by a named profile. A profile is a `[profile.<name>]` table
that states how steep and how deep the walker can go. Only the reach lint reads them.

## 3. The document format

A world is one TOML file. Unknown top-level keys are ignored. Unknown keys inside a feature or an
instance are passed through to the build.

### 3.1 Loading and merging

The document is built in this order:

1. Every `include` file is merged into an empty document. The world's own `include` list comes first,
   then each layer's, in layer order. Paths are relative to the file that names them. A file named
   twice is merged once. An included file's own `include` is not followed.
2. The world's own file is merged over that.
3. Each layer file is merged, lowest first (rules below).
4. Each `--overlay` file is merged, in command-line order.
5. Each `--variant` table (`[variant.<name>]` of the merged document) is merged, in order.
   An unknown variant name stops the run with the message `unknown variant '<name>'`.

Merge rule (includes, the world, overlays, variants): tables merge key by key. A list replaces the
list under it, with one exception: when the incoming list is not empty and every item in it is a
table with an `id`, items are merged by `id`. An incoming item with `remove = true` deletes the item
with that id. An item with a known id merges into it. Any other item is appended. Lists whose items
have no id (`effect`, `layers`, `[[expect]]`, coordinates) are therefore replaced whole.

Layer rule: a top-level list of tables is appended to the document's list of the same name, and each
item gets the layer's id. A top-level table merges into the table of the same name by the merge rule.
Any other value replaces. If two layers define the same `id` under the same list name, the run stops
with a message naming both layers.

| Key | Type | Default | Meaning |
|---|---|---|---|
| `include` | list of paths | `[]` | Files merged before the world's own text. Read from the world file and from layer files only. |

### 3.2 `[world]`

| Key | Type | Default | Meaning |
|---|---|---|---|
| `extent` | `[x0, y0, x1, y1]` | required | The world rectangle, m. |
| `resolution` | float | `0.5` | Post spacing, m. |
| `seed` | int | `0` | Seed mixed into every noise function and every random producer. |
| `chunk` | float | `64` | Side of a square chunk, m. Budgets and the instance list are grouped by chunk. |
| `pack` | path | required unless `--pack` | The asset pack, relative to the world file. |
| `name` | string | `"world"` | Written to `world.json`. |

### 3.3 `[params]`, `[look]`, `[budget]`, `[export]`, `[lint]`

`[params]` is a table of names and values. Every name is available in every expression (section 4).
The table is written to `world.json` unchanged.

`[look]` is not read by the compiler. It is written to `world.json` unchanged. The viewer reads the
keys in section 10.4.

| Key | Type | Default | Meaning |
|---|---|---|---|
| `budget.max_instances_per_chunk` | int | none | A chunk with more instances raises `budget.exceeded`. |
| `budget.max_source_tris_per_chunk` | int | none | A chunk whose instances sum to more source triangles raises `budget.exceeded`. |
| `export.fields` | list of field names | `[]` | Scalar fields written as `<name>.f32` besides `height`. |

There is no `[lint]` section. Lints are chosen on the command line only.

### 3.4 `[field.<name>]`

| Key | Type | Default | Meaning |
|---|---|---|---|
| `type` | string | scalar | `"material"` or `"volume"`. Any other value, or none, is a scalar field. |
| `unit` | string | none | Any non-empty value marks the field as measured. Only then are anchored effects on it compared (`authority.conflict`, `authority.squeezed`). |
| `tags` | list of strings | `[]` | An effect whose `field` is `"#<tag>"` targets every 2D field with that tag. |
| `let` | table of expressions | `{}` | Named sub-expressions, evaluated in order. Each sees the ones before it. |
| `base` | expression | `0.0` (scalar), `"ground"` (volume) | The field before layers and effects. Not read for a material field. |
| `layers` | list of tables | `[]` | Applied in order after `base`. Not read for a volume field. |
| `cell` | float | `0.75` | Volume fields only: cell size of the 3D grid, m. |

A layer of a scalar field:

| Key | Type | Default | Meaning |
|---|---|---|---|
| `value` | expression | required | The target value. `v` is the field so far. |
| `mask` | expression | `1.0` | Clipped to 0..1. `v = v + (value - v) * mask`. |

A layer of a material field:

| Key | Type | Default | Meaning |
|---|---|---|---|
| `paint` | string | required | A ground material name. |
| `mask` | expression | `1.0` | Clipped to 0..1. Every weight is multiplied by `1 - mask`, then `mask` is added to the painted material. |

The material list of a material field is: each `paint` of its layers in order, then each `paint` of
every effect that targets the field, in kind order, but only for kinds that at least one feature uses.

### 3.5 `[kind.<name>]`

| Key | Type | Default | Meaning |
|---|---|---|---|
| `authority` | string | `"shaped"` | Authority of the kind's features unless a feature sets its own. |
| `reach` | float | `48.0` | Distance, m, beyond which the kind's effects are not evaluated. An effect may set its own. |
| `defaults` | table of numbers | `{}` | Attribute values a feature of the kind has unless it sets them. Each is a name in the kind's expressions. |
| `let` | table of expressions | `{}` | Named sub-expressions for the kind's effects, evaluated in order for each effect. |
| `effect` | list of tables | `[]` | The effects (below). |
| `map` | table | none | Draws the kind's features on `minimap.png`. `color` (`[r, g, b]` 0..1, default white) and `width` (px, default 2, paths only). |
| `hosts` | string | none | Name of a volume field. A region feature of this kind is a box of that volume. |
| `encloses` | bool | false | Features of this kind are rooms. |
| `opening` | string | none | Features of this kind are openings in a room's wall. `"door"` is a door; any other value is a window. |
| `link` | string | none | `"stair"`: features of this kind are stairs. |
| `light` | table | none | Point features of this kind emit a light: `color` (default `[1.0, 0.8, 0.55]`), `range` (m, `7.0`), `energy` (`1.6`), `h` (m above the level, `2.2`). Read only when the world has levels. |
| `measure` | string | none | `"water"`: features of this kind are measured as water bodies in `report.json`. |
| `module_m` | float | none | Rooms only: wall module, m, checked by `kit.remainder`. |

Attribute names the compiler itself reads from a feature or its kind's `defaults`:

| Attribute | Default | Read by |
|---|---|---|
| `grade` | `24.0` | Length, m, over which a path's `profile` is averaged. |
| `max_grade` | none | Percent. A path steeper than this after averaging raises `seam.grade`. |
| `width` | `2.0` / `1.0` | Target path width in the `joins` check (2.0); opening width (1.0). |
| `sill`, `head` | `0.0`, `2.1` | Bottom and top of an opening above the level, m. |
| `wall` | `0.4` | Wall thickness of a room, m. |
| `margin` | `0.0` | From `defaults` only: how far an instance's footprint feature extends past its footprint, m. |

An effect:

| Key | Type | Default | Meaning |
|---|---|---|---|
| `field` | string | `""` | A field name, or `"#<tag>"`. An effect on a field the world does not declare is never applied. Volume fields are matched by name only. |
| `value` | expression | required for scalar and volume | The target value. |
| `mask` | expression | `1.0` | Clipped to 0..1. Set to 0 beyond `reach` (2D only). |
| `paint` | string | required for material | The ground material to paint. |
| `reach` | float | the kind's `reach` | Overrides the kind's reach for this effect. Not read for volume fields. |
| `max_change` | number or string | none | Limit on `abs((value - v) * mask)`. A string is evaluated with the feature's attributes as the only names. Exceeding it raises `seam.change_exceeds`; the change is still applied. Scalar fields only. |
| `check_where` | expression | everywhere | The `max_change` limit applies only where this is greater than 0. |

The shared library in `kinds/` declares these kinds, in this order per file:
`terrain.toml`: guide, cliff, lake, stack, pad, road, terrace, lawn. `rock.toml`: rockwork, cave.
`building.toml`: room, tower_room, door, window, stair, lamp. `review.toml`: bookmark, poi.
The library is data: the compiler has no built-in kind. It reads only the flags and names above.

### 3.6 `[[feature]]`

Geometry. Exactly one source is used, in this order of precedence: `path`, then `region` or `ellipse`,
then `at`.

| Key | Type | Default | Meaning |
|---|---|---|---|
| `id` | string | required | Unique among features. Instance footprints and kit-run pads share this namespace. |
| `kind` | string | none | A declared kind. An undeclared kind raises the info `kind.passthrough`; the feature is still emitted. |
| `path` | list of `[x, y]` | | A path. Resampled every 0.25 m. |
| `smooth` | bool | `true` (`false` with `path3`) | Catmull-Rom smoothing of a path with more than two points. |
| `path3` | list of `[x, y, z]` | | A 3D path. Its plan becomes `path` if none is given. Volume effects read the 3D line. |
| `region` | list of `[x, y]` | | A closed polygon. The outline is stored counter-clockwise. |
| `ellipse` | `{ center = [x, y], size = [dx, dy] }` | | A region: a 72-sided polygon. `size` is the two full diameters. Used when `region` is absent. |
| `at` | `[x, y]` | | A point. |
| `on` | `{ feature, t, offset }` | | A point at fraction `t` (default `0.5`) of the arc length of another feature's path or outline, moved `offset` m (default `0`) along the normal. The normal is to the left of a path and outward from a region. Sets `at`. |
| `flood` | table | | Makes the feature a derived region (below). Needs `at`. |
| `joins` | list of feature ids | `[]` | Declares that one end of this path lies on each named path. Checked (section 6). |
| `authority` | string | the kind's | `derived`, `shaped` or `anchored`. |
| `over` | list of feature ids | `[]` | Silences `authority.conflict` and `authority.squeezed` against the named anchors. The number of entries also orders effects (section 5.2). |

`flood` table:

| Key | Type | Default | Meaning |
|---|---|---|---|
| `field` | string | `"height"` | The scalar field the flood reads. |
| `level` | number or `"spill"` | `"spill"` | The water level, m. `"spill"` finds the highest level the ground holds inside the fence. |
| `fence` | float | `120.0` | Radius, m, about `at` inside which the water must stay. |
| `max_depth` | float | `8.0` | `"spill"` only: the search stops this far above the ground at `at`. |
| `freeboard` | float | `0.3` | `"spill"` only: the level is set this far below the spill level. |

After flooding, the feature has a `region` (the largest closed shore loop), an exact cell mask, and a
numeric attribute `level`.

Attributes read from features of particular kinds:

| Key | Applies to | Type | Default | Meaning |
|---|---|---|---|---|
| `z` | host of a volume | `[z0, z1]` | required | The box's height range, m. |
| `apron` | host of a volume | float | `6.0` | Width, m, inside the region's edge over which the volume blends back to the ground. |
| `level` | room, light | level id | required for rooms | The level the room stands on. For a light, an unknown level counts as z = 0. |
| `to` | door | room id or `"outside"` | `"outside"` | What the door leads to. |
| `from`, `to` | stair | room ids | required | The two rooms a stair joins. Steps rise from `from` to `to`. |
| `arc` | stair | `{ center, radius, from_deg, to_deg }` | | A curved stair. Treads lie between `radius - width` and `radius`. |
| `line` | stair | `[[x, y], [x, y]]` | | A straight stair. Read when `arc` is absent. |
| `riser` | stair | float | `0.19` | Target step height, m. Step count is `max(3, round(rise / riser))`. |
| `width` | stair | float | `1.0` | Tread width, m. Read from the feature only, not from kind defaults. |
| `h` | light | float | the kind's `light.h` | Height of the light above its level, m. |
| `look` | bookmark | `[x, y]` or `{ feature, t }` | 50 m north of the eye | The target. The table form needs a path feature; `t` defaults to `0.5`. |
| `eye` | bookmark | float | `1.7` | Eye height above its ground, m. |
| `look_height` | bookmark | float | `1.5` | Target height above the ground at the target, m. |
| `fov` | bookmark | float | `55.0` | Field of view, degrees. |
| `note` | bookmark | string | `""` | Free text. |
| `under` | bookmark | bool | false | Stand on the `bed` field (the lowest floor) instead of `height`. |
| `cut_z` | bookmark | float | none | The viewer hides generated meshes whose base is at or above this height. Also disables the view checks. |
| `in` | any feature | room id | none | The feature gets `floor_z`, the room's level height. A bookmark with it is indoors. |
| `label` | kind `poi` | string | `""` | Label of the icon in `map.json`. |

Every numeric attribute of a feature (int or float, not bool) is a name in the expressions of its
kind and overrides the kind's `defaults`. Every key of a feature, read or not, is written to
`world.json` except the geometry keys.

### 3.7 `[[instance]]`

| Key | Type | Default | Meaning |
|---|---|---|---|
| `id` | string | `instance_<n>` | `n` is the index in the expanded spec list. Required when `set` is used. |
| `asset` | asset id | required | Unknown ids raise `asset.unknown`; the instance is dropped. |
| `at` | `[x, y]` | | World position. |
| `yaw` | float | `0.0` | With `at` only. |
| `on` | `{ feature, t, offset, slide }` | | Position in a feature's frame. `t` default `0.5`, `offset` along the normal (default `0`), `slide` along the tangent in m (default `0`). Takes precedence over `at`. |
| `face` | string or number | `"toward"` | With `on` only. `"toward"`: face the feature's line. `"in"`: face against the normal (into a region). `"along"`: face along the tangent. A number is an absolute yaw. |
| `turn` | float | `0.0` | Added to the yaw from `face`, or to the host's yaw with `atop` and `beside`. Not read with `at`. |
| `in` | room id | none | The instance stands on that room's level and is checked against the room (section 6.5). |
| `atop` | instance id | | Stand on top of an instance placed earlier, at `offset` in its frame. Height is the host's floor height plus the host asset's height times its scale. Inherits the host's `in`. |
| `beside` | instance id | | Stand on the ground at `offset` in the frame of an instance placed earlier. |
| `offset` | `[right, forward]` | `[0, 0]` | With `atop` and `beside`: metres to the host's right and to its front. |
| `against` | bool | false | With `on`: stand inside the feature with the back to its edge. Sets the offset to minus half the footprint depth minus 0.03 m and `face` to `"in"` unless given. On a region, moves further in by 0.02 m steps (at most 25) until the footprint is inside. Replaces any `on.offset`. |
| `mount` | float | `0.0` | Height above the floor, m. Applies only when the instance has a floor (`in` or `atop`). |
| `scale` | float | `1.0` | Uniform scale. Scales the footprint and the asset's `sink`. |
| `tint` | `[r, g, b]` | `[1, 1, 1]` | Colour multiplier. |
| `kind` | kind name | none | The instance also becomes a region feature of this kind with the instance's id (below). |
| `margin` | float | the kind's `defaults.margin` or `0` | With `kind`: the feature's rectangle extends this far past the footprint. |
| `set` | set name | none | Place the named set here instead of one asset. |
| `group` | string | none | Instances with the same group are not tested against each other for overlap. |

Order of precedence for position: `atop`, then `beside`, then `on`, then `at`. With none of these the
instance raises `instance.no_position`.

Any other key is a free attribute. Free attributes are written to the build as the instance's
`attrs` and are not interpreted.

An instance with `kind` adds a feature: a rectangle of the scaled footprint plus `margin`, turned to
the instance's yaw (a rectangle also for round assets). The feature copies every key of the instance
except `asset`, `at`, `on`, `face`, `yaw`, `scale` and `tint`, so numeric attributes, `authority` and
`over` written on the instance reach the kind. This feature is not written to `world.json`.

### 3.8 `[set.<name>]`

| Key | Type | Default | Meaning |
|---|---|---|---|
| `items` | list of tables | `[]` | The set's members. |
| `items[].name` | string | required | The member's instance id is `<instance id>.<name>`. |
| `items[].asset` | asset id | required | |
| `items[].offset` | `[right, forward]` | `[0, 0]` | In the set's frame, or in the host's frame with `atop`. |
| `items[].atop` | member name | none | Stand on that member of the same set. |
| `items[].turn` | float | `0.0` | Added to the set's yaw or the host's yaw. |

The placing instance gives the set its position and yaw (`at` and `yaw`, or `on`) and its `in`.
Its other keys do not reach the members.

### 3.9 `[[run]]`

| Key | Type | Default | Meaning |
|---|---|---|---|
| `id` | string | `run_<n>` | Children are `<id>.<k>`, `k` from 0. |
| `on` | feature id (a string) | required | The path or region outline to run along. |
| `from`, `to` | float | `0.0`, `1.0` | The stretch, as fractions of the arc length. |
| `count` | int | `1` | Number of children. Child `k` is at `t = (from + (to - from) * (k + 0.5) / count) mod 1`. |
| `offset` | float | none | With it: each child is offset this far along the normal and `face` defaults to `"along"`. Without it: `against` defaults to `true`. |

Every other key (`asset`, `in`, `mount`, `scale`, `kind`, free attributes) is copied to each child as
if written on an `[[instance]]`. `group` defaults to the run's id.

### 3.10 `[[kitrun]]`

| Key | Type | Default | Meaning |
|---|---|---|---|
| `id` | string | `kitrun_<n>` | Bays are `<id>.<k>`. The pad feature has the id itself. |
| `bay` | asset id | required | The asset repeated. |
| `from`, `to` | `[x, y]` | required | The two ends of the run. |
| `faces` | string | `"left"` | `"left"`: bays face 90 degrees left of the run direction. Any other value: right. |
| `tolerance` | float | `0.01` | Allowed misfit as a share of the length; the floor is 0.08 m. |
| `margin` | float | `1.0` | The pad extends this far past the run on every side, m. |
| `kind` | kind name | `"pad"` | Kind of the one feature laid under the whole run. |

The module is the asset's `module_m`, or its footprint width. The bay count is
`max(1, round(length / module))`. Bays are spread evenly over the length and share the group `<id>`.

### 3.11 `[[dress]]`

| Key | Type | Default | Meaning |
|---|---|---|---|
| `id` | string | `dress_<n>` | Items are `<id>.<k>`. |
| `atop` | instance id | required | The host: an explicit instance of this world. |
| `assets` | list of `{ id, weight }` | required | Choices. `weight` defaults to `1.0`. |
| `count` | int | `3` | Items to place. An item that finds no free spot in 30 tries is skipped silently. |
| `gap` | float | `0.16` | Least distance between items, m. |
| `margin` | float | `0.08` | Items keep this far inside the host's footprint, m. |
| `seed` | int | the row's index | Random seed, mixed with the world seed. |

Each item is placed `atop` the host at a random offset with a random `turn`.

### 3.12 `[[flank]]`

| Key | Type | Default | Meaning |
|---|---|---|---|
| `id` | string | `flank_<n>` | Items are `<id>.<host>.l` and `<id>.<host>.r`. |
| `hosts` | list of instance ids | `[]` | Each host gets one pair. |
| `asset` | asset id | required | |
| `spread` | float | `1.8` | Distance of each item to the left and right of the host's centre line, m. |
| `forward` | float | `0.9` | Distance in front of the host's front edge, m. |

Each item is placed `beside` its host and takes the host's yaw.

### 3.13 `[[scatter]]`

| Key | Type | Default | Meaning |
|---|---|---|---|
| `id` | string | `scatter_<n>` | Written on each instance as `by`. |
| `assets` | list of `{ id, weight }` | `[]` | Choices. `weight` defaults to `1.0`. Unknown ids raise `asset.unknown` and are left out. |
| `spacing` | float | required | Side of a grid cell, m. One candidate per cell, jittered between 10% and 90% of the cell. |
| `density` | expression | `1.0` | Clipped to 0..1. The chance that a candidate is kept. |
| `scale` | `[lo, hi]` | `[1.0, 1.0]` | Uniform random scale. |
| `tint` | `[lo, hi]` | `[0.86, 1.08]` | Each colour channel is uniform in this range, times one common factor in 0.94..1.06. |
| `seed` | int | the row's index | Random seed, mixed with the world seed. |

A candidate is also dropped when it lies within about 1 m of the footprint of a solid explicit
instance of this world, or where a field named in its asset's `avoid` list is 0.5 or more.
Yaw is uniform in 0..360.

### 3.14 `[[level]]`

| Key | Type | Default | Meaning |
|---|---|---|---|
| `id` | string | required | |
| `z` | float | required | Floor height, m. |
| `height` | float | required | Wall height, m. |
| `wall` | surface name | `"plaster"` | Material of the level's wall mesh. |
| `floor` | surface name | `"timber"` | Material of the level's floor mesh. |

Interiors are built only when at least one level is declared.

### 3.15 `[[layer]]`

| Key | Type | Default | Meaning |
|---|---|---|---|
| `id` | string | required | The layer's name. Order of rows is the order of layers, lowest first. |
| `file` | path | required | Relative to the world file. |

Read from the world's own file only.

### 3.16 `[[part]]`

| Key | Type | Default | Meaning |
|---|---|---|---|
| `id` | string | required | Prefix of everything the part brings. |
| `file` | path | required | The part's world file, relative to the master's file. |
| `pack` | path | the master's pack | The pack the part is compiled with, relative to the master's file. |
| `portal` | `{ door, at = [x, y] }` | | Makes the part a portal part. `door` is a feature id in the part; `at` is where it lands in the master. |
| `at` | `[x, y]` | required for seam | Translation of the part's origin in the master. |
| `footprint` | `[x0, y0, x1, y1]` | the part's extent | Seam: the rectangle of the part, in the part's coordinates, that is brought. |
| `apron` | float | `20.0` | Seam: width, m, inside the footprint's edge over which fields blend. |
| `max_step` | float | `3.0` | Seam: limit on the height difference at the footprint's edge, m. |
| `joins_at` | `[x, y]` | none | Seam: a point in the part's coordinates where the part's height is set equal to the master's. |
| `expects` | string | `"land"` | Seam: with `"land"`, water on the master at the edge raises `part.context`. |
| `max_relief` | float | `0.4` | Portal: limit on the master's height range under the part, m. |

A row with `portal` is a portal part; any other row is a seam part. A part is compiled without
overlays, variants or lints. A part's own `[[part]]` rows are ignored.

### 3.17 `[profile.<name>]` and `[[expect]]`

| Key | Type | Default | Meaning |
|---|---|---|---|
| `profile.<name>.max_slope_deg` | float | required | Steepest ground the profile can stand on, degrees. |
| `profile.<name>.max_wade_m` | float | `0.0` | Deepest water it can stand in, m. |
| `expect[].from` | id | required | Start: a feature that has `at`, or an instance. |
| `expect[].to` | list of ids | `[]` | Targets: features that have `at`, or instances. |
| `expect[].by` | profile name | required | |
| `expect[].reach` | bool | `true` | `false` states that the targets must not be reachable. |
| `expect[].why` | string | `""` | Quoted in diagnostics. |

### 3.18 `[variant.<name>]`

A table with the shape of a world document. `--variant <name>` merges it over the document.

## 4. Expressions

An expression is a TOML number or a TOML string. A number is a constant. A string is evaluated with
Python `eval` over numpy arrays, with whitespace collapsed and with no Python built-ins. Names are
looked up in the tables below. A name defined later in a list overrides an earlier one.

Expressions appear in: a field's `base`, `let` and layers (`value`, `mask`); a kind's `let`; an
effect's `value`, `mask`, `check_where` and string `max_change`; a scatter's `density`.

### 4.1 Names in every 2D expression

Field `base`, field `let`, field layers, effects on 2D fields, and scatter `density` all have:

| Name | Meaning |
|---|---|
| `x`, `y` | Post coordinates, m. |
| `pi` | 3.14159... |
| `sin`, `cos`, `abs`, `sqrt`, `exp` | Element-wise functions. |
| `pow(a, b)` | Element-wise power. |
| `min(a, b)`, `max(a, b)` | Element-wise, two arguments. |
| `clamp(v, lo, hi)` | |
| `where(cond, a, b)` | |
| `smoothstep(a, b, v)` | 0 at `a`, 1 at `b`, cubic between. |
| `mix(a, b, t)` | `a + (b - a) * t`. |
| `fbm(wavelength, octaves=4, seed=0, gain=0.5, lacunarity=2.0)` | Gradient noise summed over octaves, about -1..1. `wavelength` in m. |
| `ridged(wavelength, octaves=4, seed=0, gain=0.5, lacunarity=2.0)` | Ridged noise, 0..1. |
| `f('name')` | Another field, resolved on demand. |
| `slope('name')` | Slope of a scalar field, degrees. |
| `dist('id')` | Distance to a feature, m; 0 inside a region. |
| `sdist('id')` | Signed distance: negative inside a region, positive to the left of a path. For a point, equal to `dist`. |
| `along('id')` | Fraction along a path, at the nearest path point. Paths only. |
| `near('id', radius)` | `1 - smoothstep(0, radius, dist('id'))`. |
| every `[params]` key | Its value. Overrides the names above. |

### 4.2 Extra names by context

| Context | Extra names |
|---|---|
| Field `let` | The `let` names before it. |
| Scalar field `base` | The field's `let` names. |
| Scalar field layer | The field's `let` names; `v`, the field so far. |
| Material field layer `mask` | The field's `let` names. No `v`. |
| Scatter `density` | None. |
| Effect on a 2D field | See below. |
| Effect `max_change` (string) | Only the feature's attributes. None of the names in 4.1. |

Names in an effect's `value`, `mask` and `check_where` on a 2D field, and in the kind's `let`:

| Name | Meaning |
|---|---|
| each attribute | The kind's `defaults` overlaid with the feature's numeric attributes. |
| `d` | Distance outside the feature, m; 0 inside a region or on a path. |
| `sd` | Signed distance, as `sdist`. |
| `t` | Fraction along the path at the nearest path point; `0.0` for a region or a point. |
| `v` | The field as it stands, before this effect. Scalar fields only. |
| `profile` | Paths, scalar fields only: the field sampled along the path, averaged twice with a moving window of `grade` metres, taken at the nearest path point. |
| `base`, `base_min`, `base_max` | Scalar fields only: mean, minimum and maximum of the field under the feature. Path: over `profile`. Region: over posts with `sd <= 0`. Point: the one sample. |
| `under('field', how='mean')` | Another resolved field measured under this feature. `how` is `'mean'`, `'min'`, `'max'` or `'edge_min'` (the lowest value within 1.5 posts of a region's outline). |
| the kind's `let` names | Evaluated in order. A `let` that reads a name this context lacks is skipped without a diagnostic. |

All arrays in an effect cover only the window of the grid within `reach` of the feature.

### 4.3 Names in a volume expression

A volume field's `base`, and the `value`, `mask` and kind `let` of an effect on a volume field, are
evaluated on the 3D grid of one host box. Arrays are indexed `[z, y, x]`.

| Name | Meaning |
|---|---|
| `x`, `y`, `z` | Cell coordinates, m. |
| `ground` | `height(x, y) - z`: positive below the heightfield. |
| `steep` | Slope of `height` at the cell's column, degrees, 0..90. |
| `crest` | The highest `height` within a square of about 10 m around the column, minus `z`. |
| `min`, `max`, `abs`, `clamp`, `where`, `smoothstep`, `mix`, `sin`, `cos`, `sqrt` | As in 4.1. |
| `fbm3(wavelength, octaves=3, seed=0)` | 3D noise. |
| `fbm2(wavelength, octaves=2, seed=0)` | 2D noise, constant in `z`. Not normalised. |
| `f('name')` | A 2D field sampled at the column. |
| every `[params]` key | |
| each attribute | Effects only: the feature's attributes. |
| `v` | Effects only: the volume as it stands. |
| `d`, `sd` | Effects only: the feature's 2D distance and signed distance at the column. |
| `tube` | Effects of `path3` features: 3D distance to the path, capped at 40 m. |
| `pd` | Effects of `path3` features: distance to the path in plan, m. |
| `pz` | Effects of `path3` features: the path's height at the nearest plan point, m. |
| `t` | Effects of `path3` features: fraction along the path at the nearest plan point. |

`pi`, `exp`, `pow`, `fbm`, `ridged`, `slope`, `dist`, `sdist`, `along`, `near`, `under`, `profile` and
`base` do not exist in a volume expression.

### 4.4 Failure

| Failure | 2D result | Volume result | Diagnostic |
|---|---|---|---|
| The expression reads an undeclared field | 0 | `ground` | `field.unknown` |
| The expression reads an unknown feature | 0 | `ground` | `feature.unknown` |
| The expression closes a cycle of fields | 0 | `ground` | `field.cycle`, once per cycle |
| Any other error | 0 | `ground` | `expr.invalid` |

The compile continues after each of these.

## 5. The compile pipeline

### 5.1 Steps

`compile_world` runs these steps in this order. A part runs the same steps with the differences noted.

1. **Lints named.** An unknown lint name stops the run before any work.
2. **Document.** Load and merge (section 3.1).
3. **Pack.** Load the pack: `--pack`, else `[world].pack`. For a part: the row's `pack`, else the
   master's pack. Raises `pack.*`.
4. **Layer directions.** Raises `layer.direction` (section 6.1).
5. **World.** Build the grid. Give every feature without `on` and without `flood` its shape. Then, in
   document order, resolve every feature with `on`: one on a derived feature waits; any other gets
   its point now. Raises `feature.*`, `frame.unknown`, `kind.passthrough`.
6. **Parts loaded.** Master only. Each `[[part]]` is compiled completely by these same steps. Its
   pack's assets, surfaces and ground materials are added to the master's pack where the master has
   none of that name. Its errors and warnings are copied to the master with the subject prefixed by
   `<part id>.` and without positions; its infos are dropped. A part with an unresolved field, a seam
   part without a `ground` field, and a portal part without its door are left out.
7. **Producers expanded.** Kit runs, then runs, then `[[instance]]` rows (sets expanded in place)
   become one list of plain instance specs. Each kit run adds its pad feature now.
8. **Placement, first phase.** Every spec that is not `on` a derived feature is placed, in list order.
   A spec with `kind` adds its footprint feature now, before any field is resolved.
9. **Height.** `height` is resolved: base, layers, then every effect on it (section 5.2).
   Then, master only, each part meets the height (section 6.7): a seam part's height is blended in.
   Then volumes are built (section 6.6); they rewrite `height` inside their regions and create the
   fields `bed` and `hole`. Then every flood not yet derived is derived, and features `on` a flood
   get their points. If `height` cannot be resolved this raises `field.unresolved`.
10. **Placement, second phase.** Specs `on` a derived feature are placed. Their footprint features
    are added after `height` is final, so their kind's effects on `height` are never applied.
11. **Dress and flank.** Their items are placed like explicit instances.
12. **Remaining fields.** Every non-volume field is resolved in the order the document declares
    them. A field that an earlier expression already read is already resolved.
13. **Stop conditions.** A part with any unresolved field stops here and is unsound. A master
    without a resolved `height` and `ground` raises `field.required` and stops; nothing is written.
14. **Part fields.** Master only. Inside each seam part's footprint the master's 2D fields and
    ground weights are blended to the part's. Then the ground materials are checked against the pack
    and the limit of eight (`ground.unknown`, `ground.too_many`).
15. **Scatter.** Every `[[scatter]]` row produces instances. Raises `scatter.count` (info).
16. **Structural checks.** Overlap, relief and `avoid` on explicit instances; `joins` on paths.
17. **Grounding.** Every instance gets `z`: its floor height if it has one, else `height` at its
    position; minus the asset's `sink` times the instance's scale.
18. **Interiors.** Rooms, openings, stairs and lights are built and checked (section 6.5).
19. **Parts brought.** Master only. Each part's instances, features, expectations, profiles,
    generated meshes, volume meshes, lights and rooms are translated and added (section 6.7).
    Brought instances are grounded as in step 17.
20. **Indoor features.** Every feature of this world whose `in` names a room gets `floor_z`.
21. **Bookmarks, budgets, water bodies.** Master only, in this order (sections 6.8 and 9).
22. **Lints.** Only when writing: each named lint once, in the order asked.
23. **Outputs.** Only when writing: every file of section 9; then the map folder with `--map`.

Every diagnostic except a lint's is raised before the first file is written. A build is written even
when there are errors, except in the `field.required` case.

### 5.2 Ordering rules that affect results

**Fields resolve on demand.** A field is resolved the first time it is asked for, and once. If field
A's expression reads `f('B')`, B is resolved, with all its effects, in the middle of resolving A.

**Order of effects on one 2D field.** After the field's base and layers, the compiler lists every
(effect, feature) pair that targets the field and sorts the pairs by, in order:

1. authority tier: derived, shaped, anchored;
2. the number of entries in the feature's `over` list, fewer first (in every tier);
3. anchored regions only: polygon area, larger first, so the smaller anchor is applied later;
4. the kind's position in the document's kind table;
5. the feature's position in the feature list.

Pairs equal on all five keep effect order. The kind table's order is: kinds from included files in
include order, then kinds first declared by the world or a layer. Overriding part of a kind does not
move it. The feature list's order is: document features, then kit-run pads, then instance footprints
in placement order.

**The rule among anchors.** Anchored effects are applied last, so each sees the result of those
before it: a small anchor inside a large one takes its `base` from ground the large one has already
shaped. Conflicts between anchors are detected as described under Authority in section 2.

**Derived features.** A flood reads its field after that field is completely resolved. Its kind's
effects on that same field are skipped (info `authority.conforms`). Its effects on other fields are
applied in the derived tier. Before any field other than a flood's source is given its effects, every
flood whose source is not being resolved at that moment is derived. `dist`, `sdist`, `along` and
`near` on a flood's id derive that flood on demand.

**Two-phase placement.** Instances are placed before fields are resolved, so that a footprint
feature can shape `height`. Instances `on` a derived feature cannot be placed until `height` is
final, so they are placed afterwards. Such an instance whose kind has an effect on the flood's source
field (by exact field name) raises `authority.cycle` and is dropped.

**Placement order.** `atop` and `beside` find only instances placed earlier. The order is: kit-run
bays, run children, `[[instance]]` rows and set members in document order (first phase), then the
second phase, then dress items, then flank items.

**Volumes.** Effects on a volume field are applied in kind order, then effect order, then feature
order. Authority, `over` and `reach` are not consulted.

## 6. Rules and checks

### 6.1 Document

- **Layer direction.** For every item of `feature`, `instance`, `run`, `kitrun`, `dress`, `flank` and
  `expect` that came from a layer, these references are read: `beside`, `atop`, `in`, `on` (the id),
  `look.feature`, each id in `joins` and `hosts`, and for `expect` rows `from` and each id in `to`.
  A reference's home is the layer of the item with that id, or of the id before its first dot.
  A reference to a higher layer raises `layer.direction`. Ids inside expressions are not read.

### 6.2 Features

- A feature needs an id (`feature.no_id`) and a unique id (`feature.duplicate`; the later one gets
  no shape). It needs `path`, `region`, `ellipse` or `at` (`feature.shape`).
- A feature whose bounding box lies wholly outside the extent raises `feature.outside` and is kept.
- `on` needs a host with a path or an outline (`frame.unknown`; the feature gets no shape).
- A feature without a shape takes no part in anything later and is not written.
- **Joins.** For each id in `joins`, both features must be paths (`frame.unknown`). The nearer end of
  this path must be within `0.5 * width + 1.0` m of the target path, where `width` is the target's
  attribute (default 2.0). Otherwise `seam.junction`.

### 6.3 Effects on 2D fields

- **Bounded masks.** If an effect's mask exceeds 0.02 on the border of its window, and the window
  does not touch the world's edge, the effect raises `effect.unbounded`. The effect is applied.
- **Limits.** `max_change`, optionally restricted by `check_where`, raises `seam.change_exceeds`.
- **Grade.** For a path feature with `max_grade`, the gradient of `profile` in percent is compared
  with it for every scalar field one of its kind's effects targets. Exceeding it raises `seam.grade`.
- **Authority.** An authority that is not one of the three raises `authority.unknown` and the pair is
  skipped. Anchors are compared as in section 2.

### 6.4 Floods

- The source field must be declared (`field.unknown`).
- The flooded body is the 4-connected set of posts below the level that contains the post at `at`.
- `"spill"`: if at 0.05 m above the ground at `at` the body already passes the fence, the flood
  raises `water.no_basin` and gets no shape. Otherwise 22 bisection steps find the highest level
  whose body stays inside the fence; the level is that minus `freeboard`.
- A numeric level whose body passes the fence raises `water.leaks`; the body is cut at the fence.
- An empty body, or a body with no closed shore loop of more than 8 points, raises `water.dry`
  (error) and the feature gets no shape.

### 6.5 Instances

- **Position.** `instance.no_position`, `asset.unknown`, `set.unknown`, `place.no_host` (`atop` or
  `beside` names no earlier instance), `dress.no_host`, `flank.no_host`: the instance or row is
  dropped. `place.off_surface`: an `atop` offset beyond half the host's footprint; kept.
- **Extent.** An instance outside the extent raises `instance.outside` and is dropped. It stays
  available as a host for `atop` and `beside`.
- **Kit fit.** A kit run whose length differs from `count * module` by more than
  `max(tolerance * length, 0.08)` raises `kit.misfit`. The bays are still placed.
- **Overlap.** Two explicit instances of this world raise `place.overlap` when both assets are solid,
  their footprint polygons overlap (separating-axis test), their floor heights differ by at most
  0.05 m (an instance without a floor counts as 0), they have the same `in`, and they do not share
  a group. A round asset's footprint is a 12-sided ellipse; any other is a rectangle.
- **Relief.** For each explicit instance without a floor, `height` is sampled at the footprint's
  corners, edge midpoints and centre. The range is stored as `relief`. If the asset declares
  `max_relief` and the range exceeds it: `seam.relief`.
- **Avoid.** For each explicit instance without a floor and without a `kind`, for each field in its
  asset's `avoid` that the world declares: a value above 0.5 at its position raises `place.avoid`.

Interiors (only with levels):

- **Rooms.** A feature of a kind with `encloses` whose `level` is not declared raises
  `room.no_level` and is not a room.
- **Openings.** An opening must be `on` a room (`opening.no_wall`). It cuts a box through the wall:
  `width` wide, from `sill` to `head` above the level.
- **Doors.** `to = "outside"`: `height` is sampled `wall + 0.8` m outside the door; if it differs from
  the level by more than 0.3 m: `seam.threshold`. `to` a room: that room must be on the same level
  and within `wall + its wall + 0.3` m of the door point, else `door.dangling`. Any other `to`:
  `door.dangling`.
- **Stairs.** `from` and `to` must be rooms (`stair.dangling`). Their levels must differ by at least
  0.5 m (`stair.flat`). A going under 0.22 m or a rise over 0.21 m per step raises `stair.steep`.
  More than 0.05 m2 of tread outside both rooms (grown by 0.05 m) raises `stair.through_wall`.
  A stair cuts a well in the floor of the upper room.
- **Sealed rooms.** Doors and stairs form a graph over rooms and `outside`. A room not connected to
  `outside` raises `room.sealed`.
- **Wall modules.** With the room kind's `module_m`, each wall edge's length is divided by the
  module; a remainder further than 0.06 m from a whole number raises `kit.remainder`.
- **Instances in rooms.** `in` must name a room (`place.no_room`). More than 0.02 m2 of footprint
  outside the room polygon: `place.in_wall`. With `mount`: a footprint that meets an opening's box,
  at a height within the opening, raises `place.over_opening`; no further checks. With `atop`: no
  further checks. Otherwise: meeting a stair's treads or well raises `place.on_stair`; a solid
  asset meeting a door's zone (the door's width, 1 m either side of the wall line) on the same level
  raises `place.blocks_door`.

### 6.6 Volumes

For each volume field, for each region feature whose kind `hosts` it:

- The box is the region's bounding rectangle by `z = [z0, z1]`, on a grid of `cell` m.
- The volume starts as the field's `base`. Effects on the field are applied to it. A hosting kind's
  effects apply only to the host's own box. A `path3` feature is applied only to a box whose region
  contains at least one of its points in plan. Any other feature of a kind with an effect on the
  field is applied to every box.
- A `path3` point outside the host region or outside `z0..z1` raises `volume.outside`.
- The result is blended back to `ground` over `apron` m inside the region's outline.
- Air in the bottom layer raises `volume.floor`. Solid in the top layer raises `volume.ceiling`.
- The surface at value 0 is meshed (marching cubes). Each vertex gets a normal toward the air and a
  light value 0..1, an estimate of how much open sky it sees.
- Inside the region, `height` becomes the top of the highest solid in each column, `bed` becomes the
  top of the lowest solid run (never above `height`), and `hole` becomes 1 more than one cell inside
  the outline.

### 6.7 Parts

Loading: a portal part needs its door (`part.no_door`). A seam part needs a `ground` field
(`field.required`). A part with an unresolved field is left out; its own diagnostic says why.

Meeting the height (step 9):

- A part's footprint is mapped to master posts by rounding. If it does not fit inside the master's
  grid: `part.outside`, and the part is left out.
- **Portal.** The part is lifted by `dz` = master `height` at `portal.at` minus the door's floor
  height. The footprint is the bounding box of the part's rooms plus 1 m. If the master's `height`
  range under it exceeds `max_relief`: `part.ground`. Nothing is blended.
- **Seam.** A weight `w` rises from 0 at the footprint's edge to 1 at `apron` m inside. The edge ring
  is where `0 < w < 0.15`. `dz` is the mean master height minus the mean part height on the ring, or,
  with `joins_at`, the difference at that point. If the part plus `dz` differs from the master by
  more than `max_step` anywhere on the ring: `part.seam`. If the master declares a `water` field,
  `expects` is `"land"`, and water stands above the ground on more than 2% of the ring:
  `part.context`. Then `height = mix(height, part height + dz, w)`.

Fields (step 14), seam parts only: every master scalar field except `height`, `bed` and `hole` that
the part also has is blended to the part's by `w`. A master field the part lacks is multiplied by
`1 - w` if it carries the tag `vegetation`, and is otherwise left alone. Ground materials the part
uses and the master lacks are appended to the master's list; weights are blended by `w`.

A seam part that has volumes also brings what lies under its surface. Where the part's `hole` is
above 0 inside the footprint, the master's `hole` becomes the larger of the two values, and the
master's `bed` becomes the lower of its own value and `height - (part height - part bed) * w`. The
master's `hole` and `bed` are created if absent. If any such post has `w` below 0.999, the part
raises `part.volume_apron`: there the ground is a blend and does not meet the part's rigid mesh.

Bringing (step 19):

- Instances: translated; ids and `in` prefixed with `<part id>.`; floor heights raised by `dz`.
- Features with a shape, except instance footprints: copied without `on`, `flood` and `path3`;
  id and `in` prefixed; geometry translated; `floor_z` and `cut_z` raised by `dz`; a `look` point
  translated. A `look` table is resolved on the part's own path and brought as a point; if it names
  no path of the part it raises `frame.unknown` and the feature is not brought. Brought features get
  the key `part`. Their effects are not applied again.
- `[[expect]]` rows: `from` and `to` prefixed. Profiles: added where the master has none of the name.
- Generated meshes and volume meshes: ids prefixed with `<part id>_`, vertices moved by the offset
  and `dz`. Lights moved. Rooms listed under `<part id>.<room id>`.
- Portal only: `hole` is set to 1 inside the part's room polygons; `bed` is created if absent.

### 6.8 What leaves the compiler

- **Ground.** More than eight ground materials: `ground.too_many`. A material the pack lacks:
  `ground.unknown`.
- **Bookmarks.** The eye stands on `height`, or on `floor_z` when indoors, or on `bed` with `under`,
  and never below `water`. Three checks follow. `bookmark.blocked`: `height` rises more than 0.3 m
  above the straight line from eye to target. `bookmark.crowded`: an instance at least 0.8 of the eye
  height tall stands within 22 m ahead, inside a widening corridor about the view line. Both are
  skipped for a bookmark that is indoors, `under`, or has `cut_z`. `bookmark.inside`: the eye is
  within half the larger footprint side of an explicit instance; skipped indoors only.
  A `look` table that names no path raises `frame.unknown` and the bookmark is not written.
- **Budgets.** Per chunk: instance count and the sum of the assets' triangle counts, against
  `[budget]`. `budget.exceeded`.
- **Water bodies.** For each feature of a kind with `measure = "water"`, if the world has a `water`
  field: depth is `water - height` within 8 m of the feature. No positive depth: `water.dry`
  (warning). Otherwise a row in `report.json`.

### 6.9 The reach lint

The lint is opt-in (`--lint reach`). It reads: `[[expect]]` and `[profile.*]` (including those
brought by parts), the final `height`, the `water` field if there is one, and the footprints of solid
explicit instances.

How it decides:

1. With no expectations it raises the info `reach.nothing_expected` and stops.
2. It works on every second post (cells of `2 * resolution`). Slope is the gradient of `height`.
   Depth is `water - height`.
3. For each expectation, its profile must declare `max_slope_deg` (`reach.no_profile`). A cell is
   walkable when slope is at most `max_slope_deg`, depth is at most `max_wade_m`, and no solid
   explicit footprint covers it. Walkable cells are grouped into 8-connected areas.
4. A place is a feature with `at`, or else an instance; for an instance the place is 2.5 m in front
   of its footprint. A place snaps to the nearest walkable cell within 8 m. A start that does not
   snap raises `reach.no_start`. A target that names nothing raises `reach.unknown`.
5. A target is reached when its cell is in the start's area. `reach = true` and not reached:
   `reach.cut_off`, with the narrowest gap to the start's area and what closes it (deep water, a
   building, or the slope there). `reach = false` and reached: `reach.not_sealed`.
6. It writes `reach.png` if at least one expectation had a valid start.

What it cannot see: anything under the top surface (caves, ground under an overhang, the `bed`
field), interiors (rooms, doors, stairs, floors), generated meshes, bridges, scattered instances and
non-solid instances.

## 7. Diagnostics

A diagnostic has `severity` (`error`, `warning` or `info`), `code`, `subject`, `message`, and
optionally `where` (`[x, y]` rounded to 0.1 m) and `hint`. Only a registered code can be raised.
Subjects are a feature or instance id, `world`, `chunk <cx>,<cy>`, `expect <n>`, `field.<name>`,
`field.<name>.let.<key>`, `kind.<kind>`, `kind.<kind>.let.<key>`, `scatter.<id>` or `ground.<name>`.
A diagnostic from a part has `<part id>.` before its subject and no `where`.

| Code | Severity | Fires when |
|---|---|---|
| `layer.direction` | error | An item in one layer refers to something defined in a higher layer. |
| `pack.missing_file` | error | The pack names a texture, model, recipe script or recipe atlas that does not exist. |
| `pack.undeclared` | error | A recipe asset does not declare `size` and `tris`. |
| `asset.unknown` | error | An instance, kit run or scatter names an asset the pack lacks. |
| `feature.no_id` | error | A feature has no id. |
| `feature.duplicate` | error | Two features share one id. |
| `feature.shape` | error | A feature has no `path`, `region`, `ellipse` or `at`. |
| `feature.outside` | warning | A feature lies wholly outside the extent. |
| `kind.passthrough` | info | A feature's kind is not declared. |
| `frame.unknown` | error | `on` names something without a path or outline; `look` names something that is not a path (in a world or in a part's brought feature); `joins` involves something that is not a path. |
| `expr.invalid` | error | An expression could not be evaluated. |
| `field.unresolved` | error | A field could not be resolved, in a world or in a part. |
| `field.unknown` | error | An expression or a flood reads an undeclared field. |
| `field.cycle` | error | Fields read each other in a circle. Once per cycle, with the chain. |
| `feature.unknown` | error | An expression measures from a feature that does not exist. |
| `field.required` | error | The master lacks a resolved `height` or `ground`; or a seam part lacks `ground`. |
| `ground.too_many` | error | More than eight ground materials after merging parts. |
| `ground.unknown` | error | A ground material in use is not in the pack. |
| `effect.unbounded` | warning | An effect's mask is above 0.02 at the border of its reach. |
| `seam.change_exceeds` | error | An effect moves a field by more than its `max_change`. |
| `seam.grade` | warning | A path's averaged profile is steeper than `max_grade`. |
| `authority.unknown` | error | An authority is not derived, shaped or anchored. |
| `authority.conforms` | info | A flood's effect on its own source field is skipped. |
| `authority.conflict` | error | Two anchors ask for different values where both masks exceed 0.5. |
| `authority.squeezed` | warning | Two anchors ask for different values where only their aprons overlap. |
| `authority.cycle` | error | An instance on a flood has a kind that reshapes the flood's source field. |
| `water.no_basin` | error | A spill flood holds no water inside its fence. |
| `water.leaks` | warning | A flood at a fixed level passes its fence and is cut there. |
| `water.dry` | error or warning | Error: a flood has no water or no closed shore. Warning: a measured water body holds no water. |
| `instance.no_position` | error | An instance has neither `at` nor `on` (nor `atop`, `beside`). |
| `instance.outside` | error | An instance stands outside the extent. |
| `set.unknown` | error | An instance names an undeclared set. |
| `kit.misfit` | error | A kit run's length is not a whole number of bays within tolerance. |
| `kit.remainder` | warning | A room's walls do not divide into its kind's `module_m`. |
| `dress.no_host` | error | A dress row's `atop` is not a placed explicit instance. |
| `flank.no_host` | error | A flank host is not a placed explicit instance. |
| `place.no_host` | error | `atop` or `beside` names no instance placed before. |
| `place.off_surface` | warning | An `atop` offset is past the host's footprint. |
| `place.overlap` | error | Two solid footprints overlap. |
| `place.avoid` | warning | An instance stands where a field its asset avoids exceeds 0.5. |
| `seam.relief` | warning | The ground under a footprint varies by more than the asset's `max_relief`. |
| `seam.junction` | warning | A path declares `joins` but neither end is on the target. |
| `scatter.count` | info | Always, once per scatter row: how many instances it produced. |
| `room.no_level` | error | A room's `level` is not declared. |
| `room.sealed` | error | A room is not connected to the outside. |
| `opening.no_wall` | error | An opening is not `on` a room. |
| `door.dangling` | error | A door's `to` is not a room, or is a room it does not touch or on another level. |
| `seam.threshold` | warning | An outside door's level differs from the ground outside by more than 0.3 m. |
| `stair.dangling` | error | A stair's `from` or `to` is not a room. |
| `stair.flat` | error | A stair's rooms are less than 0.5 m apart in height. |
| `stair.steep` | warning | A step's going is under 0.22 m or its rise over 0.21 m. |
| `stair.through_wall` | error | Part of a stair lies outside both its rooms. |
| `place.no_room` | error | An instance's `in` is not a room. |
| `place.in_wall` | error | Part of a footprint is outside its room. |
| `place.over_opening` | error | A mounted instance hangs across a door or window. |
| `place.on_stair` | error | An instance stands on a stair or over its well. |
| `place.blocks_door` | error | A solid instance stands in a door's zone. |
| `volume.outside` | warning | A `path3` leaves the volume it carves. |
| `volume.floor` | error | Air reaches the bottom of a volume's z range. |
| `volume.ceiling` | error | Solid reaches the top of a volume's z range. |
| `part.no_door` | error | A portal part lacks the named door. |
| `part.outside` | error | A part's footprint does not fit inside the master's extent. |
| `part.ground` | warning | The ground under a portal part varies by more than `max_relief`. |
| `part.seam` | error | A seam part's edge is further than `max_step` from the master's ground. |
| `part.volume_apron` | warning | A volume of a seam part reaches into the part's apron. |
| `part.context` | error | A seam part that expects land has more than 2% of its edge under water. |
| `bookmark.blocked` | warning | Terrain hides a bookmark's target. |
| `bookmark.crowded` | warning | Instances stand in the first 22 m of a bookmark's view. |
| `bookmark.inside` | warning | A bookmark's eye is inside an explicit instance's footprint. |
| `budget.exceeded` | warning | A chunk exceeds a declared budget. |
| `reach.nothing_expected` | info | The reach lint ran with no `[[expect]]` rows. |
| `reach.no_profile` | error | An expectation's profile is missing or lacks `max_slope_deg`. |
| `reach.no_start` | error | An expectation's `from` is not a place the profile can stand on. |
| `reach.unknown` | error | A target names neither a feature with `at` nor an instance. |
| `reach.cut_off` | error | A target expected reachable has no route. |
| `reach.not_sealed` | error | A target expected out of reach has a route. |

## 8. The asset pack format

A pack is one TOML file. File paths in it are relative to `root`.

| Key | Type | Default | Meaning |
|---|---|---|---|
| `root` | path | the pack's folder | Base folder for every file the pack names. |

### 8.1 `[ground.<name>]`

| Key | Type | Default | Meaning |
|---|---|---|---|
| `color` | `[r, g, b]` | `[0.5, 0.5, 0.5]` | Flat colour, 0..1. |
| `texture` | path | none | An image. A missing file raises `pack.missing_file`; the material is kept without it. |
| `scale_m` | float | `4.0` | Metres one repeat of the texture covers. |
| `tint` | `[r, g, b]` | `[1, 1, 1]` | Multiplier on the texture. |

### 8.2 `[surface.<name>]`

Surface tables are not interpreted. The whole table is written to `world.json` as `surfaces`.
Generated meshes name a surface as their material. The viewer reads `color`.

### 8.3 `[asset.<id>]`

Keys common to every asset:

| Key | Type | Default | Meaning |
|---|---|---|---|
| `tags` | list of strings | `[]` | The compiler reads only `"landmark"` (an icon in `map.json`). |
| `avoid` | list of field names | `[]` | Scatter drops candidates, and the `place.avoid` check fires, where such a field is high. |
| `sink` | float | `0.0` | The instance is lowered this far, m, times its scale. |
| `max_relief` | float | none | Tolerated height range under the footprint, m. |
| `clearance` | float | `0.0` | Stored and written. Not read by the compiler. |
| `color` | `[r, g, b]` | `[0.7, 0.7, 0.7]` | |
| `solid` | bool | `true` | Solid assets take part in overlap, door blocking, scatter exclusion and the reach lint. |
| `footprint` | `[across, depth]` | first two of `size` | Ground rectangle, m, before scale. |
| `module_m` | float | none | The spacing a kit run repeats the asset at. |

The source type is chosen by the first of `source`, `recipe` that is present; otherwise the asset is
a primitive.

Primitive:

| Key | Type | Default | Meaning |
|---|---|---|---|
| `primitive` | string | `"box"` | The viewer draws `box`, `cylinder`, `cone`, `tree`, `tower`; any other name is drawn as a blob. |
| `size` | `[across, depth, height]` | required | m. |
| `round` | bool | true for `cylinder`, `blob`, `tower`, `cone`, `tree` | A round asset has an elliptical footprint polygon. |

Model file:

| Key | Type | Default | Meaning |
|---|---|---|---|
| `source` | path | required | A binary glTF file. A missing file raises `pack.missing_file` and the asset is dropped. |
| `fit_height` | float | none | Scale the model so its height is this, m. |
| `fit_width` | float | none | Else: scale so its width (x) is this, m. |
| `scale` | float | `1.0` | Else: this scale. |
| `albedo` | path | none | A texture for the viewer. Not checked for existence. |

Recipe (an engine-side generator the compiler cannot open):

| Key | Type | Default | Meaning |
|---|---|---|---|
| `recipe` | table | required | Passed to the engine. `script` and `atlas`, if present, are made absolute and must exist (`pack.missing_file`; the asset is kept). The viewer also reads `call`, `variants`, `args`, `tint`, `bark`. |
| `size` | `[across, depth, height]` | required | Missing raises `pack.undeclared`; the asset is dropped. |
| `tris` | int | required | As `size`. |

### 8.4 Measured and declared

| Fact | Primitive | Model file | Recipe |
|---|---|---|---|
| `size` | declared | measured: bounds of the model's position accessors through its node transforms, times the scale, reordered to across (x), depth (z), height (y) | declared |
| `tris` | fixed table: box 12, cone 64, sphere 320, tree 160, blob 320, any other 12 | measured: index count / 3, summed over primitives | declared |
| `scale` | 1 | from `fit_height`, `fit_width` or `scale` | 1 |
| `lift` (raise so the base sits at 0) | 0 | measured: minus the lowest y, times the scale | 0 |
| everything else | declared | declared | declared |

Only the JSON chunk of a model file is read.

## 9. Outputs

The build folder is `build/<name>/` under `spike/`. Files are overwritten; stale files from an
earlier build of the same name are not removed.

### 9.1 Files

| File | When | Format |
|---|---|---|
| `world.json` | always | The manifest (9.2). Compact JSON. |
| `height.f32` | always | Raster (below). |
| `<field>.f32` | each name in `export.fields` that is a resolved scalar field | Raster. |
| `bed.f32`, `hole.f32` | when the world has a volume field or a portal part, or a seam part brings a volume | Raster. `bed`: height of the lowest floor, m. `hole`: 1 where the terrain surface is replaced by a volume or an interior, else 0. |
| `ground_0.rgba8`, `ground_1.rgba8` | always | Ground weights (below). |
| `vol_<id>.verts.f32` | per volume | float32 `x, y, z` per vertex, world frame, m. |
| `vol_<id>.normals.f32` | per volume | float32 `x, y, z` per vertex, unit, toward the air. |
| `vol_<id>.faces.u32` | per volume | uint32, three vertex indices per triangle. |
| `vol_<id>.light.f32` | per volume | float32 per vertex, 0..1. |
| `mesh_<id>.verts.f32`, `.normals.f32`, `.faces.u32` | per generated mesh with geometry | As for volumes. Flat shaded. |
| `plan_<level>.png` | per level with rooms, this world only | Floor plan, 28 px per m, north up. |
| `layer_<id>.json` | per layer | 9.3. |
| `budgets.json` | always | 9.4. |
| `report.json` | always | 9.4. |
| `diagnostics.json` | always | A JSON list of every diagnostic, infos included, in the order raised. |
| `minimap.png` | always | 1024 x 1024 px, north up: ground colours with relief shading, water, explicit footprints, features of kinds with `map`, bookmarks. |
| `reach.png` | with `--lint reach` and a valid expectation | One px per lint cell, north up. Start area, walkable ground, deep water; a dot per target; a cross at each gap. |
| `map/` | with `--map` | 9.5. |

All binary files are little-endian with no header.

**Rasters.** `ny` rows of `nx` float32 values. Row 0 is the south edge (`y0`); within a row, index 0
is the west edge (`x0`). Value `[j][i]` belongs to post `(x0 + i * resolution, y0 + j * resolution)`.

**Ground weights.** Two files of `ny` x `nx` x 4 bytes, same row and column order. `ground_0` holds
materials 0 to 3 of the manifest's `ground` list, `ground_1` holds 4 to 7. Each byte is the material's
weight divided by the sum of the eight weights at that post, times 255, rounded.

**Generated meshes.** `stairs` (visible steps, no collision), `stairs_ramp` (hidden, collision),
and per level with rooms `walls_<level>`, `floors_<level>` (a 0.25 m slab under the level, wells cut
out) and `roof_<level>` (a 0.3 m slab over rooms with nothing built above). Meshes from a part are
`<part id>_<mesh id>`.

### 9.2 `world.json`

| Key | Content |
|---|---|
| `name` | `[world].name`. |
| `frame` | `extent`, `resolution`, `nx`, `ny`, `chunk`, and `axes`, the string `"x east, y north, z up, metres"`. |
| `fields` | For each raster written: `{ file, min, max }` under the field's name. |
| `ground` | Ordered list, one entry per ground material in use: `name`, `color` (the pack's colour, or the tinted mean of its texture), and the pack's `scale_m`, `tint`, and `texture` (absolute path) if any. |
| `assets` | For each asset with at least one instance: the pack's resolved entry. Common keys `tags`, `avoid`, `sink`, `max_relief`, `clearance`, `color`, `solid`, `footprint`, `module_m`, `size`, `tris`, `scale`, `lift`; plus `primitive` and `round`, or `source` (absolute path) and `albedo`, or `recipe`. |
| `instances` | `{ asset id: { "<cx>,<cy>": [row, ...] } }`. Every instance, explicit or scattered. Chunk indices are `floor((x - x0) / chunk)`, `floor((y - y0) / chunk)`. A row is `[x, y, z, yaw, scale, r, g, b]`. |
| `explicit` | List of explicit instances: `id`, `asset`, `pos` `[x, y, z]`, `yaw`, `relief`, and `in` and `attrs` when present. |
| `features` | Every feature with a shape, except instance footprints and kit-run pads. Each entry has all the feature's keys except `path`, `path3`, `region`, `ellipse`, `smooth`; plus `shape` (`path`, `region` or `point`) and `points` (a path sampled every 2 m; a region's polygon). Compiler-added keys may appear: `at`, `level`, `floor_z`, `_layer`, `part`. |
| `volumes` | One row per volume: `id`, `field`, `cells` `[nx, ny, nz]`, `cell_m`, `tris`, `overhang_share` (share of the footprint where solid ground has more than 1.5 m of open space under it), `top_moved_m` (largest change to `height`). |
| `meshes` | One row per generated mesh: `id`, `material`, `z0` (base height), `tris`, `collide`, `hidden`. |
| `lights` | List of `{ pos [x, y, z], color, range, energy }`. |
| `rooms` | `{ room id: { level, area_m2, joins, things } }`. `joins` lists connected rooms and `"outside"`. `things` counts instances in the room. Empty when the world has no levels. |
| `surfaces` | The pack's `[surface]` table. |
| `bookmarks` | List of `{ id, eye [x, y, z], target [x, y, z], fov, note, cut_z }`. `cut_z` is null when unset. |
| `look` | `[look]`, unchanged. |
| `params` | `[params]`, unchanged. |

### 9.3 `layer_<id>.json`

`{ "layer": id, "instances": [...], "features": [...] }`. `instances` lists the explicit instances
that came from the layer: `id`, `asset`, `pos` (rounded to 1 mm), `yaw`, and `attrs` when present.
`features` lists the manifest's feature entries that came from the layer.

### 9.4 `budgets.json` and `report.json`

`budgets.json`: `chunk_m`; `chunks`, a list of `{ chunk [cx, cy], instances, source_tris,
unique_assets }` for chunks that hold at least one instance; `total` `{ instances, source_tris }`.
Triangle counts are the assets' source counts, not what an engine draws.

`report.json`: `{ "water": [...] }`, one row per measured water body: `id`, `authority`, `level_m`,
`area_m2`, `shore_m`, `shoreline_development` (shore length / (2 * sqrt(pi * area))), `islands`
(closed shore loops minus one), `max_depth_m`, `mean_depth_m`, `shallow_share` (share of the wet
area under 1 m deep).

### 9.5 The map folder (`--map`)

| File | Format |
|---|---|
| `class_index.png` | 8-bit grey, `nx` x `ny` px, one px per post, north up. The value is the index of the cell's class. |
| `class.png` | The same picture in the palette's colours. |
| `base.png` | `class.png` with relief shading on land. |
| `map.json` | Below. |

Classes, in index order: `deep`, `shallow`, `sand`, `grass`, `forest`, `rock`, `paving`, `road`,
`building`. A cell's class comes from its heaviest ground material through a fixed table
(`grass`, `meadow`, `dry_grass` to grass; `forest` to forest; `sand`, `shore` to sand; `rock`,
`scree`, `snow` to rock; `paving` to paving; `path` to road; any other name to grass). Water deeper
than 0 is `shallow`, deeper than 2.5 m is `deep`. Footprints of solid explicit instances at least 4 m
on their longer side are `building`.

`map.json` keys:

| Key | Content |
|---|---|
| `transform` | `extent`, `width_px`, `height_px`, `px_per_m` (`1 / resolution`), `north` (`"up"`). |
| `classes` | The class list. |
| `palette` | `{ class: [r, g, b] }`, 0..255. |
| `icons` | `{ id, type, pos [x, y], label }`. `type` is `poi` (features of the kind named `poi`, label from `label`) or `landmark` (explicit instances whose asset has the tag `landmark`, label from the id). |
| `labels` | `{ text, pos, class }`: one per `[[part]]` row with `at` and `footprint`, at the footprint's centre, `class` `"region"`. |
| `footprints` | `{ id, polygon }` for each footprint painted as `building`. |
| `tiles` | `{ grid: 8, hash: { "<row>,<col>": fingerprint } }`. The class picture is cut into 8 x 8 tiles, row 0 at the north. A fingerprint is the first 12 hex digits of the SHA-1 of the tile's class indices. |

## 10. Command line and tools

### 10.1 `python -m worldc`

```
python -m worldc WORLD.toml [--pack PACK.toml] [--overlay FILE]... [--variant NAME]...
                            [--out NAME] [--lint NAME]... [--map]
```

| Flag | Meaning |
|---|---|
| `WORLD.toml` | The world file. |
| `--pack` | Use this pack instead of `[world].pack`. Parts without their own `pack` use it too. |
| `--overlay` | Merge a file over the document. Repeatable, applied in order. |
| `--variant` | Merge `[variant.NAME]` over the document. Repeatable, applied in order. |
| `--out` | The build's name. Default: the world's folder name, the overlays' file stems and the variant names, joined by `_`. |
| `--lint` | Run a lint. `reach` is the only one. Repeatable; each runs once. |
| `--map` | Also write the map folder. |

The build is always written to `build/NAME` beside the `worldc` package. The command prints one
summary line (grid size, ground materials, instances, source triangles), every error and warning with
its hint, and a line `E errors, W warnings, T s`. Infos are not printed.

| Exit status | When |
|---|---|
| 0 | No error diagnostics. |
| 1 | At least one error diagnostic; an unknown variant; a layer id defined twice; any uncaught exception. |
| 2 | Bad arguments, including an unknown lint name. |

### 10.2 `check.py`

`python check.py` compiles six cases with the committed pack into `build/check_<case>`: `vale`,
`vale_winter` (variant), `vale_invalid` (overlay), `station`, `college` (with `--lint reach`), `coast`
(with `--lint reach --map`). For each it compares the exit status, the `E errors, W warnings` text
and a SHA-1 of every non-PNG file with `golden.json`, prints `same` or `DIFFERS`, and exits 1 if any
case differs or has no baseline. `python check.py --record` writes a new `golden.json`.

### 10.3 `maptool.py`

| Subcommand | Does |
|---|---|
| `measure MAPDIR STYLED.png` | Compares a restyled map picture with `class_index.png`: aspect ratio, water overlap, shoreline distance both ways, colour contrast of buildings, roads and paving. Prints numbers. |
| `compose MAPDIR STYLED.png OUT.png` | Draws `map.json`'s labels and icons on the picture. |
| `changed OLD_MAPDIR NEW_MAPDIR` | Lists tiles whose fingerprint changed, and icons added or moved. |

### 10.4 The viewer

```
godot --path viewer -- --build=DIR [--out=DIR] [--shots [--only=a,b] | --bench | --walktest | --mapshot]
```

| Mode | Does |
|---|---|
| (none) | Walk the world. Keys: W A S D, Shift to run, Space to jump, F to fly, M for the full map, digits 1 to 9 to jump to bookmarks, Esc to free the mouse. |
| `--shots` | Renders each bookmark to `<out>/<bookmark id>.png` at 1920 x 1080. `--only` limits the ids. |
| `--bench` | Prints the frame rate at each bookmark; saves `<out>/walk_<id>.png` if `--out` is given. |
| `--walktest` | A scripted walk that checks collision, floors and a stair. Only for the world named `Station`. Exit 0 on pass. |
| `--mapshot` | Saves `<out>/runtime_minimap.png` and `<out>/runtime_fullmap.png`. |

Giving more than one mode is an error. `--build` defaults to `build/vale`.

Files the viewer reads from a build: `world.json`; `height.f32`; `water.f32`, `grass.f32`, `hole.f32`
and `bed.f32` when `fields` lists them; `ground_0.rgba8` and `ground_1.rgba8`; every `vol_*` and
`mesh_*` file named by the manifest; `map/map.json` with the first of `map/styled.png`,
`map/styled.jpg`, `map/base.png` that exists. It also opens the absolute paths in the manifest:
ground textures, model files, albedo textures, recipe scripts and atlases.
It does not read `budgets.json`, `report.json`, `diagnostics.json`, `minimap.png`, `layer_*.json`,
`plan_*.png`, `reach.png`, `class.png` or `class_index.png`.

Manifest keys the viewer reads: `name`, `frame`, `fields`, `ground`, `assets`, `instances`,
`explicit` (collision boxes of solid assets), `volumes`, `meshes`, `lights`, `rooms` (only whether
any exist), `surfaces`, `bookmarks`, `look`, `params.sea`. It does not read `features`.

`[look]` keys the viewer reads:

| Key | Default | Meaning |
|---|---|---|
| `sky` | `"summer"` | `"winter"` selects colder defaults. |
| `sun_azimuth`, `sun_elevation` | `215`, `38` | Degrees. |
| `sun_color`, `sun_energy` | by `sky` | |
| `sky_top`, `sky_horizon`, `fog_color` | by `sky` | Colours. |
| `ambient` | `1.0` | Ambient light energy. |
| `haze` | `0.5` | Fog amount. |
| `horizon` | none | `"sea"` adds a water plane beyond the world's edge at `params.sea`. |
| `surf` | `0.0` | Surf amount on the water. |
| `water_deep`, `water_shallow` | shader defaults | Water colours. |
| `rock_a`, `rock_b`, `rock_tint`, `rock_band_m` | shader defaults, `1.0`, `3.0` | Banding of volume rock. |

`shoot.sh`, `explore.sh` and `bench.sh` start the viewer on `build/<name>` with output in
`out/<name>`. `sheet.py` joins bookmark renders into a contact sheet.

## 11. Known limitations and inconsistencies

Fields and materials

- The output holds eight ground materials. More raises `ground.too_many`; planes past the eighth are
  not written.
- One material field is supported, and it must be named `ground`. The field must be named `height`
  for instances, floods' default, volumes, checks and the lint to work. Both are required.
- The names `water`, `bed` and `hole` are interpreted: `water` by bookmarks, water measurement, the
  part context check, the minimap, the map and the lint; `bed` and `hole` are created by the compiler.
- `grass` is read only by the viewer, and only if the world exports it. `clear` has meaning only
  through the shared kinds and the pack's `avoid` lists.
- The tag `vegetation` is hard-coded in the part blend. No other tag has built-in meaning.
- `unit` is not a unit: any value only switches on the comparison of anchors.
- A second field of type `material` would overwrite the material list of the first.
- Reading a volume field with `f('name')` in a 2D expression resolves it as a scalar field and fails.
- Expressions are Python `eval`. `min` and `max` take exactly two arguments. A `[params]` name or a
  feature attribute can shadow a built-in function.
- The noise functions in 2D and 3D expressions differ (`fbm`, `ridged` against `fbm3`, `fbm2`), and
  `pi`, `exp` and `pow` are missing from volume expressions.
- A failed 2D expression becomes 0; a failed volume expression becomes `ground`.

Kinds, effects and authority

- The kind names `bookmark` and `poi` are hard-coded. A kit run's pad kind defaults to the name `pad`.
- The number of entries in `over` changes effect order in every tier; which ids it names does not.
- Among anchors only regions are ordered by area. Anchored paths and points are applied after all
  anchored regions with the same `over` count.
- A kind's `let` that fails with a missing name in a 2D effect is skipped silently; the same failure
  in a volume effect raises `expr.invalid`. Any other exception in a 2D kind `let` is not caught.
- The `seam.grade` check runs for every scalar field a path's kind touches, not only `height`.
- `effect.unbounded` is not checked where the effect's window touches the world's edge.
- `seam.change_exceeds` and `kit.misfit` are errors, but the change and the bays are still applied.
- Effects of an instance's kind placed on a derived feature never reach `height`.
- Effects on volume fields ignore authority, `over`, `reach`, `max_change` and tags.
- A non-hosting feature without `path3` whose kind has an effect on a volume field is applied to
  every volume box, wherever the feature is.
- `along` on a region or point raises `expr.invalid`.

Features

- A feature with both `flood` and `on` raises `feature.duplicate` when it floods. A flood needs `at`.
- `on.slide` is read for instances but not for features.
- `on` hosts must be declared so that they have a shape first: a feature `on` another feature that is
  itself placed `on` must come later in the document.
- An opening's `on.offset` moves the feature's point but not the hole cut in the wall.
- A stair needs `at` (or other geometry) besides `arc` or `line`; with neither `arc` nor `line` the
  compile fails with an exception. Stair `width` and `riser` ignore kind defaults.
- A door's target named `outside` is reserved; a room with that id cannot be told apart.
- One overhang per column: `height` holds the top surface and `bed` the lowest floor. Anything
  between is only in the mesh.
- A volume's box is the bounding rectangle of its region, aligned to the axes.
- Lights are collected only when the world declares levels.
- A world's water has no generated geometry: it is the `water` raster.

Instances and producers

- `atop` on a host that has no floor height (an outdoor instance) places the item at the host
  asset's height above z = 0, not above the terrain.
- `yaw` is ignored with `on`; `face` and `turn` are ignored with `at`. `against` overwrites `on.offset`.
- `[[instance]].on` must be a table and `[[run]].on` must be a string.
- Instance ids are not checked for uniqueness. The default id depends on the instance's index.
- An instance outside the extent is dropped but still serves as a host.
- A set passes only position, yaw and `in` to its members: `kind`, `scale`, `tint`, `mount` and free
  attributes on the placing instance are lost.
- Free attributes include `over`, `authority` and kind attributes written on an instance.
- The footprint feature of a round asset is a rectangle. `round` can be set only on primitives.
- Dress items that find no room are dropped without a diagnostic.
- The asset key `clearance` is never read. Asset `tags` are read only for `landmark`.
- Primitive triangle counts are a fixed table; `cylinder` and `tower` count as 12.
- Scatter avoids only this world's own explicit solid instances, with a fixed margin of about 1 m.
- Scattered instances have no id and appear in no layer file.
- Overlap is tested with convex polygons, between this world's explicit instances only: not against
  a part's instances, and not against scattered ones.
- `place.avoid` is skipped for an instance with a `kind`.
- Interior checks cover this world's explicit instances only.

Layers and merging

- Lists merge by `id` only when every incoming item has one. `[[expect]]` rows and effect lists are
  replaced whole by an overlay or variant.
- A layer's id clash is detected between layers only, not against the world's own file.
- `layer.direction` does not read ids inside expressions, a door's or a stair's `from` and `to`, or
  `over`.
- `[[layer]]` rows are read from the world's own file only; an overlay cannot add a layer.
- An unknown variant and a layer id clash stop the run with a message, not a diagnostic.

Parts

- A part is translated only: never rotated, never scaled.
- Parts do not nest: a part's own `[[part]]` rows are ignored without a diagnostic.
- The part and the master must share `resolution`; posts are copied one to one with rounding and no
  resampling.
- A seam part's scalar fields are blended without the lift `dz`: a part's `water` level and a flood's
  `level` attribute stay at the part's own heights.
- A seam part's volume mesh is brought rigid while the ground in the apron is blended, so a volume
  inside the apron does not meet the ground (`part.volume_apron`). An interior inside a seam part
  sets no `hole` in the master; only volumes of a seam part and rooms of a portal part do.
- A portal part's `hole` covers its room polygons without their walls.
- A part's floor plans, levels and layer files are not brought. Its bookmarks are, as master
  bookmarks.
- Brought features keep unprefixed references (`from`, `to`, `joins`, `level`); brought instances
  keep unprefixed `atop` and `group`.
- A part's effects are computed in the part only; the master's fields are not re-resolved.
- The master's pack wins where both packs define a name; a part compiled with its own pack may then
  be drawn with the master's asset of the same id.
- Part diagnostics lose their positions, and their infos are dropped.

Outputs

- `minimap.png` is always square; a world that is not square is stretched.
- `plan_<level>.png` is written for this world's levels only.
- The `points` of a path in `world.json` are every eighth sample; the path's last point may be absent.
- `world.json` holds absolute paths to pack files.
- Instance footprint features and kit-run pads are not in `world.json`.
- When `field.required` fires nothing is written, not even `diagnostics.json`.
- Old files in a build folder are not removed.
- The map's class table is keyed on fixed material names; an unknown material is `grass`.
- The map's tiles ignore the rows and columns left over when the size is not a multiple of 8.
- The map's part labels use the part id and exist only for seam parts with a `footprint`.

The reach lint

- It reads the top surface only: no caves, overhangs, interiors, stairs, generated meshes or bridges.
- Its cell is fixed at two posts; its snap radius (8 m) and instance approach (2.5 m) are fixed.
- `reach.png` shows the walkable area of the first valid expectation's profile only.
- A feature with a region or path shape cannot be a start or a target unless it has `at`.

Tools

- The build folder is fixed to `build/` beside the package.
- `check.py` skips PNG files.
- `shoot.sh` does not pass `--shots` itself; the mode must be given as an extra argument.
- The viewer's walk test is written for one world by name, with that world's coordinates.
- The viewer finds rock by the ground material name `rock` and sea level by the parameter `sea`.
- A relative `root` in a pack is resolved against the working directory, not the pack's folder.

## 12. Additions since the refinement (2026-11-12)

Sections 1 to 11 describe the compiler as it stood after the refinement. This section records what
has been added since, each entry naming the section it amends. They are appended, not merged in.
Design reasoning is in `docs/design/blackbox.md` and `docs/design/ways-and-assets.md`.

### 12.1 Fields read from a file (amends 3.4)

| Key | Type | Default | Meaning |
|---|---|---|---|
| `file` | path | none | Relative to the document that names it. Replaces `base`; `layers` and effects still apply. |
| `channel` | `r`, `g`, `b`, `a`, `luma` | `luma` | Pictures only. A 16-bit or float picture has one channel and ignores this. |
| `range` | `[black, white]` | `[0, 1]` | Pictures only. The values the darkest and brightest levels stand for. |
| `extent` | `[x0, y0, x1, y1]` | the world extent | Pictures only. Where the picture lies. |
| `outside` | float | none | Pictures only. The value beyond `extent`; without it the picture's edge carries on. |
| `filter` | `nearest`, `linear`, `cubic` | `linear` | Pictures only. |

A `.f32` file is raw 32-bit floats, one per post, south row first, and must match the grid exactly.
Any other suffix is read as a picture with its top row north and resampled to the grid. Material
and volume fields cannot be read from a file.

### 12.2 `[[step]]` (new; pipeline step before 5.1 step 1, and after the last)

| Key | Type | Default | Meaning |
|---|---|---|---|
| `id` | string | required | Names the step, its output folder and its diagnostics. |
| `at` | `generate` or `lint` | required | The point of the build it joins. |
| `run` | list of strings | required | The command, run in the world's folder. `python` is the compiler's own interpreter. |
| `after` | layer id | none | `generate` only. The step is handed the build of the layers at and below this one. |
| `params` | table | `{}` | Passed to the step unread. |
| `inputs` | list of paths | `[]` | Further files the step reads, so that a change to one reruns it. |

The compiler writes one request file and sets `VIA_STEP` to its path:
`{contract, point, id, params, frame {extent, resolution, nx, ny, seed}, build, out}`. `build` is a
build folder or null. The step writes into `out`: `layer.json` (generate; a document, JSON or TOML
shaped), `.f32` rasters the layer names, and `diagnostics.json` (a list of
`{severity, code, subject, message, where?, hint?}`).

- A generated layer sits lowest when the step has no `after`, else directly above that layer.
- A generate step is skipped when a hash of its command, parameters, named files, frame and the
  build it is handed matches the last run. Its outputs are kept in `gen/<id>/` beside the world.
- A lint step always runs, after every output file is written; `diagnostics.json` is then rewritten.
- A step's diagnostic codes are written `<step id>:<code>` and are not in the registry.
- Steps of a part are not run. A world's steps run unasked.

`sdk/viasdk` wraps this contract for Python and holds no compiler logic.

### 12.3 Kit runs (amends 3.10)

| Key | Type | Default | Meaning |
|---|---|---|---|
| `bay` | asset id, or list of `{ id, weight }` | required | A list is chosen from per bay, seeded. All pieces must share a module (`kit.module`). |
| `end` | asset id | none | Used for the first and last bay. |
| `seed` | int | the row's index | Seed for the choice of bays, mixed with the world seed. |
| `along` | path feature id | none | Frontage form: replaces `from` and `to`. The run stands beside this path. |
| `side` | `left` or `right` | `left` | With `along`. The side of the path, looking along it. |
| `at` | float | `0.0` | With `along`. Where the run starts, as a fraction of the path's length. |
| `bays` | int | `1` | With `along`. The number of modules. |
| `setback` | float | `0.0` | With `along`. From the path's centre line to the bays' front. |

A kit run may carry `group`; its bays then share that group with other instances and may
overlap them (an end piece placed by hand, for instance). Without it the group is the run's id.

With `along` the run is a straight chord between two points of the path, so on a curve its middle
stands nearer or further than `setback`. Its bays face the path.

### 12.4 Effect order (amends 3.5 and 5.2)

An effect may carry `order` (int, default 0). Within an authority tier, effects are applied by
`order` first, whatever feature or kind they belong to, then as before. `kinds/ways.toml` uses it
so that every way lays its pavement (0) before any lays its roadway (1) or track (2).

A feature may also carry `order`. It then stands for every effect of that feature, in place of the
effects' own. This lets a document state a strict sequence across features of different kinds,
which is how a painter's-order terrain (each row blending over the last) is written.

### 12.5 Decks (new; a mesh, like 3.14's interiors)

A kind may carry `deck = { surface, thickness, parapet, pier_every }`. A path feature of such a
kind has no effect on any field and produces one mesh, `deck_<id>`, that collides.

| Feature key | Type | Default | Meaning |
|---|---|---|---|
| `width` | float | `8.0` | Across the deck, m. |
| `rise` | float | `0.0` | Height gained above the straight line between the two ends, m. |
| `ramp` | float | half the length | Distance over which the rise is gained at each end, m. |
| `over` | list of feature ids | `[]` | What it must clear. A path is crossed where the deck is within its half width plus 2 m; a region where a deck station lies inside it. |
| `clear` | float | `4.5` | Room required between the underside and the ground or water at each crossing, m. |

| `from_on`, `to_on` | deck feature id | none | That end rests on the named deck: it takes the height of that deck's surface at the nearest point. An end further from the deck's centre line than half of both widths plus 1 m raises `deck.detached`. Naming a way that is not a deck leaves the end on the ground. |
| `max_grade` | float | none | Percent. A steeper deck raises `deck.grade`. |
| `settle` | float | none | The height difference between the two ends is taken up over the last this-many metres. Without it the deck runs straight from one end's height to the other's. |

The deck's ends take the terrain height at those points after all fields are resolved, unless
`from_on` or `to_on` says otherwise. A deck that rests on another is built after it. Piers stand
every `pier_every` metres where the underside is more than 2 m up and nothing named in `over` is
beneath. The ways that meet a deck are separate features that end where it begins.

### 12.6 Smaller additions

- `[export] feature_attrs = true` adds `attrs` (kind defaults under the feature's own numbers) to
  every feature in `world.json`. Off by default.
- Primitive assets: `house` (ridge along the front) and `gable` (ridge front to back), with an
  optional `roof` colour. The viewer draws them; the compiler counts 12 triangles for either.
- `kinds/ways.toml`: street, motorway, railway, waterway, freeway, bridge, ramp, rail_bridge, lot, park.
- Worlds: `massif`, `relief`, `market`, `city`.

### 12.6a Taxonomy (amends 8.3 and 3.10)

An asset may carry `taxon` (a slash-separated path, for example `building/row/bay`) and `labels`
(a list of cross-cutting marks, for example `biome/temperate`). The compiler gives neither any
meaning. Both are written to `world.json`.

In a kit run's `bay`, an id that starts with `@` names a place in the taxonomy, not an asset: it
stands for every asset of the pack filed at or under that path. No match raises `asset.unknown`.
Scatter, dress and flank do not accept it yet.

`python -m worldc.zoo PACK` reads a pack alone: `--match`, `--taxon`, `--label`, `--tag`, `--where`
filter; `--tree` prints the taxonomy with counts; `--json` prints rows; `--world NAME` lays the
matches out as a world.

`mount = "ceiling"` on an instance with `in` hangs it so its top meets the room's ceiling (the
level's `z` plus `height`); `drop` lowers it by that many metres. Without a room whose level has
a height it raises `place.no_ceiling`.

The zoo also counts use: `--builds [NAME ...]` reads the instances of those builds (all builds
when none is named) and adds `placed` and `in_builds` to each row; `--unused` keeps only assets
placed in none of them.

### 12.6b Formulas, planless kinds and free solids (amends 3.3, 3.5, 3.6 and volumes)

- **Formula attributes.** On a feature, in `[params]`, or in a kind's `defaults`, a string that
  starts with `=` is a formula over numbers. Names, in order: the feature's own attributes
  (including its kind's defaults), `[params]`, then feature ids, read as `id.attribute`. Functions:
  `tan sin cos radians degrees sqrt min max abs`, and `pi`. Each is worked out once, before any
  shape or field, in the order references need. Afterwards the attribute is a plain number.
  A formula that fails raises `attr.invalid` and becomes 0; a circle raises `attr.cycle` with its
  chain. Overlays and variants are merged first, so a variant may change a parameter that many
  formulas read. Only scalars: `at`, `path` and `region` cannot hold formulas.
- **`plan = false` on a kind.** Its features need no `at`, `path` or `region` and get no shape.
  Inside a volume effect they have no `d` or `sd`. They are not written to `world.json`.
- **`free = true` on a volume's host feature.** The volume is an object, not land: no apron, no
  `volume.floor` check, and `height`, `bed` and `hole` are left untouched.

### 12.6c Mounts (amends 3.6, 3.15 and 8.3)

A mount is a named place on something, which other things may refer to as `owner.name`.

- **On an asset.** `[asset.<id>.mount.<name>] at = [right, forward]`, in metres from the asset's
  pivot, forward being the way its front looks. `expects = "path"` says a path should end there;
  an explicit instance with such a mount and no path ending within 2 m raises `mount.unmet`.
- **On a path.** Every path feature has `start` and `end`.
- **Use.** A path feature may carry `starts_at` and `ends_at`, each a mount. Its `path` then holds
  only the points in between, and may be absent. The mount's point is added at that end. A path
  that names a mount is shaped after every other feature; one that names another waiting path's
  end takes a later turn. An unknown mount raises `mount.unknown` and lists the mounts the owner has.
- **Output.** An explicit instance whose asset has mounts gets `mounts: { name: [x, y] }` in
  `world.json`.
- **Layers.** Naming a mount is a reference to its owner, so it obeys the layer direction rule.

A mount has a position only: no height, no direction, no type. Only explicit instances written as
`[[instance]]` have mounts; members of sets, runs and kit runs do not. Nothing but a path can
refer to one.

### 12.6d Scatter ids, hand edits, locks and the change report (amends 3.9 and 6)

- **Ids.** A scatter member is `<scatter id>:<cell>`; a clump member is `<scatter id>:<cell>.<n>`.
- **`[[override]]`** `{ id, at?, asset?, yaw?, scale?, remove? }`: patch one member by id.
  `override.orphan` when the id no longer exists. Kept as the weaker of two designs (see FINDINGS).
- **`[[pin]]`** `{ id, asset, at, yaw?, scale? }`: an instance at a place, whatever the rules do.
  `pin.conflict` when it stands on a field its scatter avoids.
- **`[[clear]]`** `{ at, radius?, scatter? }`: no scatter member inside the circle (default radius
  0.6 of the cell).
- **`[[lock]]`** `{ id, region, scatter? }`: the first build records the members inside the region
  under `locks/` beside the world file; later builds replay the record there. `lock.taken` on
  recording; `lock.holds` when the rule would now place differently, with the counts.
- **Change report.** Every build writes `placed.json`; when one from the previous build exists it
  also writes `changes.json` (added, removed, moved, swapped per rule, the bounding box, the
  untouched count) and prints one line per rule.
- **`clump`** on a scatter: `{ size = [lo, hi], radius, shrink }`. Each accepted point becomes a
  parent with members round it, each smaller by `shrink`.
- **`around('<scatter id>', reach)`** in a scatter's expressions: 1 beside what an earlier scatter
  placed, falling to 0 at `reach`. The named scatter must come first in the file.

### 12.6e Places in parts, and a part's own blend (amends 3.15 and 6.7)

A master may name a place in a part wherever a mount is accepted (`starts_at`, `ends_at`). Parts are
compiled before the master's shapes, and each brings its places in the master's frame:

| Name | Is |
|---|---|
| `<part>.<path id>.start`, `.end` | An end of a path in the part. |
| `<part>.<point id>.at` | A point feature of the part. |
| `<part>.<instance id>.<mount>` | A mount the pack declares on an instance of the part. |

A part that moves takes these with it, so a master feature that names them follows. If a named place
lies where the part's weight is under 0.98 the ground there is a blend: `part.named_in_apron`.

`[[part]]` gains `blend` (string, optional): an expression giving the weight of the part's ground,
clamped to 0..1. Names: `w` (the default weight, 0 at the footprint's edge and 1 an apron inside),
`d` (metres inside the edge), `apron`, and `smoothstep`, `min`, `max`, `clamp`, `where`, `sqrt`.
Without it the weight is `w`. An expression that fails raises `part.blend` and the default is used.
The seam checks (`part.seam`, `part.context`) still read the default edge ring. `expects` set to
anything other than `"land"` (for example `"sea"`) skips `part.context`.

Limits: the falloff is a function of distance from a rectangular footprint, not of what the ground
is; it applies to every field alike; a part still cannot be rotated; a master feature cannot be
placed `on` a part's feature, only start or end at one.

### 12.6f Parts at any spacing, with their steps, a skirt, a lift and renamed ground (amends 3.15 and 6.7)

Five things changed so that a master can place worlds that were never made to meet (world `atlas`).

**Steps.** A part runs its own `generate` steps, as it does when compiled alone, and their cached
results are reused. Its `lint` steps are not run: a part writes no build for them to read.

**Spacing.** A part's `resolution` need not be the master's. With the same spacing its rasters are
taken post for post, as before. Otherwise every raster of the part (height, each scalar field, each
ground weight, `hole`, `bed`) is sampled bilinearly at the master's posts. Instances, meshes and
features are not affected. Nothing is filtered when the master is coarser, so detail narrower than
the master's spacing is lost without a message.

**New keys on `[[part]]`** (seam parts):

| Key | Type | Default | Meaning |
|---|---|---|---|
| `variants` | list of strings | none | Variants the part is compiled with. |
| `lift` | float, m | none | The part's heights plus this, said outright. Overrides `joins_at` and the edge ring. |
| `outer` | float, m | `0` | Width of a skirt outside the footprint. See below. |
| `ground_as` | table | none | `{ theirs = "ours" }`: a ground material of the part is carried as another one. |

**Skirt.** With `outer > 0` the part keeps every post of its own height: inside the footprint the
height is the part's plus its lift, with no blend. Outside, over `outer` metres, the master's height
is mixed toward the part's nearest edge post, by `1 - smoothstep(0, outer, d)`. Beyond 24 m out the
target is a copy of the part's ground smoothed with a 14 m Gaussian, so the edge's wrinkles are not
drawn out as ribs. Where two skirts overlap, each is weighted by its mix weight divided by its
distance, so both edges are met exactly. A skirt never changes a post inside another seam part's
footprint. `part.seam` is not raised for a part with `outer`: the land has come to meet it. The
other fields (ground, vegetation, water) are still blended inside the footprint by `apron` and
`blend`. The skirt is applied after the master's own features, so a master road inside a skirt
follows the skirt, not its own grading.

**Lifted fields.** A field of the part declared with `unit = "m"` (water) is raised by the part's
lift when it is blended in, unless the part `expects = "sea"`: such a part shares the master's sea
level, and only its ground is lifted.

**Ground.** The output still holds eight ground materials after merging (`ground.too_many`).
`ground_as` is how a master with many parts stays under it.

Limits: the skirt knows distance and the edge's height, nothing else: it does not continue a river,
a road or a ridge, and the ground material outside is the master's. `lift` is a number the author
finds by looking; nothing picks it. Parts still do not nest, rotate or scale.

### 12.6g A join by ground: edges by name, openings, a layer laid after the parts (amends 3.15, 6.7 and 9)

For two maps that meet by ground, with the meeting owned by one master layer (world `border`).

**After the parts.** A master feature with `after = "parts"`, or any feature of a `[[layer]]` that
says `after = "parts"`, is not applied when its field is first worked out. It is applied once the
parts have brought theirs: on `height` after the seam blend and before volumes and floods, on every
other field after the parts' fields are blended in. So a ridge along a join or a path through it is
not blended away by the parts it crosses. Order among such features is authority, then `order`, then
kind and declaration. Where one of them moves `height` by more than 0.3 m on ground a part anchored
(a feature of the part whose authority is `anchored`), the master wins and says so: `part.overruled`.
A part's instances are not moved or removed by it.

**A part's edge as a path.** A seam part offers `<part>.edge` (the whole outline of its footprint,
closed, run with the part on its left) and `<part>.edge.<n>` (side `n`, from 0). A box also offers
its sides as `<part>.edge.south`, `.east`, `.north`, `.west`. A feature's `path` may be one such name
or a list of them, taken end to end. They move with the part.

| Key on a path feature | Type | Default | Meaning |
|---|---|---|---|
| `outset` | float, m | `0` | The named line is moved this far outward from its part (negative: inward). |
| `open_at` | list | none | Leaves an opening at each entry. An entry is a place a part offers (`a.road.end`), or the id of a path of this world: the opening is then where that path crosses. |
| `gap` | float, m | `12` | Width of each opening, measured along the line. |

A feature with openings becomes several: `<id>.0`, `<id>.1`, in order along the line, each with the
attribute `piece_of = "<id>"`. Each opening is a place `<id>.gap.<n>`, numbered as in `open_at`.
`open_at` also works on a path written as coordinates.

**A footprint that is not a box.** `footprint` on a seam part may be a list of points, in the
part's own coordinates. The blend weight is then measured from that outline, and an instance of the
part that stands outside it is left out. The checks on the edge ring cover the whole outline,
including any side that lies on the master's edge. `outer` still reads the box.

**Reading the join in an expression.** `inside('<part>')` is the part's blend weight (0 outside its
footprint, 1 an apron inside); `outside('<part>')` is the distance in metres outside its footprint.
Both can be read by any master field except `height`, and by a master scatter. Read too early, or
for an unknown part: `part.unknown`.

**Proving a funnel.** `[[expect]]` takes `closed = ["<id>.gap.<n>", ...]`: the check is made with
those openings shut (a disc of the opening's width is not walkable). With `reach = false` it states
that every route passes one of them. The reach lint reads the joined ground, and the explicit
instances parts bring stand in the way like the master's own.

Limits: a side of an outline has a number, not a name; the ridge is a kind the project writes; a
part's own growth still stops at its footprint, so growth across a join is a master scatter written
to match; `max_step` is one number for all sides; the reach check is the top surface only.

### 12.6h Exits: the edge of a world built apart (amends 3.5, 3.15 and 6.7)

**`exit` on a kind.** `exit = true` makes every point feature of that kind an *exit*: a place where
something of this world leaves it. The compiler knows the flag and nothing of what leaves; the
library kind `exit` in `kinds/ways.toml` carries it. An exit is placed like any point, usually `on`
the end of the way it belongs to (`on = { feature = "central_ave", t = 1.0 }`), and may stand on
the boundary or just past it without `feature.outside`. In a world compiled alone an exit does
nothing: it is a promise, passed through to the build like any feature.

**What a master gets.** For a part `p` with an exit `x`:

- the place `p.x`, usable wherever a place is named (`starts_at`, `ends_at`, `open_at`);
- the way it faces: along the path it stands `on`, pointing out of the part, or square to the
  nearest side of the part's box when it is a plain point;
- its attributes. One is read: `width` is the stretch of the part's edge the exit takes up, and
  water there does not count against `expects = "land"` (`part.context`). Every other attribute,
  `class` among them, is passed on unread and quoted in the diagnostic below.

**The obligation.** Every exit of every part must be *answered*: some feature of the master has
`starts_at` or `ends_at` naming it. One that is not raises the error `exit.unanswered`, at the
exit, naming the part, the exit and its attributes, with what to write.

**`onward` on a path.** `onward = <m>` with `starts_at = "<part>.<exit>"` and no `ends_at` gives the
path its far end: straight on from the exit, the way the exit faces, for that many metres. It is how
an exit that leads nowhere is answered without a coordinate. Without an exit to start from it is
`frame.unknown`.

**Library.** `kinds/ways.toml` adds `exit` (the flag, and a mark on the minimap) and `fade`: a way
that narrows from `width` to `track` and dies out along its length, written with `t` in its effects.
It is ordinary data; a project may write its own.

### 12.6i What a master gives a part: `given` and `give` (amends 3.15, 5 and 6.7)

A world built apart is, until now, the same thing wherever it lands. Some worlds should not be: a
city is less a map with an edge than a rule that fills the land a region allows it. So a world may
declare what it can be *given*, with a default, and a master may give it.

**`[given.<name>]` in a world.** `type = "region"` or `type = "field"`.

- A given region becomes an ordinary feature of that name (kind `guide` unless `kind` says
  otherwise), so `dist('limits')` and `sdist('limits')` read it. Default: the world's whole extent.
  The feature carries `given = true` or `false` in the build, so a step can tell which it has.
- A given field becomes an ordinary scalar field of that name, read as `f('urban')`. Default:
  `default` (0). It is added to `[export] fields`, so the world's steps are handed it.

Built alone, the defaults stand and the world is what it was. Nothing else in the world's text
changes between the two cases.

**`give = { <name> = "<the master's>" }` on a `[[part]]`.** Each name is one the part declares; each
value is a region feature or a scalar field of the master. Both are worked out on the master *as it
stands before any part is placed*: its own terrain and its own features. They are then handed over
in the part's frame (a region is moved by `at`; a field is sampled at the part's posts), and the part
is compiled with them. A given field that reads something only a part brings (`inside()`, a path
that starts at a part's exit) is the error `give.early`. A name the part does not declare, or a
value that is not a region or field of the master, is `give.unknown`.

**`footprint = "<region>"` on a seam part.** The part is cut to a region of the master, by name. Its
box stays its own extent. One thing reaches past the outline: the path an exit stands `on` is
carried, with ground, as far as that exit, so a way promised to the master does not stop at the
limits. At the box itself the blend is a third of `apron`.

**Steps and the record.** What was given is part of the frame every step of the part is handed
(`frame.given`, a digest) and so of its input record. A part's generate steps keep what they write
for each giving apart, in `gen/<step>@<digest>/`, beside what the world writes alone.

**Order.** `load_parts` now builds the master once without parts, only to read what is given, then
each part, then the master proper. Cost on `atlas`: about 30 s.

Shown in `city` (`[given.limits]`, `[given.urban]`; `tools/network.py` and `tools/lots.py` obey
them) and `atlas` (`city_limits`, `[field.urban]`).

### 12.6j Regions by intent, and a set handed down (amends 3.3, 4.1 and 12.6i)

Shown in `atlas` and `city`: a city planned as two squares and four axes (anchored), five districts
(shaped) and nothing else; its limits, streets, lots and buildings are derived.

**`where = "<expression>"` on a feature.** The feature is a region: the largest connected piece of
ground where the expression is true, holes filled, its outline simplified to two posts. The
expression is the field language over the whole grid (`dist`, `sdist`, `f`, `x`, `y`, params, noise;
`&`, `|` and `~` join conditions). It is derived when first read, like a flood, so it may read fields
and other features, another `where` region included, and it moves when they do. A `where` that is
true nowhere is the error `feature.where_empty`. A region that would be several pieces keeps the
largest only, without a message.

**`dist('kind:<name>')`, `sdist('kind:<name>')`.** Every feature of that kind as one shape. With
`where`, a union is `sdist('kind:district') < 0`.

**A set handed down.** `[given.<name>]` with `type = "features"` and `kind = "<kind>"` in the part;
`give = { <name> = "kind:<kind>" }` on the master's `[[part]]`. Every region feature of that kind in
the master is handed over in the part's frame with all its attributes and `given = true`. They take
the place of the part's own features of that kind; built alone, the part's own stand. A `where`
region is handed over as the outline it came to.

**The way to an exit** is carried past a named footprint only from the footprint out to the exit;
its other end is cut like any other way. (Before, the whole way was carried.)

**Library, not compiler** (`kinds/town.toml`): `district`, `square`, `axis`, `ward`. The compiler
knows none of them. What a district says (`pattern`, `block`, `bearing`, `around`, `buildings`,
`floors`, `density`) is read by `city/tools/network.py` and `lots.py`.

Not done: a given set may hold regions only; a `where` region cannot be named by a part; nothing
checks that given districts lie inside the part's own extent; the order in which districts win is
the order they are written.

### 12.6k A simulation as a step, under stated facts (amends 3.6 and 5)

Shown in `basin`. The author states a starting surface loosely, a lake at a level and a river
between two named points. A generate step gives those to the Rust terrain stage of this repository
(`crates/via-terrain`, run as the command `via terrain`) and takes back the ground. The compiler
learns nothing about erosion; one small thing was added to it.

**A point feature is a place by its own id.** `starts_at = "spring"` and `ends_at = "haven"` name
point features directly. Before, only `id.start`, `id.end` and an instance's mounts were places. A
path may be nothing but two named ends (`path = []`).

**What the step reads is said by where it stands.** `after = "facts"` hands it the build of the
facts layer and nothing above; `inputs` names the engine binary. So it reruns when a stated fact,
the starting surface, its parameters, its script or the engine change, and not when a layer above
changes. A step cannot yet say "these features and this field only".

**The Rust side accepts three optional rasters** (raw little-endian `f32`, one per cell, row-major),
through `via terrain --initial F --uplift F --held F` and `via_terrain::run_given(cfg, &Given, ..)`.
With none given the stage behaves exactly as before.

| Raster | Meaning |
|---|---|
| `--initial` | The surface the model starts from, in place of its own noise. |
| `--uplift` | The uplift field, m per year, in place of its own. |
| `--held` | A finite value pins the cell at that height and makes it base level: the model cannot change it and water ends there. NaN leaves the cell free. Applied after the border ring. |

**How a stated fact reaches the model** (this is the step's own convention, in `tools/terrain.py`):

| Stated | Given to the model as | Checked afterwards |
|---|---|---|
| The land's edge | Held at the starting surface, on a margin of cells outside the world | |
| A lake at a level | A hollow in the starting surface, and its shore held at the level (a sill) | It is flooded, stands at its level, and the land does not fall away below it along its shore |
| A river from A to B | A groove in the starting surface that only falls and lies below all ground within reach of the line | The model's own water is followed down from the source: how far it keeps to the line, where it ends, how much land it gathers |
| A river by its two ends only (`course = "find"`) | A least-cost course over the starting surface, then as above | As above |

A held cell is where water ends, so it is right for an edge and a shore and wrong for a river,
which must carry water on. Holding a river's cells was tried first and gave a channel standing
above the land beside it.

The step hands back fields `height`, `water`, `wet`, `damp` and `silt`, and each river's found
course as a feature `<id>_course`, which layers above place things `on`. Its own diagnostics:
`stated.lake_dam`, `stated.lake_perched`, `stated.lake_drained`, `stated.lake_level`,
`stated.uphill`, `stated.mouth_dry`, `stated.gorge`, `stated.river_leaves`, `stated.river_wanders`,
`stated.river_dry`, `terrain.engine`. A lint step, `tools/facts_hold.py`, checks the same facts
again on the finished build (`fact.*`), in case a layer above broke one.

### 12.6l Main ways by their ends, and a measure of street form (amends 3.5 and 12.6j)

Shown in `townland`: a town on land the Rust terrain stage shaped (the step of 12.6k, unchanged),
with its main ways found over that land. Both additions are town library tools under
`kinds/tools/`; the compiler learned one small thing.

**A point placed `on` a feature is a place by its own id.** 12.6k made a point with `at` a place.
Now `starts_at`, `ends_at` may also name a point written `on = { feature, t, offset }`, provided
its host is not derived. It is resolved when first asked for. So a crossing or a market that stands
"on the river, halfway down" can end a way, and follows the river.

**`main_way` and `crossing`** (kinds in `kinds/town.toml`; the compiler knows neither). A main way
is a path with `starts_at`, `ends_at`, `path = []` and optionally `via = ["<point id>", ...]`,
plus `width` and `sidewalk`. A crossing is a point on water. The skeleton tool
(`kinds/tools/main_ways.py`, a generate step placed after the layer that states them) reads the
`height` and `water` fields beneath it and finds each course:

- over a grid of `cell` m (8), steps to the 16 nearest directions;
- a step costs its length times `1 + (gradient / easy_grade)^2`, times `water_cost` (40) through
  water further than `crossing_reach` (30 m) from a crossing, times `joined` (0.45) along a main way
  found earlier; the sea cannot be entered;
- the walk is resampled every 4 m and smoothed along its length over `ease` m (34), ends kept.

It writes, per main way: `street` features `<id>_<n>` marked `arterial = 1` (one per bank), a
`bridge` `<crossing>_deck` where it crosses a watercourse, and an `axis` `<id>_axis` along the
whole course for fields to measure from (`dist('kind:axis')`). Each feature named in the step's
`watercourses` is written back as a region `<id>_water` marked `water = 1`. Diagnostics, all with a
position: `main_way.place`, `main_way.no_course` (errors), `main_way.steep`, `main_way.wades`
(warnings).

**The city's street and lot tools** take two more step params: `limits = "<region id>"` (the town's
limits as a feature of this world, treated like limits handed down) and `close_ways = false`. They
keep off any region marked `water`.

**Street form** (`kinds/tools/street_form.py`, a lint step, or run by hand on a build folder with
`--ids <prefix>`). It writes `street_form.json` and one `info` diagnostic:

| Number | Meaning |
|---|---|
| `bearing_spread` | Streets cut into straight pieces, bearings folded to 0..90 degrees, weighted by length, 18 bins of 5 degrees; entropy over ln 18. 0 is one grid, 1 is every way alike. |
| `order` | The orientation order of the street-network literature (Boeing 2019): 36 bins over the full circle; 1 is a perfect grid. For comparison. |
| `main_bearing_spread`, `main_order` | The same for ways marked `arterial`, and roads. |
| `junctions` | Share of meeting places that are four-way or more, three-way, dead ends. Ends are carried 9 m to meet a centre line; places within 11 m are one. |

With `min_spread` in its params it warns (`one_grid`) below that.

### 12.6m A town seen on foot: the water's edge, the bridge, the bank (amends 12.6k and 12.6l)

Shown in `townland`. Three faults were visible only from the ground; all three fixes are library
work, and the compiler did not change. One shared kind gained an effect.

**A stated river** (`worlds/basin/kinds.toml`, `tools/terrain.py`) gained four things.

| Key | Meaning |
|---|---|
| `starts_at = "<an exit>"` | A river that starts on a feature of kind `exit` comes from beyond the map. Its groove is carried on into the model's margin, and its course begins at the map's edge. A parent that places the world must answer the exit, as for a road (12.6g). |
| `width = [at its source, at its mouth]` | The width grows between them. A single number still means one width. |
| `estuary`, `estuary_run` | How many times wider it is where it meets the sea, reached over the last `estuary_run` metres. Default 1 (none). |
| `bends` | How far it swings to each side, in widths, about every twelve widths. Bends are finer than the model's grid and are added after it, inside the valley it cut; they shrink where higher ground stands beside the river and vanish at its ends. Default 0. |

The cross-section is now stated in the tool: a bed under the water, the waterline at the stated
half width, a lip 0.45 m above the water 2.5 m beyond it, then back to the ground as it was. The
`water` field is carried 4.5 m beyond the waterline, under the bank, so the shore is where the
water's surface meets the ground and not the edge of a grid cell. The tool hands back
`<river>_course` (a path, every 4 m) and `<river>_course_water` (the water as a region, as wide as
it is at each point), and reports `sinuosity` (length over straight line) before and after bends.

**A `crossing` owns its bridge** (`kinds/town.toml`, `kinds/tools/main_ways.py`). A crossing is a
point on a watercourse. The skeleton tool gives it one straight deck, square to the water there,
named `<id>_deck`, between two bridgeheads `setback` metres back from the water's edge, and raises
the ground at each bridgehead to `clear` metres above the water plus the deck's thickness, with an
abutment's face to the water. A main way that names the crossing in `via` is led to one bridgehead
and away from the other, running `lead` metres in line with the deck before it may turn; a main way
that starts or ends at a crossing starts or ends at the bridgehead on its own side, chosen by cost.
The way's `street` stretches stop 6 m short of a bridgehead and the deck laps 4 m over them. A way
no longer cuts a deck out of itself, and a way that goes through water with no crossing is warned
(`main_way.wades`) and gets no deck. New errors: `crossing.no_water`, `crossing.dry`.

Keys on a crossing, with defaults: `width` 19, `setback` 8, `clear` 1.3, `lead` 36, `shoulder` 38.

**A `waterfront`** (`kinds/town.toml`, lowered by the street tool `worlds/city/tools/network.py`):
a street along the bank of a watercourse. The feature is a region that says which stretch of the
banks (a `where` condition will do), with `along = "<course>"`, `sides` ("both", "left", "right"),
`setback` (17), `width`, `sidewalk`. The tool lays a street `setback` metres from the water on each
side wanted, inside the region and the town's limits. It is an edge of the wards, so local streets
end on it, and a grid with no bearing of its own follows it. Error: `waterfront.no_water`.

**Loose ends.** With `join_within = <m>` in the street tool's params, a local street that ends on
nothing is carried straight on to the next street if one lies within that distance (and not across
water), and otherwise cut back to its last junction if the tail is shorter than `stub` (default the
same distance). A street left with no junction is dropped. Without the param nothing changes.

**The measure** (12.6l) now also gives `dead_ends` (their number) and `stubs` (dead ends under
40 m from their last junction), and counts a bridge as a street so a way does not end at each bank.

**`bridge`** (`kinds/ways.toml`) clears vegetation under its deck.

### 12.7 Diagnostics added (amends 7)

`field.file`, `field.image_coarse`, `step.point`, `step.after`, `step.failed`, `step.no_output`,
`step.ran`, `kit.module`, `place.no_ceiling`, `attr.invalid`, `attr.cycle`, `mount.unknown`, `mount.unmet`, `override.orphan`, `pin.conflict`, `lock.taken`, `lock.holds`, `deck.clearance`, `deck.misses`, `deck.detached`, `deck.grade`, `part.blend`, `part.named_in_apron`, `part.overruled`, `part.unknown`, `exit.unanswered`, `give.unknown`, `give.early`, `feature.where_empty`. The registry in
`worldc/diagnostics.py` holds the description of each.

From 12.6l, said by steps: `skeleton:main_way.place`, `skeleton:main_way.no_course` (errors);
`skeleton:main_way.steep`, `skeleton:main_way.wades`, `street_form:one_grid` (warnings);
`street_form:street_form` (info).

From 12.6m, said by steps: `skeleton:crossing.no_water`, `skeleton:crossing.dry`,
`network:waterfront.no_water` (errors).

### 12.8 Limitations added (amends 11)

- A step that returns a changed field causes the effects below it to be applied a second time.
- A step's record is one slot: building a world with and without an overlay by turns reruns it each time.
- A step that runs an outside program (the Rust terrain engine in `basin`) is only as portable as that program; the check suite then needs it built.
- A step reads only exported fields, and feature attributes only with `feature_attrs`.
- A region written as a condition is the largest connected piece. A river with banks steep enough to fail the condition splits a district in two and the smaller half is silently dropped; `townland` adds the river's own corridor to the condition to keep the district whole.
- A feature must have a shape. A `waterfront` is really "a path along another feature"; the core has no such shape, so it is written as a region.
- The step cache covers inputs, not outputs; undeclared inputs go stale unseen.
- A deck's end on the ground is not checked against any way: the author lines them up. Where a
  ramp meets a deck there is no merge geometry, and a deck is flat across its width.
- Decks ignore authority and are not seen by the reach lint or the player map's walkable layers.
- A deck's clearance over a region is measured at every station inside it, banks included.
- Two ways crossing at grade each grade the ground in turn; nothing models the junction itself.
- A kit run's pieces have no corners, no stretch and no filler; variation is by weighted choice only.
- `along` places a straight run on a curved path without checking how far it strays.
- Blocks, lots, districts, local streets and interchanges are not concepts: the `city` world's
  come from its own steps, as plain features and kit runs.
- A way cannot be interrupted by a deck. An arterial that crosses the river is still graded and
  painted across the bed, under the bridge the step lays over it.
- A deck's profile is a straight line between its ends plus `rise`, `ramp` and `settle`. Anything
  else (a crest, a dip under another deck) cannot be stated.
- Piers avoid only what a deck names in `over`; they can stand in a street it does not name.
