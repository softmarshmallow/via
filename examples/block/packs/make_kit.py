"""Writes the city kit: packs/city/*.glb, the pieces of a modern city block.

    python3 examples/block/packs/make_kit.py
    via pack accept examples/block/packs/city.toml       # after a piece changes

A studio's proof kit (docs/research/Modular kit failures in studios.md, section 2): untextured
pieces at their exact sizes. One module, the 3 m bay of `examples/kit`, and two storey heights: a
ground storey of 4.5 m with shopfronts and the storeys above it of 3.3 m with windows. Every wall
piece is `T` thick. What the pieces share is stated once in packs/city.toml (`defaults.shared`).

A facade is read by storey (docs/research/Modular city asset contracts.md, section 5): the ground
storey's pieces carry `storey:ground` in the pack, the others serve every storey above it. On a
street the bay holds a window; `blank` is the same bay with none, for a wall another building
stands against; `slot` is the bay with two narrow windows, for variety along a street.

The models are made by tools/packgen's `glb` and `box`: glTF's convention, +Y up, front +Z, origin
at the base centre. A wall piece lies along its x axis.
"""

import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parents[2] / "tools" / "packgen"))

import packgen  # noqa: E402

T = 0.3            # metres: every wall piece's thickness
H = 3.3            # metres: a storey above the ground storey
GROUND_H = 4.5     # metres: the ground storey
BAY = 3.0          # metres: the module
HALF = 1.5         # metres: the half bay, the door frame and the floor tile
SLAB = 0.2         # metres: a floor tile's thickness, below the floor it tops
PARAPET = 1.1      # metres: the wall round a flat roof


def pierced(width, high, holes, part="render"):
    """A wall piece `width` long (x), `T` thick, `high` tall, with rectangular holes through it:
    each `(x0, x1, y0, y1)` in metres from the piece's middle and its foot. Boxes round the holes,
    all inside the piece's own box: strips between the holes' sides, each solid but where a hole
    crosses it."""
    w, t = width / 2, T / 2
    cuts = sorted({-w, w, *[x for h in holes for x in h[:2]]})
    out = []
    for a, b in zip(cuts, cuts[1:]):
        inside = sorted((h[2], h[3]) for h in holes if h[0] <= a and b <= h[1])
        y = 0.0
        for lo, hi in inside:
            if lo > y:
                out.append((part, packgen.box(a, y, -t, b, lo, t)))
            y = hi
        if y < high:
            out.append((part, packgen.box(a, y, -t, b, high, t)))
    return out


def window_bay():
    """The street bay: a window 2 m wide from 0.9 to 2.5 m."""
    return pierced(BAY, H, [(-1.0, 1.0, 0.9, 2.5)])


def slot_bay():
    """The bay with two narrow windows, 0.6 m each, from 0.5 to 2.7 m."""
    return pierced(BAY, H, [(-1.05, -0.45, 0.5, 2.7), (0.45, 1.05, 0.5, 2.7)])


def shop_bay():
    """The ground storey's bay: a shopfront 2.4 m wide from 0.3 to 3.3 m under a 1.2 m fascia."""
    return pierced(BAY, GROUND_H, [(-1.2, 1.2, 0.3, 3.3)])


def door(high):
    """A door frame one half bay wide: a doorway 1.4 m wide and 2.4 m high, the wall over it."""
    return pierced(HALF, high, [(-0.7, 0.7, 0.0, 2.4)], part="timber")


def post(high):
    t = T / 2
    return [("stone", packgen.box(-t, 0.0, -t, t, high, t))]


def tile():
    h = HALF / 2
    return [("render", packgen.box(-h, 0.0, -h, h, SLAB, h))]


CITY = {
    # the storeys above the ground storey
    "bay.glb": window_bay(),
    "bay_blank.glb": pierced(BAY, H, []),
    "bay_slot.glb": slot_bay(),
    "half.glb": pierced(HALF, H, []),
    "post.glb": post(H),
    "door.glb": door(H),
    # the ground storey, 4.5 m
    "ground_bay.glb": shop_bay(),
    "ground_blank.glb": pierced(BAY, GROUND_H, []),
    "ground_half.glb": pierced(HALF, GROUND_H, []),
    "ground_post.glb": post(GROUND_H),
    "ground_door.glb": door(GROUND_H),
    # the floor, and a flat roof
    "tile.glb": tile(),
    # the wall round a flat roof
    "parapet.glb": pierced(BAY, PARAPET, [], part="stone"),
    "parapet_half.glb": pierced(HALF, PARAPET, [], part="stone"),
    "parapet_post.glb": post(PARAPET),
}


def main():
    (HERE / "city").mkdir(exist_ok=True)
    for name, parts in sorted(CITY.items()):
        (HERE / "city" / name).write_bytes(packgen.glb(parts))


if __name__ == "__main__":
    main()
