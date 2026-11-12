# Contributing

This file records how work is done in the rewrite and the engineering rules that hold. The goal,
principles and rulings are in [`docs/doctrine.md`](docs/doctrine.md); where this file and the
doctrine disagree, the doctrine wins and this file is fixed.

The earlier rule "Rust, whole hog, not a Rust core with a Python rim" is superseded by R19 and R20:
a Rust core, with steps and libraries in Python through a thin SDK in its own process.

## How work is done

Every milestone in [`docs/plan.md`](docs/plan.md) runs in this order.

1. **Chapter first.** The mechanism is written as a chapter in `spec/`, with a tiny example. A
   mechanism taken from literature or another tool names its source; one of our own says so.
2. **Conformance cases next.** Cases and vectors are written and fail before any code exists.
3. **Code last,** until they pass.
4. **The doctrine decides details.** A question nobody has time to review is answered from
   `docs/doctrine.md`. A new answer is written there as a ruling. Challenge a ruling there; do not
   work round it.
5. **Every request is recorded** as a row in `docs/challenges.md`, whether it fits or not
   (principle 13). Leaving one out is a decision written down with its reason.

## Rules that still hold

- **Same text, same bytes** (R26). On macOS, Linux and Windows, for one engine version, at any
  thread count. Seeds are named and derived, never global. No iteration over an unordered map
  where the order reaches output. No reduction whose order follows thread scheduling. A
  nondeterminism bug is the highest priority.
- **CPU first.** The work is graphs, queues and geometry: irregular and branchy. Parallelism is
  across cores. A GPU may serve optional detail passes and rendering, never a decision in a build.
- **No AI inside the build.** No learned model runs in a build, and no model-made data is an input
  or an output of one. AI is welcome outside: writing documents, analysis, review.
- **Checks before a commit:** `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test --workspace`.
- **Commit style:** `type(scope): summary`.

## New rules for the rewrite

- **No domain noun in core crates** (principle 5). Road, lake, room and their kin live in `lib/`
  as data. A deny-list test enforces it.
- **Library code imports only the SDK** (R19b). Our libraries reach the engine by the same door a
  user's step does. A test enforces it.
- **All maths goes through `via-num`** (R26), the one module that fixes how numbers are computed
  on every system. A test enforces it.
- **No non-finite value in a build** (R26). A NaN or an infinity is an error that names the
  formula, the place and the fix, never a stored value.
- **Never commit private material.** It lives only in the gitignored `spike/local/` folder and is
  never exported into fixtures. Search the tracked files for the private names before every
  commit.
