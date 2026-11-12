# 0001 — What model is this based on? Is there a modern one?

**Status: doctrine.**

**Q:** What study/model is the terrain engine based on — and is there a more
modern one we should be using?

**A:** It is a **landscape evolution model (LEM)** of the stream-power
family, assembled from cited components:

| Component | Source |
|---|---|
| Stream-power incision ∂h/∂t = U − K·Q^½·S | Howard & Kerby 1983; Whipple & Tucker 1999 |
| Hillslope diffusion κ∇²h | Culling 1960 |
| Erosion–deposition (G-coefficient) | Davy & Lague 2009; Yuan et al. 2019 |
| O(n) implicit solver | Braun & Willett 2013 |
| Depression handling (priority-flood+ε) | Barnes et al. 2014 |
| MFD routing + channel convergence | Freeman 1991; Holmgren 1994 (ADR 0005) |
| Orographic climate as forcing | standard regional practice; cf. Roe 2005 |
| Biome classes | Whittaker 1975 |

**This lineage *is* the modern one.** It is the current workhorse of
academic geomorphology — FastScape, Landlab (Hobley et al. 2017), Badlands
and CHILD are all built on these equations, and the newest piece we use
(Yuan et al.) is from 2019. No newer paradigm replaces the law at these
scales; the field's frontier is **breadth of process components on the
same substrate** (glaciers, vegetation coupling, flexure, storms), which
is exactly the shape of this project's roadmap (docs/ROADMAP.md). Landlab
in particular mirrors our architecture: composable process components over
a shared grid.
