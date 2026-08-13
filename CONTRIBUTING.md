# Contributing

Early project. This file records the decisions that are **settled**, so they are not relitigated. Research commitments live in the [README](README.md); this is engineering only. Anything not listed here is open.

## Locked

**Rust, whole hog.** Pipeline, gates, metrics, renderers — one Cargo workspace, one language. Not a Rust core with a Python rim. When a Python-only library is genuinely needed, it is a throwaway script run *against* an exported artifact, never a dependency of this project.

**CPU, not GPU.** The workload is graphs, priority queues, and computational geometry — irregular and branchy. Parallelism is `rayon` across cores. GPU work is permitted for optional detail passes and rendering, never inside the simulation's decision loop.

**Determinism is a hard requirement.** Same seed → bitwise-identical output, on any core count. Seeds derive from `global_seed + stage + tile`; no global RNG. No iteration over `HashMap` where order reaches output. No float accumulation whose order varies with thread scheduling. A nondeterminism bug is a P0.

**Stages communicate through artifacts.** Every stage reads typed artifacts and writes typed artifacts plus a stats record. Stages do not call each other. This is what makes caching, resuming, and swapping implementations possible.

**The chain is one-directional.** Terrain → hydrology → suitability → corridors → settlements → flows → network → morphology. No stage mutates its inputs. A stage that cannot work with what it was given fails loudly rather than repairing it.

**Nothing authored that can be derived.** Classes, hierarchies, and sizes are measurements over simulated state, never typed-in labels. Where the simulation cannot satisfy a constraint, the discrepancy goes in the stats record — never silently absorbed.

**Fixed-point coordinates.** Integer centimetres internally; floats at the boundaries only.

## Working

- `cargo fmt`, `cargo clippy -- -D warnings`, `cargo test` clean before a PR.
- Snapshot tests (`insta`) guard determinism. Review snapshot diffs; never blind-accept.
- Benchmark (`criterion`) any change to a hot loop and include the numbers in the PR.
- Commits: `type(scope): summary`.

## Open

Epoch model, export target. Raise a discussion before building on an assumption about any of these.

Artifact format and crate layout were settled in [ADR 0001](docs/adr/0001-crate-layout-and-artifact-format.md).
