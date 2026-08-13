# 0002 — Why do the maps look synthetic?

**Status: largely resolved (ADR 0005); residuals open (ledger).**

**Q:** The rendered maps look unnatural. Is that expected, or actually
wrong?

**A:** Both — and the split matters. Three signatures were identified by
inspection (M3-era maps), each with a different verdict:

1. **D8 grid bias** (artifact): channels locked to 0°/45°/90°, parallel
   fall-line fans on smooth flanks, dead-straight valley courses.
   → **Resolved by M3.5 hybrid MFD routing** (ADR 0005): valleys curve,
   fans converged to dendritic channels. A residual fine-scale grid
   flavor remains — channels are still per-cell steepest between
   confluences. Full cure: irregular (Voronoi) mesh — **open (ledger,
   M7 era)**.
2. **Single-thread lacustrine delta** (artifact): comb-like deposition
   bands. → Largely resolved by MFD spreading on flooded cells.
3. **Texture monotony** (missing process, not a bug): no coastal
   processes, purely diffusive hillslopes — and, until M4 landed, one
   rock type everywhere. Lithology shipped (ADR 0007): declared strata
   put ridge-and-valley corrugation, escarpments, and knickzones back
   (see experiments/strata512), though the *default* column stays
   homogeneous. → **Remaining: M5 (soil), M6 (coasts).**

Things that look odd but are **correct physics**: the regular ridge–valley
spacing (wavelength selection from the K/κ competition — Perron et al.
2009; the Gabilan Mesa really looks like this) and one-sided dissection
asymmetry (the rain shadow working as designed).

Also worth remembering: rivers that never meander are *not* on this list —
that is an absent process, not an artifact. See FAQ 0005.
