# ADR 0002 — Climate coupling and the ecology stage

Status: accepted (2026-08-13)
Scope: M2 — how climate enters the terrain stage, how erosion consumes it,
and how biome/vegetation attributes are derived. Also records two model
corrections found during the island8k experiment.

## Decision 1 — Climate is computed inside the terrain stage

The causal chain is rooted at uplift/climate, but orographic precipitation
depends on the topography it helps shape. The loop is resolved *inside* the
terrain stage: precipitation is recomputed against the evolving surface
every step, so the stage boundary stays one-directional and downstream
stages see a single consistent (terrain, climate) pair.

- **Wind** is a seed-derived boundary condition (one of 8 directions), like
  uplift.
- **Precipitation**: single-layer moisture advection along the wind — air
  saturates over open water, rains out a convective fraction everywhere and
  an orographic fraction proportional to forced lift, and dries crossing
  ridges. Lift is measured against an exponential moving average of the
  terrain (~600 m): air columns ride over gullies, and without the smoothing
  rainfall traces every valley wall at fine cell sizes. The field is
  normalized to a configured land mean, then cross-wind blurred.
- **Temperature**: sea-level base − lapse·elevation + weak meridional
  gradient + low-amplitude noise. Mean-annual values; seasonality is out of
  scope for M2.

Rejected alternative: climate as a separate upstream stage. It would either
ignore the terrain (no rain shadow) or require a stage cycle, which
CONTRIBUTING forbids.

## Decision 2 — Erosion runs on discharge, not area

Drainage accumulation is precipitation-weighted (each cell contributes its
precipitation relative to the land mean, floored at 0.05 so full rain
shadow still routes a trickle). The stream-power term is K·√Q·S with Q in
equivalent cells, so K keeps its calibration. Consequences, all deliberate:

- Wet windward flanks erode faster than lee flanks — asymmetry the README's
  thesis demands.
- The slope–area gate regresses against **discharge** (with variable
  precipitation the steady state is S = U/(K√Q); regressing on raw area
  would mix climates). River extraction and Strahler orders also use
  discharge; Hack's law keeps planimetric area.
- The steady-state residual gate is the full balance
  |（K·√Q·S − κ∇²h）/U − 1| — at fine cell sizes the hillslope flux into
  valleys rivals uplift and must not be misread as disequilibrium.
- A **rain-shadow gate** (advisory): mean land precipitation in the upwind
  half over the downwind half of the wind axis, expected > 1.

## Decision 3 — Ecology is a downstream stage over artifacts

`via-ecology` reads terrain artifacts (heights, precip, temperature,
discharge, fill depth) and derives:

- **Biome classes** from the Whittaker (1975) mean-annual T × P diagram
  (documented piecewise approximation), with overrides that are themselves
  measurements: standing water (ε-fill ponding depth) or high topographic
  wetness → wetland; steep slopes → bare rock. No class is ever authored.
- **TWI** = ln(a/tanβ) over the discharge field.
- **Vegetation attributes** (fields, not placements): density from
  moisture × warmth with a slope penalty; canopy height as a per-biome
  ceiling scaled by density. Downstream consumers (or a game) scatter
  assets; via ships distributions.

Artifacts: `biome` (u32 classes), `veg_density`, `canopy_height`, `twi`
(f32) + `ecology.json` with the legend and class areas. Terrain gained
`precip`, `temperature`, `discharge`, `fill_depth` (f32).

**Land-mask contract**: downstream stages derive land from the `receivers`
artifact (self-receiver = base level = water; the pit gate guarantees no
land self-receivers), never from the cm-quantized heights — quantization
flips shoreline cells within ±0.5 cm of the datum. All renderers and both
downstream stages follow this. Rivers, widths, and flow accumulation all
use the discharge metric end to end.

## Model corrections recorded (found via the island8k experiment)

1. **Subsidence floor.** Negative uplift now stops at `base_depth_m`.
   Without it, open-ocean cells with U < 0 deepen linearly forever and
   hillslope diffusion drags every subsidence-coast cell down each step — a
   permanent churn band that poisoned convergence at any resolution.
2. **Operator-splitting limit.** Erosion and diffusion are split within a
   step; the split is only faithful while κ·∇²h·dt ≲ U·dt. At 16 m cells
   this bounds dt to ~2×10³ yr (with κ ≈ 0.005 m²/yr). Configs must respect
   this; the residual gate is the tripwire that catches violations.
