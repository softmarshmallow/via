"""Writes the pictures relief reads, standing in for an author's own (challenge 31):

    python3 examples/relief/inputs/make_inputs.py

- height16.png: a 16-bit grey heightmap, 257 by 257, top row north, as a terrain tool saves one.
  Moorland rising from the south-west to the north-east, with rolling noise over it.
- height8.png: the same land at 8 bits, as a careless export would save it (changes/coarse.toml).
- cover.png: a painted mask, RGB 8-bit: red where woods stand, green where stone lies bare.
- hollow.png: a 64 by 64 grey stamp, white at its middle, black at its rim: a round hollow.

Standard library only, so the pictures are made the same way on every machine: value noise on
seeded lattices (random.Random), smoothed between lattice points, written by a small PNG writer.
The same script writes the same bytes; run it again after changing it, never edit a picture.
"""
import math
import random
import struct
import zlib
from pathlib import Path

HERE = Path(__file__).parent
N = 257                                   # samples across: one per metre over the 256 m world


def lattice(seed, cells):
    rng = random.Random(seed)
    return [[rng.random() for _ in range(cells + 2)] for _ in range(cells + 2)]


def value_noise(seed, octaves, base_cells):
    """Sum of octaves of smoothed lattice noise over N by N, 0 to 1 before normalising."""
    out = [[0.0] * N for _ in range(N)]
    amp, cells, total = 1.0, base_cells, 0.0
    for o in range(octaves):
        lat = lattice(seed * 101 + o, cells)
        for r in range(N):
            y = r / (N - 1) * cells
            j = min(int(y), cells - 1)
            fy = y - j
            sy = fy * fy * (3 - 2 * fy)
            row = out[r]
            for c in range(N):
                x = c / (N - 1) * cells
                i = min(int(x), cells - 1)
                fx = x - i
                sx = fx * fx * (3 - 2 * fx)
                a = lat[j][i] + (lat[j][i + 1] - lat[j][i]) * sx
                b = lat[j + 1][i] + (lat[j + 1][i + 1] - lat[j + 1][i]) * sx
                row[c] += amp * (a + (b - a) * sy)
        total += amp
        amp *= 0.5
        cells *= 2
    return [[v / total for v in row] for row in out]


def png(path, width, height, colour, depth, rows):
    """colour 0 (grey) or 2 (RGB); rows of samples (tuples for RGB); no filtering."""
    raw = bytearray()
    fmt = ">%dH" if depth == 16 else ">%dB"
    for row in rows:
        flat = [v for px in row for v in px] if colour == 2 else row
        raw.append(0)
        raw += struct.pack(fmt % len(flat), *flat)

    def chunk(kind, body):
        return struct.pack(">I", len(body)) + kind + body + struct.pack(">I", zlib.crc32(kind + body))

    ihdr = struct.pack(">IIBBBBB", width, height, depth, colour, 0, 0, 0)
    data = b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", ihdr) + chunk(b"IDAT", zlib.compress(bytes(raw), 9))
    path.write_bytes(data + chunk(b"IEND", b""))


# ---- the land: a rise to the north-east, rolling noise, normalised to 0.08 .. 0.92
rolling = value_noise(11, 5, 3)
land = []
for r in range(N):
    v = 1 - r / (N - 1)                   # 0 at the south row, 1 at the north
    row = []
    for c in range(N):
        u = c / (N - 1)                   # 0 at the west, 1 at the east
        rise = 0.55 * v + 0.2 * u * v
        row.append(rise + 0.45 * rolling[r][c])
    land.append(row)
lo = min(min(row) for row in land)
hi = max(max(row) for row in land)
land = [[0.08 + 0.84 * (x - lo) / (hi - lo) for x in row] for row in land]
png(HERE / "height16.png", N, N, 0, 16, [[round(x * 65535) for x in row] for row in land])
png(HERE / "height8.png", N, N, 0, 8, [[round(x * 255) for x in row] for row in land])

# ---- the cover: woods where one noise is high, stone where another is very high
woods = value_noise(23, 4, 4)
stone = value_noise(37, 4, 6)
cover = []
for r in range(N):
    row = []
    for c in range(N):
        red = 255 if woods[r][c] > 0.55 else 0
        green = round(255 * min(1.0, max(0.0, (stone[r][c] - 0.62) * 7)))
        row.append((red, green, 0))
    cover.append(row)
png(HERE / "cover.png", N, N, 2, 8, cover)

# ---- the stamp: a round hollow, 1 at the middle falling to 0 at the rim along a cosine
M = 64
stamp = []
for r in range(M):
    row = []
    for c in range(M):
        d = math.hypot(c - (M - 1) / 2, r - (M - 1) / 2) / (M / 2)
        row.append(round(255 * (0.5 + 0.5 * math.cos(math.pi * d)) if d < 1 else 0))
    stamp.append(row)
png(HERE / "hollow.png", M, M, 0, 8, stamp)
print("wrote height16.png, height8.png, cover.png, hollow.png")
