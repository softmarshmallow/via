# Research 0011 — Open questions

Status: research corpus — pre-decision. Load-bearing questions the
survey did NOT answer. Any adopting ADR must either resolve the
relevant entries or declare them as limitations. Part of
docs/research/humanity (see README).

## Design problems with no literature template (via's own work)

- **Two-scale coupling contract.** No literature template exists for
  how settlement-layer mass W_j maps onto intra-city demand fields
  (platting rate, massing demand, movement budget). This interface
  is via's own design problem and needs its own ADR treatment.
- **Single-seed vs ensemble gating.** Every published band is
  cross-sectional or ensemble-mean; a bitwise-deterministic single
  run passing a distributional gate is a different claim. A declared
  protocol (n-seed ensemble per gate? widened bands?) is required
  before any gate is scored.
- **City/settlement delineation algorithm.** Rank-size, scaling, and
  density gates all shift with delineation (ζ 0.90 vs 1.17 from
  delineation alone). The survey says "fix one" but chose none
  (candidate: CCA with a declared threshold) — must be frozen before
  gate numbers mean anything.
- **Era transitions.** All sources give era SNAPSHOTS; whether
  parameters move as continuous paths or dated switches — and what
  happens mid-transition (e.g. the 1857–1885 height regime) — is
  undetermined. It matters because Harris–Wilson and Fujita–Ogawa
  have bifurcations, so trajectory shape changes outcomes.
- **Equilibrium-selection protocol.** Seeding each era's solve from
  the prior era's state is the proposed path-dependence device, but
  it is a modeling choice with no validation literature; needs
  explicit ADR treatment including sweep-order/damping determinism.
- **Macro closure.** What supplies total regional population/economy
  growth per era (the O_i budget Harris–Wilson allocates)? Candidate
  is a declared demographic forcing series plus the graveyard
  mechanism, but the survey did not propose the closure. Uncollected
  lead: Verbavatz & Barthélemy 2020 (migration-shock growth
  equation).

## Missing empirical inputs

- **Wall unit cost c_w.** The mechanism (cost ~ perimeter, artillery
  multiplier) is citable, but no compiled empirical cost series
  exists; needs primary murage/fortification-accounts work, or the
  parameter stays an undimensioned forcing scalar.
- **Burgage-cycle quantitative targets.** Climax coverage fractions
  and fallow durations exist only in Conzen's Alnwick tables and
  Scrase's Wells case study — digitization from primary sources is
  required before the coverage-ODE gate has numbers.
- **Movement-to-land-use multiplier strength** (the Hillier
  centrality-as-process feedback): no published value anywhere; it
  would be a free declared parameter, flagged as such.
- **Medieval density-gradient numerics do not exist** (walls, tax
  rolls, no fitted exponentials): the medieval interior gate must be
  structural (compactness, density jump at the wall, rich-center
  sorting, A ~ P^0.85) — the ledger should say so rather than invent
  a b-band.
- **Quantitative water-supply link.** The mortality/carrying-capacity
  mechanism needs potable-water capacity from via's
  hydrology/lithology, but no source quantifies a wells/springs →
  population-cap conversion.
- **Pack/foot maximum grade.** Turnpike ~5% and rail 1–2.2% are
  verified; the pre-wagon (pack animal, footpath) grade tolerance
  has no engineering-standard source — currently a soft number in
  the cost surface.
- **Cross-era cost normalization.** Nominal cents/ton-mile are not
  comparable across centuries; freight and commuting costs need a
  real-terms expression (wage-relative or value-share), and the
  survey supplied no conversion series.

## Scope and verification debts

- **Non-Western generality.** Essentially all gates come from
  Europe/US/China; the framework's claims should be scoped to those
  settlement systems or the gap declared.
- **Unverified digits flagged by the survey itself.** The exact
  Makse et al. 1998 satellite-area exponents and the Cardillo 2006
  per-sample meshedness table were not re-extracted from full text —
  verify before hard-coding either gate.
