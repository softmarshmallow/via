# ADR 0003 — Epistemic tiers and the opinion boundary

Status: accepted (2026-08-13)
Scope: which parts of via are science, which are declared preference, and
where the line is drawn structurally. Prompted by the observation that the
climate/biome layer "requires an explicit preference setup" — this ADR
answers whether that breaks the project's scientific claim (it does not,
provided the tiers below are respected).

## Decision — three tiers, labeled everywhere

**Tier 1 — Process.** Falsifiable mechanism: the LEM, flow routing, sediment
transport, later the settlement economics. Gates exist for this tier and
only this tier; a gate failure means the implementation or its calibration
is wrong.

**Tier 2 — Forcing.** Exogenous boundary conditions a bounded domain cannot
derive for itself: uplift field, wind, sea-level climate parameters, rock
unit geometry (M4), economic cost weights (later). This is standard research
practice — regional models prescribe their forcing (LEM papers prescribe
rainfall; Smith & Barstad 2004 is itself a parameterization used as
forcing). Rules: forcing lives in **config, never hardcoded**; it is
propagated mechanistically (a rain shadow emerges from transport, it is
never painted); it is never claimed as emergent. Deliberate distortion of
forcing (game-scale climate compression, exaggerated lapse rates) is
legitimate *because* it is declared — the output world is fictional but
causally consistent.

**Tier 3 — Interpretation.** Derived description over tier-1/2 fields:
biome classification, vegetation attribute heuristics, suitability scoring,
landform grammars. Optional stages a research consumer can ignore; each
output is labeled either **standard** (an academically recognized scheme,
cited and fixed — Whittaker classes, Strahler orders, geomorphons, TWI) or
**heuristic** (our invention — vegetation density and canopy formulas).
Heuristics are permitted but must be named as such in the stage's docs and
JSON summary.

## Corollary rules

1. **Modules never consider practical use.** Every stage is written for a
   research consumer. Game adaptation happens in `experiments/` configs
   (tier-2 distortion) and in downstream consumers, outside the crates.
2. **Spectra over taxonomy.** Default output is a continuous field.
   Discretization requires an academically recognized scheme; otherwise
   ship the spectrum and let consumers threshold it.
3. **No authored content in core.** Species lists, asset placements, ore
   veins are content consumed by downstream modules; via may ship
   distributions (mixture weights, densities, potentials) at most.
4. **Selection is legitimate; editing is not.** Choosing among seeds/worlds
   against stated criteria is curation of valid solutions. Editing a solved
   surface is forbidden at every tier.

## Goals audit (recorded verdicts on the practical goal list)

| Practical goal | Verdict |
|---|---|
| Realistic-but-interesting terrain | Accept: forcing authorship + seed selection |
| Biome spectrum → rock/grass/forest | Accept: emergent; compression is declared tier-2 |
| Decide species | Counter: species pools are downstream content tables; SDM without occurrence data would be fake science |
| Foliage sparseness/clustering | Split: density spectrum is core; clustering is a tier-3 point process or consumer-side |
| Humanity sim (tunnels/bridges/reclamation) | Accept: it is the thesis; interventions are tier-1 economics with tier-2 cost weights |

## Consequences

- The four moisture-sweep constants hardcoded in `climate.rs` (evaporation,
  convective rainout, orographic coefficient, airflow smoothing length)
  were undeclared forcing → promoted to `TerrainConfig` with unchanged
  defaults (this ADR's companion change).
- `via-ecology` docs must mark Whittaker classes as *standard* and the
  vegetation attribute formulas as *heuristic*.
- Existing gate set is untouched: gates measure tier-1 statistics only.
  *(Note, 2026-08-20: ADR 0008 D1 re-labels the band-scored members of
  that set — θ, Hack, Horton R_b — as benchmarks; the tier rule stands,
  the instrument names changed.)*
