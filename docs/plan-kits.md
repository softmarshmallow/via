# via: the kit plan

Status: approved 2026-11-20, running. The plan after the first cut (`docs/plan.md`): own the
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
