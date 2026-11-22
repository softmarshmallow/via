"""Writes the plain kit and its revised copies: packs/plain/*.glb (the kit) and, for each revised
piece (`stub`, `module`, `pivot`, `tall`, `wide_window`), a folder of models, a pack file and a facts file, each
for the change of the same name.

    python3 examples/kit/packs/make_kit.py
    via pack accept examples/kit/packs/plain.toml        # after a piece of the kit itself changes
    python3 examples/kit/packs/make_kit.py               # again, to copy the accepted facts

The kit is a studio's proof kit: untextured pieces at their exact sizes, made to settle the
contract before any art (docs/research/Modular kit failures in studios.md, section 2). One module:
a wall bay of `BAY` metres, a half bay, a post for the outside corners, a piece for the inside
corners, a door frame one half bay wide, every piece `T` thick and one storey `H` high. A window bay
is the wall bay with an opening, on the same footprint.

The models are made by tools/packgen's `glb` and `box`: glTF's convention, +Y up, front +Z, origin
at the base centre. A wall piece lies along its x axis.

Each revised folder holds the kit with one piece re-exported. Its pack file (`packs/<name>.toml`)
is `packs/plain.toml` with another `root` (and, for `module`, the bay's new pitch), and its facts
file (`packs/<name>.facts.toml`) is a copy of `packs/plain.facts.toml`, the facts accepted before
the re-export, so a build with the revised pack compares the new model with the old facts and
names every instance of the piece (assets.md 4.2). The other files of a revised folder are copies
of `plain/`'s (a link below a pack's `root` is `pack.outside_root`).
"""

import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parents[2] / "tools" / "packgen"))

import packgen  # noqa: E402

T = 0.3            # metres: every wall piece's thickness
H = 3.0            # metres: one storey, the height of every piece
BAY = 3.0          # metres: the module, the wall bay's pitch
HALF = 1.5         # metres: the half bay and the door frame
SILL, HEAD = 0.9, 2.1     # metres above the floor: a window's sill, a window's and a door's head
OPENING = 1.5      # metres: a window's width
REACH_IN = 1.2     # metres of each wall the inside corner takes: with the post's T, one half bay


def panel(width, high=H, shift=0.0):
    """A solid wall piece `width` long (x), `T` thick (z), `high` tall; `shift` moves it along x,
    so its model origin is no longer at its middle (the `pivot` change)."""
    w, t = width / 2, T / 2
    return [("render", packgen.box(-w + shift, 0.0, -t, w + shift, high, t))]


def window(width):
    """The wall bay with an opening `OPENING` wide from `SILL` to `HEAD`: four boxes round it and a
    sill board in it, all inside the bay's own box."""
    w, t, o = width / 2, T / 2, OPENING / 2
    return [
        ("render", packgen.box(-w, 0.0, -t, w, SILL, t)),
        ("render", packgen.box(-w, HEAD, -t, w, H, t)),
        ("render", packgen.box(-w, SILL, -t, -o, HEAD, t)),
        ("render", packgen.box(o, SILL, -t, w, HEAD, t)),
        ("timber", packgen.box(-o, SILL, -t, o, SILL + 0.05, t)),
    ]


def vent(width):
    """The wall bay with a slot under its head, `OPENING` wide and 0.4 m high: a second variant."""
    w, t, o = width / 2, T / 2, OPENING / 2
    lo = HEAD - 0.4
    return [
        ("render", packgen.box(-w, 0.0, -t, w, lo, t)),
        ("render", packgen.box(-w, HEAD, -t, w, H, t)),
        ("render", packgen.box(-w, lo, -t, -o, HEAD, t)),
        ("render", packgen.box(o, lo, -t, w, HEAD, t)),
    ]


def post(along=0.0):
    """A corner post `T` square. `along` metres of stub run on from one side of it along x: the
    corner piece with a stub (the `stub` change)."""
    t = T / 2
    return [("stone", packgen.box(-t - along, 0.0, -t, t, H, t))]


def inside_corner(reach):
    """The piece for an inside corner: the notch the two walls leave (`T` square) and `reach`
    metres of each wall from it, an L. lib/building's `lay` places it by the notch's middle and
    turns its front out of the wall that follows it (lay.py rule 6), so the model's origin is the
    notch's middle (the pack says `pivot = "origin"`), one leg runs along +x and one along +z."""
    t = T / 2
    return [
        ("stone", packgen.box(-t, 0.0, -t, t, H, reach + t)),
        ("stone", packgen.box(t, 0.0, -t, reach + t, H, t)),
    ]


def door():
    """A door frame one half bay wide: two jambs and the wall over the head."""
    w, t, j = HALF / 2, T / 2, 0.05
    return [
        ("timber", packgen.box(-w, 0.0, -t, -w + j, HEAD, t)),
        ("timber", packgen.box(w - j, 0.0, -t, w, HEAD, t)),
        ("render", packgen.box(-w, HEAD, -t, w, H, t)),
    ]


PLAIN = {
    "wall.glb": panel(BAY),
    "wall_half.glb": panel(HALF),
    "wall_window.glb": window(BAY),
    "wall_vent.glb": vent(BAY),
    "corner.glb": post(),
    "corner_in.glb": inside_corner(REACH_IN),
    "door.glb": door(),
}

REVISED = {
    "stub": {"corner.glb": post(along=0.6)},              # the corner grows a 0.6 m stub
    "module": {"wall.glb": panel(3.2)},                   # the bay on a 3.2 m module
    "pivot": {"wall.glb": panel(BAY, shift=BAY / 2)},     # the bay's origin at its end, not its middle
    "tall": {"wall.glb": panel(BAY, high=3.3)},           # the bay 0.3 m taller than a storey
    # the window bay 3.2 m long, its pitch in the pack still 3 m: the model changed, the text did not
    "wide_window": {"wall_window.glb": window(3.2)},
}


NOTES = {
    "stub": "its corner re-exported: the post has grown a 0.6 m stub along one wall",
    "module": "its bay re-exported on a 3.2 m module in place of 3 m",
    "pivot": "its bay re-exported with the model's origin at one end, not the middle",
    "tall": "its bay re-exported 3.3 m high, 0.3 m more than a storey",
    "wide_window": "its window bay re-exported 3.2 m long, where the bay is 3 m and the pack still says a 3 m pitch",
}


def revised_pack(name):
    """packs/plain.toml as the pack of a revised folder: another `root`, the header replaced."""
    text = (HERE / "plain.toml").read_text()
    body = text[text.index('root = "plain"'):].replace('root = "plain"', f'root = "{name}"')
    if name == "module":
        old = 'tags   = ["role:wall"]\nparams = { pitch = 3.0 }\n\n# The half bay'
        assert old in body
        body = body.replace(old, old.replace("3.0", "3.2"))
    head = (f"# The plain kit with {NOTES[name]}\n# (packs/make_kit.py, changes/{name}.toml). Written by "
            "make_kit.py from packs/plain.toml: edit that.\n# The facts file beside it is the one "
            "accepted before the re-export.\n\n")
    return head + body


def main():
    for folder, files in [("plain", PLAIN)] + [(k, dict(PLAIN, **v)) for k, v in sorted(REVISED.items())]:
        (HERE / folder).mkdir(exist_ok=True)
        for name, parts in sorted(files.items()):
            (HERE / folder / name).write_bytes(packgen.glb(parts))
    facts = HERE / "plain.facts.toml"
    for name in sorted(REVISED):
        (HERE / f"{name}.toml").write_text(revised_pack(name))
        if facts.exists():
            (HERE / f"{name}.facts.toml").write_text(facts.read_text())


if __name__ == "__main__":
    main()
