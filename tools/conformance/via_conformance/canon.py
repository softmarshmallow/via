"""Canonical JSON, JSON Pointers and the value equality of conformance.md 3.4.

The runner never trusts Python's own JSON writer to decide what "canonical" means: documents.md 8
fixes the form (members ordered bytewise by UTF-8 key, strings escaped minimally, no white space,
one final LF) and numbers.md 3.4 fixes how a double is written (ECMAScript's Number-to-String). A
build file is canonical exactly when re-encoding its parsed value by those rules gives its bytes.
"""

from __future__ import annotations

import json
import math
from decimal import Decimal
from fractions import Fraction
from typing import Any


class PointerError(Exception):
    """A JSON Pointer names nothing in the document; the message says which part failed."""


def _ecma_number(x: float) -> str:
    """ECMAScript Number::toString(x) in radix 10 (numbers.md 3.4).

    Python's repr gives the shortest digits that read back to the same bits (the same digits
    ECMAScript chooses); only the layout differs, so we take repr's digits and exponent and lay
    them out by ECMA-262's rules.
    """
    if math.isnan(x) or math.isinf(x):
        raise ValueError("non-finite number in canonical JSON")
    if x == 0:
        # Both zeros print as "0" in ECMAScript; a signed zero is caught elsewhere (build.md 1.4).
        return "0"
    sign = "-" if x < 0 else ""
    t = Decimal(repr(abs(x))).as_tuple()
    digits = "".join(str(d) for d in t.digits).lstrip("0")
    exp = int(t.exponent)
    stripped = digits.rstrip("0")
    exp += len(digits) - len(stripped)
    digits = stripped
    if not digits:
        return "0"
    k = len(digits)
    n = k + exp  # value = 0.<digits> * 10^n, ECMA-262's n
    if k <= n <= 21:
        return sign + digits + "0" * (n - k)
    if 0 < n <= 21:
        return sign + digits[:n] + "." + digits[n:]
    if -6 < n <= 0:
        return sign + "0." + "0" * (-n) + digits
    e = n - 1
    es = ("+" if e >= 0 else "-") + str(abs(e))
    if k == 1:
        return sign + digits + "e" + es
    return sign + digits[0] + "." + digits[1:] + "e" + es


def _string(s: str) -> str:
    out = ['"']
    for ch in s:
        o = ord(ch)
        if ch == '"':
            out.append('\\"')
        elif ch == "\\":
            out.append("\\\\")
        elif o < 0x20:
            out.append("\\u%04x" % o)
        else:
            out.append(ch)
    out.append('"')
    return "".join(out)


def encode(value: Any) -> str:
    """The canonical text of a parsed JSON value, without the final LF."""
    if value is None:
        return "null"
    if value is True:
        return "true"
    if value is False:
        return "false"
    if isinstance(value, int):
        if abs(value) >= 10**21:
            return _ecma_number(float(value))
        return str(value)
    if isinstance(value, float):
        return _ecma_number(value)
    if isinstance(value, str):
        return _string(value)
    if isinstance(value, list):
        return "[" + ",".join(encode(v) for v in value) + "]"
    if isinstance(value, dict):
        keys = sorted(value.keys(), key=lambda k: k.encode("utf-8"))
        return "{" + ",".join(_string(k) + ":" + encode(value[k]) for k in keys) + "}"
    raise TypeError(f"not a JSON value: {type(value).__name__}")


def canonical_bytes(value: Any) -> bytes:
    return (encode(value) + "\n").encode("utf-8")


def _reject_constant(name: str) -> Any:
    raise ValueError(f"non-finite number {name} in JSON")


def loads(data: bytes) -> Any:
    """Parse JSON bytes strictly: UTF-8, no NaN or Infinity, duplicate members refused."""

    def pairs(items: list[tuple[str, Any]]) -> dict[str, Any]:
        out: dict[str, Any] = {}
        for k, v in items:
            if k in out:
                raise ValueError(f"duplicate member {k!r}")
            out[k] = v
        return out

    text = data.decode("utf-8")
    return json.loads(text, object_pairs_hook=pairs, parse_constant=_reject_constant)


def is_canonical(data: bytes) -> bool:
    try:
        value = loads(data)
    except (ValueError, UnicodeDecodeError):
        return False
    try:
        return canonical_bytes(value) == data
    except (ValueError, TypeError):
        return False


def _tokens(pointer: str) -> list[str]:
    if pointer == "":
        return []
    if not pointer.startswith("/"):
        raise PointerError(f"pointer {pointer!r} does not start with '/'")
    return [t.replace("~1", "/").replace("~0", "~") for t in pointer[1:].split("/")]


def _step(node: Any, token: str, pointer: str) -> Any:
    if isinstance(node, dict):
        if token not in node:
            raise PointerError(f"{pointer}: no member {token!r}")
        return node[token]
    if isinstance(node, list):
        if not token.isdigit() or (len(token) > 1 and token[0] == "0"):
            raise PointerError(f"{pointer}: {token!r} is not an array index")
        i = int(token)
        if i >= len(node):
            raise PointerError(f"{pointer}: index {i} past the end ({len(node)} elements)")
        return node[i]
    raise PointerError(f"{pointer}: cannot step into a {type(node).__name__} with {token!r}")


def resolve(doc: Any, pointer: str) -> Any:
    """RFC 6901 resolution."""
    node = doc
    for token in _tokens(pointer):
        node = _step(node, token, pointer)
    return node


def remove(doc: Any, pointer: str) -> None:
    """Remove the member a pointer names, in place; a pointer naming nothing removes nothing.

    conformance.md 3.4's `ignore` removes members "from both sides first"; a member absent on one
    side is simply not there to remove, and the comparison of the rest decides.
    """
    tokens = _tokens(pointer)
    if not tokens:
        raise PointerError("the empty pointer names the whole document; it cannot be ignored")
    try:
        parent = doc
        for token in tokens[:-1]:
            parent = _step(parent, token, pointer)
    except PointerError:
        return
    last = tokens[-1]
    if isinstance(parent, dict):
        parent.pop(last, None)
    elif isinstance(parent, list) and last.isdigit() and int(last) < len(parent):
        del parent[int(last)]


def _is_number(v: Any) -> bool:
    return isinstance(v, (int, float)) and not isinstance(v, bool)


def values_equal(expected: Any, got: Any) -> bool:
    """conformance.md 3.4 `value`: numbers by exact value, strings and booleans exactly, lists
    element by element, tables member by member with no extra member."""
    if _is_number(expected) and _is_number(got):
        if isinstance(expected, float) and not math.isfinite(expected):
            return False
        if isinstance(got, float) and not math.isfinite(got):
            return False
        return Fraction(expected) == Fraction(got)
    if isinstance(expected, bool) or isinstance(got, bool):
        return isinstance(expected, bool) and isinstance(got, bool) and expected == got
    if isinstance(expected, str) or isinstance(got, str):
        return isinstance(expected, str) and isinstance(got, str) and expected == got
    if isinstance(expected, list) and isinstance(got, list):
        return len(expected) == len(got) and all(values_equal(a, b) for a, b in zip(expected, got))
    if isinstance(expected, dict) and isinstance(got, dict):
        return expected.keys() == got.keys() and all(
            values_equal(expected[k], got[k]) for k in expected
        )
    return expected is None and got is None


def includes(expected: Any, got: Any) -> bool:
    """conformance.md 3.4 `includes`: a table has these members with these values, or a list holds
    these elements; extras allowed. Each expected element takes a distinct element of the list."""
    if isinstance(expected, dict) and isinstance(got, dict):
        return all(k in got and values_equal(v, got[k]) for k, v in expected.items())
    if isinstance(expected, list) and isinstance(got, list):
        used = [False] * len(got)
        for e in expected:
            for i, g in enumerate(got):
                if not used[i] and values_equal(e, g):
                    used[i] = True
                    break
            else:
                return False
        return True
    return False


def show(value: Any, limit: int = 300) -> str:
    """A short canonical rendering of a value for a failure message."""
    try:
        text = encode(value)
    except (TypeError, ValueError):
        text = repr(value)
    return text if len(text) <= limit else text[: limit - 3] + "..."
