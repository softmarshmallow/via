# Design records of the erosion model

The decisions behind `crates/via-terrain`'s model, written in the old workspace (2026-08) and kept
because its code cites them (`ADR 0002` to `0005` in comments). They speak of that workspace's
stages and crates (`via-ecology`, `via-artifact`, the `.vrast` rasters), which are retired; what
still holds is the model: climate as forcing (0002), the line between science and declared
preference (0003), sediment and standing water (0004), and multiple-flow-direction routing (0005).

The rest of the old workspace's documents (its README, roadmap, FAQ and ADR 0001 on the retired
crate layout) were dropped at the M7 cutover; they are in the history at the tag `spike-final`
(`README.md`, `docs/ROADMAP.md`, `docs/faq/`, `docs/adr/`). The current documents are
`docs/doctrine.md`, `docs/plan.md` and `spec/`.
