"""The review library's reach lint: can a declared body walk between the places a world states?

Opt-in, and driven only by what the world writes (TODO "Design the reach lint properly"; R13, M3
"reach as a library lint"): a movement profile, and the places that must (or must not) be reachable
from each other. Nothing is assumed: a profile without one of its keys is an error, not a default.
A lint step through the SDK, as a worker (protocol.md 8): only a worker may ask the engine for the
kind surfaces and the meetings, so a plain command walks the terrain only and says `terrain_only`.
A world restates it (a library declares no step, protocol.md 1):

    [step.reach]
    when    = "lint"
    worker  = true
    command = ["python", "-m", "viasdk", "lib:review/reach.py"]

    [step.reach.params.profiles.walker]       # metres; every key required
    grade = 0.5      # steepest rise over run walked between neighbouring posts
    step  = 0.3      # largest rise taken at once whatever the run (a kerb, a stair's riser)
    width = 0.8      # clear width the body needs: a post nearer than width/2 to an obstacle is shut
    wade  = 0.3      # deepest water walked through (water level less ground)
    reach = 2        # how far from a place the body may stand and count as at it

    [step.reach.params]
    reach  = [{ profile = "walker", from = "quay_head", to = [{ mount = "door", assets = ["@building/house"] }] }]
    sealed = [{ profile = "walker", from = "quay_head", to = ["look_harbour"] }]
    links  = [{ profile = "walker", a = "<place>", b = "<place>", reason = "a ladder" }]
    waive  = [{ to = "<place>", reason = "why this one may stay cut off" }]

A **place** is written as one of:
  - a point feature's name (a bookmark, a named point such as `quay_head`);
  - `<path>.start` or `<path>.end`, a path feature's end;
  - `<instance>.<mount>`, an instance's mount (assets.md 2.7) taken to the world by numbers.md 5.1;
  - `{ mount = "<name>", assets = [<asset ids or @taxon paths>] }`: that mount of every instance of
    those assets that has it, each its own place, in instance name order; `mount` may be a list
    (`["door", "side"]`: a corner house's two doors), each instance's mounts in the order written.

**What it reads, and nothing else** (honest about its view): the finished build's terrain
(`role:surface`), every kind surface at every post (`surfaces_at`, surfaces.md 2.1: a deck, a
floor, a ramp), the meetings (`meetings`, 2.4), the water (`tag:water_level`; without it nothing
is wet), every instance with its `z` and the pack's `footprint`, `solidity`, `sink` and `mounts`,
and the features. Ways are not walked differently from the land they lie on: a way too steep for
the profile is reported as a stop on that way (`data.on`). Not read: the terrain's cut (surfaces.md
1.3; a cut quad is still walked as ground: the SDK serves no cut, TODO).

**The walk graph** (surfaces.md 4.4, the list a walk may rely on). A node is a surface at a post
(protocol.md 2: `grid`): every post of the terrain, and every post inside a kind surface's bound
for that surface, at the z `surfaces_at` gives, so a deck over a river is a second node over the
bed's (item 5). A node is shut, in this order, when it lies inside (or on) the footprint of a solid
instance that stands on that node's surface (`why = "instance"`, `by` its name; the instance's
surface is the one at its origin whose z is its z plus its `sink`, scaled; a `passable` footprint,
a gate or an arcade, is ground, item 4), when the water over it is deeper than `wade` (`"water"`,
`value` the depth; on a kind surface only where water stands over the ground at that post, so a
floor under a dry hill is dry and a deck is read at its own z over the river), or when such a node
of its own surface, or a post off a kind surface's bound (its edge, a wall: item 3), lies nearer
than `width / 2` (`"narrow"`, `by` what is near, `value` the metres). Two open neighbours on one
surface (eight neighbours; a diagonal only when both posts beside it are open on that surface) are
joined when the rise between them is at most `step`, or rise over run at most `grade` (`"steep"`,
`value` the grade). The graph is undirected, and a diagonal climbs at rise over its own run, so a
walk may zigzag up a face too steep straight on, as a switchback does. A `width` shuts nodes only
when `width / 2` is a post spacing or more: below that, the posts cannot say how wide a gap is.

Two surfaces are joined only at a meeting that holds (item 3): gap 0 and, where both sides have a
connector, the same one (surfaces.md 3.4; facing is not read: a joint turned wrong is still a
joint). Each side's surface is the one at the meeting's point whose z is that side's (to the
millimetre); its node is where the body stands at the point on that surface (as a place, below);
the two are joined when the profile takes the rise between them (`step`, or `grade` over the run
between their posts), so a meeting whose posts differ by a kerb the body cannot take stays cut. A
`link` joins its two places whatever lies between (a ladder, a ferry; no condition yet). A place
stands at the nearest post within `reach` with an open node on its surface (ties: post order): an
instance's mount on the instance's surface, a path end on its path's one kind surface or else the
terrain (surfaces.md 3.3); a point feature (the SDK gives no z for it) on the lowest open surface
at the nearest post with one.

**What it says.** Codes are `step.<id>.<code>` (protocol.md 3.3); records in a fixed order: profiles,
then each expectation as written, each of its `to` places in order. A record about a place has the
place's name as subject (diagnostics.md 1.1 maps a place or produced name to its entity or producer);
`profile`, `unknown_place`, `waiver_unused` and `walked` have the step.

| code          | severity | when                                                                 |
| profile       | error    | a profile named nowhere, or missing a key; its expectations are skipped |
| unknown_place | error    | a place that names nothing in the build                              |
| stranded      | warning  | no open post within `reach` of a place; `data.why` for its nearest post |
|               |          | (a `sealed` place with nowhere to stand holds, and says nothing)      |
| unreached     | warning  | a `reach` pair not joined; at the post of the `from` side nearest the  |
|               |          | `to` place, with why the next post toward it is not walked            |
| not_sealed    | warning  | a `sealed` pair joined; at the `to` place, with `data.narrowest`:    |
|               |          | where the fewest-posts route between them passes nearest a shut     |
|               |          | post or the edge (`at`, `clear` metres, `by`), within 20 m          |
| waived        | info     | a record a `waive` entry names (by its `to` place), with its reason  |
| waiver_unused | warning  | a `waive` entry no record matched                                    |
| walked        | info     | one per profile used: nodes (`posts`), open nodes, pieces, pairs    |
|               |          | reached, kind `surfaces` walked and meetings `joins`ed               |
| unmeasured    | warning  | no single `role:surface` field: nothing is walked                    |
| terrain_only  | info     | a plain command: no kind surface or meeting could be asked           |

`unreached` gives `data.surface` when its stop is on a kind surface.

Deterministic: every loop runs in node, post or name order, and positions are integer millimetres until
the record. A mount's place uses Python's `math.sin` and `math.cos` for the instance's facing, which
are not numbers.md 2's bits off the four axis facings; the place then snaps to a post, so a
last-bit difference moves nothing unless a mount lies on the half-way line between posts.

Library code imports only viasdk and the standard library (R19b).
"""

import json
import math

PROFILE_KEYS = ("grade", "step", "width", "wade", "reach")

EXPLAIN = {
    "profile": "A reach expectation names a movement profile that `params.profiles` does not declare, "
    "or the profile lacks a key. Every key is required (grade, step, width, wade, reach): the lint "
    "assumes no body.",
    "unknown_place": "A place an expectation or link names is not in the build: no point feature, no "
    "path end, no instance mount by that name, or a mount group that selects no instance.",
    "stranded": "No post the profile may stand on lies within its `reach` of the place. `data.why` "
    "says why the nearest post is shut: inside an instance, under water deeper than `wade`, or "
    "nearer than `width / 2` to either.",
    "unreached": "The body cannot walk from the `from` place to this one over the build's ground. The "
    "position is where its walk comes nearest; `data.why` is why the next post toward the place is "
    "not walked: `steep` (rise over run, above `grade` and a rise above `step`), `water`, "
    "`instance`, `narrow`, or `edge` of the world. `data.on` names a way the stop lies on.",
    "not_sealed": "A place the world states must stay cut off from `from` is reachable.",
    "waived": "A reach record the world waived, with the reason it gave.",
    "waiver_unused": "A `waive` entry that matched no record: the place is reached now, or the name "
    "is wrong. Remove it.",
    "walked": "The walk graph of one profile: posts, open posts, pieces (connected parts), and how "
    "many stated pairs were reached.",
    "unmeasured": "The ground could not be read: `role:surface` selects no field, or several.",
    "terrain_only": "The step could not ask the engine for kind surfaces and meetings (a plain "
    "command asks nothing: protocol.md 8), so only the terrain is walked: a deck or a floor is not "
    "ground, and a way over a deck is cut where the deck begins.",
}

HINTS = {
    "profile": "Declare `[step.<id>.params.profiles.<name>]` with grade, step, width, wade and reach.",
    "unknown_place": "Name a point feature, `<path>.start`/`.end`, `<instance>.<mount>`, or "
    "`{ mount, assets }`.",
    "stranded": "Clear the ground around the place, or move the place where the body can stand.",
    "unreached": "Ease the ground at the stop (a gentler way, a pad), bridge the water, move the "
    "instance, or add a link (a ladder, a stair) and say why; or waive it with a reason.",
    "not_sealed": "Shut the way in (a wall, water, a steeper face), or drop the `sealed` statement.",
    "waived": "Informs only.",
    "waiver_unused": "Remove the `waive` entry.",
    "walked": "Informs only (R15).",
    "unmeasured": "Give the world one field with `role = \"surface\"`.",
    "terrain_only": "Declare the step with `worker = true` to walk decks and floors.",
}

WAYS = ("road", "lane", "trail", "street", "main_street")


def lint(step):
    say = Sayer(step)
    p = step.params or {}
    try:
        height = _rows(step.fields(["role:surface"])[0])
    except LookupError as e:
        say("unmeasured", "step." + str(getattr(step, "id", "reach")), f"Nothing is walked: `role:surface` could not be read ({e}).")
        return
    try:
        water = _rows(step.fields(["tag:water_level"])[0])
    except LookupError:
        water = None
    grid = Grid(step.grid)
    feats = _features(step)
    insts = _instances(step)
    facts = _pack(step)
    try:
        kinds, meets = _surfaces(step, grid, insts, facts)
    except NotImplementedError as e:          # a plain command asks the engine nothing
        say("terrain_only", "step." + str(getattr(step, "id", "reach")),
            f"Only the terrain is walked: {e}.")
        kinds, meets = [], []
    for rec in judge(grid, height, water, feats, insts, facts, p, step, kinds, meets):
        say(*rec)


def judge(grid, height, water, feats, insts, facts, params, step=None, kinds=(), meets=()):
    """Every record as (code, subject, message, position or None, data), in a fixed order.

    `kinds` is every kind surface walked, [(place name, {post: z})] in name order; `meets` every
    meeting, dicts with `x`, `y` (mm), `gap` (mm), `z_a`, `z_b`, `connector_a`, `connector_b` and
    `at`, the surfaces at its point as [(name, z)] ("" for the terrain)."""
    out = []
    me = "step." + (getattr(step, "id", None) or "reach")
    profiles = params.get("profiles") or {}
    groups = [("reach", e) for e in params.get("reach") or []]
    groups += [("sealed", e) for e in params.get("sealed") or []]
    links = params.get("links") or []
    waivers = [(str(w.get("to", "")), str(w.get("reason", ""))) for w in params.get("waive") or []]
    used = sorted({str(e.get("profile", "")) for _, e in groups} | {str(l.get("profile", "")) for l in links})
    good = {}
    for name in used:
        prof = profiles.get(name)
        if not isinstance(prof, dict):
            out.append(("profile", me,
                        f"No movement profile `{name}` is declared in params.profiles.", None,
                        {"profile": name}))
            continue
        lack = [k for k in PROFILE_KEYS if k not in prof]
        if lack:
            out.append(("profile", me,
                        f"Profile `{name}` lacks {', '.join(lack)}: no default is assumed.", None,
                        {"profile": name, "missing": lack}))
            continue
        good[name] = {k: float(prof[k]) for k in PROFILE_KEYS}
    owned = {}
    for name, _ in kinds:
        owned.setdefault(name.rsplit(".", 1)[0], []).append(name)
    places = Places(grid, feats, insts, facts, step, owned)
    obstacles = footprints(insts, facts)
    ways = [f for f in feats if f["kind"] in WAYS and f["geometry"] == 1]
    walks = {}
    for name in sorted(good):
        walks[name] = Walk(grid, height, water, obstacles, good[name], kinds)
    # Meetings that hold join the surfaces at their point (surfaces.md 4.4 item 3), in the order
    # the engine gives them (by `a`, then `b`).
    for m in meets:
        if not _holds(m):
            continue
        sides = []
        for z in (m["z_a"], m["z_b"]):
            sides.append([n for n, zz in m["at"] if abs(round(zz * 1000) - round(z * 1000)) <= 1])
        pairs = sorted({tuple(sorted((a, b))) for a in sides[0] for b in sides[1] if a != b})
        for w in walks.values():
            for a, b in pairs:
                w.meet((m["x"], m["y"]), a, b)
    # Links, then pieces
    unknown = set()
    for l in links:
        w = walks.get(str(l.get("profile", "")))
        if w is None:
            continue
        ends = []
        for side in ("a", "b"):
            got = places.resolve(l.get(side))
            if not got:
                _unknown(out, unknown, me, l.get(side))
            ends.append(got)
        if all(ends):
            for _, pa, sa in ends[0]:
                for _, pb, sb in ends[1]:
                    w.link(w.stand(pa, sa), w.stand(pb, sb))
    for w in walks.values():
        w.settle()
    tally = {n: [0, 0] for n in walks}
    stranded = set()
    hits = set()

    def emit(rec, to_name):
        for k, (wto, why) in enumerate(waivers):
            if wto == to_name:
                hits.add(k)
                code, subject, msg, pos, data = rec
                out.append(("waived", subject, f"Waived ({why}): {msg}", pos,
                            dict(data, waived=code, reason=why)))
                return
        out.append(rec)

    for mode, e in groups:
        pname = str(e.get("profile", ""))
        w = walks.get(pname)
        if w is None:
            continue
        froms = places.resolve(e.get("from"))
        if not froms:
            _unknown(out, unknown, me, e.get("from"))
            continue
        tos = []
        for t in e.get("to") or []:
            got = places.resolve(t)
            if not got:
                _unknown(out, unknown, me, t)
            tos.extend(got)
        for fname, fp, fs in froms:
            fpost = w.stand(fp, fs)
            if fpost is None:
                if (pname, fname) not in stranded:
                    stranded.add((pname, fname))
                    emit(_stranded(w, pname, fname, fp, fs), fname)
                continue
            for tname, tp, ts in tos:
                tally[pname][1] += 1
                tpost = w.stand(tp, ts)
                if tpost is None and mode == "sealed":
                    tally[pname][0] += 1        # nowhere to stand there: sealed holds
                    continue
                if tpost is None:
                    if (pname, tname) not in stranded:
                        stranded.add((pname, tname))
                        emit(_stranded(w, pname, tname, tp, ts), tname)
                    continue
                joined = w.piece(fpost) == w.piece(tpost)
                if mode == "reach":
                    if joined:
                        tally[pname][0] += 1
                        continue
                    stop, why = w.stop(fpost, tpost)
                    at = w.xy(stop)
                    gap = round(_dist(at, w.xy(tpost)) / 1000.0, 2)
                    data = {"profile": pname, "from": fname, "to": tname, "why": why[0],
                            "gap": gap}
                    if w.surf[stop]:
                        data["surface"] = w.names[w.surf[stop]]
                    if why[1] is not None:
                        data["value"] = why[1]
                    if why[2] is not None:
                        data["by"] = why[2]
                    on = _on_way(at, ways)
                    if on:
                        data["on"] = on
                    emit(("unreached", tname,
                          f"{pname} cannot walk from {fname} to {tname}: the walk stops "
                          f"{gap} m short, {_why_text(why)}{' on ' + on if on else ''}.",
                          _m(at), data), tname)
                else:
                    if not joined:
                        tally[pname][0] += 1
                        continue
                    data = {"profile": pname, "from": fname, "to": tname}
                    narrow = w.narrowest(fpost, tpost)
                    where = ""
                    if narrow is not None:
                        k, clear, by = narrow
                        at = _m(w.xy(k))
                        data["narrowest"] = {"at": at, "clear": clear, "by": by}
                        where = (f" The walked route is narrowest at ({at[0]}, {at[1]}), {clear} m "
                                 f"from {by}.")
                    emit(("not_sealed", tname,
                          f"{pname} can walk from {fname} to {tname}, which is stated sealed."
                          + where, _m(w.xy(tpost)), data), tname)
    for k, (wto, why) in enumerate(waivers):
        if k not in hits:
            out.append(("waiver_unused", me,
                        f"The waiver for {wto} ({why}) matched no record.", None, {"to": wto}))
    for name in sorted(walks):
        w = walks[name]
        n_open = sum(1 for c in w.cause if c is None)
        held, total = tally[name]
        surf = len(w.names) - 1
        on = (f" on the terrain and {surf} kind surface{'s' if surf != 1 else ''}, joined at "
              f"{w.joined} meeting{'s' if w.joined != 1 else ''}") if surf else ""
        out.append(("walked", me,
                    f"{name}: {n_open} of {len(w.cause)} posts open in {w.pieces} pieces{on}; "
                    f"{held} of {total} stated pairs hold.", None,
                    {"posts": len(w.cause), "open": n_open, "pieces": w.pieces, "held": held,
                     "pairs": total, "surfaces": surf, "joins": w.joined}))
    return out


def _unknown(out, seen, me, place):
    key = json.dumps(place, sort_keys=True)
    if key in seen:
        return
    seen.add(key)
    out.append(("unknown_place", me,
                f"`{_place_text(place)}` names no place in the build.", None,
                {"place": _place_text(place)}))


def _place_text(place):
    if isinstance(place, dict):
        return f"{{mount={place.get('mount')}, assets={place.get('assets')}}}"
    return str(place)


def _holds(m):
    """surfaces.md 3.4 as a walk reads it: the gap is 0 and, where both sides have a connector,
    they are the same. Facing is not read: a meeting turned wrong is still a joint to walk."""
    if int(m.get("gap", 0)) != 0:
        return False
    a, b = m.get("connector_a"), m.get("connector_b")
    return not (a and b and a != b)


def _stranded(w, pname, name, p, on=None):
    near = w.nearest_post(p, on)
    why = w.why_shut(near) if near is not None else ("edge", None, None)
    data = {"profile": pname, "place": name, "why": why[0]}
    if why[1] is not None:
        data["value"] = why[1]
    if why[2] is not None:
        data["by"] = why[2]
    return ("stranded", name,
            f"{pname} has nowhere to stand within reach of {name}: {_why_text(why)}.", _m(p), data)


def _why_text(why):
    kind, value, by = why
    if kind == "steep":
        return f"the next post is too steep (rise over run {value})"
    if kind == "water":
        return f"the water is {value} m deep"
    if kind == "instance":
        return f"blocked by {by}"
    if kind == "narrow":
        return f"too narrow: {by} is {value} m away"
    return "the world's edge"


def _on_way(p, ways):
    best = None
    for f in ways:
        half = f["attrs"].get("width", 0.0) * 500.0
        d = _dist_to_line(p, f["rings"][0][0])
        if d <= max(half, 1.0) and (best is None or (d, f["name"]) < best):
            best = (d, f["name"])
    return best[1] if best else None


NARROW_LOOK = 20.0                                    # metres round a route `narrowest` looks


class Walk:
    """The walk graph of one profile. A node is (surface, post): every post of the terrain is a node
    (node k is post k), and every post inside a kind surface's bound is one more node of that surface
    (surfaces.md 4.4 item 5: one node per surface and place, never "the highest surface here").
    ``cause[n]`` is None for an open node, or (why, value, by); ``label[n]`` the piece of an open node
    (-1 when shut). Two nodes of one surface are joined as two posts are; nodes of two surfaces only
    by a meeting that holds or a `link` (4.4 item 3)."""

    def __init__(self, grid, height, water, obstacles, prof, kinds=()):
        self.g = grid
        self.prof = prof
        nx, ny, r = grid.nx, grid.ny, grid.r
        n = nx * ny
        terrain = [v for row in height for v in row]
        self.h = list(terrain)
        self.post = list(range(n))
        self.surf = [0] * n
        self.names = [""] + [name for name, _ in kinds]
        self.index = {name: s for s, name in enumerate(self.names)}
        self.at = [None]
        self.stack = {}                       # post -> its kind nodes, lowest first (2.1's order)
        for s, (name, zs) in enumerate(kinds, start=1):
            mine = {}
            for k in sorted(zs):
                mine[k] = len(self.h)
                self.h.append(float(zs[k]))
                self.post.append(k)
                self.surf.append(s)
                self.stack.setdefault(k, []).append(mine[k])
            self.at.append(mine)
        for k in self.stack:
            self.stack[k].sort(key=lambda q: (self.h[q], self.names[self.surf[q]]))
        total = len(self.h)
        cause = [None] * total
        # Instances, in name order: the first name holding a node says it. An instance shuts the
        # nodes of the surface it stands on, and no other (4.4 item 4: a footprint on the terrain
        # under a deck does not shut the deck).
        for name, rings, on in obstacles:
            s = self.index.get(on or "", 0)
            xs = [q[0] for q in rings]
            ys = [q[1] for q in rings]
            i0 = max(0, -(-(min(xs) - grid.x0) // r))
            i1 = min(nx - 1, (max(xs) - grid.x0) // r)
            j0 = max(0, -(-(min(ys) - grid.y0) // r))
            j1 = min(ny - 1, (max(ys) - grid.y0) // r)
            for j in range(j0, j1 + 1):
                for i in range(i0, i1 + 1):
                    q = self.node(s, j * nx + i)
                    if q is not None and cause[q] is None and _in_ring(grid.post(i, j), rings):
                        cause[q] = ("instance", None, name)
        if water is not None:
            wade = prof["wade"]
            wl = [v for row in water for v in row]
            for q in range(total):
                if cause[q] is None:
                    k = self.post[q]
                    if q >= n and wl[k] - terrain[k] <= 0:
                        continue                  # no water over the ground here: a floor is dry
                    d = wl[k] - self.h[q]
                    if d > wade:
                        cause[q] = ("water", round(d, 3), None)
        half = prof["width"] * 500.0
        if half >= r:
            rad = int(half // r)
            offs = sorted(((di, dj) for di in range(-rad, rad + 1) for dj in range(-rad, rad + 1)
                           if 0 < (di * di + dj * dj) * r * r < half * half),
                          key=lambda o: (o[0] * o[0] + o[1] * o[1], o[1], o[0]))
            hard = list(cause)
            for q in range(total):
                if hard[q] is not None:
                    continue
                s = self.surf[q]
                i, j = grid.ij(self.post[q])
                for di, dj in offs:
                    ii, jj = i + di, j + dj
                    if not (0 <= ii < nx and 0 <= jj < ny):
                        continue
                    o = self.node(s, jj * nx + ii)
                    if o is None:                 # off a kind surface's bound: its edge, a wall
                        c, by = None, self.names[s]
                    elif hard[o] is not None:
                        c = hard[o]
                        by = c[2] if c[0] == "instance" else "water"
                    else:
                        continue
                    dist = round(math.sqrt(di * di + dj * dj) * r / 1000.0, 3)
                    cause[q] = ("narrow", dist, by)
                    break
        self.cause = cause
        self.extra = {}
        self.label = None
        self.pieces = 0
        self.joined = 0

    def node(self, s, k):
        """The node of surface index s at post k, or None when k lies off that surface."""
        return k if s == 0 else self.at[s].get(k)

    def nodes_at(self, k):
        return [k] + self.stack.get(k, [])

    def shut(self, q):
        return q is None or self.cause[q] is not None

    def joins(self, k, q, diag):
        """Whether open nodes k and q (neighbours on one surface) are joined; (False, grade) when
        too steep."""
        rise = abs(self.h[q] - self.h[k])
        if rise <= self.prof["step"]:
            return True, None
        run = self.g.r / 1000.0 * (math.sqrt(2.0) if diag else 1.0)
        grade = rise / run
        return grade <= self.prof["grade"], grade

    def neighbours(self, k):
        """The eight neighbours of node k on its own surface, each with whether it is a diagonal
        and the two nodes beside a diagonal (None where that post lies off the surface)."""
        nx, ny = self.g.nx, self.g.ny
        s = self.surf[k]
        p = self.post[k]
        i, j = p % nx, p // nx
        for di, dj in ((1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (-1, 1), (1, -1), (-1, -1)):
            ii, jj = i + di, j + dj
            if 0 <= ii < nx and 0 <= jj < ny:
                q = self.node(s, jj * nx + ii)
                if q is not None:
                    yield q, di != 0 and dj != 0, (self.node(s, j * nx + ii), self.node(s, jj * nx + i))

    def link(self, a, b):
        if a is None or b is None or a == b:
            return
        self.extra.setdefault(a, []).append(b)
        self.extra.setdefault(b, []).append(a)

    def meet(self, p, s_a, s_b):
        """Join surfaces s_a and s_b (names; "" the terrain) at the plan point p of a meeting that
        holds: each side's node is where the body stands at p on that surface (`stand`), and the two
        are joined when the profile takes the rise between them (`step`, or `grade` over the run
        between their posts). True when joined."""
        a = self.stand(p, s_a)
        b = self.stand(p, s_b)
        if a is None or b is None or a == b:
            return False
        rise = abs(self.h[a] - self.h[b])
        if rise > self.prof["step"]:
            run = _dist(self.g.post(*self.g.ij(self.post[a])), self.g.post(*self.g.ij(self.post[b]))) / 1000.0
            if run <= 0 or rise / run > self.prof["grade"]:
                return False
        self.link(a, b)
        self.joined += 1
        return True

    def settle(self):
        n = len(self.cause)
        label = [-1] * n
        pieces = 0
        for s in range(n):
            if self.cause[s] is not None or label[s] >= 0:
                continue
            label[s] = pieces
            todo = [s]
            while todo:
                k = todo.pop()
                for q, diag, (a, b) in self.neighbours(k):
                    if label[q] >= 0 or self.cause[q] is not None:
                        continue
                    if diag and (self.shut(a) or self.shut(b)):
                        continue
                    if self.joins(k, q, diag)[0]:
                        label[q] = pieces
                        todo.append(q)
                for q in self.extra.get(k, ()):
                    if label[q] < 0:
                        label[q] = pieces
                        todo.append(q)
            pieces += 1
        self.label = label
        self.pieces = pieces

    def piece(self, k):
        return self.label[k]

    def route(self, a, b):
        """The fewest-nodes walk from node a to node b over the joins (links and meetings count as
        one), as a list of nodes from a to b; None when they are not joined. Breadth first,
        neighbours in `neighbours` order, so the route is the same on every run."""
        if self.label[a] != self.label[b] or self.label[a] < 0:
            return None
        came = {a: None}
        todo = [a]
        for k in todo:
            if k == b:
                break
            nxt = []
            for q, diag, (s, t) in self.neighbours(k):
                if q in came or self.cause[q] is not None:
                    continue
                if diag and (self.shut(s) or self.shut(t)):
                    continue
                if self.joins(k, q, diag)[0]:
                    nxt.append(q)
            nxt += [q for q in self.extra.get(k, ()) if q not in came]
            for q in nxt:
                if q not in came:
                    came[q] = k
                    todo.append(q)
        out, k = [], b
        while k is not None:
            out.append(k)
            k = came[k]
        return out[::-1]

    def narrowest(self, a, b, limit_m=NARROW_LOOK):
        """Where the walked route from node a to node b passes nearest a shut node of its own
        surface (an instance, water, a narrow gap), a kind surface's edge, or the world's edge,
        within `limit_m`: (node, metres to it, what it is), the first such node along the route on a
        tie; None when no route, or nothing shut lies within `limit_m` of it."""
        path = self.route(a, b)
        if not path:
            return None
        g = self.g
        best = None
        for k in path:
            s = self.surf[k]
            i, j = g.ij(self.post[k])
            edge = min(i, j, g.nx - 1 - i, g.ny - 1 - j) * g.r / 1000.0
            q = self._ring_search(g.post(i, j), lambda p: self.shut(self.node(s, p)),
                                  limit_m * 1000.0)
            d, by = (edge, "edge") if edge <= limit_m else (math.inf, None)
            if q is not None:
                dq = _dist(g.post(i, j), g.post(*g.ij(q))) / 1000.0
                o = self.node(s, q)
                if o is None:
                    what = self.names[s]
                else:
                    c = self.cause[o]
                    what = c[2] if c[0] in ("instance", "narrow") and c[2] else c[0]
                if dq < d:
                    d, by = dq, what
            if by is not None and (best is None or d < best[1]):
                best = (k, round(d, 3), by)
        return best

    def _ring_search(self, p, ok, limit_mm=None):
        """The post satisfying ``ok`` nearest to the point p (mm); ties by post order."""
        g = self.g
        ci = min(max(round((p[0] - g.x0) / g.r), 0), g.nx - 1)
        cj = min(max(round((p[1] - g.y0) / g.r), 0), g.ny - 1)
        best = None
        ring = 0
        most = max(g.nx, g.ny)
        while ring <= most:
            if best is not None and (ring - 1) * g.r > math.sqrt(best[0]):
                break
            if limit_mm is not None and (ring - 1) * g.r > limit_mm:
                break
            for j in range(cj - ring, cj + ring + 1):
                if j < 0 or j >= g.ny:
                    continue
                step = 1 if abs(j - cj) == ring else 2 * ring
                for i in range(ci - ring, ci + ring + 1, max(step, 1)):
                    if i < 0 or i >= g.nx:
                        continue
                    k = j * g.nx + i
                    if not ok(k):
                        continue
                    q = g.post(i, j)
                    d2 = (q[0] - p[0]) ** 2 + (q[1] - p[1]) ** 2
                    if limit_mm is not None and d2 > limit_mm * limit_mm:
                        continue
                    if best is None or (d2, k) < best:
                        best = (d2, k)
            ring += 1
        return None if best is None else best[1]

    def _open_at(self, k, on):
        """The open node at post k on surface `on` (a name, "" the terrain), or with `on` None the
        lowest open node there; None when there is none."""
        if on is None:
            for q in self.nodes_at(k):
                if self.cause[q] is None:
                    return q
            return None
        s = self.index.get(on)
        if s is None:
            return None
        q = self.node(s, k)
        return q if q is not None and self.cause[q] is None else None

    def stand(self, p, on=None):
        """The node a body at the point p stands on: the nearest post within `reach` with an open
        node on the surface `on` ("" the terrain; None: any surface, the lowest open one there)."""
        k = self._ring_search(p, lambda k: self._open_at(k, on) is not None, self.prof["reach"] * 1000.0)
        return None if k is None else self._open_at(k, on)

    def nearest_post(self, p, on=None):
        """The node nearest the point p on the surface `on` (the terrain when None or unknown),
        open or not."""
        s = self.index.get(on or "", 0)
        k = self._ring_search(p, lambda k: self.node(s, k) is not None)
        return None if k is None else self.node(s, k)

    def why_shut(self, k):
        return self.cause[k]

    def xy(self, k):
        """The plan point (mm) of node k."""
        return self.g.post(*self.g.ij(self.post[k]))

    def stop(self, a, b):
        """Where the walk from node a comes nearest node b, and why the next node toward b is not
        walked: (node, (why, value, by))."""
        mine = self.label[a]
        target = self.xy(b)
        kpost = self._ring_search(target, lambda q: any(self.label[o] == mine for o in self.nodes_at(q)))
        k = next(o for o in self.nodes_at(kpost) if self.label[o] == mine)
        best = None
        for q, diag, (s, t) in self.neighbours(k):
            if self.label[q] == mine:
                continue
            d2 = _dist2(self.xy(q), target)
            if best is None or (d2, q) < best[:2]:
                best = (d2, q, diag, s, t)
        if best is None:
            return k, ("edge", None, None)
        _, q, diag, s, t = best
        if self.cause[q] is not None:
            return k, self.cause[q]
        if diag and (self.shut(s) or self.shut(t)):
            for side in (s, t):
                if side is not None and self.cause[side] is not None:
                    return k, self.cause[side]
            return k, ("edge", None, None)
        _, grade = self.joins(k, q, diag)
        return k, ("steep", round(grade, 3) if grade is not None else None, None)


class Places:
    """Resolves a written place to [(name, (x, y) mm, surface)]: one entry, or one per instance of
    a group. `surface` is the surface the place lies on, when the build says it: an instance's mount
    lies on the instance's surface, a path end on its path's one kind surface or else the terrain
    (surfaces.md 3.3), each "" for the terrain; None for a point feature, which stands on the lowest
    open surface at its nearest open post (`Walk.stand`)."""

    def __init__(self, grid, feats, insts, facts, step, owned=None):
        owned = owned or {}
        self.points = {}
        self.on = {}
        for f in feats:
            if f["geometry"] == 0:
                self.points[f["name"]] = tuple(f["rings"][0][0][0])
                self.on[f["name"]] = None
            elif f["geometry"] == 1:
                line = f["rings"][0][0]
                mine = owned.get(f["name"], [])
                on = mine[0] if len(mine) == 1 else ""
                for end, p in ((".start", line[0]), (".end", line[-1])):
                    self.points[f["name"] + end] = tuple(p)
                    self.on[f["name"] + end] = on
        self.insts = sorted(insts, key=lambda r: r["name"])
        self.by_name = {r["name"]: r for r in insts}
        self.facts = facts
        self.step = step

    def mount(self, inst, mount):
        m = self.facts.get(inst["asset"], {}).get("mounts", {}).get(mount)
        if m is None:
            return None
        return frame(inst, m["at"])

    def resolve(self, place):
        if isinstance(place, dict):
            mounts = place.get("mount", "")
            mounts = [str(m) for m in mounts] if isinstance(mounts, list) else [str(mounts)]
            assets = _asset_ids(self.facts, list(place.get("assets") or []))
            out = []
            for r in self.insts:
                if r["asset"] in assets:
                    for mount in mounts:
                        p = self.mount(r, mount)
                        if p is not None:
                            out.append((f"{r['name']}.{mount}", p, r.get("surface") or ""))
            return out
        place = str(place)
        if place in self.points:
            return [(place, self.points[place], self.on[place])]
        if "." in place:
            inst, mount = place.rsplit(".", 1)
            r = self.by_name.get(inst)
            if r is not None:
                p = self.mount(r, mount)
                if p is not None:
                    return [(place, p, r.get("surface") or "")]
        return []


def frame(inst, at):
    """numbers.md 5.1: a point written in an instance's frame (mm) to the world (mm)."""
    k = float(inst.get("scale", 1.0) or 1.0)
    m = int(inst.get("face", 0)) % 360000
    a = float(at[0]) * k
    b = float(at[1]) * k
    s, c = _sincos(m)
    return (int(inst["x"]) + _round(a * c + b * s), int(inst["y"]) + _round(-(a * s) + b * c))


def _sincos(m):
    exact = {0: (0.0, 1.0), 90000: (1.0, 0.0), 180000: (0.0, -1.0), 270000: (-1.0, 0.0)}
    if m in exact:
        return exact[m]
    t = math.radians(m / 1000.0)
    return math.sin(t), math.cos(t)


def _round(v):
    return int(round(v))  # Python rounds ties to even, as numbers.md 5.1 does


def footprints(insts, facts):
    """(instance name, footprint ring in world mm, the surface it stands on or None for the
    terrain) of every solid instance with a footprint, in name order."""
    out = []
    for r in sorted(insts, key=lambda r: r["name"]):
        f = facts.get(r["asset"])
        if not f or f.get("solidity", "solid") in ("none", "passable") or len(f.get("footprint") or []) < 3:
            continue
        out.append((r["name"], [frame(r, q) for q in f["footprint"]], r.get("surface")))
    return out


def _asset_ids(facts, entries):
    out = set()
    for e in entries:
        if e.startswith("@"):
            want = e[1:]
            out.update(a for a, f in facts.items()
                       if f.get("taxon") == want or str(f.get("taxon", "")).startswith(want + "/"))
        else:
            out.add(e)
    return out


class Sayer:
    """Sends each record, the first of each code with its ``explain``."""

    SEVERITY = {"profile": "error", "unknown_place": "error", "stranded": "warning",
                "unreached": "warning", "not_sealed": "warning", "waived": "info",
                "waiver_unused": "warning", "walked": "info", "unmeasured": "warning",
                "terrain_only": "info"}

    def __init__(self, step):
        self.step = step
        self.explained = set()

    def __call__(self, code, subject, message, position=None, data=None):
        kw = {}
        if data is not None:
            kw["data"] = data
        if position is not None:
            kw["position"] = [position[0], position[1]]
        if code not in self.explained:
            kw["explain"] = EXPLAIN[code]
            self.explained.add(code)
        getattr(self.step, self.SEVERITY[code])(code, subject, message, HINTS[code], **kw)


class Grid:
    """The world's posts (protocol.md 2: ``grid``)."""

    def __init__(self, g):
        self.x0, self.y0, self.x1, self.y1 = (int(v) for v in g["extent_mm"])
        self.r = int(g["resolution_mm"])
        self.nx = int(g["nx"])
        self.ny = int(g["ny"])

    def post(self, i, j):
        return (self.x0 + i * self.r, self.y0 + j * self.r)

    def ij(self, k):
        return (k % self.nx, k // self.nx)


def _in_ring(p, ring):
    """Inside or on the ring; exact in integers."""
    n = len(ring)
    wn = 0
    for k in range(n):
        a, b = ring[k], ring[(k + 1) % n]
        cross = (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0])
        if cross == 0 and min(a[0], b[0]) <= p[0] <= max(a[0], b[0]) \
                and min(a[1], b[1]) <= p[1] <= max(a[1], b[1]):
            return True
        if a[1] <= p[1] < b[1] and cross > 0:
            wn += 1
        elif b[1] <= p[1] < a[1] and cross < 0:
            wn -= 1
    return wn != 0


def _dist2(a, b):
    return (a[0] - b[0]) ** 2 + (a[1] - b[1]) ** 2


def _dist(a, b):
    return math.sqrt(_dist2(a, b))


def _dist_to_line(p, line):
    best = math.inf
    for a, b in zip(line, line[1:]):
        ex, ey = b[0] - a[0], b[1] - a[1]
        wx, wy = p[0] - a[0], p[1] - a[1]
        ee = ex * ex + ey * ey
        t = 0.0 if ee == 0 else max(0.0, min(1.0, (wx * ex + wy * ey) / ee))
        dx, dy = wx - t * ex, wy - t * ey
        best = min(best, math.sqrt(dx * dx + dy * dy))
    return best


def _m(p):
    return (p[0] / 1000.0, p[1] / 1000.0)


# Reading through the SDK.


def _rows(a):
    return a.tolist() if hasattr(a, "tolist") else a


def _list(col):
    return col.tolist() if hasattr(col, "tolist") else list(col)


def _features(step):
    """Every feature as name, kind, geometry, rings (pieces of rings of mm pairs), number attrs."""
    fl = step.features("*")
    names = _list(fl["name"])
    kinds = _list(fl["kind"])
    geometry = _list(fl["geometry"])
    pieces = _list(fl["pieces"])
    rings = _list(fl["rings"])
    starts = _list(fl["ring_starts"])
    coords = [tuple(int(v) for v in c) for c in _list(fl["coords"])]
    widths = (_list(fl["width?"]), _list(fl["width"])) if "width?" in fl else None
    out = []
    for f, name in enumerate(names):
        ps = []
        for p in range(pieces[f], pieces[f + 1]):
            ps.append([coords[starts[r]:starts[r + 1]] for r in range(rings[p], rings[p + 1])])
        attrs = {}
        if widths and widths[0][f] and isinstance(widths[1][f], (int, float)):
            attrs["width"] = float(widths[1][f])
        out.append({"name": name, "kind": kinds[f], "geometry": int(geometry[f]), "rings": ps,
                    "attrs": attrs})
    return out


def _instances(step):
    t = step.instances("*")
    n = len(_list(t["name"]))
    cols = {k: _list(t[k]) if k in t else None for k in ("name", "asset", "x", "y", "z", "face", "scale")}
    out = []
    for r in range(n):
        out.append({"name": cols["name"][r], "asset": cols["asset"][r], "x": int(cols["x"][r]),
                    "y": int(cols["y"][r]),
                    "z": float(cols["z"][r]) if cols["z"] is not None else None,
                    "face": int(cols["face"][r]) if cols["face"] is not None else 0,
                    "scale": float(cols["scale"][r]) if cols["scale"] is not None else 1.0})
    return out


def _surfaces(step, grid, insts, facts):
    """Through the SDK only (R19b): every kind surface at the posts, [(name, {post: z})] in name
    order, from one `surfaces_at` over every post; every meeting from `meetings`, with the surfaces
    at its point; and, on each instance, the surface it stands on (`surface`, None for the
    terrain): the surface at its origin whose z is its z plus its asset's `sink`, scaled (4.2).
    Raises NotImplementedError when the step cannot ask (a plain command)."""
    from viasdk import NotAWorker

    nx, ny = grid.nx, grid.ny
    xs = [grid.x0 + i * grid.r for _ in range(ny) for i in range(nx)]
    ys = [grid.y0 + j * grid.r for j in range(ny) for _ in range(nx)]
    if not hasattr(step, "surfaces_at"):
        raise NotImplementedError("the step has no `surfaces_at`")
    try:
        t = step.surfaces_at(xs, ys)
    except NotAWorker as e:
        raise NotImplementedError(f"`surfaces_at` is not answered ({e})")
    kinds = {}
    for k, name, z in zip(_list(t["point"]), _list(t["surface"]), _list(t["z"])):
        if "." in name:                       # `<feature>.<surface>`; the terrain is a field name
            kinds.setdefault(name, {})[int(k)] = float(z)
    kinds = sorted(kinds.items())
    meets = []
    if kinds:
        m = step.meetings()
        rows = len(_list(m.get("a", [])))
        col = {k: _list(m[k]) if k in m else [None] * rows for k in
               ("a", "b", "x", "y", "gap", "z_a", "z_b", "connector_a", "connector_b",
                "connector_a?", "connector_b?")}
        for r in range(rows):
            meets.append({"a": col["a"][r], "b": col["b"][r], "x": int(col["x"][r]),
                          "y": int(col["y"][r]), "gap": int(col["gap"][r]),
                          "z_a": float(col["z_a"][r]), "z_b": float(col["z_b"][r]),
                          "connector_a": col["connector_a"][r] if col["connector_a?"][r] else None,
                          "connector_b": col["connector_b"][r] if col["connector_b?"][r] else None,
                          "at": []})
        on = [(q["x"], q["y"]) for q in meets] + [(r["x"], r["y"]) for r in insts]
        if on:
            got = step.surfaces_at([p[0] for p in on], [p[1] for p in on])
            at = [[] for _ in on]
            for k, name, z in zip(_list(got["point"]), _list(got["surface"]), _list(got["z"])):
                at[int(k)].append((name if "." in name else "", float(z)))
            for q, here in zip(meets, at):
                q["at"] = here
            for r, here in zip(insts, at[len(meets):]):
                if r.get("z") is None:
                    continue
                f = facts.get(r["asset"], {})
                want = r["z"] + float(f.get("sink") or 0.0) * float(r.get("scale") or 1.0)
                best = min(here, key=lambda c: (abs(c[1] - want), c[0]), default=None)
                if best is not None and best[0] and abs(best[1] - want) <= 0.002:
                    r["surface"] = best[0]
    return kinds, meets


def _pack(step):
    """asset id -> {taxon, footprint (mm points), solidity, mounts {name: {at (mm), face}}}."""
    t = step.pack("*", ["footprint", "solidity", "mounts", "sink"])
    ids = _list(t.get("id", []))
    out = {}
    for r, a in enumerate(ids):
        def col(k, default):
            return _list(t[k])[r] if k in t else default
        fp = col("footprint", "[]")
        mounts = col("mounts", "{}")
        out[a] = {"taxon": col("taxon", ""),
                  "footprint": json.loads(fp) if isinstance(fp, str) and fp else (fp or []),
                  "solidity": col("solidity", "solid") or "solid",
                  "sink": float(col("sink", 0.0) or 0.0),
                  "mounts": json.loads(mounts) if isinstance(mounts, str) and mounts else (mounts or {})}
    return out
