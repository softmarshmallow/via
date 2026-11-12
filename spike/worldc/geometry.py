"""Plain geometry and noise: paths, polygons, a gradient noise and smoothstep.

Everything here is a pure function of its arguments (or, for `PathGeom`, a value built once from
them). Nothing knows about the world, its grid or its diagnostics.
"""
from __future__ import annotations

import math

import numpy as np
from scipy.spatial import cKDTree


def smooth_path(points: list, spacing: float = 0.25, smooth: bool = True) -> np.ndarray:
    """A polyline through the points (Catmull-Rom when smooth), resampled to an even spacing."""
    p = np.asarray(points, dtype=np.float64)
    if smooth and len(p) > 2:
        ext = np.vstack([2 * p[0] - p[1], p, 2 * p[-1] - p[-2]])
        out = []
        for i in range(1, len(ext) - 2):
            p0, p1, p2, p3 = ext[i - 1], ext[i], ext[i + 1], ext[i + 2]
            n = max(8, int(np.linalg.norm(p2 - p1) / spacing))
            u = np.linspace(0.0, 1.0, n, endpoint=False)[:, None]
            out.append(0.5 * ((2 * p1) + (-p0 + p2) * u + (2 * p0 - 5 * p1 + 4 * p2 - p3) * u ** 2
                              + (-p0 + 3 * p1 - 3 * p2 + p3) * u ** 3))
        out.append(p[-1][None, :])
        p = np.vstack(out)
    seg = np.linalg.norm(np.diff(p, axis=0), axis=1)
    s = np.concatenate([[0.0], np.cumsum(seg)])
    n = max(2, int(s[-1] / spacing) + 1)
    si = np.linspace(0.0, s[-1], n)
    return np.stack([np.interp(si, s, p[:, 0]), np.interp(si, s, p[:, 1])], axis=1)


class PathGeom:
    def __init__(self, points: list, smooth: bool = True) -> None:
        self.pts = smooth_path(points, smooth=smooth)
        d = np.gradient(self.pts, axis=0)
        self.tan = d / np.maximum(np.linalg.norm(d, axis=1, keepdims=True), 1e-9)
        seg = np.linalg.norm(np.diff(self.pts, axis=0), axis=1)
        self.s = np.concatenate([[0.0], np.cumsum(seg)])
        self.length = float(self.s[-1])
        self.tree = cKDTree(self.pts)

    def at(self, t: float):
        """Position, tangent and left normal at fraction t of the arc length."""
        i = int(np.clip(np.searchsorted(self.s, t * self.length), 0, len(self.pts) - 1))
        tan = self.tan[i]
        return self.pts[i], tan, np.array([-tan[1], tan[0]])

    def bbox(self):
        return self.pts[:, 0].min(), self.pts[:, 1].min(), self.pts[:, 0].max(), self.pts[:, 1].max()


def ellipse_polygon(center, size, n: int = 72) -> list:
    a = np.linspace(0, 2 * np.pi, n, endpoint=False)
    return [[center[0] + 0.5 * size[0] * math.cos(t), center[1] + 0.5 * size[1] * math.sin(t)] for t in a]


def rect_polygon(center, size, yaw_deg: float) -> list:
    """Footprint corners. `size` is [across the front, depth]; yaw is where the front looks, CCW from east."""
    th = math.radians(yaw_deg)
    f = np.array([math.cos(th), math.sin(th)])
    r = np.array([math.sin(th), -math.cos(th)])
    c = np.asarray(center, dtype=np.float64)
    hw, hd = 0.5 * size[0], 0.5 * size[1]
    return [list(c + sx * hw * r + sy * hd * f) for sx, sy in ((-1, -1), (1, -1), (1, 1), (-1, 1))]


def polygons_overlap(a: list, b: list) -> bool:
    """Separating axis test for two convex polygons."""
    pa, pb = np.asarray(a), np.asarray(b)
    for poly in (pa, pb):
        for i in range(len(poly)):
            e = poly[(i + 1) % len(poly)] - poly[i]
            axis = np.array([-e[1], e[0]])
            if (pa @ axis).max() < (pb @ axis).min() or (pb @ axis).max() < (pa @ axis).min():
                return False
    return True


def _perlin(x: np.ndarray, y: np.ndarray, seed: int) -> np.ndarray:
    rng = np.random.default_rng(seed)
    perm = rng.permutation(256).astype(np.int32)
    ang = rng.uniform(0, 2 * np.pi, 256)
    gx, gy = np.cos(ang).astype(np.float32), np.sin(ang).astype(np.float32)
    xi, yi = np.floor(x).astype(np.int32), np.floor(y).astype(np.int32)
    xf, yf = (x - xi).astype(np.float32), (y - yi).astype(np.float32)
    u = xf * xf * xf * (xf * (xf * 6 - 15) + 10)
    v = yf * yf * yf * (yf * (yf * 6 - 15) + 10)

    def grad(ix, iy, dx, dy):
        h = perm[(perm[ix & 255] + (iy & 255)) & 255]
        return gx[h] * dx + gy[h] * dy

    a = grad(xi, yi, xf, yf)
    b = grad(xi + 1, yi, xf - 1, yf)
    c = grad(xi, yi + 1, xf, yf - 1)
    d = grad(xi + 1, yi + 1, xf - 1, yf - 1)
    top = a + u * (b - a)
    return (top + v * ((c + u * (d - c)) - top)) * 1.41


def smoothstep(a, b, v):
    t = np.clip((v - a) / (b - a + 1e-12), 0.0, 1.0)
    return t * t * (3.0 - 2.0 * t)
