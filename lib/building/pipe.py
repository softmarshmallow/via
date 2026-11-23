"""The building library's pipes step: one `pipe` feature becomes level legs, each a path that owns
the surface its pieces stand on, with the kit's straight pieces along it, an elbow at each turn,
a riser at each change of height, a flange or a cap on each end and supports under it.

A generate step, `for_each = "kind:pipe"`, run as a worker through the SDK (kinds.toml shows the
table a world restates): one call for each pipe, so a pipe that is refused takes no other with
it (a step that says an error returns nothing). Run with no `for_each` it lays every pipe in one
call. It reads the pipe (`build.features`), its kit (`pack.query`) and the ground under its
first point and its supports (`expr.eval` of the terrain). Everything it returns is keyed by the
pipe's name, in one document, because a leg names its riser and a piece its leg (R3).

A pipe is the glue between two kits (docs/research/Modular kit failures in studios.md, sections 3
and 6: the pipe kit as the model of a kit, and pipes kept as glue when the kit they came with was
not). The rules are ours: the research found no account of pipes joined freely in three
dimensions (the same report, section 12). README.md, "Pipes", is the design note.

The rules.

1. **A pipe** is a point, `at`, where it starts; `to`, the points it goes on through, in order;
   and `heights`, one entry for each of those points, the first included: metres above the
   ground at its first point (the terrain there, read once, to the millimetre). It is not
   written as a path: its legs are paths, and a path drawn over them would cross each one
   (spec/geometry.md 2.7), as a row's frontage would its doorways (row.py). An
   entry is a number, or two, `[arriving, leaving]`: the pipe comes to that point at one height
   and leaves it at another. A pipe written by heights above one point stays level over ground
   that is not; nothing is fitted to the land. Every point lies inside the world's extent, or
   the pipe is refused (`params`): its legs and pieces stand there, and the ground is read there.
2. **A leg** is the pipe between two points. It is level: its two ends are at one height, or the
   pipe is refused (`params`) with the reason, since a kit of straight pieces has none that
   climbs; the hint writes the riser at whichever end of the leg can hold one (a point between
   two legs in one line), or says a point is wanted. Each leg is returned as a `pipe_leg`, `<pipe>_l<i>`: a path from one point to the
   next that owns one surface, `line`, at the leg's height, as a flight owns its `tread`
   (spec/surfaces.md 6.4). Every piece of the leg stands on that surface, so a pipe piece's
   model is made about its middle line and its pack says `pivot = "origin"`. A leg that follows
   another at one height starts at the other's end (`starts_at = "<leg>.end"`), so the two own
   their meeting; two legs either side of a riser lie one over the other at their common point
   and do not meet (R71).
3. **What stands at a point.** At a point where the pipe turns, an elbow (`role:elbow`, its
   `params.angle` the turn it makes, in degrees): `<pipe>_e<i>`, on the surface of the leg that
   arrives. The piece has one arm toward its back and one `angle` degrees to the right of its
   front: for a right turn its back arm lies along the leg that arrives, for a left turn the
   piece stands the other way round, its back arm along the leg that leaves, so one piece makes
   its turn to either hand, whatever its angle. A turn the kit has no elbow for is refused,
   naming the angles it has. At a point with two heights, a riser
   (`role:riser`, its `params.rise` the height it climbs): `<pipe>_r<i>`, on the surface of the
   lower of the two legs, at their common point. The riser's pack gives it a mount at its
   origin, `top`, with a `rise` (spec/assets.md 2.7, R75) and the connector `pipe`, a leg's own
   word: the upper leg starts (or ends) at `<riser>.top`, so the engine holds that leg's height
   against the lower leg's and the piece's own rise (`surface.gap`). The lower leg names
   nothing: the riser stands on it, and a leg that ended on what stands on it would be a loop
   (`graph.cycle`). A height the kit's riser does not make, to the millimetre, is
   refused, naming what it makes (the engine holds the upper leg to the riser's top exactly, so
   a climb a millimetre off would pass here only to be `surface.gap` there); so is a riser where the pipe also turns, and a riser on the
   pipe's first or last point. A point where the pipe neither turns nor climbs holds no piece.
4. **The ends.** `ends = [first, last]`, each `"flange"` or `"cap"`; a flange where none is
   written. A flange (`role:flange`) is the plug a port takes: `<pipe>_end0`, `<pipe>_end1`, on
   the end leg's surface at the pipe's end, its front along the pipe and out of it. Its pack
   gives it a mount with a connector there. Where a port of the same connector stands at that
   point, at that height, facing it, the engine finds the joint (spec/surfaces.md 3.3, R76); where
   none does, the engine's `surface.unmet` names the nearest and the miss in plan and in height.
   **Nothing in this step pairs them**: it does not read the buildings, and a pipe that misses
   its port is told so by the core. A cap (`role:cap`) closes an end that joins nothing. So a
   pipe has no loose end: each is a plug or a cap.
5. **The fit.** Each piece at a point takes `params.reach` of each leg it stands on. What is left
   of a leg holds one run of straight pieces (`role:pipe`, at their `params.pitch`),
   `<pipe>_l<i>_s`, and one of slip pieces (`role:slip`, the kit's filler), `<pipe>_l<i>_f`,
   after them: the most straight pieces such that the rest is a whole number of slip pieces, to
   `FIT` (5 mm) long and `FIT_SHORT` (1.5 mm) short, as a wall's stretch closes
   (lay.py rule 6). A leg that does not close, or is shorter than what its two ends take, is
   `pipe_open` (warning) with the lengths that would close, and holds no straight piece: its end
   pieces stand, and the gap shows. A leg shorter than its two ends take closes at what they
   take, and that one length is named.
6. **Supports.** Under a leg whose height is the height the kit's support is made for, to the
   millimetre (`role:support`, `params.height`, which a support must say), supports: as many as keep them no further apart than the
   kit's `shared.support`, evenly along what the leg's straight pieces cover;
   `<pipe>_l<i>_p<k>`. A support is a pipe piece: it stands on the leg's surface, as the leg's
   other pieces do, and its model reaches down from the pipe's middle line by the height it is
   made for. (Stood on the terrain it would be on one level with the pipe over it, by the
   core's rule for a surface whose bound meets a thing on the ground, and each would be
   `place.crowded` with the other: spec/rules.md 8.1, 3.) A leg at another height has none, and
   the pipe says how many such legs it has (`support_none`, info). A support is as long as it
   is: where the ground under one differs from the ground under the pipe's first point by more
   than 0.05 m its foot is in the ground or in the air, and the pipe says by how much
   (`support_off`, warning).

Diagnostics: `params` (error; the pipe is not returned), `pipe_open`, `support_off` (warnings),
`support_none` and `pipes` (infos; `pipes` is what was returned).
"""

import json
import math

from viasdk import RpcError

FIT = 5              # mm a leg may be longer than its pieces and still close
FIT_SHORT = 1.5      # mm it may be shorter (lay.py `fit`)
ANGLE = 0.05         # degrees a turn may differ from an elbow's angle
SEEN = 0.05          # metres of ground under a support that is said (rule 6)
ATTRS = ["to", "heights", "kit", "ends"]
ENDS = ("flange", "cap")

EXPLAIN = {
    "params": "A pipe is written as a path, a height for each of its points, and a kit. This one cannot be "
              "laid as written: the message says which point or leg and why.",
    "pipe_open": "A leg of a pipe holds straight pieces and slip pieces between the pieces at its two ends. "
                 "This leg is no whole number of them, so none is laid on it and the gap shows.",
    "support_off": "A support is a piece of one height, made for a pipe at one height above the ground. The "
                   "ground under this one is not the ground the pipe's heights are measured from.",
    "support_none": "A kit's support is made for a pipe at one height. A leg at another height has none.",
    "pipes": "What the pipes step returned.",
}


class ParamError(Exception):
    """A pipe that cannot be laid as written. `data` holds the figures the message names (the
    point or leg, the turn in degrees, the climb in metres), so a record can be told from
    another of its code without reading its sentence."""

    def __init__(self, key, message, hint, pipe=None, data=None):
        super().__init__(message)
        self.key, self.message, self.hint, self.pipe, self.data = key, message, hint, pipe, data or {}


def bearing(v):
    """Degrees clockwise from north of a plan vector, to the millidegree, in [0, 360)."""
    b = round(math.degrees(math.atan2(v[0], v[1])) % 360.0, 3)
    return 0.0 if b >= 360.0 else b


def turn(a, b):
    """The turn from bearing `a` to bearing `b`, degrees in (-180, 180]: right is positive."""
    d = (b - a) % 360.0
    return d - 360.0 if d > 180.0 else d


def fit(length, pitch, slip):
    """The most straight pieces `n` and slip pieces `k` that close `length` (mm), or None and
    the nearest lengths below and above that would (lay.py `fit`, the same tolerances)."""
    for n in range(int((length + FIT_SHORT) // pitch), -1, -1):
        rest = length - n * pitch
        k = int(round(rest / slip)) if slip else 0
        if -FIT_SHORT <= rest - k * (slip or 0) <= FIT:
            return (n, k), None
    closes = sorted({n * pitch + k * (slip or 0) for n in range(int(length // pitch) + 2)
                     for k in (range(0, int(length // slip) + 2) if slip else (0,))})
    return None, (max((c for c in closes if c < length), default=None),
                  min((c for c in closes if c > length), default=None))


def _mm(v):
    return int(round(float(v) * 1000))


def kit_facts(rows, taxon):
    """What the step lays a pipe from: {pipe, slip, elbows, risers, flange, cap, support,
    spacing}, lengths in mm, from pack rows (id, tags, bounds, params, shared). One piece a role
    but for elbows and risers, of which a kit may hold several: the first id bytewise."""
    def number(r, key):
        v = (r.get("params") or {}).get(key)
        if v is None:
            return None
        if isinstance(v, bool) or not isinstance(v, (int, float)) or not math.isfinite(v) or v <= 0:
            raise ParamError("kit", f"`{r['id']}` of `{taxon}` says `{key} = {v!r}`", f"`params.{key}` is a positive number")
        return v

    def piece(r):
        b = r.get("bounds")
        out = {"id": r["id"]}
        pitch = number(r, "pitch")
        out["pitch"] = _mm(pitch) if pitch is not None else (b[3] - b[0] if b else 0)
        for key in ("reach", "rise", "height"):
            v = number(r, key)
            out[key] = _mm(v) if v is not None else (0 if key == "reach" else None)
        out["angle"] = number(r, "angle")
        return out
    facts = {"taxon": taxon, "pipe": None, "slip": None, "elbows": [], "risers": [], "flange": None, "cap": None,
             "support": None, "spacing": None}
    if not rows:
        raise ParamError("kit", f"no piece of `{taxon}` is in the pack", "name the pipe kit's taxon in `kit`")
    for r in sorted(rows, key=lambda r: r["id"]):
        for tag in r.get("tags") or []:
            role = tag[5:] if tag.startswith("role:") else None
            if role == "elbow":
                facts["elbows"].append(piece(r))
            elif role == "riser":
                facts["risers"].append(piece(r))
            elif role in ("pipe", "slip", "flange", "cap", "support") and facts[role] is None:
                facts[role] = piece(r)
    if facts["pipe"] is None or facts["pipe"]["pitch"] <= 0:
        raise ParamError("kit", f"`{taxon}` has no `role:pipe` piece with a length",
                         "a pipe kit's straight piece is tagged `role:pipe` and says `params.pitch`")
    for e in facts["elbows"]:
        if e["angle"] is None:
            raise ParamError("kit", f"the elbow `{e['id']}` says no `params.angle`", "an elbow says the turn it makes, in degrees")
    for r in facts["risers"]:
        if r["rise"] is None:
            raise ParamError("kit", f"the riser `{r['id']}` says no `params.rise`", "a riser says the height it climbs, in metres")
    if facts["support"] is not None and facts["support"]["height"] is None:
        raise ParamError("kit", f"the support `{facts['support']['id']}` says no `params.height`",
                         "a support says the height of the pipe it stands under, in metres")
    shared = [r["shared"] for r in rows if r.get("shared")]
    spacing = (shared[0] if shared else {}).get("support")
    if spacing is not None:
        if isinstance(spacing, bool) or not isinstance(spacing, (int, float)) or not math.isfinite(spacing) or spacing <= 0:
            raise ParamError("kit", f"`{taxon}` states `shared.support = {spacing!r}`",
                             "`shared.support` is the most metres between two supports")
        facts["spacing"] = _mm(spacing)
    return facts


def points_of(to):
    """The pipe's points after its first, in mm, from `to` (rule 1)."""
    if not isinstance(to, list) or not to or any(
            not isinstance(p, list) or len(p) != 2
            or any(isinstance(v, bool) or not isinstance(v, (int, float)) or not math.isfinite(v) for v in p) for p in to):
        raise ParamError("to", f"`to` is {to!r}", "write the points the pipe goes on through: `to = [[x, y], ...]`, in metres")
    return [(_mm(p[0]), _mm(p[1])) for p in to]


def heights_of(heights, n):
    """Each point's (arriving, leaving) height in mm, from `heights` (rule 1)."""
    if not isinstance(heights, list) or len(heights) != n:
        got = len(heights) if isinstance(heights, list) else "none"
        raise ParamError("heights", f"`heights` has {got} entries for a pipe of {n} points",
                         "write one height for `at` and one for each point of `to`, in metres above the ground at `at`")
    out = []
    for i, h in enumerate(heights):
        pair = h if isinstance(h, list) else [h, h]
        if len(pair) != 2 or any(isinstance(v, bool) or not isinstance(v, (int, float)) or not math.isfinite(v) for v in pair):
            raise ParamError("heights", f"`heights[{i}]` is {h!r}",
                             "a height is a number of metres, or two: `[arriving, leaving]` where the pipe climbs")
        out.append((_mm(pair[0]), _mm(pair[1])))
    return out


def level_hint(pts, hs, i, risers=()):
    """What to write where leg `i` is not level (rule 2): a riser at one of its two points, which
    must be a point between two legs where the pipe does not turn (rule 3). The far point first,
    so the leg keeps the height it starts at; then the near one; else a point is wanted. Where
    the climb is not one the kit's riser makes, it says what the riser makes."""
    n = len(pts)

    def straight(j):
        a, b, c = pts[j - 1], pts[j], pts[j + 1]
        return abs(turn(bearing((b[0] - a[0], b[1] - a[1])), bearing((c[0] - b[0], c[1] - b[1])))) <= ANGLE
    lo, hi = hs[i][1], hs[i + 1][0]
    makes = "" if any(r["rise"] == abs(hi - lo) for r in risers) or not risers else (
        ". The kit's riser makes " + ", ".join(f"{r['rise'] / 1000:g} m" for r in risers) + ": the climb has to be that")
    if i + 1 < n - 1 and straight(i + 1):
        return (f"give both ends one height, and climb at a point: `heights[{i + 1}] = "
                f"[{lo / 1000:g}, {hs[i + 1][1] / 1000:g}]` is a riser there" + makes)
    if i > 0 and straight(i):
        return (f"give both ends one height, and climb at a point: `heights[{i}] = "
                f"[{hs[i][0] / 1000:g}, {hi / 1000:g}]` is a riser there" + makes)
    return ("give both ends one height. A riser stands at a point between two legs in one line, and this leg has "
            "none: add a point on the pipe's line and write its height as `[arriving, leaving]`" + makes)


def plan_pipe(name, pts, attrs, kit, ground=0.0):
    """One pipe's legs, runs, instances and notes (rules 1 to 6), or ParamError. `pts` are its
    points in mm, `ground` the terrain at the first, metres. Returns {legs: [{key, path, level,
    starts_at, ends_at}], runs: [{key, asset, count, ref, span, face}], instances: [{key, asset,
    at, face, surface}], notes, supports: [(key, (x, y))]}."""
    n = len(pts)
    if n < 2:
        raise ParamError("to", "a pipe goes from `at` through one point or more", "write `to = [[x, y], ...]`")
    hs = heights_of(attrs.get("heights"), n)
    ends = attrs.get("ends") or ["flange", "flange"]
    if not isinstance(ends, list) or len(ends) != 2 or any(e not in ENDS for e in ends):
        raise ParamError("ends", f"`ends` is {ends!r}", '`ends = [first, last]`, each "flange" or "cap"')
    for i, at in ((0, "first"), (n - 1, "last")):
        if hs[i][0] != hs[i][1]:
            raise ParamError("heights", f"`heights[{i}]` gives the pipe's {at} point two heights",
                             "a riser stands between two legs: start or end the pipe with a leg")
    legs = []
    for i in range(n - 1):
        a, b = pts[i], pts[i + 1]
        L = math.hypot(b[0] - a[0], b[1] - a[1])
        if L < 1:
            raise ParamError("to", f"points {i} and {i + 1} of the pipe are one place",
                             f"a riser is one point with two heights: `heights[{i}] = [arriving, leaving]`")
        if hs[i][1] != hs[i + 1][0]:
            raise ParamError("heights", f"leg {i}, from point {i} to point {i + 1}, climbs from {hs[i][1] / 1000:g} m to "
                             f"{hs[i + 1][0] / 1000:g} m: a leg is level", level_hint(pts, hs, i, kit["risers"]),
                             data={"leg": i, "from": hs[i][1] / 1000, "to": hs[i + 1][0] / 1000})
        legs.append({"i": i, "key": f"{name}_l{i}", "a": a, "b": b, "L": L, "h": hs[i][1],
                     "bearing": bearing((b[0] - a[0], b[1] - a[1])), "take": [0, 0], "starts_at": None, "ends_at": None})
    insts, notes = [], []

    def surface(leg):
        return f"{leg['key']}.line"

    def end_piece(word, leg, at, face, key, which):
        piece = kit.get(word)
        if piece is None:
            raise ParamError("kit", f"`{kit['taxon']}` has no `role:{word}` piece, which the pipe's {which} end takes",
                             f'add one to the kit, or write the other in `ends` ({" or ".join(ENDS)})')
        insts.append({"key": key, "asset": piece["id"], "at": at, "face": face, "surface": surface(leg)})
        return piece["reach"]
    legs[0]["take"][0] = end_piece(ends[0], legs[0], pts[0], round((legs[0]["bearing"] + 180.0) % 360.0, 3),
                                   f"{name}_end0", "first")
    legs[-1]["take"][1] = end_piece(ends[1], legs[-1], pts[-1], legs[-1]["bearing"], f"{name}_end1", "last")
    for i in range(1, n - 1):
        before, after = legs[i - 1], legs[i]
        t = turn(before["bearing"], after["bearing"])
        dh = hs[i][1] - hs[i][0]
        if dh and abs(t) > ANGLE:
            raise ParamError("to", f"the pipe climbs and turns at point {i}",
                             "a riser's two legs lie in one line: turn at another point, a leg away")
        if dh:
            # To the millimetre: the engine holds the upper leg to the riser's top exactly
            # (`surface.gap`), so a climb a millimetre off is refused here, with what closes.
            riser = next((r for r in kit["risers"] if r["rise"] == abs(dh)), None)
            if riser is None:
                makes = ", ".join(f"{r['rise'] / 1000:g} m" for r in kit["risers"]) or "no riser"
                raise ParamError("heights", f"the pipe climbs {abs(dh) / 1000:g} m at point {i}: `{kit['taxon']}` has "
                                 + (f"a riser of {makes}" if kit["risers"] else "no `role:riser` piece"),
                                 f"climb by what the kit's riser makes ({makes})" if kit["risers"]
                                 else "add a riser to the kit, or keep the pipe at one height",
                                 data={"point": i, "climb": abs(dh) / 1000})
            key = f"{name}_r{i}"
            up = dh > 0
            low, high = (before, after) if up else (after, before)
            face = before["bearing"] if up else round((before["bearing"] + 180.0) % 360.0, 3)
            insts.append({"key": key, "asset": riser["id"], "at": pts[i], "face": face, "surface": surface(low)})
            if up:                              # the upper leg names the riser's top; the lower names nothing (rule 3)
                after["starts_at"] = f"{key}.top"
            else:
                before["ends_at"] = f"{key}.top"
            before["take"][1] = after["take"][0] = riser["reach"]
        elif abs(t) > ANGLE:
            elbow = next((e for e in kit["elbows"] if abs(e["angle"] - abs(t)) <= ANGLE), None)
            if elbow is None:
                has = ", ".join(f"{e['angle']:g}" for e in kit["elbows"])
                raise ParamError("to", f"the pipe turns {abs(t):g} degrees at point {i}: `{kit['taxon']}` has "
                                 + (f"an elbow of {has} degrees" if kit["elbows"] else "no `role:elbow` piece"),
                                 f"turn by what the kit's elbow makes ({has} degrees)" if kit["elbows"]
                                 else "add an elbow to the kit, or keep the pipe straight",
                                 data={"point": i, "turn": round(abs(t), 3)})
            # One arm lies toward the piece's back and one `angle` degrees to the right of its
            # front. A right turn: the back arm along the leg that arrives. A left turn: the
            # piece the other way round, its back arm along the leg that leaves and its other
            # arm back along the leg that arrives (rule 3). For a quarter turn that is the
            # piece turned a quarter; for any other angle it is not, and was laid wrong.
            face = before["bearing"] if t > 0 else round((before["bearing"] + 180.0 - elbow["angle"]) % 360.0, 3)
            insts.append({"key": f"{name}_e{i}", "asset": elbow["id"], "at": pts[i], "face": face,
                          "surface": surface(before)})
            before["take"][1] = after["take"][0] = elbow["reach"]
            after["starts_at"] = f"{before['key']}.end"
        else:
            after["starts_at"] = f"{before['key']}.end"      # one line, one height: the legs own their meeting
    runs, supports, bare = [], [], 0
    slip = kit["slip"]["pitch"] if kit.get("slip") else 0
    pitch = kit["pipe"]["pitch"]
    for leg in legs:
        a, b = leg["take"]
        S = leg["L"] - a - b
        mid = (leg["a"][0] + (leg["b"][0] - leg["a"][0]) / 2, leg["a"][1] + (leg["b"][1] - leg["a"][1]) / 2)
        got, near = (None, (None, None)) if S < -FIT_SHORT else fit(S, pitch, slip)
        if S < -FIT_SHORT or got is None:
            lo, hi = near
            # The lengths that close, nearest below and above; a leg shorter than what its two
            # ends take closes at that length and no shorter.
            notes.append({"code": "pipe_open", "leg": leg["i"], "length": leg["L"], "takes": a + b,
                          "near": [v + a + b for v in (lo, hi) if v is not None] if S >= -FIT_SHORT else [a + b],
                          "at": mid})
            continue
        m, k = got
        face = round((leg["bearing"] - 90.0) % 360.0, 3)        # a straight piece lies along its own x
        s0, s1 = a, a + m * pitch
        if m:
            runs.append({"key": f"{leg['key']}_s", "asset": kit["pipe"]["id"], "count": m, "ref": leg["key"],
                         "span": [round(s0 / leg["L"], 9), round(s1 / leg["L"], 9)], "face": face})
        if k:
            runs.append({"key": f"{leg['key']}_f", "asset": kit["slip"]["id"], "count": k, "ref": leg["key"],
                         "span": [round(s1 / leg["L"], 9), round((leg["L"] - b) / leg["L"], 9)], "face": face})
        sup = kit.get("support")
        if sup and sup["height"] == leg["h"] and S > FIT:       # to the millimetre: a support is one length
            count = max(1, math.ceil(S / kit["spacing"])) if kit.get("spacing") else 1
            ux, uy = (leg["b"][0] - leg["a"][0]) / leg["L"], (leg["b"][1] - leg["a"][1]) / leg["L"]
            for j in range(count):
                s = a + (j + 0.5) * S / count
                at = (int(round(leg["a"][0] + ux * s)), int(round(leg["a"][1] + uy * s)))
                key = f"{leg['key']}_p{j}"
                insts.append({"key": key, "asset": sup["id"], "at": at, "face": face, "surface": surface(leg)})
                supports.append((key, at))
        elif sup:
            bare += 1
    if bare:
        notes.append({"code": "support_none", "legs": bare, "height": kit["support"]["height"], "at": pts[0]})
    # A leg's `path` holds the points its named ends do not give (spec/surfaces.md 3.3: a path
    # that starts or ends at a place has that place for its first or last point).
    out_legs = [{"key": leg["key"], "level": round(ground + leg["h"] / 1000, 3),
                 "path": ([] if leg["starts_at"] else [leg["a"]]) + ([] if leg["ends_at"] else [leg["b"]]),
                 "starts_at": leg["starts_at"], "ends_at": leg["ends_at"]} for leg in legs]
    return {"legs": out_legs, "runs": runs, "instances": insts, "notes": notes, "supports": supports}


def _list(col):
    return col.tolist() if hasattr(col, "tolist") else list(col)


def _json(v):
    if isinstance(v, str):
        try:
            return json.loads(v) if v else None
        except ValueError:
            return v
    return v


def read_pipes(step, name=None):
    """Every pipe, or the one a `for_each` call is for: [(name, first point mm, attributes)]."""
    try:
        fl = step.features(select=[name] if name else "kind:pipe", columns=ATTRS)
    except Exception:                             # no pipe in the world
        return []
    out = []
    for f, name in enumerate(_list(fl.get("name", []))):
        pt = _list(fl.rings(f)[0][0])[0]
        attrs = {}
        for a in ATTRS:
            if a in fl and (f"{a}?" not in fl or _list(fl[f"{a}?"])[f]):
                attrs[a] = _json(_list(fl[a])[f])
        out.append((name, (int(pt[0]), int(pt[1])), attrs))
    return out


def read_kit(step, taxon):
    t = step.pack(select="@" + taxon, columns=["bounds", "params", "shared"])
    rows = []
    for r, i in enumerate(_list(t.get("id", []))):
        tags = _json(_list(t["tags"])[r]) if "tags" in t else []
        b = _list(t["bounds"])[r] if "bounds" in t else None
        rows.append({"id": i, "tags": tags if isinstance(tags, list) else [tags],
                     "bounds": [int(v) for v in _list(b)] if b is not None else None,
                     "params": _json(_list(t["params"])[r]) or {} if "params" in t else {},
                     "shared": _json(_list(t["shared"])[r]) or {} if "shared" in t else {}})
    return rows


def read_ground(step, pts):
    """The terrain at each point (mm), metres; None where the world has no surface field."""
    try:
        (surface,) = step.fields(["role:surface"], window=[0, 0, 1, 1])
    except (LookupError, RpcError):
        return None
    return [float(v) for v in _list(step.eval(f"= f('{surface.name}')", x=[p[0] for p in pts], y=[p[1] for p in pts]))]


def write(step, out, section):
    o = step.out
    for leg in out["legs"]:
        ends = {k: leg[k] for k in ("starts_at", "ends_at") if leg[k]}
        o.feature(leg["key"], "pipe_leg", path=leg["path"], level=leg["level"], width=section, **ends)
    for r in out["runs"]:
        o.table("run", r["key"], asset=r["asset"], count=r["count"], ends="half", face=r["face"],
                on={"ref": r["ref"], "span": r["span"], "offset": 0})
    for it in out["instances"]:
        on = {"surface": it["surface"]} if it["surface"] is not None else {}
        o.instance(it["key"], it["asset"], at=it["at"], face=it["face"], **on)


def generate(step):
    one = getattr(step, "for_each_item", None)
    pipes = read_pipes(step, one)
    facts = {}
    x0_, y0_, x1_, y1_ = (step.grid or {}).get("extent_mm") or (-math.inf, -math.inf, math.inf, math.inf)

    def where(at):
        return {"position": (at[0] / 1000, at[1] / 1000)} if x0_ <= at[0] <= x1_ and y0_ <= at[1] <= y1_ else {}
    laid = {"pipes": 0, "legs": 0, "pieces": 0}
    for name, at, attrs in pipes:
        try:
            pts = [at] + points_of(attrs.get("to"))
            for i, q in enumerate(pts):
                if not (x0_ <= q[0] <= x1_ and y0_ <= q[1] <= y1_):
                    raise ParamError("to", f"point {i} of the pipe, [{q[0] / 1000:g}, {q[1] / 1000:g}], is outside the world's extent",
                                     "keep every point of the pipe inside the extent: its legs and pieces stand there")
            taxon = attrs.get("kit")
            if not isinstance(taxon, str) or not taxon:
                raise ParamError("kit", "the pipe names no kit", 'write `kit = "<taxon of the pipe kit>"`')
            if taxon not in facts:
                facts[taxon] = kit_facts(read_kit(step, taxon), taxon)
            kit = facts[taxon]
            g = read_ground(step, [at])
            out = plan_pipe(name, pts, attrs, kit, g[0] if g else 0.0)
        except ParamError as e:
            step.error("params", name, f"`{name}`: {e.message}", e.hint, explain=EXPLAIN["params"],
                       data={"attribute": e.key, **e.data})
            continue
        section = next((r["shared"].get("section") for r in read_kit(step, taxon) if r.get("shared")), None) or 0.3
        write(step, out, section)
        laid["pipes"] += 1
        laid["legs"] += len(out["legs"])
        laid["pieces"] += len(out["instances"]) + sum(r["count"] for r in out["runs"])
        for note in out["notes"]:
            if note["code"] == "pipe_open":
                closes = " or ".join(f"{v / 1000:g} m" for v in note["near"] if v > 0)
                step.warning("pipe_open", name,
                             f"leg {note['leg']} of `{name}` is {note['length'] / 1000:g} m and the pieces at its ends take "
                             f"{note['takes'] / 1000:g} m: what is left is no whole number of the kit's pieces, and none is laid",
                             f"make the leg {closes} long" if closes else "the kit has no piece that closes it",
                             data={"leg": note["leg"], "length": round(note["length"] / 1000, 3),
                                   "takes": round(note["takes"] / 1000, 3),
                                   "near": [round(v / 1000, 3) for v in note["near"]]},
                             explain=EXPLAIN["pipe_open"], **where(note["at"]))
            elif note["code"] == "support_none":
                step.info("support_none", name,
                          f"{note['legs']} of `{name}`'s legs have no support: the kit's stands under a pipe "
                          f"{note['height'] / 1000:g} m up", "none needed",
                          data={"legs": note["legs"], "height": round(note["height"] / 1000, 3)},
                          explain=EXPLAIN["support_none"], **where(note["at"]))
        if out["supports"] and g:
            under = read_ground(step, [at for _, at in out["supports"]])
            # to the millimetre, as the ground is read (rule 1)
            diff = [(round(h - g[0], 3), key, at) for h, (key, at) in zip(under, out["supports"])]
            off = [(abs(d), d, key, at) for d, key, at in diff if abs(d) > SEEN]
            if off:
                _, d, key, at = max(off)
                step.warning("support_off", name,
                             f"the ground under {len(off)} of `{name}`'s supports is not the ground under its first point: "
                             f"under `{key}` it is {abs(d):.2f} m {'higher' if d > 0 else 'lower'}, and a support is one length",
                             "a pipe's heights are above the ground at its first point: level the ground, or carry the pipe "
                             "at a height the land allows",
                             data={"supports": len(off), "most": round(d, 3)}, explain=EXPLAIN["support_off"], **where(at))
    if one and laid["pipes"]:
        step.info("pipes", one, f"{laid['legs']} legs, {laid['pieces']} pieces", "none needed", data=laid,
                  explain=EXPLAIN["pipes"])
    elif not one:
        step.info("pipes", "step." + step.id, f"{laid['pipes']} pipes, {laid['legs']} legs, {laid['pieces']} pieces",
                  "none needed", data=laid, explain=EXPLAIN["pipes"])

