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

A rewrite is under way on branch `rewrite`, at milestone M0 (decisions written, the cheapest risks
probed). Nothing in the new engine runs yet.

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
| [`TODO.md`](TODO.md) | Open items with their context, each filed to a milestone or a chapter. |
| [`CONTRIBUTING.md`](CONTRIBUTING.md) | How work is done and the engineering rules. |
| [`docs/archive/`](docs/archive/) | The research project before the 2026-11 reboot, kept as record. |

## License

MIT. See [LICENSE](LICENSE).
