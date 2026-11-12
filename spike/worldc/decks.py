"""Decks: ways that leave the ground. A bridge over water and an overpass over a road are the same thing.

A path feature whose kind has a `deck` table gets a mesh instead of an effect on the ground. Its two
ends rest on the terrain as it stands there, which the ways that meet it have already graded; in
between it follows a profile of its own. The height field stays what it is: one surface. A deck is
the second surface above it, carried as geometry, as interiors are.

Kind:     deck = { surface, thickness, parapet, pier_every }
Feature:  width, rise (m above the straight line between its ends), ramp (m over which it gains the
          rise at each end; default half its length), over (features it must clear), clear (m).
"""
from __future__ import annotations

import numpy as np

from .world import World


def _smooth(x):
    x = np.clip(x, 0.0, 1.0)
    return x * x * (3 - 2 * x)


def _box(cx, cy, hx, hy, z0, z1, tx, ty):
    """A box with half extents hx along (tx, ty) and hy across it."""
    nx, ny = -ty, tx
    corners = [(cx + sx * hx * tx + sy * hy * nx, cy + sx * hx * ty + sy * hy * ny) for sx, sy in ((-1, -1), (1, -1), (1, 1), (-1, 1))]
    V, N, F = [], [], []
    for z, up, order in ((z1, 1.0, (0, 1, 2, 3)), (z0, -1.0, (3, 2, 1, 0))):
        base = len(V)
        V += [(corners[i][0], corners[i][1], z) for i in order]
        N += [(0.0, 0.0, up)] * 4
        F += [(base, base + 1, base + 2), (base, base + 2, base + 3)]
    for i in range(4):
        a, b = corners[i], corners[(i + 1) % 4]
        nrm = np.array([b[1] - a[1], -(b[0] - a[0]), 0.0])
        nrm /= max(np.linalg.norm(nrm), 1e-9)
        base = len(V)
        V += [(a[0], a[1], z0), (b[0], b[1], z0), (b[0], b[1], z1), (a[0], a[1], z1)]
        N += [tuple(nrm)] * 4
        F += [(base, base + 1, base + 2), (base, base + 2, base + 3)]
    return np.array(V, dtype=np.float64), np.array(N, dtype=np.float64), np.array(F, dtype=np.int64)


def build_decks(world: World) -> list[dict]:
    """One mesh per deck feature, and the check that each clears what it says it passes over."""
    g, diag = world.grid, world.diag
    if "height" not in world.fields:
        return []
    height = world.fields["height"]
    water = world.fields.get("water")
    meshes = []
    decks = [f for f in world.features if world.kinds.get(f.get("kind", ""), {}).get("deck")
             and world.shapes.get(f.get("id")) is not None and world.shapes[f["id"]].path is not None]
    # a deck that rests on another deck is built after it
    decks.sort(key=lambda f: sum(1 for k in ("from_on", "to_on") if f.get(k) in {d["id"] for d in decks}))
    tops: dict = {}                                    # deck id -> (stations, top heights)
    for feat in decks:
        spec = world.kinds[feat["kind"]]["deck"]
        shape = world.shapes[feat["id"]]
        a = world.attrs(feat)
        width, thick = float(a.get("width", 8.0)), float(spec.get("thickness", 1.0))
        path = shape.path
        n = max(2, int(path.length / 2.0) + 1)
        idx = np.linspace(0, len(path.pts) - 1, n).round().astype(int)
        pts, tan = path.pts[idx], path.tan[idx]
        along = path.s[idx]
        u = along / max(path.length, 1e-9)
        ends = g.sample(height, pts[[0, -1], 0], pts[[0, -1], 1]).astype(float)
        for which, key in ((0, "from_on"), (-1, "to_on")):      # an end may rest on another way instead of the ground
            host = feat.get(key)
            if host is None:
                continue
            if host in tops:
                hp, ht, hw = tops[host]
                k = int(np.argmin(np.hypot(hp[:, 0] - pts[which, 0], hp[:, 1] - pts[which, 1])))
                gap = float(np.hypot(hp[k, 0] - pts[which, 0], hp[k, 1] - pts[which, 1]))
                ends[which] = ht[k] - 0.06
                if gap > hw / 2 + width / 2 + 1.0:
                    diag.error("deck.detached", feat["id"], f"its end is {gap:.1f} m from the centre of '{host}', which is "
                               f"{hw:g} m wide: they do not touch", where=pts[which], hint="move the end onto the way it joins")
            elif host not in world.shapes:
                diag.error("frame.unknown", feat["id"], f"`{key}` names '{host}', which is not a feature")
        ramp = float(a.get("ramp", path.length / 2))
        settle = float(a.get("settle", 0.0))      # the difference between the ends, taken up over the last metres only
        share = _smooth((along - (path.length - settle)) / settle) if settle > 0 else u
        top = ends[0] + (ends[1] - ends[0]) * share + float(a.get("rise", 0.0)) * np.minimum(_smooth(along / ramp), _smooth((path.length - along) / ramp)) + 0.06
        tops[feat["id"]] = (pts, top, width)
        limit = a.get("max_grade")
        if limit is not None and n > 2:
            grade = np.abs(np.gradient(top, along)) * 100.0
            if grade.max() > float(limit):
                k = int(grade.argmax())
                diag.warn("deck.grade", feat["id"], f"it climbs at {grade.max():.0f}%; its limit is {float(limit):g}%", where=pts[k],
                          hint="lengthen it, lower `rise`, or lengthen `ramp`")
        ground = g.sample(height, pts[:, 0], pts[:, 1])
        if water is not None:
            ground = np.maximum(ground, g.sample(water, pts[:, 0], pts[:, 1]))
        nrm = np.column_stack([-tan[:, 1], tan[:, 0]])
        left, right = pts + nrm * width / 2, pts - nrm * width / 2
        parts = []
        # the slab: top, underside and two faces
        for (p, q, zp, zq, up) in ((left, right, top, top, 1.0), (right, left, top - thick, top - thick, -1.0),
                                   (left, left, top - thick, top, 0.0), (right, right, top, top - thick, 0.0)):
            V = np.vstack([np.column_stack([p, zp]), np.column_stack([q, zq])])
            if up:
                N = np.tile([0.0, 0.0, up], (2 * n, 1))
            else:
                side = nrm if p is left and zq is top else -nrm
                N = np.tile(np.column_stack([side, np.zeros(n)]), (2, 1))
            i = np.arange(n - 1)
            F = np.vstack([np.column_stack([i, i + n, i + n + 1]), np.column_stack([i, i + n + 1, i + 1])])
            parts.append((V, N, F if up >= 0 else F[:, ::-1]))
        # parapets, as short walls on each edge
        para = float(spec.get("parapet", 1.0))
        if para > 0:
            for edge in (left, right):
                for k in range(n - 1):
                    mid = (edge[k] + edge[k + 1]) / 2
                    seg = edge[k + 1] - edge[k]
                    ln = float(np.linalg.norm(seg))
                    z = float((top[k] + top[k + 1]) / 2)
                    parts.append(_box(mid[0], mid[1], ln / 2, 0.18, z, z + para, seg[0] / ln, seg[1] / ln))
        # what it passes over, and the room it leaves
        crossed = np.zeros(n, bool)
        need = float(a.get("clear", 4.5))
        for other in feat.get("over", []):
            osh = world.shapes.get(other)
            if osh is None:
                diag.error("frame.unknown", feat["id"], f"`over` names '{other}', which is not a feature")
                continue
            if osh.path is not None:
                dist, _ = osh.path.tree.query(pts)
                near = dist < float(world.attrs(osh.feat).get("width", 6.0)) / 2 + 2.0
            else:
                from shapely.geometry import Point, Polygon
                poly = Polygon(osh.polygon)
                near = np.array([poly.contains(Point(x, y)) for x, y in pts])
            if not near.any():
                diag.warn("deck.misses", feat["id"], f"it says it passes over '{other}' but never does")
                continue
            crossed |= near
            room = float(np.min((top - thick - ground)[near]))
            if room < need:
                k = int(np.argmin(np.where(near, top - thick - ground, np.inf)))
                diag.error("deck.clearance", feat["id"], f"it clears '{other}' by {room:.1f} m and needs {need:g} m", where=pts[k],
                           hint="raise `rise`, shorten `ramp`, or lengthen the deck so it reaches height sooner")
        # piers, where the deck stands high and nothing it passes over is underneath
        every = float(spec.get("pier_every", 0.0))
        if every > 0:
            for at in np.arange(every / 2, path.length, every):
                k = int(np.argmin(np.abs(along - at)))
                if top[k] - thick - ground[k] > 2.0 and not crossed[max(0, k - 2):k + 3].any():
                    parts.append(_box(pts[k, 0], pts[k, 1], 0.9, width * 0.32, float(ground[k]) - 1.0, float(top[k] - thick), tan[k, 0], tan[k, 1]))
        meshes.append({"id": f"deck_{feat['id']}", "material": spec.get("surface", "concrete"), "z0": 0.0, "parts": parts, "collide": True})
    return meshes
