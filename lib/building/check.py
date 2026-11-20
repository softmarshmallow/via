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
5. **Doorways, clear.** At each doorway's inner end (where it meets its floor), the clear width
   across the doorway between the wall pieces beside it: every instance of an asset under `kits`
   that is not a door leaf (`role:door`), whose footprint (its `bounds`, scaled and turned by its
   `face`) reaches within `DEPTH` (1 m) of that end along the doorway and whose foot `z` stands
   within `LOW` (0.5 m) below and `HIGH` (2 m) above its `sill_z` (so the storey above's walls,
   in plan over the doorway, are not counted), is an obstacle across it;
   the clear width is the free interval round the doorway's line. Under `inner_door` is `door_narrow`
   (warning) naming the doorway, with the width and the pieces either side. A side with no piece
   within `REACH` (3 m) is open. This measures what lay's rule 6 promises (the path's width plus a
   margin); M5 slice 1 found a front door 0.20 m clear and a step 0.29 m, under a 0.6 m walker.
6. **The kit.** Of every asset under the taxons in `kits` with `params.pitch`, one whose `bounds`
   width differs from its pitch by more than `pitch_slack` metres is `kit_pitch` (warning): a bay
   that measures 6.44 m at a 6 m pitch (the college's) overlaps its neighbour by 0.44 m.
7. **The kit's corners.** `lay` gives a corner the box its reach states (lay.py rule 6), where `T`
   is the thickness of the taxon's first `role:wall` piece and `reach` is `params.reach` (by
   default `T` outside, 0 inside). `role:corner` is `reach` square. `role:corner_in` is placed by
   the middle of its notch, so its `bounds` span x from `-(reach + T / 2)` to `T / 2` and y from
   `-T / 2` to `reach + T / 2`. A corner whose `bounds` differ from that by more than `pitch_slack`
   on any side is `kit_corner` (warning), with its measured plan and the plan it should have: a
   post re-exported with a stub along one wall is 0.9 by 0.3 m where 0.3 m square is laid for,
   and an inside corner turned the wrong way, or left on the default pivot, is found here.
9. **The kit against itself** (docs/research/Modular kit failures in studios.md, sections 2 and 3:
   what a studio settles on untextured proof pieces before any art). Per taxon in `kits` that has
   a `role:wall` piece, with `m` the pitch of its smallest wall piece (its `role:filler`, else its
   bay) and `T` the bay's thickness:
   - `kit_module` (warning): the bay's pitch, or the door piece's width, is no whole number of
     `m`. The lengths that close then follow no one module (a 3.2 m bay beside a 1.5 m half bay;
     a hall of one size beside a room of another, which joins once and misses when a plan loops).
   - `kit_ell` (warning): the outside corner's reach and the inside corner's do not add up to a
     whole number of `m`, so no plan with an inside corner closes (lay.py rule 6, "Corners"), and
     a building of this kit has one. The data names the buildings and the reach an inside corner
     piece would need. A kit that only ever walls rectangles is not told.
   - `kit_slip` (warning): a filler shorter than the wall is thick. It closes any length, so it
     hides every plan that was not sized to the kit, and is laid by the dozen where it is used.
   - `kit_uneven` (warning): the wall pieces (`wall`, `filler`, `door`) differ in thickness, or
     `wall`, `filler` and the corners differ in height, by more than `pitch_slack`. A door piece
     may be any height: a leaf, a frame, or a lintel over an open doorway.
8. **The kit against the storey.** For every building of two storeys or more, each piece of its
   `kit` that is a wall or a corner (`wall`, `filler`, `corner`, `corner_in`) whose `bounds`
   height differs from the building's `storey_h` by more than `pitch_slack` is `kit_height`
   (warning): a taller piece stands into the storey above, a shorter one stops short of it.

`params`: `riser_max` (0.19 m), `going_min` (0.25 m), `turn_max` (45), `inner_door` (0.9 m),
`pitch_slack` (0.05 m, for every kit measure), `kits` (taxon paths, default none). Codes are `step.<id>.<code>`;
`checked` (info) counts what was judged.

The clear width is the plan's: two boxes' footprints, not their collision meshes (the engine's).

Library code imports only viasdk and the standard library (R19b).
"""

import json
import math

DEPTH = 1000         # mm along a doorway from its inner end within which a piece stands beside it
LOW, HIGH = 0.5, 2.0  # metres below and above a doorway's sill a piece's foot stands to be beside it
REACH = 3000         # mm across a doorway past which a piece does not narrow it

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
    "door_narrow": "The clear width between the wall pieces at a doorway is under `inner_door`: a "
    "walker cannot pass. Widen the door or the margin lay leaves (`params.margin`), or use pieces no "
    "wider than their pitch.",
    "kit_pitch": "A kit piece's measured width differs from the pitch it repeats at: its neighbours "
    "in a run overlap or leave a gap.",
    "kit_corner": "A corner piece's measured plan is not what its reach states (`params.reach`; by "
    "default the wall's thickness for `role:corner`, nothing for `role:corner_in`). The walls beside "
    "it are laid for that plan, so the piece overlaps them, leaves a gap or stands outside the "
    "outline. Correct the model, or state the reach it really takes.",
    "kit_module": "The kit's pieces do not share one module: the bay, or the door piece, is not a "
    "whole number of its smallest wall piece. Lengths that close then follow no one module, and a "
    "plan sized for one piece fails with another. Make each a multiple of the smallest.",
    "kit_ell": "The kit's corners cannot close a plan with an inside corner: what the outside corner "
    "and the inside corner take from a wall must add up to a whole number of the smallest wall piece "
    "(lib/building/README.md, Sizing a plan to its kit). Give the kit a `role:corner_in` piece with "
    "the reach the data names.",
    "kit_slip": "A filler shorter than the wall is thick closes any length, so it hides plans that "
    "were not sized to the kit and is laid by the dozen. Size the plan's sides to the kit and take "
    "the piece out.",
    "kit_uneven": "Pieces that stand in one wall differ in thickness or height: they step where "
    "they meet. Make them one thickness and one storey high.",
    "kit_height": "A kit piece's measured height is not the storey height of a building that stacks "
    "it: on a storey below another it stands into the one above, or stops short of it. Make the "
    "piece one storey high, or set the building's `storey_h` to the kit's.",
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


def number(v):
    """A pack param as a number, or None where it says none or says something that is no length
    (which `lay` reports when it reads the kit: lay.py `kit_number`)."""
    if isinstance(v, bool) or not isinstance(v, (int, float)) or not math.isfinite(v) or v < 0:
        return None
    return float(v)


def footprint(piece):
    """The four corners (mm, world) of an instance's `bounds` box, scaled and turned by its face
    (millidegrees clockwise from north; the asset's +y looks along it)."""
    b, k = piece["bounds"], piece.get("scale", 1.0)
    th = math.radians(piece["face"] / 1000.0)
    c, s_ = math.cos(th), math.sin(th)
    out = []
    for px, py in ((b[0], b[1]), (b[3], b[1]), (b[3], b[4]), (b[0], b[4])):
        px, py = px * k, py * k
        out.append((piece["x"] + px * c + py * s_, piece["y"] - px * s_ + py * c))
    return out


def clear_width(path, pieces, sill=None):
    """(clear mm or None when a side is open, the pieces bounding it either side) across a
    doorway's inner end (rule 5); with `sill` (metres), only pieces whose `z` stands beside it."""
    end, prev = path[-1], path[-2]
    L = math.dist(prev, end)
    d = ((end[0] - prev[0]) / L, (end[1] - prev[1]) / L)
    a = (-d[1], d[0])
    left, right = (-math.inf, None), (math.inf, None)
    for pc in pieces:
        if sill is not None and pc.get("z") is not None and not sill - LOW <= pc["z"] <= sill + HIGH:
            continue
        qs, cs = [], []
        for x, y in footprint(pc):
            qs.append((x - end[0]) * d[0] + (y - end[1]) * d[1])
            cs.append((x - end[0]) * a[0] + (y - end[1]) * a[1])
        if max(qs) < -DEPTH or min(qs) > DEPTH or min(cs) > REACH or max(cs) < -REACH:
            continue
        if min(cs) < 0 < max(cs):
            return 0.0, (pc["name"], pc["name"])
        if max(cs) <= 0 and max(cs) > left[0]:
            left = (max(cs), pc["name"])
        if min(cs) >= 0 and min(cs) < right[0]:
            right = (min(cs), pc["name"])
    if left[1] is None or right[1] is None:
        return None, (left[1], right[1])
    return right[0] - left[0], (left[1], right[1])


def judge(features, meetings, kit_rows, p, pieces=()):
    """Records `(severity, code, subject, message, hint, data, position)`, in a fixed order:
    storeys, rooms, flights, meetings, doorways, kit (each in name order), then `checked`.
    `pieces` are the kit's placed wall pieces (rule 5): name, x, y (mm), z (metres), face (mdeg),
    scale, bounds (mm, the asset's six)."""
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
    doorways = sorted((f for f in features if f["kind"] == "doorway"), key=lambda f: f["name"])
    measured = 0
    for f in doorways:
        if len(f["path"]) < 2 or not pieces:
            continue
        sill = f["attrs"].get("sill_z")
        w, (lo, hi) = clear_width(f["path"], pieces, float(sill) if sill is not None else None)
        if w is None:
            continue
        measured += 1
        if w < inner:
            end = f["path"][-1]
            out.append(("warning", "door_narrow", f["name"],
                        f"`{f['name']}` is {w / 1000:.3f} m clear between `{lo}` and `{hi}`, under "
                        f"{inner / 1000:g} m", "widen the door or lay's `margin`",
                        {"clear": round(w / 1000, 3), "need": inner / 1000, "pieces": [lo, hi]},
                        (end[0] / 1000, end[1] / 1000)))
    for r in sorted(kit_rows, key=lambda r: r["id"]):
        pitch = number((r.get("params") or {}).get("pitch"))
        b = r.get("bounds")
        if pitch is None or not b:
            continue
        width = (b[3] - b[0]) / 1000.0
        if abs(width - float(pitch)) > p["pitch_slack"]:
            out.append(("warning", "kit_pitch", r["id"], f"`{r['id']}` is {width:g} m wide at a "
                        f"{float(pitch):g} m pitch", "measure the piece again or correct its `params.pitch`",
                        {"asset": r["id"], "width": width, "pitch": float(pitch)}, None))
    wall_roles = ("role:wall", "role:filler", "role:corner", "role:corner_in", "role:door")
    by_taxon = {}
    for r in sorted(kit_rows, key=lambda r: r["id"]):
        by_taxon.setdefault(r.get("taxon"), []).append(r)
    for taxon in sorted(by_taxon, key=str):
        rows = by_taxon[taxon]
        wall = next((r for r in rows if "role:wall" in (r.get("tags") or []) and r.get("bounds")), None)
        if wall is None:
            continue
        T = (wall["bounds"][4] - wall["bounds"][1]) / 1000.0
        for r in rows:
            tags, b = r.get("tags") or [], r.get("bounds")
            inner = "role:corner_in" in tags
            if not b or not (inner or "role:corner" in tags):
                continue
            reach = number((r.get("params") or {}).get("reach"))
            reach = reach if reach is not None else (0.0 if inner else T)
            w, d = (b[3] - b[0]) / 1000.0, (b[4] - b[1]) / 1000.0
            if inner:
                want = [-(reach + T / 2), -T / 2, T / 2, reach + T / 2]
                got = [b[0] / 1000.0, b[1] / 1000.0, b[3] / 1000.0, b[4] / 1000.0]
                if any(abs(g - v) > p["pitch_slack"] for g, v in zip(got, want)):
                    out.append(("warning", "kit_corner", r["id"], f"`{r['id']}` spans x {got[0]:g} to {got[2]:g} m "
                                f"and y {got[1]:g} to {got[3]:g} m where an inside corner reaching {reach:g} m "
                                f"spans x {want[0]:g} to {want[2]:g} m and y {want[1]:g} to {want[3]:g} m",
                                "put the model's origin at the notch's middle with `pivot = \"origin\"`, a leg "
                                "along each wall (lay.py rule 6), or state the reach it takes",
                                {"asset": r["id"], "plan": got, "reach": reach, "want": want}, None))
            elif abs(w - reach) > p["pitch_slack"] or abs(d - reach) > p["pitch_slack"]:
                out.append(("warning", "kit_corner", r["id"], f"`{r['id']}` is {w:g} by {d:g} m where its "
                            f"reach ({reach:g} m) makes it {reach:g} m square",
                            "correct the model, or state the reach it takes in `params.reach`",
                            {"asset": r["id"], "plan": [w, d], "reach": reach, "square": reach}, None))
    slack = p["pitch_slack"]

    def turns_in(ring):
        """Whether an outline has an inside corner: a turn against the way it winds."""
        n = len(ring)
        area = sum(ring[i][0] * ring[(i + 1) % n][1] - ring[(i + 1) % n][0] * ring[i][1] for i in range(n))
        for i in range(n):
            a, b, c = ring[i - 1], ring[i], ring[(i + 1) % n]
            if ((b[0] - a[0]) * (c[1] - b[1]) - (b[1] - a[1]) * (c[0] - b[0])) * area < 0:
                return True
        return False
    ells = {}
    for f in sorted((f for f in features if f["kind"] == "building"), key=lambda f: f["name"]):
        if f["attrs"].get("kit") and turns_in(f["ring"]):
            ells.setdefault(f["attrs"]["kit"], []).append(f["name"])

    def off(length, m):
        """How far `length` is from a whole number of `m`, metres."""
        r = length % m
        return min(r, m - r)
    for taxon in sorted(by_taxon, key=str):
        rows = by_taxon[taxon]
        role = {}
        for r in rows:
            for t in r.get("tags") or []:
                if t.startswith("role:") and r.get("bounds"):
                    role.setdefault(t[5:], r)

        def pitch_of(r):
            v = number((r.get("params") or {}).get("pitch"))
            return v if v is not None else (r["bounds"][3] - r["bounds"][0]) / 1000.0
        if "wall" not in role:
            continue
        wall = role["wall"]
        T = (wall["bounds"][4] - wall["bounds"][1]) / 1000.0
        m = pitch_of(role["filler"]) if "filler" in role else pitch_of(wall)
        if m <= 0:
            continue
        for name in ("wall", "door"):
            if name in role and off(pitch_of(role[name]), m) > slack:
                r = role[name]
                out.append(("warning", "kit_module", r["id"], f"`{r['id']}` takes {pitch_of(r):g} m of wall, "
                            f"no whole number of the kit's smallest piece ({m:g} m)",
                            f"make it a multiple of {m:g} m", {"asset": r["id"], "pitch": pitch_of(r), "module": m},
                            None))
        if "corner" in role:
            def reach_of(r, default):
                v = number((r.get("params") or {}).get("reach"))
                return v if v is not None else default
            r_out = reach_of(role["corner"], T)
            inner = role.get("corner_in") or role["corner"]
            r_in = reach_of(inner, 0.0)
            if off(r_out + r_in, m) > slack and taxon in ells:
                need = (m - r_out % m) % m
                out.append(("warning", "kit_ell", inner["id"], f"the corners of `{taxon}` take {r_out:g} m "
                            f"and {r_in:g} m of a wall, no whole number of its smallest piece ({m:g} m): "
                            f"{', '.join('`' + n + '`' for n in ells[taxon])}, with an inside corner, will not close",
                            f"give the kit a `role:corner_in` piece reaching {need:g} m",
                            {"taxon": taxon, "outside": r_out, "inside": r_in, "module": m, "reach": round(need, 3),
                             "buildings": ells[taxon]}, None))
        if "filler" in role and m < T - slack:
            r = role["filler"]
            out.append(("warning", "kit_slip", r["id"], f"`{r['id']}` is {m:g} m long in a wall {T:g} m thick: "
                        "a filler that closes any length", "size the plans to the kit and take the piece out",
                        {"asset": r["id"], "pitch": m, "thick": T}, None))
        thick = {r["id"]: (r["bounds"][4] - r["bounds"][1]) / 1000.0 for n, r in role.items()
                 if n in ("wall", "filler", "door")}
        high = {r["id"]: (r["bounds"][5] - r["bounds"][2]) / 1000.0 for n, r in role.items()
                if "role:" + n in wall_roles and n != "door"}       # a door piece may be a leaf or a lintel
        for what, sizes in (("thick", thick), ("high", high)):
            if max(sizes.values()) - min(sizes.values()) > slack:
                out.append(("warning", "kit_uneven", wall["id"], f"the wall pieces of `{taxon}` are not one size: "
                            + ", ".join(f"`{i}` {v:g} m {what}" for i, v in sorted(sizes.items())),
                            "make them one thickness and one storey high",
                            {"taxon": taxon, "measure": what, "pieces": sorted(sizes),
                             "sizes": [sizes[i] for i in sorted(sizes)]}, None))
    stacked = {}
    for f in sorted((f for f in features if f["kind"] == "building"), key=lambda f: f["name"]):
        a = f["attrs"]
        if a.get("kit") and int(a.get("storeys") or 1) >= 2:
            stacked.setdefault((a["kit"], float(a.get("storey_h") or 3.2)), []).append(f["name"])
    for (taxon, storey_h), names in sorted(stacked.items()):
        for r in by_taxon.get(taxon, []):
            b = r.get("bounds")
            if not b or not any(t in wall_roles and t != "role:door" for t in (r.get("tags") or [])):
                continue
            high = (b[5] - b[2]) / 1000.0
            if abs(high - storey_h) > p["pitch_slack"]:
                how = (f"it stands {high - storey_h:g} m into the storey above" if high > storey_h
                       else f"it stops {storey_h - high:g} m short of the storey above")
                out.append(("warning", "kit_height", r["id"], f"`{r['id']}` is {high:g} m high on the "
                            f"{storey_h:g} m storeys of {', '.join('`' + n + '`' for n in names)}: {how}",
                            "make the piece one storey high, or set `storey_h` to the kit's",
                            {"asset": r["id"], "height": high, "storey_h": storey_h, "buildings": names}, None))
    out.append(("info", "checked", None, f"{len(storeys)} storeys, {len(rooms)} rooms, {len(flights)} "
                f"flights, {len(meetings)} meetings ({nturn} with a turn), {measured} doorways measured, "
                f"{len(kit_rows)} kit pieces", "none needed",
                {"storeys": len(storeys), "rooms": len(rooms), "flights": len(flights),
                 "meetings": len(meetings), "doorways": measured, "kit": len(kit_rows)}, None))
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
                       ("doorway", ["sill_z"]), ("footpath", []), ("building", ["kit", "storey_h", "storeys"])):
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
            tags = _json(_list(t["tags"])[r]) if "tags" in t else []
            rows.append({"id": i, "taxon": taxon,
                         "bounds": [int(v) for v in _list(_list(t["bounds"])[r])] if "bounds" in t else None,
                         "params": (_json(_list(t["params"])[r]) or {}) if "params" in t else {},
                         "tags": tags if isinstance(tags, list) else [tags]})
    return rows


def read_pieces(step, taxons, kit_rows):
    """Placed instances of the kits' assets that are not door leaves, with their asset's bounds."""
    bounds = {r["id"]: r["bounds"] for r in kit_rows
              if r.get("bounds") and "role:door" not in (r.get("tags") or [])}
    out = []
    for taxon in taxons:
        t = step.instances(select="@" + taxon)
        names = _list(t.get("name", []))
        for r, name in enumerate(names):
            asset = _list(t["asset"])[r]
            if asset not in bounds:
                continue
            out.append({"name": name, "x": int(_list(t["x"])[r]), "y": int(_list(t["y"])[r]),
                        "face": int(_list(t["face"])[r]), "z": float(_list(t["z"])[r]) if "z" in t else None,
                        "scale": float(_list(t["scale"])[r]) if "scale" in t else 1.0, "bounds": bounds[asset]})
    return out


def lint(step):
    try:
        p = params_of(step.params)
    except ParamError as e:
        step.error("params", "step." + step.id, str(e), "see lib/building/check.py", explain=EXPLAIN["params"])
        return
    said = set()
    kits = read_kits(step, p["kits"])
    for sev, code, subject, msg, hint, data, pos in judge(read_features(step), read_meetings(step), kits, p,
                                                         read_pieces(step, p["kits"], kits)):
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
    if not subject or code.startswith("kit_"):
        return None
    return "feature." + subject.split(".")[0]
