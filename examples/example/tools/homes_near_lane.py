"""This project's own lint step: every home stands within reach of a lane.

A lint reads the finished build and only reports (R22). Declared in layers/dress.toml:

    [step.homes]
    when    = "lint"
    command = ["python", "-m", "viasdk", "tools/homes_near_lane.py"]
    params  = { assets = ["house_low", "house_wide", "house_row"], ways = ["lane"], max_m = 30 }

For each instance of an asset in `assets` (by hand, or a run's), the plan distance from its point
to the nearest way (a path feature of a kind in `ways`). Every home gets one info,
`step.homes.lane_reach`, with its distance, so the measure is read beside the eye (R15); one
farther than `max_m` also gets a warning, `step.homes.far_from_lane`. A plain command: it imports
only viasdk and the standard library.
"""

import math

EXPLAIN = {
    "lane_reach": "Metres in plan from a home's point to the nearest point of any way of a kind in "
    "the step's `ways`.",
    "far_from_lane": "A home stands farther from every way than this project allows (`max_m`): "
    "the project's rule, not the engine's.",
}


def lint(step):
    p = step.params
    assets = set(p.get("assets", []))
    kinds = set(p.get("ways", ["lane"]))
    max_m = float(p.get("max_m", 30))
    ways = _paths(step.features("*"), kinds)
    t = step.instances("*")
    names, kept, xs, ys = (_list(t[k]) for k in ("name", "asset", "x", "y"))
    told = set()
    for r, name in enumerate(names):
        if kept[r] not in assets:
            continue
        here = (int(xs[r]), int(ys[r]))
        best = min(((min(_seg(here, a, b) for a, b in zip(line, line[1:])), way)
                    for way, line in ways), default=None)
        pos = [here[0] / 1000.0, here[1] / 1000.0]
        if best is None:
            _say(step, told, "warning", "far_from_lane", name, pos,
                 f"{name} has no way to reach: none of a kind in {sorted(kinds)}.",
                 "Bring a way to it.", {"max_m": max_m})
            continue
        dist, way = round(best[0] / 1000.0, 1), best[1]
        _say(step, told, "info", "lane_reach", name, pos,
             f"{name} stands {dist} m from the nearest way ({way}).",
             "Move it nearer a way, or bring a way to it.", {"value": dist, "way": way})
        if dist > max_m:
            _say(step, told, "warning", "far_from_lane", name, pos,
                 f"{name} is {dist} m from the nearest way ({way}); this project allows {max_m:g} m.",
                 "Move it nearer a way, or bring a way to it.",
                 {"value": dist, "way": way, "max_m": max_m})


def _say(step, told, severity, code, name, pos, message, hint, data):
    kw = {"position": pos, "data": data}
    if code not in told:
        kw["explain"] = EXPLAIN[code]
        told.add(code)
    getattr(step, severity)(code, "instance." + name, message, hint, **kw)


def _paths(fl, kinds):
    """(name, [(x, y) mm, ...]) of every path feature of a kind in `kinds` (protocol.md 4.3)."""
    names, kind, geometry = _list(fl["name"]), _list(fl["kind"]), _list(fl["geometry"])
    pieces, rings, starts = _list(fl["pieces"]), _list(fl["rings"]), _list(fl["ring_starts"])
    coords = [(int(c[0]), int(c[1])) for c in _list(fl["coords"])]
    out = []
    for f, name in enumerate(names):
        if kind[f] in kinds and int(geometry[f]) == 1:
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
