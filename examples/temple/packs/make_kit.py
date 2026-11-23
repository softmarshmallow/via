"""Writes the order kit: packs/order/*.glb, the pieces of a classical temple.

    python3 examples/temple/packs/make_kit.py
    via pack accept examples/temple/packs/order.toml          # after a piece changes
    via pack accept examples/temple/packs/fat.toml            # and the pack made from it

A studio's proof kit (docs/research/Modular kit failures in studios.md, section 2): untextured
pieces at their exact sizes. Our own design: the research found no developer's account of
classical building (the same report, section 12). Every size is a whole number of one unit, the
column's width, 0.9 m: a column stands every three of it, the walls are one thick and their bay
two long, a storey is ten high (a column of eight and an entablature of two), the podium one.

Three groups of pieces in one taxon. The walls of the room inside (the cella): a bay, a bay with
a niche, a half bay, a corner post, a door frame, a floor tile. The base under the outline, tagged
`storey:base`. And the colonnade, tagged `storey:colonnade`: the column, which is a post, and the
lintel that spans from one column to the next, open under it.

The models are made by tools/packgen's `glb`, `box` and `cylinder`: glTF's convention, +Y up,
front +Z, origin at the base centre. A wall piece lies along its x axis.
"""

import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parents[2] / "tools" / "packgen"))

import packgen  # noqa: E402

U = 0.9            # metres: the unit, a column's width
T = U              # every wall piece's thickness
BAY = 2 * U        # the cella's bay, the base's, and the roof slope along its eave
AXIS = 3 * U       # from one column's middle to the next
SHAFT = 8 * U      # a column, capital and all
H = 10 * U         # a storey: the column and its entablature
DOOR = 4 * U       # the door frame; its opening is two units wide and six high
PODIUM = U         # the base, from the floor down
SLAB = 0.3         # a floor tile's thickness, below the floor it tops
SPAN = 8 * U       # in plan from the roof's eave to its ridge: half of a temple five columns' bays wide
RISE = 3 * U       # what a roof slope climbs over its span


def block(length, high, y0=0.0, part="stone"):
    """A plain block `length` long, `T` thick, from `y0` up to `y0 + high`."""
    w, t = length / 2, T / 2
    return [(part, packgen.box(-w, y0, -t, w, y0 + high, t))]


def niche():
    """The bay with a niche in its front: the bay's own box, less a recess one unit wide, four
    high and a third of the wall deep, from one unit above the floor."""
    w, t, d = BAY / 2, T / 2, T / 3
    return [("stone", packgen.box(-w, 0.0, -t, w, H, t - d)),              # the wall behind the recess
            ("stone", packgen.box(-w, 0.0, t - d, -U / 2, H, t)),          # left of it
            ("stone", packgen.box(U / 2, 0.0, t - d, w, H, t)),            # right of it
            ("stone", packgen.box(-U / 2, 0.0, t - d, U / 2, U, t)),       # under it
            ("stone", packgen.box(-U / 2, 5 * U, t - d, U / 2, H, t))]     # over it


def door():
    """The door frame: four units of wall with an opening two wide and six high in its middle,
    nothing in it."""
    w, t = DOOR / 2, T / 2
    return [("stone", packgen.box(-w, 0.0, -t, -U, H, t)),
            ("stone", packgen.box(U, 0.0, -t, w, H, t)),
            ("stone", packgen.box(-U, 6 * U, -t, U, H, t))]


def column(width=U):
    """A column and what it carries: a round shaft, a square capital a third of a unit high, and
    the block of entablature over it, `width` square in plan and a storey high."""
    r = width / 2
    return [("stone", packgen.cylinder(0.0, 0.0, 0.0, SHAFT - SLAB, r * 0.8, 16)),
            ("stone", packgen.box(-r, SHAFT - SLAB, -r, r, SHAFT, r)),
            ("stone", packgen.box(-r, SHAFT, -r, r, H, r))]


def lintel():
    """The entablature from one column to the next: a beam two units long and two high, at the
    top of the storey, with nothing under it. Its place repeats every three units."""
    return block(AXIS - U, H - SHAFT, SHAFT)


def tile():
    h = U / 2
    return [("stone", packgen.box(-h, 0.0, -h, h, SLAB, h))]


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
    """Half a pediment: a wall `T` thick and `span` long (x) under the roof's edge, 0 high at +x
    and `rise` at -x. Two stand on each end of a roof, the second turned about."""
    x, t = span / 2, T / 2
    lo0, lo1 = (x, -t, 0.0), (x, t, 0.0)
    b0, b1 = (-x, -t, 0.0), (-x, t, 0.0)
    r0, r1 = (-x, -t, rise), (-x, t, rise)
    return [("stone", solid([[lo0, lo1, r1, r0], [lo0, b0, b1, lo1], [b0, r0, r1, b1], [lo0, r0, b0], [lo1, b1, r1]]))]


ORDER = {
    # the cella
    "wall.glb": block(BAY, H),
    "wall_niche.glb": niche(),
    "corner.glb": block(T, H),
    "wall_half.glb": block(U, H),
    "door.glb": door(),
    "tile.glb": tile(),
    # the base: the podium's face, from the floor down
    "base.glb": block(BAY, PODIUM),
    "base_corner.glb": block(T, PODIUM),
    # the colonnade
    "column.glb": column(),
    "lintel.glb": lintel(),
    # the roof
    "roof_slope.glb": roof_slope(BAY, SPAN, RISE),
    "roof_gable.glb": roof_gable(SPAN, RISE),
    # changes/fat.toml: the column re-exported 1.0 m wide
    "fat_column.glb": column(1.0),
}


def revised(name):
    """packs/order.toml as another pack of the same folder of models, for a change: `fat` is the
    kit whose column was re-exported 1.0 m wide, off the unit."""
    text = (HERE / "order.toml").read_text()
    text = text.replace("# The order kit (examples/temple)", "# The order kit with its column re-exported 1.0 m wide "
                        f"(changes/{name}.toml). Written\n# by packs/make_kit.py from packs/order.toml: edit that.\n"
                        "# The order kit (examples/temple)")
    assert '"column.glb"' in text
    return text.replace('"column.glb"', '"fat_column.glb"')


def main():
    (HERE / "order").mkdir(exist_ok=True)
    for name, parts in sorted(ORDER.items()):
        (HERE / "order" / name).write_bytes(packgen.glb(parts))
    if (HERE / "order.toml").exists():
        (HERE / "fat.toml").write_text(revised("fat"))


if __name__ == "__main__":
    main()
