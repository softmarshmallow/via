# Experiment: island8k

A game-scale spike: one 8.192 km × 8.192 km island (512 × 16 m cells) with at
least one region a village could plausibly occupy.

The core stays agnostic: everything specific to this experiment is the two
config files in this directory. No crate knows the word "village" — the
suitability stage measures generic affordance fields (slope, freshwater
distance, coast distance, elevation) and extracts contiguous patches passing
the thresholds in `suitability.json`; this experiment merely labels the gate
`village_site`.

**No authored terrain.** "Ensure a region for villages" is deliberately NOT
implemented as a terrain constraint (CONTRIBUTING: nothing authored that can
be derived). The guarantee comes from selection by measurement: sweep seeds,
keep those whose terrain gates and suitability gate both pass, pick the best.
Same seed → same island, bitwise; the chosen seed is the shippable asset.

## Physics at this scale

Constants in `terrain.json` were calibrated for the 16 m cell size
(see git history of the spike): K = 3.2e-5, κ = 0.015, U ≤ 5e-4 m/yr,
10 Myr of evolution, relief ≈ 250–350 m. At 16 m cells the hillslope
diffusion flux into valley floors is first-order relative to uplift — the
steady-state residual gate in core accounts for it (|K√A·S − κ∇²h|/U − 1|).

## Protocol

```bash
# sweep
for s in 1 2 3 4 5 6 7 8 9 10; do
  cargo run --release -p via -- terrain \
    --config experiments/island8k/terrain.json --seed $s --out runs/island8k-s$s
  cargo run --release -p via -- suitability runs/island8k-s$s \
    --config experiments/island8k/suitability.json
done
# select: highest-ranked run with OVERALL PASS and a passing village_site gate,
# largest patch area as tie-break; then eyeball the map pack.
```
