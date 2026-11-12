"""The `ford` world as case inputs, derived from the documents that fix it byte for byte.

`docs/plan.md` ("Milestone one: ford") gives `examples/ford/world.toml` and its change files
exactly, and `spec/assets.md` 6.2 gives `packs/primitive_large.toml` as 6.1's text with one line
changed. Cases cite their lines and bytes, so a copy typed by hand could drift; this module derives
every file from those texts, writes it into each ford case folder, and checks that the folders
still hold exactly that (`python -m via_conformance.fixtures --check`, and the self-tests).

Until `examples/ford/` exists, each ford case carries the world in its own folder (conformance.md
2: "anything else" in a case folder is an input copied into the working folder).
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[3]

# The cases whose folders hold the ford world (chapter/case).
FORD_CASES = [
    "assets/pack_swap",
    "build/c21_failed_build",
    "build/c37_change_report",
    "diagnostics/ford_four_bad_edits",
    "fields/change_windows",
    "graph/null",
    "graph/vertex",
    "identity/c31_spacing_hand_edits",
]

LARGE_HOUSE_FROM = "size      = [8, 6, 4]"
LARGE_HOUSE_TO = "size      = [10, 8, 4]"


class FixtureError(Exception):
    pass


def _toml_block_after(text: str, marker: str) -> str:
    i = text.find(marker)
    if i < 0:
        raise FixtureError(f"marker not found: {marker!r}")
    m = re.compile(r"```toml\n(.*?)```", re.S).search(text, i)
    if not m:
        raise FixtureError(f"no toml block after {marker!r}")
    return m.group(1)


def _table_body(plan: str, name: str) -> str:
    """A change file whose body the plan's table gives as backticked lines, each a line of the
    file, up to the first `:` or `,` outside backticks (the rest of the cell is description)."""
    row = re.search(rf"^\| `{re.escape(name)}` \| (.*?) \|$", plan, re.M)
    if not row:
        raise FixtureError(f"no row for {name} in the change-file table")
    cell = row.group(1)
    spans: list[str] = []
    k = 0
    while k < len(cell):
        ch = cell[k]
        if ch == "`":
            end = cell.index("`", k + 1)
            spans.append(cell[k + 1 : end])
            k = end + 1
            continue
        if ch in ":,":
            break
        k += 1
    if not spans:
        raise FixtureError(f"no body in the row for {name}")
    return "".join(s + "\n" for s in spans)


def ford_files(repo: Path = REPO) -> dict[str, bytes]:
    """Every file of the ford world, by its path relative to the world's folder."""
    plan = (repo / "docs" / "plan.md").read_text(encoding="utf-8")
    assets = (repo / "spec" / "assets.md").read_text(encoding="utf-8")
    out: dict[str, str] = {
        "world.toml": _toml_block_after(plan, "`examples/ford/world.toml` is exactly this block"),
        "changes/bad_anchor.toml": _toml_block_after(plan, "`bad_anchor.toml`:\n"),
        "changes/bad_typo.toml": _toml_block_after(plan, "`bad_typo.toml`:\n"),
    }
    for name in ("vertex", "bigger", "bad_cycle", "bad_asset"):
        out[f"changes/{name}.toml"] = _table_body(plan, f"{name}.toml")
    primitive = _toml_block_after(assets, "### 6.1 `lib/packs/primitive.toml`, exactly")
    if primitive.count(LARGE_HOUSE_FROM) != 1:
        raise FixtureError("assets.md 6.1: the house_low size line is not found exactly once")
    out["packs/primitive_large.toml"] = primitive.replace(LARGE_HOUSE_FROM, LARGE_HOUSE_TO)
    return {k: v.encode("utf-8") for k, v in out.items()}


def write(repo: Path = REPO) -> list[Path]:
    files = ford_files(repo)
    written: list[Path] = []
    for case in FORD_CASES:
        folder = repo / "spec" / "cases" / case
        for rel, data in files.items():
            p = folder / rel
            p.parent.mkdir(parents=True, exist_ok=True)
            if not p.is_file() or p.read_bytes() != data:
                p.write_bytes(data)
                written.append(p)
    return written


def check(repo: Path = REPO) -> list[str]:
    """Each ford case folder holds exactly the derived files: none missing, none different."""
    files = ford_files(repo)
    problems: list[str] = []
    for case in FORD_CASES:
        folder = repo / "spec" / "cases" / case
        for rel, data in files.items():
            p = folder / rel
            if not p.is_file():
                problems.append(f"{case}/{rel}: missing")
            elif p.read_bytes() != data:
                problems.append(f"{case}/{rel}: differs from the documents")
    return problems


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--check", action="store_true", help="only check; write nothing")
    ns = ap.parse_args(argv)
    if ns.check:
        problems = check()
        for p in problems:
            print(p)
        return 1 if problems else 0
    for p in write():
        print(f"wrote {p.relative_to(REPO)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
