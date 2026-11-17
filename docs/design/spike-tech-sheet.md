# Tech sheet

The model as the spike implemented it (the spike is at the tag `spike-final` since M7), by the
tokens you type. One page for review. Every key with its default is in
`spike-final:spike/VOCABULARY.md` and `spike-final:spike/SPEC.md`; this sheet is the map between
them.
`ours` in the last column of a table means the compiler knows the token by name.

## What it is for

Decide what matters; the world settles around it. Change your mind, and it settles again. You
never repair a map by hand. In tokens: `authority` and `over` are how a decision is stated;
kinds, `on`, `along` and derived features are how the rest follows; `--overlay`, `[variant.*]`
and a rebuild are how a change is made.

## Pipeline

    load_doc (include, [[layer]], --overlay, --variant, [[step]] generate)
      -> World (features, shapes, frames)
      -> field("height") -> parts -> volumes -> derive (flood)
      -> place_explicit -> dress -> every other [field.*]
      -> scatter -> checks -> interiors + decks -> bookmarks, budgets
      -> emit build/NAME -> --lint, [[step]] lint, --map

## Data model

| Token | Type | Reads | Writes |
|---|---|---|---|
| `[world]` `extent` `resolution` `chunk` `seed` `pack` | grid and pack | | the grid every field lives on |
| `[field.<name>]` | scalar raster | `base`, `layers[]`, `let`, or `file` | `fields[name]` |
| `[field.<name>] type = "material"` | weight planes, at most 8 | `layers[] {paint, mask}` | `ground_N.rgba8` |
| `[field.<name>] type = "volume"` | bounded 3D density | volume expression | `vol_<id>.*`, and `height`, `bed`, `hole` |
| `[[feature]]` | point `at`, `path`, `region`, `ellipse` | `kind`, attributes, `on` | a shape, a frame, effects |
| `[kind.<name>]` | list of `[[effect]]` | `reach`, `defaults`, `let`, `authority` | fields, through features of that kind |
| `[[instance]]` | asset at a pose | `at` or `on`, `yaw`/`face`/`turn`, `scale` | `instances`, `explicit` |
| `[asset.<id>]` (pack) | declared facts | `size`, `footprint`, `solid`, `max_relief`, `avoid`, `module_m` | placement and checks |

## Field

| Token | Meaning |
|---|---|
| `base = "<expr>"` | the field before layers and effects |
| `layers = [{ value, mask }]` | `v + (value - v) * mask`, in order |
| `layers = [{ paint, mask }]` | material field: paint over what is there |
| `let.<name> = "<expr>"` | a named sub-expression |
| `file`, `channel`, `range`, `extent`, `outside`, `filter` | read a `.f32` raster or a picture in place of `base` |
| `unit = "m"` | makes the field measurable: only then can two anchors conflict |
| `tags = ["vegetation"]` | lets an effect target `#vegetation` |

Expression names: `x` `y` `fbm()` `ridged()` `f('field')` `slope('field')` `dist('id')` `sdist('id')`
`along('id')` `near('id', r)` `smoothstep(a, b, v)` `where` `min` `max` `clamp` `abs` `sqrt` `exp`
`pow` `sin` `cos`, and every `[params]` key. Expressions are Python evaluated over arrays.

## Kind and effect

| Token | Meaning |
|---|---|
| `[[kind.K.effect]] field = "F"` | the field this effect changes (`"#tag"` for every field with that tag) |
| `value`, `mask` | `F = v + (value - v) * mask` inside the feature's window |
| `paint`, `mask` | for a material field |
| `reach` | metres past the shape that the window extends |
| `max_change`, `check_where` | limit on how far the effect may move the field: `seam.change_exceeds` |
| `order` | place among effects of the same tier; a feature's own `order` overrides it |
| `authority` | `derived` < `shaped` < `anchored`; among anchors the smaller region is later |
| `over = ["id"]` (feature) | this anchor wins against that one |
| `defaults` | attribute values a feature may override; all numeric attributes are names in the effect |
| `map = { color, width }` | how features of this kind are drawn on the map |
| `deck = { ... }` | features of this kind become a mesh, not an effect |

Names inside an effect: `v` (field so far) `d` (distance outside) `sd` (signed distance) `t`
(fraction along a path) `profile` (field along the path, graded) `base` (mean under the shape)
`x` `y`, the kind's `let`, the feature's numeric attributes, `under('id')`.

## Feature

| Token | Meaning |
|---|---|
| `on = { feature, t, offset }` | position taken from another feature's frame |
| `flood = { field, level, fence, max_depth, freeboard }` | derived region: fill the ground and take the shore |
| `joins = ["id"]` | declared junction, checked: `seam.junction` |
| `order`, `authority`, `over` | see above |
| any other key | passes through to `world.json` |

## Placement

| Token | Lowers to | `ours` |
|---|---|---|
| `[[instance]] at / on / atop / beside` `face` `turn` `offset` `group` `in` | one placed instance | core |
| `[[instance]] kind = "K"` | lends its footprint to kind K as a region | core |
| `[set.<name>] items[]` | members in one local frame | yes |
| `[[run]] on, from, to, count, offset` | `count` instances along a feature | yes |
| `[[kitrun]] bay, from, to` or `along, side, at, bays, setback`, `end`, `faces`, `group` | bays of one `module_m`, plus one region feature | yes |
| `[[dress]] atop, assets, count` | random items on a host's top | yes |
| `[[flank]] hosts, asset, spread, forward` | a pair either side of each host's front | yes |
| `[[scatter]] assets, spacing, density, scale, seed` | jittered grid thinned by `density` | yes |

Checks on the result: `place.overlap` `seam.relief` `place.avoid` `kit.misfit` `kit.module`
`instance.outside` `budget.exceeded`.

## Geometry beyond the height field

| Token | Makes | `ours` |
|---|---|---|
| `[field.*] type = "volume"` + kinds `rockwork`, `cave` | marching-cubes mesh, blended to the ground | `height` `bed` `hole` |
| `[[level]] id, z, height` + kinds `room`, `tower_room`, `door`, `window`, `stair`, `lamp` | walls, floors, stairs, plans | all of them |
| `[kind.K] deck` + feature `width` `rise` `ramp` `settle` `over` `clear` `from_on` `to_on` `max_grade` | a slab along a path, piers, clearance check | `deck` |

## Composition

| Token | Does | Merge rule |
|---|---|---|
| `include = ["kinds/x.toml"]` | pulls in kind libraries | merged first, beneath everything |
| `[[layer]] id, file` | another document of the same world | lists joined; refs go down only: `layer.direction` |
| `--overlay FILE` | an external change | tables merge; lists with `id` merge by id; `remove = true` |
| `[variant.<name>]` + `--variant` | a named change | as an overlay |
| `--pack FILE` | replaces `[world] pack` | whole |
| `[[part]]` seam (`joins_at`, `apron`, `max_step`) | a world compiled alone, blended into the terrain | ids prefixed; translate only |
| `[[part]]` portal (a door) | an interior hung on a door | ids prefixed |
| `[[step]] at = "generate"`, `after`, `run`, `params`, `inputs` | a command writes `layer.json` and `.f32` fields | inserted above `after` |
| `[[step]] at = "lint"` | a command reads the build, writes `diagnostics.json` | codes become `<step>:<code>` |

## Step and SDK

Request (`$VIA_STEP`): `contract` `point` `id` `params` `frame {extent, resolution, nx, ny, seed}`
`build` `out`. Skipped when the hash of `run`, `params`, named files, `frame` and `build` is unchanged.

`viasdk`: `step()` -> `.params` `.frame.xy()` `.build.field(name)` `.build.features`
`.build.explicit` `.build.instances()` `.write_field(name, array)` `.add(table, row)`
`.set(table, key, value)` `.warn()` `.error()` `.done()`.

## Pack

| Token | Meaning |
|---|---|
| `root` | base folder for every file named |
| `[ground.<name>] color, texture, scale_m, tint` | a ground material |
| `[surface.<name>]` | passed through; generated meshes name one |
| `[asset.<id>] primitive, size` | a stand-in shape: `box` `cylinder` `cone` `tree` `tower` `house` `gable` |
| `[asset.<id>] source, fit_height / fit_width / scale, albedo` | a model file; bounds and triangles are measured |
| `[asset.<id>] recipe = { script, call, variants, ... }` | a generator the engine side runs; `size`, `tris` declared |

## Build

`world.json` (frame, fields, ground, assets, instances by chunk, explicit, features, meshes,
bookmarks) `height.f32` `water.f32` `<field>.f32` `ground_N.rgba8` `mesh_<id>.{verts,normals}.f32`
`mesh_<id>.faces.u32` `vol_<id>.*` `diagnostics.json` `budgets.json` `report.json` `minimap.png`
`layer_<id>.json` `map/` (`--map`) `reach.png` (`--lint reach`).

    python -m worldc WORLD.toml [--pack P] [--overlay F]... [--variant N]... [--out NAME] [--lint NAME]... [--map]

## Extension points

| Add | By | Compiler change |
|---|---|---|
| a feature type | `[kind.<name>]` in a library | no |
| a field | `[field.<name>]`; any effect may target it | no |
| data on a feature or instance | any extra key | no |
| a generator | `[[step]] at = "generate"` | no |
| a rule | `[[step]] at = "lint"` | no |
| assets | a pack | no |
| a producer | | yes: all six are code |
| a mesh generator | | yes: volumes, interiors, decks are code |
| an expression function | | yes: the name table is fixed |
| code inside a layer's resolution | | yes: steps join at layer boundaries only |

## Known weak points, by token

- `type = "volume"`, `[[level]]`, `deck`: three unrelated ways to leave the height field.
- `authority`, feature `order`, effect `order`: three orderings with no stated relation.
- `kitrun` `level` `room` `door` `stair` `deck` `bookmark` `poi` `height` `ground` `water`: names
  the compiler knows that belong in a library or a convention.
- `set` `run` `kitrun` `dress` `flank`: five overlapping ways to place a group.
- `[asset.<id>] source`: one file, where a real asset is a model, a texture and copies.
- `dist()` `sdist()` over the whole grid: unbounded cost.
- Nothing keys a hand edit to a lowered instance, so regeneration loses it.
