# 0005 — Will enough seeds produce oxbows, braids, every river type?

**Status: open by design — the roadmap is the plan for closing it.**

**Q:** If we run the simulation many times, will we encounter the various
water-flow types nature holds — oxbow lakes, for instance?

**A:** No — and the reason is the project's central epistemic point:
**seeds sample geometry; mechanisms do not emerge from sampling.** An
oxbow is not a rare configuration of our equations; it is the product of
a process (lateral meander migration: bank erosion, point-bar growth,
neck cutoff) that the equations do not contain. Its probability across
seeds is exactly zero.

Nature's taxonomy is the **union of process domains** — fluvial,
hillslope, glacial, coastal, aeolian, karst… A model containing N
processes emergently produces the taxonomy of those N domains with the
right statistics (that is what the gates verify), and cannot produce the
rest. "Following the science" means growing the union — which is what
the milestone layers are — never sampling harder inside one domain, and
never painting landforms whose process we lack.

What seed-sampling already varies: network geometry, captures and
abandoned valleys (wind gaps — the closest cousin to an oxbow we have),
lakes and their lifespans, deltas, rain-shadow asymmetries.

What cannot appear without new process:

| Landform | Missing mechanism | Unlock |
|---|---|---|
| Meanders, oxbows, scroll bars, levees | lateral bank erosion + point-bar deposition | **river-corridor meander stage** (ledger): kinematic centerline migration, Howard & Knutson 1984 lineage; runs downstream of terrain on extracted corridors — justified by timescale separation (meanders: 10²–10⁴ yr; terrain: 10⁶⁺). Oxbows then *emerge* from cutoffs into the water-depth spectrum |
| Braided rivers | bedload sorting + width dynamics | same era; may remain content |
| Terminal lakes, salt flats | evaporative lake water budget | ledger (core routing upgrade) |
| Persistent waterfalls | rock-strength contrast | **landed (M4, ADR 0007)** — declared lithology forcing; knickzones hold where rivers cross resistant contacts. The default column is homogeneous, so they appear only when a config declares strata |
| Springs, underground rivers | karst hydrology | M4 shipped karst *potential* (solubility × discharge, ADR 0007); conduits stay content |
| Estuaries, tidal channels | waves/tides | **M6** |
| Glacial valleys, cirques, fjords | ice flow | unplanned; the Landlab ecosystem shows it is addable |
