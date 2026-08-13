# FAQ — design Q&A and the limitation ledger

A record of recurring design questions asked of this project (usually by a
non-geomorphologist, answered from the literature and the code) and the
honest limitations they surfaced. Kept because:

1. The answers encode *why* the engine is shaped the way it is, in plainer
   language than the ADRs.
2. The **limitations are a work-queue**: entries tagged `open` are the raw
   material for later grouping, study, and solving — including with AI
   tooling *outside* the engine (see CONTRIBUTING: no AI inside it).

Rules: each entry states the question, the short answer, and a status.
Update statuses when a milestone changes them; never delete an entry —
supersede it.

Status vocabulary:
- **doctrine** — settled understanding; unlikely to change.
- **resolved (ADR/Mx)** — was a limitation; fixed, pointer to where.
- **open (ledger)** — known limitation with a recorded upgrade path.
- **open (unplanned)** — known limitation, no path chosen yet.

## Index

- [0001 — What model is this based on? Is there a modern one?](0001-what-model-is-this.md)
- [0002 — Why do the maps look synthetic?](0002-why-do-the-maps-look-synthetic.md)
- [0003 — What is the resolution, and why not go finer?](0003-resolution-and-scale-limits.md)
- [0004 — How do lakes form? Do they last?](0004-lakes-how-they-form-and-die.md)
- [0005 — Will enough seeds produce oxbows, braids, every river type?](0005-missing-landforms-and-why.md)
- [0006 — How does this differ from Houdini-style terrain tools?](0006-authored-vs-emergent.md)
- [0007 — What actually decides the climate?](0007-climate-is-forcing-not-simulation.md)
- [0008 — Is Whittaker the right biome taxonomy?](0008-biome-taxonomy-choice.md)
