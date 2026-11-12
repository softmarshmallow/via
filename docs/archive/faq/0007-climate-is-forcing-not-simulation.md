# 0007 — What actually decides the climate?

**Status: doctrine + ledger.**

**Q:** Based on what does the climate get simulated — is it simply
deterministic from the heightmap?

**A:** Almost, and deliberately. The dependency graph: wind direction ←
seed; precipitation ← topography along the wind + four sweep
coefficients + a normalization to the configured land mean; temperature
← elevation lapse + a linear north–south gradient + sea-level base +
small seeded noise. No weather, no seasons, no continentality.

**Climate here is forcing, not simulation — and that is standard
research practice.** A bounded 8–100 km domain cannot derive its own
climate (that is set by global circulation and latitude, outside the
box); regional models prescribe it. The scientific obligations are:
declare the forcing in config (never hardcoded — ADR 0003), propagate it
mechanistically (the rain shadow *emerges* from the moisture sweep, it
is never painted), and never claim it was emergent.

Consequence for small-map biome variety: real 8×8 km areas hold one
climatic biome — except steep oceanic islands (Tenerife, Hawaiʻi), where
huge relief × trade-wind shadow packs many zones into few km. Games
should exaggerate exactly those two real mechanisms via the declared
knobs (lapse rate, meridional gradient, orographic coefficients, relief)
— **distort the planet, never the pipeline**. The classifier's own
thresholds are not a knob.

Ledger: Smith & Barstad 2004 linear orographic model (better rain
fields); parametric seasonality (unlocks Köppen — see FAQ 0008).
