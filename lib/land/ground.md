# Ground paint: the land library's convention

The ground's paint is a `weights` field with role `material` (`spec/fields.md` 2.1), handed to the
engine as its terrain layers and drawn by the Godot loader weight-blended in each material's pack
colour (`spec/build.md` 15.1). The world owns it, as it owns the terrain (R25, R31): a library
declares no field, and a kind's effect reaches only its own feature, so the paint that covers the
whole ground is the world's rows. No kind of this library paints, since an effect on
`role:material` would be `authority.no_field` (info) in every world that has no such field.

The rows, in this order (`fields.md` 5.2: each row paints over the ones before it by its mask):

| Row | Paint | Mask | Why |
|---|---|---|---|
| `grass` | `grass` | none | Every post painted first, so `field.unpainted` never fires. |
| `scree` | `rock` | `= smoothstep(9, 13, slope('<surface>'))` | Bare rock where turf will not hold: the cut faces of a ford's approach or a lane's bed (a grade of 0.25 is 14 degrees); a beck's banks at `bank_grade` 0.2 (11 degrees) are shingle below. |
| `shingle` | `gravel` | `= 1 - smoothstep(4, 5.5, dist('<water path>', 8))` | The floor and banks of running water: `lib/water`'s beck is 6 m wide, so 1 to 2.5 m of bank beyond the water's edge. For a narrower stream, `width / 2 + 1` to `width / 2 + 2.5`. |
| `shore` | `gravel` | `= 1 - smoothstep(1.5, 4, dist('<water region>', 6))` | Where standing water meets the land: a lake's or a pond's region (`kind:lake`, `kind:pond`, one row or a `max` of one term each), its floor and 1.5 m of beach past its outline, fading by 4 m, so turf never runs straight into deep water (asked by `vale`, `relief`, `basin`). |
| `track` | `earth` | `= 1 - smoothstep(1.4, 2.2, dist('kind:lane', 4))` | A lane worn to earth: `lib/ways`' lane is 4 m wide, so the verge stays green. |

`<surface>` is the world's terrain field, `<water path>` its running water and `<water region>` its
standing water, each a feature name or a `kind:` selector (formulas read fields and features by name,
never by role). A selector must match at least one feature (`formula.empty_selection`), so a world
writes a row for the water and ways it has. A `road` or a `trail` (`lib/ways`) is a `track` row like
a lane's, at its own width: `dist('kind:road', 6)` from `width / 2 + 0.5`, `dist('kind:trail', 3)`
from `width / 2`. `gravel` and `earth` are not in the primitive pack of
`spec/assets.md` 6.1, which ships `grass` and `rock` only; a world painting them needs a pack that
binds them (`pack.unknown_material` otherwise).

## `ford`, painted

`ford` paints nothing (`build.md` 15.1). Its painted form was built and reviewed as a change, so its
base text, which `docs/plan.md` fixes and the graph cases' node lists read, is untouched:

```toml
[field.ground]
type = "weights"
role = "material"
[field.ground.then.grass]
paint = "grass"
[field.ground.then.scree]
paint = "rock"
mask = "= smoothstep(9, 13, slope('height'))"
[field.ground.then.shingle]
paint = "gravel"
mask = "= 1 - smoothstep(4, 5.5, dist('beck', 8))"
[field.ground.then.track]
paint = "earth"
mask = "= 1 - smoothstep(1.4, 2.2, dist('kind:lane', 4))"
```

with these two materials added to the primitive pack (`lib/packs/primitive.toml`, `assets.md` 6.1):

```toml
[material.gravel]
colour = [0.36, 0.33, 0.28]

[material.earth]
colour = [0.3, 0.22, 0.14]
```

Measured on `ford` with that change (0.5 m posts; chunk edge posts counted once per chunk): posts
whose byte is above one half, grass 246,783, gravel 9,848, rock 5,537, earth 3,791. The steepest
ground is 13 degrees, so `scree` paints only the cut faces of the lane beds and the ford's approach.
Folding the rows into `ford`'s base adds a `field:ground` node per chunk and the `distance:kind:lane`
nodes, so six graph cases' exact node lists or cited lines (`downstream`, `vertex`, `shuffle`,
`undo_chain` among them, measured by running the graph chapter on a copy) and `docs/plan.md`'s ford block change with it: a spec change, not a library one.

## Seasons are changes (M2, `vale`)

A season is not a mechanism: it is a change (`documents.md` 6) that moves the world's own `let`
values the paint rows and densities read. The land library's convention is one `let`, `snowline`,
metres above which snow lies, read by a `snow` row painted last, and by any density that should
thin under snow:

```toml
[let]
snowline = 150

[field.ground.then.snow]
paint = "snow"
mask  = "= smoothstep(snowline - 7, snowline + 7, f('height') + 7 * fbm(38, 2, seed = 'drift')) * (1 - smoothstep(44, 56, slope('height')))"

[change.winter.let]
snowline = 34
```

`--change winter` then repaints the ground above 34 m; only what reads `snowline` is made again
(`graph.md` 5.1). Snow on steep ground slides: the second factor leaves faces steeper than about
50 degrees bare. `snow` is not in the primitive pack; a world painting it needs a pack that binds it
(`pack.unknown_material` otherwise).
