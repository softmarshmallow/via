"""Field files: the .npy 1.0 subset of protocol.md 4.2 and build.md 3.2.

The header is written here rather than by ``numpy.save``, which pads further on some releases: the
build writes the smallest padding, and a step's returned field should be the same bytes as the
build's (one format, build.md 3.2). Reading needs no code of ours: ``numpy.load(mmap_mode="r")``.
"""

import struct

MAGIC = b"\x93NUMPY\x01\x00"


def header(descr, shape):
    """The header bytes for ``descr`` ('<f4' or '|u1') and ``shape``, padded with spaces to the
    smallest multiple of 64 bytes and ended by LF."""
    dims = ", ".join(str(int(d)) for d in shape)
    if len(shape) == 1:
        dims += ","
    text = "{'descr': '%s', 'fortran_order': False, 'shape': (%s), }" % (descr, dims)
    unpadded = len(MAGIC) + 2 + len(text) + 1
    pad = (-unpadded) % 64
    body = text.encode("ascii") + b" " * pad + b"\n"
    return MAGIC + struct.pack("<H", len(body)) + body


def write_field(path, array):
    """Writes a float32 field file of 2 or 3 dimensions.

    Refuses non-finite values (R26) and writes -0.0 as +0.0 (protocol.md 4.2): adding +0.0 under
    round-to-nearest turns -0.0 into +0.0 and changes no other value.
    """
    import numpy as np

    a = np.asarray(array)
    if a.ndim not in (2, 3):
        raise ValueError(f"a field is 2 or 3 dimensions, not {a.ndim}")
    a = np.asarray(a, dtype=np.float32)
    if not np.isfinite(a).all():
        bad = np.argwhere(~np.isfinite(a))[0].tolist()
        raise ValueError(f"non-finite value at index {bad}; no non-finite value enters a build")
    # +0.0 first, '<f4' last: arithmetic gives native byte order, so converting after it is what
    # makes the bytes little-endian on a big-endian host too, matching the header.
    a = np.ascontiguousarray(a + np.float32(0.0), dtype="<f4")
    with open(path, "wb") as f:
        f.write(header("<f4", a.shape))
        f.write(memoryview(a).cast("B"))
