# ADR 0006 — Implicit erosion–deposition solve (Yuan et al. 2019)

Status: accepted. Foundation work for M4 (lithology & structure).

## Context

M3 (ADR 0004) added the deposition half of Davy & Lague (2009) as an
**explicit** flux-routing sweep after the implicit erosion solve. That
scheme is correct but carries two costs, both recorded in ADR 0004:

1. **A timestep limit.** The erode-then-deposit operator splitting
   oscillates unless G·U·dt stays small against valley relief; the
   research preset had to drop from dt = 1e5 yr (M2) to 1e4 yr with 3000
   steps — a 7.5× cost purely for numerical stability.
2. **A hard aggradation cap.** Deposition was clamped below the lowest
   donor (`DONOR_MARGIN_M`) because the sweep mutated heights mid-pass
   and a within-pass graph inversion would corrupt the bookkeeping.

M4 needs per-unit erodibility contrast (bedrock K vs sediment K, then
lithology). Yuan et al. (2019) is the field's method for exactly this:
an O(N), implicit, iterated solve of the coupled erosion–deposition
equation, and the scheme their paper recommends for K contrast. This ADR
migrates the numerics **without changing the physics claims**: same
equation, same G, same deposition domain, same lake contract.

## The scheme

The governing equation is unchanged (ADR 0004):

∂h/∂t = U − K·√Q·S + G·q_s/Q̃   (fluvial domain; Q̃ = discharge, cell units)

Yuan et al. solve it as a fixed point: within one timestep, the erosion
field E and deposition field D must agree with each other — E computed
implicitly on slopes that include D's aggradation, D computed from the
sediment flux that E supplies. Their Gauss–Seidel iteration is, in our
structure, the two sweeps M3 already had, **iterated until they agree**:

- **Sweep A (erosion, receivers-first over the MFD DAG):** the
  Braun & Willett implicit relaxation of ADR 0005, with the source
  elevation lifted by the previous iterate's deposition:
  elev_i = ht_i + D_i. Base cells are pinned; flooded cells do not
  incise; rivers grade to water surfaces (all as before, ADR 0004).
  Detachment E_i = elev_i − h_i ≥ 0.
- **Sweep B (flux routing, donors-first):** M3's bookkeeping sweep,
  unchanged in its rules — dry fluvial cells keep the min(G/Q̃, 1)
  fraction of incoming flux, flooded cells trap up to the water level,
  hillslope cells pass through, base cells export — but it no longer
  mutates heights; it just produces the next D field and the budget.
- **Assemble** h_i = ht_i + D_i − E_i and iterate until
  max |Δh| between iterations ≤ 1e-6 m (hard cap 100 iterations, then
  panic — a solver that stops converging is a defect, not a warning).

At convergence this is exactly Yuan et al.'s solution: erosion evaluated
on the step's final slopes, deposition on the step's final fluxes,
simultaneously. The reference implementation (fastscapelib-fortran,
`StreamPowerLaw.f90`) has the same structure — including multi-receiver
weighted flow, which independently validates the ADR 0005 DAG
generalization.

Mass closure is an identity of Sweep B's bookkeeping, as in M3: the
budget from the **final** sweep is the budget of the heights actually
applied, so detached = deposited + exported to numerical precision and
the 1e-9 core gate is unchanged.

## What changes

- **The dt limit dies.** The splitting oscillation was between the two
  sweeps; iterating them to a fixed point removes it. The research
  preset returns to its M2 calibration (dt = 1e5 yr, 400 steps),
  validated by gates, in a separate commit from the scheme migration so
  each is auditable alone.
- **The donor-floor cap (`DONOR_MARGIN_M`) is dropped.** It protected a
  single mutating pass that no longer exists. If converged deposition
  fills a reach above a donor, that is real aggradation; the next step's
  priority-flood sees the inversion and floods or reroutes it — the
  same contract every other within-step surface change already follows.
- **Iteration count** is deterministic (sequential sweeps, sequential
  max-fold convergence test), so bitwise determinism is preserved by
  construction.

## What deliberately does not change

- **Deposition stays restricted to the fluvial domain.** ADR 0004's
  dome-smearing evidence was gathered under the explicit scheme, whose
  one-cell-per-step creep artifact the fixed point removes — under the
  implicit scheme, deposition-everywhere is the faithful Davy & Lague
  configuration and exactly what fastscape runs. Re-examining the
  restriction is now legitimate but is **its own physics change** with
  its own calibration consequences (hillslope steady slopes would become
  transport-limited), so it goes on the ledger, not into this
  migration. One numerics change at a time.
- The min(G/Q̃, 1) cap stays: with the precipitation floor (0.05), rain-
  shadow cells can have Q̃ < G, and an uncapped G/Q̃ fraction would
  deposit more than arrives. Declared numerical guard, as in M3.
- Lake contract (ADR 0004): frozen flooded mask, trap-to-water-level,
  first-cell-aggrades delta progradation. Headroom is measured against
  the start-of-step surface, so it is iteration-order-free.
- K keeps its M2 calibration (ADR 0004's lesson 4), the hybrid MFD
  routing and two-pass build (ADR 0005), climate recomputed per step,
  and every gate definition.

## Consequences

- Per-step cost rises (typically a handful of sweep pairs instead of
  one), repaid ~10× by the dt restoration at research scale.
- **Measured at migration** (both scales, dt unchanged, all core gates
  pass): worst-case Gauss–Seidel count 6 (island8k) / 7 (research);
  wall time +26% / +33%. Dropping the donor-floor cap releases real
  valley fill — mean sediment blanket 0.87 → 1.37 m at research — and
  shifts the sub-grid ponding it used to suppress into the pond-merge
  meter (island 22 k → 40 M m³ = 3.6% of deposition; research
  16.7 M → 98.9 M m³ = 1.9%): the declared-mass-source meter is doing
  exactly its job, and these fractions are the honest price of
  permitting within-step aggradation. Lakes moved both ways, as the
  transient-lake contract predicts: the island seed-6 lake (9 cells at
  M3.5) filled completely; research gained 40 lake cells where
  aggradation dammed reaches past the 0.5 m persistence threshold.
  At island8k the blanket thinned (5.90 → 3.96 m): the fixed point
  re-erodes within-step deposits that the explicit scheme's
  once-per-step ordering let stand.
- M4 unlock: Sweep A already takes K per edge evaluation; per-cell K
  from a lithology column is now a local change.
- Ledger (ROADMAP): deposition-everywhere re-examination (faithful
  Davy & Lague hillslope behavior vs the κ-diffusion term's role —
  decide with gates, not taste).

## References

- Yuan, X. P., Braun, J., Guerit, L., Rouby, D., Cordonnier, G. (2019).
  A new efficient method to solve the stream power law model taking
  into account sediment deposition. JGR Earth Surface 124, 1346–1365.
- Davy, P., Lague, D. (2009). Fluvial erosion/transport equation of
  landscape evolution models revisited. JGR 114, F03007.
- Braun, J., Willett, S. D. (2013). A very efficient O(n), implicit and
  parallel method to solve the stream power equation. Geomorphology 180.
- fastscapelib-fortran `StreamPowerLaw.f90` (reference implementation).
