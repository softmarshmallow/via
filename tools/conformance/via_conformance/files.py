"""File checks of conformance.md 3.4 (`[action.<a>.file.<id>]`) and the array reads they need.

Array files are read with numpy alone (build.md 3.2: `numpy.load(path, mmap_mode="r")`).
"""

from __future__ import annotations

from fractions import Fraction
from pathlib import Path
from typing import Any

import numpy as np

from . import canon

Failure = tuple[str, str, str]  # (check id, expected, got)


def load_array(path: Path) -> np.ndarray:
    return np.load(path, mmap_mode="r", allow_pickle=False)


def descr(arr: np.ndarray) -> str:
    """The header's `descr` as NumPy writes it: `<f4`, `<i4`, `|u1`."""
    return arr.dtype.str


def narrow(value: float | int, arr: np.ndarray) -> Any:
    """A case's number brought to the element type before an exact compare (conformance.md 3.4):
    a 32-bit float element takes the nearest 32-bit float, ties to even (numbers.md 8)."""
    if arr.dtype.kind == "f":
        return arr.dtype.type(value)
    return value


def element_equal(element: Any, value: float | int, arr: np.ndarray, tolerance: float) -> bool:
    want = narrow(value, arr)
    if arr.dtype.kind == "f":
        e = float(element)
        w = float(want)
        if tolerance == 0:
            return Fraction(e) == Fraction(w)
        return abs(Fraction(e) - Fraction(w)) <= Fraction(tolerance)
    e = int(element)
    if tolerance == 0:
        return Fraction(e) == Fraction(value)
    return abs(Fraction(e) - Fraction(value)) <= Fraction(tolerance)


def _first_unequal(flat: np.ndarray, value: float | int, arr: np.ndarray, tol: float) -> int | None:
    """The first element not equal to `value`, vectorised; exact where `element_equal` is exact.

    Float elements are compared in their own type against the narrowed value, which is exact;
    with a tolerance the difference is taken in doubles, which hold every f4 and i4 exactly.
    """
    if flat.size == 0:
        return None
    if tol == 0:
        if arr.dtype.kind == "f":
            ok = flat == narrow(value, arr)
        elif isinstance(value, float) and not value.is_integer():
            ok = np.zeros(flat.shape, dtype=bool)
        else:
            ok = flat.astype(object) == int(value) if arr.dtype.itemsize > 4 else flat == int(value)
    else:
        want = float(narrow(value, arr))
        ok = np.abs(flat.astype(np.float64) - want) <= tol
    bad = np.flatnonzero(~np.asarray(ok, dtype=bool))
    return int(bad[0]) if bad.size else None


def element_bits(element: Any, arr: np.ndarray) -> int:
    raw = np.asarray(element, dtype=arr.dtype).tobytes()
    order = "little" if arr.dtype.byteorder in ("<", "|", "=") else "big"
    return int.from_bytes(raw, order)


def _json_at(data: bytes, line: int | None, where: str) -> Any:
    if line is None:
        return canon.loads(data)
    lines = data.split(b"\n")
    if lines and lines[-1] == b"":
        lines.pop()
    if line > len(lines):
        raise ValueError(f"{where} has {len(lines)} lines; line {line} asked for")
    return canon.loads(lines[line - 1])


def _equals_json(got: bytes, want: bytes, ignore: list[str]) -> tuple[bool, str]:
    if not canon.is_canonical(got):
        return False, "the file is not canonical JSON"
    if not canon.is_canonical(want):
        return False, "the expected file is not canonical JSON"
    a = canon.loads(got)
    b = canon.loads(want)
    for p in ignore:
        canon.remove(a, p)
        canon.remove(b, p)
    if canon.encode(a) == canon.encode(b):
        return True, ""
    return False, canon.show(a, 600)


def run_file_check(
    cid: str, t: dict[str, Any], work: Path, case_folder: Path, stdout: bytes
) -> list[Failure]:
    """One `[action.<a>.file.<id>]` table. Returns failures; [] when every key holds."""
    check = f"file.{cid}"
    fails: list[Failure] = []

    def fail(key: str, expected: Any, got: Any) -> None:
        """Expected and got as descriptions (plain text)."""
        fails.append((f"{check}.{key}", str(expected), str(got)))

    def fail_value(key: str, expected: Any, got: Any) -> None:
        """Expected and got as values, written as canonical JSON so `"1"` and `1` differ."""
        fails.append((f"{check}.{key}", canon.show(expected), canon.show(got)))

    if "stream" in t:
        data: bytes | None = stdout
        path = None
        label = "standard output"
    else:
        path = work / t["path"]
        label = t["path"]
        data = None

    if "exists" in t:
        there = path is not None and (path.exists() or path.is_symlink())
        if there != t["exists"]:
            fail("exists", f"{label} {'exists' if t['exists'] else 'absent'}",
                 f"{label} {'exists' if there else 'absent'}")
        if not t["exists"]:
            return fails
    if path is not None and not path.exists():
        others = [k for k in t if k not in ("path", "exists")]
        if others:
            fail(others[0], f"{label} to exist", "it does not exist")
        return fails

    if "listing" in t:
        if not path.is_dir():
            fail("listing", f"{label} a folder", "not a folder")
        else:
            got = sorted(p.name for p in path.iterdir())
            want = sorted(t["listing"])
            if got != want:
                fail_value("listing", want, got)

    def read() -> bytes:
        nonlocal data
        if data is None:
            data = path.read_bytes()
        return data

    if any(k in t for k in ("equals", "text", "size", "pointer")) and path is not None and path.is_dir():
        fail("path", f"{label} a file", "a folder")
        return fails

    if "equals" in t:
        want = (case_folder / t["equals"]).read_bytes()
        if "ignore" in t:
            ok, got = _equals_json(read(), want, t["ignore"])
            if not ok:
                fail("equals", f"{t['equals']} less {t['ignore']}", got)
        elif read() != want:
            fail("equals", f"the bytes of {t['equals']}", f"{len(read())} bytes that differ")
    if "text" in t:
        try:
            got_text = read().decode("utf-8")
        except UnicodeDecodeError:
            got_text = None
        if got_text != t["text"]:
            fail_value("text", t["text"], got_text if got_text is not None else "(bytes that are not UTF-8)")
    if "size" in t and len(read()) != t["size"]:
        fail("size", str(t["size"]), str(len(read())))
    if "pointer" in t:
        try:
            doc = _json_at(read(), t.get("line"), label)
            got = canon.resolve(doc, t["pointer"])
        except (ValueError, UnicodeDecodeError, canon.PointerError) as e:
            fail("pointer", t["pointer"], str(e))
        else:
            if "value" in t and not canon.values_equal(t["value"], got):
                fail_value("value", t["value"], got)
            if "includes" in t and not canon.includes(t["includes"], got):
                fail_value("includes", t["includes"], got)
            if "value_file" in t:
                text = (case_folder / t["value_file"]).read_bytes().decode("utf-8")
                if text.endswith("\n"):
                    text = text[:-1]
                if not isinstance(got, str) or got != text:
                    fail_value("value_file", text, got)

    if any(k in t for k in ("dtype", "shape", "post", "all")):
        try:
            arr = load_array(path)
        except (ValueError, OSError) as e:
            fail("array", f"{label} an array file", str(e))
            return fails
        if "dtype" in t and descr(arr) != t["dtype"]:
            fail_value("dtype", t["dtype"], descr(arr))
        if "shape" in t and list(arr.shape) != t["shape"]:
            fail_value("shape", t["shape"], list(arr.shape))
        tol = t.get("tolerance", 0)
        if "post" in t:
            i, j = t["post"]
            index: tuple[int, ...] = (j, i)
            if "plane" in t:
                index = (t["plane"], j, i)
            if len(index) != arr.ndim or any(x >= n for x, n in zip(index, arr.shape)):
                fail("post", f"post {t['post']}" + (f" plane {t['plane']}" if "plane" in t else ""),
                     f"outside shape {list(arr.shape)}")
            else:
                element = arr[index]
                if "value" in t and not element_equal(element, t["value"], arr, tol):
                    fail_value("value", t["value"], element.item())
                if "bits" in t:
                    want_bits = int(t["bits"], 16)
                    got_bits = element_bits(element, arr)
                    if want_bits != got_bits:
                        width = arr.dtype.itemsize * 2
                        fail("bits", t["bits"], f"0x{got_bits:0{width}X}")
        if "all" in t:
            flat = np.asarray(arr).reshape(-1)
            bad = _first_unequal(flat, t["all"], arr, tol)
            if bad is not None:
                fail("all", t["all"], f"element {bad} (flat, C order) is {flat[bad].item()}")
    return fails
