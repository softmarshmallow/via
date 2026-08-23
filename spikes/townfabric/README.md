# SPIKE — town fabric (streets, blocks, lots, buildings)

**Status: research spike. Not a stage, not gated, not adopted.** It
answers a question the macro spike could not: what a settlement looks
like at the scale a person walks through. `spikes/settlements` stops at
a dot with a population; this takes one of those dots and grows the
fabric.

```bash
cargo run --release -p via-spike-townfabric -- runs/m4c-research-s42 \
  --config spikes/townfabric/eras/three-fabrics.json \
  --out runs/spike-townfabric-s42 \
  --from-spike runs/spike-settlements-s42/spike.json --era medieval --rank 0
```

## The chain

1. **Terrain patch** — heights, water and slope sampled from the terrain
   run into a local metric frame at the town site. River channels are
   carried as *lines* whose width follows hydraulic geometry
   (w ≈ k·√A, the downstream form of Leopold & Maddock 1953), because a
   200 m DEM cell would otherwise make every stream 200 m wide — wider
   than any pre-modern bridge, which silently made medieval towns
   impossible to found.
2. **Corridor seeding** — the intercity roads that met here are laid
   first, bridging water only as far as the era can span. The town grows
   on the roads, not the other way round.
3. **Platting (forcing, one shot)** — an era that surveys stamps a
   lattice: block module, bearing, radius. The survey is terrain-blind;
   which parts of it get built is not.
4. **Organic growth** — Courtat, Gloaguen & Douady (2011): a rejection
   tube λ₀ around existing streets, sites placed at the potential's
   optimum with probability P_e, the shortest connection always built and
   each further one with probability ω. ω near 0 grows trees and dead
   ends; ω near 1 grows loops and grids.
5. **Extent from population** — streets are laid, the blocks they
   enclose are subdivided into lots, and the frontier moves out only as
   far as the households housed so far require. When the fabric inside
   the frontier can take no more, the town extends. Growth stops when
   the town's people are housed.
6. **Parcels** — recursive OBB subdivision (Vanegas et al. 2012): a
   block deeper than two lots is cut into back-to-back rows, then halved
   along its frontage until the era's target width is met. Binary
   halving is what reproduces the half- and quarter-width structure
   Slater measured in burgage series.
7. **Buildings** — each parcel's OBB inset by the era's setbacks, capped
   by its coverage ratio; storeys follow a negative-exponential access
   gradient (Clark 1951) under the era's height cap.

## Validation

**Read [VALIDATION.md](VALIDATION.md) first.** The numbers in the table
below were computed on an unsimplified graph and compared against
literature bands that do not describe real towns measured under a fixed
protocol. They are retained only as a record of what the spike produced
on 2026-08-19; the standard, the protocol and the measured gaps against
four real towns live in VALIDATION.md.

## Findings (2026-08-19, seed 42, medieval town rank 0 of the macro run) — SUPERSEDED

| | meshedness | dead ends | median lot | frontage | GSI | FSI |
| --- | --- | --- | --- | --- | --- | --- |
| medieval organic | 0.177 | 28.7% | 131 m² | 8.5 m | 0.49 | 1.03 |
| frontier plat | 0.248 | 25.5% | 230 m² | 8.3 m | 0.27 | 0.43 |
| modern zoned | 0.045 | 35.3% | 811 m² | 21.5 m | 0.27 | 0.97 |

- ~~Medieval meshedness 0.177 sits inside the organic band~~ —
  **retracted**, see VALIDATION.md: unsimplified graph, and the band
  does not describe real Alnwick under a fixed protocol.
- Median frontages land on their declared metrology — burgage ~8.5 m
  (≈2 perches), plat 8.3 m, modern 21.5 m (≈70 ft) — which is a check
  that the subdivision honours the module, not a discovery.
- GSI 0.49 with FSI 1.03 puts the medieval core in the dense low-rise
  region of the Spacematrix plane, where historic cores sit; the modern
  town reaches a similar FSI at a quarter of the coverage, which is the
  right direction of travel.

## What is emergent, and what is not

Emergent: where streets run, where blocks form and how big they are,
every lot boundary, every footprint, the town's outline, and which side
of the river the town takes.

Declared: λ₀, P_e, ω, the plat module and bearing, lot frontage and
depth targets, setbacks, coverage, storey gradient and cap, and the
packing fraction that converts households into built radius.

**Known weaknesses — read before believing a number:**

- **Lots exist only inside closed blocks.** Tree-like fabric (the
  modern cul-de-sac case) encloses few blocks, so lots along dead-end
  strings are never generated and the modern town is under-housed
  relative to its declared households. Frontage-strip parcelling is the
  fix and is not implemented.
- **No land use.** Everything is housing. There is no commerce, no
  market square, no church, no industry — so no functional centre, and
  the storey gradient is a stand-in for a land market, not a land
  market.
- **No history.** Each era is grown from scratch on the same site. The
  palimpsest — a medieval core with a plat bolted on and a modern
  suburb around it — is exactly what docs/research/humanity/0008 says
  must happen and exactly what this does not do.
- **Terrain is 200 m coarse.** Slope, river position and the buildable
  mask are interpolated from the DEM, so the fabric responds to valleys
  but not to anything smaller than a DEM cell.
- **Buildings are rectangles** from the parcel's bounding box; no
  courtyards, no L-plans, no party-wall geometry.
- **The site is nudged** up to 500 m to find dry ground, because the
  macro spike's site cell can be the river cell itself.
