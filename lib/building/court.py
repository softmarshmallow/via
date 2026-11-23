"""The building library's court step: a walled yard and the halls in it, written about one axis.

A generate step, run once as a worker through the SDK. It reads every `court` feature and returns
`building` features, the yard and each hall, which `lay` then walls: `lay` runs over
`kind:building`, so the world declares it in a layer above this step (a step reads another
step's output only from above it, protocol.md 3.3), as for `row.py`. Design note: README.md, "A
court".

Why (K8 of docs/plan-kits.md, from examples/court). A compound is laid out about an axis: the
gate on it, the main hall across it facing the gate, and the side halls in pairs that face each
other across it. Written as four buildings by hand that was 32 corner coordinates, each a sum of
the kit's bay and post and the distances from the axis; the two side halls were the same
numbers reflected by the author, and nothing held them to it. A court is written in bays and in
distances from the gate and from the axis, and a side hall is written once.

Rules (ours):

1. **The axis.** The court's own point, `at`, is the middle of the gate, on the outer face of the
   yard's wall. `to` (two numbers, metres) is any other point on the axis, further into the
   court. Along the axis is `v`, measured from the gate; `u` is to the right of the way in.
   Every length below is exact in that frame, and each returned outline is rounded to the
   millimetre, so a court may be turned to any angle (what `lay` then does with a turned
   building is `lay`'s: lay.py rules 6 and 9).
2. **Sizes are the kit's.** A side of `n` bays is `n` bays and what stands at its two ends: in a
   kit laid by posts (lay.py rule 15: a `role:post` piece and a `role:wall` piece, neither tagged
   for a storey) `n * B + p`, the bay from one post's middle to the next and one post's width;
   in a kit of whole wall pieces `n * m + 2 * r`, its module (the stated `module`, else its
   bay's pitch) and its outside corner's reach (`params.reach`, else the stated `wall`, else the
   bay's thickness).
3. **The yard.** `yard = { kit, bays = [across, along], storey_h }`: the wall round the court,
   `across` bays wide and `along` deep, of its own kit, its front on the gate's line. It is
   returned as a building of no storeys, `<court>_yard`: its walls alone, on the ground
   (lay.py rule 18), `storey_h` high (else the kit's first stated `storey`, else its bay's
   height), and its one door, the gate, on the axis. A building, because a wall round an
   outline with a door in it is what `lay` already lays; of no storeys, because a storey is a
   floor, and a court's ground is the ground. A court with no `yard` returns none.
4. **A door on the axis.** A door stands on the middle of a front: the gate on the yard's, and
   each hall's door on its own. The kit's door piece takes whole bays (one, in a kit laid by
   posts), and the bays left must be the same number each side: a front of an even number of
   bays has a post or a joint on its middle, not a bay, and is `params` (error) with the two
   numbers of bays next to it that have one.
5. **The hall.** `hall = { bays = [across, deep], from, veranda, fill }` stands across the
   axis, its front `from` metres along the axis from the gate's line and facing the gate:
   `<court>_hall`, side 0 its front. `veranda` is how many bays deep a colonnade stands across
   its front (lay.py rule 16; 0 for none, and fewer than `deep`).
6. **Wings.** `wings = [{ bays = [along, deep], from, off, veranda, fill }]`: each is written
   once and returned twice, `<court>_w<i>_l` on the left of the way in and `<court>_w<i>_r` on
   the right, mirror images about the axis. Its front is `off` metres from the axis and faces
   it; its near end is `from` metres along the axis from the gate's line; it is `along` bays
   long beside the axis and `deep` bays deep away from it. Side 0 of each is its front. Round a
   counter-clockwise outline the left one's side 1 is its far end and the right one's its near
   end, so in the right one's `fill` sides 1 and 3 change places; and every `fill` entry of a
   wing is laid mirrored (`mirror = true`, lay.py rule 8), because a pattern read from one end
   of a side would come out turned about on the twin, not reflected.
7. **What is passed on.** `kit`, `storey_h`, `plinth`, `porch`, `walk_out` and `roof` of the
   court are the hall's and each wing's; `fill` is each one's own.
8. **What stands in what.** Each hall's outline is grown by its kit's eaves (`shared.eaves`,
   lay.py rule 17). Two grown outlines that overlap, or one that passes the inner face of the
   yard's wall, is `court_meets` (warning), naming the two and the metres: nothing else in the
   build knows that a roof stands beyond a building's outline. Both are still returned. Two
   halls whose own outlines overlap, or a hall whose outline passes the yard's inner face, are
   said the same way and the later one (the hall, then the wings in order, left before right)
   is not returned: `lay` would wall two buildings in one place.
9. **Level ground.** `ground = "level"`: the court returns a `plot` (kinds.toml), `<court>_plot`:
   the yard's outline (with no yard, the box round the halls) moved out `PLOT_APRON` (1.5 m) on
   every side, with `level` the mean of the land this step reads at every post inside it and
   at its corners, to the millimetre. `lay`, a layer above, then reads level ground under the
   wall and under every hall, so the halls' floors are one height and the wall's foot is on
   the ground all round; none of them needs a plot of its own (lay.py rule 19). The author
   writes the court's place once: without this the yard's four corners had to be worked out
   from `at`, `to` and the bays and written again as a pad (examples/realm, "Before"). The
   court's `max_cut` (3 m) and `grade` (0.3) are its plot's, passed on: the core's
   `authority.max_change`, naming the plot, is answered on the court. What holds: the plot is
   shaped with `order = 1` (kinds.toml), so a way in this layer or a layer above, ending at the
   gate or passing by, gives way to it, and the halls stand on the level they were reckoned
   from. What does not: an anchored kind, a second plot over the same ground, or anything
   ordered after it still overrides it; this step cannot see that, and `check` says it where it
   reaches a hall's wall (check.py rule 12, `plot_lost`). The yard's own ground away from a hall,
   and the yard wall (rule 18: it stands on the ground as it lies), are not judged. Any other
   word is `params`.

Codes are `step.<id>.<code>`: `params` (error: a court's attribute, or a kit with no bay),
`court_meets` (warning), `courts` (info, what was returned).

Library code imports only viasdk and the standard library (R19b).
"""

import json
import math

from viasdk import RpcError

FIT = 5              # mm two outlines may overlap and still be said to touch
PASSED = ("storey_h", "plinth", "porch", "walk_out", "roof")
ATTRS = ["to", "kit", "yard", "hall", "wings", "ground", "max_cut", "grade", *PASSED]
PLOT_APRON = 1500    # mm a court's plot reaches beyond its yard (rule 9)

EXPLAIN = {
    "params": "A court's attribute, or its kit, cannot be laid from. The message names it.",
    "court_meets": "Two halls of a court, each with the eaves its kit states, stand in one place, or one "
    "passes the yard's wall. A roof stands beyond its building's outline, and only the court knows what "
    "stands beside what. Move one of them: `from` along the axis, `off` away from it.",
    "courts": "What the step returned: the courts read and the buildings written.",
}


class ParamError(Exception):
    def __init__(self, key, message, hint):
        super().__init__(message)
        self.key, self.message, self.hint, self.court = key, message, hint, None


def kit_facts(rows, taxon):
    """What a court sizes a building by (rule 2), in mm, from a kit's pack rows: `bay`, `ends`
    (what stands at a side's two ends together), `posts` (laid by posts), `door` (the bays its
    door piece takes), `eaves`, `wall` (the wall's thickness, a post's in a kit laid by posts)
    and `storey` (its first stated storey, else its bay's height)."""
    def role(r, name):
        return f"role:{name}" in (r.get("tags") or [])

    def untagged(r):
        return not any(t.startswith("storey:") for t in r.get("tags") or [])

    def plain(r):
        return untagged(r) and not any(t.startswith("variant:") for t in r.get("tags") or [])
    rows = rows or []
    bays = sorted((r for r in rows if role(r, "wall") and r.get("bounds") and untagged(r)),
                  key=lambda r: (not plain(r), r["id"]))
    if not bays:
        raise ParamError("kit", f"the kit `{taxon}` has no `role:wall` piece", "name a kit with a bay")
    shared = next((r["shared"] for r in rows if r.get("shared")), {})
    if not isinstance(shared, dict):
        shared = {}

    def mm(v, key):
        if isinstance(v, bool) or not isinstance(v, (int, float)) or not math.isfinite(v) or v <= 0:
            raise ParamError("kit", f"the kit `{taxon}` states `{key} = {v!r}`, not a length above 0",
                             f"write `{key}` as metres in the kit's `defaults.shared`, or on the piece")
        return int(round(float(v) * 1000))

    def pitch(r):
        v = (r.get("params") or {}).get("pitch")
        return mm(v, "params.pitch") if v is not None else r["bounds"][3] - r["bounds"][0]
    bay = bays[0]
    post = next((r for r in sorted(rows, key=lambda r: r["id"]) if role(r, "post") and r.get("bounds") and untagged(r)),
                None)
    thick = mm(shared["wall"], "shared.wall") if "wall" in shared else bay["bounds"][4] - bay["bounds"][1]
    door = next((r for r in sorted(rows, key=lambda r: r["id"]) if role(r, "door") and r.get("bounds") and untagged(r)),
                None)
    storeys = shared.get("storey")
    storeys = storeys if isinstance(storeys, list) else [storeys] if storeys is not None else []
    out = {"taxon": taxon, "posts": post is not None, "eaves": mm(shared["eaves"], "shared.eaves") if "eaves" in shared else 0,
           "storey": mm(storeys[0], "shared.storey") if storeys else bay["bounds"][5] - bay["bounds"][2]}
    if post is not None:
        p = post["bounds"][3] - post["bounds"][0]
        out.update(bay=pitch(bay), ends=p, wall=p, door=1)
    else:
        module = mm(shared["module"], "shared.module") if "module" in shared else pitch(bay)
        corner = next((r for r in sorted(rows, key=lambda r: r["id"]) if role(r, "corner") and untagged(r)), None)
        reach = (corner.get("params") or {}).get("reach") if corner else None
        reach = mm(reach, "params.reach") if reach is not None else thick
        took = pitch(door) / module if door else 1
        if abs(took - round(took)) * module > FIT or round(took) < 1:
            raise ParamError("kit", f"the door piece of `{taxon}` takes {pitch(door) / 1000:g} m of wall, no whole "
                             f"number of its {module / 1000:g} m bays", "make the door piece whole bays long")
        out.update(bay=module, ends=2 * reach, wall=thick, door=int(round(took)))
    return out


def _whole(v, lo=1):
    """`v` as a whole number of at least `lo`, or None. A number read through the engine is a
    float, so 3.0 is a whole number."""
    if isinstance(v, bool) or not isinstance(v, (int, float)) or not math.isfinite(v) or v != int(v) or v < lo:
        return None
    return int(v)


def _mm(v, lo=0.0):
    """Metres as mm, or None where `v` is no number of at least `lo`."""
    if isinstance(v, bool) or not isinstance(v, (int, float)) or not math.isfinite(v) or v < lo:
        return None
    return float(v) * 1000.0


def _bays(table, key, what, hint):
    """`table.bays` as two whole numbers of bays."""
    v = table.get("bays")
    if not isinstance(v, list) or len(v) != 2 or any(_whole(x) is None for x in v):
        raise ParamError(key, f"{what} has `bays = {v!r}`", hint)
    return _whole(v[0]), _whole(v[1])


def side(n, facts):
    """mm of a side of `n` bays (rule 2)."""
    return n * facts["bay"] + facts["ends"]


def on_axis(n, facts, key, what):
    """Rule 4: the door on the middle of a front of `n` bays, or `params`. The door takes
    `facts["door"]` whole bays, and what is left must be the same whole number of bays each
    side of it."""
    g = facts["door"]
    took = "a bay" if g == 1 else f"{g} bays"
    if n < g:
        raise ParamError(key, f"{what} is {n} bay{'s' if n != 1 else ''} across, and the kit's door takes {took}: "
                         "the door is wider than the front", f"make it {g} or {g + 2} bays across")
    if (n - g) % 2:
        middle = ("a post" if facts["posts"] else "a joint between two bays") if n % 2 == 0 else "a bay"
        raise ParamError(key, f"{what} is {n} bays across, and its door stands on its middle, where a front of {n} "
                         f"bays has {middle}: the kit's door takes {took}, which leaves {n - g}, and as many bays must "
                         "stand on each side of it", f"make it {n - 1} or {n + 1} bays across")        # `n - 1` is at least the door: `n - g` is odd


def swap_sides(fill):
    """A `fill` for the twin on the other side of the axis (rule 6): sides 1 and 3 change
    places. A value that is no list of tables is returned as it is, for `lay` to refuse."""
    if not isinstance(fill, list):
        return fill
    out = []
    for e in fill:
        if isinstance(e, dict) and isinstance(e.get("sides"), list):
            e = dict(e, sides=[{1: 3, 3: 1}.get(x, x) if isinstance(x, (int, float)) and not isinstance(x, bool) else x
                               for x in e["sides"]])
        out.append(e)
    return out


def mirrored(fill):
    """A wing's `fill`, each entry laid mirrored (rule 6)."""
    if not isinstance(fill, list):
        return fill
    return [dict(e, mirror=True) if isinstance(e, dict) else e for e in fill]


def overlap(a, b):
    """mm two boxes `(u0, v0, u1, v1)` overlap by on each axis: `(du, dv)`, each above 0 where
    they overlap."""
    return min(a[2], b[2]) - max(a[0], b[0]), min(a[3], b[3]) - max(a[1], b[1])


def plan_court(name, at, attrs, kits):
    """One court's buildings, its plot and notes (rules 1 to 9). `at` is its point in mm, `kits` a function
    of a taxon that gives `kit_facts`. Returns `(buildings, notes)`: a building is `{key, ring,
    attrs, box}`, its ring in mm and counter-clockwise from its front, `box` its outline in the
    court's own frame `(u0, v0, u1, v1)`."""
    to = attrs.get("to")
    if not isinstance(to, list) or len(to) != 2 or any(
            isinstance(x, bool) or not isinstance(x, (int, float)) or not math.isfinite(x) for x in to):
        raise ParamError("to", f"`to` is {to!r}", "write `to = [x, y]`, a point on the axis further into the court")
    tx, ty = to[0] * 1000.0 - at[0], to[1] * 1000.0 - at[1]
    far = math.hypot(tx, ty)
    if far < 1.0:
        raise ParamError("to", "`to` is the court's own point, and gives no axis", "write a point further into the court")
    v = (tx / far, ty / far)
    u = (v[1], -v[0])                              # to the right of the way in

    def ring(pts):
        return [(int(round(at[0] + u[0] * a + v[0] * b)), int(round(at[1] + u[1] * a + v[1] * b))) for a, b in pts]

    def world(a, b):
        return (at[0] + u[0] * a + v[0] * b, at[1] + u[1] * a + v[1] * b)
    passed = {k: attrs[k] for k in PASSED if k in attrs}
    out, notes = [], []
    level = attrs.get("ground") or ""
    if level not in ("", "level"):
        raise ParamError("ground", f"`ground` is {level!r}",
                         'write `ground = "level"` for one plot levelled under the court, or leave it out')
    bank = {}
    for k, d in (("max_cut", 3.0), ("grade", 0.3)):
        got = attrs.get(k, d)
        if _mm(got) is None:
            raise ParamError(k, f"`{k}` is {got!r}", f"write `{k}` as a number, 0 or more")
        bank[k] = float(got)
    yard = attrs.get("yard") or None
    inner = None                                   # the yard's inner face, in the frame
    if yard is not None:
        if not isinstance(yard, dict) or not isinstance(yard.get("kit"), str) or not yard["kit"] or \
                set(yard) - {"kit", "bays", "storey_h"}:
            raise ParamError("yard", f"`yard` is {yard!r}", "write `yard = { kit, bays = [across, along] }`")
        yf = kits(yard["kit"])
        across, along = _bays(yard, "yard", "the yard", "write `bays = [across, along]`, two whole numbers of bays")
        on_axis(across, yf, "yard", "the yard's front")
        high = yard.get("storey_h") or yf["storey"] / 1000.0      # 0: the kit's, as a building's (F3)
        if _mm(high, 0.5) is None:
            raise ParamError("yard", f"the yard has `storey_h = {high!r}`", "write the wall's height in metres, 0.5 or more")
        W, D = side(across, yf), side(along, yf)
        a = {"kit": yard["kit"], "storey_h": high, "storeys": 0,
             "doors": [{"storey": 0, "side": 0, "at": round(W / 2000.0, 4)}]}
        box = (-W / 2, 0.0, W / 2, float(D))
        out.append({"key": f"{name}_yard", "ring": ring([(-W / 2, 0), (W / 2, 0), (W / 2, D), (-W / 2, D)]),
                    "attrs": a, "box": box, "yard": True})
        t = yf["wall"]
        inner = (box[0] + t, box[1] + t, box[2] - t, box[3] - t)
    halls = []                                     # (key, box, front side's ring points, attrs, facts)
    taxon = attrs.get("kit")
    hall, wings = attrs.get("hall") or None, attrs.get("wings") or []
    if (hall is not None or wings) and (not isinstance(taxon, str) or not taxon):
        raise ParamError("kit", "the court has a hall and no `kit`", "name the kit its halls are laid from")
    facts = kits(taxon) if taxon else None

    def common(table, key, what, across):
        """The attributes every hall takes: the door on its front's middle, and its veranda."""
        on_axis(across, facts, key, what)
        a = dict(passed, kit=taxon, storeys=1,
                 doors=[{"storey": 0, "side": 0, "at": round(side(across, facts) / 2000.0, 4)}])
        if "fill" in table:
            a["fill"] = table["fill"]
        ver = table.get("veranda", 0)
        deep = _whole(table["bays"][1])
        if _whole(ver, 0) is None or _whole(ver, 0) >= deep:
            raise ParamError(key, f"{what} has `veranda = {ver!r}`", f"write how many bays deep it is, from 0 to "
                             f"{deep - 1}: fewer than the hall's {deep}")
        if _whole(ver, 0):
            a["colonnade"] = [{"sides": [0], "depth": round(_whole(ver, 0) * facts["bay"] / 1000.0, 4)}]
        return a
    if hall is not None:
        if not isinstance(hall, dict) or set(hall) - {"bays", "from", "veranda", "fill"}:
            raise ParamError("hall", f"`hall` is {hall!r}", "write `hall = { bays = [across, deep], from }`")
        across, deep = _bays(hall, "hall", "the hall", "write `bays = [across, deep]`, two whole numbers of bays")
        f = _mm(hall.get("from"))
        if f is None:
            raise ParamError("hall", f"the hall has `from = {hall.get('from')!r}`",
                             "write the metres along the axis from the gate's line to its front")
        a = common(hall, "hall", "the hall", across)
        W, D = side(across, facts), side(deep, facts)
        halls.append((f"{name}_hall", (-W / 2, f, W / 2, f + D),
                      [(-W / 2, f), (W / 2, f), (W / 2, f + D), (-W / 2, f + D)], a))
    if not isinstance(wings, list):
        raise ParamError("wings", f"`wings` is {wings!r}", "write `wings = [{ bays = [along, deep], from, off }]`")
    for i, w in enumerate(wings):
        what = f"wing {i}"
        if not isinstance(w, dict) or set(w) - {"bays", "from", "off", "veranda", "fill"}:
            raise ParamError("wings", f"{what} is {w!r}", "write `{ bays = [along, deep], from, off }`")
        along, deep = _bays(w, "wings", what, "write `bays = [along, deep]`, two whole numbers of bays")
        f, o = _mm(w.get("from")), _mm(w.get("off"))
        if f is None or o is None:
            raise ParamError("wings", f"{what} has `from = {w.get('from')!r}` and `off = {w.get('off')!r}`",
                             "write `from`, the metres along the axis from the gate's line to its near end, and `off`, "
                             "the metres from the axis to its front")
        a = common(w, "wings", what, along)
        if "fill" in a:
            a["fill"] = mirrored(a["fill"])
        L, D = side(along, facts), side(deep, facts)
        halls.append((f"{name}_w{i}_l", (-o - D, f, -o, f + L),
                      [(-o, f), (-o, f + L), (-o - D, f + L), (-o - D, f)], a))
        b = dict(a)
        if "fill" in b:
            b["fill"] = swap_sides(b["fill"])
        halls.append((f"{name}_w{i}_r", (o, f, o + D, f + L),
                      [(o, f + L), (o, f), (o + D, f), (o + D, f + L)], b))
    e = facts["eaves"] if facts else 0
    kept = []                                      # (key, box, grown box)
    for key, box, pts, a in halls:
        grown = (box[0] - e, box[1] - e, box[2] + e, box[3] + e)
        mid = world((box[0] + box[2]) / 2, (box[1] + box[3]) / 2)
        dropped = False
        if inner is not None:
            past = max(inner[0] - box[0], inner[1] - box[1], box[2] - inner[2], box[3] - inner[3])
            over = max(inner[0] - grown[0], inner[1] - grown[1], grown[2] - inner[2], grown[3] - inner[3])
            if past > FIT:
                notes.append({"code": "court_meets", "court": name, "key": key, "other": f"{name}_yard", "by": past,
                              "eaves": False, "wall": True, "at": mid})
                dropped = True
            elif over > FIT:
                notes.append({"code": "court_meets", "court": name, "key": key, "other": f"{name}_yard", "by": over,
                              "eaves": True, "wall": True, "at": mid})
        for other, obox, ogrown in kept:
            du, dv = overlap(box, obox)
            if du > FIT and dv > FIT:
                notes.append({"code": "court_meets", "court": name, "key": key, "other": other, "by": min(du, dv),
                              "eaves": False, "wall": False, "at": mid})
                dropped = True
                continue
            du, dv = overlap(grown, ogrown)
            if du > FIT and dv > FIT:
                notes.append({"code": "court_meets", "court": name, "key": key, "other": other, "by": min(du, dv),
                              "eaves": True, "wall": False, "at": mid})
        for n in notes:
            if n["key"] == key:
                n["dropped"] = dropped
        if dropped:
            continue
        kept.append((key, box, grown))
        out.append({"key": key, "ring": ring(pts), "attrs": a, "box": box})
    if level and out:                              # one plot under the whole court (rule 9)
        boxes = [b["box"] for b in out if b.get("yard")] or [b["box"] for b in out]
        u0, v0 = min(b[0] for b in boxes) - PLOT_APRON, min(b[1] for b in boxes) - PLOT_APRON
        u1, v1 = max(b[2] for b in boxes) + PLOT_APRON, max(b[3] for b in boxes) + PLOT_APRON
        out.append({"key": f"{name}_plot", "ring": ring([(u0, v0), (u1, v0), (u1, v1), (u0, v1)]), "plot": True,
                    **bank})
    return out, notes


def plan(courts, kits):
    """Every court's buildings and notes. `courts` is `[(name, point mm, attrs)]`."""
    buildings, notes = [], []
    for name, at, attrs in sorted(courts, key=lambda c: c[0]):
        try:
            b, n = plan_court(name, at, attrs, kits)
        except ParamError as e:
            e.court = name
            raise
        buildings += b
        notes += n
    return buildings, notes


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


def read_courts(step):
    try:
        fl = step.features(select="kind:court", columns=ATTRS)
    except Exception:                             # no court in the world
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


def inside(ring, p):
    """Whether `p` lies in the ring or on its edge."""
    x, y, odd = p[0], p[1], False
    for (x0, y0), (x1, y1) in zip(ring, ring[1:] + ring[:1]):
        cross = (x1 - x0) * (y - y0) - (y1 - y0) * (x - x0)
        if cross == 0 and min(x0, x1) <= x <= max(x0, x1) and min(y0, y1) <= y <= max(y0, y1):
            return True
        if (y0 > y) != (y1 > y) and x < x0 + (y - y0) * (x1 - x0) / (y1 - y0):
            odd = not odd
    return odd


def posts_in(ring, grid):
    """Every post of the grid inside the ring (edges included), then the ring's corners: the
    points lay.py reads a building's ground at (its rule 2), so a court's level and the ground
    its halls then read are one number."""
    out = []
    if grid and grid.get("resolution_mm") and grid.get("extent_mm"):
        x0, y0, r = int(grid["extent_mm"][0]), int(grid["extent_mm"][1]), int(grid["resolution_mm"])
        nx, ny = int(grid.get("nx", 0)), int(grid.get("ny", 0))
        i0 = max(0, -(-(min(p[0] for p in ring) - x0) // r))
        i1 = min(nx - 1, (max(p[0] for p in ring) - x0) // r)
        j0 = max(0, -(-(min(p[1] for p in ring) - y0) // r))
        j1 = min(ny - 1, (max(p[1] for p in ring) - y0) // r)
        for j in range(j0, j1 + 1):
            for i in range(i0, i1 + 1):
                p = (x0 + i * r, y0 + j * r)
                if inside(ring, p):
                    out.append(p)
    return out + list(ring)


def read_level(step, ring):
    """The mean of the land under a plot (rule 9), metres to the millimetre; 0 in a world with
    no terrain."""
    try:
        (surface,) = step.fields(["role:surface"], window=[0, 0, 1, 1])
    except (LookupError, RpcError):                # as lay.py's read_ground: no surface field
        return 0.0
    pts = posts_in(ring, step.grid)
    # A plot reaches 1.5 m past its halls, and may reach past the world's extent, where the
    # ground cannot be read: its level is the land inside (F2's review: it raised). A hall past
    # the edge is then lay.py's `outside` (its rule 20).
    ext = (step.grid or {}).get("extent_mm")
    if ext:
        pts = [p for p in pts if ext[0] <= p[0] <= ext[2] and ext[1] <= p[1] <= ext[3]] or pts
    h = [float(v) for v in _list(step.eval(f"= f('{surface.name}')", x=[p[0] for p in pts], y=[p[1] for p in pts]))]
    return round(sum(h) / len(h), 3)


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


def generate(step):
    courts = read_courts(step)
    facts = {}

    def kits(taxon):
        if taxon not in facts:
            facts[taxon] = kit_facts(read_kit(step, taxon), taxon)
        return facts[taxon]
    try:
        buildings, notes = plan(courts, kits)
    except ParamError as e:
        step.error("params", e.court or "step." + step.id, f"{e.court}: {e.message}" if e.court else e.message, e.hint,
                   explain=EXPLAIN["params"], data={"attribute": e.key})
        return
    o = step.out
    for b in buildings:
        if b.get("plot"):
            o.feature(b["key"], "plot", region=b["ring"], level=read_level(step, b["ring"]),
                      grade=b["grade"], max_cut=b["max_cut"])
            continue
        o.feature(b["key"], "building", region=b["ring"], **b["attrs"])       # the SDK takes millimetres
    x0_, y0_, x1_, y1_ = (step.grid or {}).get("extent_mm") or (-math.inf, -math.inf, math.inf, math.inf)
    for i, n in enumerate(notes):
        kw = {"data": {"building": n["key"], "other": n["other"], "by": round(n["by"] / 1000, 3),
                       "eaves": n["eaves"], "returned": not n.get("dropped")}}
        if x0_ <= n["at"][0] <= x1_ and y0_ <= n["at"][1] <= y1_:
            kw["position"] = (n["at"][0] / 1000, n["at"][1] / 1000)
        who = f"`{n['key']}`" + (", with its eaves," if n["eaves"] else "")
        said = (f"{who} passes the inner face of the yard's wall by {n['by'] / 1000:g} m" if n["wall"]
                else f"{who} stands {n['by'] / 1000:g} m into `{n['other']}`" + (" and its eaves" if n["eaves"] else ""))
        if n.get("dropped"):
            said += ": it is not returned"
        step.warning("court_meets", n["court"], said,
                     f"move it {n['by'] / 1000:g} m or more: `from` is along the axis and `off` away from it"
                     + ("" if n["wall"] else "; or move the other"),
                     **kw, **({"explain": EXPLAIN["court_meets"]} if i == 0 else {}))
    count = sum(1 for b in buildings if not b.get("plot"))
    step.info("courts", "step." + step.id, f"{len(courts)} courts, {count} buildings", "none needed",
              data={"courts": len(courts), "buildings": count}, explain=EXPLAIN["courts"])
