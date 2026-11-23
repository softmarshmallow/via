"""Writes the works kits: packs/works/*.glb, the pieces of a shed, a boiler house, the pipes
between them and a ring of rooms.

    python3 examples/works/packs/make_kit.py
    via pack accept examples/works/packs/works.toml          # after a piece changes
    via pack accept examples/works/packs/low.toml            # and each pack made from it
    via pack accept examples/works/packs/short_pipe.toml
    via pack accept examples/works/packs/wrong_plug.toml

A studio's proof kit (docs/research/Modular kit failures in studios.md, section 2): untextured
pieces at their exact sizes. Our own design. The research found the pipe kit as the model of a
kit and pipes kept as the glue between two kits, from one studio (the same report, sections 3
and 6), and no account of pipes joined freely in three dimensions (section 12).

Four kits in one pack.

`structure/shed`: a shed laid by posts (lib/building/lay.py rule 15). A bay is 6.0 m from one
post's middle to the next, a post 0.4 m square, a panel 5.6 m long and 0.2 m thick, the storey
7.2 m. One panel has a hole for a pipe 4.5 m up, and says so: its mount `port`.

`structure/boiler`: a boiler house whose walls are laid at a pitch (rule 6): a bay 3.0 m long and
0.3 m thick, 6.0 m high, a corner post, and its own panel with a port at the same 4.5 m.

`service/pipe`: the glue. A straight piece 2.0 m long, a slip piece 0.5 m, an elbow, a riser
3.0 m high, a flange (the plug a port takes), a cap, and a support for a pipe 4.5 m up. A pipe
piece's origin is on the pipe's middle line: lib/building/pipe.py stands each on a surface at
the pipe's height.

`interior/room`: whole rooms, joined door to door. A piece's origin is its way in; its way out
is where the next piece stands.

The models are made by tools/packgen's `glb` and `box`: glTF's convention, +Y up, front +Z.
"""

import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parents[2] / "tools" / "packgen"))

import packgen  # noqa: E402

# the shed
BAY = 6.0          # metres from one post's middle to the next
POST = 0.4
PANEL = BAY - POST
T = 0.2            # a panel's thickness, half a post
H = 7.2            # the shed's storey
TILE = 3.0         # its floor tile, half a bay
SLAB = 0.15
# the boiler house
B_BAY = 3.0
B_T = 0.3
B_H = 6.0
B_TILE = 1.5
# the port, on both
PORT = 4.5         # metres from the floor to the hole's middle
HOLE = 0.6         # the hole, square
# the pipe
SECTION = 0.3      # a pipe's section, square (a proof kit has no round)
STRAIGHT = 2.0
SLIP = 0.5
REACH = 0.5        # what an elbow, a riser, a flange and a cap each take of a leg
RISE = 3.0         # the riser
SUPPORT = 0.2      # a support's post
# the rooms
ROOM = 6.0
HALL = 3.0         # a hall's width
WALL = 0.15
ROOM_H = 3.0
DOOR = 1.2
LONG = 0.128       # what the room on another module is longer by


def abox(x0, y0, z0, x1, y1, z1):
    """A box given in the ASSET's frame (x to the piece's right, y its front, z up), in the
    model's (glTF): model (x, y, z) = asset (-x, z, y) (spec/assets.md 2.3)."""
    return packgen.box(-x1, z0, y0, -x0, z1, y1)


def bar(length, thick, high, z0=0.0, part="iron"):
    """A plain block `length` long (x), `thick` through, from `z0` up to `z0 + high`."""
    return [(part, abox(-length / 2, -thick / 2, z0, length / 2, thick / 2, z0 + high))]


def holed(length, thick, high, part):
    """A wall piece with a square hole through it, `HOLE` wide, its middle `PORT` up."""
    w, t, h, lo, hi = length / 2, thick / 2, HOLE / 2, PORT - HOLE / 2, PORT + HOLE / 2
    return [(part, abox(-w, -t, 0.0, w, t, lo)), (part, abox(-w, -t, hi, w, t, high)),
            (part, abox(-w, -t, lo, -h, t, hi)), (part, abox(h, -t, lo, w, t, hi))]


def opening(length, thick, high, wide, tall, part):
    """A wall piece with a doorway `wide` by `tall` in its middle."""
    w, t, o = length / 2, thick / 2, wide / 2
    return [(part, abox(-w, -t, 0.0, -o, t, high)), (part, abox(o, -t, 0.0, w, t, high)),
            (part, abox(-o, -t, tall, o, t, high))]


def tile(size):
    h = size / 2
    return [("stone", abox(-h, -h, 0.0, h, h, SLAB))]


# ---- the pipe: every piece about its middle line, z = 0

def pipe(length):
    s = SECTION / 2
    return [("slate", abox(-length / 2, -s, -s, length / 2, s, s))]


def elbow():
    """A quarter turn: one arm toward the piece's back and one toward its right, each `REACH`
    long from the corner. Every quarter turn is this piece turned (lib/building/pipe.py)."""
    s = SECTION / 2
    return [("slate", abox(-s, -REACH, -s, s, s, s)), ("slate", abox(s, -s, -s, REACH, s, s))]


def riser():
    """Up `RISE`: a stub toward the piece's back at the foot, the upright, a stub toward its
    front at the top."""
    s = SECTION / 2
    return [("slate", abox(-s, -REACH, -s, s, s, s)), ("slate", abox(-s, -s, s, s, s, RISE - s)),
            ("slate", abox(-s, -s, RISE - s, s, REACH, RISE + s))]


def flange():
    """The pipe's end at a port: a stub from the piece's back to a plate on its origin."""
    s = SECTION / 2
    return [("slate", abox(-s, -REACH, -s, s, -0.06, s)), ("slate", abox(-0.25, -0.06, -0.25, 0.25, 0.0, 0.25))]


def cap():
    """A closed end: a stub from the piece's back and a dome of two plates."""
    s = SECTION / 2
    return [("slate", abox(-s, -REACH, -s, s, -0.1, s)), ("slate", abox(-0.2, -0.1, -0.2, 0.2, 0.0, 0.2))]


def support():
    """A post and its saddle under a pipe whose middle is `PORT` up: made about the pipe's middle
    line, as every pipe piece is, and reaching from under the pipe down to where the ground is."""
    s, top = SUPPORT / 2, -SECTION / 2
    return [("iron", abox(-s, -s, -PORT, s, s, top - 0.1)), ("iron", abox(-0.3, -0.2, top - 0.1, 0.3, 0.2, top))]


# ---- the rooms: origin on the way in, the piece ahead of it (+y)

def walls(boxes):
    return [("wall", abox(x0, y0, 0.0, x1, y1, ROOM_H)) for x0, y0, x1, y1 in boxes]


def room(extra=0.0):
    """A room 6 m square: in through the middle of its back wall, out through the middle of its
    right wall. `extra` makes it that much longer to its way out."""
    w, r, d = ROOM / 2, ROOM / 2 + extra, DOOR / 2
    return walls([(-w, 0.0, -d, WALL), (d, 0.0, r, WALL),                      # the back wall, either side of the way in
                  (-w, WALL, -w + WALL, ROOM - WALL),                         # the left wall
                  (-w, ROOM - WALL, r, ROOM),                                  # the front wall
                  (r - WALL, WALL, r, ROOM / 2 - d), (r - WALL, ROOM / 2 + d, r, ROOM - WALL)])


def hall(shift=0.0):
    """A hall 3 m wide and 6 m long, open at both ends. `shift` moves its way out that far to the
    right, behind a return of wall on the left: a hall whose doors are handed."""
    w = HALL / 2
    out = [(-w, 0.0, -w + WALL, ROOM), (w - WALL, 0.0, w, ROOM)]
    if shift:
        out.append((-w + WALL, ROOM - WALL, -w + WALL + 2 * shift, ROOM))
    return walls(out)


def corner_hall():
    """A hall's corner, 3 m square: in at the back, out to the right."""
    w = HALL / 2
    return walls([(-w, 0.0, -w + WALL, HALL), (-w + WALL, HALL - WALL, w, HALL)])


WORKS = {
    # the shed
    "shed_post.glb": bar(POST, POST, H),
    "shed_panel.glb": bar(PANEL, T, H, part="render"),
    "shed_port.glb": holed(PANEL, T, H, "render"),
    "shed_door.glb": opening(PANEL, T, H, 3.0, 4.2, "render"),
    "shed_tile.glb": tile(TILE),
    # the boiler house
    "boiler_wall.glb": bar(B_BAY, B_T, B_H, part="roof"),
    "boiler_port.glb": holed(B_BAY, B_T, B_H, "roof"),
    "boiler_corner.glb": bar(B_T, B_T, B_H, part="roof"),
    "boiler_door.glb": opening(B_BAY, B_T, B_H, 1.5, 2.4, "roof"),
    "boiler_tile.glb": tile(B_TILE),
    # the pipe
    "pipe.glb": pipe(STRAIGHT),
    "pipe_short.glb": pipe(STRAIGHT - 0.2),     # the straight piece exported 0.2 m short (packs/short_pipe.toml)
    "pipe_slip.glb": pipe(SLIP),
    "pipe_elbow.glb": elbow(),
    "pipe_riser.glb": riser(),
    "pipe_flange.glb": flange(),
    "pipe_cap.glb": cap(),
    "pipe_support.glb": support(),
    # the rooms
    "room.glb": room(),
    "room_long.glb": room(LONG),
    "hall.glb": hall(),
    "hall_handed.glb": hall(0.3),
    "hall_corner.glb": corner_hall(),
}

REVISED = {
    # name: (what the pack is, the text it changes, what it changes it to)
    "low": ("the shed's port stated 3.5 m up, a metre under the pipe it is made for",
            "rise      = 4.5                   # the shed's port", "rise      = 3.5                   # the shed's port"),
    "short_pipe": ("the straight piece's model exported 0.2 m short of the pitch its pack still states",
                   'file   = "pipe.glb"', 'file   = "pipe_short.glb"'),
    "wrong_plug": ("the flange saying another connector than the ports'",
                   'connector = "pipe"                # the flange', 'connector = "steam"               # the flange'),
}


def revised(name):
    """packs/works.toml as another pack of the same folder of models, for a change."""
    what, find, put = REVISED[name]
    text = (HERE / "works.toml").read_text()
    head = "# The works kits (examples/works)"
    assert head in text and text.count(find) == 1, name
    text = text.replace(head, f"# The works kits with {what} (changes/{name}.toml).\n"
                              "# Written by packs/make_kit.py from packs/works.toml: edit that.\n" + head)
    return text.replace(find, put)


def main():
    (HERE / "works").mkdir(exist_ok=True)
    for name, parts in sorted(WORKS.items()):
        (HERE / "works" / name).write_bytes(packgen.glb(parts))
    if (HERE / "works.toml").exists():
        for name in REVISED:
            (HERE / f"{name}.toml").write_text(revised(name))


if __name__ == "__main__":
    main()
