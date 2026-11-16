"""A step at the join (composition.md 6.5): the wild's cover carried out over the bare ground.

Reads both sides: `wild.cover` (the wild's own, 0 outside its footprint) and `abbey.clearing` (what
the abbey keeps clear: its ways, its yard). Returns `spread`: at each post, the largest cover of a
wild post within `reach` metres, times `fade` per metre of distance (8-neighbour steps, a diagonal
counted as 1.414 m), times (1 - the abbey's clearing). Inside the wild it is the wild's cover.
"""

import numpy as np


def generate(step):
    cover, clear = step.fields(["wild.cover", "abbey.clearing"])
    fade = float(step.params.get("fade", 0.965))
    reach = int(step.params.get("reach", 70))
    f = np.array(cover, dtype="<f8")
    straight, diagonal = fade, fade ** 1.4142135623730951
    for _ in range(reach):
        g = f.copy()
        for dj, di, k in ((0, 1, straight), (0, -1, straight), (1, 0, straight), (-1, 0, straight),
                          (1, 1, diagonal), (1, -1, diagonal), (-1, 1, diagonal), (-1, -1, diagonal)):
            s = np.zeros_like(f)
            sj = slice(max(dj, 0), f.shape[0] + min(dj, 0))
            si = slice(max(di, 0), f.shape[1] + min(di, 0))
            tj = slice(max(-dj, 0), f.shape[0] + min(-dj, 0))
            ti = slice(max(-di, 0), f.shape[1] + min(-di, 0))
            s[sj, si] = f[tj, ti]
            np.maximum(g, s * k, out=g)
        f = g
    f *= 1.0 - np.clip(np.array(clear, dtype="<f8"), 0.0, 1.0)
    step.out.field("spread", f.astype("<f4"), tags=["fringe"])
