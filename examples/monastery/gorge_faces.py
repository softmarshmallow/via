"""Writes layers/gorge.toml: the gorge's two walls as lib/land `sheer_cliff` pieces (M5 slice 3).

    python3 examples/monastery/gorge_faces.py > examples/monastery/layers/gorge.toml

The gorge is cut into the land by world.toml's `height` (inside `w` metres of the stream's line the
ground is the floor `fl`, else the land `g`), so the terrain alone draws each wall as a step from
one 1 m post to the next. Each wall is written here as sheer faces along the line where that step
lies: the contour `dist(p, stream) = w(p)`, found per station of the stream by iterating the offset
`o = w(q + o n)` (60 rounds, halving), `n` the stream's left normal. A face's foot and brink are
straight in `t` (lib/land `foot_rise`, `top_rise`), so a wall whose floor or rim curves (the head
round the tarn, the mountain's rise above y = 235) is cut into pieces, each the longest (in steps of
0.0025 of the stream, shrunk by 0.7) whose `fl` and `g` (without the `lumps` noise) at the contour
stay within 0.5 m of the straight line between its ends (its path the samples
thinned to within 0.02 m); the face's `step` writes the straight line
beside the face, and the land fades back to its own by `back`. Below the head (`t` from 0.29 to 1)
`w` is 5 everywhere and both are straight in `y`: one face each side, the stream's line offset 5 m
(mitred at its bends). Two pieces meet square to the bisector of their segments at the joint
(`mitre`), so their strips share the joint's line and leave no wedge. The faces are written by coordinates, so moving the stream would leave them:
run this again (`follows` would carry the lower faces, but a path written by `follows` cannot also
start at the piece above by name). Standard library only; the formulas are world.toml's, restated.
"""

import math

STREAM = [(196, 320), (198, 250), (202, 180), (206, 100), (204, 0)]
LIP, BACK, TOL, STEP, LOWER, MITRE = 0.75, 6, 0.5, 0.0025, 0.29, 0.5


def smoothstep(e0, e1, x):
    u = min(max((x - e0) / (e1 - e0), 0.0), 1.0)
    return u * u * (3 - 2 * u)


SEGS, LENGTH = [], 0.0
for a, b in zip(STREAM, STREAM[1:]):
    SEGS.append((a, b, LENGTH, math.dist(a, b)))
    LENGTH += math.dist(a, b)


def at(t):
    s = t * LENGTH
    for a, b, c, seg in SEGS:
        if s <= c + seg or b == STREAM[-1]:
            u = (s - c) / seg
            return (a[0] + u * (b[0] - a[0]), a[1] + u * (b[1] - a[1])), ((b[0] - a[0]) / seg, (b[1] - a[1]) / seg)


TARN = at(0.2)[0]                                  # world.toml: tarn_site, on the stream at t 0.2
SHELF = (100, 190)


def w(p):
    return 5 + 12 * (1 - smoothstep(8, 28, math.dist(p, TARN)))


def fl(p):
    return 8 + 0.15 * p[1] + 20 * smoothstep(235, 320, p[1]) - 8 * (1 - smoothstep(4, 24, math.dist(p, TARN)))


def g(p):
    return 18 + 0.17 * p[1] + 24 * smoothstep(235, 320, p[1]) - 0.7 * 0.17 * (p[1] - 190) * (1 - smoothstep(50, 80, math.dist(p, SHELF)))


def contour(t, side):
    q, d = at(t)
    n = (-d[1] * side, d[0] * side)
    o = 5.0
    for _ in range(60):
        o = 0.5 * o + 0.5 * w((q[0] + o * n[0], q[1] + o * n[1]))
    return (q[0] + o * n[0], q[1] + o * n[1])


def piece(a, b, side):
    n = max(2, round((b - a) / STEP) + 1)
    pts = [contour(a + (b - a) * k / (n - 1), side) for k in range(n)]
    cum = [0.0]
    for p, q in zip(pts, pts[1:]):
        cum.append(cum[-1] + math.dist(p, q))
    f0, f1, g0, g1 = fl(pts[0]), fl(pts[-1]), g(pts[0]), g(pts[-1])
    err = max(max(abs(fl(p) - (f0 + (f1 - f0) * c / cum[-1])), abs(g(p) - (g0 + (g1 - g0) * c / cum[-1])))
              for p, c in zip(pts, cum))
    return err, pts, (f0, f1, g0, g1)


def pieces(t0, t1, side):
    out, a = [], t0
    while a < t1 - 1e-9:
        b = t1
        while True:
            err, pts, h = piece(a, b, side)
            if err <= TOL or b - a <= 4 * STEP:
                break
            b = a + (b - a) * 0.7
        out.append((pts, h, err))
        a = b
    return out


def thin(pts, tol=0.02):
    """Douglas-Peucker: the fewest points within `tol` metres of the contour's samples."""
    if len(pts) < 3:
        return pts
    a, b = pts[0], pts[-1]
    ab = math.dist(a, b) or 1e-12
    far, k = max((abs((b[0] - a[0]) * (a[1] - p[1]) - (a[0] - p[0]) * (b[1] - a[1])) / ab, i)
                 for i, p in enumerate(pts[1:-1], 1))
    if far <= tol:
        return [a, b]
    return thin(pts[:k + 1], tol)[:-1] + thin(pts[k:], tol)


def mitre(paths):
    """Each joint of two pieces made square to the bisector of their two segments there.

    A piece's strip ends square to its own last segment (a butt cap), so two pieces meeting at an
    angle leave an open wedge on the outside of the turn and overlap on the inside (the light
    wedges at the joints, M5 slice 3). A short segment along the bisector, `MITRE` metres (at
    most 0.4 of the segment it cuts into) is put on each side of the joint, so both strips end
    on the same line and the turn is taken inside each piece, at a round join.
    """
    out = [list(p) for p in paths]
    for a, b in zip(out, out[1:]):
        j, p, q = a[-1], a[-2], b[1]
        din, dout = math.dist(p, j), math.dist(j, q)
        u = ((j[0] - p[0]) / din + (q[0] - j[0]) / dout, (j[1] - p[1]) / din + (q[1] - j[1]) / dout)
        n = math.hypot(*u)
        e = min(MITRE, 0.4 * din, 0.4 * dout)
        u = (u[0] / n * e, u[1] / n * e)
        a.insert(len(a) - 1, (j[0] - u[0], j[1] - u[1]))
        b.insert(1, (j[0] + u[0], j[1] + u[1]))
    return out


def num(v):
    return f"{round(v, 2):g}"


def face(name, h, extra, note):
    f0, f1, g0, g1 = (round(v, 2) for v in h)
    lines = [f"[feature.{name}]{note}", 'kind      = "sheer_cliff"', *extra,
             f"foot_z    = {num(f0)}", f"foot_rise = {num(f1 - f0)}",
             f"top_z     = {num(g0)}", f"top_rise  = {num(g1 - g0)}"]
    return "\n".join(lines) + "\n"


def lower(side):
    """The stream from LOWER to its end, offset 5 m to `side`: each segment's offset, mitred."""
    q0, _ = at(LOWER)
    pts = [q0] + [p for p in STREAM if p[1] < q0[1]]
    out = []
    for k, p in enumerate(pts):
        ns = []
        for a, b in [s for s in ((pts[k - 1], p) if k else None, (p, pts[k + 1]) if k + 1 < len(pts) else None) if s]:
            seg = math.dist(a, b)
            ns.append((-(b[1] - a[1]) / seg * side, (b[0] - a[0]) / seg * side))
        nx, ny = sum(n[0] for n in ns), sum(n[1] for n in ns)
        dot = nx * ns[0][0] + ny * ns[0][1]
        out.append((p[0] + 5 * nx / dot, p[1] + 5 * ny / dot))
    return out


def main():
    print("# Layer `gorge` (rank 2): the gorge's two walls as sheer faces (lib/land `sheer_cliff`).")
    print("# Written by gorge_faces.py (its docstring says how); do not edit by hand, run it again.")
    print("# East wall: the high ground on the left of the stream's travel (`side = 1`); west: on the")
    print(f"# right (`side = -1`). `lip` {LIP} (the world's 1 m posts: at least 0.71), `back` {BACK} (the")
    print("# gorge is 10 m wide, so one wall's fade never reaches the other's brink), `taper = 0` so")
    print("# the pieces meet at full height, `beyond = 0` so no piece's step writes past its ends. Each piece starts at the one before's end by name, so the")
    print("# meeting of two pieces is owned (geometry.md 2.7) and held at gap 0 (surfaces.md 3.4).")
    for side, tag in ((1, "e"), (-1, "w")):
        common = [f"side      = {side}", f"lip       = {LIP}", f"back      = {BACK}", "taper     = 0", "beyond    = 0"]
        print()
        prev = None
        parts = [(pts, h, f"within {num(err)} m") for pts, h, err in pieces(0.0, LOWER, side)]
        lo = lower(side)
        parts.append((lo, (fl(lo[0]), fl(lo[-1]), g(lo[0]), g(lo[-1])), "below the head: w = 5, straight in y"))
        paths = mitre([thin(pts) for pts, _, _ in parts])
        for k, ((_, h, note), pts) in enumerate(zip(parts, paths)):
            name = f"gorge_{tag}{k}"
            if prev:
                pts = pts[1:]
            path = ", ".join(f"[{num(p[0])}, {num(p[1])}]" for p in pts)
            head = [f'starts_at = "{prev}.end"'] if prev else []
            print(face(name, h, [*head, f"path      = [{path}]", *common], f"                    # {note}"))
            prev = name


main()
