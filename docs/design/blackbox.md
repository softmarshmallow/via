# via: the README we want to be true

Status: design target, written 2026-11-12. Nothing here is built. It describes the tool from the
outside only, so we can argue about the surface before the inside. `via` is a placeholder name.
Everything marked (open) is undecided.

---

**Decide what matters; the world settles around it. Change your mind, and it settles again. You
never repair a map by hand.**

`via` compiles a world written as text into files a game engine loads. You describe terrain, water,
roads, buildings and dressing by intent. The compiler resolves where they meet, places every
instance, and tells you what does not fit. When an asset or a settlement changes, you edit a line
and rebuild; you do not repaint a map.

It is one binary. The contract is three things: the documents you write, the build directory you
get, and the lifecycle your own code can join. An SDK is optional: it is a convenient way to join
that lifecycle, never the only way.

## A world

```toml
# world.toml
name    = "vale"
extent  = [0, 0, 512, 512]            # metres
include = ["kinds/terrain.toml"]
pack    = "packs/primitive.toml"

[field.height]
unit = "m"
base = "40 * fbm(180, 4, seed=7)"

[feature.mill_road]
kind = "road"
path = [[40, 300], [210, 260], [470, 280]]

[feature.mill]
kind  = "pad"
on    = { feature = "mill_road", t = 0.4, offset = 12 }
asset = "building.mill"
```

Documents are TOML. Every entity is a table keyed by its id (`[feature.mill]`), and structure is
expressed by one entity naming another, not by nesting. Expressions are strings in a small language
of our own.

```bash
via build world.toml
```

```
vale: 0 errors, 1 warning
  warning seam.grade  mill_road  grade 14% exceeds 12% for 31 m near [205, 262]
build/vale/  written
```

## What you write

| File | Holds |
|---|---|
| world | fields, features, instances, bookmarks; the intent |
| kinds | what a kind of feature does to fields (a road flattens, a lake floods); data, not code |
| pack | binds asset IDs to files and declares their facts (footprint, pivot, collision) |
| layers | separately owned documents for one world, lowest first; a layer refers down, never up |
| parts | another world placed inside this one, at a seam or through a door |
| variants, overlays | named or external changes merged over a world (winter; a proposed edit) |

A world never names an asset file. Swap the pack and the same world builds with other assets.

## What you run

```
via build WORLD [--pack P] [--overlay F]... [--variant N]... [--out DIR] [--lint NAME]... [--map]
via explain CODE          # what a diagnostic means and how to resolve it
```

The exit status is non-zero on any error. Diagnostics are also written as JSON, each with a stable
code, the feature it concerns and a position. The same inputs give the same bytes, on any machine.

## What you get

```
build/vale/
  manifest.json      format version, extent, frames, the list of everything below
  fields/            height, ground material weights, water, masks, as rasters
  instances.json     every placed instance: asset ID, transform, layer, pass-through data
  meshes/            geometry the compiler made: volumes, interiors
  bookmarks.json     named views for review
  diagnostics.json
  map/               the player-facing map as data layers (with --map)
```

The build directory is versioned and documented. It is the only thing an engine, a script or
another tool needs to understand.

## Engines

- **Godot 4:** a loader addon reads a build directory into a scene.
- **Unreal:** the build format is documented with a reference importer. (open: how far we take it)

The engine owns rendering, streaming, physics, navigation and gameplay. `via` hands over instances
grouped so the engine can batch them, and declares facts the engine needs. It promises no frame rate.

## Extending it: one lifecycle, two ways in

A build runs in stages, and your code can join at named points. At each point the engine hands over
plain data (documents, rasters, instances) and takes plain data back. It validates what comes back
and applies its own rules to it, as it does to its own work.

| Point | When | Receives | Returns |
|---|---|---|---|
| generate | before, or between layers | the build of the layers below | a layer document |
| effect | during, at its place in the authority order | a field and a feature's shape | the field, changed |
| place | during, after the ground is resolved | fields and the instances so far | instances |
| lint | after | a finished build | diagnostics |
| export | after | a finished build | whatever the target needs |

There are two ways to join, and they are the same contract:

- **A command.** Name it in the world, as you add a step to a CI workflow. It reads files and writes
  files, in any language. This needs nothing from us but the format.
- **The SDK.** A package in your language that carries the engine, gives you typed access to the
  same data, and keeps your code running beside the engine so a build does not start a process per
  call. You opt in when a command stops being enough.

Built-in producers and lints use the same points, so nothing built in can do what yours cannot.

```toml
[[generator]]                      # writes a layer
id      = "hamlet"
command = ["python", "tools/hamlet.py"]
after   = "terrain"                # it receives the build of the layers at and below this one
writes  = "layers/hamlet.gen.toml"

[lint.patrol_routes]               # reads a build, writes diagnostics
command = ["python", "tools/patrol_lint.py"]
```

| Contract | Receives | Returns |
|---|---|---|
| generator | the build of the layers below it, and its own parameters | a layer document |
| lint | a finished build | diagnostics as JSON |
| exporter | a finished build | whatever the target needs |

A generator's output is an ordinary layer file. You can read it, diff it, commit it, or edit it and
stop running the generator. Built-in lints (`reach`) use the same contract as yours.

What a command does is yours: `via` records its inputs and reruns it when they change, but cannot
promise its output is reproducible.

## Ours and not ours

| Ours | Not ours |
|---|---|
| the document format and its meaning | modelling assets, textures, shaders |
| resolving seams and authority between features | the look of the final frame |
| placement, with every instance explicit in the output | runtime streaming and level of detail |
| diagnostics that name the feature and the reason | gameplay, AI, navigation meshes |
| the build format and the Godot loader | what your generator scripts do |
| the player map as data | painting the map's final style |

## Decided

- **TOML, flat, keyed by id** (2026-11-12). On a real excerpt it was the shortest form: 45 lines
  against 52 for the spike's `[[feature]]` lists and 57 for HCL. HCL's gains touch few lines
  (expressions are about 4% of a world) and its function calls have no named arguments. YAML was
  rejected for implicit typing and meaningful indentation. Keyed tables make overlays merge entry
  by entry, give every entity an address, and make a duplicate id a parse error.
- **Set up for an SDK from the start** (2026-11-12). User code is the end state of any world that
  lives long enough, so the lifecycle contract is part of the design, not an addition. The spike is
  no evidence either way: every need was met by changing the core. The engine holds all the logic;
  a command and the SDK are two transports for one contract.

- **The SDK language is Python** (2026-11-12). To the user this is tooling: short scripts beside
  the world. Python is what content pipelines already script in (Blender, Houdini, Maya and
  Unreal's editor), it has the array and image libraries a field or raster needs, and agents
  write it well. The engine side (Godot, Unreal) is a separate delivery in the engine's language.

## Open

- Three rules TOML leaves to our schema: which keys hold expressions (a string and an expression
  look alike), how an overlay removes a key or an entity (TOML has no null; the spike used
  `level = -1e9` for "unset"), and how a constant is shared across entities.
- The compiler's first step turns a document into a plain data model, so the syntax stays a front
  end and could gain a second one later.
- Table order no longer carries meaning. The spike applied effects in the order kinds were
  declared; that order must now be stated, for example as a priority.
- The expression language in fields and kinds: its grammar, types and function list.
- Whether a generator may run mid-compile only at layer boundaries (as above) or anywhere.
- How a command is trusted: a world from a stranger must not run programs unasked.
- Unreal: reference importer only, or a maintained plugin.
- Incremental rebuilds and a watch mode for agents.
- The lifecycle points above are a first list. Each needs its data defined, and `effect` and
  `place` need the engine to pause mid-build, which nothing has tested.
- When the Python SDK ships, and how it is installed on a game developer's machine.
- A function called per cell inside an expression has no place in this contract; the answer today
  is to precompute a field.
