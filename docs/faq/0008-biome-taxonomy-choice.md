# 0008 — Is Whittaker the right biome taxonomy?

**Status: doctrine; known gaps ledgered.**

**Q:** Is Whittaker the standard model, or is there an argument for
something else?

**A:** Whittaker (1975) is the standard *teaching* taxonomy and — the
deciding fact — the only recognized scheme expressible with the data our
climate produces (mean-annual temperature × precipitation). The
alternatives need inputs we don't compute yet: Köppen–Geiger requires
seasonality; Holdridge requires potential evapotranspiration. Both are
ledgered as unlocks (parametric seasons; PET), not rejected.

Known honesty gaps, recorded rather than hidden:

- **Savanna–forest bistability**: at intermediate tropical rainfall,
  fire disturbance decides savanna vs forest (Staver et al. 2011; Bond
  et al. 2005). A pure climate envelope cannot capture it; **M8's
  disturbance model** closes this.
- Classes with azonal causes (wetland, bare rock, lake) are measured
  from terrain state (water depth, TWI, slope), never from the T×P
  diagram — thresholds declared in EcologyConfig (ADR 0003/0004).
- The vegetation *attribute* fields (density, canopy) are labeled
  **heuristic** — our invention, unlike the classification. Consumers
  wanting only defensible outputs stop at the spectra plus the classes.

Per ADR 0003: spectra over taxonomy — discretization is allowed only
where academia recognizes the scheme, and the scheme's thresholds are
never tuned.
