# via: the kit plan

Status: approved 2026-11-20, running. Committed: the research (cc99109), K1 and K2 (f529008),
K3 (38b8822), K4 (bf60bb0), K5 (660ec5b); K6 in two commits (aa93d3a, the kit stating what its pieces share; bc2ac73, the city block). K7 so far in two commits (e469406, the wall's foot; 26cb33f, a storey's own outline); its third slice, the roof on an L, is built and follows them with K7's review. Next: K8. K11 (a fantasy map) was added on 2026-11-23. The plan after the first cut (`docs/plan.md`): own the
modular building system, not only where a settlement stands, and review the archetypes and the
SDK against what it shows.

## Context

The owner's direction (2026-11-19): via places settlements but does not own the system that
builds a building from a kit. Kitbashing is a practice, not one algorithm, so it is owned through
several examples, each for the structure it forces: a medieval or fantasy village, a modern city,
traditional East Asian building, industrial sites, classical building, and building that adapts to
terrain.

Three decisions shape the plan:

- **Trial and error, not numbers.** The examples recreate what studios say went wrong, since that
  tells us what an archetype must hold; a studio's numbers would only let us copy one kit
  (`docs/research/Modular kit failures in studios.md`, 27 cases in section 13).
- **Grey kits made by code.** No third-party kit is committed. A kit is a studio's proof kit:
  untextured pieces at their exact sizes.
- **Plain before styled.** `examples/kit` came first (challenge 64): two buildings, eight
  failures, built once with nothing changed and again after lib/building was mended.

## Milestones

The order: mend what corrupts every later example, make a kit checkable, build upward, then the
styles.

| # | Milestone | Done when |
|---|---|---|
| K1 | **The two engine faults.** A piece does not give way to another piece of its own building; a pin removes on its own floor only and can stand on an upper one. | `kit` `stub` removes no piece and reports the clash; `window` removes one piece and stands on its floor; a window can be pinned upstairs. |
| K2 | **The kit as a contract.** One written contract for lib/building and lib/town (`docs/design/kit-contract.md`); a check of a kit against itself; a build that says where a side, a door and a stair would close; doors and stairs placed by a distance along a side. | A plan that does not close is told the sizes and places that would, so none is worked out by hand; `station`'s loft closes without its 46 slip fillers. |
| K3 | **Choosing among pieces.** Variants of one role, a rule that neighbours may not repeat, a stated fallback where a piece does not fit. | `wall_window` is placed by a rule, not a pin; a variant with another footprint is refused. |
| K4 | **Upward.** A facade by storey (base, ground storey, repeating storeys, top), the wall over a stairwell, floor slabs, roofs on rectangles and L-shapes. A roof on any other plan is refused with its reason, or falls back to a stated whole roof. | A three-storey building with another ground storey and a roof; no open wall in `kit`. |
| K5 | **First review.** The four gaps the research showed (the kit as a unit, closure, choosing among pieces, two strengths of claim) each ruled: a law added to an archetype, or a new one. | `docs/ledger/archetypes.md` and the ledger carry the rulings, with K1 to K4 as evidence. |
| K6 | **Modern city block.** Floors by bays, corner buildings, set-backs, variation along a street. The best evidenced of the six. | Each of K6 to K9: one grey kit made by code, a small settlement placed with what we have, four to eight failure cases, measures with their protocol, a sheet, and a table of pattern against archetype. |
| K7 | **Medieval village and terrain.** Pitched roofs over plans that are not rectangles, overhanging storeys, stepped rows, plinths and steps to doors. Mends `market`'s rows, which stand on the ground as it lies. | As K6. |
| K8 | **East Asian and classical.** The roof as the main kit, posts and bays, a walled compound; every size from one module, symmetry, a colonnade. | As K6. Labelled as our own design: the research found no account by a developer of either. |
| K9 | **Industrial.** Pipes and gantries joined by sockets in three dimensions, a glue kit between two kits, an interior kit. | As K6, with a socket at a height. |
| K10 | **Someone else's kit, and the last review.** A freely licensed kit fetched on demand and never committed. Then the archetypes and the SDK again: what a kit's author and a world's author had to write, and what should be shorter. | The kit fits or the report says where it does not; challenge 33 closes or says what is owed. |
| K11 | **A fantasy map.** One world on simulated land that places settlements built from this plan's kits (the village, the East Asian and classical kits, a walled compound), joined by roads and water, with the player map as data. Added by the owner on 2026-11-23, after K10. | One world a person can walk from one settlement to the next; its sheet; its measures with their protocol; and the count of what the map's author had to write against what the build laid. |

## How every milestone runs

1. **Before, then mend.** An example is first built against the tree as it stands and its faults
   recorded, as `examples/kit` was. Then the mend, and the same measures again.
2. **Note, then code, then tests.** A design note in the library's or the chapter's own page
   before code. Every number carries its protocol.
3. **One review by an agent that did not write it,** per milestone, and one fix pass.
4. **A sheet,** a PNG in `docs/reports/`: a report, not a gate.
5. **The record.** `docs/challenges.md`, the ledger's rows, `TODO.md`; a ruling goes in
   `docs/doctrine.md`, made by the agent and open to the owner's challenge afterwards.
6. **If the evidence says the plan is wrong,** the plan is changed here, with the reason, and the
   work goes on. The likeliest place is roofs (K4, K7).

Changed so far:

- **Commits, 2026-11-20.** The plain kit, lib/building's first mends, K1 and K2 were worked in the
  same files on one day, before the first commit window, so they are one commit, with the
  research before it as its own. From K3 on, one commit per milestone.
- **K2, 2026-11-20.** "One reader of a kit" became one written contract and two readers. A library
  step imports only the SDK and the standard library (R19b), so lib/building and lib/town cannot
  share a module; and they answer leftover length differently for a reason the contract states.
  "A check before a build" became a build that says what would close: a build of a plan takes a
  fifth of a second, so what was missing was the answer, not an earlier question.

- **K3, 2026-11-22.** "A rule that neighbours may not repeat" holds inside one wall stretch, not
  across a door or a corner. And a variant renames the bays after it, because a run holds one
  asset: found here, filed under `spec/rules.md` in `TODO.md`, and left for K5 to rule, since it
  is the core's half of "choosing among pieces" and not the library's. The review found that
  "a variant with another footprint is refused" held only for the pitch a pack states, not for
  the model: mended, and `wide_window` is now that case.

- **K4, 2026-11-22.** Three changes, each from working the case before writing code.
  "Roofs on rectangles and L-shapes" is a flat roof on both and a gable on rectangles; a gable
  over an L is refused with its reason and left to K7, which already owns pitched roofs on plans
  that are not rectangles. Worked on the plain kit, the slopes beside a valley must be whole
  pieces from the inside corner, and the walls there are whole pieces from 1.5 m off it (the
  inside corner's reach and the post), so a wing that closes its walls cannot close its roof: the
  roof needs a reach of its own at a valley, which is a rule, not a piece. "A base" (a plinth
  course) moves to K7 with plinths and steps to doors: it only matters where there is a plinth
  to show. And "another ground storey" became storeys of two heights and pieces chosen by the
  storey's height, because with a tag alone one kit could not serve a building with a 4 m
  ground storey and another with a 3 m one. The review then showed the footing and the tiles
  wrong off the axes (walls above up to 1.5 m off the walls below, tiles failing the build):
  mended for the walls and the gable; floor tiles are not laid on a turned building, and say so.

- **K5, 2026-11-23.** The review ruled five things, not four (R66 to R70): the two surfaces at
  one height that K4 met are their own ruling. It added no archetype. It also built the three
  rulings whose mechanism is the core's (a run that names each station's piece, two things one
  step returned, levels by height), because K3 and K4 had each recorded a fault that only the
  core could mend and K6 leans on the first. Two are ruled and owed: the kit stating what its
  pieces share (K6, with the second kit) and bodies that meet by the lattice's rounding.
  The independent review of the engine change found one fault of the change (a warm build
  failed after a floor's height changed in a layered world), which is mended; seven faults older
  than it, of which two are mended and five filed; and nine untested branches, now held by six
  new cases and one more action (`TODO.md`, `spec/graph.md`).
- **K6, first step, 2026-11-23.** R66's mechanism is built before the second kit, not with it:
  the second kit should be written against a contract that already has a place for its facts.
  A taxon's `defaults.shared` in the pack, read by lib/building (`wall`, `stair`, `door` laid
  by; `module`, `storey` checked). It is its own commit, since it changes the pack format.
- **K6, the block, 2026-11-23.** Built: the city kit, one block by hand and as rows, twelve
  changes, the row of buildings in bays, `faces`, the parapet, and one ruling the block forced
  (R71: two paths on two levels do not meet). **Not built: the set-back.** It needs a storey
  whose outline is not its building's, which every rule of `lay.py` takes as given; an
  overhanging storey is the same mechanism, so both are K7's. And the block has no feature of
  its own: four rows state it, and the corners are still the author's sum (filed). The
  independent review found eleven defects; the gravest was R71 as first written, which compared
  two heights at one point and hid real meetings. It now compares ranges of height over the
  ground both paths cover, with half a metre between them.
- **K7, first slice, 2026-11-23.** The village is built on a slope before any roof work, since
  the ground is where the evidence is thinnest and the tool said least: every wall of every
  kit-built house stood over the ground with nothing under it and no word said. Built: the
  base (a kit's `storey:base` pieces) and what `lay` says of a wall's foot. A stepped row
  needed nothing. **Changed:** steps to a door are not built (the doorway already shapes the
  ground to its sill; filed), and `market`'s rows are not mended here: they are lib/town's
  whole-house pieces, another library, and stay filed under `examples/market`.
- **K7, second slice, 2026-11-23.** A storey with its own outline (`setback`, `overhang`): the
  set-back K6 did not build and the jetty are one mechanism, as K6's note said. It is built on
  a rectangle and closes nothing by fitting; what it showed is that a terrace and a jetty do
  not close on either kit, which is said and filed, not mended with new pieces.
- **K7, third slice, 2026-11-23.** The gable roof over an L, which K4 put off: a hip piece at
  the outer corner and a valley piece at the inside one, the valley with a stated reach along
  both inner eaves. A T, a U and wings of two depths are still refused with their reasons.
  K7's one review found the roof sound and six defects elsewhere, two of them in the village's
  own measure (it read the ground inside the neighbour at a party wall, and a door's step where
  the doorway sets the ground): the first slice's numbers are corrected in its README.

- **K11 added, 2026-11-23.** The owner asked whether a fantasy map demo was in the plan. It was
  not: the word stood only in K7's title, and the dogfood map of `docs/plan.md` was proposed
  (`county`) and never chosen. The owner added one, after K10 ("Yes, after K10"). It comes last
  because it places what K7 to K9 build: before them it would be a map of grey boxes from one
  kit. Its size and what stands on it are proposed in its own design note when it starts.

Checks before each commit: the library's tests, `cargo run -p xtask -- check`, and, where a crate
changed, `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test --workspace` and the
conformance runner.

## The owner's part

- **Any push or merge.** Everything stays local until the owner says. Commits are local, one per
  slice, made between 19:00 and 09:00.
- **K10's download.** The kit's licence and size are shown first.
- Nothing else. A sheet is a progress report; the work goes on after it.

## Not in this plan

A two-dimensional facade grammar that cuts geometry, a constraint solver over tiles, and generated
street networks: output stays instances of models (`docs/research/Modular city asset contracts.md`,
"What the framework should not attempt"). If a style cannot be done under that rule, the example
says so and stops there.
