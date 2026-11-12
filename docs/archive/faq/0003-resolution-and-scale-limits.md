# 0003 — What is the resolution, and why not go finer?

**Status: doctrine.**

**Q:** What is the resolution? Can we make it higher and more dynamic
while respecting the science?

**A:** Current presets: research 512² cells at 200 m (102.4 km side);
island8k 512² at 16 m (8.192 km). Every rendered pixel is one cell.
Heights are f64 during simulation, quantized to integer centimetres in
artifacts. Time: 10⁴-year steps × 3000 (research, 30 Myr); 2×10³ × 2500
(island8k, 5 Myr).

**The floor is validity, not performance.** Stream power on drainage area
assumes the cell is wider than the river; κ∇²h hillslopes are a
statistical law that is false at boulder scale. The literature runs LEMs
at 10–100 m cells for this reason. One octave finer (8 m) is defensible;
beyond that the equations are out of their domain and sharper terrain
must come from **M7's amplification layer**, which is gated: amplified
terrain must route water identically to the coarse solution and preserve
per-landform slope statistics, or it fails.

"More dynamic" in the sense of meandering/braiding rivers is a separate
absent process, not a resolution issue — see FAQ 0005.
