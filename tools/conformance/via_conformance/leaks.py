"""U5, nothing absolute (build.md case 7, conformance.md 5).

No JSON or text file the build writes, nor the run report or the `--diagnostics` document, holds a
path starting `/`, `\\`, `//` or a drive letter, the working folder's path, the test user name or
the host name; and no `..` but in `locations` and in source paths the text wrote so.
"""

from __future__ import annotations

import re
import socket
import sys
import tomllib
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Iterator

from . import canon

BINARY_SUFFIXES = {".npy", ".glb", ".png", ".bin", ".lock"}
DRIVE_RE = re.compile(r"^[A-Za-z]:[\\/]")
# Members whose string values are source paths (build.md 3.4): a diagnostic's `file`, an input's
# `path`, `run.changes`. They may hold `..` where the world's text wrote one (documents.md 2.1).
SOURCE_PATH_KEYS = {"file", "path"}


def host_names() -> list[str]:
    names = set()
    try:
        full = socket.gethostname()
    except OSError:
        full = ""
    for n in (full, full.split(".")[0]):
        if len(n) >= 4:
            names.add(n)
    return sorted(names)


@dataclass
class Secrets:
    """What must never appear: the working folder (as given and resolved, with native and `/`
    separators, and in its 8.3 short form on Windows), user and host names."""

    texts: list[str]

    @classmethod
    def for_run(cls, work: Path, user: str) -> "Secrets":
        # The engine writes paths wire-style, with `/` (protocol.md), so `C:\\x` leaks as `C:/x`.
        texts = {user}
        for w in (work, work.resolve()):
            texts.update({str(w), w.as_posix()})
            short = _short_path(w)
            if short:
                texts.update({short, short.replace("\\", "/")})
        texts.update(host_names())
        return cls(sorted(t for t in texts if t))

    def found_in(self, text: str) -> list[str]:
        # Windows paths are case-insensitive: `c:/users/...` names the same folder.
        if sys.platform == "win32":
            low = text.lower()
            return [s for s in self.texts if s.lower() in low]
        return [s for s in self.texts if s in text]


def _short_path(path: Path) -> str | None:
    """The 8.3 short form of a Windows path (`C:\\Users\\RUNNER~1\\...`), when it differs."""
    if sys.platform != "win32":
        return None
    import ctypes
    from ctypes import wintypes

    k = ctypes.WinDLL("kernel32", use_last_error=True)
    k.GetShortPathNameW.restype = wintypes.DWORD
    k.GetShortPathNameW.argtypes = [wintypes.LPCWSTR, wintypes.LPWSTR, wintypes.DWORD]
    buf = ctypes.create_unicode_buffer(32768)
    n = k.GetShortPathNameW(str(path), buf, len(buf))
    if n == 0 or n >= len(buf) or buf.value == str(path):
        return None
    return buf.value


def _is_absolute(s: str) -> bool:
    return s.startswith("/") or s.startswith("\\") or bool(DRIVE_RE.match(s))


# Around a path in plain text: quotes, brackets, `=` and separators of a list or a sentence.
TOKEN_SPLIT_RE = re.compile(r"[\s=,;]+")
TOKEN_STRIP = "\"'`()[]{}<>"


def _tokens(line: str) -> Iterator[str]:
    for t in TOKEN_SPLIT_RE.split(line):
        t = t.strip(TOKEN_STRIP)
        if t:
            yield t


def _has_dotdot(s: str) -> bool:
    return ".." in re.split(r"[\\/]", s)


def _walk(value: Any, trail: tuple[str, ...]) -> Iterator[tuple[tuple[str, ...], str]]:
    if isinstance(value, dict):
        for k, v in value.items():
            yield from _walk(v, trail + (k,))
    elif isinstance(value, list):
        for i, v in enumerate(value):
            yield from _walk(v, trail + (str(i),))
    elif isinstance(value, str):
        yield trail, value


def _strings(value: Any) -> Iterator[tuple[tuple[str, ...], str]]:
    return _walk(value, ())


def _exempt_dotdot(trail: tuple[str, ...]) -> bool:
    if trail and trail[0] == "locations":
        return True
    keys = [t for t in trail if not t.isdigit()]
    if keys and keys[-1] in SOURCE_PATH_KEYS:
        return True
    # manifest run.changes: a list of change names and source paths.
    return len(trail) >= 2 and trail[-2] == "changes" and "run" in trail


def scan_file(path: Path, label: str, secrets: Secrets) -> list[str]:
    """Problems found in one file; [] when it is clean or not a text file."""
    if path.suffix in BINARY_SUFFIXES:
        return []
    try:
        data = path.read_bytes()
    except OSError as e:
        return [f"{label}: cannot read: {e}"]
    if b"\x00" in data:
        return []
    try:
        text = data.decode("utf-8")
    except UnicodeDecodeError:
        return []
    problems = [f"{label} holds {s!r}" for s in secrets.found_in(text)]
    doc: Any = None
    try:
        doc = canon.loads(data)
    except ValueError:
        if path.suffix == ".toml":
            try:
                doc = tomllib.loads(text)
            except tomllib.TOMLDecodeError:
                doc = None
    if doc is not None:
        for trail, s in _strings(doc):
            where = "/" + "/".join(trail)
            if _is_absolute(s):
                problems.append(f"{label} {where}: absolute path {s!r}")
            if _has_dotdot(s) and not _exempt_dotdot(trail):
                problems.append(f"{label} {where}: '..' in {s!r}")
    else:
        for n, line in enumerate(text.splitlines(), 1):
            for token in _tokens(line):
                if _is_absolute(token):
                    problems.append(f"{label} line {n}: absolute path {token!r}")
                if _has_dotdot(token):
                    problems.append(f"{label} line {n}: '..' in {token!r}")
    return problems


def scan(paths: list[tuple[Path, str]], secrets: Secrets) -> list[str]:
    """Scan files and folders (recursively); each given with the label a message uses."""
    problems: list[str] = []
    for path, label in paths:
        if path.is_dir():
            for p in sorted(path.rglob("*"), key=lambda q: str(q).encode()):
                if p.is_file():
                    problems += scan_file(p, f"{label}/{p.relative_to(path).as_posix()}", secrets)
        elif path.is_file():
            problems += scan_file(path, label, secrets)
    return problems
