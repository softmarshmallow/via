"""BLAKE3 (O'Connor, Aumasson, Neves, Wilcox-O'Hearn, 2020), hash mode, 32-byte output, in pure
Python: the digest `build.md` 3.3 names, computed apart from the engine so a conformance case's
expected file is never recorded from the engine's own output.

    python3 tools/packgen/blake3.py FILE...    # prints `<hex>  <file>` per file
"""

from __future__ import annotations

import sys

IV = [0x6A09E667, 0xBB67AE85, 0x3C6EF372, 0xA54FF53A, 0x510E527F, 0x9B05688C, 0x1F83D9AB, 0x5BE0CD19]
PERM = [2, 6, 3, 10, 7, 0, 4, 13, 1, 11, 12, 5, 9, 14, 15, 8]
CHUNK_START, CHUNK_END, PARENT, ROOT = 1, 2, 4, 8
M32 = 0xFFFFFFFF


def _rotr(x: int, n: int) -> int:
    return ((x >> n) | (x << (32 - n))) & M32


def _g(s: list[int], a: int, b: int, c: int, d: int, x: int, y: int) -> None:
    s[a] = (s[a] + s[b] + x) & M32
    s[d] = _rotr(s[d] ^ s[a], 16)
    s[c] = (s[c] + s[d]) & M32
    s[b] = _rotr(s[b] ^ s[c], 12)
    s[a] = (s[a] + s[b] + y) & M32
    s[d] = _rotr(s[d] ^ s[a], 8)
    s[c] = (s[c] + s[d]) & M32
    s[b] = _rotr(s[b] ^ s[c], 7)


def _compress(cv: list[int], block: list[int], counter: int, length: int, flags: int) -> list[int]:
    s = cv[:8] + IV[:4] + [counter & M32, (counter >> 32) & M32, length, flags]
    m = block[:]
    for r in range(7):
        _g(s, 0, 4, 8, 12, m[0], m[1])
        _g(s, 1, 5, 9, 13, m[2], m[3])
        _g(s, 2, 6, 10, 14, m[4], m[5])
        _g(s, 3, 7, 11, 15, m[6], m[7])
        _g(s, 0, 5, 10, 15, m[8], m[9])
        _g(s, 1, 6, 11, 12, m[10], m[11])
        _g(s, 2, 7, 8, 13, m[12], m[13])
        _g(s, 3, 4, 9, 14, m[14], m[15])
        if r < 6:
            m = [m[i] for i in PERM]
    for i in range(8):
        s[i] ^= s[i + 8]
        s[i + 8] ^= cv[i]
    return s


def _words(b: bytes) -> list[int]:
    b = b.ljust(64, b"\0")
    return [int.from_bytes(b[4 * i : 4 * i + 4], "little") for i in range(16)]


class _Out:
    """An output not yet compressed: what becomes a chaining value or, with ROOT, the digest."""

    def __init__(self, cv: list[int], block: list[int], counter: int, length: int, flags: int):
        self.cv, self.block, self.counter, self.length, self.flags = cv, block, counter, length, flags

    def chaining(self) -> list[int]:
        return _compress(self.cv, self.block, self.counter, self.length, self.flags)[:8]

    def root(self) -> bytes:
        w = _compress(self.cv, self.block, 0, self.length, self.flags | ROOT)[:8]
        return b"".join(x.to_bytes(4, "little") for x in w)


def _chunk(data: bytes, index: int) -> _Out:
    cv = IV[:]
    blocks = [data[i : i + 64] for i in range(0, len(data), 64)] or [b""]
    for k, blk in enumerate(blocks):
        flags = (CHUNK_START if k == 0 else 0) | (CHUNK_END if k == len(blocks) - 1 else 0)
        if k == len(blocks) - 1:
            return _Out(cv, _words(blk), index, len(blk), flags)
        cv = _compress(cv, _words(blk), index, 64, flags)[:8]
    raise AssertionError


def _parent(left: list[int], right: list[int]) -> _Out:
    return _Out(IV[:], left + right, 0, 64, PARENT)


def digest(data: bytes) -> str:
    chunks = [data[i : i + 1024] for i in range(0, len(data), 1024)] or [b""]
    stack: list[list[int]] = []
    for n, c in enumerate(chunks[:-1]):
        cv = _chunk(c, n).chaining()
        total = n + 1
        while total & 1 == 0:
            cv = _parent(stack.pop(), cv).chaining()
            total >>= 1
        stack.append(cv)
    out = _chunk(chunks[-1], len(chunks) - 1)
    while stack:
        out = _parent(stack.pop(), out.chaining())
    return out.root().hex()


def _selftest() -> None:
    assert digest(b"") == "af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262"
    assert digest(b"abc") == "6437b3ac38465133ffb63b75273a8db548c558465d79db03fd359c6cd5bd9d85"


_selftest()

if __name__ == "__main__":
    for f in sys.argv[1:]:
        with open(f, "rb") as h:
            print(f"{digest(h.read())}  {f}")
