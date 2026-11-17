# The revisit after the first cut

One page for the owner: what was reorganised on 2026-11-18, how it was checked, and the verdict
on each thing found. Nothing here changes what a world builds. No engine crate, chapter's
meaning, case, library or example was changed: the work is in `docs/`, `TODO.md`, `README.md`,
`CONTRIBUTING.md`, a comment in `Cargo.toml`, two new checks under `tools/`, and `crates/xtask`,
which now runs them.

## What changed

| What | Before | Now |
|---|---|---|
| The ledger | One file of 654 rows whose `Now` column described the spike, and a second ledger of 305 rows kept as a draft outside git | [`docs/ledger/`](../ledger/README.md): 865 rows, each under one of twenty archetypes that have contracts, each `Now` pointing at what shows it |
| The layout | A folder list in the README; the order of the crates a comment in `Cargo.toml` | [`docs/layers.md`](../layers.md): five layers of meaning, six rings of code, a row for every crate, chapter and library, and the engine's form of each archetype |
| Our own words | A table of words the spike coined | Section 25 of the ledger: the eleven nouns, 50 kinds and 17 steps, each with its archetypes and what it is made of |
| `TODO.md` | A log by milestone and slice: 2,502 lines, 159 of its items ticked | 374 open items filed by layer in 2,112 lines; the lessons and decisions of each slice kept |
| Pages that spoke from before the cut-over | The README said a rewrite was under way; the plan read as current; the spike's tech sheet was `docs/design/concepts.md` | The README's status and layout rewritten; the plan marked complete; the sheet renamed `docs/design/spike-tech-sheet.md` |
| Checks on the record | None | `tools/ledger_check.py` and `tools/layers_check.py`, run by `xtask check` and by `cargo test` |

## How it was checked

- **The filing.** A second reader filed 173 rows, a fifth of each section drawn with a fixed
  seed, from `docs/ledger/archetypes.md` alone and without seeing the first filing. 145 were
  filed the same (84%). Cohen's kappa over the archetype is 0.83, where chance alone would agree
  on 7%. Where the second reader called itself confident, 68 of 74 were the same. Of the 28
  that differed, 8 followed the second reader and 20 stood. The twelve boundaries it found
  unclear are now written into the contracts, and 8 more rows were refiled by them. One reader,
  one pass.
- **The claims.** Every `shown` and `partly`, 312 in all, was read against the repository by
  four readers that did not write them. 256 stood (82%). 13 said too much and 15 too little, 27
  pointed at the wrong thing, and 1 had nothing behind it. All 56 were corrected, and every
  pointer still resolves. One reader to a claim: a claim that passed was not read twice.
- **The sources.** 65 citations and addresses were checked against their publishers: 60 stood
  as written and 5 were corrected (a paper's title, a talk's title, two addresses that had
  moved, and the names a paper gives its three rules).
- **The repository's checks.** `cargo fmt --check`, `cargo clippy --workspace --all-targets`,
  `cargo test --workspace` and `cargo run -p xtask -- check` pass on the result: its eleven
  checks, the two new ones among them, with no finding. The conformance runner was not run: no
  engine code, case, library or example changed.
- **The to-do list.** Counted before and after: 284 open items and 83 notes without a box came
  in; four notes were already marked done; 363 came out, each under a layer, and every one's
  words were found in the new file. The count also showed four open items that sat under one
  done item; they were put back.

## Findings and verdicts

What the layout shows is counted in [`docs/layers.md`](../layers.md), "What the layout shows". A
verdict is one of: done, or left, with the reason. What is left and needs work is an item of
`TODO.md`.

| Finding | Verdict | Reason, or where it went |
|---|---|---|
| The ledger's `Now` column recorded where the spike stood, a week after the spike was deleted | Done | Every claim was read again and names what shows it; a script checks the name exists |
| Two ledgers disagreed on names and shape | Done | One ledger, one row shape, 114 rows of the second folded into the first |
| Archetypes were one-line tags, two of them catch-alls | Done | Twenty, each with a definition, laws, a source and a boundary; Derivation split by how far a change reaches |
| Nothing tied a kind, a chapter or an example to an archetype | Done | The table of forms in `docs/layers.md`, and section 25 of the ledger |
| The order of the crates was a comment | Done | Each crate has a ring; `tools/layers_check.py` reads every `Cargo.toml`, and the comment now points there |
| Libraries stated no order among themselves | Done | Listed from the ground up; one may name only the kinds of those above it, and the six references that exist comply |
| The README said the rewrite was under way and `main` held an older tree | Done | Status rewritten |
| `docs/plan.md` read as the current plan | Done | Marked complete, kept as the record |
| The spike's model sat in `docs/design/concepts.md`, under the name of a layer | Done | Renamed `spike-tech-sheet.md`; its one citation follows |
| `TODO.md` was a log by slice under the heading "Spikes", most of it done | Done | Re-filed by layer; done items removed; the list before is `git show ed27503:TODO.md` |
| `lib/land` beside `lib/terrain`; a lake in both `lib/water` and `lib/terrain` | Left | A library's name is written in every world and cited by chapters and cases: a rename changes the format. `TODO.md`, "Every library" |
| Seven kinds no example writes | Left | Needs a world, or a decision to retire. `TODO.md`, "Every library" |
| Six keys of the format that nothing but a case writes | Left | `TODO.md`, `spec/vocabulary.md` |
| Seven steps do in code what the engine has a form for: 3,788 lines, against 1,707 lines of kinds in all | Left | A question about what a kind may say, and the owner's. `TODO.md`, "The owner's to decide" |
| Grammar and Guidance have no form in the engine | Left | The same question |
| `docs/reference/` beside `docs/references.md` | Left | Each is the right word: generated reference pages (R33, and the path is in code), and a list of references |
| `docs/research/` holds file names with spaces | Left | Both are cited from `spec/`; rename with those citations when next edited |
| `crates/via-terrain`, a library's program, sits among the engine's crates | Left | It is a workspace member; its ring is stated, and the check refuses it the model |
| `lib/packs` holds assets beside the kind libraries | Left | `spec/assets.md` 2 fixes where a pack is found |
| A material's `file` and `repeat` are not written to the build, though `spec/build.md` 7 says they are; found by a reader of the claims, and seen in the code, not by a build | Left | `TODO.md`, `spec/build.md` |
| `[lock.<id>]` is in the vocabulary and still refused; the ledger had called it shown | Done | The row now reads partly, and names pins and clears only |
| Nothing checked the record | Done | `cargo run -p xtask -- check` and `cargo test --workspace` run both new checks, so CI does when it runs |

## Not read

`docs/faq.md`, `docs/challenges.md`, the 23 rulings open to the owner and the three design notes
dated before the first cut were not re-read. The meaning of no spec chapter was changed: one
citation in `spec/vocabulary.md` follows the ledger's new path.
