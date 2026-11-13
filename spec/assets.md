# Assets and packs (v0)

A world names an asset by id, or a group of assets by a place in a taxonomy; a pack binds each id to
a set of files and to the facts the build needs about it. This chapter says what a pack holds, how
it is found, which facts are measured and which are declared, how a changed fact is reported
against every instance it touches, and how a pack is checked on its own.

Words are from `vocabulary.md`; section 1.1 lists the words this chapter adds there. Frames, axes,
the lattice and rounding are in `numbers.md`; ids and place names in `identity.md`; where an
instance stands, overlap and relief in `rules.md`; heights in `surfaces.md`; the change report in
`identity.md` and `graph.md`; files handed to the loader in `build.md`; the shape of a diagnostic in
`diagnostics.md`.

**What arrives when.** Every section is marked. M1 builds `ford` on primitive packs only (plan,
milestone one). Asset facts measured from model files arrive in M2 (plan, order notes).

| Milestone | Sections |
|---|---|
| M1 | 1 (the rule, finding a pack), 2 (a pack; the `primitive` and `engine` sources), 5 (taxonomy), 6 (primitives), 7 (pack swaps in the change report), 8 (`via pack check`, `via pack list`) |
| M2 | 3 (measuring a model, lower-detail copies), 4 (the facts file, `via pack accept`), 9 (stand-ins) |
| M5 | 10 (forward pointers) |

In M1, `file` and `detail` on an asset are `doc.not_yet`, naming M2.

## 1. The rule (M1)

- A world never names a file for an asset (principle 6). It writes `pack` once, then names assets by
  id (`asset = "cottage"`) or by selection (`assets = ["@plant/tree"]`).
- A pack is a document a user writes, flat TOML keyed by id (R1), with one exception: taxa are
  keyed by path (section 5). A pack may name files; a world may not.
- One pack per world in v0. Whether an import brings its own pack is (open) in `documents.md`, M4.
- Swapping the pack changes no world text.

**What a swap may change (R30).** Swapping the pack swaps the look and the facts the look carries
(`bounds`, `footprint`, `origin`, `sink`, `max_relief`, mounts, `solidity`). What follows from those
facts settles and is reported: in `ford`, a larger house grows its `ground` footprint feature
(section 7). Nothing the author wrote changes. A field's `file` is a declared outside input
(principle 7, `fields.md` 4), not an asset, so this chapter does not govern it.

### 1.1 Words to add to `vocabulary.md`

`vocabulary.md` lists only `root`, `[asset.<id>]`, its mounts and `[material.<name>]` for a pack. Each
word below is added there, with the Takes and Meaning this chapter gives, before M1 code is written:

- top level: `[defaults]`, `[taxon."<path>"]` (keyed by path: an exception to R1, recorded there);
- on an asset: `engine`, `primitive`, `size`, `detail`, `front`, `up`, `pivot`, `solidity`, `sink`,
  `max_relief`, `colour`, `params`; `triangles` on an engine-made asset;
- on a taxon: `defaults`;
- on a material: `colour`, `repeat`, `params`;
- terms: facts file, measured fact, declared fact, stand-in;
- commands: `via pack check`, `via pack list`, `via pack accept`;
- the `file` row: in a pack, `file` and `detail` resolve against `root`, not the pack file;
- the `pack` row: the lookup rule of 1.2;
- a key spelling policy: keys use British spelling, as prose does (`colour`, `base_centre`).
  `build.md`'s `color` follows.

`documents.md`'s pack row then lists `root`, `[defaults]`, `[taxon]`, `[asset]` and `[material]`,
and points here for how a pack is found.

### 1.2 Finding a pack

This chapter owns the rule; `documents.md` and `vocabulary.md` point here.

- A `pack` string holding a `/` or ending in `.toml` is a path, relative to the world file.
  `documents.md` 7.2 applies: `doc.path_form`, `doc.path_case`.
- Any other string is a name. It is an id, and is looked up as `packs/<name>.toml` in the four
  roots of `documents.md` 4, in its order (beside, each `--lib`, each `VIA_LIB` entry, shipped).
  First match wins.
- A name found in more than one root is `pack.shadowed` (info), naming the one used. A path or name
  found nowhere is `pack.not_found` (error), naming the roots searched and the nearest name found.
  Both name roots by place, never by path, as `documents.md` 4 does: `beside`, `--lib <n>`,
  `VIA_LIB <n>`, `shipped` (R26).
- `via pack check`, `via pack list` and `via pack accept` take the same string, with paths
  relative to the working directory.

`pack = "primitive"` (the `ford` world) finds `lib/packs/primitive.toml`: the shipped root in a release, the `VIA_LIB` root in development and conformance (`documents.md` 4.1).

## 2. A pack (M1)

### 2.1 Top-level keys and tables

| Key or table | Takes | Meaning |
|---|---|---|
| `root` | string | The folder every file of the pack is found under, relative to the pack file. Default: the pack file's folder. It may climb with `..`; an absolute path is `doc.path_form`. A `root` that does not exist is `pack.missing_file`. |
| `[defaults]` | table | Values for declared facts that every asset takes unless it, or a taxon above it, writes them (2.5). |
| `[taxon."<path>"]` | table | One taxon: an entry of the pack's taxonomy (section 5). Key: `defaults`. |
| `[asset.<id>]` | table | One asset. Keys: 2.2. |
| `[asset.<id>.mount.<name>]` | table | A named place on the asset (2.7). |
| `[material.<name>]` | table | A material that a `paint` names (2.6). |

Asset ids follow `identity.md`: `[a-z][a-z0-9_]*`, with no `.`. Asset ids form the pack's own
namespace; they never collide with the world's ids. A dotted asset id (`house.low`) is
`identity.bad_id`: `<instance>.<name>` already means a mount or a set item (`identity.md`).

### 2.2 Sources and the keys each takes

An asset has exactly one source, decided by one marker key written on the asset's own table.
Inherited values never decide a source.

| Source | Marker | Facts |
|---|---|---|
| model (M2) | `file` | Measured from the file (section 3). |
| primitive | `primitive` | Fixed by this chapter (section 6). Needs no file. |
| engine-made | `engine = true` | Declared. The loader makes the thing (an engine-side generator), from `params`. |

No marker, or more than one, is `asset.source`. `engine = false` is the same as no `engine`.

Keys allowed on an asset's own table, per source. "Measured" and "fixed" keys are facts the build
works out; writing one is `asset.declared_measured`. Any key not in this table is `doc.unknown_key`.

| Key | Takes | model | primitive | engine-made |
|---|---|---|---|---|
| `file` | string | required | `asset.source` | `asset.source` |
| `detail` | list of strings | optional | `doc.unknown_key` | `doc.unknown_key` |
| `primitive` | string | `asset.source` | required | `asset.source` |
| `engine` | bool | `asset.source` | `asset.source` | required, `true` |
| `size` | 3 lengths | measured | required | required |
| `triangles` | integer | measured | fixed | required |
| `front`, `up`, `pivot` | string | optional | fixed | fixed |
| `footprint` | list of points | optional | optional | optional |
| `solidity`, `sink`, `max_relief`, `colour` | 2.4 | optional | optional | optional |
| `taxon`, `tags` | section 5 | optional | optional | optional |
| `params` | table | optional | optional | optional |

- `file`: a glTF 2.0 model, binary (`.glb`) or text (`.gltf`), relative to `root`.
- `detail`: lower-detail copies of `file`, finest first, each relative to `root`. Naming `file`
  again, or one path twice, is `asset.detail_duplicate`. The distances at which the engine switches
  between copies are the engine's (principle 12).
- `primitive`: one of section 6's shape words. Any other is `asset.unknown_primitive`, with the
  nearest.
- `size`: `[x, y, z]` metres in the asset frame, each read as a lattice value and greater than 0
  (`pack.range`).
- `triangles`: an integer, 0 or greater (`pack.range`). On an engine-made asset it is what budgets
  count (`build.md`).
- `params`: a table handed to the loader unread, on any asset. Same meaning as on a step: data the
  core passes through without reading. What an engine makes for collision is a loader convention
  inside `params` (for example `params = { collision = "hull" }`); the core has no collision key
  (principle 12: gameplay is the engine's).

**Defaults reach only what a source declares.** A value from `[defaults]` or a taxon's `defaults`
(2.5) applies to an asset only for a key its source takes as optional. It is silently not applied
otherwise: a pack-wide `pivot = "base_centre"` reaches every model asset and skips every primitive
and engine-made one. Only a value on the asset's own table can be `asset.declared_measured`.

**Bound files (M2).** The set of files an asset binds is `file`, every copy in `detail`, and every
buffer and image those models reference by URI, found by reading them. Every one is a declared input
of the graph (principle 7, `graph.md` `input:` nodes). URIs are percent-decoded, then resolved
against the model file's folder; `documents.md` 7.2 applies to the result (`doc.path_case`,
`doc.path_form`). A URI that leaves `root` (an absolute path, a `..` that climbs out of `root`, a
scheme other than `data:`) is `pack.outside_root`. A symbolic link on the path below `root` is
`pack.outside_root` too: the bytes in the folder are the input, on every system. Embedded `data:`
URIs are part of the model file.

A file kept beside a model that the model does not reference (the spike's white buildings: a colour
texture never bound) is not part of the asset. v0 has no key to bind extra files to a model: the fix
belongs in the model, and the build cannot know what an unreferenced file is for. `asset.untextured`
(info, 3.1) shows the symptom. An engine-made asset's own files are (open), below.

### 2.3 The asset frame

Every fact, footprint and mount of an asset is written in its asset frame (ours):

- origin at the asset's **pivot** (2.4);
- `x` to the asset's right, `y` toward its front, `z` up, metres;
- right-handed, like the world frame. An instance with `face = 0` lays the asset frame on the world
  frame's axes: front north, right east. How `face` turns it, and how `scale` and the instance
  frame nest, is `numbers.md` 5.2.

A model's own axes map to the asset frame through two declared facts, `front` and `up`. Each names a
model axis with its sign: `"+x"`, `"-x"`, `"+y"`, `"-y"`, `"+z"`, `"-z"`. The defaults are glTF's
convention (glTF 2.0 specification, "Coordinate System and Units": +Y up, the front of an asset faces
+Z, metres): `front = "+z"`, `up = "+y"`. The asset frame's `y` is the model's `front`, its `z` is
the model's `up`, and its `x` is `y × z`. With the defaults, a model point `(mx, my, mz)` is
`(-mx, mz, my)` in the asset frame: a glTF asset's +X is its own left. The mapping is always a
rotation, never a mirror. `front` and `up` on one axis, or a value outside the six, is
`pack.bad_axes`. Only quarter turns are expressible; a model turned by another angle is fixed in the
model.

Model units are metres, as glTF requires. A model made at another scale is (open): decided by the
first generated pack in M2, between a declared factor on the asset and a fit to a declared height.

### 2.4 Facts

A fact is either measured (the build works it out from the files, or this chapter fixes it; the pack
cannot write it) or declared (the pack writes it, or it takes its default). "Measured, not asked": a
model asset needs only `file` to build (TODO, "assets that arrive fast and uneven"). Every length is
held on the integer millimetre lattice (R24).

| Fact | Measured or declared | Takes | Meaning |
|---|---|---|---|
| `bounds` | measured (model, 3.1); from `size` (primitive, engine-made) | 6 lengths | `[x0, y0, z0, x1, y1, z1]`, the box holding every vertex, in the asset frame. |
| `triangles` | measured (model, 3.2); fixed (primitive, section 6); declared (engine-made) | integer | Triangles in the main model, or of the primitive or engine-made asset. Budgets count this number. |
| `detail_triangles` | measured (model with `detail`) | list of integers | Triangles in each `detail` copy, in `detail`'s order. |
| `origin` | measured (model); `[0, 0, 0]` (primitive, engine-made) | 3 lengths | Where the model's own origin lies in the asset frame: model origin minus pivot, `[x, y, z]`. |
| `footprint` | measured, or declared to override | list of points | The ground the asset claims, in plan. Measured: the rectangle of `bounds` in plan, counter-clockwise from `[x0, y0]` (section 6 for round primitives). Declare it when the claim is not the box (a trunk under a canopy, the ground a kit piece owns). |
| `front`, `up` | declared (model) | string | Model axes, 2.3. Default `"+z"`, `"+y"`. |
| `pivot` | declared (model) | string | The rule that puts the pivot: `"base_centre"` (default), `"base_corner"`, `"origin"`. |
| `solidity` | declared | string | `"solid"` (default) or `"none"`. See below. |
| `sink` | declared | number | How far an instance is lowered below its placing point, metres. Default 0. May be negative. |
| `max_relief` | declared | number | The largest difference in surface height under the footprint the asset tolerates, metres, 0 or greater (`pack.range`). No default: without it, nothing is checked. |
| `colour` | declared | 3 numbers | Linear red, green, blue, each 0 to 1 (`pack.range`). Default `[0.8, 0.8, 0.8]` (ours). The colour a primitive or stand-in is drawn in; `tint` (`rules.md`) multiplies it. A model draws in its own materials, which `tint` multiplies. |

An enumerated value outside its list (`pivot`, `solidity`) is `doc.type`, naming the allowed words
and the nearest.

**Bounds of a box from `size`.** For a primitive or engine-made asset, with `size = [X, Y, Z]` in
millimetres, `bounds` is `[-floor(X/2), -floor(Y/2), 0, X - floor(X/2), Y - floor(Y/2), Z]`, floor
toward minus infinity. The pivot is `base_centre` by the rule below. For `size = [1.001, 2, 3]`,
`bounds` is `[-0.5, -1, 0, 0.501, 1, 3]`.

**Pivot (M2 for models).** The pivot is the point of the asset that is put at an instance's placing
point. The rule is declared once per kit, as studios do (Bethesda, GDC 2016: "pick a pivot, and
stick with it"), through `[defaults]` or a taxon's `defaults`, and checked against the model. With
`[x0, y0, z0, x1, y1, z1]` the model's bounds in the asset frame's axes, before the pivot is placed,
in millimetres:

| `pivot` | The pivot is |
|---|---|
| `base_centre` | `[floor((x0 + x1) / 2), floor((y0 + y1) / 2), z0]`: the centre of the plan, at the lowest point. |
| `base_corner` | `[x0, y0, z0]`: left, back, lowest. |
| `origin` | `[0, 0, 0]`: the model's own origin, wherever it is. |

The build moves the model so the pivot is the asset frame's origin. `origin` is then the model's
own origin in the asset frame, that is model origin minus pivot; `bounds` is the moved box. Example:
a model whose origin is 2 m above its base, centred in plan, under `base_centre`, has
`origin = [0, 0, 2]`: its origin stands 2 m above the placing point. A model whose `origin` is not
`[0, 0, 0]` gets `asset.pivot_moved` (info): odd pivots are the normal case and are fixed, not
refused, but never silently. Under `pivot = "origin"`, `origin` is `[0, 0, 0]` by definition, and a
model re-exported with its origin elsewhere moves every instance; section 4 reports it.

**Solidity.** What the core's own checks take the asset to be. `solid`: its footprint takes part in
the overlap check and scatters give way to it (`rules.md` 7.5, 8.1). `none`: claims nothing (grass,
litter). `passable` (a gate, an arcade: claims ground but a walk passes through) is reserved for M5
(section 10); only the walk lint, a library's, would read it. In M1 and M2 it is `doc.not_yet`.

**Spacing between assets.** v0 has no asset fact for a distance kept around a footprint. Scatters
give way by solid footprint overlap (`rules.md` 7.5), and a clearing is a `keep_out` field that a
kind writes (`rules.md` 2.1, 8.3). The spike's fixed 1 m distance is gone. Whether scattered
instances keep a distance from each other or from hand placements is `rules.md` 15's (open), decided
by `glade` with real assets in M2.

**Scale.** An instance's `scale` multiplies `bounds`, `footprint`, `origin`, `sink`, `max_relief` and
mounts' `at`, each rounded to the lattice by `numbers.md` 3.1's rule. `max_relief` scales because a
uniform scale also scales the base that hides a slope (a plinth twice as deep).

**Sink and relief.** `surfaces.md` 4.2 finds the height under an instance and places the pivot there;
`sink` then lowers it. `surfaces.md` 2.3 measures relief with `relief()`; `rules.md` 8.2 owns the
comparison with `max_relief` and `place.relief`. This chapter only defines `max_relief`.

**Footprint rules.** A declared footprint is at least three points, counter-clockwise, simple
(`geometry.md`). Collinear points are allowed. A repeated point, including a closing point equal to
the first, is `asset.bad_footprint`; so is a clockwise one, which is refused, not reversed (the hint
says to reverse it). A measured footprint of zero area (a flat card: zero extent in plan `x` or `y`)
becomes a point footprint at the pivot, which `rules.md` handles, with `asset.point_footprint`
(info).

### 2.5 Declared facts: where a value comes from

For each declared fact, first found wins:

1. the asset's own table;
2. the `defaults` of its taxon, then of each taxon above it, nearest first;
3. the pack's `[defaults]`;
4. the default in 2.4.

`defaults` may hold `front`, `up`, `pivot`, `solidity`, `sink`, `max_relief`, `colour` and `params`.
Any other key there is `doc.unknown_key`, naming the nearest allowed one. This is the
pack-lookup rule `documents.md` 6.2 defers to: the nearest `params` default is taken whole, never
merged with a farther one. A value reaches an asset
only if its source takes that key as optional (2.2).

### 2.6 Materials

`[material.<name>]` binds a material that a `paint` names (`fields.md`) to what the loader paints.

| Key | Takes | Meaning |
|---|---|---|
| `colour` | 3 numbers | Linear red, green, blue, each 0 to 1 (`pack.range`). Required: the material always draws. |
| `file` | string | An image, relative to `root`. Optional. A bound file of the pack (2.2's rules apply). Checked from M1: a missing one is `pack.missing_file`, for every material in `via pack check` and for each painted one in a build. |
| `repeat` | number | Metres one repeat of `file` covers, greater than 0 (`pack.range`). Required with `file`, and only with it: otherwise `pack.material_repeat`. |
| `params` | table | Handed to the loader unread. |

A `paint` naming a material the pack lacks is `pack.unknown_material`, with the nearest. No more
keys are owed: one `material` field per world (`fields.md` 2.1).

### 2.7 Mounts on an asset

`[asset.<id>.mount.<name>]` is a named place on the asset. An instance's mount is named
`<instance>.<name>` (`identity.md`). This chapter owns the asset mount; `surfaces.md` 3.1 owns the
kind mount.

| Key | Takes | Meaning |
|---|---|---|
| `at` | point | Plan position in the asset frame, lattice metres. Required. It may lie outside `bounds` (a doorstep in front of a door). Its height is never written: it is the surface's (`surfaces.md` 3.1). |
| `face` | number | A bearing in the asset frame, 0 = the asset's front, clockwise (`numbers.md` 2). Default 0. |

- `t` and `offset` are not taken: an asset has no path. Each is `doc.unknown_key`.
- `connector` on an asset mount is `doc.not_yet` (M5, section 10). On a kind mount it is
  `surfaces.md`'s.
- A mount named `start`, `end` or `ground` is `identity.reserved`.
- Mounts are pack-wide checks (8.3) and facts of the facts file (section 4).

## 3. Measuring a model (M2)

### 3.1 What is read

- **Scene.** The scene `scene` names, or scene 0 if `scene` is absent. A file with no scenes, or a
  scene that reaches no triangles, is `asset.empty`.
- **glTF nodes.** Every glTF node under the scene, through the node transforms. A glTF node's
  matrix is its `matrix`, or `T * R * S` from its `translation`, `rotation` and `scale`. The
  transform of a vertex is `M_root * ... * M_parent * M_node * v`, column vectors, composed root to leaf in f64 through
  the one maths module (`numbers.md` 8).
- **Positions.** Every vertex position of every primitive reached, from its `POSITION` accessor,
  with sparse accessors applied. Normalised integer positions (`KHR_mesh_quantization`) are read as
  that extension says.
- **Skins.** A skinned mesh is measured from its accessor positions under its own glTF node's
  transform, as if unskinned; joints and inverse bind matrices are not read. glTF ignores a skinned glTF node's
  transform when drawing, so a model whose bind pose differs from its glTF node's transform measures
  differently from how it draws. This rule is ours, chosen because it reads no animation data.
- **Morph targets.** Not read; a mesh's default `weights` are ignored. Animations are not read.
- **To the asset frame.** Each transformed position is mapped by `front` and `up` (2.3, exact), then
  rounded to the nearest millimetre, ties to even (`numbers.md` 3.1). `bounds` before the pivot is
  the minimum and maximum of the rounded values. Rounding each value to nearest, not the box
  outward, keeps float noise in a re-export (1.0000001 m) from changing a fact.
- **Extensions.** Read: `KHR_mesh_quantization`. Tolerated, because they do not change positions:
  `KHR_texture_transform`, `KHR_texture_basisu`, `EXT_texture_webp`, `KHR_lights_punctual`, and every
  extension whose name starts `KHR_materials_`. Any other extension in `extensionsRequired`
  (compressed geometry among them: `KHR_draco_mesh_compression`, `EXT_meshopt_compression`) is
  `pack.bad_model`, naming it. `EXT_mesh_gpu_instancing` changes geometry and v0 does not read it:
  it is `pack.bad_model` whether required or only used. Any other extension only in
  `extensionsUsed` is ignored.
- **Faults.** A file that does not parse as glTF 2.0, or holds a non-finite number, is
  `pack.bad_model`.
- **Materials.** A model none of whose materials has a base colour texture is `asset.untextured`
  (info). Many low-poly kits colour by factor alone, so it is not a warning.
- **Cache.** Measuring reads the asset's bound files and nothing else. Its result depends only on
  their bytes and on the declared `front`, `up` and `pivot`, so it is cached by their digest
  (`graph.md`).

A fault inside a model has the location form "file" (`diagnostics.md` 1.2), with `data.pointer` a
JSON Pointer into the glTF JSON (RFC 6901), for example `/meshes/3/primitives/0`. For a `.glb` it
points into the JSON chunk.

### 3.2 Counting triangles

Per primitive, `n` being its index count, or its vertex count if it has no indices: mode 4
(triangles) `n / 3`; modes 5 and 6 (strip, fan) `n - 2`; points and lines 0. Mode 4 with `n` not a
multiple of 3, or mode 5 or 6 with `n` below 3, is `pack.bad_model`, pointing at the primitive.
`triangles` is the sum over every primitive reached in 3.1, once per glTF node that uses the mesh.

### 3.3 Lower-detail copies

Each `detail` copy is measured as in 3.1 with the asset's own `front` and `up`, and moved by the main
model's pivot correction (not its own). It reports its triangles in `detail_triangles`. Every other
fact is the main model's.

- A copy reaching outside the main model's `bounds` in plan is `asset.detail_bounds` (warning).
- The asset's pivot rule applied to the copy's own bounds gives a point. Where it differs from the
  main model's pivot by more than 1 mm on any axis, the copy is `asset.detail_pivot` (warning): it
  was likely exported with another origin and would shift when the engine switches to it. The 1 mm
  limit is ours.

## 4. The facts file (M2)

The facts that decide where and how an instance stands are kept, per model asset, in the pack's
**facts file**. Studios report that a revision of a placed piece's contract is the costliest change
they make, and no source shows tooling for it (`reports/Modular city asset contracts.md`, sections 1
and 5.6); this mechanism is ours.

- **Where.** The pack file's path with `.toml` replaced by `.facts.toml`: `packs/vale.toml` has
  `packs/vale.facts.toml`.
- **Which assets.** Model assets only. A primitive's or engine-made asset's facts are all in the pack
  text, so the text diff shows every change. A pack with no model asset needs no facts file and
  raises no `pack.unaccepted`; the shipped primitive pack has none.
- **Who writes it.** `via pack accept PACK`, never by hand and never by `via build` (principle 1: a
  build is thrown away). The word is not `lock`, which names a hand edit (`identity.md`, principle 9).
- **An input.** The facts file is a text source of the graph, a `source:<path>` node (`graph.md` 1).
  When it does not exist, the node's value is the fixed digest of "absent" (`graph.md` fixes the
  bytes), so creating or deleting it re-runs what read it. One node per used model asset,
  `facts:<asset id>`, reads that source, the asset's measured facts and its pack statement, and
  holds the comparison below and its diagnostics. After `via pack accept`, an incremental build
  re-runs only those nodes.

### 4.1 What it holds, byte for byte

Two correct implementations write the same bytes.

- First line: `# Written by via pack accept. Do not edit.`, then one blank line.
- One table `[asset.<id>]` per model asset, sorted by id bytewise, separated by one blank line.
- Keys in this order, each `key = value` on one line, absent facts omitted: `bounds`, `origin`,
  `footprint`, `triangles`, `detail_triangles`, `front`, `up`, `pivot`, `solidity`, `sink`,
  `max_relief`, `colour`, `taxon`, `tags`, `mounts`, `params`, `files`.
- Lattice values are written in `numbers.md` 3.4's exact metre form; `colour` components as
  `numbers.md` 3.4 writes a double; integers as decimal digits. Lists as `[a, b]` with `, ` between
  items; strings as TOML basic strings.
- `mounts` is an inline table by mount name, sorted: `{ door = { at = [0, 3.1], face = 0 } }`.
  Absent when the asset has none.
- `params` is the BLAKE3 digest (`build.md` 3.3) of the canonical JSON (`documents.md` 8) of the
  resolved `params`. Absent when there are none.
- `files` is an inline table from each bound file's path (relative to `root`, `/` separators) to its
  digest, sorted by path.
- LF line ends, and a final newline.

```toml
# Written by via pack accept. Do not edit.

[asset.cottage]
bounds = [-4.2, -3.1, 0, 4.2, 3.1, 6.35]
origin = [0, 0, 0]
footprint = [[-4.2, -3.1], [4.2, -3.1], [4.2, 3.1], [-4.2, 3.1]]
triangles = 1840
front = "-y"
up = "+z"
pivot = "base_centre"
solidity = "solid"
sink = 0
max_relief = 0.6
colour = [0.8, 0.8, 0.8]
taxon = "building/house"
tags = []
mounts = { door = { at = [0, 3.1], face = 0 } }
files = { "cottage.glb" = "9f2c…" }
```

A test vector file of this form is added under `spec/` with the M2 conformance cases.

### 4.2 What a build compares

Every build compares, for each model asset it uses, the facts it measured and resolved with the
facts file.

| What differs | Diagnostic | What the build does |
|---|---|---|
| No facts file, and the pack has a model asset | `pack.unaccepted` (info), once | Builds. |
| A used asset the facts file lacks | `asset.unaccepted` (info) | Builds. |
| Any of `bounds`, `origin`, `footprint`, `front`, `up`, `pivot`, `solidity`, `sink`, `max_relief`, `mounts` | `asset.revised` (warning), one per asset | Builds with the new facts. Lists each fact old and new, then every instance of the asset in this build. |
| `taxon` or `tags`, and no fact of the row above | `asset.refiled` (info) | Builds. What a selection now picks shows in the change report as swaps. |
| Only `files`, `triangles`, `detail_triangles`, `colour` or `params` | `asset.remodelled` (info), with old and new triangles | Builds. Nothing moves: a geometry-only revision passes. |
| An unreadable or malformed facts file | `pack.bad_facts` (error) | Stops. Hint: delete it and run `via pack accept`. |

- The build uses the current files and text for every result. Outside stand-ins (section 9), the
  facts file decides no instance, field or mesh; it decides only which of the diagnostics above are
  raised.
- `asset.revised` names every instance: hand-placed by id, produced by the place name `identity.md`
  gives it, a set item by `<instance>.<name>`. It counts them in its message and lists all of them
  in its data, never a sample.
- A declared fact edited in the pack text is reported the same way: the text diff shows the line,
  the build shows what it touched.
- The warning repeats on every build until someone runs `via pack accept`. `via build --fail-on
  asset.revised` makes it fail an unattended check (`build.md` 14).

### 4.3 `via pack accept PACK`

- Measures and resolves every model asset of the pack, used or not, and writes the facts file.
- Refuses when the pack has any error, including an unreadable model: `pack.accept_refused` (error,
  exit 1), listing the errors' codes. Nothing is written. A partial facts file never exists.
- Writes atomically: a temporary file in the same folder, then a rename over the old one.
- Its bytes depend only on the pack and its files. Accepting twice writes the same bytes.
- Exit 0 on success; it prints the ids whose entries changed.

## 5. Taxonomy (M1)

A taxonomy lets a world ask for a role ("a tree") and the pack answer it, so a pack swap changes the
answer without touching the world. The tree is the user's: the core gives no taxon any meaning and
ships none. A starter tree for our examples lives in `lib/packs/` as data. The tree is the pack's and
classifies assets only. A field's `taxon` is not checked against it: the core gives it no meaning
(`fields.md` 1).

- A taxon is declared as `[taxon."<path>"]`. A path is ids joined by `/`: `"plant/tree"`. A segment
  that is not an id, an empty segment (`//`) or a leading or trailing `/` is `identity.bad_id`. Declaring
  a taxon declares every taxon above it. Keys: `defaults` (2.5), or none.
- An asset is filed at one taxon: `taxon = "plant/tree"`. A taxon the pack does not declare is
  `pack.unknown_taxon`, naming the nearest declared path. An asset with no `taxon` is filed nowhere
  and is reachable by id only.
- `tags` are marks that cut across the tree (`tags = ["winter", "landmark"]`). Each is an id
  (`identity.bad_id`). The core gives no tag a meaning; packs, lints and steps read them.
- A selection `@<path>` stands, wherever a list of assets is taken (`assets` on a scatter), for every
  asset filed at that taxon or below it, as one list ordered by asset id (`vocabulary.md` 2.2). How a
  selection is weighted against other choices is `rules.md` 7.4. Whether `asset` on an instance or run
  may take a selection (the spike's kit run did) is (open), below.
- A selection naming a taxon the pack does not declare is `pack.unknown_taxon`, with the nearest
  declared taxon as hint. A selection naming a declared taxon with no asset at or below it is
  `asset.empty_selection`, with the nearest taxon that has assets as hint. `rules.md` 7.4's
  "unknown-asset error" for a path with no match is `asset.empty_selection`.
- Selection by tag in a world is not in v0: (open), decided by the first case that needs it (a biome
  or a season swap).
- The SDK sees the pack, its tree, tags and resolved facts, read-only (R19b: no private door; TODO,
  "a step cannot ask for the pack").

## 6. Primitives and the primitive pack (M1)

A primitive is an asset with no file. Its shape is one of a closed list, ours. Its `bounds` come
from `size` (2.4), its pivot is `base_centre`, its front is `+y`, its `origin` is `[0, 0, 0]`. The
mesh is made by whoever draws it (`build.md`); the triangle counts and footprints below are fixed,
and bind that maker.

| `primitive` | Shape in `size = [x, y, z]` | `triangles` | Footprint when not declared |
|---|---|---|---|
| `box` | A box filling `bounds`. | 12 | The rectangle of `bounds`. |
| `prism` | A triangular prism: the box's lower face, its ridge along `x` at height `z`, over the middle of `y`. | 8 | The rectangle. |
| `cylinder` | An upright cylinder, an ellipse of axes `x` and `y` in plan, 16 sides. | 60 | The 16-sided polygon of its sides. |
| `cone` | An upright cone, same base. | 30 | As `cylinder`. |
| `sphere` | An ellipsoid in the box, 16 around and 8 up. | 224 | As `cylinder`. |

A vertex of a 16-sided ring lies at `k * 22.5` degrees counter-clockwise from the `+x` axis,
`k = 0..15`. With `(cx, cy)` the plan centre of `bounds` and `(rx, ry)` half the plan extents, as
doubles in millimetres, the vertex is `(cx + rx * cos, cy + ry * sin)`, each rounded to the nearest
millimetre, ties to even. `cos` and `sin` are taken by `numbers.md` 2's quadrant reduction for
`m = 22500 * k` millidegrees, so the four axis vertices are exact. Primitive shape words are
geometry, not domain nouns (principle 5).

**The primitive pack.** A pack whose assets are all primitives needs no file, so it always builds
(FINDINGS, "Decisions this locks", 4). We ship one as library data, `lib/packs/primitive.toml`, with
the taxonomy and ids our examples use. A user writes their own the same way; it is Bethesda's proof
stage (GDC 2013: near-zero-detail pieces at exact module and pivot, before any art).

### 6.1 `lib/packs/primitive.toml`, exactly

The shipped file is this text, byte for byte, LF line ends and a final newline. Every conformance
case that says "the primitive pack" reads it, so its values are part of the specification: changing
one is a spec change, not a data edit. Sizes, colours and the taxonomy are ours, chosen for `ford`,
`rules.md` 11 to 13, `build.md` 15 and `fields.md`'s worked example.

```toml
# The primitive pack: every asset a primitive, no files. Shipped with via.

[taxon."building/house"]
[taxon."building/shed"]
[taxon."plant/tree"]
[taxon."plant/bush"]
[taxon."plant/fern"]
[taxon."rock"]
[taxon."prop/box"]
[taxon."prop/lamp"]
[taxon."prop/fence"]

[asset.house_low]
primitive = "box"
size      = [8, 6, 4]
taxon     = "building/house"
colour    = [0.55, 0.42, 0.33]

[asset.house_low.mount.door]
at = [0, 3]

[asset.shed_gable]
primitive = "prism"
size      = [4, 3, 3]
taxon     = "building/shed"
colour    = [0.5, 0.36, 0.25]

[asset.tree_oak]
primitive = "sphere"
size      = [5, 5, 7]
taxon     = "plant/tree"
footprint = [[-0.5, -0.5], [0.5, -0.5], [0.5, 0.5], [-0.5, 0.5]]   # the trunk, not the crown
colour    = [0.18, 0.35, 0.12]

[asset.bush_round]
primitive = "sphere"
size      = [1.5, 1.5, 1.2]
taxon     = "plant/bush"
solidity  = "none"
colour    = [0.22, 0.4, 0.15]

[asset.fern_low]
primitive = "cone"
size      = [0.8, 0.8, 0.5]
taxon     = "plant/fern"
solidity  = "none"
colour    = [0.25, 0.45, 0.18]

[asset.rock_a]
primitive = "sphere"
size      = [1.2, 1, 0.6]
taxon     = "rock"
colour    = [0.45, 0.43, 0.4]

[asset.box_small]
primitive = "box"
size      = [2, 2, 2]
taxon     = "prop/box"
colour    = [0.6, 0.5, 0.35]

[asset.lamp_post]
primitive = "cylinder"
size      = [0.2, 0.2, 3]
taxon     = "prop/lamp"
colour    = [0.15, 0.15, 0.15]

[asset.fence_post]
primitive = "box"
size      = [0.15, 0.15, 1.2]
taxon     = "prop/fence"
colour    = [0.4, 0.3, 0.2]

[material.grass]
colour = [0.2, 0.4, 0.1]

[material.rock]
colour = [0.45, 0.43, 0.4]
```

What follows from it, by sections 2.4 and 6:

| Asset | `bounds` | `triangles` | `footprint` | `solidity` | Mounts |
|---|---|---|---|---|---|
| `house_low` | `[-4, -3, 0, 4, 3, 4]` | 12 | the rectangle of `bounds` | solid | `door` at `[0, 3]`, face 0: the middle of its front wall |
| `shed_gable` | `[-2, -1.5, 0, 2, 1.5, 3]` | 8 | the rectangle | solid | none |
| `tree_oak` | `[-2.5, -2.5, 0, 2.5, 2.5, 7]` | 224 | the declared 1 m trunk square | solid | none |
| `bush_round` | `[-0.75, -0.75, 0, 0.75, 0.75, 1.2]` | 224 | the 16-sided ring | none | none |
| `fern_low` | `[-0.4, -0.4, 0, 0.4, 0.4, 0.5]` | 30 | the 16-sided ring | none | none |
| `rock_a` | `[-0.6, -0.5, 0, 0.6, 0.5, 0.6]` | 224 | the 16-sided ring | solid | none |
| `box_small` | `[-1, -1, 0, 1, 1, 2]` | 12 | the rectangle | solid | none |
| `lamp_post` | `[-0.1, -0.1, 0, 0.1, 0.1, 3]` | 60 | the 16-sided ring | solid | none |
| `fence_post` | `[-0.075, -0.075, 0, 0.075, 0.075, 1.2]` | 12 | the rectangle | solid | none |

- No asset declares `sink` (0) or `max_relief` (nothing checked), as `rules.md` 13's defaults
  assume. A case that needs `max_relief` writes its own pack.
- Every one of section 6's five shapes is used once or more, so the "Primitive pack" case checks
  every fixed triangle count.
- `plant/tree` holds one asset, so `ford`'s `@plant/tree` draws no asset choice. `fern_low` is filed
  apart from `plant/bush`, so `rules.md` 11's `["@plant/bush", { id = "fern_low", weight = 2 }]`
  names it once.
- The materials are the two `fields.md`'s worked example and `build.md` 15 paint. Neither has a
  `file`.
- The pack has no `root`, no `[defaults]`, no model asset and so no facts file.

### 6.2 The swap case's second pack

The M1 "Pack swap" case (section 7, `identity.md` case 46) swaps `ford` to
`examples/ford/packs/primitive_large.toml`. This section is that file's one source; `plan.md`'s
copy is generated from it. Its text is 6.1's with one line changed, in `[asset.house_low]`:

```toml
size      = [10, 8, 4]
```

Every other id, key and value is the same, the door mount included (it now lies 1 m inside the front
wall; the case asserts nothing about it, and keeping it keeps `size` the only differing statement).
So `house_low`'s `bounds` become `[-5, -4, 0, 5, 4, 4]` and its footprint the 10 m by 8 m rectangle.
The case's world is `ford`, its text untouched. The change is `ford`'s change file
`examples/ford/changes/bigger.toml` (`plan.md`), requested with `--change changes/bigger.toml` (`documents.md` 6.1); its
`pack` path is relative to the world file (1.2):

```toml
pack = "packs/primitive_large.toml"
```

The `pack_swap` case copies `examples/ford/` into its folder, so case 46 and `ford`'s acceptance
row read the same files.

## 7. Pack swaps and pack edits in the change report (M1)

The `ford` row "Pack swapped" needs the cottage named with old and new facts. This section meets it
for any pack, primitive or not, with or without a facts file.

This section follows `identity.md` 5, which owns the change report's entries and cause forms.

- The build directory records each used asset's resolved facts (`build.md` 7). The change report
  (`identity.md` 5) compares them with the baseline build's. For a primitive or engine-made asset,
  `size` is recorded and compared with the facts, since it is the declared statement `bounds` comes
  from.
- An instance whose asset's facts differ, and whose own origin, facing and asset do not, is
  `changed`. Its footprint feature, whose bounding box differs, is `moved`. Under `base_centre` a
  larger house keeps its plan origin, so the instance is `changed`, never `moved` in plan, by a swap alone.
  When its ground is levelled by a pad over the larger footprint, its height follows that ground,
  and a height that differs in integer millimetres is also `moved` (`identity.md` 5: position is
  plan and height).
- Every entry whose cause includes a differing asset fact carries `facts`: each differing fact of
  that asset, old and new.
- The cause is `identity.md` 5's form `asset.<asset id>.<fact>`, written with the asset id, never
  the instance id: `asset.house_low.size`, not `asset.cottage.size`. For a declared fact the file
  and line are the pack text's; for a measured fact the location is the model file. It is a cause,
  not a selector.
- When the requested changes write `pack` (`documents.md` 6.1), the cause is reached through that
  key, and the report says so: both causes are present, sorted bytewise as `build.md` 11.2 writes
  them, `asset.house_low.size` then `world.pack`.
- Everything that followed is listed too, as `identity.md` 5 lists it: the fields the footprint
  feature touched, and the scattered instances that now give way.

The M1 case: `ford` on `lib/packs/primitive.toml` (6.1), then the change `bigger` writing `pack` to
the second pack of 6.2; a build with that change against the base build. `cottage` is `changed`
only, with `facts` old and new: `size` `[8, 6, 4]` to `[10, 8, 4]`, `bounds` `[-4, -3, 0, 4, 3, 4]`
to `[-5, -4, 0, 5, 4, 4]`, and `footprint`. Footprint `cottage` is `moved`, its pad (margin 1) grown
from 10 m by 8 m to 12 m by 10 m, with the same `facts`. Both carry both causes, sorted bytewise
(`build.md` 11.2): `asset.house_low.size`, then `world.pack`.

## 8. Checking a pack on its own (M1)

### 8.1 `via pack check PACK`

Reads the pack without a world. Resolves every declared fact, checks every key, taxon, mount and
material, and from M2 measures every model asset, checks every bound file and compares with the
facts file. Prints the diagnostics of this chapter and writes nothing.

- Exit 0 when no diagnostic is an error, warnings and infos included; 1 when any is an error.
- `asset.revised` raised by a check carries an empty instance list: there is no world.
- A facts-file entry for an asset the pack no longer has is `pack.stale_facts` (info), naming the
  ids.

### 8.2 `via pack list PACK`

Prints one line per asset as JSON Lines: one canonical JSON object per line (`documents.md` 8:
keys in bytewise order, numbers in `numbers.md`'s form), sorted by id bytewise, LF line ends. Each
object holds `id`, `source`, `taxon`, `tags` and every resolved fact. It answers "what does this
pack hold" without a renderer (challenge 45).

- `--taxon @<path>` keeps the assets at or below the taxon; `--tag <tag>` keeps the assets with that
  tag. Both may be given; an asset must pass both.
- An undeclared taxon is `pack.unknown_taxon` (exit 1). A declared taxon or a tag that keeps no asset
  is `asset.empty_selection` (exit 1), with the nearest taxon or tag that has assets.
- Otherwise exit 0. Which builds use an asset is (open), decided in M2 with the build directory's
  instance list.

### 8.3 What fails every build, and what only a used asset

`via build` reads the whole pack text but measures only the assets the world uses, directly or
through a selection. The build hands the loader only the assets it uses (challenge 2's strain).

| Pack-wide: fails any build on this pack | Per used asset: fails a build only if the world uses it |
|---|---|
| Finding the pack (`pack.not_found`); TOML syntax; unknown keys and value forms (`doc.*`); ranges (`pack.range`); ids, taxon paths and tags (`identity.bad_id`); `taxon` references (`pack.unknown_taxon`); sources (`asset.source`, `asset.declared_measured`, `asset.unknown_primitive`, `asset.detail_duplicate`); axes (`pack.bad_axes`); declared footprints (`asset.bad_footprint`); mounts; materials (`pack.material_repeat`); a missing `root`; a malformed facts file (`pack.bad_facts`). | Bound files (`pack.missing_file`, `pack.outside_root`); reading and measuring a model (`pack.bad_model`, `asset.empty`); lower-detail copies; the facts comparison (section 4). |

Resolving `@<path>` needs every asset's `taxon`, so taxon faults are never per asset. `via pack
check` covers the rest.

## 9. Stand-ins (M2)

`via build --stand-ins` (`build.md` 14) builds a world whose model files are absent, for a review or
a machine without the art. Each model asset is drawn as a `box` of its accepted `bounds`, in its
accepted `colour`, keeping its accepted footprint, origin, solidity, mounts and every other fact.
Primitive and engine-made assets build as they always do. This is ours.

- In this mode the facts file is a declared input that decides the result: every placement fact of a
  model asset comes from it. It is the `source:` node of section 4, so the graph sees it.
- A stale facts file is an error in this mode: an asset the world uses that the facts file lacks, a
  declared fact in the pack text that differs from the accepted one, or a model file that is present
  and measures differently. Each is `asset.stand_in_stale` (error), with `via pack accept` as hint.
  No facts file at all is `build.no_facts` (`build.md`).
- Budgets count each stand-in's accepted `triangles`, not the box's 12, so a budget gives the same
  outcome as with the files.
- With a current facts file, every instance transform and footprint equals the build with the files.

## 10. Forward pointers (M5)

Not in v0. Each is a declared fact or table to come; nothing here may be read as already deciding
it. The words are reserved until then (`doc.not_yet`).

- **Passable.** `solidity = "passable"`: claims ground for overlap, but a walk passes through it (a
  gate, an arcade; FINDINGS, the gatehouse). Only the walk lint, a library's, reads the difference;
  whether it is a core value or a library tag is decided in M5 by principle 5 (TODO, "Design
  passable footprints").
- **Connectors.** A mount with a direction, a height and a type, so a wall end meets only a wall
  end, and a fallback for an unused one (an unstitched doorway filled with wall; Dungeon Architect).
  Optionally read from named glTF nodes in the model and compared with the pack, disagreement a
  diagnostic (report, 5.5). Asset mounts today have `at` and `face` only (2.7).
- **Module per axis.** The pitch a piece repeats at along run, depth and storey, apart from its
  bounds, with an allowed overhang, checked against `bounds` (report, 5.2; FINDINGS, the college's
  6.44 m bay at a 6 m pitch). Whether this is a core fact or the building library's is (open),
  decided in M5 by principle 5.
- **Kit table, roles, variants, allowed stretch.** The building library's, over `taxon`, `tags` and
  facts (report, 5.4).
- **States.** The same asset in another condition (a door open). With connectors.

## 11. Example

```toml
# packs/vale.toml
root = "models"

[defaults]
pivot = "base_centre"

[taxon."plant/tree"]

[taxon."building/house"]
defaults = { max_relief = 0.6 }

[asset.oak]
file      = "oak.glb"
detail    = ["oak_low.glb"]
taxon     = "plant/tree"
footprint = [[-0.4, -0.4], [0.4, -0.4], [0.4, 0.4], [-0.4, 0.4]]   # the trunk, not the canopy
sink      = 0.2

[asset.cottage]
file  = "cottage.glb"
taxon = "building/house"
up    = "+z"                                                   # exported Z up, front -Y
front = "-y"

[asset.cottage.mount.door]
at = [0, 3.1]
```

```toml
# world.toml (part)
pack = "packs/vale.toml"

[instance.cottage]
asset  = "cottage"
ground = "pad"
on     = { ref = "lane_west", t = 0.6, offset = 12 }

[scatter.trees]
assets  = ["@plant/tree"]
spacing = 7
```

`on = { ref = ... }` is `rules.md` 3's form. `cottage.glb` is re-exported 0.5 m wider. The next build reports:

```
warning[asset.revised] asset.cottage: 2 facts differ from packs/vale.facts.toml
  --> packs/vale.toml:20:1
  bounds [-4.2, -3.1, 0, 4.2, 3.1, 6.35] -> [-4.45, -3.1, 0, 4.45, 3.1, 6.35]
  footprint grew by 0.25 m on each side; 1 instance: cottage
  hint: review the change report, then run `via pack accept packs/vale.toml`
```

Here the asset id and the instance id are both `cottage`. The change report lists instance
`cottage` as `changed` and footprint `cottage` as `moved`, cause `asset.cottage.bounds` (the asset
id), each with `facts` old and new. No world text changed. Had only the triangles
changed, the build would say `asset.remodelled` and nothing would move.

## Diagnostics

Subjects (`diagnostics.md` 1.1): `asset.<id>`, `material.<name>`, `taxon."<path>"`, or `pack` for the
pack as a whole; a fault in one key adds the key path (`asset.cottage.front`). A fault inside a model
has the "file" location form with `data.pointer` (3.1). Codes of other chapters this chapter raises:
`doc.unknown_key`, `doc.type`, `identity.bad_id`, `doc.not_yet`, `doc.path_form`, `doc.path_case`,
`identity.reserved`, `build.no_facts`.

| Code | Severity | M | Meaning | Hint |
|---|---|---|---|---|
| `pack.not_found` | error | 1 | The `pack` string names no file; the roots searched, by place. | The nearest pack name, or write a path. |
| `pack.shadowed` | info | 1 | A pack name found in several roots. | The one used; write a path to choose another. |
| `pack.range` | error | 1 | A pack number out of range: `size` or `repeat` not above 0, `triangles`, `max_relief` below 0, a `colour` part outside 0 to 1. | The allowed range. |
| `pack.bad_axes` | error | 1 | `front` or `up` is not one of the six signed axes, or both name one axis. | A pair on two axes, for example `front = "-y"`, `up = "+z"`. |
| `pack.unknown_taxon` | error | 1 | An asset's `taxon`, a selection or a `--taxon` filter names an undeclared taxon. | The nearest declared path, or declare `[taxon."<path>"]`. |
| `pack.unknown_material` | error | 1 | A `paint` names a material the pack lacks. | The nearest material, or add `[material.<name>]`. |
| `pack.material_repeat` | error | 1 | A material has `file` without `repeat`, or `repeat` without `file`. | Write both, or neither. |
| `pack.missing_file` | error | 1, 2 | `root`, a file the pack names, or a URI a model references, does not exist. M1 checks `root` and a material's `file`; M2 adds models, `detail` copies and their URIs. | The nearest file name in that folder. |
| `pack.outside_root` | error | 2 | A file reached outside `root`: an absolute URI, a `..` out of it, a scheme other than `data:`, a symbolic link. | Move the file under `root`, or move `root`. |
| `pack.bad_model` | error | 2 | A model is not readable glTF 2.0, holds a non-finite number or a bad index count, or uses an extension v0 does not read. | Re-export without the named extension, or fix the pointed part. |
| `pack.unaccepted` | info | 2 | The pack has model assets and no facts file; revisions cannot be reported. | Run `via pack accept PACK`. |
| `pack.bad_facts` | error | 2 | The facts file is not readable, or not in 4.1's form. | Delete it and run `via pack accept PACK`. |
| `pack.accept_refused` | error | 2 | `via pack accept` found errors; nothing written. | Fix the listed errors, then accept again. |
| `pack.stale_facts` | info | 2 | The facts file holds ids the pack no longer has. | Run `via pack accept PACK` to drop them. |
| `asset.unknown` | error | 1 | A world names an asset id the pack lacks. | The nearest id. |
| `asset.source` | error | 1 | An asset has no source marker, or more than one, or lacks a key its source requires (`size`, `triangles`; 2.2). | Write exactly one of `file`, `primitive`, `engine = true`, or the named key. |
| `asset.declared_measured` | error | 1 | An asset's own table writes a fact its source measures or fixes (2.2). | Remove the key; for a model, `via pack list` shows the measured value. |
| `asset.unknown_primitive` | error | 1 | `primitive` names no shape of section 6. | The nearest shape word. |
| `asset.bad_footprint` | error | 1 | A declared footprint is not a simple polygon of at least three distinct points, counter-clockwise. | Reverse the points, or remove the repeated one. |
| `asset.point_footprint` | info | 2 | A measured footprint has zero area; the asset has a point footprint. | Declare `footprint` if it should claim ground. |
| `asset.empty_selection` | error | 1 | A selection or list filter names a declared taxon, or a tag, with no asset. | The nearest taxon or tag that has assets. |
| `asset.detail_duplicate` | error | 2 | `detail` names `file`, or one path twice. | Remove the repeat. |
| `asset.empty` | error | 2 | A model has no scene or no triangles. | Export the mesh into the default scene. |
| `asset.untextured` | info | 2 | No material of a model has a base colour texture. | Bind the texture in the model if it should have one. |
| `asset.pivot_moved` | info | 2 | A model's origin is not where its pivot rule puts the pivot; the build moved it by `origin`. | Write `pivot = "origin"` to keep the model's own origin. |
| `asset.detail_bounds` | warning | 2 | A lower-detail copy reaches outside the main model's plan bounds. | Re-export the copy inside the main model. |
| `asset.detail_pivot` | warning | 2 | A lower-detail copy's pivot differs from the main model's by more than 1 mm. | Re-export the copy with the main model's origin. |
| `asset.unaccepted` | info | 2 | An asset the build uses is missing from the facts file. | Run `via pack accept PACK`. |
| `asset.remodelled` | info | 2 | An asset's files, triangles, colour or params changed and its placement facts did not. | None needed; accept to record it. |
| `asset.refiled` | info | 2 | An asset's `taxon` or `tags` differ from the facts file. | Review what selections now pick, then accept. |
| `asset.revised` | warning | 2 | An asset's placement facts differ from the facts file; names every instance in the build. | Review the change report, then run `via pack accept PACK`. |
| `asset.stand_in_stale` | error | 2 | With `--stand-ins`, the facts file is missing a used asset or disagrees with the pack. | Run `via pack accept PACK` with the files present. |

## Conformance

Each case runs through the command line (R20). "Counts" and "facts" are read from the build
directory's `assets` and `instances` (`build.md` 7, 8) or from `via pack list`. The runner also
checks that a clean build equals the incremental one.

| Case | M | Input | Action | Must hold |
|---|---|---|---|---|
| Pack by name | 1 | `pack = "primitive"` | `via build` | `lib/packs/primitive.toml` is read, from the `VIA_LIB` root the runner sets; it is in the build's `inputs`. |
| Pack not found | 1 | `pack = "primitve"` | `via build` | `pack.not_found`, hint `primitive`, exit 1. |
| Pack shadowed | 1 | `lib/packs/primitive.toml` beside the world | `via build` | `pack.shadowed` naming root `beside`, never a path; that one is used. |
| Primitive pack | 1 | A world on `lib/packs/primitive.toml`, no model files anywhere | `via build` | Exit 0; each asset's `triangles` as section 6 (box 12, prism 8, cylinder 60, cone 30, sphere 224). |
| Box bounds | 1 | `primitive = "box"`, `size = [1.001, 2, 3]` | `via pack list` | `bounds = [-0.5, -1, 0, 0.501, 1, 3]`, `origin = [0, 0, 0]`. |
| Ring footprint | 1 | `primitive = "cylinder"`, `size = [2, 2, 1]` | `via pack list` | 16 points; the first `[1, 0]`, the fifth `[0, 1]`, counter-clockwise. |
| Engine-made | 1 | `engine = true`, `size = [4, 4, 9]`, `triangles = 500` | `via pack list` | `source = "engine"`, `bounds = [-2, -2, 0, 2, 2, 9]`, `triangles = 500`. |
| Engine-made incomplete | 1 | `engine = true`, no `triangles` | `via pack check` | `asset.source` naming `triangles`, exit 1. |
| No source | 1 | An asset with only `size` and `params` | `via pack check` | `asset.source`, exit 1. |
| Two sources | 1 | `primitive = "box"` and `engine = true` | `via pack check` | `asset.source`, exit 1. |
| Fixed on primitive | 1 | `[defaults] pivot = "base_corner"`, a primitive, and a primitive writing `pivot` itself | `via pack check` | One `asset.declared_measured`, on the asset that wrote it; none for the default. |
| Unknown primitive | 1 | `primitive = "boxx"` | `via pack check` | `asset.unknown_primitive`, hint `box`, exit 1. |
| Bad value | 1 | `solidity = "solidd"`; `size = [0, 1, 1]`; `colour = [1.2, 0, 0]` | `via pack check` | `doc.type` hint `solid`; two `pack.range`; exit 1. |
| Passable reserved | 1 | `solidity = "passable"` | `via pack check` | `doc.not_yet` naming M5. |
| Unknown default key | 1 | `[defaults] pivt = "origin"` | `via pack check` | `doc.unknown_key`, hint `pivot`. |
| Defaults order | 1 | `colour` in pack defaults, in `plant` and `plant/tree` defaults, and on one asset | `via pack list` | Each asset's resolved value per 2.5. |
| Bad axes | 1 | `front = "+y"`, `up = "-y"` | `via pack check` | `pack.bad_axes`, exit 1. |
| Bad footprint | 1 | A clockwise square; a square with its first point repeated | `via pack check` | Two `asset.bad_footprint`, exit 1. |
| Mount reserved | 1 | `[asset.box.mount.start]`; again with `ground` | `via pack check` | `identity.reserved`, exit 1. |
| Mount connector | 1 | `connector = "wall"` on an asset mount | `via pack check` | `doc.not_yet` naming M5. |
| Material | 1 | A material with `file` and no `repeat`; a `paint` naming `gras` | `via build` | `pack.material_repeat`; `pack.unknown_material` hint `grass`; exit 1. |
| Dotted id | 1 | `[asset."house.low"]` | `via pack check` | `identity.bad_id`, exit 1. |
| Unknown asset | 1 | A world on the primitive pack naming `house_lo` | `via build` | `asset.unknown`, hint `house_low`, exit 1. |
| Selection | 1 | Three assets under `plant/tree`, one under `plant` | `via build` with `@plant/tree` | Resolves to the three, ordered by id. |
| Empty selection | 1 | `@plant/shrub`, declared, nothing filed | `via build` | `asset.empty_selection`, exit 1. |
| Undeclared taxon | 1 | `@plant/shurb`, not declared | `via build` | `pack.unknown_taxon`, hint `plant/shrub`, exit 1. |
| Unknown taxon | 1 | `taxon = "plant/tre"` on an asset | `via pack check` | `pack.unknown_taxon`, hint `plant/tree`. |
| List filters | 1 | The selection pack | `via pack list --taxon @plant/tree --tag winter` | Only the assets passing both, one JSON object per line, sorted by id, exit 0. |
| Unused bad value | 1 | A pack whose unused asset has `size = [-1, 1, 1]` | `via build` | `pack.range`, exit 1: pack-wide (8.3). |
| Pack swap | 1 | `ford` on the primitive pack (6.1); `examples/ford/` copied in, with the second pack and change file of 6.2 | `via build`, then `via build --change changes/bigger.toml --against` the first | `cottage` is `changed` only, footprint `cottage` is `moved` and larger (12 m by 10 m); both carry `facts` with old and new `bounds`, `size` and `footprint` (section 7's values); causes `asset.house_low.size`, then `world.pack` (bytewise). World text untouched. |
| Root from the pack | 1 | `case/packs/p.toml` with `root = "../art"` and `[material.grass]` with `file = "grass.png"`, `repeat = 2`; `case/art/grass.png`; `case/world.toml` painting `grass`. Run from a working directory that is neither `case/` nor `case/packs/` | `via pack check case/packs/p.toml`; `via build case/world.toml` | Both exit 0, no `pack.missing_file`: `grass.png` is found under `case/art/`, relative to the pack file. Then delete `case/art/`: `pack.missing_file` naming `root`, exit 1 (limitation S11.T.6). |
| Primitive scale | 1 | A box instance at `scale = 2`, with a mount at `[1, 0]` and `max_relief = 0.5` | `via build` | Footprint, `sink`, `max_relief` and the mount's `at` doubled. |
| glTF axes | 2 | A `.glb` 1 m cube centred at model `(1, 0.5, 0)`, `pivot = "origin"` | `via pack list` | `bounds = [-1.5, -0.5, 0, -0.5, 0.5, 1]`. |
| glTF axes, base_centre | 2 | The same, default pivot | `via pack list` | `bounds = [-0.5, -0.5, 0, 0.5, 0.5, 1]`, `origin = [1, 0, 0]`, `asset.pivot_moved`. |
| Z-up export | 2 | The same cube exported Z-up, centred at model `(1, 0, 0.5)`, `up = "+z"`, `front = "-y"`, `pivot = "origin"` | `via pack list` | `bounds = [-1.5, -0.5, 0, -0.5, 0.5, 1]`. |
| Float noise | 2 | The glTF axes cube with one vertex at `1.5000001` | `via pack list` | Same `bounds` as the clean cube. |
| Pivot rule | 2 | A model centred in plan, its origin 2 m above its base, `base_centre` | `via build` | `origin = [0, 0, 2]`, `asset.pivot_moved`; the lowest point at the placing point minus `sink`. |
| Pivot origin | 2 | Same model, `pivot = "origin"` | `via build` | `origin = [0, 0, 0]`, no `asset.pivot_moved`; the model stands 2 m lower than in the previous case. |
| Triangle count | 2 | A mesh with a list of 9 indices, a strip of 5 and a fan of 4, used by two glTF nodes | `via pack list` | `triangles = 16` (3 + 3 + 2, twice). |
| Bad index count | 2 | Mode 4 with 7 indices | `via pack check` | `pack.bad_model`, `data.pointer` the primitive. |
| Sparse | 2 | A cube whose positions are a sparse accessor moving one corner 1 m out | `via pack list` | `bounds` includes the moved corner. |
| Quantized | 2 | The cube with `KHR_mesh_quantization` required | `via pack list` | Same `bounds` as the float cube. |
| Compressed | 2 | `KHR_draco_mesh_compression` required | `via pack check` | `pack.bad_model` naming it. |
| Instancing | 2 | `EXT_mesh_gpu_instancing` in `extensionsUsed` only | `via pack check` | `pack.bad_model` naming it. |
| Tolerated | 2 | `KHR_materials_emissive_strength` required | `via pack check` | No error. |
| Skinned | 2 | A skinned cube whose glTF node is translated 5 m | `via pack list` | `bounds` as the unskinned cube under that glTF node. |
| Morph | 2 | A cube with a morph target and default `weights = [1]` | `via pack list` | `bounds` of the base positions. |
| No scene | 2 | A `.gltf` with meshes and no `scenes` | `via pack check` | `asset.empty`. |
| Measured only | 2 | A model asset with `size` | `via pack check` | `asset.declared_measured`, exit 1. |
| Missing texture | 2 | A model referencing a missing image | `via pack check` | `pack.missing_file` naming the URI and the asset. |
| Encoded URI | 2 | A buffer URI `my%20mesh.bin` beside `my mesh.bin` | `via pack check` | No error. |
| URI case | 2 | A URI `Oak.png` beside `oak.png` | `via pack check` | `doc.path_case`, on every system. |
| Outside root | 2 | A model with a `../` buffer URI out of `root` | `via pack check` | `pack.outside_root`. |
| Data URI | 2 | A `.gltf` whose buffer is a `data:` URI | `via pack check` | No error; no bound file besides the model. |
| Untextured | 2 | A model whose material has only a base colour factor | `via pack check` | `asset.untextured`, exit 0. |
| Flat card | 2 | A model of one quad in the asset `x`-`z` plane | `via pack list` | A point footprint, `asset.point_footprint`. |
| Detail checks | 2 | A detail copy 0.1 m wider; another shifted 0.5 m up; `detail` naming `file` | `via pack check` | `asset.detail_bounds`, `asset.detail_pivot`, `asset.detail_duplicate`. |
| Base corner | 2 | A model with bounds `[2, 3, 1]` to `[4, 5, 2]` before the pivot, `pivot = "base_corner"` | `via pack list` | `bounds = [0, 0, 0, 2, 2, 1]`, `origin = [-2, -3, -1]`. |
| Unaccepted | 2 | A model pack with no facts file | `via build` | `pack.unaccepted` once, exit 0. |
| Accept refused | 2 | A pack with an unreadable model | `via pack accept` | `pack.accept_refused`, exit 1, no facts file written. |
| Accept bytes | 2 | A model pack | `via pack accept` twice | The facts file equals the vector in `spec/`; the second run writes the same bytes. |
| Bad facts | 2 | A facts file with a syntax error | `via build` | `pack.bad_facts`, exit 1. |
| Stale facts | 2 | Accept, then delete an asset from the pack | `via pack check` | `pack.stale_facts` naming it, exit 0. |
| New asset | 2 | Accept, then add a used model asset | `via build` | `asset.unaccepted` for it. |
| Geometry-only revision | 2 | Accepted; model re-exported with more triangles, same bounds | `via build` | `asset.remodelled` with old and new triangles; no instance moved in the change report. |
| Contract revision | 2 | Accepted; model re-exported 0.5 m wider | `via build` | `asset.revised` listing every instance of it; change report entries with cause `asset.<asset id>.bounds`. |
| Refiled | 2 | Accepted; one asset's `taxon` changed | `via build` | `asset.refiled`; no `asset.revised`. |
| Accept | 2 | After the contract revision | `via pack accept`, then an incremental `via build` | No `asset.revised`; only the `source:` node of the facts file and the `facts:` nodes ran; the facts file diff shows only that asset. |
| Unused broken asset | 2 | A pack with one unreadable model the world does not use | `via build`; `via pack check` | Build exit 0; check exit 1 with `pack.bad_model`. |
| Stand-ins | 2 | A world and an accepted pack, model files deleted | `via build --stand-ins` | Every instance transform and footprint equal to the build with the files; budgets' `triangles` equal. |
| Stand-ins stale | 2 | Accepted; a declared `sink` then edited in the pack | `via build --stand-ins` | `asset.stand_in_stale`, exit 1. |
| Same bytes | 1 | The primitive pack case; from M2 the contract revision case | Built at 1 and 8 threads, on each system | Same build digest (`build.md` 12) (R26). |

## Changed from the spike

- `size` is measured for a model and declared only for primitives and engine-made assets; `tris` is
  `triangles`, and primitive counts are a real table, not 12 for every unknown shape.
- `recipe` is an engine-made asset, marked `engine = true`, with `params`; `fit_height`, `fit_width`
  and the asset `scale` are gone (units, open).
- Front was an unwritten convention and yaw ran counter-clockwise from east; now glTF's rule with a
  per-asset correction, and `face` is a bearing (`numbers.md`).
- A pivot rule is declared and checked; the spike only lifted a model's base to 0.
- `solid` (bool) is `solidity` (`solid`, `none`; `passable` reserved for M5). The spike's `avoid` (a
  list of fields) is gone: the `keep_out` field role covers it. The spike's fixed 1 m distance is
  gone: scatters give way by footprint overlap. `clearance` (never read) is gone. The viewer
  `color` is `colour`, used by primitives and stand-ins. Collision is a loader convention in
  `params`.
- `labels` is `tags`. A taxonomy is declared in the pack and checked; `@path` works wherever assets
  are listed, not only in a kit run.
- `[ground.<n>]` is `[material.<n>]`; `[surface.<n>]` is gone (R25). `module_m` waits for M5.
- `root` is relative to the pack file, not the working directory. Round footprints are polygons.
- New: finding a pack by name; old and new facts in the change report; the facts file and its diff;
  `via pack check`, `via pack list` (from the zoo), `via pack accept`; stand-ins; an asset as a set
  of files with lower-detail copies.

## Open

- **Models not in metres.** A declared factor or a fit to a height: decided by the first generated
  pack in M2.
- **Measured footprint.** The plan rectangle of `bounds`, or the convex hull of the vertices: decided
  by the M2 real-pack swap, by how often the rectangle has to be overridden.
- **An engine-made asset's own files** (a generator's script, an atlas) as declared inputs: decided
  in M2 by the tree case that needed them in the spike.
- **A shared taxonomy across packs** (one tree file several packs include): decided by the M2 pack
  swap, if two packs drift.
- **A selection as an instance's or run's `asset`** (the spike's kit run): decided by the town kit's
  run in M3, with `rules.md`.
- **Selection by tag in a world:** decided by the first case that needs it.
- **Usage** (which builds place an asset; which assets nothing places): decided in M2 with
  `build.md`'s instance list.
- **Revisions across imports** (an asset revised in an imported world's pack): `documents.md`, M4.
- **Formats other than glTF** (FBX, USD): converted outside the build; reopened only by a user case.
- **Passable, connectors, module per axis, kit table, states:** M5 (section 10).
