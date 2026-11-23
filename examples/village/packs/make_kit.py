"""Writes the village kit: packs/village/*.glb, the pieces of a timber-framed village house.

    python3 examples/village/packs/make_kit.py
    via pack accept examples/village/packs/village.toml       # after a piece changes
    via pack accept examples/village/packs/bare.toml          # and the two packs made from it
    via pack accept examples/village/packs/shallow.toml

A studio's proof kit (docs/research/Modular kit failures in studios.md, section 2): untextured
pieces at their exact sizes. One module, the 3 m bay, one storey height, 2.7 m, walls 0.3 m thick.
A bay is a timber frame with its panel filled in: a post at each end, on the module's boundary,
which is where a kit hides its seams (section 12 of the report, on timber framing). The roof is a
steep gable for a house two bays deep: slope pieces and gable halves, as examples/kit's.

The models are made by tools/packgen's `glb` and `box`: glTF's convention, +Y up, front +Z, origin
at the base centre. A wall piece lies along its x axis.
"""

import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parents[2] / "tools" / "packgen"))

import packgen  # noqa: E402

T = 0.3            # metres: every wall piece's thickness
H = 2.7            # metres: a storey
BAY = 3.0          # metres: the module
HALF = 1.5         # metres: the half bay, the door frame and the floor tile
POST = 0.2         # metres: a frame's post, seen from the front
SLAB = 0.2         # metres: a floor tile's thickness, below the floor it tops
SPAN = 3.3         # metres in plan from a roof's eave to its ridge: half of a house two bays deep
RISE = 3.0         # metres a roof slope climbs over its span
BASE = 1.5         # metres: the stone base under the walls, from the floor down


def frame(width, holes=()):
    """A framed panel `width` long, `T` thick, `H` tall: a timber post `POST` wide at each end
    and a rail under the head, the panel between them filled, with rectangular holes
    `(x0, x1, y0, y1)` through the fill. Everything lies inside the piece's own box."""
    w, t, p = width / 2, T / 2, POST
    out = [("timber", packgen.box(-w, 0.0, -t, -w + p, H, t)),
           ("timber", packgen.box(w - p, 0.0, -t, w, H, t)),
           ("timber", packgen.box(-w + p, H - p, -t, w - p, H, t))]
    cuts = sorted({-w + p, w - p, *[x for h in holes for x in h[:2]]})
    for a, b in zip(cuts, cuts[1:]):
        inside = sorted((h[2], h[3]) for h in holes if h[0] <= a and b <= h[1])
        y = 0.0
        for lo, hi in inside:
            if lo > y:
                out.append(("render", packgen.box(a, y, -t, b, lo, t)))
            y = hi
        if y < H - p:
            out.append(("render", packgen.box(a, y, -t, b, H - p, t)))
    return out


def post():
    t = T / 2
    return [("timber", packgen.box(-t, 0.0, -t, t, H, t))]


def tile():
    h = HALF / 2
    return [("render", packgen.box(-h, 0.0, -h, h, SLAB, h))]


def solid(faces):
    """Triangles of a convex solid from its faces, each a list of points in the ASSET's frame
    (x along the piece, y its front, z up), wound outward by the solid's own middle. Returned in
    the model's frame (glTF): model (x, y, z) = asset (-x, z, y) (spec/assets.md 2.3)."""
    pts = [p for f in faces for p in f]
    c = tuple(sum(p[i] for p in pts) / len(pts) for i in range(3))
    out = []
    for f in faces:
        a, b, d = f[0], f[1], f[2]
        u, v = tuple(b[i] - a[i] for i in range(3)), tuple(d[i] - a[i] for i in range(3))
        n = (u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0])
        if sum(n[i] * (a[i] - c[i]) for i in range(3)) < 0:
            f = list(reversed(f))
        for k in range(1, len(f) - 1):
            out.append(tuple((-q[0], q[2], q[1]) for q in (f[0], f[k], f[k + 1])))
    return out


def roof_slope(pitch, span, rise):
    """A roof slope: a wedge `pitch` long along the eave (x), `span` deep in plan, climbing from
    0 at its front edge (the eave, +y) to `rise` at its back (the ridge)."""
    x, y = pitch / 2, span / 2
    e0, e1 = (-x, y, 0.0), (x, y, 0.0)
    b0, b1 = (-x, -y, 0.0), (x, -y, 0.0)
    r0, r1 = (-x, -y, rise), (x, -y, rise)
    return [("slate", solid([[e0, e1, r1, r0], [e0, b0, b1, e1], [b0, r0, r1, b1], [e0, r0, b0], [e1, b1, r1]]))]


def roof_gable(span, rise):
    """Half a gable: a wall `T` thick and `span` long (x) under the roof's edge, 0 high at +x and
    `rise` at -x. Two stand on each end of a roof, the second turned about."""
    x, t = span / 2, T / 2
    lo0, lo1 = (x, -t, 0.0), (x, t, 0.0)
    b0, b1 = (-x, -t, 0.0), (-x, t, 0.0)
    r0, r1 = (-x, -t, rise), (-x, t, rise)
    return [("timber", solid([[lo0, lo1, r1, r0], [lo0, b0, b1, lo1], [b0, r0, r1, b1], [lo0, r0, b0], [lo1, b1, r1]]))]


def stone(width, high):
    """A plain block `width` long, `T` thick, `high` tall: a course of the base."""
    w, t = width / 2, T / 2
    return [("stone", packgen.box(-w, 0.0, -t, w, high, t))]


REACH = 1.2        # metres of each wall, and of each inner eave, an inside corner's piece takes


def inside_corner(high, part="timber"):
    """The piece for an inside corner of a wall: the notch the two walls leave (`T` square) and
    `REACH` of each wall from it, an L. Placed by the notch's middle, which is the model's own
    origin (the pack says `pivot = "origin"`); one leg runs along +x and one along +z."""
    t = T / 2
    return [(part, packgen.box(-t, 0.0, -t, t, high, REACH + t)),
            (part, packgen.box(t, 0.0, -t, REACH + t, high, t))]


def roof_hip(span, rise):
    """The roof over the outer corner of an L: one span square, its eaves on its front (+y) and
    its +x side, climbing from both to the hip, the diagonal from that corner to the opposite
    one, which is as high as the ridge. Two solids, one either side of the hip."""
    h = span / 2
    low, far, top = (h, h, 0.0), (-h, -h, 0.0), (-h, -h, rise)
    a, b = (-h, h, 0.0), (h, -h, 0.0)             # the two other corners
    return [("slate", solid([[far, low, a], [far, a, top], [a, low, top], [far, low, top]])),
            ("slate", solid([[far, b, low], [far, b, top], [b, low, top], [far, low, top]]))]


def roof_valley(span, rise, reach):
    """The roof at the inside corner of an L, placed by the corner itself (the model's own
    origin): the span-square quarter of the roof beside the corner, where the two wings' inner
    slopes meet along the valley, and `reach` of each inner eave beyond the corner. In the
    asset's frame the corner is (0, 0), one wing's eave runs along y = 0 towards -x and climbs
    towards -y, the other's along x = 0 towards +y and climbs towards +x; the valley is the
    diagonal from (0, 0) to (span, -span). Two solids, one for each slope."""
    s, r, R = span, reach, rise
    one = [[(-r, -s, 0.0), (s, -s, 0.0), (0.0, 0.0, 0.0), (-r, 0.0, 0.0)],          # under
           [(-r, -s, R), (s, -s, R), (0.0, 0.0, 0.0), (-r, 0.0, 0.0)],              # the slope
           [(-r, -s, 0.0), (s, -s, 0.0), (s, -s, R), (-r, -s, R)],                   # under the ridge
           [(-r, -s, 0.0), (-r, 0.0, 0.0), (-r, -s, R)],                              # its end
           [(s, -s, 0.0), (0.0, 0.0, 0.0), (s, -s, R)]]                               # the valley's plane
    two = [[(0.0, 0.0, 0.0), (s, -s, 0.0), (s, r, 0.0), (0.0, r, 0.0)],
           [(0.0, 0.0, 0.0), (s, -s, R), (s, r, R), (0.0, r, 0.0)],
           [(s, -s, 0.0), (s, r, 0.0), (s, r, R), (s, -s, R)],
           [(0.0, r, 0.0), (s, r, 0.0), (s, r, R)],
           [(0.0, 0.0, 0.0), (s, -s, 0.0), (s, -s, R)]]
    return [("slate", solid(one)), ("slate", solid(two))]


VILLAGE = {
    # an L: the inside corner of the walls and of the base, the hip and the valley of the roof
    "corner_in.glb": inside_corner(H),
    "base_corner_in.glb": inside_corner(BASE, part="stone"),
    "roof_hip.glb": roof_hip(SPAN, RISE),
    "roof_valley.glb": roof_valley(SPAN, RISE, REACH),
    # the base: the wall under the floor, down to the ground
    "base.glb": stone(BAY, BASE),
    "base_half.glb": stone(HALF, BASE),
    "base_post.glb": stone(T, BASE),
    "frame.glb": frame(BAY),
    "frame_window.glb": frame(BAY, [(-0.5, 0.5, 1.0, 2.0)]),
    "half.glb": frame(HALF),
    "post.glb": post(),
    "door.glb": frame(HALF, [(-0.5, 0.5, 0.0, 2.0)]),
    "tile.glb": tile(),
    "roof_slope.glb": roof_slope(HALF, SPAN, RISE),
    "roof_gable.glb": roof_gable(SPAN, RISE),
}


SHALLOW = 0.5      # metres: the base of the `shallow` pack


def revised(name):
    """packs/village.toml as another pack of the same folder of models, for a change:
    `bare` is the kit with no base; `shallow` is the kit with a base 0.5 m deep (its three base
    models are `shallow_*.glb`)."""
    text = (HERE / "village.toml").read_text()
    head = text[:text.index("# ---- the base")]
    tail = text[text.index("[material.ground]"):]
    base = text[text.index("# ---- the base"):text.index("[material.ground]")]
    note = {"bare": "with no base: nothing stands under the walls",
            "shallow": "with a base 0.5 m deep in place of 1.5 m"}[name]
    head = head.replace("# The village kit (examples/village)", f"# The village kit {note} (changes/{name}.toml). Written\n"
                        "# by packs/make_kit.py from packs/village.toml: edit that.\n# The village kit (examples/village)")
    if name == "bare":
        return head.replace("base   = 1.5                     # how far the base reaches below the floor\n", "") + tail
    base = base.replace('"base.glb"', '"shallow_base.glb"').replace('"base_half.glb"', '"shallow_base_half.glb"')
    base = base.replace('"base_post.glb"', '"shallow_base_post.glb"').replace("sink   = 1.5", "sink   = 0.5")
    base = base.replace('"base_corner_in.glb"', '"shallow_base_corner_in.glb"')
    base = base.replace("sink  = 1.5", "sink  = 0.5")
    return head.replace("base   = 1.5 ", "base   = 0.5 ") + base + tail


def main():
    (HERE / "village").mkdir(exist_ok=True)
    files = dict(VILLAGE, **{"shallow_base.glb": stone(BAY, SHALLOW), "shallow_base_half.glb": stone(HALF, SHALLOW),
                             "shallow_base_post.glb": stone(T, SHALLOW),
                             "shallow_base_corner_in.glb": inside_corner(SHALLOW, part="stone")})
    for name, parts in sorted(files.items()):
        (HERE / "village" / name).write_bytes(packgen.glb(parts))
    for name in ("bare", "shallow"):
        (HERE / f"{name}.toml").write_text(revised(name))


if __name__ == "__main__":
    main()
