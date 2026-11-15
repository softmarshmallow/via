"""The command line of the runner (conformance.md 6).

    run --via PATH --milestone N [--chapter C] [--case ID] [--jobs N] [--record DIR]
    compare A B
    list [--milestone N]

`list` parses every case and prints how many there are per milestone, with any case error: the
first workspace check of conformance.md 7 ("every case.toml parses"), runnable without a binary.
`--repo`, `--work-root` and `--timeout` are test seams and operational knobs beyond the chapter's
command; their defaults are the repository holding this tool, the system's temporary folder and 900
seconds.
"""

from __future__ import annotations

import argparse
import os
import sys
import tempfile
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

from . import diags, model, record
from .runner import Outcome, Settings, run_case, system_name

REPO = Path(__file__).resolve().parents[3]


def _default_via(repo: Path) -> Path:
    """`$CARGO_TARGET_DIR/debug/via` when that is set (relative to the repository), else
    `target/debug/via`: the binary `cargo build -p via-cli` just wrote, not an older one."""
    exe = "via.exe" if sys.platform == "win32" else "via"
    target = os.environ.get("CARGO_TARGET_DIR")
    base = (repo / target) if target else (repo / "target")
    return base / "debug" / exe


def _stale_against(via: Path, repo: Path) -> Path | None:
    """The newest source under `crates/` (`.rs`, `Cargo.toml`) or the root `Cargo.toml`/`Cargo.lock`
    saved after `via` was built, or None. A default binary older than the code gives false failures
    and false passes alike, so the runner refuses it; an explicit `--via` is taken as given."""
    built = via.stat().st_mtime
    roots = [repo / "Cargo.toml", repo / "Cargo.lock"]
    crates = repo / "crates"
    if crates.is_dir():
        roots += [p for p in crates.rglob("*") if p.suffix == ".rs" or p.name == "Cargo.toml"]
    newest = max((p for p in roots if p.is_file()), key=lambda p: p.stat().st_mtime, default=None)
    return newest if newest is not None and newest.stat().st_mtime > built else None


def _selected(found: list[model.Discovered], chapter: str | None, case: str | None) -> list[model.Discovered]:
    return [
        d for d in found
        if (chapter is None or d.chapter == chapter) and (case is None or d.folder.name == case)
    ]


def cmd_run(ns: argparse.Namespace) -> int:
    repo = Path(ns.repo).resolve()
    via = Path(ns.via).resolve() if ns.via else _default_via(repo)
    if not via.is_file():
        print(f"error: the via binary {via} does not exist; build it with `cargo build -p via-cli`", file=sys.stderr)
        return 2
    if not ns.via:
        newer = _stale_against(via, repo)
        if newer is not None:
            print(f"error: the default via binary {via} is older than {newer}; rebuild it with "
                  "`cargo build -p via-cli` (same CARGO_TARGET_DIR) or pass --via", file=sys.stderr)
            return 2
    work_root = Path(ns.work_root).resolve() if ns.work_root else Path(tempfile.gettempdir())
    work_root.mkdir(parents=True, exist_ok=True)
    settings = Settings(via=via, repo=repo, milestone=ns.milestone, work_root=work_root, timeout=ns.timeout)
    registry = diags.Registry(repo / "spec" / "diagnostics.toml")
    found = _selected(model.discover(repo / "spec" / "cases"), ns.chapter, ns.case)
    if not found:
        print("error: no case selected", file=sys.stderr)
        return 2

    def one(d: model.Discovered) -> Outcome:
        if d.case is None:
            return Outcome(f"{d.chapter}/{d.folder.name}", "error", [d.error or "malformed"])
        return run_case(d.case, settings, registry)

    if ns.jobs > 1:
        with ThreadPoolExecutor(max_workers=ns.jobs) as pool:
            outcomes = list(pool.map(one, found))
    else:
        outcomes = []
        for d in found:
            outcomes.append(one(d))

    counts = {"pass": 0, "fail": 0, "pending": 0, "error": 0}
    for o in outcomes:
        counts[o.status] += 1
        head = f"{o.status:<8}{o.case}"
        if o.status == "pending":
            head += f"  ({o.detail[0]})"
            print(head)
            continue
        print(head)
        for line in o.detail:
            print(f"        {line}")
    if registry.missing:
        print(f"note: {registry.path.relative_to(repo)} does not exist; every core code fails U2")
    print(
        f"{counts['pass']} passed, {counts['fail']} failed, {counts['pending']} pending, "
        f"{counts['error']} in error (milestone {ns.milestone})"
    )
    if ns.record:
        cases = {o.case: o.record for o in outcomes if o.record is not None}
        path = record.write(Path(ns.record), system_name(), ns.milestone, cases)
        print(f"record: {path}")
    return 0 if counts["fail"] == 0 and counts["error"] == 0 else 1


def cmd_compare(ns: argparse.Namespace) -> int:
    try:
        diffs, notes = record.compare(Path(ns.a), Path(ns.b))
    except (OSError, ValueError) as e:
        print(f"error: {e}", file=sys.stderr)
        return 2
    for n in notes:
        print(f"note    {n}")
    for d in diffs:
        print(f"differ  {d}")
    print(f"{len(diffs)} differences")
    return 1 if diffs else 0


def cmd_list(ns: argparse.Namespace) -> int:
    repo = Path(ns.repo).resolve()
    found = model.discover(repo / "spec" / "cases")
    per: dict[int, int] = {}
    errors = 0
    for d in found:
        if d.case is None:
            errors += 1
            print(f"error   {d.chapter}/{d.folder.name}: {d.error}")
            continue
        per[d.case.milestone] = per.get(d.case.milestone, 0) + 1
        tag = f"M{d.case.milestone}" + (f", pending: {d.case.pending}" if d.case.pending else "")
        print(f"case    {d.case.name}  ({tag})")
    for m in sorted(per):
        print(f"M{m}: {per[m]} cases")
    print(f"{len(found)} cases, {errors} in error")
    return 1 if errors else 0


def main(argv: list[str] | None = None) -> int:
    p = argparse.ArgumentParser(prog="via-conformance", description="The conformance runner of spec/conformance.md.")
    sub = p.add_subparsers(dest="cmd", required=True)
    r = sub.add_parser("run", help="run the cases against a via binary")
    r.add_argument("--via", help="the via binary (default: $CARGO_TARGET_DIR/debug/via or target/debug/via, refused when older than crates/)")
    r.add_argument("--milestone", type=int, required=True, help="the milestone being worked on; cases above it are pending")
    r.add_argument("--chapter")
    r.add_argument("--case")
    r.add_argument("--jobs", type=int, default=1)
    r.add_argument("--record", help="write <os>-<arch>.json into this folder (U7)")
    r.add_argument("--repo", default=str(REPO), help=argparse.SUPPRESS)
    r.add_argument("--work-root", help=argparse.SUPPRESS)
    r.add_argument("--timeout", type=float, default=900.0, help=argparse.SUPPRESS)
    r.set_defaults(fn=cmd_run)
    c = sub.add_parser("compare", help="compare two run records (U7)")
    c.add_argument("a")
    c.add_argument("b")
    c.set_defaults(fn=cmd_compare)
    li = sub.add_parser("list", help="parse every case and count them per milestone")
    li.add_argument("--repo", default=str(REPO), help=argparse.SUPPRESS)
    li.set_defaults(fn=cmd_list)
    ns = p.parse_args(argv)
    if ns.cmd == "run" and not 0 <= ns.milestone <= 7:
        p.error("--milestone takes 0 to 7")
    return ns.fn(ns)


if __name__ == "__main__":
    sys.exit(main())
