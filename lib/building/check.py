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
   that is not a door leaf (`role:door`), a floor tile or a roof piece, whose footprint (its `bounds`, scaled and turned by its
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
   is the thickness of the taxon's bay (its first `role:wall` piece with no `variant:` tag, as
   `lay` reads it: for every role a piece with no such tag comes before one with) and `reach` is `params.reach` (by
   default `T` outside, 0 inside). `role:corner` is `reach` square. `role:corner_in` is placed by
   the middle of its notch, so its `bounds` span x from `-(reach + T / 2)` to `T / 2` and y from
   `-T / 2` to `reach + T / 2`. A corner whose `bounds` differ from that by more than `pitch_slack`
   on any side is `kit_corner` (warning), with its measured plan and the plan it should have: a
   post re-exported with a stub along one wall is 0.9 by 0.3 m where 0.3 m square is laid for,
   and an inside corner turned the wrong way, or left on the default pivot, is found here.
9. **The kit against itself** (docs/research/Modular kit failures in studios.md, sections 2 and 3:
   what a studio settles on untextured proof pieces before any art). A kit is judged once for
   each kind of storey its buildings have (rule 8: the word and the height), from the pieces that
   storey reads, and a fault found for several is said once; a kit no building uses is judged
   from its pieces with no `storey:` tag, once for each height its bays have. A variant is `kit_variant` only when no kind of storey
   that reads it can lay it. Per taxon in `kits` that has a `role:wall` piece, with `m` the pitch of its smallest wall piece (its `role:filler`, else its
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
   - `kit_variant` (warning): a `role:wall` piece tagged `variant:<word>` is not the bay's size:
     its pitch, or its measured box (length, thickness, height, or where the box lies about the
     origin), differs from the bay's by more than 5 mm, read in whole millimetres as `lay` reads
     them (lay.py rule 8). `lay` refuses it, and lays the bay where it is asked for. So is a piece
     tagged `variant:wall`: `wall` is the bay's own word.
   - `kit_deck` (warning): a `role:floor` tile that is not square, or of which `m` is no whole
     number (a floor inside walls sized to the kit is then not whole tiles); a `role:roof_slope`
     whose measured depth is not its `params.span`, or a `role:roof_gable` whose length is not; a
     gable half that does not rise as the slope does, is not `T` thick, or is made for another
     span than the slope; a kit with one of the two roof pieces and not the other.
   - `kit_bay` (warning): a kind of storey some building has, for which the kit has no bay: its
     only `role:wall` pieces with no `variant:` tag are tagged for other storeys. `lay` walls
     that storey with a variant, or not at all.
   - `kit_uneven` (warning): the bays of the kit's storeys differ in thickness; or the wall
     pieces one storey reads (`wall`, `filler`, `door`) differ in thickness, or its `wall`,
     `filler` and corners differ in height, by more than `pitch_slack`. A door piece may be any
     height: a leaf, a frame, or a lintel over an open doorway. An inside corner piece is held
     to the others' height only where some building of the kit turns in: one no building uses is
     not judged.
8. **The kit against the storey.** A storey reads the kit as `lay` does (lay.py rule 6, "Pieces
   by storey": for each role the pieces nearest its own height, and of its own word,
   `storey:ground`, `storey:upper` or `storey:top`, before the untagged). For every building that
   stacks or is roofed (two storeys or more, a `ground_h` or `top_h`, or `roof` flat or gable),
   the piece each of its storeys gets for `wall`, `filler` and `corner` (and `corner_in` where the
   outline turns in) whose `bounds` height differs from that storey's height by more than
   `pitch_slack` is `kit_height` (warning): a taller piece stands into the storey above, a
   shorter one stops short of it. A storey whose height no piece of a role has is said this way.
10. **The kit against what it states** (lay.py rule 11; R66 of docs/doctrine.md). A kit states
   once what its pieces share, in its taxon's `defaults.shared`, and each piece is held to it:
   `kit_stated` (warning) for a `wall`, `filler` or `door` piece whose measured thickness is not
   the stated `wall`; a `role:wall` piece whose pitch is not the stated `module`, or a filler of
   whose pitch the module is no whole number; a `wall`, `filler` or `corner` piece whose height
   is none of the stated `storey` heights, or, tagged `storey:parapet`, is not the stated
   `parapet`; each by more than `pitch_slack`. So a piece
   re-exported a little thicker is told against the kit's own word, not against whichever piece
   was read first. A kit that states nothing is `kit_unstated` (info), once, with what was worked
   out of its bay and the line that would state it.

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

# What stands in no one's way across a doorway: the door's own piece, a floor tile, the roof.
LIES_FLAT = ("role:door", "role:floor", "role:roof", "role:roof_slope", "role:roof_gable")

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
    "kit_variant": "A variant of the bay is not the bay's size. A variant stands where a bay stood, "
    "so it must share the bay's pitch, thickness and height; `lay` lays the bay in its place.",
    "kit_uneven": "Pieces that stand in one wall differ in thickness or height: they step where "
    "they meet. Make them one thickness and one storey high.",
    "kit_deck": "A floor tile or a roof piece is not what the building library lays it for: a tile is "
    "square on the kit's module; a roof slope is as deep as the span it states; a gable half is that span "
    "long, as high as the slope and as thick as the walls (lay.py rules 9 and 10).",
    "kit_stated": "A kit states once what its pieces share (the `shared` table of its taxon's `defaults`), "
    "and a piece differs from it: a wall piece of another thickness, a bay of another module, a piece of "
    "another storey height. The building library lays by what the kit states, so the piece is laid as if it "
    "were what the kit says. Correct the piece, or the statement.",
    "kit_unstated": "A kit that states nothing its pieces share is laid by what the building library works out "
    "of its bay. State the module, the wall's thickness and the storey once, in the `defaults.shared` of the "
    "kit's taxon, and every piece is held to them.",
    "kit_bay": "A kit's bays are all tagged for other kinds of storey than one a building has, so that "
    "storey has no piece to be walled with. An untagged bay serves every storey with none of its own.",
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
    # For every role a piece with no `variant:` tag comes before one with, and one with no
    # `storey:` tag before one with, then by id: lay.py's order for a storey with no word.
    def tag(r, head):
        return next((t[len(head):] for t in r.get("tags") or [] if t.startswith(head)), None)
    for r in sorted(kit_rows, key=lambda r: (tag(r, "variant:") is not None, tag(r, "storey:") is not None, r["id"])):
        by_taxon.setdefault(r.get("taxon"), []).append(r)

    def storey_rows(rows, word, height):
        """The rows a storey of kind `word`, `height` mm reads: lay.py's `kit_rows`, in its order
        (a piece tagged for this storey before an untagged one)."""
        rows = [r for r in rows if tag(r, "storey:") in (None, word)]
        if height is not None:
            for role in ("wall", "filler", "corner", "corner_in", "door"):
                mine = [r for r in rows if "role:" + role in (r.get("tags") or []) and r.get("bounds")
                        and tag(r, "variant:") is None]
                near = min((abs(r["bounds"][5] - r["bounds"][2] - height) for r in mine), default=0)
                fits = [r for r in mine if abs(r["bounds"][5] - r["bounds"][2] - height) <= near + 5]
                if len(fits) < len(mine):
                    drop = {id(r) for r in mine} - {id(r) for r in fits}
                    rows = [r for r in rows if id(r) not in drop]
        return sorted(rows, key=lambda r: (tag(r, "variant:") is not None, tag(r, "storey:") is None, r["id"]))

    def storeys_of(a):
        """`[(word, height mm)]` of a building's storeys: lay.py's `storey_kinds`."""
        n, h = int(a.get("storeys") or 1), float(a.get("storey_h") or 3.2)
        g, top = float(a.get("ground_h") or 0) or h, float(a.get("top_h") or 0) or h
        return [("ground", int(round(g * 1000))) if k == 0 else ("top", int(round(top * 1000))) if k == n - 1
                else ("upper", int(round(h * 1000))) for k in range(n)]
    kit_kinds = {}                               # taxon -> the kinds of storey its buildings have
    for f in features:
        if f["kind"] == "building" and f["attrs"].get("kit"):
            kit_kinds.setdefault(f["attrs"]["kit"], set()).update(storeys_of(f["attrs"]))
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
    start = len(out)
    refused, thicks = {}, {}
    def kinds_of(taxon):
        """The kinds of storey a kit is judged for: its buildings', else one for each height its
        untagged bays have (no storey says which pieces go together, so their heights do)."""
        if kit_kinds.get(taxon):
            return sorted(kit_kinds[taxon], key=str)
        highs = sorted({r["bounds"][5] - r["bounds"][2] for r in by_taxon[taxon]
                        if "role:wall" in (r.get("tags") or []) and r.get("bounds") and tag(r, "variant:") is None
                        and tag(r, "storey:") is None})
        return [(None, h) for h in highs] if len(highs) > 1 else [(None, None)]
    for taxon, word, height in [(t_, w, h) for t_ in sorted(by_taxon, key=str) for w, h in kinds_of(t_)]:
        rows = storey_rows(by_taxon[taxon], word, height)
        role = {}
        walls = [r for r in rows if "role:wall" in (r.get("tags") or []) and r.get("bounds")]
        if word is not None and not any(tag(r, "variant:") is None for r in walls) and any(
                "role:wall" in (r.get("tags") or []) and tag(r, "variant:") is None for r in by_taxon[taxon]):
            names = sorted(f["name"] for f in features if f["kind"] == "building" and f["attrs"].get("kit") == taxon
                           and (word, height) in storeys_of(f["attrs"]))
            out.append(("warning", "kit_bay", str(taxon), f"`{taxon}` has no bay for the {word} storeys of "
                        + ", ".join("`" + n + "`" for n in names) + ": its bays are tagged for other storeys",
                        f"give the kit a `role:wall` piece with no `storey:` tag, or one tagged `storey:{word}`",
                        {"taxon": str(taxon), "storey": word, "buildings": names}, None))
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
        thicks.setdefault(taxon, {})[wall["id"]] = T
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
        def mm(r):
            """A wall piece's sizes in whole mm, as lay.py's `kit_pieces` reads them."""
            v, b = number((r.get("params") or {}).get("pitch")), r["bounds"]
            return {"pitch": int(round(v * 1000)) if v else b[3] - b[0], "length": b[3] - b[0],
                    "thickness": b[4] - b[1], "height": b[5] - b[2]}
        for r in walls:
            words = [t[8:] for t in r["tags"] if t.startswith("variant:")]
            if r is wall or not words:
                continue
            got, want = mm(r), mm(wall)
            bad = [f"{k} {got[k] / 1000:g} m for `{wall['id']}`'s {want[k] / 1000:g} m" for k in got
                   if abs(got[k] - want[k]) > 5]
            differs = [k for k in got if abs(got[k] - want[k]) > 5]
            shift = max(abs(x - y) for x, y in zip(r["bounds"], wall["bounds"]))
            if not bad and shift > 5:
                bad, differs = [f"its box lies {shift / 1000:g} m off `{wall['id']}`'s about its origin"], ["box"]
            if "wall" in words:
                bad, differs = bad + ["`wall` is the bay's own word, never a variant's"], differs + ["word"]
            # A variant is at fault when no kind of storey that reads it can lay it: a 3 m window
            # bay is no fault of a kit whose ground storey is 4 m.
            refused.setdefault((taxon, r["id"]), []).append(
                ("warning", "kit_variant", r["id"],
                 f"`{r['id']}` (`variant:{words[0]}`) cannot stand for the bay: " + ", ".join(bad),
                 "make the variant the bay's pitch and the bay's box: it stands where a bay stood",
                 {"asset": r["id"], "variant": words[0], "bay": wall["id"], "differs": differs}, None) if bad else None)
        thick = {r["id"]: (r["bounds"][4] - r["bounds"][1]) / 1000.0 for n, r in role.items()
                 if n in ("wall", "filler", "door")}
        high = {r["id"]: (r["bounds"][5] - r["bounds"][2]) / 1000.0 for n, r in role.items()
                if "role:" + n in wall_roles and n != "door"}       # a door piece may be a leaf or a lintel
        deck = []
        if "floor" in role:
            b = role["floor"]["bounds"]
            w, d = (b[3] - b[0]) / 1000.0, (b[4] - b[1]) / 1000.0
            tp = pitch_of(role["floor"])
            if abs(w - d) > slack or tp <= 0 or off(m, tp) > slack:
                deck.append((role["floor"], f"`{role['floor']['id']}` is {w:g} by {d:g} m at a {tp:g} m pitch: a floor "
                             f"tile is square, and the kit's smallest wall piece ({m:g} m) is a whole number of tiles, "
                             "or a floor sized to the walls is not whole tiles"))
        slope, gable = role.get("roof_slope"), role.get("roof_gable")
        for r in (slope, gable):
            span = number((r.get("params") or {}).get("span")) if r else None
            if r is None:
                continue
            long = (r["bounds"][4] - r["bounds"][1]) / 1000.0 if r is slope else (r["bounds"][3] - r["bounds"][0]) / 1000.0
            if span is None or abs(long - span) > slack:
                deck.append((r, f"`{r['id']}` measures {long:g} m from eave to ridge and states "
                             + ("no `params.span`" if span is None else f"a span of {span:g} m")))
        if slope and gable:
            hs, hg = ((r["bounds"][5] - r["bounds"][2]) / 1000.0 for r in (slope, gable))
            tg = (gable["bounds"][4] - gable["bounds"][1]) / 1000.0
            if abs(hs - hg) > slack:
                deck.append((gable, f"`{gable['id']}` rises {hg:g} m and `{slope['id']}` {hs:g} m: the gable does not "
                             "meet the roof"))
            if abs(tg - T) > slack:
                deck.append((gable, f"`{gable['id']}` is {tg:g} m thick on walls {T:g} m thick"))
            ss, sg = (number((r.get("params") or {}).get("span")) for r in (slope, gable))
            if ss is not None and sg is not None and abs(ss - sg) > slack:
                deck.append((gable, f"`{gable['id']}` is made for a span of {sg:g} m and `{slope['id']}` for {ss:g} m: "
                             "no roof takes both"))
        elif slope or gable:
            r = slope or gable
            deck.append((r, f"`{taxon}` has `{r['id']}` and no " + ("`role:roof_gable`" if slope else "`role:roof_slope`")
                         + " piece: a gable roof needs both"))
        for r, said in deck:
            out.append(("warning", "kit_deck", r["id"], said,
                        "correct the piece or what the pack states of it (lay.py rules 9 and 10)",
                        {"asset": r["id"], "taxon": taxon}, None))
        if taxon not in ells and "corner_in" in role:       # an inside corner no building has is not held to a storey
            high.pop(role["corner_in"]["id"], None)
        for what, sizes in (("thick", thick), ("high", high)):
            if max(sizes.values()) - min(sizes.values()) > slack:
                out.append(("warning", "kit_uneven", wall["id"], f"the wall pieces of `{taxon}` are not one size: "
                            + ", ".join(f"`{i}` {v:g} m {what}" for i, v in sorted(sizes.items())),
                            "make them one thickness and one storey high",
                            {"taxon": taxon, "measure": what, "pieces": sorted(sizes),
                             "sizes": [sizes[i] for i in sorted(sizes)]}, None))
    for key in sorted(refused, key=str):
        if all(refused[key]):
            out.append(refused[key][0])
    for taxon in sorted(thicks, key=str):
        if max(thicks[taxon].values()) - min(thicks[taxon].values()) > slack:
            sizes = thicks[taxon]
            out.append(("warning", "kit_uneven", min(sizes), f"the bays of `{taxon}`'s storeys are not one thickness: "
                        + ", ".join(f"`{i}` {v:g} m thick" for i, v in sorted(sizes.items())),
                        "make the bay of every storey one thickness: walls stack flush or not at all",
                        {"taxon": taxon, "measure": "thick", "pieces": sorted(sizes),
                         "sizes": [sizes[i] for i in sorted(sizes)]}, None))
    for taxon in sorted(by_taxon, key=str):      # what the kit states, against its pieces (rule 10)
        rows = [r for r in by_taxon[taxon] if r.get("bounds")]
        bays = [r for r in rows if "role:wall" in (r.get("tags") or [])]
        if not bays:
            continue
        tables = []
        for r in by_taxon[taxon]:
            t = r.get("shared") or {}
            if t and t not in tables:
                tables.append(t)
        plain = [r for r in bays if tag(r, "variant:") is None] or bays

        def pitch_(r):
            v = number((r.get("params") or {}).get("pitch"))
            return v if v is not None else (r["bounds"][3] - r["bounds"][0]) / 1000.0

        def thick_(r):
            return (r["bounds"][4] - r["bounds"][1]) / 1000.0

        def high_(r):
            return (r["bounds"][5] - r["bounds"][2]) / 1000.0
        if not tables:
            found = {"module": pitch_(plain[0]), "wall": thick_(plain[0]),
                     "storey": sorted({high_(r) for r in plain})}
            out.append(("info", "kit_unstated", str(taxon), f"`{taxon}` states nothing its pieces share: its "
                        f"module ({found['module']:g} m), its wall's thickness ({found['wall']:g} m) and its storey "
                        f"({', '.join(f'{h:g}' for h in found['storey'])} m) are worked out of its bay",
                        f"state them once in the pack: `[taxon.\"{taxon}\"]` with `defaults = {{ shared = {{ module = "
                        f"{found['module']:g}, wall = {found['wall']:g}, storey = {found['storey'][0]:g} }} }}`",
                        {"taxon": str(taxon), **found}, None))
            continue
        if len(tables) > 1:                       # `lay` refuses it (`params`): nothing to hold the pieces to
            continue
        st = tables[0]
        wall_, module_ = number(st.get("wall")), number(st.get("module"))
        storeys_ = st.get("storey")
        storeys_ = [number(v) for v in (storeys_ if isinstance(storeys_, list) else [storeys_])] if storeys_ else []
        storeys_ = [v for v in storeys_ if v]
        for r in rows:
            roles = [t[5:] for t in (r.get("tags") or []) if t.startswith("role:")]
            said = []
            if wall_ and any(x in ("wall", "filler", "door") for x in roles) and abs(thick_(r) - wall_) > slack:
                said.append((f"is {thick_(r):g} m thick where the kit states walls of {wall_:g} m", "wall", thick_(r), wall_))
            if module_ and "wall" in roles and abs(pitch_(r) - module_) > slack:
                said.append((f"takes {pitch_(r):g} m of wall where the kit states a module of {module_:g} m",
                             "module", pitch_(r), module_))
            if module_ and "filler" in roles and pitch_(r) > 0 and off(module_, pitch_(r)) > slack:
                said.append((f"takes {pitch_(r):g} m of wall, of which the kit's module ({module_:g} m) is no "
                             "whole number", "module", pitch_(r), module_))
            parapet_ = number(st.get("parapet"))
            if tag(r, "storey:") == "parapet":   # the wall round a flat roof is no storey: held to its own height
                if parapet_ and any(x in ("wall", "filler", "corner") for x in roles) and abs(high_(r) - parapet_) > slack:
                    said.append((f"is {high_(r):g} m high where the kit states a parapet of {parapet_:g} m",
                                 "parapet", high_(r), parapet_))
            elif storeys_ and any(x in ("wall", "filler", "corner") for x in roles) and min(
                    abs(high_(r) - h) for h in storeys_) > slack:
                said.append((f"is {high_(r):g} m high where the kit states a storey of "
                             + " or ".join(f"{h:g}" for h in storeys_) + " m", "storey", high_(r),
                             min(storeys_, key=lambda h: abs(high_(r) - h))))
            for text, fact, got, want in said:
                out.append(("warning", "kit_stated", r["id"], f"`{r['id']}` {text}",
                            f"correct the piece, or what `{taxon}` states in `shared.{fact}`",
                            {"asset": r["id"], "taxon": str(taxon), "fact": fact, "piece": got, "stated": want}, None))
    seen, kept = set(), []
    for rec in out[start:]:                      # one kit read for several kinds of storey says a fault once
        if (rec[1], rec[2], rec[3]) not in seen:
            seen.add((rec[1], rec[2], rec[3]))
            kept.append(rec)
    out[start:] = kept
    stacked = {}
    for f in sorted((f for f in features if f["kind"] == "building"), key=lambda f: f["name"]):
        a = f["attrs"]
        if not a.get("kit") or not (int(a.get("storeys") or 1) >= 2 or a.get("ground_h") or a.get("top_h")
                                    or a.get("roof") in ("flat", "gable")):
            continue
        roles = ("wall", "filler", "corner") + (("corner_in",) if turns_in(f["ring"]) else ())
        for word, height in sorted(set(storeys_of(a))):
            rows = storey_rows(by_taxon.get(a["kit"], []), word, height)
            for name in roles:
                r = next((r for r in rows if "role:" + name in (r.get("tags") or []) and r.get("bounds")), None)
                if r is not None and abs(r["bounds"][5] - r["bounds"][2] - height) > p["pitch_slack"] * 1000:
                    stacked.setdefault((r["id"], height), [r, []])[1].append(f["name"])
    for (_, height), (r, names) in sorted(stacked.items()):
        high, storey_h = (r["bounds"][5] - r["bounds"][2]) / 1000.0, height / 1000.0
        names = sorted(set(names))
        how = (f"it stands {high - storey_h:g} m into the storey above" if high > storey_h
               else f"it stops {storey_h - high:g} m short of the storey above")
        out.append(("warning", "kit_height", r["id"], f"`{r['id']}` is {high:g} m high on the "
                    f"{storey_h:g} m storeys of {', '.join('`' + n + '`' for n in names)}: {how}",
                    "make the piece the storey's height, or the storey the piece's (`storey_h`, `ground_h`, `top_h`)",
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
                       ("doorway", ["sill_z"]), ("footpath", []),
                       ("building", ["kit", "storey_h", "storeys", "ground_h", "top_h", "roof"])):
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
        t = step.pack(select="@" + taxon, columns=["bounds", "params", "shared"])
        for r, i in enumerate(_list(t.get("id", []))):
            tags = _json(_list(t["tags"])[r]) if "tags" in t else []
            rows.append({"id": i, "taxon": taxon,
                         "bounds": [int(v) for v in _list(_list(t["bounds"])[r])] if "bounds" in t else None,
                         "params": (_json(_list(t["params"])[r]) or {}) if "params" in t else {},
                         "shared": (_json(_list(t["shared"])[r]) or {}) if "shared" in t else {},
                         "tags": tags if isinstance(tags, list) else [tags]})
    return rows


def read_pieces(step, taxons, kit_rows):
    """Placed instances of the kits' assets that are not door leaves, with their asset's bounds."""
    bounds = {r["id"]: r["bounds"] for r in kit_rows
              if r.get("bounds") and not any(t in LIES_FLAT for t in (r.get("tags") or []))}
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
