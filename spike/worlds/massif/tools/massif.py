"""The land, by the user's own means: a spectral surface, cut by streams and relaxed by creep.

Nothing here is the compiler's. It uses numpy's FFT and scikit-image, and could use anything else.
It hands back three things: the height, a `wet` field (where water gathers) and the main stream as
a feature, which the layers above measure from like any other.
"""
import numpy as np
from skimage.morphology import reconstruction
from skimage.transform import resize

from viasdk import step

s = step()
f, p = s.frame, s.params
rng = np.random.default_rng(f.seed)
cell = float(p.get("cell", 2.0))
ny, nx = int(round((f.y1 - f.y0) / cell)) + 1, int(round((f.x1 - f.x0) / cell)) + 1


def spectral(shape, beta):
    """Noise with a power-law spectrum. Made at twice the size and cut down, so it does not repeat."""
    big = (2 * shape[0], 2 * shape[1])
    ky, kx = np.fft.fftfreq(big[0])[:, None], np.fft.rfftfreq(big[1])[None, :]
    k = np.hypot(kx, ky)
    k[0, 0] = 1.0
    spec = np.fft.rfft2(rng.standard_normal(big)) * k ** (-beta / 2)
    spec[0, 0] = 0.0
    out = np.fft.irfft2(spec, s=big)[: shape[0], : shape[1]]
    return (out - out.mean()) / out.std()


v, u = np.mgrid[0:ny, 0:nx] / np.array([ny - 1, nx - 1])[:, None, None]
rise = (0.55 * (1 - u) + 0.75 * v) / 1.3                    # high in the north-west, draining south-east
ridged = 1 - np.abs(spectral((ny, nx), 2.6))
h = 14 + float(p.get("relief", 70.0)) * rise + 10 * spectral((ny, nx), 3.2) * (0.35 + rise) + 5 * (ridged - 0.5) * rise
ramp = 1e-3 * (u * nx + (1 - v) * ny)                        # so filled hollows still drain south-east
SHIFTS = [(dy, dx, np.hypot(dy, dx) * cell) for dy in (-1, 0, 1) for dx in (-1, 0, 1) if (dy, dx) != (0, 0)]


def drainage(h):
    """For every cell: the cell it drains to, how steeply, and how many cells drain through it."""
    seed = h.copy()
    seed[1:-1, 1:-1] = h.max()
    filled = reconstruction(seed, h, method="erosion") - ramp
    pad = np.pad(filled, 1, constant_values=np.inf)
    best, recv = np.zeros_like(h), np.arange(h.size).reshape(h.shape)
    rows, cols = np.mgrid[0:ny, 0:nx]
    for dy, dx, dist in SHIFTS:
        drop = (filled - pad[1 + dy : 1 + dy + ny, 1 + dx : 1 + dx + nx]) / dist
        better = drop > best
        best = np.where(better, drop, best)
        recv = np.where(better, (rows + dy) * nx + (cols + dx), recv)
    area = np.ones(h.size)
    flat_recv = recv.ravel()
    for i in np.argsort(-filled, axis=None):                 # highest first, so every cell is complete before it gives
        if flat_recv[i] != i:
            area[flat_recv[i]] += area[i]
    return flat_recv, best, area.reshape(h.shape)


for _ in range(int(p.get("erosion_rounds", 45))):
    recv, slope, area = drainage(h)
    cut = 0.007 * (area * cell * cell) ** 0.45 * slope       # stream power
    room = np.clip(h - h.ravel()[recv].reshape(h.shape), 0, None)
    h = h - np.minimum(cut, 0.5 * room)
    pad = np.pad(h, 1, mode="edge")                          # creep rounds what the streams sharpen
    h = h + 0.2 * (pad[:-2, 1:-1] + pad[2:, 1:-1] + pad[1:-1, :-2] + pad[1:-1, 2:] - 4 * h)

recv, slope, area = drainage(h)
wet = np.log1p(area) / np.log1p(area.max())
pad = np.pad(wet, 1, mode="edge")
wet = (pad[:-2, 1:-1] + pad[2:, 1:-1] + pad[1:-1, :-2] + pad[1:-1, 2:] + 4 * wet) / 8

# the main stream: from the outlet that gathers most, upstream by the larger branch each time
flat = area.ravel()
cells = np.arange(flat.size)
gives = cells[recv != cells]
gives = gives[np.argsort(flat[gives])]
donor = np.full(flat.size, -1)
donor[recv[gives]] = gives                                   # the last written is the largest
inner = np.zeros(h.shape, bool)
inner[2:-2, 2:-2] = True
at = int(np.argmax(np.where(recv == cells, flat, 0)))
course = [at]
while donor[at] >= 0 and flat[donor[at]] > 10:
    at = int(donor[at])
    course.append(at)
course = [c for c in course[::-1] if inner.ravel()[c]]
pts = [[round(f.x0 + (i % nx) * cell, 1), round(f.y0 + (i // nx) * cell, 1)] for i in course[::6] + course[-1:]]

fine = resize(h, f.shape, order=3, mode="edge", anti_aliasing=False)
fine += 0.18 * spectral(f.shape, 2.2)                        # grain the 2 m grid cannot carry
s.write_field("height", fine, unit="m")
s.write_field("wet", np.clip(resize(wet, f.shape, order=1, mode="edge", anti_aliasing=False), 0, 1))
if len(pts) >= 3:
    s.add("feature", {"id": "beck", "kind": "guide", "path": pts})
else:
    s.warn("no_stream", "beck", "no stream long enough to draw was found; the layers above name it")
s.done()
