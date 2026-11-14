"""Massif's own rule: every home stands within reach of a road.

A lint step of this project, not of a library: user code through the same SDK our libraries use
(R19b). Declared in the world file:

    [step.homes]
    when    = "lint"
    command = ["python", "-m", "viasdk", "tools/homes.py"]
    params  = { assets = ["house_low", "house_wide"], road_kinds = ["road"], max_m = 30 }

For each instance of an asset in ``assets``, it measures the distance in plan from the instance's
point to the nearest road (a feature of a kind in ``road_kinds``), along each road's segments. A
home farther than ``max_m`` metres is one warning, ``step.homes.far_from_road``, at the home, with
the distance and the road in ``data``; every home also gets one info, ``step.homes.road_reach``,
with its distance, so the measure is read beside the eye (R15) and not only when it fails.

Imports only viasdk and the standard library: the rows are a few dozen, so plain Python is enough.
"""

import math

EXPLAIN = {
    "road_reach": "Metres in plan from a home's point to the nearest point of any road "
    "(a feature of a kind in the step's `road_kinds`). Measured for every instance of an asset in "
    "the step's `assets`.",
    "far_from_road": "A home stands farther from every road than this project allows (`max_m`). "
    "The project's rule, not the engine's: a home is reached on foot from a road.",
}


def lint(step):
    p = step.params
    assets = set(p.get("assets", []))
    kinds = set(p.get("road_kinds", ["road"]))
    max_m = float(p.get("max_m", 30))
    roads = _paths(step.features("*"), kinds)
    t = step.instances("*")
    cols = {k: _list(t[k]) for k in ("name", "asset", "x", "y")}
    explained = set()
    for r in range(len(cols["name"])):
        if cols["asset"][r] not in assets:
            continue
        name = cols["name"][r]
        here = (int(cols["x"][r]), int(cols["y"][r]))
        near = None
        for road, line in roads:
            d = min(_seg(here, a, b) for a, b in zip(line, line[1:]))
            if near is None or d < near[0]:
                near = (d, road)
        pos = [here[0] / 1000.0, here[1] / 1000.0]
        if near is None:
            dist, road = None, None
        else:
            dist, road = round(near[0] / 1000.0, 1), near[1]
        _say(step, explained, "info", "road_reach", name, pos,
             f"{name} stands {dist} m from the nearest road ({road})." if road else
             f"{name}: this world has no road of a kind in {sorted(kinds)}.",
             "Move it nearer a road, or bring a road to it.",
             _data(value=dist, road=road))
        if dist is None or dist > max_m:
            _say(step, explained, "warning", "far_from_road", name, pos,
                 f"{name} is {dist} m from the nearest road ({road}); this project allows {max_m:g} m."
                 if road else f"{name} has no road to reach: none of a kind in {sorted(kinds)}.",
                 "Move it nearer a road, or bring a road to it.",
                 _data(value=dist, road=road, max_m=max_m))


def _data(**kw):
    """A diagnostic's data with the absent values left out: data holds no null (protocol.md 3.3)."""
    return {k: v for k, v in kw.items() if v is not None}


def _say(step, explained, severity, code, name, pos, message, hint, data):
    kw = {"position": pos, "data": data}
    if code not in explained:
        kw["explain"] = EXPLAIN[code]
        explained.add(code)
    getattr(step, severity)(code, "instance." + name, message, hint, **kw)


def _paths(fl, kinds):
    """(name, [(x, y) mm, ...]) of every path feature of a kind in ``kinds``, from the feature
    list's columns (protocol.md 4.3), read the same way by a worker and a plain command."""
    names = _list(fl["name"])
    kind = _list(fl["kind"])
    geometry = _list(fl["geometry"])
    pieces = _list(fl["pieces"])
    rings = _list(fl["rings"])
    starts = _list(fl["ring_starts"])
    coords = [(int(c[0]), int(c[1])) for c in _list(fl["coords"])]
    out = []
    for f, name in enumerate(names):
        if kind[f] not in kinds or int(geometry[f]) != 1:
            continue
        r = rings[pieces[f]]
        line = coords[starts[r]:starts[r + 1]]
        if len(line) >= 2:
            out.append((name, line))
    return out


def _seg(p, a, b):
    """Distance in millimetres from point p to segment ab."""
    ex, ey = b[0] - a[0], b[1] - a[1]
    px, py = p[0] - a[0], p[1] - a[1]
    ee = ex * ex + ey * ey
    t = 0.0 if ee == 0 else max(0.0, min(1.0, (px * ex + py * ey) / ee))
    return math.hypot(px - t * ex, py - t * ey)


def _list(col):
    return col.tolist() if hasattr(col, "tolist") else list(col)
