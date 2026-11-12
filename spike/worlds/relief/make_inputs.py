"""Makes the pictures this world reads, so that no artwork is committed. Run once:

    uv run python worlds/relief/make_inputs.py

They stand in for what an author already has: a 16-bit heightmap out of a terrain tool, an 8-bit
copy of it, a painted mask whose channels mean different things, and a small stamp.
"""
from pathlib import Path

import numpy as np
from PIL import Image

out = Path(__file__).parent / "inputs"
out.mkdir(exist_ok=True)
rng = np.random.default_rng(7)


def smooth_noise(n, beta):
    k = np.hypot(np.fft.fftfreq(2 * n)[:, None], np.fft.rfftfreq(2 * n)[None, :])
    k[0, 0] = 1.0
    f = np.fft.irfft2(np.fft.rfft2(rng.standard_normal((2 * n, 2 * n))) * k ** (-beta / 2), s=(2 * n, 2 * n))[:n, :n]
    return (f - f.min()) / (f.max() - f.min())


n = 257                                              # a quarter of the world's posts each way
v, u = np.mgrid[0:n, 0:n] / (n - 1)
height = np.clip(0.15 + 0.45 * (1 - v) * (0.6 + 0.4 * u) + 0.4 * smooth_noise(n, 3.4), 0, 1)   # row 0 is north, and high
Image.fromarray((height * 65535).astype(np.uint16)).save(out / "height16.png")
Image.fromarray((height * 255).astype(np.uint8)).save(out / "height8.png")

woods = (smooth_noise(n, 3.0) > 0.52) * 255          # red: where woods stand
stone = np.clip((smooth_noise(n, 2.6) - 0.6) * 6, 0, 1) * 255   # green: bare stone
Image.merge("RGB", [Image.fromarray(c.astype(np.uint8)) for c in (woods, stone, np.zeros((n, n)))]).save(out / "cover.png")

m = 64                                               # a stamp: a round hollow, white at its middle
yy, xx = np.mgrid[0:m, 0:m] - (m - 1) / 2
Image.fromarray((np.clip(1 - np.hypot(xx, yy) / (m / 2), 0, 1) ** 1.5 * 255).astype(np.uint8)).save(out / "hollow.png")
