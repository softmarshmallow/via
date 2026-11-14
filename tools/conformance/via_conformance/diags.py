"""Diagnostics: the `--diagnostics` document, check U2, and expectation matching (conformance.md 3.5).

Records are read from the document the runner asked for with `--diagnostics`, never from the
terminal: the terminal is not compared but for its summary line (diagnostics.md 7).
"""

from __future__ import annotations

import re
import tomllib
from dataclasses import dataclass
from fractions import Fraction
from pathlib import Path
from typing import Any

from . import canon

# build.md 11.1: run diagnostics are about this run of the engine, never in diagnostics.json.
RUN_CODES = {
    "identity.no_previous",
    "identity.reselected",
    "graph.build_file_restored",
    "graph.store_corrupt",
    "graph.store_unavailable",
    "graph.step_unstable",
    "graph.busy",
    "protocol.slow_exit",
}
NOT_RUN_BUILD_CODES = {"build.chunk_not_multiple", "build.internal"}

CODE_PART = r"[a-z][a-z0-9_]*"
CORE_CODE_RE = re.compile(rf"^{CODE_PART}\.{CODE_PART}$")
STEP_CODE_RE = re.compile(rf"^step\.{CODE_PART}\.{CODE_PART}$")


def is_run_code(code: str) -> bool:
    if code in RUN_CODES:
        return True
    return code.startswith("build.") and code not in NOT_RUN_BUILD_CODES


class Registry:
    """spec/diagnostics.toml, flat TOML keyed by code (diagnostics.md 2).

    A registry that does not exist registers nothing: every core code then fails U2, which is
    the truth until the registry is written.
    """

    def __init__(self, path: Path):
        self.path = path
        self.severity: dict[str, str] = {}
        self.missing = not path.is_file()
        self.error: str | None = None
        if self.missing:
            return
        try:
            doc = tomllib.loads(path.read_text(encoding="utf-8"))
        except (tomllib.TOMLDecodeError, UnicodeDecodeError) as e:
            self.error = f"{path.name} does not parse: {e}"
            return
        for group, names in doc.items():
            if not isinstance(names, dict):
                continue
            for name, row in names.items():
                if isinstance(row, dict) and isinstance(row.get("severity"), str):
                    self.severity[f"{group}.{name}"] = row["severity"]

    def why_unregistered(self, code: str) -> str:
        if self.missing:
            return f"{code!r} is not registered: {self.path.name} does not exist"
        if self.error:
            return f"{code!r} is not registered: {self.error}"
        return f"{code!r} is not in {self.path.name}"


@dataclass
class DiagDoc:
    """One `--diagnostics` document, split into world and run records (build.md 11.1)."""

    status: Any
    world: list[dict[str, Any]]
    run: list[dict[str, Any]]

    @property
    def records(self) -> list[dict[str, Any]]:
        return self.world + self.run


def read_doc(data: bytes) -> DiagDoc:
    """Parse a diagnostics document; raises ValueError with what is wrong."""
    doc = canon.loads(data)
    if not isinstance(doc, dict):
        raise ValueError("the document is not a JSON object")
    if doc.get("format") != "via-diagnostics-1":
        raise ValueError(f"format is {canon.show(doc.get('format'))}, not \"via-diagnostics-1\"")
    records = doc.get("diagnostics")
    if not isinstance(records, list) or not all(isinstance(r, dict) for r in records):
        raise ValueError("`diagnostics` is not a list of records")
    world = [r for r in records if not is_run_code(str(r.get("code", "")))]
    run = [r for r in records if is_run_code(str(r.get("code", "")))]
    return DiagDoc(doc.get("status"), world, run)


def _location_form(r: dict[str, Any]) -> str | None:
    """diagnostics.md 1.2: exactly one of span, file, argument."""
    span = all(k in r for k in ("file", "line", "column", "bytes"))
    has_any_span_key = any(k in r for k in ("line", "column", "bytes"))
    if "argument" in r:
        if "file" in r or has_any_span_key:
            return None
        return "argument" if isinstance(r["argument"], int) and not isinstance(r["argument"], bool) else None
    if span:
        ok = (
            isinstance(r["file"], str)
            and r["file"] != ""
            and all(isinstance(r[k], int) and not isinstance(r[k], bool) for k in ("line", "column"))
            and isinstance(r["bytes"], list)
            and len(r["bytes"]) == 2
        )
        return "span" if ok else None
    if "file" in r and not has_any_span_key:
        return "file" if isinstance(r["file"], str) and r["file"] != "" else None
    return None


def check_u2(records: list[dict[str, Any]], registry: Registry, fail_on: set[str]) -> list[str]:
    """U2 (diagnostics.md Conformance): every record well formed and registered.

    A step code `step.<id>.<name>` is checked for its form and for one severity per document; that
    `<id>` is a step of the world is left to the engine, since the runner reads no world text.
    """
    problems: list[str] = []
    step_severity: dict[str, str] = {}
    for n, r in enumerate(records):
        code = r.get("code")
        label = f"record {n} ({code})"
        if not isinstance(code, str):
            problems.append(f"{label}: no code")
            continue
        sev = r.get("severity")
        if sev not in ("error", "warning", "info"):
            problems.append(f"{label}: severity {canon.show(sev)} is not error, warning or info")
        if code.startswith("step."):
            if not STEP_CODE_RE.match(code):
                problems.append(f"{label}: a step code is step.<id>.<name>")
            elif isinstance(sev, str):
                first = step_severity.setdefault(code, sev)
                if first != sev:
                    problems.append(f"{label}: step code raised as {first} and as {sev}")
        elif not CORE_CODE_RE.match(code):
            problems.append(f"{label}: not of the form group.name")
        elif code not in registry.severity:
            problems.append(f"{label}: {registry.why_unregistered(code)}")
        elif sev != registry.severity[code]:
            problems.append(f"{label}: severity {sev}, the registry says {registry.severity[code]}")
        subject = r.get("subject")
        if not isinstance(subject, str) or not subject:
            problems.append(f"{label}: no subject")
        if _location_form(r) is None:
            problems.append(f"{label}: not exactly one location (span, file or argument)")
        hint = r.get("hint")
        if not isinstance(hint, str) or not hint.strip():
            problems.append(f"{label}: empty or missing hint")
        if "failed_on" in r:
            if r["failed_on"] is not True:
                problems.append(f"{label}: failed_on is {canon.show(r['failed_on'])}, not true")
            elif code not in fail_on:
                problems.append(f"{label}: failed_on on a code --fail-on did not name")
    return problems


def _agrees(exp: dict[str, Any], r: dict[str, Any]) -> bool:
    """Does a record agree with every key an expectation writes (3.5)?"""
    for k, v in exp.items():
        if k in ("records", "from"):
            continue
        if k == "count":
            got = r.get("count", 1)
            if not canon.values_equal(v, got):
                return False
        elif k == "occurrences":
            occ = r.get("occurrences", [])
            if not isinstance(occ, list) or len(occ) != v:
                return False
        elif k == "data":
            data = r.get("data")
            if not isinstance(data, dict):
                return False
            for dk, dv in v.items():
                if dk not in data or not canon.values_equal(dv, data[dk]):
                    return False
        elif k == "data_has":
            data = r.get("data")
            if not isinstance(data, dict) or any(n not in data or data[n] is None for n in v):
                return False
        elif k == "data_below":
            data = r.get("data")
            if not isinstance(data, dict):
                return False
            for lo, hi in v.items():
                a, b = data.get(lo), data.get(hi)
                if not all(isinstance(n, (int, float)) and not isinstance(n, bool) for n in (a, b)):
                    return False
                if not Fraction(a) < Fraction(b):
                    return False
        elif k == "related":
            rel = r.get("related")
            if not isinstance(rel, list):
                return False
            for want in v:
                if not any(isinstance(g, dict) and canon.includes(want, g) for g in rel):
                    return False
        elif k == "position_within":
            pos = r.get("position")
            if not isinstance(pos, list) or len(pos) not in (2, 3):
                return False
            if not all(isinstance(n, (int, float)) and not isinstance(n, bool) for n in pos):
                return False
            x0, y0, x1, y1 = (Fraction(n) for n in v)
            if not (x0 <= Fraction(pos[0]) <= x1 and y0 <= Fraction(pos[1]) <= y1):
                return False
        elif k == "chain":
            chain = r.get("chain")
            if not isinstance(chain, list) or len(chain) != len(v):
                return False
            for want, got in zip(v, chain):
                if not isinstance(got, dict) or not canon.includes(want, got):
                    return False
        else:
            if k not in r or not canon.values_equal(v, r[k]):
                return False
    return True


def _assign(slots: list[tuple[str, dict[str, Any]]], records: list[dict[str, Any]]) -> list[int | None]:
    """Maximum bipartite matching of expectation slots to records (Kuhn's augmenting paths).

    3.5 asks whether an assignment exists in which no record serves two expectations; a greedy
    first-fit could miss one that exists, so this searches for it.
    """
    adj = [[i for i, r in enumerate(records) if _agrees(exp, r)] for _, exp in slots]
    owner: list[int | None] = [None] * len(records)

    def augment(s: int, seen: list[bool]) -> bool:
        for i in adj[s]:
            if seen[i]:
                continue
            seen[i] = True
            if owner[i] is None or augment(owner[i], seen):
                owner[i] = s
                return True
        return False

    for s in range(len(slots)):
        augment(s, [False] * len(records))
    taken: list[int | None] = [None] * len(slots)
    for i, s in enumerate(owner):
        if s is not None:
            taken[s] = i
    return taken


def match(
    expectations: dict[str, dict[str, Any]], doc: DiagDoc, mode: str
) -> list[tuple[str, str, str]]:
    """Compare expectations with a document. Returns failures as (check id, expected, got)."""
    if mode == "any":
        return []
    failures: list[tuple[str, str, str]] = []
    step_expected = any(
        str(e.get("code", "")).startswith("step.") for e in expectations.values() if e.get("from", "world") == "world"
    )
    for source in ("world", "run"):
        exps = {cid: e for cid, e in expectations.items() if e.get("from", "world") == source}
        records = doc.world if source == "world" else doc.run
        slots = [(cid, e) for cid in sorted(exps) for e in [exps[cid]] for _ in range(e.get("records", 1))]
        taken = _assign(slots, records)
        short: dict[str, int] = {}
        for (cid, _), t in zip(slots, taken):
            if t is None:
                short[cid] = short.get(cid, 0) + 1
        for cid, n in sorted(short.items()):
            want = exps[cid].get("records", 1)
            similar = [r for r in records if r.get("code") == exps[cid]["code"]]
            got = (
                f"{want - n} of {want} matched; records with code {exps[cid]['code']}: "
                + (canon.show(similar, 600) if similar else "none")
            )
            failures.append((f"diag.{cid}", canon.show(exps[cid]), got))
        for cid in sorted(exps):
            if exps[cid].get("records", 1) == 0:
                hits = [r for r in records if _agrees(exps[cid], r)]
                if hits:
                    failures.append((f"diag.{cid}", f"no record agrees with {canon.show(exps[cid])}",
                                     canon.show(hits, 600)))
        if source == "world" and mode == "exact":
            used = {t for t in taken if t is not None}
            left = [
                r
                for i, r in enumerate(records)
                if i not in used and (step_expected or not str(r.get("code", "")).startswith("step."))
            ]
            if left:
                failures.append(("diag", "no world diagnostic left over (mode exact)", canon.show(left, 600)))
    return failures
