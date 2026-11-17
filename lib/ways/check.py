"""The ways library's crossing lint: does water pass beneath each crossing's deck, does a way
meet each end of it level, and is every way's end beside another path's end joined to it?

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

Only the world's own crossings are judged under 1 to 3: an import's are judged by its own build,
where its lints run, and its `deck_z` is in its own frame, not lifted (composition.md 5).

4. **Ends joined.** A way's end (a `lane`, `road`, `trail` or `crossing` path's first or last
   point) within `reach` (1 m) in plan of an end of a path of another of those kinds, with no
   meeting between the two, is `end_unjoined` (warning), with the plan distance and the height gap
   (each end's height: a crossing's `deck_z`, a way's the surface there): a bridgehead the way was
   meant to name and did not (`ends_at = "bridge.start"`), so the two meet at a seam no one owns.
   Ours (M7 close, asked for by an agent's on-foot review); two paths of one kind ending together
   are not judged, a fork being written by name between them anyway (geometry.md 2.7). Two ends
   that both meet one named point are joined through it (an import's exit and the parent's road
   that answers it, composition.md 7), except at a crossing, whose end only a way naming it meets
   (3). A pair of two imports' ends is the imports' own; at least one end must be the world's. An
   import's crossing's height is not said (its `deck_z` is in its own frame).

`params`: `wet` (0.01 m), `reach` (1 m). Codes are `step.<id>.<code>`; `checked` (info) counts
what was judged.

Library code imports only viasdk and the standard library (R19b).
"""

import json
import math

from viasdk import RpcError

DEFAULTS = {"wet": 0.01, "reach": 1.0}
PATH_KINDS = ("lane", "road", "trail", "crossing")

EXPLAIN = {
    "params": "A param of the crossing lint is outside what it takes: `wet` or `reach`, metres above 0.",
    "dry": "No water passes beneath the crossing's middle: it spans dry ground, or the ground under "
    "its deck stands at the water's level. Move the crossing onto the water it `crosses`.",
    "low_deck": "The deck stands less than the crossing's `freeboard` over the water under its middle. "
    "Raise `deck_z` or lower `freeboard`.",
    "deck_unmet": "No way arrives at this end of the crossing: write a way whose `starts_at` or "
    "`ends_at` names it.",
    "deck_gap": "A way meets this end of the crossing at a different height from the deck.",
    "end_unjoined": "A way ends within a metre in plan of another kind of path's end, and no meeting "
    "joins them: name the other end in the way's `starts_at` or `ends_at`, so the two meet level.",
    "checked": "What the crossing lint judged: crossings and their ends (`crossings`, `ends`), and the path ends `end_unjoined` read (`path_ends`).",
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
                    # A deck under the water says so, not "-0.082 m over" (M7 close, a fresh agent).
                    stands = (f"stands {deck - water:.3f} m over the water" if deck >= water else
                              f"stands {water - deck:.3f} m under the water, which covers it")
                    out.append(("warning", "low_deck", name,
                                f"`{name}`'s deck {stands}, under its "
                                f"freeboard of {float(c['freeboard']):g} m",
                                f"raise `deck_z` to {water + float(c['freeboard']):.2f} or more",
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


def own(place):
    """The world's own: a name with no import's scope (`townland.old_bridge` is townland's); a
    place's `.start` or `.end` is its feature's."""
    owner, _, part = place.rpartition(".")
    return "." not in (owner if part in ("start", "end") else place)


def subject_of(subject, step_id):
    """A record's subject: the feature a crossing's name or a path's end names, scope and all."""
    if not subject:
        return "step." + step_id
    owner, _, part = subject.rpartition(".")
    return "feature." + (owner if part in ("start", "end") and owner else subject)


def unjoined(ends, meetings, p):
    """`end_unjoined` records (judge's shape) for each pair of ends of paths of different kinds
    within `reach` in plan with no meeting between them, in name order. `ends` holds
    `{"place", "kind", "at" (mm), "z" (metres or None)}`."""
    met = {frozenset((m["a"], m["b"])) for m in meetings}
    touches = {}
    for m in meetings:
        touches.setdefault(m["a"], set()).add(m["b"])
        touches.setdefault(m["b"], set()).add(m["a"])
    out = []
    ends = sorted(ends, key=lambda e: e["place"])
    for i, a in enumerate(ends):
        for b in ends[i + 1:]:
            if a["kind"] == b["kind"] or frozenset((a["place"], b["place"])) in met:
                continue
            if not (own(a["place"]) or own(b["place"])):
                continue
            if "crossing" not in (a["kind"], b["kind"]) and touches.get(a["place"], set()) & touches.get(b["place"], set()):
                continue
            d = math.dist(a["at"], b["at"]) / 1000
            if d > p["reach"]:
                continue
            way, other = (b, a) if a["kind"] == "crossing" else (a, b)
            gap = None if a["z"] is None or b["z"] is None else round(abs(a["z"] - b["z"]), 3)
            said = f", {gap:.2f} m apart in height" if gap is not None else ""
            name = other["place"].rpartition(".")[0]
            to = "ends_at" if way["place"].endswith(".end") else "starts_at"
            out.append(("warning", "end_unjoined", way["place"],
                        f"`{way['place']}` ends {d:.2f} m from `{other['place']}`{said}, and no meeting joins them",
                        f"write `{to} = \"{other['place']}\"` on `{way['place'].rpartition('.')[0]}`"
                        if other["kind"] == "crossing" else f"join `{way['place']}` to `{name}` by name",
                        {"distance": round(d, 3), "gap": gap, "other": other["place"]},
                        (way["at"][0] / 1000, way["at"][1] / 1000)))
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


def read_ends(step):
    """Every end of a path of PATH_KINDS: its place, kind, plan point (mm) and height."""
    ends = []
    for kind in PATH_KINDS:
        try:
            fl = step.features(select="kind:" + kind, columns=["deck_z"] if kind == "crossing" else [])
        except (LookupError, RpcError):           # no feature of that kind
            continue
        for f, name in enumerate(_list(fl.get("name", []))):
            path = [(int(v[0]), int(v[1])) for v in _list(fl.rings(f)[0][0])]
            deck = _num(_list(fl["deck_z"])[f]) if kind == "crossing" and own(name) else None
            for end, pt in (("start", path[0]), ("end", path[-1])):
                ends.append({"place": f"{name}.{end}", "kind": kind, "at": pt, "z": deck})
    ways = [e for e in ends if e["kind"] != "crossing"]
    if ways:
        try:
            (f,) = step.fields(["role:surface"], window=[0, 0, 1, 1])
            zs = _list(step.eval(f"= f('{f.name}')", x=[e["at"][0] for e in ways], y=[e["at"][1] for e in ways]))
            for e, z in zip(ways, zs):
                e["z"] = float(z)
        except (LookupError, ValueError, RpcError):  # no surface field: the gap is not said
            pass
    return ends


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
    crossings = [c for c in read_crossings(step) if own(c["name"])]
    meetings = read_meetings(step)
    recs = judge(crossings, meetings, read_samples(step, crossings), p)
    sev, code, subject, msg, hint, data, pos = recs.pop()   # judge's `checked`, said last
    assert code == "checked"
    ends = read_ends(step)
    recs += unjoined(ends, meetings, p)
    recs.append((sev, code, subject, f"{msg}, {len(ends)} path ends", hint,
                 {**data, "path_ends": len(ends)}, pos))
    said = set()
    for sev, code, subject, msg, hint, data, pos in recs:
        kw = {"data": data}
        if pos is not None:
            kw["position"] = pos
        if code not in said:
            kw["explain"] = EXPLAIN[code]
            said.add(code)
        subj = subject_of(subject, step.id)
        getattr(step, sev)(code, subj, msg, hint, **kw)
