"""Writes inputs/survey.npy, the 16-bit survey as an array file, for changes/float_survey.toml.

    python3 examples/relief/inputs/make_survey_npy.py

An author whose terrain tool saves floats saves them as this one float format (fields.md 4.1,
challenge 31). Here the float raster is made from height16.png so the two can be compared: each
level L becomes (1 - t) * 8 + t * 74 with t = L / 65535, as fields.md 4.2 maps a PNG level, then
narrowed to a 32-bit float, the array file's only type. Its first row is south (build.md 3.2); a
PNG's top row is north, so rows are written in the opposite order.

Standard library only. Reads only what height16.png is: 16-bit grey, not interlaced.
"""
import struct
import zlib
from pathlib import Path

HERE = Path(__file__).parent
LO, HI = 8.0, 74.0


def read_grey16(path):
    data = path.read_bytes()
    assert data[:8] == b"\x89PNG\r\n\x1a\n", "not a PNG"
    pos, idat, ihdr = 8, b"", None
    while pos < len(data):
        n, kind = struct.unpack(">I4s", data[pos:pos + 8])
        body = data[pos + 8:pos + 8 + n]
        if kind == b"IHDR":
            ihdr = struct.unpack(">IIBBBBB", body)
        elif kind == b"IDAT":
            idat += body
        pos += 12 + n
    w, h, depth, colour, _, _, interlace = ihdr
    assert (depth, colour, interlace) == (16, 0, 0), "expects 16-bit grey, not interlaced"
    raw = zlib.decompress(idat)
    bpp, stride = 2, 2 * w
    rows, prev = [], bytearray(stride)
    for r in range(h):
        f = raw[r * (stride + 1)]
        line = bytearray(raw[r * (stride + 1) + 1:(r + 1) * (stride + 1)])
        for i in range(stride):
            a = line[i - bpp] if i >= bpp else 0
            b = prev[i]
            c = prev[i - bpp] if i >= bpp else 0
            if f == 1:
                line[i] = (line[i] + a) & 255
            elif f == 2:
                line[i] = (line[i] + b) & 255
            elif f == 3:
                line[i] = (line[i] + (a + b) // 2) & 255
            elif f == 4:
                p = a + b - c
                pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
                line[i] = (line[i] + (a if pa <= pb and pa <= pc else b if pb <= pc else c)) & 255
        rows.append(struct.unpack(">%dH" % w, bytes(line)))
        prev = line
    return w, h, rows


def write_npy(path, w, h, values):
    header = "{'descr': '<f4', 'fortran_order': False, 'shape': (%d, %d), }" % (h, w)
    pad = 64 - (10 + len(header) + 1) % 64
    header = header + " " * pad + "\n"
    out = b"\x93NUMPY\x01\x00" + struct.pack("<H", len(header)) + header.encode("latin1")
    out += struct.pack("<%df" % len(values), *values)
    path.write_bytes(out)


w, h, rows = read_grey16(HERE / "height16.png")
values = []
for row in reversed(rows):                 # south first
    for level in row:
        t = level / 65535.0
        values.append((1.0 - t) * LO + t * HI)
write_npy(HERE / "survey.npy", w, h, values)
print(f"survey.npy: {h} rows of {w}, {min(values):.3f} to {max(values):.3f} m")
