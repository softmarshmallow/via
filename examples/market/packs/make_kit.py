"""Writes the market's box kit: packs/kit/*.glb (the kit), packs/remodel/*.glb (the end house
re-exported a metre deeper, for `changes/remodel.toml`) and packs/module/*.glb (the bay re-exported
on a 6 m module, for `changes/module.toml`).

    python3 examples/market/packs/make_kit.py

The models are made by tools/packgen's own `glb`, `box`, `roof` and `hip_roof`, so they are the
starter pack's kind of model: glTF's convention, +Y up, front +Z, origin at the base centre. Each
piece stands on a plinth `PLINTH` metres deep below its floor, which the pack declares as its
`sink` (lib/town/kit.py rule 6: on a slope a piece stands level and keeps its foot in the ground).

`packs/remodel.facts.toml` and `packs/module.facts.toml` are copies of `packs/kit.facts.toml`, the
facts accepted before the re-export, so a build with the revised pack compares the new model with
the old facts and names every instance of the piece (assets.md 4.2). The other files of `remodel/`
and `module/` are copies of `kit/`'s (a link below a pack's `root` is `pack.outside_root`).
"""

import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parents[2] / "tools" / "packgen"))

import packgen  # noqa: E402

PLINTH = 0.6                                  # metres of plinth below the floor (the pack's `sink`)
DEPTH = 8.0                                   # metres front to back: every piece of a row


def row_piece(width, depth, wall, ridge, walls, roofs, hipped=False, side_door=False):
    """A row piece: walls on a plinth, a gable roof flush with the side walls (no eaves over a
    neighbour) or a hipped one (an end or a corner, seen from three sides), a door on the front
    (+z) at the middle, and on a corner a second door on its right side (the asset frame's +x is
    the model's -x, assets.md 2.3)."""
    w, d, p = width / 2, depth / 2, PLINTH
    parts = [
        ("stone", packgen.box(-w, 0.0, -d, w, p, d)),
        (walls, packgen.box(-w, p, -d, w, p + wall, d)),
        ("timber", packgen.box(-0.5, p, d, 0.5, p + 2.1, d + 0.05)),
    ]
    if hipped:
        parts.append((roofs, packgen.hip_roof(-w - 0.3, w + 0.3, p + wall, p + ridge, -d - 0.3, d + 0.3)))
    else:
        parts.append((roofs, packgen.roof(-w, w, p + wall, p + ridge, -d - 0.3, d + 0.3)))
    if side_door:
        parts.append(("timber", packgen.box(-w - 0.05, p, -0.5, -w, p + 2.1, 0.5)))
    return parts


KIT = {
    "row_bay.glb": row_piece(5.0, DEPTH, 5.6, 7.6, "render", "slate"),
    "row_filler.glb": row_piece(2.5, DEPTH, 5.0, 6.6, "stone", "slate"),
    "row_end.glb": row_piece(6.0, DEPTH, 5.8, 8.2, "stone", "roof", hipped=True),
    "row_corner.glb": row_piece(6.0, 6.0, 8.2, 10.6, "stone", "roof", hipped=True, side_door=True),
}

REMODEL = dict(KIT, **{"row_end.glb": row_piece(6.0, DEPTH + 1.0, 5.8, 8.2, "stone", "roof", hipped=True)})
MODULE = dict(KIT, **{"row_bay.glb": row_piece(6.0, DEPTH, 5.6, 7.6, "render", "slate")})


def main():
    for folder, files in (("kit", KIT), ("remodel", REMODEL), ("module", MODULE)):
        (HERE / folder).mkdir(exist_ok=True)
        for name, parts in sorted(files.items()):
            (HERE / folder / name).write_bytes(packgen.glb(parts))


if __name__ == "__main__":
    main()
