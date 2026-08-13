# ADR 0001 — Crate layout, artifact format, and terrain-stage representation

Status: accepted (2026-08-13)
Scope: the two engineering decisions CONTRIBUTING lists as open — artifact format and crate layout — plus the representation choices the terrain stage forces. Epoch model and export target remain open.

## Context

The terrain stage is the first stage built, and it cannot exist without (a) a place to live in the workspace and (b) a format for what it emits. Per CONTRIBUTING, stages communicate only through typed artifacts, determinism is a hard requirement, and coordinates are fixed-point.

## Decision 1 — Crate layout

One Cargo workspace, crates under `crates/`:

| Crate | Role |
|---|---|
| `via-artifact` | Typed artifact store: rasters, run manifest, stats records, content hashing. No simulation logic. |
| `via-terrain` | Stage 1: uplift/climate inputs → heightfield, drainage, rivers, basins + gates. |
| `via-viz` | Debug/QA renderers: reads artifacts, writes PNGs. Never a dependency of a stage. |
| `via` | CLI runner: wires config → stage → artifacts → renders. |

Stages get one crate each as they arrive (`via-suitability`, `via-corridors`, …). Shared *types* live in `via-artifact`; shared *algorithms* do not exist yet and will not be pre-abstracted.

## Decision 2 — Artifact format

A run is a directory:

```
runs/<name>/
  manifest.json      # config, seed, crate versions, artifact list with blake3 hashes
  *.vrast            # binary rasters (see below)
  gates.json         # the stage's statistical gates + pass/fail against target ranges
  render/*.png       # QA map pack (via-viz output; not a stage artifact)
```

`.vrast` layout (little-endian): magic `VRAS`, format version u32, dtype tag u32
(i32 | u32 | u64 | f32), width u32, height u32, cell size u32 (centimetres),
then row-major payload. Nothing else. JSON carries everything human-readable;
binary carries everything bulky. Content hashes (blake3) in the manifest are
the determinism witness: same seed ⇒ identical hashes on any core count.

Rejected alternatives: GeoTIFF (pulls in a heavy dependency or C bindings for
what is, internally, a memcpy; `via-viz` may grow a GeoTIFF *export* for GIS
inspection later), and a database (artifacts are immutable and stage-scoped;
files + hashes are the simplest thing that supports caching and resuming).

## Decision 3 — Terrain representation (M1)

- **Regular grid**, default 512×512 at 200 m cells (~102 km × 102 km). D8 tie-breaking by lowest cell index is trivially deterministic on a grid; fixed-point cell centres are exact. An irregular (Delaunay) mesh à la Cordonnier 2016 is the recorded upgrade path if axis-aligned drainage artifacts become objectionable.
- **Heights are i32 centimetres in artifacts.** Inside the solver kernel, heights are f64 with a fixed, single-threaded operation order for every accumulation that reaches output; quantization to centimetres happens once, at the artifact boundary. This is a deliberate reading of the fixed-point rule: an implicit PDE solve in integer centimetres would buy no additional determinism (the f64 kernel is already bitwise deterministic) at real cost in complexity. If this reading is wrong, the artifact format does not change — only the kernel does.
- **Drainage area is stored as u64 upslope cell counts** (exact integers), converted to m² only in derived statistics.

## Decision 4 — Algorithms (M1) and upgrade paths

- Stream-power incision ∂h/∂t = U − K·Aᵐ·Sⁿ with n = 1, m = 0.5, solved with the Braun & Willett (2013) O(n) implicit scheme on the D8 receiver tree; explicit hillslope diffusion; run to quasi-steady state.
- Depressions: priority-flood + ε (Barnes et al. 2014) applied per iteration, heap ordered by (elevation, cell index) for determinism. Consequence: M1 terrain has no closed depressions and therefore no lakes; lakes become explicit objects when depression *routing* (Cordonnier, Bovy & Braun 2019) replaces filling in M2.
- Parallelism: `rayon` only for per-cell maps (receivers, diffusion stencil, noise, rendering). Everything order-sensitive (flood, stack, accumulation, implicit solve) is sequential O(n) or O(n log n) and cheap.

## QA contract

Every terrain run emits `gates.json`. Hard gates (all must pass for
`overall_pass`; a gate that cannot be evaluated for lack of data is a fail):

- **Slope–area concavity θ** ∈ [0.40, 0.60], with fit R² reported. The
  regression runs over fluvial cells (A ≥ 0.5 km²) *restricted to a ±25%
  band around the median uplift rate* — the θ ≈ m/n prediction assumes
  uniform uplift — and excludes ε-fill flats (S < 10⁻⁵), as on real DEMs.
- **Hack exponent h** ∈ [0.45, 0.70], fit over **all river subbasins**
  (every river cell), not outlet basins only: outlet-only fits on an island
  are dominated by radius-truncated coastal basins and read low.
- **Horton bifurcation ratio Rb** ∈ [3, 5], measured **within the largest
  basin** (Horton's laws are per-basin; pooling the landmass lets order-1
  coastal microbasins inflate N₁). Length ratio RL is reported unbounded.
- **Pit cells = 0** and **drainage completeness = 1.0** — invariants of the
  fill/route path, kept as regression tripwires for ocean masking and basin
  labeling.

Advisory (reported, outside `overall_pass`): hypsometric integral
(0.35–0.60), and the SPL steady-state residual median |K·√A·S/U − 1|
(≤ 0.35) — persistently ~0.45 in practice because drainage-capture churn
keeps trunk rivers above instantaneous equilibrium; this is the documented
behaviour of statistical (not exact) steady state.

`gates.json` also carries blake3 hashes of every artifact raster. CI runs a
small grid at 1 and 4 threads and fails on any bit of divergence.
