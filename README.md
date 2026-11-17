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

A rewrite is under way on branch `rewrite`, at the close of milestone M7 (the first cut: agents
of two model families, both Claude models (Fable and Sonnet), built worlds from these documents
alone, each world built, the first build within 4.5 to 9 minutes; a run by another vendor's model
is the owner's; M6 joined every example in one map, `examples/atlas`, streamed in chunks, with
budgets and the player map as data). Until `main`
moves to it at the owner's review, `main` holds the older research tree: check out `rewrite` (a
tree with `spec/`, `lib/` and `examples/`). The new engine builds every world under `examples/`;
`docs/plan.md` says what each milestone adds and `TODO.md` what is open.

### First build of your own world

```sh
cargo build --release -p via-cli -p via-terrain     # a cargo-built via has no shipped library root:
export VIA_LIB=$PWD/lib                             #   name ours (documents.md 4.1)
export PATH=$PWD/target/release:$PATH                # or your --target-dir's release folder
uv sync --project sdk/python --python 3.12          # the SDK's venv: Python 3.12 and NumPy
via config program python $PWD/sdk/python/.venv/bin/python   # steps run on Python 3.12 (the SDK's venv)
via config program via-terrain $PWD/target/release/via-terrain
via trust path/to/world.toml --yes                  # record trust in the world's steps (protocol.md)
via build path/to/world.toml --out build/w
```

A step you have not trusted does not run: a generate step builds from the result committed beside
its world (`gen/`, `protocol.md` 7.1), so `via build examples/basin/world.toml --out build/basin`
builds `basin` without running its erosion, with `protocol.untrusted_result`; its lint is skipped.

A change is passed by its path (`--change changes/x.toml`; a bare name holds no `.` or `/`,
`build.md` 14). Copy a world from `examples/` to start (its files, never its `.via/` store);
`spec/vocabulary.md` lists every key.

- **A world of your own** may live anywhere. Its `include`d libraries come from `VIA_LIB`; an
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

The target shape, from `docs/plan.md`. Folders appear as their milestone reaches them.

```
spec/            normative: chapters, schemas, vectors, conformance cases
crates/          the engine: num, diag, doc, expr, geom, raster, graph, model, proto, cli, step
                 (+ terrain: lib/terrain's erosion program, library tier; xtask: the checks)
sdk/python/      thin port: pure Python, never links the engine
lib/             our libraries: kinds as TOML + steps in Python, through the SDK only
                 (land, water, ways, town, building, review, map; terrain's step is a program)
engines/godot/   loader addon and the review viewer (eye-height sheets)
engines/gltf/    a second reader of the build directory (glTF)
tools/           the conformance runner, the pack generator, the map renderer
fixtures/        the spike's oracle data (exported at `spike-final`) and the kernel's fixtures
examples/        the final examples, each with change files and ground bookmarks
docs/            doctrine, challenges, design, references
```

Of the old workspace, only `crates/via-terrain` carries over, as the erosion program behind
`lib/terrain` (`spec/protocol.md` 8 admits it); its model is described in
`crates/via-terrain/docs/model.md`. The rest was dropped at the M7 cutover and is in the history at
the tag `spike-final`.

## Where to read

| Document | What it is |
|---|---|
| [`docs/charter.md`](docs/charter.md) | One page: the intended use, what we own, what we do not, and what we do not promise. Read it first. |
| [`docs/doctrine.md`](docs/doctrine.md) | The goal, the principles, the eleven nouns and every ruling. It decides details. |
| [`docs/plan.md`](docs/plan.md) | The rewrite plan: milestones M0 to M7 and milestone one, `ford`. |
| [`docs/reports/first-cut.md`](docs/reports/first-cut.md) | One page for the owner's review of the first cut: what holds, what does not, the open rulings and decisions. |
| [`spec/`](spec/) | The contract: chapters, schemas, test vectors and conformance cases. Start with `spec/vocabulary.md`. |
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
