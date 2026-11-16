"""The ways library's crossing lint: does water pass beneath each crossing's deck, and does a way
meet each end of it level?

A lint step (`when = "lint"`), run through the SDK; it reads the finished build (R29):
`build.features` of kind `crossing`, `surface.meetings` (surfaces.md 2.4) and the fields under each
deck's middle (`expr.eval`). It changes nothing.

What it judges (ours, written for this library; kinds.toml's `crossing` is what it checks):

1. **Water beneath.** At the deck's middle (the point at half its path's arc length) it samples the
   terrain (the world's `role:surface` field) and the water (the first field tagged `water_level`).
   No water there, or water no deeper than `wet` (0.01 m) over the ground, is `dry` (warning): the
   crossing spans nothing, or the ground under it was raised to the deck (the M3 stand-in did).
2. **Freeboard.** The deck's line (`deck_z`) less the water there under the crossing's
   `freeboard` is `low_deck` (warning), with both heights: the water would wet the deck.
3. **Ends met.** Each end of the crossing's path (`<crossing>.start`, `.end`) lies on its deck
   (surfaces.md 3.3). An end that no meeting joins to another path's end (a way's `starts_at` or
   `ends_at`; the point the crossing's own end is written at does not count) is `deck_unmet` (warning): a deck to nowhere,
   which the core does not report (surfaces.md 3.4: `surface.unmet` never reads a path end). A
   meeting there whose `gap` is not 0 is `deck_gap` (warning), with the gap: the core's
   `surface.gap` says the same as an error, so this is seen only where that error was let pass.

`params`: `wet` (0.01 m). Codes are `step.<id>.<code>`; `checked` (info) counts what was judged.

Library code imports only viasdk and the standard library (R19b).
"""

import json
import math

DEFAULTS = {"wet": 0.01}

EXPLAIN = {
    "params": "A param of the crossing lint is outside what it takes: `wet`, metres above 0.",
    "dry": "No water passes beneath the crossing's middle: it spans dry ground, or the ground under "
    "its deck stands at the water's level. Move the crossing onto the water it `crosses`.",
    "low_deck": "The deck stands less than the crossing's `freeboard` over the water under its middle. "
    "Raise `deck_z` or lower `freeboard`.",
    "deck_unmet": "No way arrives at this end of the crossing: write a way whose `starts_at` or "
    "`ends_at` names it.",
    "deck_gap": "A way meets this end of the crossing at a different height from the deck.",
    "checked": "What the crossing lint judged: crossings, their ends and meetings.",
}


class ParamError(ValueError):
    pass


def params_of(raw):
    p = dict(DEFAULTS)
    for k, v in (raw or {}).items():
        if k not in DEFAULTS:
            raise ParamError(f"`{k}` is no param of the crossing lint")
        try:
            p[k] = float(v)
        except (TypeError, ValueError):
            raise ParamError(f"`{k}` must be a number")
        if not p[k] > 0:
            raise ParamError(f"`{k}` must be above 0")
    return p


def middle(path):
    """The point (mm, rounded to the lattice) at half the path's arc length."""
    segs = list(zip(path, path[1:]))
    total = sum(math.dist(a, b) for a, b in segs)
    s, c = total / 2, 0.0
    for a, b in segs:
        L = math.dist(a, b)
        if s <= c + L:
            u = (s - c) / L if L else 0.0
            return (int(round(a[0] + (b[0] - a[0]) * u)), int(round(a[1] + (b[1] - a[1]) * u)))
        c += L
    return tuple(path[-1])


def other_of(m, place):
    return m["b"] if m["a"] == place else m["a"]


def is_way_end(place, crossing):
    """A path end of another feature (`<way>.start`, `<way>.end`): the crossing's own ends also
    meet the points they are written at (`starts_at = "bridge_west"`), which are no way."""
    owner, _, part = place.rpartition(".")
    return part in ("start", "end") and owner != crossing and owner != ""


def judge(crossings, meetings, samples, p):
    """Records `(severity, code, subject, message, hint, data, position)`: per crossing in name
    order, water then freeboard then its two ends; then `checked`. `samples` maps a crossing's
    name to `(ground, water or None)` in metres at its middle."""
    out = []
    ends = 0
    for c in sorted(crossings, key=lambda c: c["name"]):
        name, mid = c["name"], middle(c["path"])
        pos = (mid[0] / 1000, mid[1] / 1000)
        ground, water = samples.get(name, (None, None))
        if ground is not None:
            if water is None or water - ground <= p["wet"]:
                out.append(("warning", "dry", name, f"no water passes beneath `{name}`'s middle",
                            "move the crossing onto the water it crosses",
                            {"ground": round(ground, 3), "water": None if water is None else round(water, 3)}, pos))
            else:
                deck = float(c["deck_z"])
                if deck - water < float(c["freeboard"]):
                    out.append(("warning", "low_deck", name,
                                f"`{name}`'s deck stands {deck - water:.3f} m over the water, under its "
                                f"freeboard of {float(c['freeboard']):g} m", "raise `deck_z`",
                                {"deck": deck, "water": round(water, 3), "freeboard": float(c["freeboard"])}, pos))
        for end, pt in (("start", c["path"][0]), ("end", c["path"][-1])):
            place = f"{name}.{end}"
            rows = [m for m in meetings if place in (m["a"], m["b"]) and is_way_end(other_of(m, place), name)]
            ends += 1
            epos = (pt[0] / 1000, pt[1] / 1000)
            if not rows:
                out.append(("warning", "deck_unmet", place, f"no way arrives at `{place}`",
                            f"write a way that {'ends_at' if end == 'start' else 'starts_at'} `{place}`",
                            {"end": end}, epos))
            for m in rows:
                if m["gap"] != 0:
                    other = other_of(m, place)
                    out.append(("warning", "deck_gap", place, f"`{other}` meets `{place}` {m['gap']} mm off "
                                "the deck", "bring the way's end to `deck_z`", {"gap": m["gap"], "way": other}, epos))
    out.append(("info", "checked", None, f"{len(crossings)} crossings, {ends} ends", "none needed",
                {"crossings": len(crossings), "ends": ends}, None))
    return out


# --- the step ---------------------------------------------------------------------------------


def _list(col):
    return col.tolist() if hasattr(col, "tolist") else list(col)


def _num(v):
    if isinstance(v, str):
        v = json.loads(v)
    return float(v)


def read_crossings(step):
    try:
        fl = step.features(select="kind:crossing", columns=["deck_z", "freeboard"])
    except Exception:                             # no crossing in the world: nothing to judge
        return []
    out = []
    for f, name in enumerate(_list(fl.get("name", []))):
        path = [(int(v[0]), int(v[1])) for v in _list(fl.rings(f)[0][0])]
        out.append({"name": name, "path": path, "deck_z": _num(_list(fl["deck_z"])[f]),
                    "freeboard": _num(_list(fl["freeboard"])[f])})
    return out


def read_samples(step, crossings):
    if not crossings:
        return {}
    mids = [middle(c["path"]) for c in crossings]
    xs, ys = [m[0] for m in mids], [m[1] for m in mids]

    def at(selector):
        try:
            (f,) = step.fields([selector], window=[0, 0, 1, 1])
        except Exception:                         # no such field in the world
            return None
        return [float(v) for v in _list(step.eval(f"= f('{f.name}')", x=xs, y=ys))]
    ground, water = at("role:surface"), at("tag:water_level")
    if ground is None:
        return {}
    return {c["name"]: (ground[i], water[i] if water is not None else None) for i, c in enumerate(crossings)}


def read_meetings(step):
    t = step.meetings()
    n = len(_list(t.get("a", [])))
    return [{"a": _list(t["a"])[r], "b": _list(t["b"])[r], "gap": int(_list(t["gap"])[r])} for r in range(n)]


def lint(step):
    try:
        p = params_of(step.params)
    except ParamError as e:
        step.error("params", "step." + step.id, str(e), "see lib/ways/check.py", explain=EXPLAIN["params"])
        return
    crossings = read_crossings(step)
    said = set()
    for sev, code, subject, msg, hint, data, pos in judge(crossings, read_meetings(step),
                                                         read_samples(step, crossings), p):
        kw = {"data": data}
        if pos is not None:
            kw["position"] = pos
        if code not in said:
            kw["explain"] = EXPLAIN[code]
            said.add(code)
        subj = "feature." + subject.split(".")[0] if subject else "step." + step.id
        getattr(step, sev)(code, subj, msg, hint, **kw)
