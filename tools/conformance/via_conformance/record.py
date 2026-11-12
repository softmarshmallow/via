"""U7, the same results on every system (conformance.md 5.3): one record per run, and `compare`.

A record is canonical JSON, `<os>-<arch>.json`: for every case and every `via` action, the exit
status, the build `id`, the digest of each file of `files`, and each record of `diagnostics.json`
less `message` and `hint`; never the digest of the whole file, because messages
are not compared (diagnostics.md 1.3).
"""

from __future__ import annotations

from pathlib import Path
from typing import Any

from . import canon

FORMAT = "via-conformance-record-1"


def write(folder: Path, system: tuple[str, str], milestone: int, cases: dict[str, Any]) -> Path:
    folder.mkdir(parents=True, exist_ok=True)
    path = folder / f"{system[0]}-{system[1]}.json"
    doc = {
        "cases": cases,
        "format": FORMAT,
        "milestone": milestone,
        "system": {"arch": system[1], "os": system[0]},
    }
    path.write_bytes(canon.canonical_bytes(doc))
    return path


def _load(path: Path) -> dict[str, Any]:
    doc = canon.loads(path.read_bytes())
    if not isinstance(doc, dict) or doc.get("format") != FORMAT:
        raise ValueError(f"{path.name} is not a {FORMAT} record")
    return doc


def compare(a: Path, b: Path) -> tuple[list[str], list[str]]:
    """Differences between two records, and notes on what could not be compared.

    Every case in both records without `one_system` is compared action by action. A case in one
    record only is a note, not a difference: the two runs may have been at different milestones.
    """
    ra, rb = _load(a), _load(b)
    ca, cb = ra["cases"], rb["cases"]
    diffs: list[str] = []
    notes: list[str] = []
    for name in sorted(set(ca) | set(cb), key=str.encode):
        if name not in ca or name not in cb:
            notes.append(f"{name}: only in {a.name if name in ca else b.name}")
            continue
        x, y = ca[name], cb[name]
        if x.get("one_system") or y.get("one_system"):
            notes.append(f"{name}: one_system ({x.get('one_system') or y.get('one_system')})")
            continue
        xa, ya = x.get("actions", {}), y.get("actions", {})
        for act in sorted(set(xa) | set(ya), key=str.encode):
            if act not in xa or act not in ya:
                diffs.append(f"{name} action {act}: run on one system only")
                continue
            for key in ("exit", "id", "files", "diagnostics"):
                if canon.encode(xa[act].get(key)) != canon.encode(ya[act].get(key)):
                    diffs.append(
                        f"{name} action {act}: {key} {canon.show(xa[act].get(key), 120)}"
                        f" on {a.name}, {canon.show(ya[act].get(key), 120)} on {b.name}"
                    )
    return diffs, notes
