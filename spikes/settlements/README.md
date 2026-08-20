# SPIKE — settlement systems across eras

**Status: research spike. Not a stage, not gated, not adopted, not
depended on by anything.** It exists to test one claim from
[docs/research/humanity](../../docs/research/humanity/README.md) cheaply
enough to throw away: that **era is forcing, not a different model** —
that one mechanism chain reproduces medieval, frontier and modern
settlement patterns when only its declared parameters change.

It reads a finished terrain run's artifacts and writes figures plus a
stats record. It writes nothing into the run directory, defines no
artifact format, and is deliberately outside `crates/`.

```bash
cargo run --release -p via-spike-settlements -- runs/m4c-research-s42 \
  --config spikes/settlements/eras/three-eras.json --out runs/spike-settlements-s42
```

## The chain

Four mechanisms, run identically for every era:

1. **Food base** — per-cell carrying capacity as a product of slope,
   soil depth (M3 sediment), moisture, warmth (M2 climate) and distance
   to fresh water. This is the origin field the dynamics allocate.
2. **Travel cost (hours)** — one exponential slope-penalty form with a
   hard grade cap applied *per step*, so routes may climb by following
   contours; water carries its own speed and boarding costs a
   transshipment delay. Costs are in hours, not kilometres, so an
   era-invariant time-decay expresses Marchetti's travel-time budget as
   a mechanism rather than a second knob.
3. **Center dynamics** — Harris & Wilson (1978): T_ij allocates demand
   by A_j·exp(−β c_ij), and dW_j/dt = ε(D_j − κW_j), with attractiveness
   A_j = Q_j·W_j^α. Centers below a viability floor die and stay dead.
4. **Corridors** — links in descending interaction volume, built as
   least-cost paths over a surface that discounts already-built cells
   (reuse bundling), skipping pairs the network already connects within
   a detour tolerance.

Everything is sequential and order-fixed except the per-candidate
Dijkstras, which are independent rows.

## What is genuinely emergent, and what is not

Emergent: how many centers survive, where they stand, their spacing and
size hierarchy, which of them are ports, and every metre of road.

Declared (forcing): era speeds, grade caps, water speeds, transshipment
delays, α, β, the viability floor, and the food-base coefficients.

**Not derived at all, and load-bearing** — read these before trusting a
number:

- **The viability floor** (200 people) sets the scale of "what counts as
  a settlement" and, with the food base, bounds the possible center
  count. The spatial pattern is emergent; the count is floor-influenced.
- **The culling rule** (warm-up, then at most 4% of live centers per
  sweep) is an artifact-avoidance device, not physics. Culling from the
  first sweep empties the map in one step, because equal-seeded centers
  are all below the floor simultaneously. Nothing in the literature
  prescribes this rule.
- **The congestion term** `w_congestion` is a free parameter with no
  empirical value. It is the reduced form of the dispersion force
  Fujita & Ogawa (1982) carry as internal commuting cost; centers here
  are dimensionless points, so their crowding has to be asserted.
- **The delineation** (single-linkage at 5 km) decides what counts as
  one place, and every distributional statistic moves with it —
  docs/research/humanity/0011 records this as unresolved.
- **Total population is held equal across eras.** That makes the
  comparison a controlled experiment (only transport and agglomeration
  differ) but it is not history: real eras differ in population too.
- **Candidate thinning** at 1.5 km is a discretization; emergent spacing
  must stay well above it or the spike is measuring its own grid.

## Findings (2026-08-17, seed 42 research terrain)

| | centers / agglom | spacing | ζ (agglom) | primacy | travel |
| --- | --- | --- | --- | --- | --- |
| medieval | 38 / 32 | 7.4 km | 1.14 | 7.5% | 2.03 h |
| frontier | 10 / 10 | 16.8 km | 0.70 | 23.3% | 1.78 h |
| modern | 23 / 7 | 2.1 km | 0.83 | 5.4% | 0.94 h |
| modern, no dispersion | 1 / 1 | — | — | 100% | 0.94 h |

- Medieval spacing (7.4 km) lands inside the 5–10 km realized
  market-spacing range, and its ζ = 1.14 inside the 0.8–1.3 band, from
  transport parameters alone.
- Frontier spacing (16.8 km) lands in the 11–18 km rail-town band, but
  its ζ = 0.70 is below the band: too few centers, too equal.
- **Without a dispersion force the modern era collapses to one center**,
  and it does so at α = 1.0 as well as α = 1.15 — so the collapse is not
  agglomeration returns. Fast travel flattens exp(−βc) to near-uniform,
  the region becomes one undifferentiated market, and the culling rule
  resolves an indifference rather than a mechanism.
- With the congestion term the modern era becomes polycentric (23
  centers in 7 agglomerations) — but the centers cluster at 2.1 km
  spacing because a point-wise size cap makes the best response to a
  full center a clone next door. A hard cap is the wrong reduced form.
- Mean travel time falls across eras (2.03 → 0.94 h) instead of staying
  fixed: Marchetti's constant does **not** emerge here, and cannot,
  because population and region are fixed so the freed time has nothing
  to be spent on.

## What this spike does not model

No street network, no plots, no buildings, no land use or density — the
entire urban-morphogenesis half of the corpus. No rail as built
infrastructure (the frontier era's most important mode). No founding
events, no planning templates, no path dependence between eras: each
era is solved from scratch, which is exactly the "no palimpsest"
condition docs/research/humanity/0008 rejects. Sizes are people
allocated by a service-center model, not populations from a demographic
model.
