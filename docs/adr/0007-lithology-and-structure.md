# ADR 0007 — Lithology & structure (M4)

Status: accepted.

## Context

Through M3.5 the substrate is mechanically uniform: one K, one κ,
everywhere. Real terrain is unmistakably structured because rock is not
uniform — cliff bands hold on resistant units, rivers knick where they
cross contacts, drainage turns trellis on tilted strata, escarpments
walk back from faults. FAQ 0005 lists these landform classes as locked
behind "uniform substrate"; this ADR unlocks them. The vehicle is the
ADR 0006 implicit solve, which already evaluates K per edge — making
per-cell K a local change.

## Representation: a deformed layer-cake in the material frame

Lithology is **tier-2 forcing** (ADR 0003), exactly like the uplift
field: geometry is declared in config, never derived, and the modules
propagate it mechanistically. The model is the standard LEM treatment
(layered erodibility after Forte & Whipple 2016; Perne et al. 2017):

- **The column.** N units listed top-down from a datum elevation; each
  has `thickness_m` and per-unit `k_mult`, `kappa_mult`,
  `solubility ∈ [0, 1]`. The top unit extends upward without bound, the
  bottom unit downward (basement). Units are indices, not names —
  spectra over taxonomy applies to rocks too.
- **Structure.** A static deformation field d(x, y) displaces the
  column: regional dip (a plane through the grid centre), fold trains
  (sums of sinusoids: amplitude, wavelength, azimuth, phase), and
  faults (vertical throw across a line trace; the discontinuity is
  left sharp — its surface expression must *emerge* from differential
  erosion, never be drawn). d is precomputed once per run.
- **The material frame.** Rock advects vertically with uplift, so the
  exposed unit is looked up at the *stratigraphic* elevation
  `s = z_bedrock − Σ(applied uplift) − d(x, y)`, where z_bedrock =
  h − sediment. The applied-uplift sum is bookkept exactly per cell
  (the subsidence floor of M1 clamps some ocean cells, so U·t would be
  wrong there); erosion moves s down through the column, pure uplift
  leaves it fixed — rock rides up rigidly. Horizontal tectonics,
  flexure, and lithification of deposits are out of scope: deposited
  sediment remains `sediment_m` cover forever, one material.
- **Frozen per step.** The exposure lookup happens once per step on
  the step's starting surface, like the flooded mask (ADR 0004): K and
  κ must not be re-derived mid-solve from the evolving surface.

## What the physics does with it

- **Erodibility.** The solve's per-cell K is
  `k_spl × k_mult(exposed unit)`, or `k_spl × sediment_k_mult` where
  the frozen sediment cover exceeds `sediment_cover_min_m` — the
  Davy & Lague full form's K contrast, shipped with a **neutral
  default** (`sediment_k_mult = 1`) so M2/M3 calibration is untouched
  until an experiment declares otherwise.
- **Diffusivity.** Per-cell κ = kappa_mult(exposed unit) × κ. Variable
  κ uses a mass-conserving symmetric edge form (flux κ_ij = ½(κ_i+κ_j)
  on the 4-neighbour graph, subcycled to the max-κ stability limit).
  When every multiplier is 1 the code takes the **existing uniform
  path unchanged** — homogeneous runs stay bitwise identical, which is
  a regression test, not a hope.
- **Karst potential.** `solubility(exposed) × discharge` — a spectrum
  artifact. Where water crosses soluble rock, dissolution is possible;
  cave geometry is downstream content, not via's claim (FAQ 0005).

## Gates

- **SPL residual (core, unchanged bounds)** now evaluates each cell
  against its *own* K — with heterogeneous K this is precisely the
  lithology-consistency check: the landscape must balance the equation
  it was evolved under, unit by unit.
- **Slope–area θ (core, unchanged bounds)**: when more than one unit is
  exposed in the regression population, the regression restricts to
  the modal exposed unit — mixing K regimes in one log-log fit measures
  the column, not the incision law. With one unit this is a no-op
  (bitwise-identical samples).
- **Unit SPL consistency (new, advisory)**: per exposed unit with
  enough fluvial cells in the uplift band, the median of K·√Q·S/U
  should be a unit-independent constant near 1; the gate reports
  max/min across units. Advisory because transient reaches (knickzones
  migrating through contacts — the very feature we want) legitimately
  deviate; it is reported so drift is visible, promoted only if it
  proves stable.

## Artifacts

`lithology` (u32 exposed-unit index on the final surface) and
`karst_potential` (f32) join the raster set, always emitted (a
homogeneous run writes unit 0 and zeros — contract stability over
file-count thrift), hashed in gates.json and the manifest.

## Rejected

- **Named rock types / stratigraphic content** (sandstone, limestone):
  units carry mechanical parameters and indices; naming is downstream
  content (ADR 0003).
- **3-D stratigraphy of new deposits** (lithified fills, growth
  strata): sediment stays one cover field; revisit if a consumer needs
  more than bedrock-vs-cover.
- **Horizontal advection, flexural isostasy**: different tectonic
  machinery, separate forcing decision, not smuggled in here.
- **Cave/void geometry from karst potential**: the spectrum is the
  honest boundary of the claim.

## References

- Forte, A. M., Yanites, B. J., Whipple, K. X. (2016). Complexities of
  landscape evolution during incision through layered stratigraphy.
  Earth Surf. Process. Landforms 41, 1736–1757.
- Perne, M., Covington, M. D., Thaler, E. A., Myre, J. M. (2017).
  Steady state, erosional continuity, and the topography of landscapes
  developed in layered rocks. Earth Surf. Dynam. 5, 85–100.
- Davy, P., Lague, D. (2009). JGR 114, F03007 (K contrast form).
- ADR 0003 (tiers), 0004 (physics contract), 0006 (the solve).
