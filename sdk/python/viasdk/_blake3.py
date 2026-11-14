"""BLAKE3 (O'Connor, Aumasson, Neves and Wilcox-O'Hearn, 2020), hash mode, 32-byte output.

Blobs are named by the BLAKE3 digest of their bytes (protocol.md 4.1), and Python's standard
library has no BLAKE3, so the SDK carries this small implementation of the specification's
reference design to name the column blobs it sends. Chunks are compressed with NumPy, many at a
time, so a blob of a few megabytes is named in well under a second; the tree is then merged as the
specification merges it. It is checked against the official test vectors in the SDK's tests.
"""

import struct

IV = (0x6A09E667, 0xBB67AE85, 0x3C6EF372, 0xA54FF53A, 0x510E527F, 0x9B05688C, 0x1F83D9AB, 0x5BE0CD19)
MSG_PERMUTATION = (2, 6, 3, 10, 7, 0, 4, 13, 1, 11, 12, 5, 9, 14, 15, 8)
CHUNK_START, CHUNK_END, PARENT, ROOT = 1, 2, 4, 8
CHUNK_LEN, BLOCK_LEN = 1024, 64
M32 = 0xFFFFFFFF


def _rotr(x, n):
    return ((x >> n) | (x << (32 - n))) & M32


def _g(s, a, b, c, d, mx, my):
    s[a] = (s[a] + s[b] + mx) & M32
    s[d] = _rotr(s[d] ^ s[a], 16)
    s[c] = (s[c] + s[d]) & M32
    s[b] = _rotr(s[b] ^ s[c], 12)
    s[a] = (s[a] + s[b] + my) & M32
    s[d] = _rotr(s[d] ^ s[a], 8)
    s[c] = (s[c] + s[d]) & M32
    s[b] = _rotr(s[b] ^ s[c], 7)


def _compress(cv, block_words, counter, block_len, flags):
    s = list(cv) + list(IV[:4]) + [counter & M32, (counter >> 32) & M32, block_len, flags]
    m = list(block_words)
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
            m = [m[i] for i in MSG_PERMUTATION]
    for i in range(8):
        s[i] ^= s[i + 8]
        s[i + 8] ^= cv[i]
    return s


def _words(block):
    block = block + b"\0" * (BLOCK_LEN - len(block))
    return struct.unpack("<16I", block)


def _chunk_cv(chunk, counter, last_flags=0):
    """The chaining value of one chunk; ``last_flags`` is added to its last block (ROOT for a
    one-chunk input)."""
    cv = IV
    blocks = [chunk[i:i + BLOCK_LEN] for i in range(0, max(len(chunk), 1), BLOCK_LEN)] or [b""]
    for k, b in enumerate(blocks):
        flags = 0
        if k == 0:
            flags |= CHUNK_START
        if k == len(blocks) - 1:
            flags |= CHUNK_END | last_flags
        cv = tuple(_compress(cv, _words(b), counter, len(b), flags)[:8])
    return cv


def _chunk_cvs_numpy(data, n):
    """The chaining values of the first ``n`` whole chunks of ``data``, each compressed as a
    non-root chunk, with NumPy over all chunks at once."""
    import numpy as np

    words = np.frombuffer(data[: n * CHUNK_LEN], dtype="<u4").reshape(n, 16, 16).astype(np.uint32)
    counters = np.arange(n, dtype=np.uint64)
    lo = (counters & np.uint64(M32)).astype(np.uint32)
    hi = (counters >> np.uint64(32)).astype(np.uint32)
    cv = np.tile(np.array(IV, dtype=np.uint32), (n, 1))

    def rotr(x, k):
        return (x >> np.uint32(k)) | (x << np.uint32(32 - k))

    for blk in range(16):
        flags = 0
        if blk == 0:
            flags |= CHUNK_START
        if blk == 15:
            flags |= CHUNK_END
        s = [cv[:, i].copy() for i in range(8)] + [np.full(n, IV[i], dtype=np.uint32) for i in range(4)]
        s += [lo.copy(), hi.copy(), np.full(n, BLOCK_LEN, dtype=np.uint32), np.full(n, flags, dtype=np.uint32)]
        m = [words[:, blk, i] for i in range(16)]

        def g(a, b, c, d, mx, my):
            s[a] = s[a] + s[b] + mx
            s[d] = rotr(s[d] ^ s[a], 16)
            s[c] = s[c] + s[d]
            s[b] = rotr(s[b] ^ s[c], 12)
            s[a] = s[a] + s[b] + my
            s[d] = rotr(s[d] ^ s[a], 8)
            s[c] = s[c] + s[d]
            s[b] = rotr(s[b] ^ s[c], 7)

        with np.errstate(over="ignore"):
            for r in range(7):
                g(0, 4, 8, 12, m[0], m[1])
                g(1, 5, 9, 13, m[2], m[3])
                g(2, 6, 10, 14, m[4], m[5])
                g(3, 7, 11, 15, m[6], m[7])
                g(0, 5, 10, 15, m[8], m[9])
                g(1, 6, 11, 12, m[10], m[11])
                g(2, 7, 8, 13, m[12], m[13])
                g(3, 4, 9, 14, m[14], m[15])
                if r < 6:
                    m = [m[i] for i in MSG_PERMUTATION]
        cv = np.stack([s[i] ^ s[i + 8] for i in range(8)], axis=1)
    return [tuple(int(x) for x in row) for row in cv]


def _parent(left, right, flags=0):
    return tuple(_compress(IV, left + right, 0, BLOCK_LEN, PARENT | flags)[:8])


def digest(data):
    """The BLAKE3 digest of ``data`` as 64 lower-case hexadecimal digits."""
    data = bytes(data)
    n_chunks = max(1, (len(data) + CHUNK_LEN - 1) // CHUNK_LEN)
    if n_chunks == 1:
        out = _chunk_cv(data, 0, ROOT)
        return struct.pack("<8I", *out).hex()
    whole = n_chunks - 1  # every chunk but the last is whole
    try:
        cvs = _chunk_cvs_numpy(data, whole)
    except ImportError:  # pragma: no cover - NumPy is the SDK's one dependency
        cvs = [_chunk_cv(data[i * CHUNK_LEN:(i + 1) * CHUNK_LEN], i) for i in range(whole)]
    cvs.append(_chunk_cv(data[whole * CHUNK_LEN:], whole))
    # The tree: merged as the specification's incremental hasher merges, left subtrees complete
    # and of power-of-two size.
    stack = []
    for i, cv in enumerate(cvs[:-1]):
        total = i + 1
        while total & 1 == 0:
            cv = _parent(stack.pop(), cv)
            total >>= 1
        stack.append(cv)
    cv = cvs[-1]
    while stack:
        left = stack.pop()
        cv = _parent(left, cv, ROOT if not stack else 0)
    return struct.pack("<8I", *cv).hex()
