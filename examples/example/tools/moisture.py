"""This project's own generate step: where the ground is wet.

User code through the same SDK the shipped libraries use (R19b), declared in the layer it writes
(layers/hamlet.toml):

    [step.moisture]
    when    = "generate"
    worker  = true
    command = ["python", "-m", "viasdk", "tools/moisture.py"]
    params  = { reach = 40 }

What it does:

1. It reads the ground (`role:surface`) and the water (`tag:water_level`) in their full view
   (authority.md 4.4): every entry of the layers at or below `hamlet`, so the tarn the fill step
   found below it is in the water, and the beck's channel is cut.
2. A post is open water where the water stands more than 2 cm over the ground. Every beck
   (a path of kind `beck`) is water along its line too, so a dry stretch of bed still counts.
3. It measures each post's distance to the nearest water, in metres, by a two-pass chamfer
   transform over the posts (steps of 1 and sqrt 2 post spacings; Borgefors, 1986), and returns
   one field of its own, `wet`, tagged `wet`: `1 - distance / reach`, clamped to 0 to 1. The dress
   layer reads it as `f('tag:wet')`.

It returns no row on the ground, so it may read the full view (authority.md 4.4). It imports only
viasdk and the standard library.
"""

import math

DIAG = math.sqrt(2.0)


def generate(step):
    reach = float(step.params.get("reach", 40))
    grid = step.grid
    r = int(grid["resolution_mm"]) / 1000.0
    x0, y0 = int(grid["extent_mm"][0]), int(grid["extent_mm"][1])
    ground, water = step.fields(["role:surface", "tag:water_level"], view="full")
    g = ground.tolist()
    w = water.tolist()
    ny, nx = len(g), len(g[0])
    big = float(nx + ny) * 2
    d = [[0.0 if w[j][i] - g[j][i] > 0.02 else big for i in range(nx)] for j in range(ny)]
    for line in _becks(step):
        for (ax, ay), (bx, by) in zip(line, line[1:]):
            n = max(1, int(math.hypot(bx - ax, by - ay) / (r * 1000.0 * 0.5)))
            for k in range(n + 1):
                x = ax + (bx - ax) * k / n
                y = ay + (by - ay) * k / n
                i = round((x - x0) / (r * 1000.0))
                j = round((y - y0) / (r * 1000.0))
                if 0 <= i < nx and 0 <= j < ny:
                    d[j][i] = 0.0
    _chamfer(d, nx, ny)
    wet = [[min(1.0, max(0.0, 1.0 - v * r / reach)) for v in row] for row in d]
    step.out.field("wet", wet, tags=["wet"])


def _chamfer(d, nx, ny):
    """Distances in post spacings to the nearest 0, in place: a forward then a backward pass."""
    for j in range(ny):
        row, below = d[j], d[j - 1] if j else None
        for i in range(nx):
            v = row[i]
            if i and row[i - 1] + 1 < v:
                v = row[i - 1] + 1
            if below is not None:
                if below[i] + 1 < v:
                    v = below[i] + 1
                if i and below[i - 1] + DIAG < v:
                    v = below[i - 1] + DIAG
                if i + 1 < nx and below[i + 1] + DIAG < v:
                    v = below[i + 1] + DIAG
            row[i] = v
    for j in range(ny - 1, -1, -1):
        row, above = d[j], d[j + 1] if j + 1 < ny else None
        for i in range(nx - 1, -1, -1):
            v = row[i]
            if i + 1 < nx and row[i + 1] + 1 < v:
                v = row[i + 1] + 1
            if above is not None:
                if above[i] + 1 < v:
                    v = above[i] + 1
                if i + 1 < nx and above[i + 1] + DIAG < v:
                    v = above[i + 1] + DIAG
                if i and above[i - 1] + DIAG < v:
                    v = above[i - 1] + DIAG
            row[i] = v


def _becks(step):
    """The line of every beck, as lists of (x, y) in millimetres (protocol.md 4.3's columns)."""
    fl = step.features("kind:beck")
    names = _list(fl["name"])
    geometry = _list(fl["geometry"])
    pieces = _list(fl["pieces"])
    rings = _list(fl["rings"])
    starts = _list(fl["ring_starts"])
    coords = [(int(c[0]), int(c[1])) for c in _list(fl["coords"])]
    out = []
    for f in range(len(names)):
        if int(geometry[f]) != 1:
            continue
        ring = rings[pieces[f]]
        out.append(coords[starts[ring]:starts[ring + 1]])
    return out


def _list(col):
    return col.tolist() if hasattr(col, "tolist") else list(col)
