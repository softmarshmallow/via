"""The building library's lint: is every storey and room of a building reachable, and are its
stairs, doors and kit within their profile?

A lint step (`when = "lint"`), run as a worker through the SDK; it reads the finished build
(R29): `build.features` of the library's kinds, `surface.meetings` (surfaces.md 2.4) and, for the
kit, `pack.query`. It changes nothing.

What it judges (ours, written for this library; surfaces.md 4.4 is what a walk may assume):

1. **Reach, between surfaces.** A walker starts on the terrain. It changes surface only at a
   meeting that holds (gap 0; connectors equal where both have one): a path end on a doorway or a
   footpath lies on the terrain, one on a flight on its `tread`, and a place `<storey>.floor` on that
   storey. A storey no chain of meetings reaches from the terrain is `unreached` (warning).
2. **Reach, between rooms.** On a reached storey, a room is entered where a meeting on that storey's
   floor lies inside it (edges included), and from a room the walker crosses to every room of the
   same storey that shares at least `inner_door` metres of outline with it (lay leaves an opening
   that wide in the wall there). A room not entered so is `unreached`. Wall pieces' collision is the
   engine's; this is the plan's reach, not a body's (lib/review's reach lint walks posts, and a
   walk over kind surfaces waits for `surfaces_at` and `sample` in the SDK).
3. **Stairs.** A flight of rise `|z1 - z0|` over its path's length `L` has `n = ceil(rise /
   riser_max)` risers and a going of `L / n`; a going under `going_min` is `stair` (warning).
4. **Doors that face.** A meeting with a `turn` above `turn_max` degrees is `turn` (warning): a
   path that arrives at a door's mount from the side (surfaces.md 3.4: the core reports `turn`, a
   library judges it).
5. **The kit.** Of every asset under the taxons in `kits` with `params.pitch`, one whose `bounds`
   width differs from its pitch by more than `pitch_slack` metres is `kit_pitch` (warning): a bay
   that measures 6.44 m at a 6 m pitch (the college's) overlaps its neighbour by 0.44 m.

`params`: `riser_max` (0.19 m), `going_min` (0.25 m), `turn_max` (45), `inner_door` (0.9 m),
`pitch_slack` (0.05 m), `kits` (taxon paths, default none). Codes are `step.<id>.<code>`;
`checked` (info) counts what was judged.

Library code imports only viasdk and the standard library (R19b).
"""

import json
import math

DEFAULTS = {"riser_max": 0.19, "going_min": 0.25, "turn_max": 45.0, "inner_door": 0.9,
            "pitch_slack": 0.05, "kits": []}

EXPLAIN = {
    "params": "A param of the building check is outside what it takes: numbers above 0, `kits` a "
    "list of taxon paths.",
    "unreached": "No walk from the terrain reaches this storey or room: walkers change surface only "
    "at meetings that hold (surfaces.md 4.4), and cross between rooms only at their openings.",
    "stair": "A flight's going is shorter than the profile's least: its rise needs more risers than "
    "its length gives room for. Lengthen the flight's `run` or raise `storey_h` less.",
    "turn": "A path arrives at a place that faces, from the side or behind: its `turn` is above the "
    "profile's `turn_max`. Bring the path in square to the door.",
    "kit_pitch": "A kit piece's measured width differs from the pitch it repeats at: its neighbours "
    "in a run overlap or leave a gap.",
    "checked": "What the building check judged: storeys, rooms, flights, meetings and kit pieces.",
}


class ParamError(ValueError):
    pass


def params_of(raw):
    p = dict(DEFAULTS)
    for k, v in (raw or {}).items():
        if k not in DEFAULTS:
            raise ParamError(f"`{k}` is no param of the building check")
        if k == "kits":
            if not isinstance(v, list) or not all(isinstance(x, str) for x in v):
                raise ParamError("`kits` must be a list of taxon paths")
            p[k] = v
            continue
        try:
            p[k] = float(v)
        except (TypeError, ValueError):
            raise ParamError(f"`{k}` must be a number")
        if p[k] <= 0:
            raise ParamError(f"`{k}` must be above 0")
    return p


# --- geometry, in millimetres ----------------------------------------------------------------


def inside(ring, p):
    x, y = p
    hit = False
    for a, b in zip(ring, ring[1:] + ring[:1]):
        cross = (b[0] - a[0]) * (y - a[1]) - (b[1] - a[1]) * (x - a[0])
        if cross == 0 and min(a[0], b[0]) <= x <= max(a[0], b[0]) and min(a[1], b[1]) <= y <= max(a[1], b[1]):
            return True
        if (a[1] > y) != (b[1] > y):
            xi = a[0] + (y - a[1]) * (b[0] - a[0]) / (b[1] - a[1])
            if x < xi:
                hit = not hit
    return hit


def shared(r1, r2):
    """Length (mm) of outline two rings share: collinear edges' overlap."""
    total = 0.0
    for a, b in zip(r1, r1[1:] + r1[:1]):
        L = math.dist(a, b)
        if L == 0:
            continue
        e = ((b[0] - a[0]) / L, (b[1] - a[1]) / L)
        for c, d in zip(r2, r2[1:] + r2[:1]):
            off = [((q[0] - a[0]) * e[1] - (q[1] - a[1]) * e[0]) for q in (c, d)]
            if abs(off[0]) > 0.5 or abs(off[1]) > 0.5:
                continue
            s = sorted(((q[0] - a[0]) * e[0] + (q[1] - a[1]) * e[1]) for q in (c, d))
            total += max(0.0, min(L, s[1]) - max(0.0, s[0]))
    return total


def length(path):
    return sum(math.dist(a, b) for a, b in zip(path, path[1:]))


# --- the judgement ----------------------------------------------------------------------------


def surface_of(place, kinds):
    """The walk node a mount lies on (rule 1), or None for a place this lint does not know."""
    owner, _, part = place.rpartition(".")
    kind = kinds.get(owner)
    if kind in ("doorway", "footpath") and part in ("start", "end"):
        return "terrain"
    if kind == "flight" and part in ("start", "end"):
        return owner
    if kind == "storey" and part == "floor":
        return owner
    return None


def judge(features, meetings, kit_rows, p):
    """Records `(severity, code, subject, message, hint, data, position)`, in a fixed order:
    storeys, rooms, flights, meetings, kit (each in name order), then `checked`."""
    out = []
    kinds = {f["name"]: f["kind"] for f in features}
    storeys = sorted(f["name"] for f in features if f["kind"] == "storey")
    rooms = sorted((f for f in features if f["kind"] == "room"), key=lambda f: f["name"])
    flights = sorted((f for f in features if f["kind"] == "flight"), key=lambda f: f["name"])
    graph = {}
    held = []
    for m in meetings:
        if m["gap"] != 0:
            continue
        ca, cb = m.get("connector_a"), m.get("connector_b")
        if ca and cb and ca != cb:
            continue
        sa, sb = surface_of(m["a"], kinds), surface_of(m["b"], kinds)
        if sa is None or sb is None:
            continue
        graph.setdefault(sa, set()).add(sb)
        graph.setdefault(sb, set()).add(sa)
        held.append((sa, sb, (m["x"], m["y"])))
    reached, todo = {"terrain"}, ["terrain"]
    while todo:
        for n in sorted(graph.get(todo.pop(), ())):
            if n not in reached:
                reached.add(n)
                todo.append(n)
    for s in storeys:
        if s not in reached:
            out.append(("warning", "unreached", s, f"no walk from the terrain reaches `{s}`",
                        "end a doorway or a flight at its floor", {"what": "storey"}, None))
    inner = p["inner_door"] * 1000
    by_storey = {}
    for r in rooms:
        by_storey.setdefault(r["attrs"].get("storey"), []).append(r)
    for st in sorted(by_storey, key=str):
        rs = by_storey[st]
        entries = [pt for a, b, pt in held if st in (a, b)] if st in reached else []
        entered = {r["name"] for r in rs if any(inside(r["ring"], pt) for pt in entries)}
        todo = sorted(entered)
        while todo:
            here = todo.pop()
            cur = next(r for r in rs if r["name"] == here)
            for r in rs:
                if r["name"] not in entered and shared(cur["ring"], r["ring"]) >= inner:
                    entered.add(r["name"])
                    todo.append(r["name"])
        for r in rs:
            if r["name"] not in entered:
                out.append(("warning", "unreached", r["name"], f"no walk reaches room `{r['name']}`",
                            "open it to a reached room or bring a doorway to it",
                            {"what": "room", "storey": st}, None))
    for f in flights:
        z0, z1 = f["attrs"].get("z0"), f["attrs"].get("z1")
        if z0 is None or z1 is None:
            continue
        rise = abs(float(z1) - float(z0))
        L = length(f["path"]) / 1000.0
        if rise == 0:
            continue
        n = math.ceil(rise / p["riser_max"] - 1e-9)
        going = L / n
        if going < p["going_min"]:
            out.append(("warning", "stair", f["name"],
                        f"`{f['name']}` rises {rise:g} m over {L:g} m: {n} risers at a {going:.3f} m going",
                        f"lengthen it to {n * p['going_min']:g} m or more",
                        {"rise": round(rise, 3), "length": round(L, 3), "risers": n, "going": round(going, 3)},
                        None))
    limit = p["turn_max"] * 1000
    nturn = 0
    for m in meetings:
        t = m.get("turn")
        if t is None:
            continue
        nturn += 1
        if t > limit:
            out.append(("warning", "turn", m["a"], f"`{m['a']}` arrives at `{m['b']}` turned "
                        f"{t / 1000:g} degrees from its face", "bring the path in square to it",
                        {"turn": t / 1000, "b": m["b"]}, (m["x"] / 1000, m["y"] / 1000)))
    for r in sorted(kit_rows, key=lambda r: r["id"]):
        pitch = (r.get("params") or {}).get("pitch")
        b = r.get("bounds")
        if pitch is None or not b:
            continue
        width = (b[3] - b[0]) / 1000.0
        if abs(width - float(pitch)) > p["pitch_slack"]:
            out.append(("warning", "kit_pitch", r["id"], f"`{r['id']}` is {width:g} m wide at a "
                        f"{float(pitch):g} m pitch", "measure the piece again or correct its `params.pitch`",
                        {"asset": r["id"], "width": width, "pitch": float(pitch)}, None))
    out.append(("info", "checked", None, f"{len(storeys)} storeys, {len(rooms)} rooms, {len(flights)} "
                f"flights, {len(meetings)} meetings ({nturn} with a turn), {len(kit_rows)} kit pieces",
                "none needed", {"storeys": len(storeys), "rooms": len(rooms), "flights": len(flights),
                                "meetings": len(meetings), "kit": len(kit_rows)}, None))
    return out


# --- the step ---------------------------------------------------------------------------------


def _list(col):
    return col.tolist() if hasattr(col, "tolist") else list(col)


def _json(v):
    if isinstance(v, str):
        try:
            return json.loads(v) if v else None
        except ValueError:
            return v
    return v


def read_features(step):
    out = []
    for kind, cols in (("storey", ["level"]), ("room", ["storey"]), ("flight", ["z0", "z1"]),
                       ("doorway", []), ("footpath", [])):
        try:
            fl = step.features(select=f"kind:{kind}", columns=cols)
        except Exception:                         # a kind no feature has: nothing to judge
            continue
        names = _list(fl.get("name", []))
        for f, name in enumerate(names):
            pts = [(int(v[0]), int(v[1])) for v in _list(fl.rings(f)[0][0])]
            attrs = {c: _list(fl[c])[f] for c in cols if c in fl and (f"{c}?" not in fl or _list(fl[f"{c}?"])[f])}
            out.append({"name": name, "kind": kind, "ring": pts, "path": pts, "attrs": attrs})
    return out


def read_meetings(step):
    t = step.meetings()
    n = len(_list(t.get("a", [])))

    def col(k, r):
        if k not in t or (f"{k}?" in t and not _list(t[f"{k}?"])[r]):
            return None
        return _list(t[k])[r]
    return [{"a": col("a", r), "b": col("b", r), "x": int(col("x", r)), "y": int(col("y", r)),
             "gap": int(col("gap", r)), "turn": col("turn", r), "connector_a": col("connector_a", r),
             "connector_b": col("connector_b", r)} for r in range(n)]


def read_kits(step, taxons):
    rows = []
    for taxon in taxons:
        t = step.pack(select="@" + taxon, columns=["bounds", "params"])
        for r, i in enumerate(_list(t.get("id", []))):
            rows.append({"id": i, "bounds": [int(v) for v in _list(_list(t["bounds"])[r])] if "bounds" in t else None,
                         "params": (_json(_list(t["params"])[r]) or {}) if "params" in t else {}})
    return rows


def lint(step):
    try:
        p = params_of(step.params)
    except ParamError as e:
        step.error("params", "step." + step.id, str(e), "see lib/building/check.py", explain=EXPLAIN["params"])
        return
    said = set()
    for sev, code, subject, msg, hint, data, pos in judge(read_features(step), read_meetings(step),
                                                         read_kits(step, p["kits"]), p):
        kw = {"data": data}
        if pos is not None:
            kw["position"] = pos
        if code not in said:
            kw["explain"] = EXPLAIN[code]
            said.add(code)
        getattr(step, sev)(code, address(code, subject) or "step." + step.id, msg, hint, **kw)


def address(code, subject):
    """A judged name as a diagnostic subject (diagnostics.md 1.1): a feature, or a place on one
    (`walk.end`), is `feature.<the name's first part>`. A kit piece is a pack asset, no entity of
    the world, so the step itself is the subject and the piece is named in the message and data."""
    if not subject or code == "kit_pitch":
        return None
    return "feature." + subject.split(".")[0]
