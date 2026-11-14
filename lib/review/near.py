"""The review library's reach-a-way lint: each chosen instance stands within reach of a way.

A lint step, through the SDK, as a plain command or a worker (the same script serves both,
protocol.md 8). A world restates it (a library declares no step, protocol.md 1):

    [step.homes]
    when    = "lint"
    command = ["python", "-m", "viasdk", "lib:review/near.py"]
    params  = { assets = ["@building/house"], kinds = ["road", "lane"], within = 30 }

`params`:
  assets   asset ids or `@<taxon path>` (rules.md 7.4): the instances measured
  kinds    the kinds of the features measured to (paths, regions or points)
  within   metres: the reach allowed (default 30)

For each instance of a chosen asset, in name order, the distance from its point to the nearest
feature of those kinds (to the nearest point of a path, a region's outline or inside it, a point):
one `warning` per instance farther than `within`, code `step.<id>.far`, at the instance, with
`data.distance`, `data.within` and `data.nearest` (the feature, or `""` when the world has none),
and the hint to move it or bring a way to it. An instance within reach says nothing: a project's
own rule (`massif`'s "homes near the road") is a fault list, not a measure (R15 keeps measures for
looks).

`@` entries are resolved through `pack.query` (a plain command: the request's `pack` table,
protocol.md 2.2). Library code imports only viasdk and
the standard library (R19b).
"""

import math

EXPLAIN = {
    "far": "An instance of an asset the lint's `assets` names stands farther than `within` metres "
    "from every feature of the lint's `kinds`: a home with no way to it. `data.distance` is metres "
    "to the nearest such feature, `data.nearest` its name.",
}


def lint(step):
    p = step.params
    within = float(p.get("within", 30))
    kinds = list(p.get("kinds") or [])
    wanted = _asset_ids(step, list(p.get("assets") or []))
    shapes = []
    for k in kinds:
        shapes.extend(_shapes(step, f"kind:{k}"))
    t = step.instances("*")
    rows = sorted(zip(_list(t["name"]), _list(t["asset"]), _list(t["x"]), _list(t["y"])))
    first = True
    for name, asset, x, y in rows:
        if not _wanted(asset, wanted):
            continue
        d, nearest = nearest_shape((int(x), int(y)), shapes)
        if d is not None and d <= within * 1000:
            continue
        dist = None if d is None else round(d / 1000.0, 3)
        msg = (f"{name} stands {dist} m from {nearest}; this project allows {within} m."
               if d is not None else f"{name} has no feature of kind {', '.join(kinds)} to stand near.")
        kw = {"data": {"distance": dist if dist is not None else -1, "within": within,
                       "nearest": nearest or ""},
              "position": [int(x) / 1000.0, int(y) / 1000.0]}
        if first:
            kw["explain"] = EXPLAIN["far"]
            first = False
        step.warning("far", "instance." + name, msg,
                     "Move it within reach of a way, or bring a way to it.", **kw)


def nearest_shape(p, shapes):
    """(distance in mm, feature name) to the nearest shape, or (None, None) when there is none."""
    best = (None, None)
    for name, geom, pieces in shapes:
        d = _dist(p, geom, pieces)
        if best[0] is None or d < best[0] or (d == best[0] and name < best[1]):
            best = (d, name)
    return best


def _dist(p, geom, pieces):
    if geom == 2 and any(_in_piece(p, piece) for piece in pieces):
        return 0.0
    best = None
    for piece in pieces:
        for ring in piece:
            pts = list(ring)
            if geom == 0 or len(pts) == 1:
                segs = [(pts[0], pts[0])]
            elif geom == 2:
                segs = list(zip(pts, pts[1:] + pts[:1]))
            else:
                segs = list(zip(pts, pts[1:]))
            for a, b in segs:
                d = _seg(p, a, b)
                if best is None or d < best:
                    best = d
    return best if best is not None else math.inf


def _seg(p, a, b):
    ex, ey = b[0] - a[0], b[1] - a[1]
    wx, wy = p[0] - a[0], p[1] - a[1]
    ee = ex * ex + ey * ey
    t = 0.0 if ee == 0 else max(0.0, min(1.0, (wx * ex + wy * ey) / ee))
    dx, dy = wx - t * ex, wy - t * ey
    return math.sqrt(dx * dx + dy * dy)


def _in_piece(p, rings):
    if not _in_ring(p, rings[0]):
        return False
    return not any(_in_ring(p, h) for h in rings[1:])


def _in_ring(p, ring):
    wn = 0
    n = len(ring)
    for k in range(n):
        a, b = ring[k], ring[(k + 1) % n]
        cross = (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0])
        if a[1] <= p[1] < b[1] and cross > 0:
            wn += 1
        elif b[1] <= p[1] < a[1] and cross < 0:
            wn -= 1
    return wn != 0


def _wanted(asset, wanted):
    return asset in wanted


def _asset_ids(step, entries):
    """The asset ids the entries name: ids as written, `@` paths through the pack."""
    out = set()
    for e in entries:
        if e.startswith("@"):
            out.update(_list(step.pack(select=e).get("id", [])))
        else:
            out.add(e)
    return out


def _shapes(step, select):
    """(name, geometry, pieces of rings of (x, y) mm) per selected feature; none is an empty list
    (a plain command's selector says LookupError for nothing, protocol.md 2.2)."""
    try:
        fl = step.features(select)
    except LookupError:
        return []
    names = _list(fl.get("name", []))
    geometry = _list(fl.get("geometry", []))
    pieces = _list(fl.get("pieces", []))
    rings = _list(fl.get("rings", []))
    starts = _list(fl.get("ring_starts", []))
    coords = [tuple(int(v) for v in c) for c in _list(fl.get("coords", []))]
    out = []
    for f, name in enumerate(names):
        ps = [[coords[starts[r]:starts[r + 1]] for r in range(rings[q], rings[q + 1])]
              for q in range(pieces[f], pieces[f + 1])]
        out.append((name, int(geometry[f]), ps))
    return out


def _list(col):
    return col.tolist() if hasattr(col, "tolist") else list(col)
