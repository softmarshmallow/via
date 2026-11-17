# via

> Decide what matters; the world settles around it. Change your mind, and it settles again.
> You never repair a map by hand.

`via` compiles a world written as text into files a game engine loads. You describe terrain, water,
roads, buildings and dressing by intent. The compiler resolves where they meet, places every
instance, and tells you what does not fit. When an asset or a settlement changes, you edit a line
and rebuild; you do not repaint a map.

It is for new games, small teams first, on Godot or Unreal, with an agent writing the documents and
a person reviewing them. `via` is a working name.

## Status

The first cut is on `main` since 2026-11-18: milestones M0 to M7 of `docs/plan.md`. The engine
builds every world under `examples/`. Thirteen of them are joined in one map, `examples/atlas`,
streamed in chunks, with budgets and the player map as data. Agents of two model families, both
Claude models (Fable and Sonnet), built worlds from these documents alone: each world built, the
first build within 4.5 to 9 minutes; a run by another vendor's model is the owner's.
`docs/reports/first-cut.md` says what holds and what does not, and `TODO.md` what is open. Nothing
has shipped with it (`docs/charter.md`).

## Install

From a clone (or your fork), on macOS or Linux, with Rust and uv (or Python 3.11+):

```sh
./install.sh                     # builds via and installs it, its libraries and a Python into ~/.via
. ~/.via/env.sh                  # puts via on PATH; add this line to your shell's profile
```

Inside a game's repository, with `via` as a submodule and the Godot loader copied into the game:

```sh
git submodule add https://github.com/softmarshmallow/via third_party/via
third_party/via/install.sh --prefix .via-tools --godot path/to/godot-project
```

`docs/install.md` has the rest: what each needs, updating, uninstalling, the glTF export for other
engines, the steps by hand (Windows, or working on `via` itself), and what is not there yet
(crates.io and PyPI).

### First build of your own world

```sh
via trust path/to/world.toml --yes                  # record trust in the world's steps (protocol.md)
via build path/to/world.toml --out build/w
```

A step you have not trusted does not run: a generate step builds from the result committed beside
its world (`gen/`, `protocol.md` 7.1), so `via build examples/basin/world.toml --out build/basin`
builds `basin` without running its erosion, with `protocol.untrusted_result`; its lint is skipped.

A change is passed by its path (`--change changes/x.toml`; a bare name holds no `.` or `/`,
`build.md` 14). Copy a world from `examples/` to start (its files, never its `.via/` store);
`spec/vocabulary.md` lists every key.

- **A world of your own** may live anywhere. Its `include`d libraries come from the install's `lib/` (or
  `VIA_LIB`, searched first); an
  `[import.<id>]`'s `file` is relative to the file that declares it, or absolute
  (`composition.md` 2). Diagnostics still print paths relative to the world's folder, so an import
  outside it shows as `../../…` (no absolute path in a build, `build.md` case 7).
- **`--out`** may be relative, `..` included (`--out ../build/w`), or absolute.
- **Steps.** A step of your own is Python on the SDK: `sdk/python/README.md` (setup, a worked lint,
  units, what bites). Each library's reference is its `kinds.toml` header.
- **Joining examples.** Where a parent ends a road on each example, and the lift it needed, is
  `examples/atlas/README.md`, "Joining an example". The usual round: import with any `lift` (or
  none), build, write the `lift` the `import.seam` hint names, build again, then measure the join
  where your road meets the import and adjust (`composition.md` 5: the hint is the edge's median).
  An import's own warnings print with its id before the subject (`town.…`); they are its own, not
  yours to fix. A step lists an import's features under dotted names; its report on one lands on
  the import (`import.<id>`, the name in `data.place`).

The spike was the concept phase: 19 worlds, built in throwaway Python. It was deleted at milestone
M7; its record is the tag `spike-final` (`git show spike-final:spike/FINDINGS.md`). What it said
and measured is kept as data in `fixtures/oracle/`, the oracle the examples compare against, never
a source. Where a document cites the spike's `SPEC`, `FINDINGS`, `VOCABULARY`,
`check.py` or a world's `LOG`, it means that file at the tag.

## Repository layout

`docs/layers.md` says how the folders are ordered, what may depend on what, and where every crate,
chapter and library sits.

```
spec/            the contract: chapters, schemas, vectors, conformance cases
crates/          the engine: num, diag, doc, expr, geom, raster, graph, model, proto, step, cli
                 (+ terrain: lib/terrain's erosion program, a library's; xtask: the checks)
sdk/python/      thin port: pure Python, never links the engine
lib/             our libraries: kinds as TOML + steps in Python, through the SDK only
                 (land, terrain, water, vegetation, ways, town, building, map, review; and the packs)
engines/godot/   loader addon and the review viewer (eye-height sheets)
engines/gltf/    a second reader of the build directory (glTF)
tools/           the conformance runner, the pack generator, the map renderer, the two checks of docs/
fixtures/        the spike's oracle data (exported at `spike-final`) and the kernel's fixtures
examples/        the example worlds, each with change files and ground bookmarks
docs/            charter, doctrine, layers, the ledger, challenges, design notes, reports, references
```

Of the old workspace, only `crates/via-terrain` carries over, as the erosion program behind
`lib/terrain` (`spec/protocol.md` 8 admits it); its model is described in
`crates/via-terrain/docs/model.md`. The rest was dropped at the M7 cutover and is in the history at
the tag `spike-final`.

## Where to read

| Document | What it is |
|---|---|
| [`docs/install.md`](docs/install.md) | Installing: for yourself from a clone or fork, inside a game's repository, into Godot or another engine, and by hand. |
| [`docs/charter.md`](docs/charter.md) | One page: the intended use, what we own, what we do not, and what we do not promise. Read it first. |
| [`docs/doctrine.md`](docs/doctrine.md) | The goal, the principles, the eleven nouns and every ruling. It decides details. |
| [`docs/plan.md`](docs/plan.md) | The rewrite plan: milestones M0 to M7 and milestone one, `ford`. |
| [`docs/reports/first-cut.md`](docs/reports/first-cut.md) | One page for the owner's review of the first cut: what holds, what does not, the open rulings and decisions. |
| [`docs/reports/revisit.md`](docs/reports/revisit.md) | One page: what was reorganised after the first cut, how it was checked, and the verdict on each thing found. |
| [`spec/`](spec/) | The contract: chapters, schemas, test vectors and conformance cases. Start with `spec/vocabulary.md`. |
| [`docs/layers.md`](docs/layers.md) | How the repository is laid out: the layers of meaning, the rings of code, where every crate, chapter and library sits, and what the layout shows. |
| [`docs/ledger/`](docs/ledger/README.md) | The vocabulary of a game world: every term filed under an archetype and an owner, with what the first cut shows of it. |
| [`docs/challenges.md`](docs/challenges.md) | Every request made of the tool, how it fared, and the milestone that closes it. |
| [`docs/design/blackbox.md`](docs/design/blackbox.md) | The tool seen from outside. |
| [`docs/faq.md`](docs/faq.md) | Questions a newcomer would ask, each marked shown, decided, open or not ours. |
| [`sdk/python/README.md`](sdk/python/README.md) | Writing a step of your own in Python: setup, a worked lint, units, what bites. |
| `lib/<name>/kinds.toml` | Each library's reference: its header comment gives the kinds, the steps a world restates and their params (`lib/map` and `lib/building` also have a README; `lib/land/ground.md` is one longer note). |
| `examples/<name>/README.md` | What each example world shows and what is still wrong with it. |
| [`TODO.md`](TODO.md) | Open items with their context, each filed to a milestone or a chapter. |
| [`CONTRIBUTING.md`](CONTRIBUTING.md) | How work is done and the engineering rules. |

## License

MIT. See [LICENSE](LICENSE).
