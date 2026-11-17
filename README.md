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

A rewrite is under way on branch `rewrite`, at the close of milestone M6 (whole, and handed over:
every example joined in one map, `examples/atlas`; chunks streamed; budgets; the player map as
data). The new engine builds every world under `examples/`; `docs/plan.md` says what each
milestone adds and `TODO.md` what is open.

### First build of your own world

```sh
cargo build --release -p via-cli -p via-terrain     # a cargo-built via has no shipped library root:
export VIA_LIB=$PWD/lib                             #   name ours (documents.md 4.1)
via config program python $PWD/sdk/python/.venv/bin/python   # steps run on Python 3.12 (the SDK's venv)
via config program via-terrain $PWD/target/release/via-terrain
via trust path/to/world.toml --yes                  # record trust in the world's steps (protocol.md)
via build path/to/world.toml --out build/w
```

A change is passed by its path (`--change changes/x.toml`; a bare name holds no `.` or `/`,
`build.md` 14). Copy a world from `examples/` to start; `spec/vocabulary.md` lists every key.

The spike under `spike/` is the record of the concept phase: 19 worlds, built in throwaway Python.
It is kept as an oracle to compare against, never as a source, and is tagged `spike-final`. It is
deleted at milestone M7.

## Repository layout

The target shape, from `docs/plan.md`. Folders appear as their milestone reaches them.

```
spec/            normative: chapters, schemas, vectors, conformance cases
crates/          the engine: num, diag, doc, expr, geom, raster, graph, model, proto, cli (+ step, erosion)
sdk/python/      thin port: pure Python, never links the engine
lib/             our libraries: kinds as TOML + steps in Python, through the SDK only
                 (land, water, ways, town, building, review, map)
engines/godot/   loader addon and the review viewer (eye-height sheets)
examples/        the final examples, each with change files and ground bookmarks
docs/            doctrine, challenges, design; pre-reboot research in docs/archive/
spike/           the concept phase, kept as record and oracle until M7
```

Of the old workspace, only `crates/via-terrain` carries over, as an erosion library a step calls.
Its design records are in `docs/archive/adr/`.

## Where to read

| Document | What it is |
|---|---|
| [`docs/doctrine.md`](docs/doctrine.md) | The goal, the principles, the eleven nouns and every ruling. It decides details. |
| [`docs/plan.md`](docs/plan.md) | The rewrite plan: milestones M0 to M7 and milestone one, `ford`. |
| [`spec/`](spec/) | The contract: chapters, schemas, test vectors and conformance cases. Start with `spec/vocabulary.md`. |
| [`docs/challenges.md`](docs/challenges.md) | Every request made of the tool, how it fared, and the milestone that closes it. |
| [`docs/design/blackbox.md`](docs/design/blackbox.md) | The tool seen from outside. |
| [`docs/faq.md`](docs/faq.md) | Questions a newcomer would ask, each marked shown, decided, open or not ours. |
| `lib/<name>/kinds.toml` | Each library's reference: its header comment gives the kinds, the steps a world restates and their params (`lib/map` and `lib/building` also have a README; `lib/land/ground.md` is one longer note). |
| `examples/<name>/README.md` | What each example world shows and what is still wrong with it. |
| [`TODO.md`](TODO.md) | Open items with their context, each filed to a milestone or a chapter. |
| [`CONTRIBUTING.md`](CONTRIBUTING.md) | How work is done and the engineering rules. |
| [`docs/archive/`](docs/archive/) | The research project before the 2026-11 reboot, kept as record. |

## License

MIT. See [LICENSE](LICENSE).
