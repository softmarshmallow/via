# The build directory (v0)

A build directory is what `via build` writes and the only thing an engine loader, a lint, an export
step or another tool needs to read: fields, instances, features, meshes, chunks and metadata, and
nothing an engine owns (principle 12). This chapter fixes its layout and encodings, the identity of
a build, the command line that writes it, and how three readers map it: Godot 4, Unreal on paper,
and a glTF export.

Words are from `vocabulary.md`. Units, axes, posts, windows and facings are `numbers.md`'s; fields
and roles `fields.md`; assets, facts and materials `assets.md`; produced names and the change
report's entries `identity.md`; nodes, keys, the store and the run report's contents `graph.md`;
surfaces, their meshes and meetings `surfaces.md`; the shape of a diagnostic `diagnostics.md`; step
output folders and array blobs `protocol.md`. "(open)" marks what is undecided, with what decides
it.

**What arrives when.** Every part is marked where it is not M1. M1's conformance runner treats a
case marked with a later milestone as pending.

| Milestone | Parts |
|---|---|
| M0 | The digest (3.3) and the null vector (section 12). |
| M1 | Everything not marked below. |
| M2 | `--stand-ins`, `assets.stand_in`, model files in `assets.files`, `--fail-on asset.revised`. |
| M3 | Generated meshes (section 10), kind surfaces as bounded fields (6.3), network areas and cells in `features.json`. |
| M6 | `chunk` in the world, export steps (11.4) and `exports`, the player map (9.4), budgets. |

## 1. Rules for every build

1. **Thrown away and made again** (principle 1). Nothing in a build is edited by hand or read back
   as a source. A build whose files differ from the manifest's record is written again, whole
   (section 13).
2. **Replaced whole.** A build writes a complete directory or none (section 13). No file of an
   earlier build survives into a later one. The spike left stale files.
3. **Nothing absolute.** No file the build writes holds an absolute path, a host name, a user name,
   a time or a thread count (`numbers.md` 9). Paths are written with `/` against a named root
   (3.4). What an export step writes is the step's (11.4).
4. **No non-finite value, no signed zero.** Every number in every file is finite. A zero is stored
   as `+0` (`numbers.md` 8). A non-finite value reaching the writer is a defect of ours
   (`build.internal`, exit 3).
5. **No sentinel values.** Every stored sample is a value of its field and means only that. Where a
   field has a value in part of the extent only, the build writes its cover (6.3); "no value" is
   never a number such as `0`, `-1e9` or a height below the ground. The spike's `[look]` and
   `params.sea` let a reader guess water from a magic number; v0 has no such channel.
6. **Same text, same bytes** (R26). The same text, with the same flags and numerics epoch, gives
   every file the same bytes on macOS, Linux and Windows and at any thread count. Two parts of the
   manifest are exempt and are named in section 12: `locations` (where `--out` is) and `run` (how
   the build was asked for). The run report is outside the directory for this reason (11.3).
7. **Every field is written.** Which fields a loader draws is the loader's option, never world text
   (doctrine section 6). The spike's `atlas` exported no water and nothing said so (TODO); a loader
   now says which fields it did not draw (15.1).

## 2. Layout

```
build/ford/
  manifest.json              the index: everything below, with digests (section 4)
  diagnostics.json           the world's diagnostics, always (11.1)
  changes.json               the change report (11.2)
  .previous                  the last complete build (11.2)
  fields/height.npy          one array file per scalar field, whole extent (6.1)
  weights/material/0_0.npy   one array file per weights field per chunk (6.2)
  instances/x.npy ...        instance columns, grouped by chunk and asset (section 8)
  instances/names.txt        one name per instance row
  features.json              every feature with its shape and attributes (9.1)
  meets.json                 every meeting of two surfaces (9.3)
  meshes/<name>.glb          generated meshes (section 10, M3)
  export/<step id>/...       what each export step wrote (11.4, M6)
```

| File | In the `id` (12) | Written when | Read by |
|---|---|---|---|
| `manifest.json` | yes, less `id`, `run` and `locations` | the build has no error | every reader |
| `diagnostics.json` | no; listed in `run.reports` | always, when a build directory is known | agents, people, CI |
| `changes.json` | no; listed in `run.reports` | the build has no error | review, agents |
| `fields/*`, `weights/*/*` | yes | the build has no error | loaders, steps |
| `instances/*` | yes | the build has no error | loaders |
| `features.json`, `meets.json` | yes | the build has no error | loaders, steps, the map library |
| `meshes/*.glb` | yes | a mesh exists (M3) | loaders |
| `export/<step id>/*` | yes | an export step wrote it (M6) | whatever reads that step's output |
| `.previous` | no | always | the next build (11.2) |

`files` (section 4) lists the files in the `id`; `run.reports` lists `diagnostics.json` and
`changes.json`. `manifest.json` and `.previous` are the only files of a directory that no list
names. Any other file is not part of the build, and the writer leaves none (rule 2).

## 3. Encodings

### 3.1 JSON

Every JSON file the build writes is in the canonical form of `documents.md` section 8: UTF-8, no
byte-order mark, object keys in bytewise order, no insignificant white space, one LF at the end.
Numbers follow `numbers.md` 3.4: a lattice value as exact metres (`1.005`), a double as the shortest
decimal that reads back to the same bits, a 64-bit seed as 16 hex digits in a string. A
single-precision value (a field's `min`) is widened to a double first, so it reads back exactly.

### 3.2 Array files

Every large table of numbers is an **array file**: an array blob of `protocol.md` 4.2, that is a
NumPy `.npy` file, format version 1.0 (NumPy Enhancement Proposal 1), in the subset that section
fixes, with extension `.npy`. One format serves the build, a step's returned field and an author's
`file` (`fields.md` 4.1), so a step's output and a build's field are the same bytes. The spike's
raw `.f32` and an earlier draft's header of our own are both retired.

| Part | Rule |
|---|---|
| Header | The text of `protocol.md` 4.2, padded with spaces to the smallest total that is a multiple of 64 bytes, so data is 64-byte aligned. Some NumPy releases pad further when they write; `numpy.load` reads either, and the build writes the smallest. |
| `descr` | `<f4` for field samples and the `z`, `scale`, `tint` columns; `<i4` for `x`, `y`, `face`; `\|u1` for weights and covers. |
| `shape` | A scalar field or a cover: `(ny, nx)`. A weights chunk: `(planes, rows, columns)`. An instance column: `(count,)`, and `(0,)` when there are no instances. |
| Data | C order, little-endian, no trailing bytes. |

- Row `j` is post row `j` from the south, column `i` post column `i` from the west: value `[j][i]`
  belongs to post `(i, j)` of `numbers.md` 4. A PNG's top row is north; an array file's first row is
  south.
- The header holds no extent, resolution or name. Where the samples lie comes from the manifest in
  a build, or from the field's `extent` when read as a `file` (`fields.md` 4.2). One fact in one
  place (principle 10).
- Read with numpy alone: `numpy.load(path, mmap_mode="r")`.
- A weights chunk is three-dimensional; `protocol.md` 4.2 allows one or two dimensions and is asked
  to allow three for this file (Answers, below).

### 3.3 Digests

A file's digest is the BLAKE3 digest of its bytes (O'Connor, Aumasson, Neves and
Wilcox-O'Hearn, 2020), 256 bits, written as 64 lowercase hex digits: the one digest `graph.md` 3.1
uses for keys and the store, and `protocol.md` 4.1 for blobs. It is the "content digest"
`assets.md` 4 refers to. Decided in M0, because the null vector (section 12) is useless until the
digest is fixed; SHA-256 was weighed and not taken, since `graph.md` and `protocol.md` already rest
on BLAKE3.

- A reader never needs a digest to read a build: it takes file names from the manifest and checks
  sizes from headers. `via check` checks digests (section 14).
- The pure-Python SDK computes no digest of a build file. How it names a blob it writes (the engine
  renames it, or a pure-Python BLAKE3) is `protocol.md`'s (open).

### 3.4 Paths

Every path a build file holds is written with `/`, and is one of three forms:

| Form | Base | Where |
|---|---|---|
| A build path | the build directory | `files`, every `file` key, `exports`. No `.` or `..` segment. |
| A pack path | the pack's `root` (`assets.md` 2.1) | `assets.files`, `materials.file`. No `.` or `..` segment. |
| A source path | the one form of a diagnostic's `file` (`diagnostics.md` 1.2, which `documents.md` 2.1 points to): a path under the world file's folder, or a tagged form (`lib:<library id>/`, `pack:`, `change:`, `step:<id>/`, `input:`) | `run.inputs`, `run.changes`, `diagnostics.json`. |

- A source path never depends on the machine: a file in a library found through `--lib /opt/x` is
  `lib:<library id>/...`, never `/opt/x/...`, and the order of `--lib` roots does not change it. It
  holds `..` only where the world's own text wrote a path with `..` (`documents.md` 2.1); that text
  is the same on every machine. Where `diagnostics.md` 1.2 and `documents.md` 2.1 still differ in
  wording, this chapter follows whichever form they settle on; it defines none of its own.
- `locations` (section 4) is the one key whose value depends on where the directory is. It may
  climb out with `..`.

**File names.** A build path is made from ids and names, which are lowercase (`vocabulary.md` 2.1),
so two never collide on a file system that ignores case. Names hold only `[a-z0-9_.#]` (section
8). Two escapes keep every name writable on the three systems:

- A path segment whose part before its first `.` is a Windows device name (`con`, `prn`, `aux`,
  `nul`, `com1` to `com9`, `lpt1` to `lpt9`) gets `-` after that part: a field `nul` is
  `fields/nul-.npy`. Ids and names never hold `-`, so the escape cannot collide.
- On Windows the writer uses extended-length paths (`\\?\`), so a deep `--out` with `+<change>`
  suffixes is not cut at 260 characters.

Readers take every file name from the manifest and never build one from a name.

## 4. The manifest

`manifest.json` is one object. Keys a reader does not know are ignored; a `format` it does not know
is refused.

| Key | Holds |
|---|---|
| `format` | `"via-build-0"`. Changes when a reader of the old format would misread the new one. Added keys and files do not change it. The build format version of `graph.md` 3.3. |
| `engine` | `{ "epoch": <integer>, "name": "via" }`: `graph.md` 3.3's numerics epoch. The release number is in `run`. |
| `world` | The world's `name`. |
| `seed` | The world seed, 16 hex digits. |
| `frame` | Section 5. |
| `chunks` | Section 5. |
| `fields` | Section 6. |
| `materials`, `assets` | Section 7. |
| `instances` | Section 8. |
| `features`, `outputs`, `meets` | Section 9. |
| `meshes` | Section 10 (M3). |
| `exports` | `{ <step id>: "export/<step id>/" }` (11.4, M6). |
| `files` | Every file in the `id`: `{ <build path>: { "bytes", "digest" } }`. |
| `id` | The build's identity (section 12). |
| `run` | How this build was asked for. Not in the `id`. Keys below. |
| `locations` | `{ "pack_root": <path from this directory to the pack's root, or null> }`. Not in the `id`. |

`run` holds:

| Key | Holds |
|---|---|
| `version` | The release of the `via` binary that wrote it, semver. |
| `key` | The build key of `graph.md` 6.2, which already covers the flags below. |
| `flags` | `{ "fail_on": [<codes, sorted>], "stand_ins": <bool> }`: every flag that can change an output. |
| `changes` | The requested changes, sorted bytewise: names, and change files as source paths. |
| `inputs` | Every source and input file read, `[{ "digest", "path" }]` sorted by source path: world, layers, libraries, kinds files, pack, facts file, change files, field files, step `inputs`, asset files. |
| `reports` | `{ "changes.json": { "bytes", "digest" }, "diagnostics.json": { "bytes", "digest" } }`; `changes.json` is absent when not written. |

`pack_root` is the one path that may climb out of the directory, so a loader finds asset files
without a copy (principle 6: the build names files only through the pack's entries). It is `null`
when no relative path exists (on Windows, another drive), with `build.pack_root_far` (warning); a
loader is then told where the pack is. Copying asset files into the build is (open), below.

## 5. Frame, posts and chunks

```json
"frame": {"axes":"x east, y north, z up","bearing":"degrees clockwise from north","extent":[0,0,256,256],"posts":[513,513],"resolution":0.5,"unit":"m"}
```

`frame` repeats `numbers.md` 4 and is the only place a reader finds where the posts lie. The world's
datum, once `geometry.md` declares one, is added here (open in `vocabulary.md` section 4).

**Chunks.** The build's tiling is a grid of square chunks. `fields.md` and `graph.md` say "tile" for
the same thing; `chunk` is the vocabulary's word, and those chapters are asked to use it (Answers).
The side `c` is the world's `chunk` (M6; `doc.not_yet` in M1). Until then, and when `chunk` is not
written, `c` is the largest multiple of `resolution` not above 64 m, and at least `resolution`. A
written `chunk` that is not a whole multiple of `resolution` is `build.chunk_not_multiple` (M6).
Whether 64 m stays the default is (open), decided by `ford`'s build times in M1.

- With `k = c / resolution` posts per side, chunk `(cx, cy)` holds posts `i` from `cx * k` to
  `min((cx + 1) * k, nx - 1)` and `j` likewise, both ends included. Neighbours share their edge
  posts, so ground drawn per chunk meets without a crack. The last chunk in each direction may be
  narrower.
- There are `ceil((nx - 1) / k)` by `ceil((ny - 1) / k)` chunks, at least 1 by 1.
- A plan point `(x, y)` in integer millimetres, with `C = c` in millimetres and `(x0, y0)` the
  extent's south-west corner in millimetres, is in chunk `(floor((x - x0) / C), floor((y - y0) / C))`
  in integer arithmetic, each index then clamped to `[0, chunks - 1]` in its direction. So a point
  on the north or east edge is in the last chunk. Nothing in a build stands outside the extent
  (`rules.md` 8.4), so the clamp moves only points on those two edges.
- Chunks are listed south row first, west to east: `cy` outer, `cx` inner, as posts are.
- By the window rule (`numbers.md` 6), a chunk's samples equal the same cut of the whole field.

**Chunks in the graph.** The build's `field` and `distance` nodes (`graph.md` 1) are one per chunk,
over the chunk's posts as a window: `w:<cx * k>.<cy * k>.<i1 + 1>.<j1 + 1>`, with `i1`, `j1` the
chunk's last posts. Neighbouring windows overlap by their shared edge posts; the window rule makes
both nodes give those posts the same bits, so no post needs an owner. A weights chunk file is one
`write` node reading its chunk's node. A whole-extent scalar file (6.1) is one `write` node reading
every chunk's node, taking a shared post from the first chunk in chunk order. `graph.md`'s worked
example, which shows disjoint windows, is asked to follow this.

`chunks` is a list, one entry per chunk:

| Key | Holds |
|---|---|
| `at` | `[cx, cy]`. |
| `posts` | `[i0, j0, i1, j1]`, the posts it holds, ends included. |
| `weights` | `{ <field>: { "file", "materials" } }` for each `weights` field (6.2). |
| `instances` | `{ <asset id>: [first, count] }`: its rows in the instance columns (section 8). |
| `count` | Instances in the chunk. |
| `triangles` | Sum of `triangles` of the finest copy of each instance's asset: what the pack holds, not what an engine draws. Budgets read it (M6). |

## 6. Fields

`fields` maps each field's name to an entry: `role`, `type`, `unit`, `taxon`, `tags` (each `null`
or empty when not written), and by type:

### 6.1 Scalar fields

`{ "file": "fields/<name>.npy", "max", "min" }`. One array file of `<f4` over every post of the
extent, shape `(ny, nx)`. `min` and `max` are over every post. Single precision follows `fields.md`
1.1. Written whole, since the stated users build a few kilometres (doctrine section 7): 2 km at
1 m is 16 MB. Per-chunk scalar files are (open), below.

### 6.2 Weights fields, with no limit on materials

A `weights` field is written per chunk: `weights/<field>/<cx>_<cy>.npy`, `|u1`, shape
`(planes, rows, columns)` over the chunk's posts. A world may use any number of materials; a chunk
names only its own (`fields.md` 7.1). This lifts the spike's limit of eight (TODO: `atlas` needed
eleven and drew asphalt as rock).

- **Bytes.** At each post, over every material of the field in id order: `S` is the sum of the
  weights in doubles, in material id order (`fields.md` 5.2). If `S` is 0, every byte is 0. Else
  each material gets `q = w * 255 / S` in doubles through `via-num`, each operation correctly
  rounded; bytes are `floor(q)`, then one more to each of the largest remainders `q - floor(q)`
  until the sum is 255, ties to the lower material id. This is the largest-remainder method
  (Hamilton); its use here is ours.
- **Planes.** A chunk has one plane per material with a byte above 0 at some post of the chunk,
  after the bytes above, in id order. The chunk entry's `materials` names the planes. Since bytes
  are worked out per post, before planes are chosen, a material's plane holds the same bytes in any
  chunking.
- At each post the planes sum to exactly 255, or every plane is 0 where nothing painted. A post with
  every byte 0 holds no material. That is a value, not a sentinel: the loader draws its own default.
- A chunk where no material has a byte above 0 has `{ "file": null, "materials": [] }`.
- Weights are 0 or above by `fields.md` 5.2; a weight below 0 or not finite reaching the writer is
  `build.internal`.

### 6.3 Bounded fields (M3)

A field owned by a feature with a bound (R25: a deck's surface over its strip) is written over the
posts of its bound's bounding box, with `fields/<name>.cover.npy` beside it: `|u1`, shape like the
field's, 1 where the post lies inside the bound, 0 outside. Its entry adds `posts`
`[i0, j0, i1, j1]` and `cover`; `min` and `max` are taken under cover 1 only. A sample under a 0 is
the value the field's formula gives there, never a marker; readers sample only under 1. v0 fixes
the encoding now so water, decks and floors never need a sentinel.

A kind surface is written twice, for two readers. Its **mesh** (section 10) is what a loader draws
and collides with. Its **bounded field** is for sampling only: steps, lints, the change report, and
a loader asked for a height at a point. No loader draws a surface's bounded field (15.1).

### 6.4 Water

The core knows no water (principle 5). A library that hands water to a loader writes a water
surface height field, with a `tag` the library documents (`fields.md` 2.2), whose value where dry
is the ground's height itself, so depth is `water - ground` and never below 0. The build writes it
like any other field. Drawing it is a loader option, named by that tag (15.1, `water`); `ford`'s
sheet passes `--option water=tag:water_level` (15.1). The spike's `water` raster held 0 where dry, which meant "below sea" in a valley and
"no water" on a hill. Whether a library's water is a bounded field (6.3) is that library's choice.

## 7. Assets and materials

`assets` holds every asset with at least one instance, by id. Keys are `assets.md`'s; the
`vocabulary.md` rows for them are owed there (`assets.md` 1.1).

| Key | Holds |
|---|---|
| `source` | `model`, `primitive` or `engine` (`assets.md` 2.2). |
| `files` | `[{ "digest", "path" }]`, pack paths: the model, then each `detail` copy, finest first (M2). Buffers and images a model references are found through it. Empty for a primitive or engine-made asset. |
| `primitive`, `size` | For a primitive (`assets.md` 6); `size` also for engine-made. |
| `to_asset` | 12 numbers, row by row, of the 3 by 4 matrix taking a point of the model file's node space, as stored (glTF axes, before any engine's import), to the asset frame (`assets.md` 2.3). Its 3 by 3 part is the quarter turn of `front` and `up`: entries -1, 0 or 1. Its translation is minus the pivot, which `assets.md` 2.4 gives in integer millimetres from bounds rounded by `numbers.md` 3.1, written in exact metres. Identity for a primitive or engine-made asset. |
| `bounds`, `footprint`, `origin`, `triangles` | Facts (`assets.md` 2.4), `triangles` per copy. |
| `mounts` | `{ <name>: { "at": [x, y], "face" } }`, sorted by name: each asset mount (`assets.md` 2.7) in the asset frame, `at` in exact metres, `face` in integer millidegrees. `{}` when it has none. A loader finds an instance's mount from this and the instance columns (15.1). |
| `solidity` | As declared (`solid`, `none`). What the core's own checks took it to be; a loader need not read it. |
| `colour` | As resolved (`assets.md` 2.4): the colour of a primitive or stand-in. |
| `taxon`, `tags`, `params` | Passed through. Collision is a loader convention inside `params` (`assets.md` 2.2); the core has no collision key. |
| `stand_in` | `true` when built with `--stand-ins` (M2): draw a `box` of `bounds` in `colour` (`assets.md` 9). |

`materials` holds every pack material a `weights` field names: `colour`, `file` (a pack path, with
`digest`) and `repeat` when written, and `params` passed through unread (`assets.md` 2.6).

## 8. Instances

Every instance in the build, hand-placed, produced or a set's item, is one row of one table. Rows are
sorted by chunk (in chunk order), then asset id, then name, bytewise. So each chunk's rows are
contiguous, and within it each asset's: an engine can batch per chunk and asset (TODO: one flat list
ran at 12 frames a second; grouping by chunk took it past 110).

| Column | `descr` | Holds |
|---|---|---|
| `instances/x.npy`, `y.npy` | `<i4` | Plan position of the pivot, integer millimetres, world frame (`numbers.md` 5.2). |
| `instances/z.npy` | `<f4` | Height of the pivot in metres, after `sink` (`surfaces.md` 4.2). Heights are not on the lattice (`numbers.md` 3.1). |
| `instances/face.npy` | `<i4` | Facing, integer millidegrees, `[0, 360000)` (`numbers.md` 2). |
| `instances/scale.npy`, `tint.npy` | `<f4` | As resolved (`rules.md`). |
| `instances/names.txt` | text | One name per line, each ended by LF, in row order. 0 bytes when there are none. |

Each column has shape `(count,)`; `instances` in the manifest holds `count` and the paths. A world
with no instances writes every column with shape `(0,)` and an empty `names.txt`.

- **Names are the stable ids.** A hand-placed instance's name is its id; a set's item is
  `<instance>.<name>`; a produced one is `<producer>#<place key>` (`identity.md` 2). Ids and place
  keys match `[a-z][a-z0-9_]*`, so a name holds only `[a-z0-9_.#]`. Names are the only thing a
  reader may key on across builds: re-import, a loader's own overrides, the change report. Row
  numbers change whenever anything earlier in the order does.
- **Placing a model.** The world position of a point `v` of the model file is
  `T(x, y, z) * R(face) * S(scale) * to_asset * v`, with `x`, `y` in metres and `R` the turn by the
  bearing `b = face / 1000` degrees, clockwise seen from above:

  ```
  R(b) = | cos b   sin b   0 |       R(90) * (0, 1, 0) = (1, 0, 0):
         | -sin b  cos b   0 |       face 90 turns the asset's front (+y) to east (+x).
         | 0       0       1 |
  ```

  `cos` and `sin` are `numbers.md` 2's, exact at quarter turns. The loader then maps world axes to
  its engine's (section 15).
- An instance with `ground` has a footprint feature (`rules.md` 2.1). It has no id of its own, so it
  is written under `footprints` in `features.json`, keyed by its instance's name, as `identity.md` 5
  reports it. The spike left footprints out.

## 9. Features, outputs and meetings

### 9.1 `features.json`

`{ "features": [ ... ], "footprints": [ ... ] }`. `features` holds every feature, written or
produced, guides included, sorted by name:

| Key | Holds |
|---|---|
| `name` | Its id, or its produced name. |
| `kind`, `authority`, `layer` | As resolved (`authority.md`); `kind` `null` for a guide. |
| `shape` | `point`, `path` or `region`. |
| `at` | A point: `[x, y, z]`. |
| `points` | A path: `[[x, y, z], ...]`, the vertices of its final curve: the written points, or with `smooth`, the polyline `geometry.md` 2.4 samples, on the lattice. |
| `pieces` | A region: `[{ "holes": [[[x, y], ...], ...], "outer": [[x, y], ...] }]`, every piece kept, in `geometry.md` 2.5's canonical form: rings not closed by a repeated vertex, each starting at its least vertex by `x` then `y`, outer rings counter-clockwise, holes clockwise. |
| `attrs` | Every attribute after kind defaults, in canonical JSON, passed through unread, except a name attribute (below). |

**A name attribute** (`authority.md` 1.2: an attribute the kind lists in `names`) is written
resolved, as `{ "at": [x, y, z], "name": "<place name>" }`: `name` as the text wrote it, `at` the
point `authority.md` 1.2 gives for it, in the units of `at` above. The core resolves it; a reader
reads `at` and never resolves a name itself. The review library's bookmark `toward = "ford"` is
written `"toward":{"at":[153.63,115.48,…],"name":"ford"}`: the ford's lattice point by `numbers.md`
5.5, (153630, 115480) mm, its height left out here.

`footprints` holds one entry per footprint feature, sorted by `instance`: `instance` (its instance's
name), `kind`, `authority`, `layer`, `pieces` and `attrs`, as above.

`x` and `y` are exact metres from the lattice; `z` is the height of the surface beneath at that
point (`surfaces.md` 2.2). A formula attribute is written as its text.

A network's areas and a divided region's cells (R13, M3) are region features under their produced
names (`identity.md` 2.3). Their `kind` is (open) until `geometry.md` says how a network is
declared, decided by `quay` in M3.

### 9.2 Outputs

A kind's `output` is an id (`[a-z][a-z0-9_]*`) chosen by its library. The core gives it no meaning
and knows no value of it (principle 5). `outputs` in the manifest maps each output to the names of
the features of every kind that declares it, sorted. Two kinds with one output share one list. The
library documents what its output's features hold; a loader reads the outputs it knows and ignores
the rest.

Bookmarks are such an output: the review library's `bookmark` kind declares `output = "bookmark"`
and documents its attributes: `toward`, the place looked at, a name attribute (9.1); and
`eye_height`, a number of metres, 1.7 by the kind's default in `lib/review`, so `attrs` always holds
it for that kind. Turning a bookmark into a camera is the Godot loader's work (15.1). The spike's
compiler knew the words `bookmark` and `poi`; v0's core knows neither.

What the spike's `[look]` held that is a fact of the world (time of day, sun direction: doctrine
section 7) reaches a loader the same way, as a library output's attributes, passed through unread
(doctrine section 6). The rest of `[look]` is the loader's own settings; the build carries none.

### 9.3 Meetings

`meets.json` is `{ "meets": [ ... ] }`: one row per meeting, the columns and order of `surfaces.md`
2.4. Every meeting is a pair of mounts (R31); a path that ends at a place meets that place's default
mount. In M1 its rows are path ends meeting places on the terrain, the world's surface (`ford`);
kind mounts add rows from M3. A loader joins navigation only there.

### 9.4 The player map (M6)

The player map is the map library's: an export step writes it under `export/<step id>/`, and the
manifest's `exports` points to that folder. How a loader recognises it is by an output name the step
declares; the key for that is (open) in `protocol.md`, M6. The spike's `--map` folder is not
written by the core.

## 10. Generated meshes (M3)

A mesh the build makes (a kind surface, `surfaces.md` 5; a triangulated region, `geometry.md`
3.10) is a glTF 2.0 binary file, `meshes/<feature>.<surface>.glb`, with the escapes of 3.4.
`meshes` in the manifest lists each: `name`, `file`, `feature` (the feature whose surface it is),
`surface`, `collides`, `triangles`. `surfaces.md` says "owner" in prose only, since `owner` is a
layer's key; the format key is `feature`.

- Axes are glTF's (+Y up, right-handed, metres): glTF `(X, Y, Z) = (x, z, -y)`. This is a rotation,
  never a mirror.
- Vertices are relative to one node's translation: the mesh's south-west corner on the lattice. So
  a mesh far from the origin keeps millimetre precision in single-precision floats.
- One mesh, one primitive, indexed triangles, positions and normals; no images, no animation, no
  `extras` beyond `{ "via": { "feature", "surface" } }`. `asset.generator` is `"via"`, with no
  release number, so a release that keeps the epoch keeps the bytes. Written by our own writer,
  buffers in a fixed order, so the bytes are the same everywhere.
- Spacing of a surface's mesh and its material are (open), below.

## 11. Reports

### 11.1 Diagnostics: the world's and the run's

Two kinds of diagnostic, kept apart so the same text gives the same `diagnostics.json`:

| Kind | Codes | Where |
|---|---|---|
| World | Every node's diagnostics (`graph.md` 3.4), the reading steps' (`documents.md` 8), and the checks'. | `diagnostics.json`; the terminal; `--json`; `--diagnostics`. |
| Run | `identity.no_previous`, `identity.reselected`, `graph.build_file_restored`, `graph.store_corrupt`, `graph.store_unavailable`, `graph.step_unstable`, `graph.busy`, and every `build.*` code of this chapter except `build.chunk_not_multiple` and `build.internal`. | The run report (11.3); the terminal; `--json`; `--diagnostics`. Never `diagnostics.json`. |

- `diagnostics.json` is `diagnostics.md` 7's document, with world diagnostics only. Its `status`
  is the exit status the world diagnostics alone give.
- `--json` prints that document with the run diagnostics added in their sorted place and `status`
  set to the process's exit status. `diagnostics.md` 7, which calls the two "the same document", is
  asked to say so (Answers).
- `diagnostics.json` holds spans, which move when text moves, so it is not in the `id`. Built twice
  from the same text it is the same bytes.
- It is written whenever a build directory is known. When the world's `name` cannot be read (a
  syntax error, no `name`), the default `--out` falls back to `build/<world file stem>` beside the
  world file (section 14). When no directory can be written (`graph.busy`, `build.not_a_build`,
  `build.store_inside`), the diagnostics go to the terminal, `--json` and `--diagnostics` only.

### 11.2 `changes.json` and `.previous`

The change report of `identity.md` section 5, with the regions and causes of `graph.md` 6.4. It is
one canonical JSON object (3.1). `identity.md` 5 says what is matched and when an entry is added,
removed, moved, swapped or changed; this section fixes how it is written. Every member of the
report, an entry, a region, a cause and a summary row is always present: `null` where it does not
apply, `[]` where a list is empty. A position object is the one exception (below). Positions are exact
metres from the lattice; a height is a single-precision value widened to a double (3.1); a facing
is integer millidegrees.

| Key | Type | Holds |
|---|---|---|
| `against` | object or `null` | `{ "id", "key" }` of the baseline: its `id` (section 12) and its `run.key`. `null` without a baseline. |
| `entries` | list | One per thing that differs, sorted by `table`, then `name`, bytewise. `[]` without a baseline. |
| `format` | string | `"via-changes-0"`. |
| `summary` | list | One row per maker and table (below). `[]` without a baseline. |

**An entry** (`entries[]`):

| Key | Type | Holds |
|---|---|---|
| `name` | string | The thing's name: an id, a produced name, a footprint's instance name, or a field's name. |
| `table` | string | `feature`, `footprint`, `field` or `instance`. A guide is a `feature`. |
| `what` | list of strings | `["added"]`, `["removed"]`, or one or more of `moved`, `swapped`, `changed`, in that order. |
| `keys` | list of strings | For `changed`: what differs beyond position, facing and asset, sorted: a `features.json` key (`kind`, `authority`, `layer`, `points`, `pieces`), `attrs.<attribute>`, `scale`, `tint`, `facts`, or for a field `samples` and any key of its manifest entry that differs (`role`, `tags`, ...). `[]` otherwise. A path whose vertices change inside the same box is `changed` with `points`. |
| `before`, `after` | object or `null` | The thing in the baseline and in this build (the position object, below). `before` is `null` when added, `after` when removed; both are `null` for a field. |
| `facts` | list | For an instance or footprint whose asset's resolved facts differ (`assets.md` 7): `[{ "after", "before", "fact" }]`, sorted by `fact`, each value as `manifest.json` writes it (section 7). |
| `region` | object or `null` | For a field present in both builds: where its samples differ (below). `null` for anything else. |
| `causes` | list or `null` | Why it differs (below), sorted by `cause` bytewise. `null` when the baseline's build record is not in the store, so causes cannot be given (`graph.md` 6.4); never `[]` for an entry. |

**A position object** (`before`, `after`):

| Key | Type | For | Holds |
|---|---|---|---|
| `asset` | string | instance | Its asset id. |
| `at` | `[x, y, z]` | instance, point feature | An instance's pivot (section 8: `x`, `y` from integer millimetres, `z` after `sink`); a point feature's `at` (9.1). |
| `box` | `[x0, y0, x1, y1]` | feature, footprint | The bounding box in integer millimetres (`identity.md` 5), written in metres. A point's box has no area. |
| `face` | integer | instance | Facing, millidegrees. |
| `values` | object | any `changed` entry | `{ <key>: <value> }` for each of `keys` but `facts` and `samples`, as `features.json` or the manifest writes it. `{}` otherwise. |

A key that does not apply to the table is absent, not `null`; a reader keys on `table`.

**A region** (`region`): where a field's samples differ, from `graph.md` 6.4.

| Key | Type | Holds |
|---|---|---|
| `box` | `[x0, y0, x1, y1]` or `null` | The box, in metres, of every post that differs. `null` when only a manifest key differs. |
| `windows` | list | `[{ "chunk": [cx, cy], "window": [i0, j0, i1, j1] }]` in chunk order (5): for each chunk with a post that differs, the window of `numbers.md` 6 (`i1`, `j1` excluded, as in `graph.md`'s node ids) that holds every differing post of that chunk. |
| `posts` | integer | How many posts differ. A post shared by two chunks counts once. |
| `most` | number or `null` | The largest absolute difference at a post, in the field's unit; for a `weights` field, the largest byte difference in any plane. `null` when no post differs. |

A post differs when its bits differ; for a `weights` field, when any material's byte differs; for a
bounded field (M3), when its cover differs or a sample under cover 1 differs. When the two builds'
`frame`s differ (`world.extent` or `world.resolution` changed), every post of this build differs,
`windows` lists every chunk and `most` is `null`.

**A cause** (`causes[]`):

| Key | Type | Holds |
|---|---|---|
| `cause` | string | One of `identity.md` 5's forms: `world.<key>`, `<table>.<id>.<key>`, `<table>.<id>`, `asset.<asset>.<fact>`, `file:<path>`, `step:<id>`, `select:<change>`. |
| `file` | string or `null` | The source path (3.4) of the statement: for an edited key, the line that sets it; for a table, its header. `null` only for a cause with no line of text (an asset fact measured from a model file, M2). |
| `line` | integer or `null` | Its line, from 1. `null` when `file` is. |
| `in` | string | `"this"` when `file` and `line` are in this build's text, `"baseline"` when in the baseline's: a removed table or key. |
| `digests` | object or `null` | For `file:` and `step:`: `{ "after", "before" }`, the digests of the file, or of the step's command and code (R26); `null` on the side where it is absent. `null` for every other form. |
| `via` | list of strings | `graph.md` node ids: the shortest chain of edges from the node that reads the statement to the thing's nearest node, both ends included, ties by node id (`graph.md` 6.4). The thing's nodes are `shape:` and `attr:` for a feature, `instance:` or `held:` for an instance, `footprint:` for a footprint, and for a field the `field:` node of its first differing chunk. It is one chain, not every route. |

**A summary row** (`summary[]`), `identity.md` 5's line per maker:

| Key | Type | Holds |
|---|---|---|
| `maker` | string or `null` | The maker's id; `null` for things written in text. |
| `table` | string | The table counted. A maker filling two tables (a scatter's instances and their footprints) has a row for each. |
| `added`, `removed`, `moved`, `swapped`, `changed` | integer | Entries with that word in `what`. An entry with two words counts under each. |
| `untouched` | integer | Things of that maker and table in both builds with no entry. |
| `box` | `[x0, y0, x1, y1]` or `null` | The box, in metres, of every `before` and `after` `box` or `at`, and every `region.box`, of the row's entries. `null` when there are none. |
| `line` | string | `identity.md` 5's fixed form: `<label>: +<added> -<removed> ~<moved> swapped <swapped> changed <changed>, <untouched> untouched`, where `<label>` is the maker's id, or `<table> (written)` for a row of written things. |

There is one row for each maker and table with a thing in either build, entries or not, so a build
against an identical baseline lists every row with zeros (`identity.md` case 42). Rows are sorted by
`maker`, `null` first, then by `table`.

**Example.** `ford`, then `ford` with `downstream`, both with `--out build/ford`. Two of the entries and the whole summary are
shown, spread over lines for reading; the file is one canonical line. The numbers are illustrative;
the M1 build records the real report as a vector.

```json
{
  "against": {"id": "3f1c…", "key": "9a07…"},
  "entries": [
    {"after": {"at": [144.833, 56.394, 22.75], "box": [144.833, 56.394, 144.833, 56.394],
               "values": {"attrs.toward": {"at": [155.734, 51.606, 22.25], "name": "ford"}}},
     "before": {"at": [142.874, 115.805, 27.375], "box": [142.874, 115.805, 142.874, 115.805],
                "values": {"attrs.toward": {"at": [153.63, 115.48, 26.5], "name": "ford"}}},
     "causes": [{"cause": "feature.ford.on", "digests": null, "file": "world.toml", "in": "this",
                 "line": 74, "via": ["shape:ford/v:build", "attr:at_ford"]}],
     "facts": [], "keys": ["attrs.toward"], "name": "at_ford", "region": null,
     "table": "feature", "what": ["moved", "changed"]},
    {"after": null, "before": null,
     "causes": [{"cause": "feature.ford.on", "digests": null, "file": "world.toml", "in": "this",
                 "line": 74, "via": ["shape:ford/v:build", "field:height/v:build/w:256.0.385.129"]}],
     "facts": [], "keys": ["samples"], "name": "height",
     "region": {"box": [141, 35, 164.5, 125], "most": 1.25, "posts": 3517,
                "windows": [{"chunk": [2, 0], "window": [282, 70, 330, 129]},
                            {"chunk": [2, 1], "window": [286, 128, 322, 251]}]},
     "table": "field", "what": ["changed"]}
  ],
  "format": "via-changes-0",
  "summary": [
    {"added": 0, "box": [0, 51.606, 256, 140], "changed": 3, "line": "feature (written): +0 -0 ~6 swapped 0 changed 3, 1 untouched", "maker": null, "moved": 6, "removed": 0, "swapped": 0, "table": "feature", "untouched": 1},
    {"added": 0, "box": [138.5, 33, 168, 126], "changed": 3, "line": "field (written): +0 -0 ~0 swapped 0 changed 3, 0 untouched", "maker": null, "moved": 0, "removed": 0, "swapped": 0, "table": "field", "untouched": 0},
    {"added": 0, "box": [80.1, 44.2, 101.9, 77.8], "changed": 0, "line": "footprint (written): +0 -0 ~1 swapped 0 changed 0, 0 untouched", "maker": null, "moved": 1, "removed": 0, "swapped": 0, "table": "footprint", "untouched": 0},
    {"added": 0, "box": [85.1, 49.2, 96.9, 72.8], "changed": 0, "line": "instance (written): +0 -0 ~1 swapped 0 changed 0, 0 untouched", "maker": null, "moved": 1, "removed": 0, "swapped": 0, "table": "instance", "untouched": 0},
    {"added": 5, "box": [76, 40, 106, 104], "changed": 0, "line": "trees: +5 -3 ~0 swapped 0 changed 0, 412 untouched", "maker": "trees", "moved": 0, "removed": 3, "swapped": 0, "table": "instance", "untouched": 412}
  ]
}
```

`at_ford` stands on `lane_west`, which ends at the ford, so it moved; its `toward` names the ford,
so it changed too. Its one cause reaches it by the shorter of two chains: through its name
attribute, not through `lane_west`. In the summary, `at_ford` and `to_door` count under both
`moved` and `changed`; `beck` is the one untouched feature.

- **`.previous`** is canonical JSON, `{"format":"via-previous-0","id":…,"manifest":…}`: the `id`
  and the manifest digest of the last complete build written to this directory, or `null` for
  both when there has been none. Every build writes it. A failed build writes the last complete
  build's values unchanged, so a failed build in between loses nothing.
- **The store keeps every build.** Every build puts `manifest.json` and every file of `files` in
  the store (`graph.md` 3.2) as objects named by digest. The baseline is the manifest `.previous`
  names, found in the store, and its files by their digests.
- `--against DIR` names another build directory as the baseline instead, read from that directory,
  not the store. A `DIR` without a manifest of a known `format` is `build.against_not_build`
  (error); nothing is built.
- Without a baseline, `against` is `null`. When `.previous` names none, `identity.no_previous`
  (info) says so; when it names one the store no longer holds (the store deleted, or another
  `--store`), `build.baseline_missing` (warning) says so.
- **Across change sets.** The default `--out` differs per change set (section 14), so a change
  report from one change set to another needs a shared `--out` or `--against`.
- Not written when the build has an error (`diagnostics.md` 4).

### 11.3 The run report

`graph.md` 6.3 fixes its contents. It is written beside the directory, as `<directory>.run.json`
(`build/ford.run.json`), never in it, because times differ between runs (`numbers.md` 9). It adds
`threads`, the operating system and architecture, the run diagnostics (11.1), and for each step the
record R26 asks for: its command, exit status, and the digests of what it read and returned.

### 11.4 Export steps (M6)

An export step (R22) reads the finished build (R29) and writes only under `export/<step id>/`
(`protocol.md` hands it the folder). Its
files are listed in `files` with digests and are in the `id`: what the step wrote is part of what a
reader is handed.

- What it writes is the step's: R26 records a user's step and does not promise it. `via check`
  checks that its files are listed and match their digests, and nothing else about them: not
  canonical JSON, not paths (section 14).
- An export step is a `step` node like any other (`graph.md` 5). Its instability is found as
  `graph.md` 5.2 finds any step's: when it runs again for a trace the store already holds a result
  for, and gives other bytes. Nothing else detects it. `graph.step_unstable` is then a run
  diagnostic, and the `id` follows the new bytes.

## 12. The identity of a build

The `id` answers one question: does this directory hand a reader the same world? It is the BLAKE3
digest of one canonical JSON object: the manifest with `id`, `run` and `locations` removed. That
object lists every file in `files` by digest, so the `id` covers exactly those files and the
manifest's world keys. This is the one rule; `via check` recomputes it (section 14).

| Left out | Why |
|---|---|
| `run.version` | A release that keeps the numerics epoch keeps every byte (`graph.md` 3.3); `engine.epoch` is in. |
| `run.key`, `run.inputs` | They digest the source text: a reordered table or a comment changes them, not the world. |
| `run.changes`, `run.flags` | They say how the world was asked for. A change named or given as a file gives one world; `--stand-ins` shows in `assets.stand_in`. |
| `diagnostics.json` | Its spans move with the text (`graph.md` 3.4). |
| `changes.json` | It depends on the baseline. |
| `locations` | It depends on where the directory was written (`--out`). |
| `.previous`, the run report | About the run, not the world. |

So two builds with the same `id` hand an engine the same world, and two builds of one world, asked
for in different words, share an `id`. Moving a build directory or writing it under another name
keeps its `id`.

**The null vector (M0).** `spec/vectors/build/null/world.toml` is:

```toml
name       = "null"
extent     = [0, 0, 2, 2]
resolution = 1

[field.height]
role = "surface"
base = "= 0"
```

M0 records its `manifest.json` and `id` beside it, as `manifest.json` and `id.txt`. A later change
to either without a new `format` or `epoch` fails `graph.md`'s epoch guard.

## 13. Writing and replacing a build

- **A lock.** One build at a time per directory: `via build` takes an operating-system advisory
  lock on `<directory>.lock` beside the directory (`flock` on macOS and Linux, `LockFileEx` on
  Windows), released when the process ends, killed or not. The file stays; it is never inside the
  directory. A lock held by another process is `graph.busy`.
- **Written beside, then swapped.** Under the lock, the build is written into a new sibling folder,
  `<directory>.partial`, `.previous` first, then swapped in by rename: the old directory to
  `<directory>.old`, the new one into place, the old one deleted. A reader never sees half a build.
- **An unchanged rebuild writes nothing.** When the build key equals `run.key`, every file of
  `files` and `run.reports` matches its record, and no `--against` is given, no node is touched
  (`graph.md` 6.2) and the directory is left as it is, `changes.json` included. When a file
  differs, the whole directory is written again by the swap above, the files copied from the store
  into the partial folder, with `graph.build_file_restored` naming each file that differed. No file
  is patched in place.
- **Recovery.** At start, under the lock: if the directory is missing and `<directory>.old` was
  written by `via`, it is renamed back; then any `<directory>.partial` and `<directory>.old` written
  by `via` are deleted. A build killed between the two renames therefore loses nothing.
- **Written by `via`** means a folder holding a `manifest.json` that parses with a `format` starting
  `via-build-`, or a `.previous` of the form in 11.2. Nothing else is deleted: an `--out`, a
  `.partial` or an `.old` that is a file, or a non-empty folder not written by `via`, is
  `build.not_a_build` (error), naming it. So is an `--out` that is the world file's folder or holds
  it.
- **With an error,** the directory is replaced by one holding `diagnostics.json` and `.previous`
  only (`diagnostics.md` 4): nothing a loader reads describes other text as if current. A loader
  that finds no manifest says so and points to `diagnostics.json`.
- **On Windows** a file open in another program (a loader holding a mapped field) blocks the rename.
  That is `build.dir_busy` (error), naming the file. Nothing is half replaced; the partial folder is
  kept for inspection and deleted by the next build. R20's recorded failures were Windows ones; this
  is designed for.
- **A failed write** (disk full, permission refused, a rename refused on any system) is
  `build.write_failed` (error), naming the path and the system's error. The directory is left as it
  was.
- **The store** (`graph.md` 3.2) is `.via/store/` beside the world file unless `--store DIR` names
  another. A store that resolves inside the build directory is `build.store_inside` (error).

## 14. Commands

This section is the one list of `via`'s commands and flags. Conformance runs through the command
line only (R20), so a command another chapter's cases use is listed here, with the chapter that
owns what it does. A form that chapter has not settled is marked (open). Exit status is
`diagnostics.md` 7 for every command unless its owner says otherwise; status 2 is a `cli.*` fault.

| Command | Does | Owner | Milestone |
|---|---|---|---|
| `via build WORLD` | Builds. Writes the directory, the run report and the store. | this chapter; `graph.md` | M1 |
| `via merge WORLD` | Prints the merged document in canonical JSON (`documents.md` 8). | `documents.md` 8 | M1 |
| `via lock WORLD LOCK` | Builds, then writes the record of `[lock.LOCK]` (`identity.md` 4.4). | `identity.md` 4.4 | M3 |
| `via check DIR` | Reads a build directory alone and writes nothing into it. Below. | this chapter | M1 |
| `via explain [CODE] [WORLD]` | Prints a code's registry row; with no code, every code by group; `step.<id>.<name> WORLD` prints a step's text from the last build. | `diagnostics.md` 2 | M1 |
| `via explain NODE WORLD` | Prints a node's cause in the last build and its chain of causes back to a leaf, from the run report beside the world's default build directory, or `--out`'s. An argument holding `:` is a node id; a code never holds one. A node the last build did not hold is `cli.usage`, naming the nearest node id. | `graph.md` 6.3 | M1 |
| `via pack check PACK` | Checks a pack without a world; writes nothing. | `assets.md` 8.1 | M1 |
| `via pack list PACK` | Prints each asset's resolved facts as JSON Lines. | `assets.md` 8.2 | M1 |
| `via pack accept PACK` | Writes the pack's facts file. | `assets.md` 4.3 | M2 |
| `via query WORLD Q --in FILE --out FILE` | Answers one of the surface questions (`surfaces_at`, `sample`, `relief`, `meetings`) over the whole world. | `surfaces.md` 7 | M1 |
| `via trust WORLD` | Lists the world's steps and their trust digests, and records trust. | `protocol.md` 6.3, 7 | M1 |
| `via config program NAME PATH` | Records a named program's path in the user's `config.toml`. | `protocol.md` 6.1, 6.3 | M1 |
| `via eval WORLD --field F [--window i0,j0,i1,j1] [--tile N] --out FILE` | Evaluates one scalar field in the build's own view, over one window or the whole extent, and writes it as an array file. Below. | `numbers.md` 6 | M1 |
| `via vectors FILE` | Computes every vector of a vector file with the binary's own numerics and prints what it got. Below. | `numbers.md` 10 | M1 |
| `via kernel OP --in FILE --out FILE` | Runs one geometry operation on the shapes in a file; a test command, no build uses it. | `geometry.md` 7 | M1 |
| `via --version` | Prints the engine version and exits 0. | this chapter | M0 |
| `via help` | Prints the commands and exits 0. | this chapter | M0 |

The glTF reader is not a `via` command: it lives under `engines/gltf/` (15.3).

Flags of commands other than `build`, `merge`, `lock`, `check`, `eval` and `vectors` are their
owner's and are listed there: `via pack list`'s `--taxon` and `--tag` (`assets.md` 8.2), `via trust`'s `--change`,
`--yes` and `--json` (`protocol.md` 6.3), `via query`'s `--in` and `--out` (`surfaces.md` 7),
`via explain`'s `--json` (`diagnostics.md` 2), `via kernel`'s `--in` and `--out` (`geometry.md` 7).
`--diagnostics` is on every command.

**Where diagnostics go.** Every command prints its records to standard error (`diagnostics.md` 7).
`--diagnostics FILE` also writes `diagnostics.md` 7's `--json` document to `FILE`: world and run
diagnostics, `status` the process's exit status, written on every exit status, 2 included. It is
honoured wherever it appears among the arguments, as `--json` is. It is the one channel for a
command whose standard output is its result: `via merge`, `via pack list` and `via vectors`, on
which `--json` is `cli.usage`, naming it, with the hint `--diagnostics`. `via merge` has no build
directory and writes no `diagnostics.json`. A `FILE` inside `--out` is `cli.usage`: the build
directory is replaced whole. A `FILE` that cannot be written is `build.write_failed`, on standard
error only. The conformance runner passes `--diagnostics` to every `via` action and reads both
kinds of record from it (`conformance.md` 3.5).

Flags, on `build`, `merge` and `lock` where they apply:

| Flag | Meaning |
|---|---|
| `--origins` | On `merge`: prints, beside the merged document, each value's span and chain (`documents.md` 8). |
| `--lowered` | On `merge`: prints the document after forms are lowered (`rules.md` 10). (open): whether `merge` prints this, or the lowered statements are read from the graph, is decided with `rules.md`'s first form in M3. |
| `--rerun-step ID` | On `build`: runs that step even when its trace matches, and compares (`graph.md` 5.2). Repeatable. Not in the build key: it changes no output unless the step is unstable, which `graph.step_unstable` reports. |
| `--step-deadline SECONDS` | On `build`: how long a step call may run before it is cancelled; default 600 (`protocol.md` 6.2). Not in the build key. |
| `--sdk DIR` | On `build`, `lock`, `eval`, `query`, `merge` and `trust`: a working copy of the SDK in place of the carried one; its digest is in every step's static key and trust digest (`protocol.md` 8.1). |
| `--change C` | Request a change. Repeatable; order changes nothing (changes commute). `C` holding a `.` or `/` is a change file's path, otherwise a change name: ids hold neither (`vocabulary.md` 2.1), so the two never mix. |
| `--out DIR` | The build directory. Default: `build/<name>` beside the world file, with `+<change>` appended for each requested change, sorted (a file by its stem). When `name` cannot be read, `<name>` is the world file's stem. |
| `--lib DIR` | A library root (`documents.md` 4), searched in the order given. Repeatable. |
| `--threads N` | Threads (`graph.md` 8). Default: the machine's. In no key and no build file. |
| `--store DIR` | The store (13). |
| `--against DIR` | The change report's baseline (11.2). |
| `--stand-ins` | (M2.) Build every model asset as its accepted stand-in (`assets.md` 9). Needs the facts file; without it, `build.no_facts` (error). In the build key and `run.flags`. |
| `--fail-on CODE` | Makes this warning code an error for this run (exit 1). Repeatable. For unattended checks, such as `--fail-on asset.revised` (M2, `assets.md` 4). In the build key and `run.flags`. A name that is not a registered code is `cli.unknown_code` (exit 2), naming the nearest code. A registered code that is not a warning is `cli.usage` (exit 2), naming the nearest warning code. A step code is accepted as written (`diagnostics.md` 3). A world cannot do this (`diagnostics.md` 3). |
| `--json`, `--info` | `diagnostics.md` 7, with 11.1. `--json` is refused where standard output is the result (above). |
| `--diagnostics FILE` | On every command: above. |

**`via eval WORLD --field F [--window i0,j0,i1,j1] [--tile N] --out FILE`** takes `--change`,
`--lib`, `--sdk`, `--json` and `--diagnostics` as `build` does, and no other flag. It is the one
way to test windowed evaluation (principle 10).

- `--window` is a window of `numbers.md` 6 in posts: `i0`, `j0` included, `i1`, `j1` excluded, as
  in `graph.md`'s node ids. Without it, the whole extent. An empty window, or one not inside the
  grid, is `cli.usage`.
- It writes `FILE` as an array file (3.2): `<f4`, shape `(j1 - j0, i1 - i0)`, row 0 the window's
  south row. Its bytes are what the build would write for those posts.
- It uses no store and writes no build directory, run report or `.previous`: every node it needs
  is computed, so a cached value cannot hide a difference. Without `--tile` it is evaluated as one
  window with its halo, never cut by chunks.
- `--tile N`, `N` at least 1: every `field` and `distance` node it computes is evaluated in windows
  of `N` by `N` posts, tiled from the window's south-west post, the last in each direction
  narrower, each with its halo (`numbers.md` 6). It changes no byte of `FILE` or of the
  diagnostics. `formulas.md` cases 45 and 46 compare `--tile 1` with `--tile 16`.
- Its world diagnostics are those of the nodes it computed, by `formulas.md`'s rule of one record
  per formula and owner, whatever the windows; `--diagnostics` writes them.
- `F` must be a scalar field; a `weights` field, or a name that is no field, is `cli.usage`, naming
  the nearest scalar field. A world with an error gives the build's diagnostics, exit 1, and no
  `FILE`.

**`via vectors FILE`** reads a vector file of `numbers.md` 10's form (`spec/vectors/numbers.toml`)
and computes each vector through `via-num`, the code the build uses. It judges nothing: it prints
one canonical JSON object per line to standard output, in the file's order,
`{"got":<value>,"id":<the vector's key>}`. A 64-bit result is written as `numbers.md` 10 writes it,
`0x` and 16 upper-case hex digits; a vector with several results gives a list of them. The runner
compares `got` with the file's expected bits. A file that does not parse is `doc.*`, exit 1; a
vector naming a function the binary lacks is `cli.usage`, naming the vector, exit 2.

**`via check DIR`** reports to standard error, or with `--json` to standard output as
`diagnostics.md` 7's document. Exit 0 with no error, 1 with any, 2 for a bad argument (`cli.*`).
Each failure is a `check.*` error naming the file, and the byte offset, key path or row, with a
hint (Diagnostics, below). It checks:

1. `manifest.json` exists, is canonical, and has a known `format`.
2. `id` equals the digest recomputed by the rule of section 12.
3. Every file of `files` and `run.reports` exists, with its `bytes` and `digest`; no other file is
   present but `manifest.json` and `.previous`.
4. Every array file's header is the subset of 3.2, its size matches its header, and its `descr` and
   `shape` match what the manifest says it holds: a scalar field `<f4` `(ny, nx)`; a cover `|u1`
   of the field's shape; a weights chunk `|u1` `(len(materials), rows, columns)` of its chunk's
   posts; every instance column `(count,)`.
5. Every float is finite and no zero is negative.
6. Every post of a weights chunk sums to 255 or 0.
7. `names.txt` has `count` lines, each name unique and of the alphabet of section 8.
8. Every `face` lies in `[0, 360000)`.
9. The chunks' `[first, count]` ranges tile the instance rows in order, and each row's `x`, `y` lies
   in its chunk (5).
10. Every JSON and text file the build writes is canonical and holds no absolute path. Files under
    `export/` are the step's and are checked by 3 only.

## 15. Readers

Three readers keep the format honest. Each reads only the directory and the files the manifest names
under `pack_root`; none reads world text. None is part of the core: the core knows no output name
(principle 5), and the deny-list test (`vocabulary.md` 7) covers the core crates only.

### 15.1 Godot 4 (the loader, `engines/godot/`)

Godot is right-handed with +Y up and -Z forward: Godot `(X, Y, Z) = (x, z, -y)`, and a bearing `b`
is a turn of `-b` about +Y. The loader makes one node tree; the editor saves it as a scene, the
review viewer makes it at start. Both are the same code, so compile-time export and runtime loading
differ only in when the tree is saved. The Godot release is pinned in `engines/godot/`, and the
runner uses that one.

Options are the loader's own settings, never world text. Field options take the vocabulary's field
selectors: a name, `role:<role>` or `tag:<tag>`.

| Option | Meaning |
|---|---|
| `fields` | Fields to keep as data in the root's metadata. |
| `water` | One selector naming a water surface height field (6.4) to draw as water. |
| `features` | Make nodes for features (below). |
| `diagnostics` | A path: write the loader's diagnostics there as `diagnostics.md` 7's document, for unattended runs. Without it, they go to Godot's output. |

| Build | Becomes in Godot |
|---|---|
| `manifest.json` | A `Node3D` named after `world`, holding `id` and `format` as metadata. `format` unknown or no manifest: `load.format`, naming `diagnostics.json`. |
| A chunk | A `Node3D` `c_<cx>_<cy>` under it. Streaming chunks in and out is the engine's (M6). |
| The `surface` field | Per chunk: a `MeshInstance3D` with an `ArrayMesh` over the chunk's posts, each quad split from its south-west to its north-east post; a `StaticBody3D` with a `CollisionShape3D` holding a `HeightMapShape3D` of the same posts. `HeightMapShape3D` spaces samples 1 unit apart, so its heights are divided by `resolution` and its node is scaled uniformly by `resolution`, centred on the chunk. |
| A `material` weights field | Per chunk: a `ShaderMaterial` on that mesh, its planes as one `Texture2DArray` (format `R8`, one Godot texture layer per material), and each material's `colour`, `file` and `repeat` from `materials`. The shader loops over the chunk's count; there is no fixed number. A chunk naming more materials than the loader draws is `load.too_many_materials`, naming the chunk and its materials (`fields.md` 7.1). |
| The field the `water` option names | Per chunk: a `MeshInstance3D` of the water surface over the posts where depth is above 0 (under cover 1 for a bounded field), in the loader's water material. Not collided. |
| A bounded field (6.3) | Not drawn; a kind surface is drawn from its mesh. Kept as data when `fields` names it. |
| Any other field | Not drawn. Taken as an `Image` (`FORMAT_RF`) in the root's metadata when `fields` names it. Every field neither drawn nor kept is listed once in `load.fields_not_drawn` (info). |
| Instances of one asset in one chunk | A `MultiMeshInstance3D` named after the asset id, its `MultiMesh` holding one transform per row and `tint` as instance custom data. The mesh comes from the asset's glTF through `GLTFDocument`, made with `to_asset`, which takes the file's node space as stored; a primitive or stand-in is made by the loader from `assets.md` 6. |
| Collision of an instance | Read from the asset's `params` by the loader's convention (`assets.md` 2.2): `box` from `bounds`, `hull` a `ConvexPolygonShape3D`, `mesh` a `ConcavePolygonShape3D`, each on a `StaticBody3D` named after the instance. No `params.collision`, no body. A `passable` asset (M5, `assets.md` 10) will get no blocking body. |
| An instance name | The node name where the instance has a node. `.` becomes `-`, since a Godot node name may not hold `.`; names never hold `-`, so the mapping reverses. `#` and `_` are kept. Names in `names.txt` keep each `MultiMesh` row findable. |
| Re-import (M6) | Nodes are matched by name: a name in both builds keeps its node and its editor-set properties the loader does not own; a name gone removes its node. |
| A feature | Nothing by default. With `features`: a `Path3D` with a `Curve3D` for a path, a `Marker3D` for a point, a `Polygon` in metadata for a region; `attrs` as metadata. |
| The review library's `bookmark` output | A `Camera3D` under a `bookmarks` node, named after the bookmark (as an instance name, below): the eye at the feature's `at` raised by `attrs.eye_height` metres (9.2), looking at `toward`'s `at` (9.1), with +Z of the world up. The loader reads `at` and never resolves a name. A `toward` that is missing or not of the form `{ "at", "name" }`, or whose `at` lies within 1 mm of the eye in plan, or an `eye_height` that is missing or not a finite number, is `load.bad_target`; that bookmark is not made, and no default is guessed. The sheet tool renders each (below). Other outputs: not read. |
| A generated mesh (M3) | A `MeshInstance3D` from its `.glb`; with `collides`, a `StaticBody3D` with a `ConcavePolygonShape3D`. |
| A meeting | A `NavigationLink3D` between its two mounts. |
| `diagnostics.json`, `changes.json` | Not read. The viewer may show them; the scene never holds them. |

**The place a bookmark looks at** is resolved by the core (`authority.md` 1.2) and written as
`toward`'s `at` (9.1). In `ford`, `at_ford` and `on_bank` look at `ford`, `to_door` ("from the lane
to the door") at `cottage`, the instance, and `in_trees` at `lane_east.end`, as `docs/plan.md`
writes them. An instance's point is its pivot (section 8), which `authority.md` 1.2 is asked to
state (Answers). A view of the door itself would name a mount, `cottage.door`; `ford` does not, so
M1 needs no `door` mount on `house_low`.

**The sheet tool** renders every bookmark of one build, or of two builds before and after, and lays
them out on one sheet. It is the loader's code run by Godot, not a `via` command:

```
godot --path engines/godot --rendering-method gl_compatibility res://sheet.tscn -- \
      --before DIR [--after DIR] --out DIR [--pack DIR] [--option NAME=VALUE ...]
```

- `--option` sets one loader option of the table above, repeatable; a list's items are separated
  by `,`, `features` takes `true` or `false`. `diagnostics` is not taken: the tool writes its own
  (below). An unknown name or a bad value is exit 2. `ford`'s viewer settings are exactly
  `--option water=tag:water_level`, the tag the water library documents; the Godot loader suite and
  the milestone sheet pass it.
- `--before` and `--after` are build directories (`build/ford` and `build/ford+downstream`).
  `--pack` names the pack's root when a manifest's `pack_root` is `null`. `--out` is written whole,
  like a build (section 13); a non-empty folder the tool did not write is refused.
- Godot's `--headless` display draws nothing, so the tool runs with a display: a desktop, or on
  Linux a virtual frame buffer (Xvfb). The Compatibility renderer is used because it runs on both.
- Each bookmark is rendered at **1280 by 720** pixels to `<out>/before/<bookmark>.png` and
  `<out>/after/<bookmark>.png`, the file named as the bookmark's node (an instance name's mapping,
  above: `.` becomes `-`). The camera is Godot's `Camera3D` with its defaults kept: 75 degrees
  vertical field of view; near 0.05 m; far the extent's diagonal plus 100 m. One
  `DirectionalLight3D` from bearing 225 degrees, 45 degrees above the horizon, and one environment,
  the same for both builds, so a pair differs only by the build.
- `<out>/sheet.png` is the sheet. A header band 48 pixels high holds the world's name, the first 12
  hex digits of each build's `id`, and the Godot release. Then one row per bookmark, in name order
  over both builds: a label band 24 pixels high (the bookmark's name, its eye point, its target's
  name), then the before view and the after view side by side, each scaled to 640 by 360. So the
  sheet is 1280 pixels wide and `48 + 384 * rows` high: 1584 for `ford`'s four. Without `--after`
  it is one column, 640 wide. A bookmark in one build only has a grey cell (`#808080`) marked
  "absent" in the other.
- `<out>/sheet.json` is the protocol beside the images, canonical JSON:
  `{ "after", "before", "bookmarks", "camera", "godot", "light", "renderer", "size" }`. `before`
  and `after` are `{ "id", "world" }` (`after` `null` without one); each of `bookmarks` is
  `{ "after", "before", "eye", "name", "target" }`, with the two image paths (`null` where absent),
  the eye point and `toward` as written.
- The loader's diagnostics go to `<out>/diagnostics.json` in `diagnostics.md` 7's form. Exit 0 when
  every bookmark was rendered, 1 with any error, 2 for a bad argument.
- Images are not compared byte for byte: renderers differ between machines. The Godot loader suite
  checks their size and that they are not one colour; a person or an agent reads them.

The milestone sheet (`docs/plan.md`: plan, eye-height views, numbers) takes its eye-height views
from this tool; the plan image and the numbers are made apart from it.

### 15.2 Unreal (on paper)

Set aside until after the first cut (doctrine section 7, challenge 41). Written so the format does
not set around Godot; nothing here is built or tested. Unreal is left-handed, Z up, centimetres:
Unreal `(X, Y, Z) = (100x, -100y, 100z)`. A world transform `M` becomes `S M S` with
`S = diag(1, -1, 1)`, then scaled, so a bearing turns the right way in the mirrored frame.

| Build | Would become in Unreal |
|---|---|
| A chunk | A World Partition grid cell of the chunk's side. |
| The `surface` field | A Landscape, from the posts resampled to the Landscape's component size (sections of 63 or 127 quads); the height range scaled to 16 bits. A resample is the importer's, with its error reported. |
| A `material` weights field | Landscape paint layers (the engine's word), weight-blended, one per material; each chunk's `materials` list is the set of layers its components use, which Unreal limits per component for cost. |
| Instances of one asset in one chunk | A Hierarchical Instanced Static Mesh component per chunk and asset. |
| An instance name | The instance's tag. Re-import matches by tag: moves what moved, adds what is new, removes what is gone, and keeps what a designer set on the actor. |
| An asset | A Static Mesh imported from its glTF; the collision convention in `params` as its simple collision (`box`, `hull`) or complex-as-simple (`mesh`). |
| A generated mesh | A Static Mesh actor. |
| A feature | A Spline component for a path, a Volume for a region, an empty actor for a point; `attrs` as tags or a data asset. |
| An output the importer knows (the review library's bookmark) | A Camera actor. |
| A meeting | A Nav Link Proxy. |

### 15.3 glTF export (the second reader, `engines/gltf/`)

A script under `engines/gltf/`, in Python with its standard library only, writes the whole build as
one glTF 2.0 scene: the check that the directory alone describes the world. It is written apart
from the Godot loader and uses none of its terms. It reads only what this chapter defines; a fact it
needs that is not here is a defect of this chapter, not of the reader. It knows no output name.

| Build | Becomes in glTF |
|---|---|
| A chunk | A node `c_<cx>_<cy>`. |
| The `surface` field | A mesh per chunk, quads split as in 15.1. |
| A `material` weights field | Vertex colour `COLOR_0`: each material's `colour` weighted by its plane, divided by 255. A post with every byte 0 takes `[0.8, 0.8, 0.8]`, `assets.md`'s default colour. Lossy; textures are not carried. |
| An instance | A node named after it, its transform from section 8, mapped by `(x, z, -y)`. A primitive or stand-in is its `assets.md` 6 shape in `colour`, made by the reader; a model's meshes are copied from its glTF (M2). |
| A feature | A path as a `LINE_STRIP` mesh, a region's rings as `LINE_LOOP`s, a point as an empty node; `attrs` in `extras`. |
| A feature named in `outputs` | Also an empty node under a node named after the output, `attrs` in `extras`. No output becomes a camera; a reader that wants one is given the output name and attribute names as options. |
| A generated mesh (M3) | Copied in. |

The output passes the Khronos glTF Validator with no error, at the release pinned in the runner and
run offline from a local copy.

## Worked example

```toml
name       = "tiny"
extent     = [0, 0, 128, 64]
resolution = 2
include    = ["land"]
pack       = "primitive"

[field.height]
role = "surface"
base = "= 10 + 0.05 * x"

[field.cover]
role = "material"
type = "weights"

[field.cover.then.grass]
paint = "grass"

[instance.hut]
asset = "box_small"
at    = [100, 30]
face  = 90
```

65 by 33 posts; chunk side 64 m, so `k = 32` and 2 by 1 chunks: `(0, 0)` holds posts `i` 0 to 32,
`(1, 0)` holds 32 to 64, both `j` 0 to 32; column 32 is in both. Their field nodes have windows
`w:0.0.33.33` and `w:32.0.65.33`. `via build tiny/world.toml` writes:

```
tiny/build/tiny/
  manifest.json  diagnostics.json  changes.json  .previous
  fields/height.npy                      128 + 65 * 33 * 4 = 8708 bytes
  weights/cover/0_0.npy, 1_0.npy         128 + 1 * 33 * 33 = 1217 bytes each
  instances/x.npy y.npy z.npy face.npy scale.npy tint.npy names.txt
  features.json  meets.json
```

The hut is at `x = 100000` mm, `floor(100000 / 64000) = 1`, so in chunk `(1, 0)`; `face.npy` holds
`90000`; `z.npy` holds `15` (`10 + 0.05 * 100`); `names.txt` holds `hut` and an LF. The manifest's
chunk entry for `(1, 0)` is:

```json
{"at":[1,0],"count":1,"instances":{"box_small":[0,1]},"posts":[32,0,64,32],"triangles":12,"weights":{"cover":{"file":"weights/cover/1_0.npy","materials":["grass"]}}}
```

Every byte of both weights files after the header is 255. In Python,
`numpy.load("fields/height.npy", mmap_mode="r")[0, 64]` is `16.4` in single precision: the
south-east post, `10 + 0.05 * 128`.

## Diagnostics

Kind: W a world diagnostic, R a run diagnostic (11.1).

| Code | Severity | Kind | Says |
|---|---|---|---|
| `build.chunk_not_multiple` | error | W | `chunk` is not a whole multiple of `resolution`; the hint gives the nearest that are. (M6.) |
| `build.internal` | error | W | A value reaching the writer is not finite, a weight is below 0, or a file failed its own check after writing; names the file and the post or row. A defect of ours; exit 3. |
| `build.dir_busy` | error | R | The build directory could not be replaced because a file in it is open; names the file. Hint: close the program holding it, or build to another `--out`. |
| `build.not_a_build` | error | R | `--out`, `<out>.partial` or `<out>.old` is a file, a non-empty folder `via` did not write, or the world file's own folder; nothing was deleted. Hint: name an empty or new folder. |
| `build.write_failed` | error | R | A file or rename could not be written; names the path and the system's error. Hint: free space, or fix the folder's permissions. |
| `build.store_inside` | error | R | The store resolves inside the build directory, which every build replaces. Hint: drop `--store`, or name a folder outside `--out`. |
| `build.against_not_build` | error | R | `--against` names a folder with no manifest of a known `format`. Hint: name a complete build directory. |
| `build.baseline_missing` | warning | R | `.previous` names a build the store no longer holds; the change report has no baseline. Hint: rebuild the earlier text, or pass `--against`. |
| `build.pack_root_far` | warning | R | No relative path leads from the build directory to the pack's root (another drive); `pack_root` is `null`. Hint: build onto the pack's drive, or tell the loader where the pack is. |
| `build.no_facts` | error | R | `--stand-ins` was asked for and the pack has no facts file. Hint: run `via pack accept`. (M2.) |
| `check.format` | error | | No `manifest.json`, not canonical, or an unknown `format`. |
| `check.id` | error | | `id` is missing or differs from the digest recomputed by section 12; gives both. |
| `check.missing` | error | | A listed file is absent. |
| `check.digest` | error | | A listed file's `bytes` or `digest` differs from its record; gives both. |
| `check.unlisted` | error | | A file is present that no list names. |
| `check.header` | error | | An array file's header is outside the subset of 3.2, or its size does not match it; gives the offset. |
| `check.shape` | error | | An array file's `descr` or `shape` differs from what the manifest says it holds; gives both. |
| `check.value` | error | | A float that is not finite or is `-0`; gives the file and the index. |
| `check.weights_sum` | error | | A post's weights sum to neither 255 nor 0; gives the chunk and post. |
| `check.names` | error | | `names.txt` has other than `count` lines, a repeated name, or a character outside section 8's alphabet; gives the line. |
| `check.face` | error | | A `face` outside `[0, 360000)`; gives the row. |
| `check.chunk_rows` | error | | Chunks' `[first, count]` ranges leave a gap or overlap, or a row lies outside its chunk; gives the chunk and row. |
| `check.not_canonical` | error | | A JSON file the build writes is not canonical; gives the byte offset of the first difference. |
| `check.absolute_path` | error | | A JSON or text file the build writes holds an absolute path; gives the key path. |
| `load.format` | error | | A loader found no manifest, or a `format` it does not read; names `diagnostics.json` when present. |
| `load.bad_file` | error | | An array file's header, size or shape is wrong, or a listed file is missing. |
| `load.too_many_materials` | warning | | A chunk names more materials than this loader draws; names the chunk and its materials (`fields.md` 7.1). |
| `load.fields_not_drawn` | info | | The fields the loader neither drew nor kept, by name. |
| `load.bad_target` | warning | | An output feature's camera target (a name attribute) is missing, is not the resolved form `{ "at", "name" }` of 9.1, or lies within 1 mm of the eye in plan, or its eye height is missing or not a finite number (15.1); names the feature and the attribute. That feature gets no camera. Hint: check that its kind lists the attribute in `names` (`authority.md` 1.2) and gives the eye height a default. |

Every `check.*` hint is to build again; `via check` never repairs. `load.*` codes are raised by
readers, not by `via build`, and are registered like the others (`diagnostics.md`), so a reader's
message has a code, a subject (the file or chunk) and a hint. `graph.busy`,
`graph.build_file_restored`, `graph.store_corrupt`, `graph.store_unavailable`,
`graph.step_unstable`, `identity.no_previous`, `identity.reselected`, `cli.usage` and
`cli.unknown_code` are other chapters' codes used here.

## Conformance

Each case runs through the command line (R20): input; action; what must hold. Files are read with
numpy or the standard library, never through the SDK. Every case is also checked by case 8.

1. **Null** (M0). `spec/vectors/build/null/world.toml`; build; `id` equals `id.txt`, `manifest.json`
   less `run` and `locations` equals the recorded one; `via check` passes.
2. **Systems and threads.** `ford`; build at 1 and 8 threads on macOS, Linux and Windows; same `id`,
   every file byte-identical. The Windows leg waits for hosted runners (`docs/plan.md`, "Comes back
   to you later"); until then it is pending, not dropped.
3. **Rebuild.** `ford` built twice into one directory; the second writes nothing into the directory;
   `diagnostics.json` byte-identical; the run report holds no run diagnostic.
4. **Shuffled.** `ford` with tables and keys reordered and a comment added; build; same `id`, every
   file of `files` byte-identical. `run.key` and `run.inputs` differ.
5. **Moved.** `ford` built to two `--out` folders at different depths; same `id`; `manifest.json`
   differs only in `locations`.
6. **Libraries anywhere.** `ford` with `--lib` given as an absolute path to copies in two different
   folders; same `id`; `manifest.json` byte-identical but `locations`.
7. **Nothing absolute.** Every case, run as a test user and host whose names the runner makes
   unique; no JSON or text file the build writes holds a path starting `/`, `\`, `//` or a drive
   letter, the user name or the host name; no `..` but in `locations` and in source paths the text
   wrote so.
8. **Complete.** Every case; `via check`; exit 0.
9. **Check fails.** A `ford` build with one byte of `fields/height.npy` flipped; with an added file;
   with `instances/x.npy` cut short; with `features.json`'s keys reordered; with `id` removed. Each:
   `via check` exits 1 with `check.digest`, `check.unlisted`, `check.header` (and `check.digest`),
   `check.not_canonical`, `check.id`, naming the file. `via check` with no argument: exit 2.
10. **Id rule.** `ford`; the digest of `manifest.json` less `id`, `run` and `locations` equals `id`.
    `changes.json` rewritten with other entries and its record in `run.reports` updated: `via check`
    passes and `id` is unchanged.
11. **No stale file.** `ford` with a second weights field, then without it; build twice;
    `weights/<that field>/` is gone.
12. **Array file.** `extent = [0, 0, 10, 6]`, `resolution = 2`, `base = "= 0.25 * x + y"`; build;
    `numpy.load("fields/height.npy", mmap_mode="r")` has shape `(4, 6)`, `dtype` `<f4`, and post
    `(5, 3)` holds `8.5`; the file's size is its header plus `4 * 6 * 4` bytes.
13. **One format.** A world names `ford`'s `fields/height.npy` as its height `file`, with `ford`'s
    extent and `outside = 0`; build; its `fields/height.npy` is byte-identical to `ford`'s.
14. **Row order.** `extent = [0, 0, 10, 6]`, `resolution = 2`, `base = "= y"`; build; row 0 is all
    0, row 3 is all 6.
15. **No instances.** A world with none; build; every column has shape `(0,)`, `names.txt` is 0
    bytes, `via check` passes.
16. **Chunk equals cut** (M6). `ford` with the default chunk, and with `chunk` equal to the extent;
    each material's plane in each chunk equals the same window of that material's plane in the
    single chunk, byte for byte.
17. **Twelve materials.** Twelve materials painted in overlapping strips inside one chunk; build;
    that chunk has twelve planes in id order, no warning or error; every post sums to 255 or 0.
18. **Largest remainder.** Three materials of equal weight at one post; build; bytes 85, 85, 85. Two
    of equal weight; bytes 128 and 127, the 128 on the lower id. One material at weight `1e-4` beside
    one at 1: no plane for the first.
19. **Grouping.** `ford`; build; rows sorted by chunk, asset, name; each chunk's `[first, count]`
    covers exactly the rows whose `x`, `y` fall in it.
20. **No signed zero.** A field whose formula gives `-0`; build; the bytes of `+0`.
21. **Failed build.** `ford`, then `ford` with a formula typo; exit 1; the directory holds only
    `diagnostics.json` and `.previous`, whose `id` is the first build's; the next good build's
    `changes.json` is against the first build.
22. **Unreadable world.** A world file `w.toml` with a TOML syntax error, no `--out`; build; exit 1;
    `build/w/diagnostics.json` holds the error.
23. **Not a build.** `--out` naming a folder holding one unrelated file, and a folder holding a
    `manifest.json` without a `via-build-` format; build each; `build.not_a_build`, the files
    untouched.
24. **Killed between renames.** `ford` built, then rebuilt with a test switch that stops the process
    after the first rename; build again; the build completes, and no `.partial` or `.old` remains.
25. **Baseline gone.** `ford` built with `--out build/ford`, the store deleted, rebuilt with
    `--change downstream --out build/ford`; `changes.json`
    has `against: null`; `build.baseline_missing` in the run report.
26. **Change names and files.** `--change downstream` and `--change changes/downstream.toml` with the
    same body; build each; same `id`; `run.changes` differs.
27. **Outputs.** `ford`; build; `outputs.bookmark` is `["at_ford", "in_trees", "on_bank",
    "to_door"]`; in `features.json`, `to_door`'s `attrs.toward` is `{ "at", "name": "cottage" }`, `at`
    the cottage's pivot.
28. **Reserved names.** A field named `nul`; build on each system; the manifest names
    `fields/nul-.npy`, which exists.
29. **Flags in the key.** `ford` with a change adding `[clear.in_beck]`, `on = { ref = "beck", t = 0.3 }`,
    `radius = 1`, `by = ["trees"]` (on the bed, where the beck keeps every tree off), gives
    `identity.clear_empty` (warning); built, then with `--fail-on identity.clear_empty`: exit 1,
    and not "unchanged"; then without: exit 0. (M2:) the same with `--stand-ins`: the second build
    runs, and `assets.*.stand_in` is `true`.
30. **Fail on, unknown.** `--fail-on formula.unknwn_name`; exit 2, `cli.unknown_code`, nearest code
    named; no build directory touched. `--fail-on` of a registered error code: exit 2, `cli.usage`,
    nearest warning code named.
31. **Mesh bytes** (M3). A world with one generated mesh, on two systems; same `.glb` bytes.
32. **Stand-ins** (M2). `ford` with its pack accepted; build with and without `--stand-ins`;
    instance columns byte-identical.
33. **Fail on revised** (M2). A revised asset; build with `--fail-on asset.revised`; exit 1; without
    it, exit 0.
34. **Every vertex** (M1; spike S11.O.3). A path of five written points, and a path with `smooth`;
    build; each feature's `points` in `features.json` are every vertex of its final curve: the five
    written points, and every vertex of the polyline `geometry.md` 2.4 samples, same count and same
    millimetres. The spike wrote every eighth 2 m sample.
35. **Footprints written** (M1; spike S11.O.5). `ford`; build; `features.json`'s `footprints` holds
    one entry, `instance` `cottage`, kind `pad`, whose `pieces` are the footprint feature of
    `rules.md` 2.1 for the cottage's row in the instance columns.
36. **Every field written** (M1; E4). `ford` with a scalar field `moisture` that no loader option
    names; build; the manifest's `fields` lists `height`, `clearing`, `water` and `moisture`, each
    file exists and `via check` passes. Loader options are not world text, so nothing in the build
    depends on them.
37. **Change report** (M1). `ford`, then `ford` with `downstream`, both with `--out build/ford`;
    `changes.json` has the members of
    11.2 and no others; every entry the plan's `downstream` row names is present with its `table`;
    `at_ford`'s `what` is `["moved", "changed"]` with `keys` `["attrs.toward"]`; `height` has a
    `region` with at least one window and `causes` holding `feature.ford.on` with its file and line;
    every summary row's `line` matches its counts.
38. **Eval and windows** (M1). `ford`; `via eval --field height` whole, and with `--window 0,0,1,1`,
    `--window 3,5,10,8` and `--window 128,128,256,256`; each file equals the same cut of the whole,
    bit for bit, and of `fields/height.npy`. `--window 0,0,0,1` and `--window 0,0,600,1`: `cli.usage`,
    exit 2.
39. **Version** (M0). `via --version`; exit 0.
40. **Help** (M0). `via help`; exit 0.

Stable names across a change are `identity.md`'s cases, not repeated here.

**Other suites.** These test readers and code, not the format, so they are not spec conformance
(R20):

| Suite | Holds |
|---|---|
| SDK | Values the SDK reads from `fields/height.npy` equal numpy's. |
| Deny-list | The core crates hold no string `bookmark` or `water` (`vocabulary.md` 7). |
| Godot loader | `ford`, unattended, with `diagnostics` set: one `MultiMeshInstance3D` per chunk entry's asset, holding its `count`; the sheet tool with `--before` `ford`, `--after` `ford+downstream` and `ford`'s `--option` (15.1): four PNGs per side, 1280 by 720, none all one colour, and `sheet.png` 1280 by 1584; at `at_ford`, pixels of the water material in the frame; `load.fields_not_drawn` lists every field neither drawn nor kept; a bookmark whose `toward` is plain text gives `load.bad_target` and no camera. |
| glTF reader | Khronos validator reports no error; instance nodes equal `instances.count`; a source check finds no import outside Python's standard library and nothing from `engines/godot/`. |

## Answers

TODO lines and challenge rows this chapter closes or defers.

| Item | Answer |
|---|---|
| TODO: `atlas` exported no water and nothing said so | Closed: every field written (rule 7); the loader draws water by option and lists what it did not draw (15.1). |
| TODO: lift the limit of eight ground materials | Closed: per-chunk planes, any number (6.2). |
| TODO: compile-time export versus runtime loading | Closed: one loader, the tree saved or made at start (15.1). |
| TODO: the handoff representation | Closed for M1: chunks (5), rows grouped by chunk and asset (8), stable names (8), facts for detail and budgets (7, 5). Realisation classes are not a build key: what a thing is already says how it is realised (a field is drawn per chunk, instance rows batch per chunk and asset, a mesh is static), and a key would say it twice (principle 10). |
| TODO: engine target strategy | Closed on paper: Godot (15.1), Unreal (15.2), glTF (15.3). |
| Challenge 3, walk it in the engine | The loader's half: 15.1. |
| Challenge 4, 30 frames a second | Not ours (principle 12); the hand-off shape is (8). |
| Challenge 41, Unreal | Deferred (doctrine section 7); table 15.2. |
| Challenge 42, streaming | Deferred to M6; chunks are the unit (5). |

Owed to other chapters:

- `graph.md`: say "chunk" for "tile"; add `run.flags` to the build key (6.2); follow section 5's
  overlapping windows in its worked example; close its open item on tile size here.
- `fields.md`: say "chunk"; word its field-file check as a `shape` that is not 2-D or a `descr`
  other than `<f4`.
- `protocol.md` 4.2: allow a three-dimensional shape; fix the header's padding as 3.2 does.
- `diagnostics.md` 7 and `identity.md` 5: world and run diagnostics are split (11.1);
  `identity.no_previous` and `identity.reselected` are run diagnostics. `diagnostics.md` 7 and
  `graph.md` 3.4 point to 11.1 instead of restating the run list.
- `vocabulary.md`: the asset words `assets.md` 1.1 lists. The review library (`lib/review`, M1):
  document `toward`, and `eye_height` with its default 1.7 (9.2); the retired `look` is not reused.
- `authority.md` 1.2: adopted here (9.1, 15.1). It is asked to name an instance's point, its pivot
  (section 8), since `ford`'s `to_door` looks at `cottage`.
- `identity.md` 5: its report is written as 11.2 fixes, including `causes: null` when the
  baseline's record is gone, summary rows for written things (`<table> (written)`), and `via` as one
  chain from the node that reads the statement.
- `numbers.md` and `formulas.md`: the forms of `via eval` and `via vectors` are fixed (14); their
  (open) marks on them can go. `formulas.md` cases 45 and 46 run `via eval --tile 1` and
  `--tile 16` with `--diagnostics`; the `--window N` build flag is gone.
- `conformance.md` 3.5, `diagnostics.md` 7 and `documents.md` 8: diagnostics of every command,
  `merge`, `pack list` and `query` included, reach the runner by `--diagnostics FILE` (14), which
  closes `conformance.md`'s (open) item; `--json` is refused where standard output is the result;
  `via merge` writes no `diagnostics.json`.
- `geometry.md` 7 and `protocol.md` 8.1: `via kernel` and `--sdk` are listed (14).

## Changed from the spike

| Spike (SPEC section 9) | v0 | Why |
|---|---|---|
| `world.json`, no format version | `manifest.json` with `format`, `engine`, `id`, every file's digest, and how it was asked for under `run` | A reader must know what it reads; R26 made checkable. |
| Raw `.f32`, no header | NumPy `.npy` array files, the format steps already use | Size and type checkable; one format for builds and steps (R20). |
| `ground_0.rgba8`, `ground_1.rgba8`: eight materials, `ground.too_many` | Per chunk, one plane per material present | TODO: `atlas` had eleven. |
| Only `export.fields` written; `water`, `bed`, `hole` by name | Every field written, with role and tags; a loader picks | Doctrine section 6; `atlas`'s water was silently lost. |
| `water` held 0 where dry | No sentinel; water as a surface height; a cover where one exists | Rule 5. |
| Instances `{ asset: { "cx,cy": [rows] } }`, no ids for scattered ones; `explicit` beside them | One table, sorted by chunk, asset, name; every row named | Stable ids for re-import and the change report (R3, R13). |
| `yaw` float | `face`, integer millidegrees | `numbers.md` 2. |
| Absolute paths to pack files | Pack paths under `pack_root`; source paths against named roots | Rule 3. |
| `features` without footprints; paths every eighth 2 m sample | Every feature and footprint; the final curve's vertices | Spike section 11; FINDINGS (SDK `points`). |
| `bookmarks`, `rooms`, `lights`, `look`, `params` keys | Library outputs; world facts as their attributes; the rest the loader's settings | Principles 5, 12; doctrine section 6. |
| `vol_*`, `mesh_*` raw arrays | glTF binary | A format every engine reads. |
| `budgets.json`, `report.json` (water measures) | `triangles` and `count` per chunk; measures are library lints | Principle 5. |
| `minimap.png`, `reach.png`, `plan_*.png`, `map/` by the compiler | Library export steps | Principle 5. |
| Stale files kept; nothing written on `field.required` | Replaced whole; `diagnostics.json` always | Rule 2; `diagnostics.md` 4. |
| `--variant`, `--overlay`, `--out` under the package | `--change`, `--out` anywhere | One concept, `change`. |

## Open

- **Scalar fields per chunk.** Whole-extent files until a world is too large to map whole. Decided
  at M6 by the chunk walk on `atlas`.
- **The default chunk side** (64 m). Decided in M1 by `ford`'s cold and unchanged build times.
- **`features.json` as JSON.** Kept while it stays small. Decided at M6 by `atlas`'s size and load
  time.
- **Copying asset files into a build** (`--bundle`), for a hand-over to another machine. Decided by
  the first hand-over, M6 or M7.
- **A surface's mesh spacing and material** (`surfaces.md` 5). Decided by the first deck, M3.
- **The terrain under a floor or deck** (`surfaces.md` open 7). Decided in M5 by `station`'s walk.
- **A picture passed to the engine as a texture** (`fields.md` open). Decided by `relief` in M2: if
  a field file loses what the picture carried.
- **The `kind` of a network's areas and cells** in `features.json` (9.1). Decided by `quay` in M3.
- **How the SDK names a blob** without a native BLAKE3 (3.3). Decided in `protocol.md` for M2.
- **glTF instancing** (`EXT_mesh_gpu_instancing`) in place of a node per instance. Decided by the
  second reader on `atlas`.
- **The player map's output name** (9.4). Decided in `protocol.md` for M6.
