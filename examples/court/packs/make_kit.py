"""Writes the timber kit and the yard kit: packs/timber/*.glb, the pieces of a walled court of
timber halls.

    python3 examples/court/packs/make_kit.py
    via pack accept examples/court/packs/timber.toml          # after a piece changes
    via pack accept examples/court/packs/short_eaves.toml     # and the pack made from it

A studio's proof kit (docs/research/Modular kit failures in studios.md, section 2): untextured
pieces at their exact sizes. Our own design: the research found no developer's account of East
Asian building from pieces (the same report, section 12).

Two kits in one pack.

`structure/timber`, the halls. A hall is posts on a lattice of 3.0 m, the bay, and what stands
between two posts is a panel 2.7 m long, not a wall: the kit is laid by posts (lib/building/lay.py
rule 15). The posts are 0.3 m square. The roof is the kit's main piece: it is hipped and stands
1.35 m beyond the posts on every side, so a hall three bays deep (9.3 m) is under a roof 12 m
deep and takes pieces with a span of 6.0 m, and one two bays deep (6.3 m) pieces with a span of
4.5 m. The kit has a slope and a hip for each. The base is the face of the stone platform under
the posts' outer half; the floor's tiles run from one line of posts' middles to the next.

`structure/yard`, the wall round the court: a bay 3.0 m long and 2.4 m high, a corner post, and
the gate, a bay with an opening.

The models are made by tools/packgen's `glb` and `box`: glTF's convention, +Y up, front +Z,
origin at the base centre. A wall piece lies along its x axis.
"""

import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parents[2] / "tools" / "packgen"))

import packgen  # noqa: E402

BAY = 3.0          # metres from one post's middle to the next
POST = 0.3         # a post, square
PANEL = BAY - POST # what stands between two posts
T = 0.15           # a panel's thickness, half a post
H = 3.6            # a storey: the post, and the beam it carries in its top 0.45 m
BEAM = 0.45
PLATFORM = 0.6     # the base, from the floor down
SLAB = 0.1         # a floor tile's thickness, below the floor it tops
TILE = 1.5         # a floor tile, half a bay square
EAVES = 1.35       # how far the roof stands beyond the posts
PITCH = 0.4        # what the roof climbs for each metre in plan
YARD_T = 0.3       # the yard wall's thickness
YARD_H = 2.4


def bar(length, thick, high, y0=0.0, part="timber"):
    """A plain block `length` long (x), `thick` through, from `y0` up to `y0 + high`."""
    w, t = length / 2, thick / 2
    return [(part, packgen.box(-w, y0, -t, w, y0 + high, t))]


def panel():
    """The plain panel: a sill, a head and plaster between, one box."""
    return bar(PANEL, T, H, part="render")


def lattice():
    """The panel with a lattice window: plaster to 0.9 m and from 3.0 m, and between them nine
    upright bars with the same box as the plain panel."""
    w, t = PANEL / 2, T / 2
    parts = [("render", packgen.box(-w, 0.0, -t, w, 0.9, t)), ("render", packgen.box(-w, 3.0, -t, w, H, t))]
    for k in range(10):
        x = -w + k * (PANEL - 0.06) / 9
        parts.append(("timber", packgen.box(x, 0.9, -t, x + 0.06, 3.0, t)))
    return parts


def doors():
    """The panel as four door leaves, closed: each leaf a frame's width of timber round a thinner
    board, under a head; the plain panel's box."""
    w, t, leaf = PANEL / 2, T / 2, PANEL / 4
    parts = [("timber", packgen.box(-w, 3.0, -t, w, H, t))]
    for k in range(4):
        x = -w + k * leaf
        parts.append(("timber", packgen.box(x, 0.0, -t, x + 0.06, 3.0, t)))
        parts.append(("timber", packgen.box(x + leaf - 0.06, 0.0, -t, x + leaf, 3.0, t)))
        parts.append(("render", packgen.box(x + 0.06, 0.0, -t / 2, x + leaf - 0.06, 3.0, t / 2)))
    return parts


def door():
    """The door frame: a panel's length with an opening 1.5 m wide and 2.4 m high in its middle."""
    w, t, o = PANEL / 2, T / 2, 0.75
    return [("render", packgen.box(-w, 0.0, -t, -o, H, t)),
            ("render", packgen.box(o, 0.0, -t, w, H, t)),
            ("timber", packgen.box(-o, 2.4, -t, o, H, t))]


def post():
    return bar(POST, POST, H)


def beam():
    """The beam from one post of a veranda to the next, in the top of the storey, open under."""
    return bar(PANEL, T, BEAM, H - BEAM)


def tile():
    h = TILE / 2
    return [("timber", packgen.box(-h, 0.0, -h, h, SLAB, h))]


def gate():
    """The yard's gate: a bay of its wall with an opening 2.4 m wide and 2.1 m high."""
    w, t, o = BAY / 2, YARD_T / 2, 1.2
    return [("render", packgen.box(-w, 0.0, -t, -o, YARD_H, t)),
            ("render", packgen.box(o, 0.0, -t, w, YARD_H, t)),
            ("timber", packgen.box(-o, 2.1, -t, o, YARD_H, t))]


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


def roof_slope(pitch, span):
    """A roof slope: a wedge `pitch` long along the eave (x), `span` deep in plan, climbing from
    0 at its front edge (the eave, +y) to the ridge at its back."""
    x, y, rise = pitch / 2, span / 2, span * PITCH
    e0, e1 = (-x, y, 0.0), (x, y, 0.0)
    b0, b1 = (-x, -y, 0.0), (x, -y, 0.0)
    r0, r1 = (-x, -y, rise), (x, -y, rise)
    return [("slate", solid([[e0, e1, r1, r0], [e0, b0, b1, e1], [b0, r0, r1, b1], [e0, r0, b0], [e1, b1, r1]]))]


def roof_hip(span):
    """The roof over a corner: one span square, its eaves on its front (+y) and its +x side,
    climbing from both to the hip, the diagonal from that corner to the opposite one, which is as
    high as the ridge. Two solids, one either side of the hip."""
    h, rise = span / 2, span * PITCH
    low, far, top = (h, h, 0.0), (-h, -h, 0.0), (-h, -h, rise)
    a, b = (-h, h, 0.0), (h, -h, 0.0)             # the two other corners
    return [("slate", solid([[far, low, a], [far, a, top], [a, low, top], [far, low, top]])),
            ("slate", solid([[far, b, low], [far, b, top], [b, low, top], [far, low, top]]))]


TIMBER = {
    # the halls' walls
    "post.glb": post(),
    "panel.glb": panel(),
    "panel_lattice.glb": lattice(),
    "panel_doors.glb": doors(),
    "door.glb": door(),
    "tile.glb": tile(),
    # the base: the platform's face, from the floor down, half a post thick
    "base.glb": bar(BAY, T, PLATFORM, part="stone"),
    "base_corner.glb": bar(T, T, PLATFORM, part="stone"),
    # the veranda
    "beam.glb": beam(),
    # the roof, for a hall three bays deep and for one two bays deep
    "roof_slope_6.glb": roof_slope(BAY, 6.0),
    "roof_hip_6.glb": roof_hip(6.0),
    "roof_slope_45.glb": roof_slope(BAY, 4.5),
    "roof_hip_45.glb": roof_hip(4.5),
    # the yard
    "yard_wall.glb": bar(BAY, YARD_T, YARD_H, part="render"),
    "yard_corner.glb": bar(YARD_T, YARD_T, YARD_H, part="render"),
    "yard_gate.glb": gate(),
}


def revised(name):
    """packs/timber.toml as another pack of the same folder of models, for a change:
    `short_eaves` is the kit that states eaves of 0.9 m over roof pieces made for 1.35 m."""
    text = (HERE / "timber.toml").read_text()
    text = text.replace("# The timber kit and the yard kit (examples/court)",
                        "# The timber kit stating eaves of 0.9 m, which its roof pieces are not made for "
                        f"(changes/{name}.toml).\n# Written by packs/make_kit.py from packs/timber.toml: edit that.\n"
                        "# The timber kit and the yard kit (examples/court)")
    assert "eaves  = 1.35" in text
    return text.replace("eaves  = 1.35", "eaves  = 0.9 ")


def main():
    (HERE / "timber").mkdir(exist_ok=True)
    for name, parts in sorted(TIMBER.items()):
        (HERE / "timber" / name).write_bytes(packgen.glb(parts))
    if (HERE / "timber.toml").exists():
        (HERE / "short_eaves.toml").write_text(revised("short_eaves"))


if __name__ == "__main__":
    main()
