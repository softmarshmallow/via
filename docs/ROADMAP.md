# via roadmap — the nature side, layer by layer

Status: living document. Records the agreed layer order, the method each
layer follows, and the gate that can falsify it. Humanity stages (suitability
refinement, settlements, corridors, networks, epochs) are deliberately parked
until the natural substrate is deep enough; see README for the full causal
chain.

## Standing doctrine (see ADR 0003)

- **Modules never consider game or practical use.** Each stage is built as if
  for a research consumer. Practical adaptation happens outside the crates:
  in `experiments/` configs (declared forcing) and in downstream consumers.
- **Outputs are spectra, not authored taxonomy.** Stages emit continuous
  fields (depths, densities, fluxes, indices). Discrete classification is
  allowed only where academia recognizes the scheme (e.g., Whittaker biomes,
  Strahler orders, geomorphons) and the classifier is cited and fixed.
- **Three epistemic tiers**: process (gated, falsifiable), forcing (exogenous,
  declared in config, never hardcoded), interpretation (optional, labeled).
  Gates exist only for process. Forcing distortion (e.g., game-scale climate
  compression) is legitimate exactly because it is declared.

## Layers

### M1 — Terrain (DONE)
Stream-power LEM: ∂h/∂t = U − K√Q·S + κ∇²h, Braun & Willett 2013 implicit
solve, priority-flood+ε (Barnes 2014), D8, signed uplift forcing.
Gates: slope–area θ ∈ [0.40, 0.60], Hack h ∈ [0.45, 0.70] over subbasins,
Horton Rb ∈ [3, 5] in largest basin, pits = 0, completeness = 1, SPL
residual ≤ 0.35 (core; operator-splitting tripwire). ADR 0001.

### M2 — Climate & ecology (DONE)
Climate inside the terrain stage (forcing + transport): seed-derived wind,
single-layer moisture advection with orographic rainout (600 m EMA airflow),
lapse temperature. Erosion runs on precipitation-weighted discharge.
Ecology stage: Whittaker (1975) T×P classes, wetland/bare-rock overrides as
measurements, TWI, vegetation density/canopy spectra. ADR 0002.

### M3 — Sediment & standing water (NEXT)
The two known fakes in the substrate die here: ε-fill plains (no deposition)
and no lakes.
- Method: erosion–deposition after Davy & Lague 2009 in the G-coefficient
  form of Yuan et al. 2019 (deposition rate G·q_s/q); sediment routed down
  the receiver tree; depressions hold water — flow is routed across the
  filled surface while the true surface evolves beneath it (Cordonnier et
  al. 2019 is the reference for depression routing; the filled-copy scheme
  is the grid-scale equivalent).
- Two surfaces: h = bedrock + sediment thickness. Deposition builds
  floodplains, fans, deltas (progradation into lakes and the sea).
- Gates: sediment **mass closure** (eroded = stored + exported, core),
  slope–area θ measured on bedrock channels only, lake level = spill
  elevation (consistency), floodplain slope (advisory). ADR 0004.
- New spectra: sediment_m, water_depth_m.

### M4 — Lithology & structure
Layered rock units with per-unit erodibility K, diffusivity κ, solubility;
unit geometry (dip, folds, faults) is tier-2 forcing like uplift. Emergent:
cliff bands, waterfalls, escarpments, trellis vs dendritic drainage on
tilted strata. Karst potential = soluble unit × water flux (a spectrum;
cave geometry is downstream content, not via's claim).
Gate candidates: per-unit relief/slope contrast, drainage-pattern statistics.

### M5 — Soil, regolith & microclimate
Regolith production (Heimsath et al. 1997 exponential decline with depth),
slope-dependent transport → soil depth spectrum; aspect/slope insolation
(solar geometry, no new forcing) modulating temperature and moisture →
north/south vegetation asymmetry, treeline detail.
Gates: depth vs curvature/slope relations from the hillslope literature
(thin ridges, thick hollows).

### M6 — Coastal processes
Wave fetch/exposure field × lithology × fluvial sediment supply → cliffs,
beaches, dunes, spits. Honesty flag recorded in advance: the cheap version
(exposure classification) is descriptive tier-3; process coastline evolution
(Ashton & Murray 2006) is the upgrade. Decide depth when reached.
Gate candidates: beach occurrence vs exposure × supply.

### M7 — Resolution & meshes
16 m → 1–2 m playable surface. Honest hierarchy: nested fine-scale physics
on tiles (process, expensive) over drainage-preserving amplification
(tier-3, declared). Hard gate either way: amplified terrain must route
water identically to the coarse solution and preserve per-landform slope
distributions, or it fails. River bank/terrace geometry from the M3
sediment layer.

### M8 — Ecology v2
Fire/disturbance frequency from dryness (closes the savanna bistability gap
vs the pure climate envelope — Staver 2011, Bond 2005), succession age,
riparian corridors. Species pools are downstream content tables, never a
via module's claim; via ships mixture-weight spectra per biome at most.

## Standing upgrade ledger (recorded, not scheduled)

- Smith & Barstad 2004 linear orographic precipitation (better rain fields).
- Parametric seasonality → Köppen classification becomes available.
- Holdridge via PET.
- K contrast bedrock vs sediment (Davy & Lague full form) once M4 exists,
  via the Yuan et al. 2019 implicit erosion–deposition solve (also removes
  the explicit-deposition dt limit of ADR 0004).
- Marine sediment transport (shelf deposition) beyond M3's at-mouth
  progradation.
- Research-preset dt/κ retune (residual currently carries mild splitting
  inflation at 200 m cells).
- Geomorphons (Jasiewicz & Stepinski 2013) as a descriptive landform grammar,
  any time — useful for QA and downstream consumers alike.
