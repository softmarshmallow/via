"""The review library's reach lint: can a declared body walk between the places a world states?

Opt-in, and driven only by what the world writes (TODO "Design the reach lint properly"; R13, M3
"reach as a library lint"): a movement profile, and the places that must (or must not) be reachable
from each other. Nothing is assumed: a profile without one of its keys is an error, not a default.
A lint step through the SDK, as a plain command or a worker (protocol.md 8). A world restates it (a
library declares no step, protocol.md 1):

    [step.reach]
    when    = "lint"
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
    those assets that has it, each its own place, in instance name order.

**What it reads, and nothing else** (honest about its view): the finished build's ground
(`role:surface`, its top surface only: a deck or a floor not in that field is not walked), the
water (`tag:water_level`; without it nothing is wet), every instance with the pack's `footprint`,
`solidity` and `mounts`, and the features. Ways are not walked differently from the land they
lie on: a way too steep for the profile is reported as a stop on that way (`data.on`).

**The walk graph.** The world's posts (protocol.md 2: `grid`). A post is shut, in this order, when it
lies inside (or on) the footprint of a solid instance (`why = "instance"`, `by` its name), when the
water over it is deeper than `wade` (`"water"`, `value` the depth), or when such a post lies nearer
than `width / 2` (`"narrow"`, `by` what is near, `value` the metres). Two open neighbours (eight
neighbours; a diagonal only when both posts beside it are open) are joined when the rise between
them is at most `step`, or rise over run at most `grade` (`"steep"`, `value` the grade). The graph
is undirected, and a diagonal climbs at rise over its own run, so a walk may zigzag up a face too
steep straight on, as a switchback does. A `width` shuts posts only when `width / 2` is a post
spacing or more: below that, the posts cannot say how wide a gap is. A `link` joins its two places whatever lies between (a ladder, a ferry; no
condition yet). A place stands at the nearest open post within `reach` (ties: post order).

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
| not_sealed    | warning  | a `sealed` pair joined; at the `to` place                            |
| waived        | info     | a record a `waive` entry names (by its `to` place), with its reason  |
| waiver_unused | warning  | a `waive` entry no record matched                                    |
| walked        | info     | one per profile used: posts, open posts, pieces, pairs reached       |
| unmeasured    | warning  | no single `role:surface` field: nothing is walked                    |

Deterministic: every loop runs in post or name order, and positions are integer millimetres until
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
    for rec in judge(grid, height, water, feats, insts, facts, p, step):
        say(*rec)


def judge(grid, height, water, feats, insts, facts, params, step=None):
    """Every record as (code, subject, message, position or None, data), in a fixed order."""
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
    places = Places(grid, feats, insts, facts, step)
    obstacles = footprints(insts, facts)
    ways = [f for f in feats if f["kind"] in WAYS and f["geometry"] == 1]
    walks = {}
    for name in sorted(good):
        walks[name] = Walk(grid, height, water, obstacles, good[name])
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
            for _, pa in ends[0]:
                for _, pb in ends[1]:
                    w.link(w.stand(pa), w.stand(pb))
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
        for fname, fp in froms:
            fpost = w.stand(fp)
            if fpost is None:
                if (pname, fname) not in stranded:
                    stranded.add((pname, fname))
                    emit(_stranded(w, grid, pname, fname, fp), fname)
                continue
            for tname, tp in tos:
                tally[pname][1] += 1
                tpost = w.stand(tp)
                if tpost is None and mode == "sealed":
                    tally[pname][0] += 1        # nowhere to stand there: sealed holds
                    continue
                if tpost is None:
                    if (pname, tname) not in stranded:
                        stranded.add((pname, tname))
                        emit(_stranded(w, grid, pname, tname, tp), tname)
                    continue
                joined = w.piece(fpost) == w.piece(tpost)
                if mode == "reach":
                    if joined:
                        tally[pname][0] += 1
                        continue
                    stop, why = w.stop(fpost, tpost)
                    at = grid.post(*grid.ij(stop))
                    gap = round(_dist(at, grid.post(*grid.ij(tpost))) / 1000.0, 2)
                    data = {"profile": pname, "from": fname, "to": tname, "why": why[0],
                            "gap": gap}
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
                    emit(("not_sealed", tname,
                          f"{pname} can walk from {fname} to {tname}, which is stated sealed.",
                          _m(grid.post(*grid.ij(tpost))),
                          {"profile": pname, "from": fname, "to": tname}), tname)
    for k, (wto, why) in enumerate(waivers):
        if k not in hits:
            out.append(("waiver_unused", me,
                        f"The waiver for {wto} ({why}) matched no record.", None, {"to": wto}))
    for name in sorted(walks):
        w = walks[name]
        n_open = sum(1 for c in w.cause if c is None)
        held, total = tally[name]
        out.append(("walked", me,
                    f"{name}: {n_open} of {len(w.cause)} posts open in {w.pieces} pieces; "
                    f"{held} of {total} stated pairs hold.", None,
                    {"posts": len(w.cause), "open": n_open, "pieces": w.pieces, "held": held,
                     "pairs": total}))
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


def _stranded(w, grid, pname, name, p):
    near = w.nearest_post(p)
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


class Walk:
    """The walk graph of one profile over the posts: ``cause[k]`` is None for an open post, or
    (why, value, by); ``label[k]`` the piece of an open post (-1 when shut)."""

    def __init__(self, grid, height, water, obstacles, prof):
        self.g = grid
        self.h = [v for row in height for v in row]
        self.prof = prof
        nx, ny, r = grid.nx, grid.ny, grid.r
        n = nx * ny
        cause = [None] * n
        # Instances, in name order: the first name holding a post says it.
        for name, rings in obstacles:
            xs = [q[0] for q in rings]
            ys = [q[1] for q in rings]
            i0 = max(0, -(-(min(xs) - grid.x0) // r))
            i1 = min(nx - 1, (max(xs) - grid.x0) // r)
            j0 = max(0, -(-(min(ys) - grid.y0) // r))
            j1 = min(ny - 1, (max(ys) - grid.y0) // r)
            for j in range(j0, j1 + 1):
                for i in range(i0, i1 + 1):
                    k = j * nx + i
                    if cause[k] is None and _in_ring(grid.post(i, j), rings):
                        cause[k] = ("instance", None, name)
        if water is not None:
            wade = prof["wade"]
            wl = [v for row in water for v in row]
            for k in range(n):
                if cause[k] is None:
                    d = wl[k] - self.h[k]
                    if d > wade:
                        cause[k] = ("water", round(d, 3), None)
        half = prof["width"] * 500.0
        if half >= r:
            rad = int(half // r)
            offs = sorted(((di, dj) for di in range(-rad, rad + 1) for dj in range(-rad, rad + 1)
                           if 0 < (di * di + dj * dj) * r * r < half * half),
                          key=lambda o: (o[0] * o[0] + o[1] * o[1], o[1], o[0]))
            hard = list(cause)
            for j in range(ny):
                for i in range(nx):
                    k = j * nx + i
                    if hard[k] is not None:
                        continue
                    for di, dj in offs:
                        ii, jj = i + di, j + dj
                        if 0 <= ii < nx and 0 <= jj < ny and hard[jj * nx + ii] is not None:
                            c = hard[jj * nx + ii]
                            by = c[2] if c[0] == "instance" else "water"
                            dist = round(math.sqrt(di * di + dj * dj) * r / 1000.0, 3)
                            cause[k] = ("narrow", dist, by)
                            break
        self.cause = cause
        self.extra = {}
        self.label = None
        self.pieces = 0

    def joins(self, k, q, diag):
        """Whether open posts k and q (neighbours) are joined; (False, grade) when too steep."""
        rise = abs(self.h[q] - self.h[k])
        if rise <= self.prof["step"]:
            return True, None
        run = self.g.r / 1000.0 * (math.sqrt(2.0) if diag else 1.0)
        grade = rise / run
        return grade <= self.prof["grade"], grade

    def neighbours(self, k):
        nx, ny = self.g.nx, self.g.ny
        i, j = k % nx, k // nx
        for di, dj in ((1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (-1, 1), (1, -1), (-1, -1)):
            ii, jj = i + di, j + dj
            if 0 <= ii < nx and 0 <= jj < ny:
                yield jj * nx + ii, di != 0 and dj != 0, (j * nx + ii, jj * nx + i)

    def link(self, a, b):
        if a is None or b is None:
            return
        self.extra.setdefault(a, []).append(b)
        self.extra.setdefault(b, []).append(a)

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
                    if diag and (self.cause[a] is not None or self.cause[b] is not None):
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

    def stand(self, p):
        return self._ring_search(p, lambda k: self.cause[k] is None, self.prof["reach"] * 1000.0)

    def nearest_post(self, p):
        return self._ring_search(p, lambda k: True)

    def why_shut(self, k):
        return self.cause[k]

    def stop(self, a, b):
        """Where the walk from post a comes nearest post b, and why the next post toward b is not
        walked: (post, (why, value, by))."""
        mine = self.label[a]
        target = self.g.post(*self.g.ij(b))
        k = self._ring_search(target, lambda q: self.label[q] == mine)
        best = None
        for q, diag, (s, t) in self.neighbours(k):
            if self.label[q] == mine:
                continue
            d2 = _dist2(self.g.post(*self.g.ij(q)), target)
            if best is None or (d2, q) < best[:2]:
                best = (d2, q, diag, s, t)
        if best is None:
            return k, ("edge", None, None)
        _, q, diag, s, t = best
        if self.cause[q] is not None:
            return k, self.cause[q]
        if diag and (self.cause[s] is not None or self.cause[t] is not None):
            return k, self.cause[s] if self.cause[s] is not None else self.cause[t]
        _, grade = self.joins(k, q, diag)
        return k, ("steep", round(grade, 3) if grade is not None else None, None)


class Places:
    """Resolves a written place to [(name, (x, y) mm)]: one entry, or one per instance of a group."""

    def __init__(self, grid, feats, insts, facts, step):
        self.points = {}
        for f in feats:
            if f["geometry"] == 0:
                self.points[f["name"]] = tuple(f["rings"][0][0][0])
            elif f["geometry"] == 1:
                line = f["rings"][0][0]
                self.points[f["name"] + ".start"] = tuple(line[0])
                self.points[f["name"] + ".end"] = tuple(line[-1])
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
            mount = str(place.get("mount", ""))
            assets = _asset_ids(self.facts, list(place.get("assets") or []))
            out = []
            for r in self.insts:
                if r["asset"] in assets:
                    p = self.mount(r, mount)
                    if p is not None:
                        out.append((f"{r['name']}.{mount}", p))
            return out
        place = str(place)
        if place in self.points:
            return [(place, self.points[place])]
        if "." in place:
            inst, mount = place.rsplit(".", 1)
            r = self.by_name.get(inst)
            if r is not None:
                p = self.mount(r, mount)
                if p is not None:
                    return [(place, p)]
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
    """(instance name, footprint ring in world mm) of every solid instance with a footprint, in
    name order."""
    out = []
    for r in sorted(insts, key=lambda r: r["name"]):
        f = facts.get(r["asset"])
        if not f or f.get("solidity", "solid") == "none" or len(f.get("footprint") or []) < 3:
            continue
        out.append((r["name"], [frame(r, q) for q in f["footprint"]]))
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
                "waiver_unused": "warning", "walked": "info", "unmeasured": "warning"}

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
    cols = {k: _list(t[k]) if k in t else None for k in ("name", "asset", "x", "y", "face", "scale")}
    out = []
    for r in range(n):
        out.append({"name": cols["name"][r], "asset": cols["asset"][r], "x": int(cols["x"][r]),
                    "y": int(cols["y"][r]),
                    "face": int(cols["face"][r]) if cols["face"] is not None else 0,
                    "scale": float(cols["scale"][r]) if cols["scale"] is not None else 1.0})
    return out


def _pack(step):
    """asset id -> {taxon, footprint (mm points), solidity, mounts {name: {at (mm), face}}}."""
    t = step.pack("*", ["footprint", "solidity", "mounts"])
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
                  "mounts": json.loads(mounts) if isinstance(mounts, str) and mounts else (mounts or {})}
    return out
