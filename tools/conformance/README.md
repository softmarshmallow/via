# Conformance runner

The runner of `spec/conformance.md`. It reads every case under `spec/cases/<chapter>/<case>/` and
drives the `via` binary through its command line only (R20). It uses Python's standard library and
numpy, nothing else, and never the SDK.

```
cd tools/conformance
uv run python -m via_conformance run --milestone 0                 # target/debug/via
uv run python -m via_conformance run --via PATH --milestone 1 --chapter build --case c21_failed_build
uv run python -m via_conformance run --milestone 1 --record runs/  # writes <os>-<arch>.json (U7)
uv run python -m via_conformance compare runs/macos-aarch64.json runs/linux-x86_64.json
uv run python -m via_conformance list                              # parse every case, count per milestone
uv run python -m via_conformance.fixtures --check                  # examples/ford matches the documents
uv run python -m unittest discover -s tests                        # the runner's self-tests
```

- **Outcomes** are `pass`, `fail`, `pending` and `error` (`conformance.md` 6). A failed case's
  working folder is kept and its path printed.
- **Checks added to every case** (U1 to U7, `conformance.md` 5) need no line in a `case.toml`.
  U2 reads `spec/diagnostics.toml`; while it does not exist, every core code fails U2.
- **The ford world** is `examples/ford/`, which every ford case copies into its working folder
  (`[copy.ford] from = "examples/ford"`, `to = "."`). `via_conformance/fixtures.py` checks that
  folder byte for byte against `docs/plan.md` and `spec/assets.md` 6.2 (`--check`); edit the
  documents, then run `python -m via_conformance.fixtures` to rewrite it.
- **Self-tests** drive `tests/fake_via.py`, a fake binary that is honest by default and breaks one
  rule on request (`FAKE_VIA=threads`, `incremental`, `abspath`, ...), so each check is seen both
  to pass and to fail. A `--via` ending in `.py` runs under the runner's own Python, so the
  fake binary needs no `#!` line and the self-tests run on Windows too.
- **The raw worker** is `raw_worker.py` (`conformance.md` 3.7): a step program that speaks
  `via-step-1` without the SDK and breaks one rule on request (`when_unserved`,
  `message_too_large`, `blob_digest`, `blob_header`, `slow_exit`). Cases copy it in as
  `steps/raw.py`, with `via_conformance/blake3.py` beside it when the fault names a blob.
- **BLAKE3** is `via_conformance/blake3.py`, a pure-Python copy of the reference algorithm, for the
  `id_rule` check, `rerecord` and the raw worker; `python -m via_conformance.blake3 FILE` prints a
  digest as `b3sum` does.
