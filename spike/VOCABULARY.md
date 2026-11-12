# worldc vocabulary

Every word of the world format, the asset pack, the diagnostics, the step contract and the SDK, on one page.
Checked against the code in `worldc/`, `kinds/`, `packs/` and `sdk/`. `SPEC.md` remains the full reference.
Tags: `core` a mechanism with no domain noun; `domain` a domain word the compiler knows by name; `lib` data only.

## 1. Concepts and tables

| Concept | Tag | Meaning | Known problem |
|---|---|---|---|
| frame | core | A coordinate system: world, feature local, level or instance. | |
| post | core | One sample of the world grid. | |
| field | core | A named function over the world frame, resolved to an array. | |
| scalar field | core | One float per post. | |
| material field | core | One weight per post for each ground material. | Only one, named `ground`; 8 materials. |
| volume field | core | A 3D grid inside a host box; solid where positive. | One overhang per column. |
| volume host | core | A region feature whose kind `hosts` a volume field. | Box is axis aligned. |
| feature `[[feature]]` | core | A point, path or region with id, kind and attributes. | |
| attribute | core | A numeric feature key; a name in its kind's expressions. | Can shadow a built-in name. |
| derived feature | core | A feature whose shape is computed from a field. | |
| flood | domain | The derived region of water below a level around a point. | Needs `at`; fails with `on`. |
| kind `[kind.<name>]` | core | A named table of effects, defaults and flags. | |
| effect | core | Changes one field near each feature of its kind. | |
| authority | core | `derived`, `shaped` or `anchored`; sets effect order, anchored last. | Anchors compared only on fields with `unit`. |
| expression | core | A number, or a string evaluated over arrays. | Python `eval`. |
| instance `[[instance]]` | core | One asset at a position with yaw, scale and tint. | Ids not checked unique. |
| explicit, scattered instance | core | Written or produced with an id; or made by scatter, without. | Scattered ones are in no layer file. |
| footprint feature | core | The region an instance with `kind` adds under itself. | Rectangle even for round assets. |
| asset | core | An opaque id that a pack binds to a shape and facts. | |
| pack | core | A TOML file of assets, ground materials and surfaces. | Master's pack wins over a part's. |
| ground material, surface | core | Named ground layer; named material of generated meshes. | |
| producer | core | Lowers a compact form to plain instances. | |
| set `[set.<name>]` | core | A named group of items placed as one unit. | Passes only position, yaw, `in`. |
| run `[[run]]` | core | One instance spec repeated along a path or outline. | |
| kit run `[[kitrun]]` | domain | Bays of one module laid end to end on one pad. | No corners, stretch or filler. |
| dress `[[dress]]` | core | Small assets strewn at random on top of an instance. | Items with no room dropped silently. |
| flank `[[flank]]` | core | A pair of one asset either side of a host's front. | |
| scatter `[[scatter]]` | core | A density expression turned into instances on a jittered grid. | Avoids only own solids, by about 1 m. |
| variant `[variant.<name>]` | core | A table shaped like a world, merged by `--variant`. | Lists without ids replaced whole. |
| overlay | core | A separate file merged over the world from the command line. | Cannot add a layer. |
| layer `[[layer]]` | core | A separate document joined to the world; refers downward only. | Id clash checked between layers only. |
| part `[[part]]`, master | core | A world compiled alone and translated into the master. | Not rotated, scaled or nested. |
| seam part | core | A part that brings terrain, blended over an apron. | Fields blended without its lift. |
| portal part | domain | A part that brings an interior, hung on one door. | `hole` omits its walls. |
| step `[[step]]` | core | A user command that joins the build at `generate` or `lint`. | A part's steps are not run. |
| level `[[level]]` | domain | A named floor height; interiors are built only with one. | Not brought with a part. |
| room, opening, stair, light | domain | Interior features: walled region, hole in a wall, steps, lamp. | Lights need levels. |
| deck | domain | A path carried as its own mesh above the ground. | Ignores authority; unseen by reach lint. |
| bookmark | domain | A point feature that becomes a camera position. | Kind name hard-coded. |
| poi | domain | A point feature that becomes a map icon. | Kind name hard-coded. |
| expectation `[[expect]]` | domain | Targets that must be reachable, or not, from a start. | Replaced whole by overlay or variant. |
| profile `[profile.<name>]` | domain | How steep and how deep a walker can go. | |
| lint | core | A check that runs only when asked; `reach` is the only one. | Sees the top surface only. |
| chunk, budget `[budget]` | core | A square that instances are grouped by; a limit per chunk. | Source counts, not drawn ones. |
| diagnostic | core | Severity, code, subject, message; optional place and hint. | |
| `[world]` | core | The frame and its pack. | |
| `[params]`, `[look]` | core | Names for every expression; a table passed to the viewer. | `[params]` can shadow built-ins. |
| `[export]` | core | What extra data the build carries. | |

## 2. Document keys

### 2.1 Top level, `[world]`, `[budget]`, `[export]`, `[[layer]]`, `[[part]]`, `[[step]]`, `[[level]]`, `[profile.*]`, `[[expect]]`

| Key | Tag | Meaning | Known problem |
|---|---|---|---|
| `include` | core | Files merged before the world's own text. | Read from world and layer files only. |
| `remove` | core | On a list item with `id`: delete that item when merging. | |
| `world.extent` | core | `[x0, y0, x1, y1]`, m. Required. | |
| `world.resolution` | core | Post spacing, m (0.5). | Parts must share it. |
| `world.seed`, `world.chunk` | core | Seed of all noise and randomness (0); chunk side, m (64). | |
| `world.pack`, `world.name` | core | Path of the asset pack; name written to `world.json`. | |
| `budget.max_instances_per_chunk`, `budget.max_source_tris_per_chunk` | core | A chunk over either raises `budget.exceeded`. | |
| `export.fields` | core | Scalar fields written as rasters besides `height`. | Steps read only exported fields. |
| `export.feature_attrs` | core | Adds resolved `attrs` to every feature in `world.json`. | |
| `layer.id`, `layer.file` | core | Name and path of a layer; rows ordered lowest first. | Read from the world file only. |
| `part.id`, `part.file` | core | Prefix of all it brings; the part's world file. | Inner references stay unprefixed. |
| `part.pack` | core | Pack the part is compiled with (the master's). | |
| `part.at`, `part.footprint` | core | Seam: translation; rectangle brought (the part's extent). | |
| `part.apron`, `part.max_step` | core | Seam: blend width (20); limit on the edge step (3), m. | Volumes in the apron miss the ground. |
| `part.joins_at` | core | Seam: point where part and master heights are equal. | |
| `part.expects` | domain | Seam: `"land"` makes water at the edge an error (land). | |
| `part.portal` | domain | `{ door, at }`: put the part's door at a master point. | |
| `part.max_relief` | domain | Portal: limit on master relief under the part, m (0.4). | |
| `step.id`, `step.at` | core | Name of the step; `generate` or `lint`. | |
| `step.run` | core | The command, run in the world's folder. | |
| `step.after` | core | Generate: run above this layer, handed its build. | Changed field: lower effects apply twice. |
| `step.params`, `step.inputs` | core | Table passed unread; further files whose change reruns it. | Undeclared inputs go stale. |
| `level.id`, `level.z`, `level.height` | domain | Name, floor height and wall height, m. | |
| `level.wall`, `level.floor` | domain | Surfaces of the wall and floor meshes ("plaster", "timber"). | Stairs and roofs use fixed surfaces. |
| `profile.max_slope_deg`, `profile.max_wade_m` | domain | Steepest ground (required); deepest water (0). | |
| `expect.from`, `expect.to` | domain | Start and targets: features with `at`, or instances. | Region or path needs `at`. |
| `expect.by` | domain | The profile's name. | |
| `expect.reach`, `expect.why` | domain | `false`: must not be reachable (true); text for diagnostics. | |

### 2.2 `[field.<name>]`

| Key | Tag | Meaning | Known problem |
|---|---|---|---|
| `type` | core | `material` or `volume`; anything else is scalar. | |
| `unit` | core | Any value switches on the comparison of anchors. | Not a unit. |
| `tags` | core | Effects on `#tag` target every 2D field with it. | Only `vegetation` is built in. |
| `let` | core | Named sub-expressions, evaluated in order. | |
| `base` | core | The field before layers and effects. | Not read for material fields. |
| `layers` | core | List applied in order after `base`. | Not read for volume fields. |
| `layers[].value`, `layers[].mask` | core | Scalar layer: target value; blend weight 0..1 (1). | |
| `layers[].paint` | core | Material layer: the ground material painted by `mask`. | |
| `cell` | core | Volume only: cell size, m (0.75). | |
| `file` | core | Scalar only: read from `.f32` or a picture; replaces `base`. | |
| `channel`, `filter` | core | Picture: `r`, `g`, `b`, `a`, `luma`; `nearest`, `linear`, `cubic`. | `channel` ignored for deep pictures. |
| `range` | core | Picture: values black and white stand for (`[0, 1]`). | |
| `extent`, `outside` | core | Picture: where it lies (the world); value beyond it. | |
| field name `height` | domain | The ground surface; instances, floods, volumes and checks read it. | Required by name. |
| field name `ground` | domain | The one material field. | Required by name. |
| field name `water` | domain | Water surface; read by bookmarks, map, lint, parts, decks. | No generated geometry. |
| field names `bed`, `hole` | domain | Made by the compiler: lowest floor; where terrain is replaced. | |
| tag `vegetation` | domain | A master field with it is cleared inside a seam part. | Hard-coded. |

### 2.3 `[kind.<name>]` and its `effect` rows

| Key | Tag | Meaning | Known problem |
|---|---|---|---|
| `authority` | core | Authority of the kind's features (shaped). | |
| `reach` | core | Distance beyond which effects are not evaluated, m (48). | Not read for volumes. |
| `defaults` | core | Attribute values a feature has unless it sets them. | |
| `let` | core | Named sub-expressions for the kind's effects. | Missing name: silent in 2D, error in volume. |
| `map` | core | Draws the kind on `minimap.png`: `color`, `width`. | |
| `hosts` | core | Name of the volume field a region of this kind boxes. | |
| `measure` | domain | `"water"`: features are measured as water bodies. | |
| `encloses` | domain | Features of this kind are rooms. | |
| `opening` | domain | `"door"` is a door; any other value is a window. | |
| `link` | domain | `"stair"`: features of this kind are stairs. | |
| `light` | domain | Point features emit a light: `color`, `range`, `energy`, `h`. | Needs levels. |
| `module_m` | domain | Rooms: wall module checked by `kit.remainder`. | |
| `deck` | domain | Path features become a mesh, with no effect on fields. | |
| `deck.surface`, `deck.thickness` | domain | Mesh surface ("concrete"); slab thickness, m (1.0). | Defaults only in code. |
| `deck.parapet`, `deck.pier_every` | domain | Edge wall height (1.0); pier spacing, m (0, none). | Piers avoid only what `over` names. |
| `effect[].field` | core | A field name, or `#tag`. | Undeclared field: never applied. |
| `effect[].value`, `effect[].mask` | core | `v = mix(v, value, mask)`; mask is zero beyond `reach`. | `effect.unbounded` skipped at world edge. |
| `effect[].paint` | core | Material field: the ground material painted by `mask`. | |
| `effect[].reach` | core | Overrides the kind's reach for this effect. | |
| `effect[].max_change`, `effect[].check_where` | core | Limit on the change; applies only where the second is above 0. | Error, yet still applied. |
| `effect[].order` | core | Place among all effects of its tier (0). | |

### 2.4 `[[feature]]`, with volume, interior, bookmark and deck keys

| Key | Tag | Meaning | Known problem |
|---|---|---|---|
| `id` | core | Unique among features, footprints and kit run pads. | |
| `kind` | core | A declared kind; undeclared passes through. | |
| `path`, `smooth` | core | A polyline; Catmull-Rom smoothing (true; false with `path3`). | Written out every eighth sample. |
| `path3` | core | A 3D path; volume effects read it. | |
| `region`, `ellipse` | core | A closed polygon; or `{ center, size }`, 72 sides. | |
| `at` | core | A point `[x, y]`. | |
| `on` | core | `{ feature, t, offset }`: a point in another feature's frame. | No `slide`; host must come first. |
| `flood` | domain | Makes the feature a derived water region. | |
| `flood.field`, `flood.level` | domain | Field read ("height"); level, m, or `"spill"` (spill). | |
| `flood.fence`, `flood.max_depth`, `flood.freeboard` | domain | Radius kept inside (120); spill search limit (8); margin (0.3). | |
| `joins` | core | Path ids one end of this path must lie on. | |
| `authority` | core | Overrides the kind's authority. | |
| `over` | core | Anchors it wins against; silences conflicts. | Only its length orders effects. |
| `order` | core | Stands for the `order` of every effect of this feature. | |
| `z`, `apron` | core | Volume host: `[z0, z1]` of the box; blend width, m (6). | |
| `in` | domain | A room id: the feature gets that room's floor height. | |
| `grade`, `max_grade` | domain | Path: averaging length, m (24); steepest percent allowed. | Checked on every scalar field touched. |
| `width` | domain | Path in `joins` (2); opening (1); stair (1); deck (8). | Four meanings, four defaults. |
| `level`, `wall` | domain | Room or light: its level; room wall thickness, m (0.4). | |
| `sill`, `head` | domain | Opening: bottom and top above the level, m (0, 2.1). | `on.offset` does not move the hole. |
| `to` | domain | Door: a room id or `"outside"` (outside). | `outside` is reserved. |
| `from`, `to` | domain | Stair: the two rooms it joins. | Not read by `layer.direction`. |
| `arc`, `line` | domain | Stair: `{ center, radius, from_deg, to_deg }`, or two points. | Also needs `at`; neither: exception. |
| `riser` | domain | Stair: target step height, m (0.19). | Ignores kind defaults. |
| `h` | domain | Light: height above its level, m. | |
| `look` | domain | Bookmark: target point, or `{ feature, t }` on a path. | |
| `eye`, `look_height` | domain | Bookmark: eye (1.7) and target (1.5) height above ground, m. | |
| `fov`, `note` | domain | Bookmark: field of view, degrees (55); free text. | |
| `under`, `cut_z` | domain | Bookmark: stand on `bed`; hide meshes from this height up. | `cut_z` disables view checks. |
| `label` | domain | Kind `poi`: label of the map icon. | |
| `rise`, `ramp` | domain | Deck: height gained (0); distance to gain it (half length), m. | No other profile can be stated. |
| `settle` | domain | Deck: end height difference taken up over the last metres. | |
| `over` (deck), `clear` | domain | Deck: features it must clear; room needed, m (4.5). | Region clearance includes banks. |
| `from_on`, `to_on` | domain | Deck: that end rests on the named deck. | Ground ends not checked against ways. |

### 2.5 `[[instance]]` and `[set.<name>]`

| Key | Tag | Meaning | Known problem |
|---|---|---|---|
| `id` | core | Default `instance_<n>`; required with `set`. | Default depends on index. |
| `asset` | core | An asset id of the pack. | |
| `at`, `yaw` | core | World position; front direction, degrees from east (0). | `yaw` ignored with `on`. |
| `on` | core | `{ feature, t, offset, slide }`: a place in a feature's frame. | Must be a table. |
| `face`, `turn` | core | With `on`: `toward`, `in`, `along` or a yaw; added degrees. | Both ignored with `at`. |
| `in` | domain | A room id: stand on its level, checked against it. | |
| `atop` | core | Stand on top of an instance placed earlier. | Outdoor host: placed from z = 0. |
| `beside` | core | Stand on the ground in an earlier instance's frame. | |
| `offset` | core | With `atop`, `beside`: `[right, forward]`, m. | |
| `against` | core | With `on`: stand inside, back to the edge. | Replaces `on.offset`. |
| `mount` | core | Height above the floor, m; needs a floor (0). | |
| `scale`, `tint` | core | Uniform scale (1); colour multiplier. | |
| `kind`, `margin` | core | Also a region feature of this kind, grown by `margin`. | On a flood: never reaches `height`. |
| `set` | core | Place the named set here. | |
| `group` | core | Same group: not tested against each other for overlap. | |
| any other key | core | A free attribute, written as `attrs`, not interpreted. | Includes `over`, `authority`. |
| `set.items` | core | The set's members. | |
| `items[].name`, `items[].asset` | core | Member id is `<instance id>.<name>`; its asset. | |
| `items[].offset`, `items[].atop`, `items[].turn` | core | Place in the set's frame; stand on a member; added yaw. | |

### 2.6 `[[run]]`, `[[kitrun]]`, `[[dress]]`, `[[flank]]`, `[[scatter]]`

| Key | Tag | Meaning | Known problem |
|---|---|---|---|
| `run.id` | core | Default `run_<n>`; children are `<id>.<k>`. | |
| `run.on` | core | The path or outline to run along. | Must be a string. |
| `run.from`, `run.to`, `run.count` | core | The stretch, as fractions (0, 1); number of children (1). | |
| `run.offset` | core | Offset along the normal; without it children stand `against`. | |
| other `run` keys | core | Copied to each child as instance keys. | |
| `kitrun.id` | domain | Default `kitrun_<n>`; bays `<id>.<k>`; the pad takes the id. | |
| `kitrun.bay` | domain | An asset id, or a list of `{ id, weight }` sharing one module. | |
| `kitrun.end` | domain | Asset for the first and last bay. | |
| `kitrun.seed`, `kitrun.group` | domain | Seed of the bay choice (row index); overlap group (the id). | |
| `kitrun.from`, `kitrun.to` | domain | The two ends `[x, y]`; required without `along`. | `kit.misfit` is an error, bays still placed. |
| `kitrun.faces` | domain | `left` or right of the run direction (left). | Forced to `left` with `along`. |
| `kitrun.tolerance`, `kitrun.margin` | domain | Allowed misfit share (0.01); pad margin, m (1). | |
| `kitrun.kind` | domain | Kind of the pad feature ("pad"). | Default name hard-coded. |
| `kitrun.along` | domain | A path the run stands beside; replaces `from`, `to`. | Straight chord on a curve, unchecked. |
| `kitrun.side`, `kitrun.at` | domain | With `along`: `left` or `right`; start fraction (0). | |
| `kitrun.bays`, `kitrun.setback` | domain | With `along`: number of modules (1); centre line to fronts, m (0). | |
| `dress.id`, `dress.atop` | core | Default `dress_<n>`; the host instance (required). | |
| `dress.assets`, `dress.count` | core | Choices `{ id, weight }`; items to place (3). | |
| `dress.gap`, `dress.margin`, `dress.seed` | core | Item spacing (0.16); edge margin (0.08), m; seed. | |
| `flank.id` | core | Default `flank_<n>`; items `<id>.<host>.l` and `.r`. | |
| `flank.hosts`, `flank.asset` | core | Instance ids that each get a pair; the asset placed. | |
| `flank.spread`, `flank.forward` | core | Distance to each side (1.8) and before the front (0.9), m. | |
| `scatter.id` | core | Default `scatter_<n>`; written on each instance as `by`. | |
| `scatter.assets`, `scatter.spacing` | core | Choices `{ id, weight }`; grid cell side, m (required). | |
| `scatter.density` | core | Expression, 0..1: chance a candidate is kept (1). | |
| `scatter.scale`, `scatter.tint`, `scatter.seed` | core | `[lo, hi]` random scale and tint; seed (row index). | |

`[look]` keys, read by the viewer only: `sky`, `sun_azimuth`, `sun_elevation`, `sun_color`, `sun_energy`, `sky_top`,
`sky_horizon`, `fog_color`, `ambient`, `haze`, `horizon`, `surf`, `water_deep`, `water_shallow`, `rock_a`, `rock_b`,
`rock_tint`, `rock_band_m`. The viewer also reads `params.sea`, the field `grass` and the material `rock` by name.

## 3. Names inside expressions

| Name | Where | Meaning |
|---|---|---|
| `x`, `y`, `z` | 2D and volume; `z` volume only | Post or cell coordinates, m. |
| `pi`, `exp`, `pow(a, b)` | 2D only | The constant; element-wise functions. |
| `sin`, `cos`, `abs`, `sqrt` | 2D and volume | Element-wise functions. |
| `min(a, b)`, `max(a, b)` | 2D and volume | Element-wise, exactly two arguments. |
| `clamp(v, lo, hi)`, `where(c, a, b)` | 2D and volume | Clip; choose by condition. |
| `smoothstep(a, b, v)`, `mix(a, b, t)` | 2D and volume | Cubic 0..1 step; linear blend. |
| `fbm(wavelength, octaves, seed, gain, lacunarity)`, `ridged(...)` | 2D only | Gradient noise, about -1..1; ridged noise, 0..1. |
| `fbm3(wavelength, octaves, seed)`, `fbm2(...)` | volume | 3D noise; 2D noise constant in `z`, not normalised. |
| `f('name')` | 2D and volume | Another field (2D); a 2D field at the column (volume). |
| `slope('name')` | 2D only | Slope of a scalar field, degrees. |
| `dist('id')`, `sdist('id')` | 2D only | Distance, signed distance to a feature, m. |
| `along('id')` | 2D only | Fraction along a path; fails on a region or point. |
| `near('id', radius)` | 2D only | 1 at the feature, falling to 0 at `radius`. |
| every `[params]` key | 2D and volume | Its value; overrides the names above. |
| field `let` names | field `let`, `base`, layers | Earlier sub-expressions of the same field. |
| `v` | scalar layer, effect | The field or volume as it stands. Not in material masks. |
| each attribute | effect, `max_change` | Kind `defaults` under the feature's numbers. |
| `d`, `sd` | effect | Distance outside the feature; signed distance. |
| `t` | effect | Fraction along the path; 0 for region or point. `path3` only in volumes. |
| `profile` | 2D effect, path, scalar | The field along the path, averaged over `grade`. |
| `base`, `base_min`, `base_max` | 2D effect, scalar | Mean, minimum, maximum of the field under the feature. |
| `under('field', how)` | 2D effect | Another field under the feature: `mean`, `min`, `max`, `edge_min`. |
| kind `let` names | effect | The kind's sub-expressions, in order. |
| `ground`, `steep`, `crest` | volume | `height - z`; slope of `height`, degrees; highest ground nearby minus `z`. |
| `tube`, `pd`, `pz` | volume effect, `path3` | 3D distance; plan distance; path height at nearest point. |

Scatter `density` has only the 2D names. String `max_change` has only the feature's attributes. A failed 2D
expression becomes 0; a failed volume expression becomes `ground`.

## 4. Shipped kinds (`kinds/*.toml`)

| Kind | Tag | File, meaning and attributes (defaults) | Known problem |
|---|---|---|---|
| `guide` | lib | terrain: a line others refer to; no effect. | |
| `cliff` | lib | terrain: lifts ground behind an edge. `drop` 15, `face` 3, `back` 70. | |
| `lake` | lib | terrain: digs a bed and fills it. `level`, `depth` 3.5, `shore` 13, `rim` 26, `max_bank` 2.5. | |
| `stack` | lib | terrain: a rock pillar at a point. `radius` 12, `top` 40. | |
| `pad` | domain | terrain: level ground under a footprint, anchored. `margin` 1.5, `skirt` 6, `max_cut` 2.5, `trodden` 1. | Kit run default by name. |
| `road` | lib | terrain: a graded way. `width` 5, `shoulder` 7, `grade` 30, `max_grade` 20. | |
| `terrace` | lib | terrain: a level paved court, anchored. `bank` 4, `max_cut` 4.5. | |
| `lawn` | lib | terrain: grass let back into a court, anchored. | |
| `rockwork` | lib | rock: hosts volume `solid`. `rough` 7, `bulk` 6, `ledge` 2.4, `bed_m` 6, `flute` 0, `flute_m` 5, `notch_depth` -12, `foot` 0. | Field name `solid` fixed in the file. |
| `cave` | lib | rock: a passage along a `path3`. `width` 4.2, `rise` 3. | |
| `room` | lib | building: `encloses`, `module_m` 1. `wall` 0.4. | |
| `tower_room` | lib | building: `encloses`, no module. `wall` 0.8. | |
| `door` | lib | building: `opening = "door"`. `width` 1, `head` 2.1. | Compiler knows the flag value, not the kind. |
| `window` | lib | building: `opening = "window"`. `width` 0.9, `sill` 1, `head` 2. | |
| `stair` | lib | building: `link = "stair"`. | Compiler knows the flag value, not the kind. |
| `lamp` | lib | building: `light` with `range` 7.5, `energy` 1.8, `h` 2.3. | |
| `bookmark` | domain | review: a camera; clears vegetation round it. | Name hard-coded. |
| `poi` | domain | review: a map icon with `label`. | Name hard-coded. |
| `street` | lib | ways: kerbed road. `width` 9, `sidewalk` 3, `kerb` 0.15, `shoulder` 6, `grade` 40, `max_grade` 12. | Crossings are not modelled. |
| `motorway` | lib | ways: `width` 18, `verge` 4, `shoulder` 12, `grade` 80, `max_grade` 6. | |
| `railway` | lib | ways: ballast bed. `width` 5, `shoulder` 8, `grade` 120, `max_grade` 3. | |
| `waterway` | lib | ways: drawn on the map only. | |
| `bridge` | lib | ways: deck. `width` 12, `rise` 0, `clear` 4.5, `max_grade` 14. | A way cannot be interrupted by a deck. |
| `freeway` | lib | ways: deck on a viaduct. `width` 20, `rise` 8, `clear` 4.5, `max_grade` 12. | |
| `ramp` | lib | ways: deck between levels. `width` 7, `rise` 0, `clear` 4.5, `max_grade` 8. | No merge geometry where it meets a deck. |
| `rail_bridge` | lib | ways: steel deck. `width` 6, `rise` 0, `clear` 4.5, `max_grade` 3. | |
| `lot` | lib | ways: paved, cleared ground under buildings. | |
| `park` | lib | ways: paints meadow. | |

Fields the shipped kinds write: `height`, `ground`, `water`, `clear`, `grass`, `solid`, and the tag `#vegetation`.

## 5. Asset pack

| Key | Tag | Meaning | Known problem |
|---|---|---|---|
| `root` | core | Base folder for the pack's files (the pack's folder). | Relative value resolves from the working directory. |
| `[ground.<name>]` | core | Declares a ground material. | |
| `ground.color`, `ground.tint` | core | Flat colour; multiplier on the texture. | |
| `ground.texture`, `ground.scale_m` | core | An image; metres one repeat covers (4). | |
| `[surface.<name>]` | core | A mesh material; passed through. The viewer reads `color`. | |
| surfaces `plaster`, `timber`, `roof`, `concrete` | domain | Names the compiler gives walls, floors and stairs, roofs, decks. | |
| `[asset.<id>]` | core | Declares one asset. | |
| `asset.tags` | core | List of strings. | Only `landmark` is read. |
| tag `landmark` | domain | Explicit instances of the asset become map icons. | |
| `asset.avoid` | core | Fields where scatter drops it and `place.avoid` fires. | |
| `asset.sink`, `asset.max_relief` | core | Lowered this far, m; tolerated relief under the footprint, m. | |
| `asset.clearance` | core | Stored and written. | Never read. |
| `asset.color` | core | Colour (`[0.7, 0.7, 0.7]`). | |
| `asset.roof` | domain | Roof colour of `house` and `gable`. | |
| `asset.solid` | core | Takes part in overlap, door, scatter and reach checks (true). | |
| `asset.footprint` | core | Ground rectangle `[across, depth]` (first two of `size`). | |
| `asset.module_m` | domain | Spacing a kit run repeats the asset at. | |
| `asset.primitive` | core | Shape name ("box"). Used when no `source` or `recipe`. | |
| `box`, `cone`, `sphere`, `tree`, `blob`, `cylinder`, `tower` | domain | Primitive names the compiler has triangle counts or roundness for. | `cylinder`, `tower` count 12. |
| `house`, `gable` | lib | Primitive names only the viewer knows. | Compiler counts 12 triangles. |
| `asset.size`, `asset.tris` | core | `[across, depth, height]`, m; triangle count. Measured for a model. | |
| `asset.round` | core | Elliptical footprint polygon. | Primitives only. |
| `asset.source` | core | A binary glTF model; size and triangles are measured. | |
| `asset.fit_height`, `asset.fit_width`, `asset.scale` | core | Model scale: by height, else by width, else this factor. | |
| `asset.albedo` | core | A texture for the viewer. | Not checked for existence. |
| `asset.recipe` | core | Engine-side generator table; `script`, `atlas` must exist. | Needs declared `size`, `tris`. |

## 6. Diagnostics (88 codes, equal to `CODES`; E error, W warning, I info)

| Group | Codes |
|---|---|
| Document, pack | `layer.direction` E refers to a higher layer; `pack.missing_file` E pack file not found; `pack.undeclared` E recipe lacks `size`, `tris`; `asset.unknown` E asset not in the pack |
| Features | `feature.no_id` E no id; `feature.duplicate` E id used twice; `feature.shape` E no geometry |
| | `feature.outside` W wholly outside the extent; `kind.passthrough` I kind not declared; `frame.unknown` E host is not the path or region needed |
| Fields, effects | `expr.invalid` E expression failed; `field.unresolved` E field could not be resolved; `field.unknown` E reads an undeclared field |
| | `field.cycle` E fields read each other in a circle; `feature.unknown` E measures from a missing feature; `field.required` E no `height` or `ground` |
| | `ground.too_many` E more than eight materials; `ground.unknown` E material not in the pack; `effect.unbounded` W mask not zero at its reach |
| | `seam.change_exceeds` E change over `max_change`; `seam.grade` W path steeper than `max_grade` |
| | `field.file` E field file missing or wrong size; `field.image_coarse` W 8-bit picture shows steps |
| Authority | `authority.unknown` E not one of the three; `authority.conforms` I flood's effect on its source skipped; `authority.conflict` E two anchors disagree at their cores |
| | `authority.squeezed` W two anchors' aprons meet; `authority.cycle` E instance on a flood reshapes its source |
| Water | `water.no_basin` E spill flood holds no water; `water.leaks` W flood passes its fence; `water.dry` E or W no water or no closed shore |
| Placement | `instance.no_position` E no `at`, `on`, `atop`, `beside`; `instance.outside` E outside the extent; `set.unknown` E set not declared |
| | `kit.misfit` E length is not whole bays; `kit.module` E pieces differ in module; `kit.remainder` W walls do not divide by module |
| | `dress.no_host` E host not placed; `flank.no_host` E host not placed; `place.no_host` E `atop`, `beside` host not placed before |
| | `place.off_surface` W `atop` offset past the host; `place.overlap` E two solid footprints overlap; `place.avoid` W stands where an avoided field is high |
| | `seam.relief` W ground under footprint too uneven; `seam.junction` W `joins` end not on the target; `scatter.count` I how many a scatter made |
| Interiors | `room.no_level` E level not declared; `room.sealed` E not joined to outside; `opening.no_wall` E not `on` a room |
| | `door.dangling` E leads to no touching room; `seam.threshold` W door level misses the ground; `stair.dangling` E `from` or `to` is not a room |
| | `stair.flat` E rooms under 0.5 m apart; `stair.steep` W step too high or short; `stair.through_wall` E outside both rooms |
| | `place.no_room` E `in` is not a room; `place.in_wall` E footprint outside its room; `place.over_opening` E mounted across an opening |
| | `place.on_stair` E on a stair or well; `place.blocks_door` E solid in a door's zone |
| Volumes | `volume.outside` W `path3` leaves its volume; `volume.floor` E air reaches the bottom; `volume.ceiling` E solid reaches the top |
| Parts | `part.no_door` E portal door missing; `part.outside` E footprint outside the master; `part.ground` W ground under a portal uneven |
| | `part.seam` E edge step over `max_step`; `part.volume_apron` W volume reaches into the apron; `part.context` E edge under water, land expected |
| Outputs | `bookmark.blocked` W terrain hides the target; `bookmark.crowded` W instances in the near view; `bookmark.inside` W eye inside a footprint; `budget.exceeded` W chunk over a budget |
| Decks | `deck.clearance` E too little room over a crossing; `deck.detached` E end lies off the deck it rests on; `deck.grade` W steeper than `max_grade`; `deck.misses` W never crosses what `over` names |
| Steps | `step.point` E `at` is not a point; `step.after` E `after` names no layer; `step.failed` E command ended with an error |
| | `step.no_output` E no `layer.json` written; `step.ran` I ran, or was reused |
| Reach lint | `reach.nothing_expected` I no `[[expect]]` rows; `reach.no_profile` E profile lacks `max_slope_deg`; `reach.no_start` E start is not standable |
| | `reach.unknown` E target names nothing; `reach.cut_off` E expected reachable, no route; `reach.not_sealed` E expected sealed, has a route |

A step's own codes are written `<step id>:<code>` and are not in the registry.

## 7. Step contract and SDK

| Name | Meaning |
|---|---|
| `VIA_STEP` | Environment variable: path of the request file. |
| request `contract`, `point`, `id` | The string `via-step-0`; `generate` or `lint`; the step's id. |
| request `params`, `frame` | The step's table, unread; `{ extent, resolution, nx, ny, seed }`. |
| request `build`, `out` | A build folder to read, or null; the folder for the answer. |
| answer `layer.json` | Generate: a document that becomes a layer. Required. |
| answer `<name>.f32` | Rasters that the layer's fields name with `file`. |
| answer `diagnostics.json` | List of `{ severity, code, subject, message, where?, hint? }`. |
| `gen/<id>/` | Beside the world: kept outputs of a generate step, with its input hash. |
| `viasdk.step()`, `viasdk.CONTRACT` | Reads the request and returns a `Step`; the contract string. |
| `Step.point`, `.id`, `.params`, `.frame`, `.build`, `.out` | The request's fields; `build` is a `Build` or None. |
| `Step.write_field(name, values, **spec)` | Writes a raster and declares the field in the layer. |
| `Step.add(table, row)`, `Step.set(table, key, value)` | Appends a row to a list; sets an entry of a table. |
| `Step.error(...)`, `Step.warn(code, subject, message, where, hint)` | Records a diagnostic. There is no `info`. |
| `Step.done()` | Writes `layer.json` (generate) and `diagnostics.json`. |
| `Frame.x0`, `.y0`, `.x1`, `.y1`, `.resolution`, `.nx`, `.ny`, `.seed`, `.shape` | The grid; `shape` is `(ny, nx)`. |
| `Frame.xy()`, `Frame.index(x, y)` | World x and y of every post; nearest post as `(row, column)`. |
| `Build.folder`, `.frame`, `.doc`, `.features`, `.explicit` | Folder, frame, parsed `world.json`, features by id, explicit instances. |
| `Build.fields()`, `Build.field(name)` | Raster names; one raster as `[row, column]`, row 0 south. |
| `Build.instances(asset=None)` | Every placed instance as `{ asset, pos, yaw }`. |

## 8. Command line and outputs

`python -m worldc WORLD.toml [--pack PACK.toml] [--overlay FILE]... [--variant NAME]... [--out NAME] [--lint NAME]... [--map]`

| Flag or file | Meaning |
|---|---|
| `--pack` | Use this pack instead of `[world].pack`. |
| `--overlay`, `--variant` | Merge a file, or `[variant.NAME]`, over the document; repeatable, in order. |
| `--out` | The build's name; default from world folder, overlays, variants. |
| `--lint`, `--map` | Run a lint (`reach` only); also write the map folder. |
| exit status | 0 no errors; 1 an error or a stop; 2 bad arguments. |
| `world.json` | The manifest: frame, fields, ground, assets, instances, features, meshes, bookmarks. |
| `height.f32`, `<field>.f32`, `bed.f32`, `hole.f32` | Rasters: `height`, each of `export.fields`, and the two compiler fields. |
| `ground_0.rgba8`, `ground_1.rgba8` | Weights of ground materials 0 to 3 and 4 to 7. |
| `vol_<id>.verts.f32`, `.normals.f32`, `.faces.u32`, `.light.f32` | One meshed volume. |
| `mesh_<id>.verts.f32`, `.normals.f32`, `.faces.u32` | One generated mesh. |
| mesh ids | `stairs`, `stairs_ramp`, `walls_<level>`, `floors_<level>`, `roof_<level>`, `deck_<feature>`. |
| `plan_<level>.png`, `layer_<id>.json` | Floor plan per level; instances and features of one layer. |
| `budgets.json`, `report.json`, `diagnostics.json` | Chunk counts; water body measurements; every diagnostic. |
| `minimap.png`, `reach.png` | Overview picture; with `--lint reach`, walkable area and gaps. |
| `map/class_index.png`, `class.png`, `base.png`, `map.json` | With `--map`: class per post, coloured, shaded; icons, labels, tiles. |
| map classes | `deep`, `shallow`, `sand`, `grass`, `forest`, `rock`, `paving`, `road`, `building`; keyed on fixed material names. |
| `check.py`, `maptool.py` | Compare test builds with `golden.json`; `measure`, `compose`, `changed` over a map folder. |
| viewer | `godot --path viewer -- --build=DIR [--out=DIR]` with one of `--shots`, `--bench`, `--walktest`, `--mapshot`. |

The build is written to `build/NAME` beside the package. Old files there are not removed. Nothing is written when
`field.required` fires. `world.json` holds absolute paths to pack files.

## 9. Disagreements found

| Where | Spec says | Code does |
|---|---|---|
| 10.2 `check.py` | Six cases. | Seven: `relief` is added. |
| 12.1 field `file` | Relative to the document that names it. | Anchored only in the world file and layer files; elsewhere it stays as written. |
| 12.7 `field.image_coarse` | No condition given. | Fires only when the field declares `unit`. |
| 7 `frame.unknown` | `on`, `look`, `joins`. | Also kit run `along` that is not a path, and deck `over`, `from_on`, `to_on` naming no feature. |
| 12.5 `from_on`, `to_on` | A way that is not a deck leaves the end on the ground. | True for an existing feature; an unknown id is an error. |
| 12.5 kind `deck` | Names four keys, no defaults. | `surface` "concrete", `thickness` 1.0, `parapet` 1.0, `pier_every` 0 (no piers). |
| 12.5 deck `over` | A path is crossed within its half width plus 2 m. | The crossed path's `width` defaults to 6 here; `joins` uses 2. |
| 3.10 kit run `faces` | Any value. | Overwritten with `left` when `along` is given. |
| 3.15 layer `file` | A TOML file. | A `.json` suffix is read as JSON. |
| 12.2 steps | Sets `VIA_STEP`. | Also puts `sdk/` on `PYTHONPATH`; a step severity outside the three becomes a warning. |
| 9.2 `assets` | Lists the common keys. | Also writes `roof` when the pack sets it. |
| 3.5 kinds | "The compiler has no built-in kind"; four files. | It knows `bookmark`, `poi`, default `pad` by name (11 admits it); `ways.toml` is a fifth file. |
| `kinds/terrain.toml` header | Lists the `pad` attributes. | Omits `trodden`, which the kind defines and uses. |
