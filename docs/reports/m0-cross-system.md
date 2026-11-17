# M0 cross-system check: same text, same bytes (R26)

Status: **incomplete**. The macOS side ran. The Linux side did not: Docker Desktop stopped
during the image pull because the host disk filled up (140 MiB free on `/System/Volumes/Data`).
No Linux digest exists yet, so nothing has been compared across systems.

Date: 2026-11-13. Tree: `rewrite` at `a14d1cf`, plus the uncommitted M0 work in the tree at the
time of the run (49 paths in `git status --porcelain`).

## Machines

| Side | Machine | OS | Toolchain |
|---|---|---|---|
| macOS | Apple M4 Pro, arm64 | macOS 26.6 (25G72), Darwin 25.6.0 | rustc 1.92.0 (ded5c06cf 2025-12-08), aarch64-apple-darwin, LLVM 21.1.3; cargo 1.92.0 |
| Linux arm64 | Docker Desktop 29.1.3 VM, native | `rust:1.92`, index digest `sha256:f58923369ba295ae1f60bc49d03f2c955a5c93a0b7d49acfb2b2a65bebaf350d` (pulled for linux/arm64) | **not run**: Docker stopped before the first container started |
| Linux amd64 | same VM, emulated | `rust:1.92` for linux/amd64 | **not run**: the pull failed with `failed to commit snapshot ... commit failed: input/output error` (disk full) |

## Null digest (`build.md` 14)

```
cargo run -p via-num --example null_digest --release
```

| Side | Digest | Lines hashed |
|---|---|---|
| macOS arm64 | `e313bd6c90264f5f3ff3a03acd1af9e821c31967583fc2baf7582a16544e5c43` | 113 |
| Recorded in `spec/vectors/num/null_digest.txt` | `e313bd6c90264f5f3ff3a03acd1af9e821c31967583fc2baf7582a16544e5c43` | |
| Linux arm64 | not run | |
| Linux amd64 | not run | |

The macOS digest matches the recorded one.

## Workspace tests

```
cargo test --workspace --no-fail-fast
```

| Side | Result |
|---|---|
| macOS arm64 | 111 passed, 1 failed, 6 ignored, 37 test binaries; exit 101 |
| Linux | not run |

The one failure is `xtask` `tests/workspace.rs` `limitations_name_existing_cases`:
`docs/design/limitations.md:197: W3` and `:211: E3` say "fixed in M1" but name no
conformance case. It is a documentation check and has nothing to do with platforms.

## Protocol probe (`protocol.md` case 0)

macOS only, with `sdk/python/.venv` (Python 3.12.12, numpy 2.5.4):

```
cargo test -p via-proto --release -- --include-ignored
cargo run -p via-proto --release --example probe -- --python $PWD/sdk/python/.venv/bin/python --runs 3
```

- Tests: 15 unit tests passed; all 6 probe tests passed, including the ignored ones
  (field out and back with a call back, killed mid-call, deadline, cancel, group kill, framing).
- Example, median of 3 runs: start + initialize 134.1 ms; 6.5 MB field out and back 19.2 ms;
  empty batch call 25.7 us (p99 49.5); build.field + numpy map 184.9 us; bare numpy map 143.0 us;
  engine map 39.4 us. Killed mid-call: `protocol.crashed`, both pids gone, 0 call folders left.
- Finding: when `--python` is given as a relative path (`sdk/python/.venv/bin/python`), the probe
  fails with `protocol.start` ("could not be started"). An absolute path works. The relative path
  is probably resolved against the worker's working folder, not the caller's.

Linux: not run.

## Why the Linux side did not run, and what has to happen first

1. `docker pull` hung without printing anything for about 20 minutes. The cause was the macOS
   credential helper (`docker-credential-desktop`, which reads the keychain). Running with
   `DOCKER_CONFIG` set to an empty folder, so no credential helper is used, fixed it.
2. The linux/arm64 `rust:1.92` image pulled. The linux/amd64 pull then failed with an I/O error
   while committing a layer, and Docker Desktop stopped ("Docker Desktop is unable to start").
   At that point the host had 140 MiB free: 886 GiB of 926 GiB used, and `Docker.raw` held about
   36 GiB.
3. To resume: free disk space on the host, start Docker Desktop again, then run the commands
   below (arm64 first; amd64 under emulation only if there is room for a second ~1.5 GB image).

```
docker run --rm --platform linux/arm64 -v "$PWD":/src:ro rust:1.92 bash -c '
  cp -a /src /w && cd /w && rm -rf target sdk/python/.venv &&
  export CARGO_TARGET_DIR=/tmp/target &&
  cargo run -p via-num --example null_digest --release &&
  cargo test --workspace --no-fail-fast;
  curl -LsSf https://astral.sh/uv/install.sh | sh && . $HOME/.local/bin/env &&
  uv venv sdk/python/.venv --python 3.12 && uv pip install --python sdk/python/.venv/bin/python numpy &&
  cargo test -p via-proto --release -- --include-ignored'
```

Use `--rm` so no container is left behind. If the digests differ, compare the
`null_digest --release -- --lines` output from each side to find the first vector that differs.
