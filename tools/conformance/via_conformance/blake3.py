"""BLAKE3 in pure Python (O'Connor, Aumasson, Neves and Wilcox-O'Hearn, 2020; `build.md` 3.3).

The runner reads with the standard library and numpy only (conformance.md 6), and neither has
BLAKE3, so this is the reference implementation's hash mode (no key, no derive-key), 256-bit output.
It is slow (about a megabyte a second) and is used only on small files: a manifest, a blob a raw
worker writes. The self-tests check it against the published vectors and against store names.

Run as a program it prints the digest of each file named, as `b3sum` does:
`python -m via_conformance.blake3 FILE...`.
"""

from __future__ import annotations

import sys

_IV = (0x6A09E667, 0xBB67AE85, 0x3C6EF372, 0xA54FF53A, 0x510E527F, 0x9B05688C, 0x1F83D9AB, 0x5BE0CD19)
_PERM = (2, 6, 3, 10, 7, 0, 4, 13, 1, 11, 12, 5, 9, 14, 15, 8)
_CHUNK_START, _CHUNK_END, _PARENT, _ROOT = 1, 2, 4, 8
_BLOCK, _CHUNK = 64, 1024
_M = 0xFFFFFFFF


def _compress(cv: tuple[int, ...] | list[int], m: list[int], counter: int, block_len: int, flags: int) -> list[int]:
    s = [*cv, *_IV[:4], counter & _M, (counter >> 32) & _M, block_len, flags]
    m = list(m)
    for r in range(7):
        for a, b, c, d, x, y in ((0, 4, 8, 12, 0, 1), (1, 5, 9, 13, 2, 3), (2, 6, 10, 14, 4, 5),
                                 (3, 7, 11, 15, 6, 7), (0, 5, 10, 15, 8, 9), (1, 6, 11, 12, 10, 11),
                                 (2, 7, 8, 13, 12, 13), (3, 4, 9, 14, 14, 15)):
            sa = (s[a] + s[b] + m[x]) & _M
            sd = s[d] ^ sa
            sd = ((sd >> 16) | (sd << 16)) & _M
            sc = (s[c] + sd) & _M
            sb = s[b] ^ sc
            sb = ((sb >> 12) | (sb << 20)) & _M
            sa = (sa + sb + m[y]) & _M
            sd ^= sa
            sd = ((sd >> 8) | (sd << 24)) & _M
            sc = (sc + sd) & _M
            sb ^= sc
            sb = ((sb >> 7) | (sb << 25)) & _M
            s[a], s[b], s[c], s[d] = sa, sb, sc, sd
        if r < 6:
            m = [m[i] for i in _PERM]
    for i in range(8):
        s[i] ^= s[i + 8]
        s[i + 8] ^= cv[i]
    return s


def _words(block: bytes) -> list[int]:
    block = block.ljust(_BLOCK, b"\0")
    return [int.from_bytes(block[i:i + 4], "little") for i in range(0, _BLOCK, 4)]


def _chunk_output(chunk: bytes, counter: int) -> tuple[list[int], list[int], int, int, int]:
    """The chunk's last compression, not yet done: (cv, words, counter, block_len, flags)."""
    cv: list[int] = list(_IV)
    blocks = [chunk[i:i + _BLOCK] for i in range(0, len(chunk), _BLOCK)] or [b""]
    for n, block in enumerate(blocks[:-1]):
        cv = _compress(cv, _words(block), counter, _BLOCK, _CHUNK_START if n == 0 else 0)[:8]
    last = blocks[-1]
    flags = _CHUNK_END | (_CHUNK_START if len(blocks) == 1 else 0)
    return cv, _words(last), counter, len(last), flags


def digest(data: bytes) -> str:
    """The BLAKE3 digest of `data`, 64 lower-case hexadecimal digits."""
    data = bytes(data)
    chunks = [data[i:i + _CHUNK] for i in range(0, len(data), _CHUNK)] or [b""]
    stack: list[list[int]] = []
    for n, chunk in enumerate(chunks[:-1]):
        cv = _compress(*_chunk_output(chunk, n))[:8]
        total = n + 1
        while total & 1 == 0:
            cv = _compress(_IV, stack.pop() + cv, 0, _BLOCK, _PARENT)[:8]
            total >>= 1
        stack.append(cv)
    cv, words, counter, block_len, flags = _chunk_output(chunks[-1], len(chunks) - 1)
    while stack:
        right = _compress(cv, words, counter, block_len, flags)[:8]
        cv, words, counter, block_len, flags = list(_IV), stack.pop() + right, 0, _BLOCK, _PARENT
    out = _compress(cv, words, 0, block_len, flags | _ROOT)[:8]
    return b"".join(w.to_bytes(4, "little") for w in out).hex()


def main(argv: list[str]) -> int:
    for p in argv:
        with open(p, "rb") as fh:
            print(f"{digest(fh.read())}  {p}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
